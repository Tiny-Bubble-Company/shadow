---
title: Changelog
seoTitle: "Shadow changelog: releases, new features and fixes"
description: "Release notes for Shadow: notable changes, new features and fixes in each version, following Keep a Changelog and Semantic Versioning."
faq:
  - q: "What changed about message recipients?"
    a: "A request to contact or act on someone is now classified as `act`, and the recipient is passed through as written and logged. Set `[policy.act] targets = \"surrogate\"` for the previous behaviour."
  - q: "What was in Shadow 0.1.0?"
    a: "The initial release: the `shadow-core` detection and translation library, the HTTPS proxy and gateway, the `shadow` CLI, an optional NER sidecar, and install scripts for macOS, Linux and Windows."
  - q: "Where is the canonical changelog?"
    a: "`CHANGELOG.md` in the repository. It follows Keep a Changelog, and the project uses Semantic Versioning."
---

All notable changes to Shadow are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/). The source of truth is [`CHANGELOG.md`](https://github.com/Tiny-Bubble-Company/shadow/blob/main/CHANGELOG.md).

## Unreleased

### Added

- **Intent-aware policy.** Each request is classified as `act`, `report`, `code` or `converse`, and every detected value gets a treatment: `surrogate`, `token` (typed placeholders such as `[PERSON_0001]`), `keep` (for message recipients and action targets in provider-hosted tools, always logged) or `redact` (opt-in, irreversible). Configured under `[policy]`. See [Intent-aware policy](/shadow/concepts/intent-policy/).
- **Session API** in `shadow-core`: `shadow.session_for(&request_json)` and `shadow.session(RequestContext { .. })` with `mask`, `mask_json`, `unmask`, `unmask_json`, `stream_unmasker`, `classification` and `summary`.
- **File masking.** Multipart uploads, base64 file payloads (OpenAI `input_file`, Anthropic `document`, Gemini `inline_data`) and `data:` URIs. Text, CSV, JSON, HTML and XML masked as text; DOCX, XLSX and PPTX masked in place; PDF text extracted and masked; images passed through or blocked. Configured under `[files]`. See [File masking](/shadow/concepts/file-masking/).
- **Activity dashboard.** A privacy-safe activity feed (counts and categories only, never values) served on loopback at `/shadow/ui`, plus `shadow top`, `shadow activity` and `shadow dashboard`, and a menu-bar / tray app in `apps/shadow-tray`. See [Activity dashboard](/shadow/concepts/activity-dashboard/).
- **AI service catalog.** `crates/shadow-core/catalog.json` lists 280+ AI services and 360+ API hosts (model APIs, inference clouds, model routers, coding agents' backends and chat apps) and drives what the proxy masks; the providers page in the docs is generated from it. Services Shadow recognises but cannot mask (binary protocols such as Cursor, Windsurf, Amazon Q, Kiro) are tunnelled untouched and shown on the dashboard and in `shadow activity` as *seen, not protected*. Codex CLI signed in with ChatGPT (`chatgpt.com/backend-api/codex`), Gemini CLI signed in with Google (`cloudcode-pa.googleapis.com`) and GitHub Copilot are now masked.
- **AI chat apps and the system proxy.** `shadow install --system-proxy --apply` (or `shadow system-proxy on --apply`) routes the machine's traffic through Shadow with a fail-open PAC file; only AI hosts are decrypted and nothing else is recorded. Prompts sent in ChatGPT, Claude.ai, Gemini, Perplexity, DeepSeek and Grok are masked (prompt fields only, with persona fakes and the persona's timezone), and answers, streams and reloaded history are restored. Configured with `[proxy] chat_apps`. A macOS Network Extension that captures every app's connections is scaffolded in `apps/shadow-netext` (needs an Apple Developer ID to build).
- **More inference clouds and model APIs** intercepted: Replicate, Hugging Face, DeepInfra, Hyperbolic, SambaNova, Novita, Lambda, Baseten, NVIDIA NIM, Nebius, SiliconFlow, Cloudflare AI Gateway, AI21, Reka, Moonshot, Alibaba Qwen, Zhipu, MiniMax and the Meta Llama API, with gateway routes for the OpenAI-compatible ones. The dashboard labels each destination as a model API, inference cloud or chat app, and names browsers.
- **Card expiry and security code.** An expiry date or CVV after a card number or a label ("CVV: 123") is sent as `[CARD_EXPIRY]` / `[CARD_CVV]`, and a card number followed on the same line by its expiry is no longer missed.
- **Incident details.** Open any masked or blocked request in the dashboard, the menu-bar app (**Incidents and prompts**) or `shadow incidents <ID>` to see the prompt as written and as the AI received it, each value caught and what replaced it, the model, and the app, process and conversation that sent it. Kept in memory only for the last 200 incidents and never written to disk; values are blurred until you choose to show them. Configured with `[activity] details` and `details_limit`.
- **Name-part restoration.** A fake person's first or last name on its own is restored too ("Dear Hui," becomes "Dear Jane,"), except parts under 3 characters, parts shared by two fake people and parts that are real names in the vault. Other fragments (card endings, email parts, partial addresses) are still not restored. See [Security](/shadow/reference/security/).
- **Presidio integration.** A Microsoft Presidio Analyzer can be used as an additional detector, merged with the built-in detectors and failing open. Configured under `[presidio]`.
- **Documentation site** built with Starlight, plus a new README, brand assets, contributing guide, security policy and code of conduct.

### Changed

- **Message recipients are passed through.** A request whose goal is to contact or act on someone is now `act`, including compose requests ("draft/write/compose/prepare a reply, email, letter, reminder, invitation..."). The recipient phrase (the words after "to", "for" or "at" following the verb, or right after "email", "call" and similar) is kept: names, emails, phones, companies, addresses, IBANs and usernames in it reach the provider as written, so the model greets, researches and addresses the right person. Each keep is logged. Everything else in the request (other people, health data, IDs, cards, secrets, hostnames) is still masked. `[policy.act] targets` now defaults to `"keep"`; set `targets = "surrogate"` for the previous behaviour, where the model sees a stand-in recipient and Shadow restores the real one in the answer and in tool-call arguments. See [Intent-aware policy](/shadow/concepts/intent-policy/).
- **Goal-preserving intent.** Action verbs are also detected after "to", "and", "then" and "you". For provider-hosted actions with `remote_tool_targets = "keep"`, usernames and IBANs named in the instruction are kept real, and a follow-up that names no email or phone keeps the emails, phones and people anywhere in the request. Secrets, cards, national IDs, hostnames and IPs stay masked; every keep is logged. See [Intent-aware policy](/shadow/concepts/intent-policy/).

## 0.1.0

Initial release.

### Added

- `shadow-core`: detection (email, phone, person, address, IBAN with mod-97, cards with Luhn, SSN / NINO / Aadhaar, IP, MAC, hostnames, secrets, home-directory usernames, local machine facts), dictionary terms, persistent persona with 11 regions, format-preserving fakes, ChaCha20-Poly1305 encrypted vault with atomic, multi-process-safe writes, JSON and streaming unmaskers, custom detector trait.
- `shadow-proxy`: HTTPS forward proxy with a local CA that intercepts only known AI hosts, `/gw/<provider>` gateway with 14 built-in routes, PAC file, SSE and NDJSON restoration with split-fake reassembly for OpenAI, Anthropic, Gemini, Cohere and Ollama-style streams, identifier and header scrubbing, upstream proxy support, SigV4 detection.
- `shadow` CLI: `init`, `run`, `install` / `uninstall` (dry run by default), `status`, `identity`, `mask`, `unmask`, `scan`, `vault`, `terms`, `hosts`.
- Optional PyTorch NER sidecar in `ml/`.
- Packaging: install scripts for macOS, Linux and Windows; launchd, systemd and Scheduled Task integration; Dockerfile.
