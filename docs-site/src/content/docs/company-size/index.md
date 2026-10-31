---
title: "Rolling out AI privacy by company size: Shadow from one laptop to a fleet"
description: "How solo developers, startups, small businesses, mid-market firms and enterprises deploy Shadow to keep customer data and secrets out of AI tools."
keywords:
  - ai privacy for small business
  - llm data protection for startups
  - enterprise ai dlp rollout
  - self-hosted pii masking
  - deploy ai privacy proxy
  - mdm rollout ai tools
faq:
  - q: "How long does it take to set up Shadow?"
    a: "About five minutes on one machine: install, run shadow init, then point a tool at the gateway or run shadow install --apply for proxy mode. A team rollout takes longer because you decide on shared terms, policy and how to distribute the configuration."
  - q: "Is there a central admin console?"
    a: "No. Shadow is configured with a shadow.toml file on each machine or container. Teams distribute that file with their existing device or configuration management. A local audit log and shared team identities are on the roadmap."
  - q: "Does Shadow need SSO or user accounts?"
    a: "No. There are no accounts, logins or licences. That keeps setup simple, but it also means Shadow has no per-user identity or central reporting of its own."
  - q: "What does Shadow cost at scale?"
    a: "The software is free under Apache-2.0 at any size. The costs are rollout time, support for your own users, and optional detector infrastructure such as Presidio containers."
  - q: "Can Shadow run in Kubernetes?"
    a: "Yes, as a sidecar container next to each application, reached on localhost in gateway mode. You build the image from the repository's Dockerfile; a published image is on the roadmap."
sidebar:
  label: Overview
  order: 0
---

Shadow is the same binary for a freelancer and for a bank. What changes with size is how you install it, how you keep its configuration consistent, and what evidence you need to show that it works. These guides cover each stage, with a recommended deployment, a rollout checklist and an honest list of what is not there yet.

## Choose your size

| Guide | Typical setup | Main concern |
|---|---|---|
| [Solo developers and freelancers](/shadow/company-size/solo-developers-freelancers/) | One laptop, coding assistants, client work | Client confidentiality, your own secrets |
| [Startups](/shadow/company-size/startups/) | A handful of laptops, one product with AI features | Speed, security questionnaires, SOC 2 readiness |
| [Small businesses](/shadow/company-size/small-business/) | Up to ~100 people, little or no IT team | Simple setup, customer data in everyday AI use |
| [Mid-market](/shadow/company-size/mid-market/) | Hundreds of devices, MDM, internal apps | Consistent configuration, pilot and rollout |
| [Enterprise](/shadow/company-size/enterprise/) | Thousands of devices, platform teams, existing DLP | Coverage gaps, scale, fit with the security stack |
| [Regulated enterprise](/shadow/company-size/regulated-enterprise/) | Banks, insurers, health systems, public bodies | Evidence, change control, risk assessments |

## What changes with size

| | Solo / startup | Small business | Mid-market | Enterprise |
|---|---|---|---|---|
| **Install** | Install script, `shadow install --apply` | Same, done by whoever runs IT | MDM script per user | MDM plus your own signed build |
| **Configuration** | Local `shadow.toml` | One shared file copied to each laptop | Managed `shadow.toml` pushed into each data directory | Managed file, versioned and reviewed |
| **Servers and apps** | Gateway in `.env` | Rarely needed | systemd service or sidecar | Sidecar per workload, `NetworkPolicy` egress |
| **Detection** | Built-ins plus terms | Built-ins plus terms | Plus Presidio or the NER sidecar where needed | Tuned per workload, tested with `shadow scan` |
| **Evidence** | None needed | Written policy | Config in version control, rollout logs | Risk assessment, change records, test results |

## What every size gets

- **No licence and no account.** Apache-2.0, nothing to buy, nothing to sign in to.
- **No telemetry.** Shadow sends only the masked requests you were already sending, to the providers you already use.
- **Local mapping.** The real-to-fake mapping lives in an encrypted [vault](/shadow/concepts/vault/) on each machine or volume.
- **Three integration modes.** [Proxy](/shadow/integrate/proxy-mode/), [gateway](/shadow/integrate/gateway-mode/) and the [Rust library](/shadow/integrate/rust/), mixable on one machine.

## What is not there yet, at any size

:::note[Current gaps, from the roadmap]
- **No central admin console or fleet management.** Configuration is a file; distribute it with the tools you already use.
- **No audit log yet.** Today the proxy logs the intent and counts per request, never values, and each machine's local [activity dashboard](/shadow/concepts/activity-dashboard/) shows counts by data type. An optional local, append-only audit log is a [roadmap](/shadow/project/roadmap/) "Next" item.
- **No shared team identity yet.** Each machine has its own persona and vault. Team identities with key distribution are a "Later" item; today you can copy `persona.json`, `vault.bin` and `vault.key`.
- **Release binaries and a published container image are in progress.** The install scripts fall back to building with `cargo` where no release exists, and the Docker image is built from `packaging/Dockerfile`.
- **API traffic only.** Consumer web apps such as chatgpt.com are not built-in hosts.
:::

## Related

- [Shadow by industry](/shadow/industries/) for sector-specific risks and regulations
- [Use cases](/shadow/use-cases/) for step-by-step guides
- [How Shadow compares](/shadow/compare/) with DLP products, gateways and privacy vaults
