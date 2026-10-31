---
title: Integrating any endpoint
description: Put Shadow in front of anything that talks to an LLM, including custom HTTP APIs, agents, IDEs, desktop apps and self-hosted gateways.
---

Shadow is not tied to a list of applications. If something sends HTTP requests to a model API, Shadow can sit in front of it. This page shows how to decide where to plug it in.

## Decision guide

1. **Can you set a base URL?** Use [gateway mode](/shadow/integrate/gateway-mode/). Most SDKs, CLIs and frameworks (LangChain, LlamaIndex, Vercel AI SDK, LiteLLM, the OpenAI and Anthropic SDKs) accept one, often from `OPENAI_BASE_URL` or `ANTHROPIC_BASE_URL`.
2. **Does it honour `HTTPS_PROXY` or the system proxy?** Use [proxy mode](/shadow/integrate/proxy-mode/). This covers most IDE extensions, Electron apps, Python and Node scripts, and CI jobs.
3. **Is it your own Rust code?** [Embed `shadow-core`](/shadow/integrate/rust/).
4. **Is the endpoint not a built-in provider?** Add a route or an intercepted host for it (below).

## A custom or self-hosted model API

Any HTTPS endpoint that accepts JSON prompts can be protected: an internal LLM gateway, a vLLM or TGI server on another machine, a vendor API that wraps a model.

```toml
[proxy]
# proxy mode: decrypt and mask traffic to these hosts too
extra_hosts = ["llm.internal.example.com"]

[proxy.routes]
# gateway mode: http://127.0.0.1:8877/gw/internal/... -> https://llm.internal.example.com/...
internal = "https://llm.internal.example.com"
```

Shadow masks every string value in a JSON request body, except protocol keys such as `model`, `role` and `type`, so it works with request shapes it has never seen. Responses are restored in full JSON bodies and in SSE or NDJSON streams. Streams of unknown formats are still restored string by string; only the cross-event reassembly of split fakes is format-specific (OpenAI Chat Completions and Responses, Anthropic, Gemini, Cohere and Ollama-style NDJSON).

Non-JSON request bodies are forwarded as they are, except for [file uploads](/shadow/concepts/file-masking/), which are masked.

## Agents and tool calls

Agents are where privacy and usefulness collide: the model has to *act* on real values. Shadow handles this with the [intent-aware policy](/shadow/concepts/intent-policy/):

- When the agent's tools run **on your side** (your code executes the tool call), the model sees fakes and writes tool calls with fakes. Shadow restores the real values in the tool-call arguments before they reach your code, so the email goes to the real address.
- When tools run **in the provider's cloud** (hosted MCP servers, server-side tools), Shadow cannot restore values there. By default it keeps the action's target real and logs that decision. You can change this.

Details and examples in [Tool calls and agents](/shadow/concepts/tool-calls/).

## IDEs and coding assistants

Coding assistants send file contents, paths, hostnames and pasted configuration, which is exactly what Shadow is good at protecting. Usernames in paths (`/Users/jdoe/...`), internal hostnames, IPs and API keys are masked, and the `code` intent keeps identifiers readable.

- **VS Code and JetBrains extensions.** Most use the IDE's proxy setting or `HTTPS_PROXY`. Set the IDE's HTTP proxy to `http://127.0.0.1:8877` and make sure the CA is trusted by the OS (VS Code uses the OS store; JetBrains IDEs can be pointed at it in *Settings → Tools → Server Certificates*).
- **Terminal assistants and CLIs.** Start them from a shell where `env.sh` is sourced, or set their base URL variable.
- **Desktop chat apps.** Use the [PAC file](/shadow/integrate/proxy-mode/#system-wide-with-a-pac-file) or system proxy. Apps that pin certificates will refuse the local CA; there is no workaround for those besides using a different client.

## Backend services in any language

Point the service's model client at the gateway URL through configuration, and run Shadow next to it as a service or [sidecar](/shadow/integrate/docker-kubernetes/). No code change is required beyond the base URL.

```sh
# Python, Node, Go, Java: any OpenAI-compatible client
OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1 ./my-service
```

## Checklist

- Run `shadow scan` on a few representative prompts and documents to see what is detected before you rely on it.
- Add your organisation names, codenames and customers with `organization`, `terms` or `shadow terms add`.
- Decide how actions in provider-hosted tools are handled (`[policy.act] remote_tool_targets`).
- Keep Shadow on loopback, or inside the pod.
- Check the log line per request (`masked N value(s)`) during rollout.
