---
title: "AI data protection for mid-market companies: roll out PII masking with your MDM"
description: "Deploy Shadow across hundreds of laptops and internal AI apps: MDM scripts, managed configuration, pilot groups, sidecars and an honest list of gaps."
keywords:
  - mid-market ai data protection
  - mdm rollout ai privacy
  - intune jamf ai proxy
  - mask pii internal llm apps
  - ai usage policy rollout
  - self-hosted llm privacy gateway
faq:
  - q: "Can we deploy Shadow with Intune, Jamf or another MDM?"
    a: "Yes, with a script. Shadow has no MDM-specific package yet: the script installs the binary, writes the managed shadow.toml into the user's data directory, and runs shadow init and shadow install --apply as the signed-in user, because the service and CA trust are per user."
  - q: "Can IT change the policy centrally?"
    a: "Only by redistributing the configuration file. There is no admin console or policy server. Treat shadow.toml like any other managed configuration: version it, review changes and push it with your MDM or configuration management."
  - q: "How do we know Shadow is running on every laptop?"
    a: "Use shadow status --quiet in an MDM compliance or health script: it exits 0 when the proxy answers and 1 otherwise. There is no central fleet view."
  - q: "Should internal AI apps use the same setup as laptops?"
    a: "No. Servers and containers should use gateway mode: run Shadow as a systemd service or sidecar and set the app's base URL. No CA is needed and the data path is explicit."
  - q: "Can all laptops share one persona and vault?"
    a: "Not as a managed feature yet; shared team identities are on the roadmap. Each laptop generates its own persona, which is fine for privacy. You can copy persona.json, vault.bin and vault.key between machines if you need consistent fakes."
sidebar:
  order: 4
---

At a few hundred people, AI use is everywhere and inconsistent: some teams have approved tools, some have API keys, and a few internal apps call models directly. Shadow fits this stage as a per-device background service pushed with your MDM, plus a gateway or sidecar for internal apps, all driven by one versioned `shadow.toml`. There is no central console, so plan to use the device and configuration management you already have.

## What matters at this size

| Priority | What it means |
|---|---|
| Consistency | Every laptop protects the same customer names, domains and codenames |
| Low-touch rollout | Pushed by IT, not installed by hand |
| Internal apps | A few services call models with production customer data |
| Proportionate evidence | Enough to answer customers, auditors and the board, without a project team |
| Mixed fleet | macOS, Windows and some Linux |

## Recommended architecture

| Component | Deployment | Notes |
|---|---|---|
| Laptops | MDM script per user: install, config, `shadow install --apply` | Proxy mode covers API-based assistants and scripts |
| Internal apps on VMs | System-wide systemd unit with a dedicated `User=`, gateway URLs | See [Installation](/shadow/getting-started/installation/#run-as-a-background-service) |
| Internal apps in containers | Sidecar per app on `localhost` | See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |
| Extra recall where needed | Presidio or the NER sidecar on the same host or pod | Keep it on loopback: it receives plaintext |
| Configuration | One `shadow.toml` in git, pushed by MDM and deployment pipelines | Review changes like code |

## Rolling out to laptops

### 1. Build and host the binary

Release binaries are on the [roadmap](/shadow/project/roadmap/). Until they exist for your platforms, build in CI from a pinned commit and host the binaries in your software distribution system, so laptops do not need a Rust toolchain:

```sh
cargo build --release -p shadow-cli
```

### 2. Write the managed configuration

```toml
organization = ["Acme Holdings Ltd", "Acme Services GmbH"]
terms = ["org:Globex", "org:Initech", "Project Falcon", "Nightjar"]
internal_domains = ["acme.example", "acme.internal", "corp.acme.example"]
allow_domains = ["docs.acme-public.example"]

[policy.always]
secret = "redact"

[files]
pdf = "text"
images = "block"
```

### 3. Push a per-user script

The service and CA trust are per user (LaunchAgent and login keychain on macOS, systemd user unit on Linux, a Scheduled Task and the user certificate store on Windows), so run the setup in the user's context:

```sh
#!/bin/sh
# macOS example, run as the signed-in user
DATA="$HOME/Library/Application Support/shadow"
mkdir -p "$DATA" "$HOME/.local/bin"
install -m 0755 /Library/Acme/shadow/shadow "$HOME/.local/bin/shadow"
cp /Library/Acme/shadow/shadow.toml "$DATA/shadow.toml"
"$HOME/.local/bin/shadow" init --org "Acme Holdings Ltd"
"$HOME/.local/bin/shadow" install --apply
```

Adding a certificate to the login keychain may ask the user to confirm. Test the script on each OS in your pilot group, and add `source "<data dir>/env.sh"` to the managed shell profile so terminal tools use the proxy.

### 4. Health check

```sh
shadow status --quiet && echo ok || echo "shadow not running"
```

Use it as an MDM compliance or custom attribute script.

## Internal apps

Point each app's model client at Shadow and nothing else changes:

```sh
OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1 ./support-assistant
```

For Azure OpenAI or an internal model server, add routes:

```toml
[proxy.routes]
azure = "https://acme-prod.openai.azure.com"
llm = "https://llm.acme.internal"
```

If an app runs agents, decide how provider-hosted tools are treated (`[policy.act] remote_tool_targets`) and read [Tool calls and agents](/shadow/concepts/tool-calls/).

## Rollout plan

| Phase | Scope | Exit criteria |
|---|---|---|
| Pilot | 10–20 users across OSes, one internal app | No connectivity issues for a week; `shadow scan` reviewed on real samples |
| Tune | Add missed terms, decide on Presidio for teams with free-text names | Terms list agreed with data owners |
| Expand | Department by department via MDM | Health check passing on target devices |
| Steady state | Config changes through review; quarterly scan of samples | Owner named for `shadow.toml` |

## Rollout checklist

- [ ] Inventory AI tools and which provider APIs they call
- [ ] Choose approved, API-based tools for each team
- [ ] Build binaries from a pinned commit; host them internally
- [ ] Agree the managed `shadow.toml` with security and data owners
- [ ] Pilot on macOS, Windows and Linux; check certificate-pinning apps
- [ ] Put internal apps behind a gateway or sidecar
- [ ] Add `shadow status --quiet` to MDM health checks
- [ ] Document the control in your AI policy and risk register

## Gaps to plan around

:::note[Not available yet]
- **No admin console, policy server or fleet dashboard.** MDM and configuration management are the control plane.
- **No central logging or SIEM export.** Each instance logs intent and counts locally; an optional local audit log is a roadmap "Next" item.
- **Per-user `shadow terms add` entries** live in each user's `terms.json` and are not synchronised. Put shared terms in `shadow.toml`.
- **No coverage for consumer web chat** (chatgpt.com, claude.ai), mobile apps or vendor backends such as GitHub Copilot. Pair Shadow with your web filter or CASB for those.
:::

## Related

- [Enterprise](/shadow/company-size/enterprise/)
- [Shadow AI governance](/shadow/use-cases/shadow-ai-governance/)
- [Self-hosted AI gateway with PII masking](/shadow/use-cases/self-hosted-ai-gateway-with-pii-masking/)
- [Configuration reference](/shadow/reference/configuration/)
- [Benchmarks](/shadow/benchmarks/) and [comparisons](/shadow/compare/)
