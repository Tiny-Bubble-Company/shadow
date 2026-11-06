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

## Shadow for the whole company, set up by Meridian

When a company creates its Meridian workspace, Meridian provisions its Shadow Hub workspace through Shadow Connect (`POST /connect/v1/provision`) and keeps a `manage` key for it. Nobody creates a Hub account:

- Meridian's own server enrols as a server, so every model call it makes (agent, document writer, importer) reaches the Hub with a signed scan receipt.
- The Shadow container installed with Meridian enrols the host as well.
- From **Personnel** in Meridian, admins invite people, share an enrolment link and copy the install command for servers.
- **AI requests & threats** shows every request Shadow scanned across the company, and **Receipts** each machine's receipt chain and the workspace ledger.

| Shadow data | Lands in Meridian |
|---|---|
| People, laptops and servers | Personnel |
| Every scanned request, masking, blocks and dangerous tool calls | AI requests & threats |
| Scan receipts (AIR format), chains and the ledger | Receipts, and the evidence behind SHD-06 |
| Evidence pack | The evidence store, with its SHA-256 fingerprint and an activity ledger entry |
| SHD controls | Linked to the matching ISO 27001, ISO 42001 and SOC 2 controls |
| AI services in use | The AI systems register, without duplicates |

A company that already runs Shadow Hub can connect its own workspace instead with an evidence key: in Shadow Hub, **Integrations** → **Meridian** → **Connect**, then paste the key under **Settings → Shadow** in Meridian.
