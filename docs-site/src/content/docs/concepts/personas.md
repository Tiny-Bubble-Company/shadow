---
title: Personas
description: The persistent fake identity that Shadow presents to AI providers, and how fakes are drawn from it.
---

A persona is a coherent fake identity for one machine: a company, its domain, a city, a primary person, colleagues, a username, a user id and a hostname. Shadow creates it once, on `shadow init` or the first `Shadow::open`, stores it in `persona.json`, and draws every fake from it.

```text
$ shadow identity
◆ Persona — what AI providers see
  company         Xinghe Technology Co., Ltd. (xinghe.cn)
  person          Ying Chen <ying.chen@xinghe.cn>
  location        Chengdu, China · Asia/Shanghai · zh-CN
  user id         user-941ea3e72322
  username        yingchen
  hostname        xinghe-srv-11
  colleagues      Tao Zhu, Hui Yang, Lei Li, Yong Xu
```

`shadow identity --json` prints the full record, including the region code, locale, dial code and creation time.

## Why a persona instead of random fakes

Random, unrelated fakes make prompts incoherent: an email at one fake company, a signature from another, a phone number from a third country. Models notice, and answers get worse. A persona keeps fakes **consistent with each other**:

- your organisation maps to the persona's company, and emails at your domain map to addresses at the persona's domain;
- your own git identity and OS user map to the persona's primary person and username;
- phone numbers use the persona region's dial code; names come from the region's naming conventions;
- your machine's hostname maps to the persona hostname.

And because the persona is persistent, the same customer shows up under the same fake name tomorrow, which keeps conversation history and long-running agents consistent.

## Regions

By default the persona's region is chosen **far from the real one**, based on the machine's timezone. A machine in Europe gets a persona in, for example, China, Japan or Brazil. The idea is that the fake identity should not be a near miss of the real one.

Available regions:

| Code | Country | Code | Country |
|---|---|---|---|
| `CN` | China | `IN` | India |
| `JP` | Japan | `AU` | Australia |
| `KR` | South Korea | `MX` | Mexico |
| `BR` | Brazil | `ZA` | South Africa |
| `US` | United States | `CA` | Canada |
| `DE` | Germany | | |

Pin a region in configuration or at init:

```toml
[persona]
region = "JP"      # or "auto" (default)
```

```sh
shadow init --region JP
```

The region matters for answers. A persona in Shenzhen gets advice about Chinese public holidays and phone formats. If your prompts depend on locale (tax rules, address formats, legal context), pick a region that suits your work.

## Overrides

You can choose the company name and domain yourself:

```toml
[persona]
company = "Northwind Trading Ltd."
domain = "northwind.example"
```

Everything else is still generated.

## How fakes are generated

| Kind | Fake |
|---|---|
| Email | A name from the persona region at the faked domain: your organisation's domain becomes the persona domain, webmail domains such as `gmail.com` are kept. Separators, digits and case follow the original |
| Person | A name from the persona region's first and last names |
| Organisation | The persona company for your primary organisation (and its short name for the bare name); other organisations get a generated name with the region's legal suffix |
| Term (codename, project) | A generated codeword such as `Velora Quilent` |
| Phone | Same digit layout with the persona's dial code |
| Credit card | Same length and grouping, valid Luhn check digit |
| IBAN | Same country and length, valid mod-97 check digits |
| National ID | Same format |
| IP / MAC | Same family and format |
| Hostname | A generated domain with the persona region's TLD; internal TLDs (`.internal`, `.corp`, `.lan`, `.local`) and generic labels such as `www` or `api` are kept |
| Username (in paths) | A username in the persona region's style; your own OS user maps to the persona username |
| Secret | Same prefix and shape (`sk-proj-…` stays `sk-proj-…`) with a random body; PEM blocks keep their header and footer. Opt in to [redaction](/shadow/concepts/intent-policy/#treatments) instead |
| Address | A house number and a street from the persona region |

Every fake is checked against the vault so that no two real values share one.

## Changing or resetting the persona

`shadow init --force` deletes `persona.json` and `vault.bin` and starts over with a new identity. The old mapping is gone, so responses to old prompts that mention old fakes are no longer restored. To move the same identity to another machine, copy `persona.json`, `vault.bin` and `vault.key` (see [The vault](/shadow/concepts/vault/)).
