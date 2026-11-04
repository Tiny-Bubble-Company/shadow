---
title: Evidence receipts
seoTitle: "Signed evidence receipts for every AI request · Shadow"
description: "Shadow signs a receipt for every request it scans: what was found, what was sent and what came back, chained so a removed or edited record shows up. Verifiable with standard COSE tools."
keywords:
  - AI audit trail
  - tamper-evident AI logs
  - AI compliance evidence
  - COSE receipt
  - attested inference receipt
  - AI data loss prevention audit
---

Every request Shadow scans gets an **evidence receipt**: a small signed record of what Shadow found, what it sent to the AI and what came back. Receipts are chained, each one naming the one before it, so if a record is removed or changed later, the chain shows it. An auditor can check a receipt years later with the machine's public key and standard tools, without trusting Shadow, Shadow Hub or the logs around it.

A receipt never contains your text or the values Shadow caught. It holds hashes, counts and categories.

## Where to see them

- **Dashboard**: open any incident. The **Evidence receipt** card shows the result of checking it on this machine: the signature, the link to the previous receipt and whether the prompt shown matches what was signed. **Copy receipt** and **Download** give you the receipt and the public key as JSON. The overview has an **Evidence receipts** card that checks every receipt in the selected period.
- **Terminal**: `shadow receipts` checks the whole chain, `shadow receipts show <ID>` shows one and `shadow receipts verify <file>` checks a downloaded receipt without Shadow running.
- **Shadow Hub**: receipts are sent with each event. Hub checks each one against the machine's key when it arrives, keeps the chain per machine and has a verifier for receipts from any source.

## What a receipt says

| Claim | Meaning |
|---|---|
| `iss` | `shadow-device:<key id>`: the machine key that signed it |
| `iat`, `cti` | When it was signed, and a random receipt id |
| `sequence`, `prev_receipt` | Its position in this machine's chain and the SHA-256 of the receipt before it |
| `event_id`, `event_ts_ms` | The request in the local activity log, and when it arrived |
| `action` | `clean`, `masked`, `blocked` or `passed` |
| `provider`, `host`, `client`, `endpoint` | Where it went and which app sent it |
| `findings`, `detectors` | How many values of each kind were found, and by which detectors |
| `sent_body_hash` | SHA-256 of the exact bytes forwarded to the AI provider (absent when blocked) |
| `response_hash` | SHA-256 of the provider's answer as received |
| `delivered_hash` | SHA-256 of what was handed back to the app, with real values restored |
| `sent_text_hash` | SHA-256 of the prompt text the AI received, as shown in the incident |
| `original_text_commitment` | Salted SHA-256 of the prompt as written. The salt stays with the incident, so the hash can't be guessed from a short prompt |
| `policy_hash` | SHA-256 of the Shadow settings in force |
| `shadow_version`, `status`, `restored`, `block_reason`, `intent` | Version, HTTP status, values restored, why it was blocked, request intent |

## Format

Receipts follow the structure of the IETF draft [Attested Inference Receipt (AIR v1)](https://datatracker.ietf.org/doc/draft-tsyrulnikov-rats-attested-inference-receipt/), using the same building blocks:

- a **COSE_Sign1** envelope ([RFC 9052](https://www.rfc-editor.org/rfc/rfc9052)) with CBOR tag 18;
- a **CWT** claims set ([RFC 8392](https://www.rfc-editor.org/rfc/rfc8392)) profiled as an **EAT** ([RFC 9711](https://www.rfc-editor.org/rfc/rfc9711)), `eat_profile` = `https://getshadow.dev/spec/scan-receipt/v1`;
- **Ed25519** only: protected header `{1: -8, 3: 61}`, and the unprotected header must be empty;
- **deterministic CBOR** (RFC 8949 §4.2.1), at most 64 KiB, and a **closed** claims map: unknown claims are rejected.

Shadow's own claims use the private keys `-70001` to `-70022`. AIR describes a model inference inside confidential-computing hardware, with enclave measurements and a model hash. A scan receipt describes what a data-protection proxy saw and changed, so it uses its own profile and claims. It also adds chaining, which AIR v1 leaves for a later version.

### Verifying

Verification has four layers, as in AIR:

1. **Parse**: tag 18, four elements, the EdDSA algorithm, the CWT content type, an empty unprotected header, and the scan-receipt profile.
2. **Signature**: Ed25519 over `["Signature1", protected, h'', payload]` with strict verification.
3. **Claims**: every required claim is present with the right type, hashes are 32 bytes, there are no unknown or duplicate keys, the encoding is deterministic and `iss` names the signing key.
4. **Policy**:
   - **Chain**: `sequence` is one more than the previous receipt, and `prev_receipt` is its hash.
   - **Content**: the stored prompt hashes to `sent_text_hash`, and with the salt, to `original_text_commitment`.
   - **Key**: the key is the one registered for the machine.

Any COSE library can check layers 1 and 2. For example, in Python with `pycose`:

```python
import base64, json, cbor2
from pycose.messages import Sign1Message
from pycose.keys import OKPKey
from pycose.keys.curves import Ed25519

r = json.load(open("shadow-receipt-42.json"))
msg = Sign1Message.decode(base64.b64decode(r["receipt"]))
msg.key = OKPKey(crv=Ed25519, x=base64.b64decode(r["public_key"]))
assert msg.verify_signature()
claims = cbor2.loads(msg.payload)
print(claims[-70003], claims[-70005], claims[-70010])  # sequence, action, findings
```

## Keys and storage

- The signing key is `receipt.key` in Shadow's data folder, created on first use and readable only by your user account. Its public key and key id appear in the dashboard and in Shadow Hub.
- Receipts are kept in `activity/receipts.log` for as long as their requests (`retention_days`, 30 days by default). The newest receipt is always kept, so the chain carries on after a quiet month. For longer audits, Shadow Hub keeps the receipts for as long as your workspace's retention.
- Turn receipts off with `receipts = false` under `[activity]` in `shadow.toml`.

## What a receipt does and doesn't prove

A valid receipt proves that this machine's key signed these claims, and that nothing in the receipt has changed since. A valid chain proves that no receipt was removed or reordered after it was signed.

A receipt does not prove the machine itself was honest. Someone in control of the machine and its key could sign false claims. Receipts aren't hardware-attested either. Sending receipts to Shadow Hub as they happen limits this: once Hub has a receipt, a later rewrite on the laptop no longer matches what Hub holds.
