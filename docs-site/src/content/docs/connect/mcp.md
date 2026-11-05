---
title: MCP server
seoTitle: "MCP server to mask and restore PII for agents | Shadow"
description: "shadow serve includes an MCP server at /mcp with three tools, shadow_scan, shadow_protect and shadow_restore, so any MCP-capable agent can keep personal data and secrets out of other AI services and tools."
sidebar:
  order: 2
keywords:
  - mcp server pii
  - mcp privacy tool
  - agent data masking
  - mcp redaction
---

`shadow serve` includes an MCP server at `http://127.0.0.1:8878/mcp` (streamable HTTP, JSON responses). Agents use it to mask data before handing it to another model, a hosted tool or a third-party API, and to restore the answer.

## Tools

| Tool | Arguments | Returns |
|---|---|---|
| `shadow_scan` | `text`, `reveal?` | Findings with kind, detector, position and a shortened value |
| `shadow_protect` | `text`, `session_id?`, `instruction?`, `policy?` | Masked `text`, `session_id`, replacements and kinds |
| `shadow_restore` | `text`, `session_id?` | `text` with the real values back, and how many were restored |

The tools behave exactly like the [Privacy API](/shadow/connect/privacy-api/) endpoints of the same name. Results come back as `structuredContent` and as JSON text.

## Connect an agent

Start the server, then add it to your client.

```sh
shadow serve
```

**Claude Code**

```sh
claude mcp add --transport http shadow http://127.0.0.1:8878/mcp
```

**Cursor, Windsurf and other clients with an `mcp.json`**

```json
{
  "mcpServers": {
    "shadow": { "url": "http://127.0.0.1:8878/mcp" }
  }
}
```

**With a token** (required when Shadow listens on a non-loopback address), add the header your client supports:

```json
{
  "mcpServers": {
    "shadow": {
      "url": "https://shadow.internal.example.com/mcp",
      "headers": { "Authorization": "Bearer ${SHADOW_API_TOKEN}" }
    }
  }
}
```

## When to use the MCP server, and when the proxy

The MCP server protects what the agent *chooses* to pass through it: a document it is about to send to a hosted tool, a customer record going to another model. It does not see the agent's own model traffic. To protect that too, run the agent through Shadow Core in [gateway](/shadow/integrate/gateway-mode/) or [proxy](/shadow/integrate/proxy-mode/) mode, which also checks the agent's tool calls.
