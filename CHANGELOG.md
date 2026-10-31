# Changelog

All notable changes to Shadow are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Intent-aware policy.** Each request is classified as `act`, `report`, `code` or `converse`, and every detected value gets a treatment: `surrogate` (default), `token` (typed placeholders such as `[PERSON_0001]`), `keep` (for message recipients and action targets in provider-hosted tools, always logged) or `redact` (opt-in, irreversible). Configured under `[policy]`, `[policy.act]`, `[policy.report]` and `[policy.always]`.
- **Session API** in `shadow-core`: `Shadow::session_for(&request_json)` and `Shadow::session(RequestContext { .. })`, with `mask`, `mask_json`, `unmask`, `unmask_json`, `stream_unmasker`, `classification` and `summary`.
- **File masking.** `multipart/form-data` uploads, base64 file payloads in JSON (OpenAI `input_file.file_data`, Anthropic `document` blocks, Gemini `inline_data`) and `data:` URIs. Text, Markdown, CSV, JSON, HTML and XML masked as text; DOCX, XLSX and PPTX masked in place; PDF text extracted, masked and sent as text; images passed through or blocked. Configured under `[files]`.
- **Activity dashboard.** A privacy-safe activity feed (counts and categories only, never values) served on loopback at `/shadow/ui`, plus `shadow top`, `shadow activity` and `shadow dashboard`, and a menu-bar / tray app in `apps/shadow-tray`.
- **Name-part restoration.** A fake person's first or last name on its own is restored too ("Dear Hui," becomes "Dear Jane,", "Dr Zhu" becomes "Dr Okafor"). Parts under 3 characters, parts shared by two fake people and parts that are real names in the vault are skipped; real text equal to a fake name part is masked so a real person who shares it is not renamed. Other fragments (card endings, email parts, partial addresses) are still not restored.
- **Presidio integration.** A Microsoft Presidio Analyzer can be used as an additional detector. Findings are merged with the built-in detectors; Shadow fails open if Presidio is unavailable. Configured under `[presidio]`.
- **Documentation site** built with Astro Starlight in `docs-site/`, with a GitHub Pages workflow.
- New README, logo and banner, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, issue and pull request templates.

### Changed

- **Message recipients are passed through.** A request whose goal is to contact or act on someone is now `act`, including compose requests ("draft/write/compose/prepare a reply, email, letter, reminder, invitation..."). The recipient phrase (the words after "to", "for" or "at" following the verb, or right after "email", "call" and similar) is kept: names, emails, phones, companies, addresses, IBANs and usernames in it reach the provider as written, so the model greets, researches and addresses the right person. Each keep is logged. Everything else in the request (other people, health data, IDs, cards, secrets, hostnames) is still masked. `[policy.act] targets` now defaults to `"keep"`; set `targets = "surrogate"` for the previous behaviour, where the model sees a stand-in recipient and Shadow restores the real one in the answer and in tool-call arguments.
- **Goal-preserving intent.** Action verbs are also detected after "to", "and", "then" and "you" ("can you send...", "draft it and email..."). For provider-hosted actions with `remote_tool_targets = "keep"`, usernames and IBANs named in the instruction are kept real, and a follow-up that names no email or phone keeps the emails, phones and people anywhere in the request (earlier messages, pasted documents, tool results). Secrets, cards, national IDs, hostnames and IPs stay masked; every keep is logged.

### Removed

- The single-page `docs/index.html`, replaced by the documentation site.

## [0.1.0]

Initial release.

### Added

- `shadow-core`: detection of emails, phones, person names, street addresses, IBANs (mod-97), payment cards (Luhn), national IDs (SSN, NINO, Aadhaar), IPv4/IPv6, MACs, internal hostnames, secrets and home-directory usernames; dictionary terms and local machine facts; a persistent persona with 11 regions; format-preserving fakes; a ChaCha20-Poly1305 encrypted vault with atomic, multi-process-safe writes; JSON and streaming unmaskers; the `Detector` trait for custom detectors.
- `shadow-proxy`: HTTPS forward proxy with a local CA that terminates TLS only for known AI hosts; `/gw/<provider>` gateway with 14 built-in routes; PAC file; SSE and NDJSON restoration with split-fake reassembly for OpenAI, Anthropic, Gemini, Cohere and Ollama-style streams; identifier and header scrubbing; upstream proxy support; SigV4 detection.
- `shadow` CLI: `init`, `run`, `install` / `uninstall` (dry run by default), `status`, `identity`, `mask`, `unmask`, `scan`, `vault`, `terms`, `hosts`.
- Optional PyTorch NER sidecar in `ml/`.
- Packaging: install scripts for macOS, Linux and Windows; launchd, systemd and Scheduled Task integration; Dockerfile.

[Unreleased]: https://github.com/Tiny-Bubble-Company/shadow/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Tiny-Bubble-Company/shadow/releases/tag/v0.1.0
