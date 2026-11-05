---
title: Privacy API
seoTitle: "PII masking API: scan, protect, restore | Shadow"
description: "Run shadow serve and call /v1/scan, /v1/protect and /v1/restore to find, mask and restore sensitive data in text or JSON before it reaches an AI service. Self-hosted, with an OpenAPI description."
sidebar:
  order: 1
keywords:
  - pii masking api
  - pseudonymisation api
  - llm data masking api
  - reversible redaction api
---

The Privacy API is Shadow Core's masking and restoring, served over HTTP for other software. It runs on your side: start it next to the application, agent or platform that calls a model, and real values never leave that machine.

## Start it

```sh
shadow init --orgs "Acme GmbH"     # once: persona, vault key, protected terms
shadow serve                       # http://127.0.0.1:8878
```

| Option | Default | Meaning |
|---|---|---|
| `--listen` | `127.0.0.1:8878` | Address to listen on |
| `--token` / `SHADOW_API_TOKEN` | none | Bearer token every call except `/v1/health` and `/v1/openapi.json` must send |

On any address other than loopback, Shadow refuses to start without a token. The API uses the same persona, vault and protected terms as the proxy on that machine, so a value gets the same fake whether it went through the proxy or the API.

In a container, build the image as described in [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) and run it as a sidecar next to your service:

```yaml
services:
  shadow:
    image: shadow:local
    command: ["serve", "--listen", "0.0.0.0:8878"]
    environment:
      SHADOW_API_TOKEN: ${SHADOW_API_TOKEN}
    volumes: ["shadow-data:/data"]
    healthcheck:
      disable: true   # the image's check probes the proxy, which serve does not start
```

Keep the data volume: it holds the encrypted vault that maps fakes back to real values.

## Endpoints

| Method | Path | Does |
|---|---|---|
| GET | `/v1/health` | Liveness, no token needed |
| GET | `/v1/info` | Persona (the fake identity), vault counts, open sessions |
| POST | `/v1/scan` | Find sensitive values without changing anything |
| POST | `/v1/protect` | Replace sensitive values with consistent fakes |
| POST | `/v1/restore` | Translate fakes back to the real values |
| DELETE | `/v1/sessions/{id}` | Forget a session and the tokens it handed out |
| POST | `/mcp` | The same operations as [MCP tools](/shadow/connect/mcp/) |
| GET | `/v1/openapi.json` | OpenAPI 3.1 description, for generating clients |

Errors are JSON: `{"error": {"code": "session_not_found", "message": "..."}}`. Codes: `invalid_request`, `invalid_json`, `unauthorized`, `not_found`, `session_not_found`. Request bodies are limited to 4 MB.

### Protect

Send exactly one of `text` and `json`. With `json`, every string inside is masked, so you can pass a whole chat request body.

```sh
curl -s localhost:8878/v1/protect -H 'content-type: application/json' -d '{
  "text": "Please ask Maria Lopez (maria.lopez@chatgpt.com, +44 20 7946 0958) about the Acme GmbH renewal"
}'
```

```json
{
  "session_id": "ses_b70607075779a3f00561a6d084f31044",
  "text": "Please ask Kenji Arai (k.arai@hoshino-trading.jp, +44 20 7946 0412) about the Hoshino Trading renewal",
  "replacements": 4,
  "kinds": ["email", "org", "person", "phone"],
  "intent": "converse",
  "summary": {"surrogates": 4, "tokens": 0, "kept": 0, "redacted": 0,
              "kinds": {"email": 1, "org": 1, "person": 1, "phone": 1},
              "detectors": ["email", "org", "person", "phone"]}
}
```

| Field | Meaning |
|---|---|
| `text` or `json` | The payload to mask |
| `session_id` | Continue an earlier session instead of starting one |
| `policy` | `fakes` (default): persona fakes everywhere, restorable without a session. `intent`: Shadow's [intent policy](/shadow/concepts/intent-policy/), which may use typed tokens or keep values an action needs |
| `instruction` | The user's request, when `text` is a document rather than the request. With `policy: intent` it defaults to `text` |
| `tools` | `[{"name": "send_email", "remote": false}]`: tools the model may call, for the intent policy |

Secrets such as API keys and private keys are redacted, not faked: they never come back.

### Restore

```sh
curl -s localhost:8878/v1/restore -H 'content-type: application/json' \
  -d '{"text": "I wrote to Kenji Arai at k.arai@hoshino-trading.jp.", "session_id": "ses_b706..."}'
```

```json
{"text": "I wrote to Maria Lopez at maria.lopez@chatgpt.com.", "restored": 2}
```

Fakes are kept in the vault, so they restore even without `session_id` (an answer read back days later, say). Typed tokens from the `intent` policy only restore within their session. Sessions expire after an hour without use.

### Scan

```sh
curl -s localhost:8878/v1/scan -H 'content-type: application/json' -d '{"text": "Call +44 20 7946 0958"}'
```

```json
{"findings": [{"kind": "phone", "detector": "phone", "start": 5, "end": 21, "score": 1.0, "value": "+4************58"}],
 "counts": {"phone": 1}}
```

Offsets are UTF-8 byte offsets. Values come back shortened; add `"reveal": true` for the full value.

## Example: wrap a model call

```python
import requests, openai

SHADOW = "http://127.0.0.1:8878"

def ask(prompt: str) -> str:
    p = requests.post(f"{SHADOW}/v1/protect", json={"text": prompt}).json()
    answer = openai.OpenAI().chat.completions.create(
        model="gpt-4o-mini",
        messages=[{"role": "user", "content": p["text"]}],
    ).choices[0].message.content
    return requests.post(f"{SHADOW}/v1/restore",
                         json={"text": answer, "session_id": p["session_id"]}).json()["text"]
```

```ts
const SHADOW = 'http://127.0.0.1:8878';
const post = (path: string, body: unknown) =>
  fetch(SHADOW + path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) }).then((r) => r.json());

const { text, session_id } = await post('/v1/protect', { text: prompt });
const answer = await callYourModel(text);
const { text: restored } = await post('/v1/restore', { text: answer, session_id });
```

Generate a typed client in any language from `/v1/openapi.json`.

## API, gateway or library?

| You have | Use |
|---|---|
| A tool or SDK that can change its base URL | [Gateway mode](/shadow/integrate/gateway-mode/): no code |
| A platform or agent that builds its own requests, in any language | The Privacy API |
| An agent that speaks MCP | The [MCP server](/shadow/connect/mcp/) |
| A Rust service | The [`shadow-core` library](/shadow/integrate/rust/), in process |
