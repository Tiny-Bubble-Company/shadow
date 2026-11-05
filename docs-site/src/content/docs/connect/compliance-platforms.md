---
title: Integrate Shadow with compliance platforms
seoTitle: "Shadow with Vanta, Drata, Secureframe and more"
description: "Step-by-step paths to bring Shadow's AI usage evidence into Meridian, Vanta, Drata, Secureframe, Sprinto, Thoropass, Hyperproof or any other compliance platform, using Shadow Hub's Evidence API."
sidebar:
  order: 4
  label: Compliance platforms
keywords:
  - vanta ai governance
  - drata ai evidence
  - secureframe ai
  - sprinto ai compliance
  - hyperproof evidence api
  - iso 42001 evidence automation
---

Your compliance platform tracks controls, policies and evidence. Shadow produces the evidence that AI usage is under control: what was masked, which AI services are in use, which machines are covered. This page shows how to get one into the other.

Every path starts the same way:

1. Roll out Shadow Core on your machines and connect them to [Shadow Hub](/shadow/hub/).
2. In Hub, open **Integrations**, pick your platform and click **Connect**. Copy the key; it is shown once.
3. Follow the steps for your platform below.

Hub maps each Shadow control to SOC 2, ISO/IEC 27001, ISO/IEC 42001, the EU AI Act, GDPR and HIPAA, so you know where to attach it. See the [control table](/shadow/connect/evidence-api/#controls).

| Platform | How evidence arrives | Effort |
|---|---|---|
| [Meridian](#meridian) | Native: synced daily into controls, the AI systems register and the evidence ledger | Paste a key |
| [Vanta](#vanta) | Evidence pack on mapped controls, by API or upload | Small script or monthly upload |
| [Drata](#drata) | External evidence on mapped controls, by API or upload | Small script or monthly upload |
| [Secureframe](#secureframe) | Evidence on mapped controls, by upload or custom integration | Monthly upload |
| [Sprinto](#sprinto) | Evidence on mapped controls | Monthly upload |
| [Thoropass](#thoropass) | Evidence requests and controls | Upload per audit period |
| [Hyperproof](#hyperproof) | Proof on mapped controls, by API or upload | Small script or monthly upload |
| [Custom](#custom) | Evidence API or scheduled CSV | Your integration |

Native listings in the Vanta, Drata and Secureframe integration marketplaces are on the roadmap. Until then, the paths below use each platform's own custom-evidence features, which every one of them supports.

## Meridian

[Meridian](https://github.com/Tiny-Bubble-Company/meridian-stack) is the compliance platform built with Shadow. Shadow is part of every Meridian install: it already protects Meridian's own model calls, and connecting Hub adds the team's evidence.

1. In Hub, **Integrations** → **Meridian** → **Connect**.
2. In Meridian, open **Shadow evidence** and paste the Hub URL and key.
3. Meridian syncs the evidence pack daily. Each sync is stored as evidence with its SHA-256 fingerprint and an entry in the activity ledger, linked to the matching ISO 27001, ISO 42001 and SOC 2 controls. AI services Shadow sees fill the AI systems register.

See [Meridian](/shadow/connect/meridian/) for what lands where.

## Vanta

1. In Hub, **Integrations** → **Vanta** → **Connect**, and keep the key in your secret store.
2. In Vanta, find the controls and tests in the mapping table, for example ISO 27001 A.8.12 *Data leakage prevention* and SOC 2 CC6.7.
3. Attach the evidence pack to them: download it from Hub (**Download evidence pack**, JSON or CSV), or have a scheduled job fetch `/connect/v1/evidence` and upload it through Vanta's API where your plan includes it.
4. Repeat monthly, or schedule the job. A 30-day window each month gives continuous coverage.

## Drata

1. In Hub, **Integrations** → **Drata** → **Connect**.
2. In Drata, open each mapped control (for example SOC 2 CC6.1 and CC6.7) and add the evidence pack as external evidence.
3. To automate it, have a scheduled job fetch `/connect/v1/evidence.csv` and upload it to those controls with Drata's public API where your plan includes it.
4. Set the evidence renewal to monthly to match the window.

## Secureframe

1. In Hub, **Integrations** → **Secureframe** → **Connect**.
2. In Secureframe, open the mapped controls and upload the evidence pack as evidence, or use a custom integration if your account has one.
3. Refresh it monthly from Hub's **Download evidence pack** or a scheduled fetch.

## Sprinto

1. In Hub, **Integrations** → **Sprinto** → **Connect**.
2. In Sprinto, upload the evidence pack to the mapped controls (data leakage prevention, monitoring, asset inventory).
3. Refresh it monthly.

## Thoropass

1. In Hub, **Integrations** → **Thoropass** → **Connect**.
2. When your auditor requests evidence for data leakage prevention, AI system inventory or endpoint coverage, answer with the evidence pack for the audit period (`window=90d`), and the CSV for a one-page summary.

## Hyperproof

1. In Hub, **Integrations** → **Hyperproof** → **Connect**.
2. In Hyperproof, add the evidence pack as proof on the mapped controls.
3. To automate it, a scheduled job can fetch `/connect/v1/evidence` and upload it as proof through Hyperproof's API where your plan includes it.

## Custom

For any other GRC tool, data warehouse or internal dashboard:

- **API**: call the [Evidence API](/shadow/connect/evidence-api/) with the key. `GET /connect/v1/evidence` gives everything in one call.
- **CSV**: `GET /connect/v1/evidence.csv` returns one row per control with its framework references, ready for a spreadsheet or a manual upload.

A scheduled job is usually a few lines:

```sh
curl -sf "https://hub.getshadow.dev/connect/v1/evidence.csv?window=30d" \
  -H "Authorization: Bearer $SHADOW_EVIDENCE_KEY" \
  -o "shadow-evidence-$(date +%Y-%m).csv"
```

## What auditors get

The pack states its window, its source and the time it was generated. Each control has a status, a one-sentence summary with real numbers, the metrics behind it and the framework references. For tamper-evidence, point auditors to [evidence receipts](/shadow/concepts/evidence-receipts/): every Shadow machine signs a receipt per request, and Hub checks the chain.
