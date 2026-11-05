---
title: Shadow in Meridian
seoTitle: "Shadow in Meridian: built-in privacy and evidence"
description: "Meridian ships with Shadow built in: every model call is masked by default, the Shadow privacy tab shows what was protected, and the Shadow evidence tab syncs Shadow Hub's evidence into controls, the AI systems register and the evidence ledger."
sidebar:
  order: 5
  label: Meridian
keywords:
  - meridian compliance
  - ai compliance platform
  - iso 42001 platform
  - endpoint ai protection evidence
---

[Meridian](https://github.com/Tiny-Bubble-Company/meridian-stack) is an open-source compliance platform run by an agent: it builds your programme, drafts the policies, collects the evidence and keeps a ledger your auditor can verify. Shadow is built into it.

Together they cover AI compliance end to end: **Shadow is the endpoint control** that keeps sensitive data out of AI services on every laptop and server, and **Meridian is the system of record** that documents it, maps it to frameworks and shows it to auditors.

## Privacy, by default

Every model call Meridian makes goes through Shadow, in process. Your company's identity and sensitive data are swapped for a consistent fake persona before the request leaves your server, and the answer is translated back. There is nothing to configure.

The **Shadow privacy** tab shows it working: Shadow's status, the fake identity Meridian's model calls use, and a log of recent calls with how many values of each kind were masked and restored. The log holds counts only, never the values.

## Evidence, from the whole company

Connect Shadow Hub to bring in the evidence from every machine:

1. In Shadow Hub, **Integrations** → **Meridian** → **Connect**, and copy the key.
2. In Meridian, open **Shadow evidence**, paste the Hub URL (`https://hub.getshadow.dev`) and the key, and click **Sync**.

From then on Meridian syncs daily. The tab shows the seven [Shadow controls](/shadow/connect/evidence-api/#controls) with their status and the framework references that apply to your programme.

| Shadow data | Lands in Meridian |
|---|---|
| Evidence pack | The evidence store, with its SHA-256 fingerprint and an activity ledger entry |
| SHD controls | Linked to the matching ISO 27001, ISO 42001 and SOC 2 controls |
| AI services in use | The AI systems register, without duplicates |

Until Hub is connected, Meridian's own Shadow privacy numbers stand in as evidence that its model calls are protected.
