---
title: Embed in Rust
seoTitle: "Embed Shadow in Rust: in-process PII masking for LLMs"
description: "Embed the shadow-core Rust crate to mask and restore LLM calls in-process: sessions, intent policy, JSON and streaming helpers, no proxy hop."
faq:
  - q: "Do I need the proxy to use Shadow from Rust?"
    a: "No. The `shadow-core` crate masks and restores in-process, with no proxy hop. `shadow-proxy` is available too if you want to host the proxy inside your own binary."
  - q: "How do I restore a streamed response in Rust?"
    a: "Create a `StreamUnmasker` with `shadow.stream_unmasker()`, push each chunk and emit what it returns, then call `finish()` to flush the held-back tail. The output equals unmasking the whole text at once."
  - q: "When should I use a session instead of shadow.mask?"
    a: "Use a session for requests with tools or bulk data. It applies the intent policy, keeps the request's typed tokens for restoration, and should be created once per model call or agent turn."
  - q: "Is Shadow safe to share across threads?"
    a: "Yes. `Shadow` is cheap to clone and safe to share across threads and tasks, and several processes can share one data directory."
---

`shadow-core` is the engine behind the `shadow` binary: detection, persona, vault, policy and translation. Embed it when your own Rust service calls models and you want masking in-process, with no proxy hop. Meridian, a compliance platform, uses it this way so that every model call it makes is masked before it leaves.

## Add the dependency

```toml
[dependencies]
shadow-core = { git = "https://github.com/Tiny-Bubble-Company/shadow" }
serde_json = "1"
```

## Open an instance

```rust
use shadow_core::{Shadow, ShadowConfig};

let config = ShadowConfig::from_dir("/var/lib/myapp/shadow")
    .with_organization(vec!["Acme GmbH".into()])
    .with_terms(vec!["Project Falcon".into(), "person:Jane Doe".into()])
    .with_region("JP");

let shadow = Shadow::open(config)?;
```

`Shadow::open` creates or loads the persona, the vault and its key in the data directory. Use `ShadowConfig::load(path)` to read a `shadow.toml` instead. `Shadow` is cheap to clone and safe to share across threads and tasks.

Keep the data directory on persistent storage. It holds the mapping that restores responses, and the persona that keeps fakes stable over time.

## Mask and restore text

```rust
let masked = shadow.mask("Ping jane@acme.com about Acme GmbH");
// masked.text          -> "Ping hui.zhao@zhilan.cn about Zhilan Technology Co., Ltd."
// masked.replacements  -> 2
// masked.kinds         -> ["email", "org"]

let answer = call_llm(&masked.text)?;
let restored = shadow.unmask(&answer);
```

`shadow.mask` applies the default treatment (realistic surrogates for everything). It is the simplest API and fine for single-purpose prompts. For requests that carry tools or bulk data, use a [session](#sessions-and-intent).

## Mask a JSON request body

```rust
let mut body = serde_json::json!({
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Write to jane@acme.com"}]
});
let n = shadow.mask_json(&mut body);    // masks string leaves in place
shadow.scrub_identifiers(&mut body);    // OpenAI `user`/`safety_identifier`, Anthropic `metadata.user_id`

let mut response: serde_json::Value = send(&body)?;
shadow.unmask_json(&mut response);
```

`mask_json` skips protocol keys whose values must not change: `model`, `role`, `type`, `object`, `id`, `tool_call_id`, `tool_use_id`, `finish_reason`, `stop_reason`, `encoding_format`, `media_type`, `mime_type`, `detail`, `service_tier`, `reasoning_effort`, `voice`, `format`, `tool_choice` and `stop_sequence`.

## Restore a stream

Models stream text in small pieces, so a fake like `Wei Zhang` can arrive as `"Wei"` and `" Zhang"`. A `StreamUnmasker` holds back just enough text to restore values split across chunks:

```rust
let mut s = shadow.stream_unmasker();
let mut out = String::new();
for chunk in stream {
    out.push_str(&s.push(&chunk));   // text that is safe to emit now
}
out.push_str(&s.finish());           // flush the held-back tail
```

The concatenated output is identical to unmasking the whole text at once. It holds back at most the length of the longest fake minus one byte.

## Sessions and intent

A session applies the [intent-aware policy](/shadow/concepts/intent-policy/) to one request: it classifies what the request is for, chooses a treatment per value (surrogate, token, keep or redact), and remembers the request's tokens so the response can be restored.

```rust
// From a provider request body: reads the instruction, the tools and their kind.
let session = shadow.session_for(&request_json);

// Or from parts you already have.
use shadow_core::RequestContext;
let session = shadow.session(RequestContext {
    instruction: "Build a churn report from this customer list".into(),
    tools: vec![],   // the tools offered to the model, if any
});

let masked = session.mask(&prompt);          // Masked { text, replacements, kinds }
let n = session.mask_json(&mut body);        // usize
let text = session.unmask(&answer);
session.unmask_json(&mut response);
let mut s = session.stream_unmasker();

let c = session.classification();            // intent, remote_actions, reasons
let sum = session.summary();                 // counts: surrogates, tokens, kept, redacted
```

Typed tokens look like `[PERSON_0001]` and `[EMAIL_0002]`. They are reversible within the session and restored by `session.unmask`.

Use one session per model call (or per agent turn). Tool-call arguments that come back from the model are restored with `session.unmask_json`, so a local tool receives real values. See [Tool calls and agents](/shadow/concepts/tool-calls/).

## Inspect and manage

```rust
let findings = shadow.scan(text);          // Vec<Finding>: start, end, kind, score, detector
let persona = shadow.persona();            // the fake identity
let stats = shadow.stats();                // entries by kind
shadow.add_terms(&["Nightjar".into()]);    // protect more terms at runtime
shadow.forget("jane@acme.com")?;           // drop a mapping (real or fake)
```

## Custom detectors

Implement `shadow_core::Detector` and pass it to `Shadow::open_with_detectors`:

```rust
use shadow_core::{Detector, Finding, Kind};

struct TicketIds;

impl Detector for TicketIds {
    fn name(&self) -> &'static str { "ticket" }
    fn detect(&self, text: &str) -> Vec<Finding> {
        // Return byte ranges with a kind; Shadow resolves overlaps with other detectors.
        find_ticket_ids(text)
            .map(|(start, end)| Finding::new(start, end, Kind::Term, "ticket"))
            .collect()
    }
}

let shadow = Shadow::open_with_detectors(config, vec![Box::new(TicketIds)])?;
```

## Concurrency and multiple processes

- One `Shadow` can be shared across threads; masking takes a short write lock only when a new value is added to the vault.
- Several processes can share a data directory. Each save merges the other processes' entries first, so no mapping is lost, and readers refresh when the file changes.

## Using the proxy as a library

`shadow-proxy` exposes the proxy and gateway (`Proxy::new(shadow)`, `Proxy::run(addr)`) if you want to host them inside your own binary, for example with a custom listener or tests that redirect a host with `with_upstream_override`.
