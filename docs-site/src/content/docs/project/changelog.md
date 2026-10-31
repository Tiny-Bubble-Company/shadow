---
title: Changelog
description: "Release notes for Shadow: notable changes, new features and fixes in each version, following Keep a Changelog and Semantic Versioning."
---

All notable changes to Shadow are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/). The source of truth is [`CHANGELOG.md`](https://github.com/Tiny-Bubble-Company/shadow/blob/main/CHANGELOG.md).

## Unreleased

### Added

- **Intent-aware policy.** Each request is classified as `act`, `report`, `code` or `converse`, and every detected value gets a treatment: `surrogate`, `token` (typed placeholders such as `[PERSON_0001]`), `keep` (for action targets in provider-hosted tools, always logged) or `redact` (opt-in, irreversible). Configured under `[policy]`. See [Intent-aware policy](/shadow/concepts/intent-policy/).
- **Session API** in `shadow-core`: `shadow.session_for(&request_json)` and `shadow.session(RequestContext { .. })` with `mask`, `mask_json`, `unmask`, `unmask_json`, `stream_unmasker`, `classification` and `summary`.
- **File masking.** Multipart uploads, base64 file payloads (OpenAI `input_file`, Anthropic `document`, Gemini `inline_data`) and `data:` URIs. Text, CSV, JSON, HTML and XML masked as text; DOCX, XLSX and PPTX masked in place; PDF text extracted and masked; images passed through or blocked. Configured under `[files]`. See [File masking](/shadow/concepts/file-masking/).
- **Activity dashboard.** A privacy-safe activity feed (counts and categories only, never values) served on loopback at `/shadow/ui`, plus `shadow top`, `shadow activity` and `shadow dashboard`, and a menu-bar / tray app in `apps/shadow-tray`. See [Activity dashboard](/shadow/concepts/activity-dashboard/).
- **Name-part restoration.** A fake person's first or last name on its own is restored too ("Dear Hui," becomes "Dear Jane,"), except parts under 3 characters, parts shared by two fake people and parts that are real names in the vault. Other fragments (card endings, email parts, partial addresses) are still not restored. See [Security](/shadow/reference/security/).
- **Presidio integration.** A Microsoft Presidio Analyzer can be used as an additional detector, merged with the built-in detectors and failing open. Configured under `[presidio]`.
- **Documentation site** built with Starlight, plus a new README, brand assets, contributing guide, security policy and code of conduct.

### Changed

- **Goal-preserving intent.** Action verbs are also detected after "to", "and", "then" and "you". For provider-hosted actions with `remote_tool_targets = "keep"`, usernames and IBANs named in the instruction are kept real, and a follow-up that names no email or phone keeps the emails, phones and people anywhere in the request. Secrets, cards, national IDs, hostnames and IPs stay masked; every keep is logged. See [Intent-aware policy](/shadow/concepts/intent-policy/).

## 0.1.0

Initial release.

### Added

- `shadow-core`: detection (email, phone, person, address, IBAN with mod-97, cards with Luhn, SSN / NINO / Aadhaar, IP, MAC, hostnames, secrets, home-directory usernames, local machine facts), dictionary terms, persistent persona with 11 regions, format-preserving fakes, ChaCha20-Poly1305 encrypted vault with atomic, multi-process-safe writes, JSON and streaming unmaskers, custom detector trait.
- `shadow-proxy`: HTTPS forward proxy with a local CA that intercepts only known AI hosts, `/gw/<provider>` gateway with 14 built-in routes, PAC file, SSE and NDJSON restoration with split-fake reassembly for OpenAI, Anthropic, Gemini, Cohere and Ollama-style streams, identifier and header scrubbing, upstream proxy support, SigV4 detection.
- `shadow` CLI: `init`, `run`, `install` / `uninstall` (dry run by default), `status`, `identity`, `mask`, `unmask`, `scan`, `vault`, `terms`, `hosts`.
- Optional PyTorch NER sidecar in `ml/`.
- Packaging: install scripts for macOS, Linux and Windows; launchd, systemd and Scheduled Task integration; Dockerfile.
