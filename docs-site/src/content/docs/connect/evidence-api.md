---
title: Evidence API
seoTitle: "AI compliance evidence API for SOC 2 and ISO | Shadow"
description: "Shadow Hub's Evidence API serves seven controls, SHD-01 to SHD-07, with status, metrics and mappings to SOC 2, ISO/IEC 27001, ISO/IEC 42001, the EU AI Act, GDPR and HIPAA. Counts only: never prompts or values."
sidebar:
  order: 3
keywords:
  - ai compliance evidence
  - soc 2 ai evidence
  - iso 42001 evidence
  - iso 27001 a.8.12 evidence
  - ai inventory api
---

Shadow Hub knows which AI services your people use, what Shadow masked or blocked, and which machines are protected. The Evidence API serves that to compliance platforms as evidence for specific controls. It returns counts, statuses and summaries only: prompts and original values are never part of it, whatever [sharing level](/shadow/integrate/organisations/) your organisation uses.

## Create a key

In Shadow Hub, open **Integrations**, pick the platform and click **Connect**. Hub creates a key for that platform and shows it once:

```
shc_live_5Qd0Y...        base URL: https://hub.getshadow.dev/connect/v1
```

Keys are read-only and limited to your organisation. Hub stores only a hash, shows when each key was last used, and records creating and revoking keys in the settings history. Revoke a key on the same page; it stops working immediately.

## Endpoints

Send the key as a bearer token: `Authorization: Bearer shc_live_...`. `window` is `7d`, `30d` (default) or `90d`.

| Method | Path | Returns |
|---|---|---|
| GET | `/connect/v1/org` | Organisation, sharing level and plan |
| GET | `/connect/v1/evidence?window=30d` | The evidence pack (below) |
| GET | `/connect/v1/evidence.csv?window=30d` | One row per control, for spreadsheets and manual uploads |
| GET | `/connect/v1/controls` | The control catalogue and its framework mappings, without metrics |
| GET | `/connect/v1/ai-inventory?window=30d` | AI services and apps in use, unprotected apps, extension and MCP server counts |
| GET | `/connect/v1/coverage` | People, machines, and when each machine last reported |
| GET | `/connect/v1/incidents?window=30d&limit=50` | Masked and blocked requests: time, service, app, action and counts per kind |

A wrong or revoked key gets `401 {"error": "invalid_key"}`.

```sh
curl -s https://hub.getshadow.dev/connect/v1/evidence?window=30d \
  -H "Authorization: Bearer $SHADOW_EVIDENCE_KEY"
```

## The evidence pack

```json
{
  "schema": "shadow.evidence/v1",
  "org": {"id": "org_...", "name": "Acme GmbH"},
  "window": "30d", "from": 1793000000000, "to": 1795600000000, "generated_at": 1795600000000,
  "source": {"product": "Shadow Hub", "url": "https://hub.getshadow.dev"},
  "coverage": {"people": 12, "devices": 15, "active_devices": 14},
  "totals": {"requests": 5210, "masked": 1032, "blocked": 4, "clean": 4174, "unprotected": 0, "values_protected": 4210},
  "controls": [
    {
      "id": "SHD-01",
      "title": "Sensitive data is masked before it reaches AI services",
      "status": "pass",
      "summary": "4,210 values masked in 1,032 AI requests across 14 devices in the last 30 days.",
      "metrics": {"masked": 1032, "values_protected": 4210},
      "frameworks": {"soc2": ["CC6.1", "CC6.7"], "iso27001": ["A.8.12", "A.5.12"], "iso42001": ["A.7.4"],
                     "eu_ai_act": [], "gdpr": ["Art. 25", "Art. 32"], "hipaa": ["164.312(e)(1)"]}
    }
  ],
  "ai_inventory": {"providers": [...], "clients": [...], "unprotected": [...]}
}
```

Times are Unix milliseconds. `status` is `pass`, `attention` or `no_data` (nothing reported in the window).

## Controls

| Id | Control | SOC 2 | ISO 27001 | ISO 42001 | EU AI Act | GDPR | HIPAA |
|---|---|---|---|---|---|---|---|
| SHD-01 | Sensitive data is masked before it reaches AI services | CC6.1, CC6.7 | A.8.12, A.5.12 | A.7.4 | | Art. 25, 32 | 164.312(e)(1) |
| SHD-02 | Prohibited data is blocked from AI services | CC6.7 | A.8.12 | A.7.4 | | Art. 32 | 164.312(e)(1) |
| SHD-03 | AI services in use are inventoried | CC2.1, CC9.2 | A.5.9, A.5.23 | A.4.2, A.6.2.6 | Art. 26 | Art. 30 | 164.308(a)(1)(ii)(A) |
| SHD-04 | Unprotected AI usage is detected and reported | CC7.2 | A.8.16 | A.6.2.6 | Art. 26 | Art. 32 | 164.308(a)(1)(ii)(D) |
| SHD-05 | Endpoint protection covers enrolled machines | CC6.8 | A.8.1, A.8.7 | A.4.5 | | Art. 32 | 164.310(b) |
| SHD-06 | AI activity records are tamper-evident | CC7.2, CC4.1 | A.8.15 | A.6.2.8 | Art. 12 | Art. 5(2) | 164.312(b) |
| SHD-07 | AI extensions, MCP servers and plugins are reviewed | CC6.8, CC8.1 | A.8.19 | A.10.3 | | | |

ISO/IEC 27001 references are to the 2022 Annex A. Mappings say where Shadow's evidence supports a control; your auditor decides whether it is sufficient on its own. SHD-06 is backed by the [evidence receipts](/shadow/concepts/evidence-receipts/) every Shadow machine signs.

## Next

- [Compliance platforms](/shadow/connect/compliance-platforms/): how to bring this into Vanta, Drata, Secureframe and others.
- [Meridian](/shadow/connect/meridian/): the native integration.
