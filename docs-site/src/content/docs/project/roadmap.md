---
title: Roadmap
description: "What is planned for Shadow, the open-source privacy layer for AI: shipped features and upcoming directions. Directions, not promises."
---

Shadow is young. This page lists what we intend to work on next, roughly in priority order. It is a statement of direction, not a commitment to dates. Suggestions and pull requests are welcome: open an issue to discuss before starting on anything large.

## Shipped

- Reversible, format-preserving translation with a persistent persona and encrypted vault
- HTTPS forward proxy with a local CA, gateway mode, PAC file
- SSE and NDJSON restoration with split-fake reassembly, tool-call argument restoration
- Built-in regex, checksum, dictionary and local-facts detectors; optional NER sidecar
- Intent-aware policy: `act`, `report`, `code`, `converse` with surrogate, token, keep and redact treatments
- File masking: text formats, DOCX, XLSX, PPTX in place, PDF as text
- Presidio as an additional detector
- Service installation on macOS, Linux and Windows; Docker image
- Documentation site

## Next

- **Release binaries** for macOS (arm64, x86_64), Linux (x86_64, aarch64) and Windows, built and signed in CI, so the install scripts no longer fall back to `cargo`.
- **Published container image** on a public registry.
- **OCR for images and scanned PDFs**, local and optional, so text in screenshots can be masked.
- **WebSocket and Realtime API** support.
- **Audit log**: an optional local, append-only record of per-request decisions (intent, counts by kind and treatment, keeps), never values.
- **More regions** for personas, and more national ID and phone formats.

## Later

- **Team identities**: a shared persona and vault for a group of machines, with key distribution.
- **Policy per host and per tool**: different treatments for different providers or tools.
- **Language bindings** for `shadow-core` (Python, Node.js) for in-process masking outside Rust.
- **Desktop status app**: a menu-bar indicator for status, recent counts and quick bypass.
- **Detection quality benchmarks**: a public corpus and reproducible precision/recall numbers per detector.

## Not planned

- A hosted Shadow service. Shadow is local by design.
- Blocking-only DLP features. Shadow translates; blocking is available only where translation cannot work (for example `pdf = "block"`, `images = "block"`).
