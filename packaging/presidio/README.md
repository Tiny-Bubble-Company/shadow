# Presidio Analyzer for Shadow

Shadow can ask [Microsoft Presidio Analyzer](https://microsoft.github.io/presidio/) for
extra findings on top of its built-in regex, checksum and dictionary detectors. This is
optional and off by default.

## What Presidio adds

- **NER-based detection** of people, organizations and locations using spaCy models,
  which catches names Shadow's dictionary-based person detector does not know.
- **50+ predefined recognizers** (national IDs, passports, licences, bank numbers and
  more across the US, UK, EU, India, Australia, Singapore, Korea, Thailand, ...), each
  combining patterns, checksums and context words.
- **Other languages** via different NLP engines (spaCy, Stanza or transformers models).
  The stock image ships English; other languages need a custom image with the matching
  models and a `language` setting in Shadow.

Shadow still owns everything after detection: it generates the fakes, keeps the
persona consistent, stores mappings in the encrypted vault and restores real values in
responses. Presidio only returns spans; it never sees the fakes or the vault.

## Run it

```sh
docker compose -f packaging/presidio/docker-compose.yml up -d
curl -s -X POST http://127.0.0.1:5002/analyze \
  -H 'Content-Type: application/json' \
  -d '{"text":"Call John Smith at 212-555-0134","language":"en"}'
```

The first start takes a while as the spaCy model loads.

## Enable it in `shadow.toml`

```toml
[presidio]
enabled = true
url = "http://127.0.0.1:5002/analyze"   # or http://presidio-analyzer:3000/analyze inside a compose network
language = "en"
min_score = 0.5        # drop findings below this confidence
entities = []          # empty = everything Presidio knows; or e.g. ["PERSON", "UK_NHS", "IN_PAN"]
timeout_ms = 800
```

Only plain `http://` is supported. Keep the analyzer on loopback or a private network:
it receives the unmasked text.

Mapping notes:

- `PERSON`, `ORGANIZATION`, `LOCATION`, `EMAIL_ADDRESS`, `PHONE_NUMBER`, `IBAN_CODE`,
  `CREDIT_CARD`, `IP_ADDRESS` map to the matching Shadow kinds; national IDs, passports,
  licences and bank numbers become `national_id`; `CRYPTO` becomes `secret`.
- `DATE_TIME` is always ignored (dates alone are not identifying).
- `URL` and `NRP` (nationality, religious or political group) are ignored unless you
  list them in `entities`; a requested `URL` masks just its host.

## Latency trade-off

Every masked request makes one extra HTTP round trip to the analyzer, and spaCy NER
typically adds tens of milliseconds for short prompts and more for long documents.
`timeout_ms` caps the wait. If the analyzer is down, slow or returns an error, Shadow
logs one warning and continues with its built-in detectors only (it fails open), so a
missing sidecar never blocks traffic, but it also means fewer findings while it is
down. Many common identifiers (UK NHS, US ITIN, India PAN, Australian TFN/ABN, Bitcoin
and Ethereum addresses) are also detected natively without Presidio.
