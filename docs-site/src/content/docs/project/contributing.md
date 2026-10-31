---
title: Contributing
description: "How to contribute to Shadow: build from source, run the tests, work on the docs and detectors, and send a pull request."
---

Thanks for your interest in Shadow. Bug reports, detector improvements, new provider formats, documentation fixes and test cases are all welcome. The canonical guide is [`CONTRIBUTING.md`](https://github.com/Tiny-Bubble-Company/shadow/blob/main/CONTRIBUTING.md) in the repository; this page summarises it.

## Repository layout

```text
crates/shadow-core    detection, persona, vault, policy, translation (library)
crates/shadow-proxy   CA, forward proxy, gateway, files, SSE translation (library)
crates/shadow-cli     the `shadow` binary
ml/                   optional PyTorch NER sidecar
packaging/            install scripts, launchd / systemd / Windows service, Dockerfile
docs-site/            this documentation (Astro Starlight)
assets/               logo, banner, diagrams
```

## Build and test

You need Rust 1.88 or newer. Nothing else.

```sh
cargo build --workspace
cargo test --workspace                                  # unit and integration tests
cargo clippy --workspace --all-targets -- -D warnings   # lints, as in CI
cargo fmt --all                                         # formatting
```

The integration tests run the proxy against a mock upstream, covering gateway routing, TLS interception and SSE restoration. They need no network access and no API keys.

The optional NER sidecar has its own tests:

```sh
cd ml && python -m unittest discover -s tests
```

## Documentation

The docs site lives in `docs-site/` and uses [Starlight](https://starlight.astro.build/). You need Node.js 22 or newer and pnpm (or npm).

```sh
cd docs-site
pnpm install
pnpm dev      # http://localhost:4321/
pnpm build    # static site in docs-site/dist
```

The site is published at [getshadow.dev](https://getshadow.dev/) and served from the domain root. `SITE` (default `https://getshadow.dev`) sets canonical URLs, the sitemap and structured data; `BASE` builds for a sub-path. `docs-site/deploy/` has a Docker and Caddy setup for self-hosting a copy.

Pages are Markdown or MDX files under `docs-site/src/content/docs/`. The sidebar is configured in `docs-site/astro.config.mjs`. Every page has an *Edit page* link to its source on GitHub.

## Pull requests

- Open an issue first for large changes, so we can agree on the approach.
- Keep pull requests focused. Add tests for behaviour changes; detector changes need positive and negative cases.
- CI must pass: `fmt`, `clippy -D warnings` and `test` on Linux, macOS and Windows.
- Update the docs and `CHANGELOG.md` when behaviour changes.
- Never put real personal data or real secrets in tests, fixtures or issues. Use obviously fake values.

## Security issues

Do not open public issues for vulnerabilities. Follow [`SECURITY.md`](https://github.com/Tiny-Bubble-Company/shadow/blob/main/SECURITY.md).

## Code of conduct

This project follows the [Contributor Covenant 2.1](https://github.com/Tiny-Bubble-Company/shadow/blob/main/CODE_OF_CONDUCT.md).
