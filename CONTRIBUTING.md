# Contributing to Shadow

Thanks for helping make AI traffic more private. Bug reports, detector improvements, new provider formats, documentation and tests are all welcome.

## Ground rules

- **Never put real personal data or real secrets** in code, tests, fixtures, issues or pull requests. Use obviously fake values (`jane@example.com`, `4111 1111 1111 1111`).
- **Security issues go to [SECURITY.md](SECURITY.md)**, not to public issues.
- Open an issue before starting a large change so we can agree on the approach.
- Be kind. This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).

## Development setup

You need **Rust 1.88 or newer** (`rustup` recommended). Nothing else: no OpenSSL, no Python, no Node, for the Rust workspace.

```sh
git clone https://github.com/Tiny-Bubble-Company/shadow
cd shadow
cargo build --workspace
```

Run your build against a throwaway data directory so you don't touch your real persona and vault:

```sh
export SHADOW_HOME=$(mktemp -d)
cargo run -p shadow-cli -- init --org "Acme GmbH"
echo "Mail jane@acme.com" | cargo run -q -p shadow-cli -- mask
cargo run -p shadow-cli -- run
```

## Layout

| Path | What lives there |
|---|---|
| `crates/shadow-core` | Detection, persona, vault, policy, translation (library) |
| `crates/shadow-proxy` | Local CA, forward proxy, gateway, file handling, SSE translation (library) |
| `crates/shadow-cli` | The `shadow` binary |
| `ml/` | Optional PyTorch NER sidecar |
| `packaging/` | Install scripts, launchd / systemd / Windows service, Dockerfile |
| `docs-site/` | Documentation site (Astro Starlight) |
| `assets/` | Logo, banner, diagrams |

## Before you push

CI runs these on Linux, macOS and Windows. Run them locally first:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The integration tests use a mock upstream (gateway routing, TLS interception, SSE restoration) and need no network access or API keys.

The optional NER sidecar has its own tests:

```sh
cd ml && python -m unittest discover -s tests
```

## Writing good changes

- **Detectors:** add positive *and* negative test cases. False positives on code, URLs and ordinary prose matter as much as misses.
- **Provider formats:** add a test with a realistic request or stream captured from the provider's documentation, with values replaced by fakes.
- **Behaviour changes:** update the docs in `docs-site/` and add an entry under *Unreleased* in [CHANGELOG.md](CHANGELOG.md).
- Keep pull requests focused, and explain the *why* in the description.

## Documentation

The site in `docs-site/` uses [Starlight](https://starlight.astro.build/). You need Node.js 22+ and pnpm (or npm).

```sh
cd docs-site
pnpm install
pnpm dev        # http://localhost:4321/shadow/
pnpm build      # static output in docs-site/dist
```

Pages are Markdown/MDX in `docs-site/src/content/docs/`. The sidebar is defined in `docs-site/astro.config.mjs`. Everything you document must match the code.

## License

By contributing, you agree that your contributions are licensed under the [Apache License 2.0](LICENSE).
