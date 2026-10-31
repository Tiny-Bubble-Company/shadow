---
title: "AI data privacy for small businesses: use ChatGPT-style tools without leaking customer data"
description: "A practical guide for SMBs: keep customer names, emails and payment details out of AI tools with free, self-hosted Shadow. Setup, limits and checklist."
keywords:
  - small business ai data privacy
  - smb chatgpt customer data
  - protect customer data ai small business
  - free ai privacy tool
  - gdpr ai small business
  - msp ai security
faq:
  - q: "Can a small business use ChatGPT safely with customer data?"
    a: "Safer use comes from three things: a short written policy, an approved tool, and a technical control that keeps identifiers out of prompts. Shadow is that control for API-based tools; it replaces customer names, emails, phone numbers, card numbers and IBANs with fakes and restores them in the answer."
  - q: "Does Shadow work with the ChatGPT website?"
    a: "No. Shadow protects traffic to AI provider APIs such as api.openai.com. The chatgpt.com web app is not a built-in host. Give staff an approved chat client that uses an API key and runs through Shadow."
  - q: "Do we need an IT person to set it up?"
    a: "Someone comfortable with a terminal, for about fifteen minutes per computer. If you use a managed service provider, they can script the install. There is no graphical installer or admin console yet."
  - q: "Is it really free for business use?"
    a: "Yes. Shadow is licensed under Apache-2.0, which allows commercial use. There is no subscription, account or usage limit. You still pay your AI provider as usual."
  - q: "Does Shadow help with GDPR for a small company?"
    a: "It supports data minimisation and pseudonymisation when staff use AI with customer data, which are GDPR principles. You still need a lawful basis, a data processing agreement with the AI provider and a record of the processing."
sidebar:
  order: 3
---

Small businesses use AI for the same things larger ones do: answering customer emails, summarising orders, drafting quotes, cleaning up spreadsheets. Shadow keeps the customer data in those requests on your computers by swapping it for realistic fakes before it reaches the AI provider, then putting the real details back in the answer. It is free, but it is a technical tool: it needs someone to install it, and it protects API-based AI tools, not the chatgpt.com website.

## What matters for a small business

| Concern | Reality for an SMB | How Shadow fits |
|---|---|---|
| Budget | No money for enterprise DLP | Free, open source, no subscription |
| Time | Nobody owns "AI security" full time | Install once per computer; runs in the background |
| Customer trust | One leaked customer list can hurt more than a fine | Customer names, emails, phones, cards and IBANs replaced before leaving |
| Everyday tools | Staff use whatever AI tool is easiest | Works when the tool uses an AI provider's API |
| Regulation | GDPR, state privacy laws, PCI DSS if you take cards | Supports data minimisation; [never a compliance guarantee](#honest-limits) |

## The pattern that works

Banning AI rarely works; people use it on their phones instead. The pattern that does work is to make the safe option the easy one:

1. **Pick one approved AI tool** that uses an API key: a desktop or web chat client you host or install, or an internal assistant, connected to OpenAI, Anthropic or Azure OpenAI.
2. **Run it through Shadow** on each computer, so customer details are masked automatically.
3. **Write a one-page policy**: use the approved tool for work, never paste customer data into personal AI accounts, ask before uploading files.

The [shadow AI governance guide](/shadow/use-cases/shadow-ai-governance/) explains the approach in more detail.

## Setting it up

### On each computer

macOS or Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Northwind Bakery Ltd"
shadow install --apply
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.ps1 | iex
shadow init --org "Northwind Bakery Ltd"
shadow install --apply
```

`shadow install` without `--apply` prints every change first, so whoever installs it can read the plan. With `--apply` it starts Shadow at login and trusts its local certificate.

### One shared settings file

Write one `shadow.toml` with your business name, key customers and suppliers, and copy it into each computer's Shadow data folder (`~/Library/Application Support/shadow` on macOS, `%APPDATA%\shadow` on Windows, `~/.local/share/shadow` on Linux):

```toml
organization = ["Northwind Bakery Ltd"]
terms = ["org:Contoso Catering", "org:Fabrikam Events", "person:Jane Doe"]
internal_domains = ["northwind.example"]

[files]
images = "block"     # photos and scans are not masked, so don't send them

[policy.always]
secret = "redact"
```

Restart Shadow after changing it (or log out and back in).

### Connect the approved tool

If the tool lets you set an "API base URL" or "OpenAI-compatible endpoint", use:

```text
http://127.0.0.1:8877/gw/openai/v1
```

Otherwise, if the tool follows the system proxy settings, point the system at Shadow's PAC file, which sends only AI traffic through Shadow:

- **macOS:** `networksetup -setautoproxyurl "Wi-Fi" http://127.0.0.1:8877/proxy.pac`
- **Windows:** *Settings → Network → Proxy → Use setup script*, address `http://127.0.0.1:8877/proxy.pac`

Then check that `shadow status` shows the proxy as running. Details in [Proxy mode](/shadow/integrate/proxy-mode/#system-wide-with-a-pac-file).

## Example

A staff member asks the approved assistant to reply to a complaint:

```text
They type:   Reply to Jane Doe (jane.doe@contoso.example, +1 415 555 0134) about order 1182,
             refunded to card ending 1111.
AI sees:     Reply to Jane Doe (jane.doe@contoso.example, +1 415 555 0134) about order 1182,
             refunded to card ending 1111.
They read:   Dear Jane Doe, ...
```

Jane is the person the reply is for, so her name, email and phone are sent as written (and logged): the AI has to know who it is writing to. Anything else in the request, such as a colleague's name or a full card number, would be replaced with a fake and restored in the answer. The order number and the last four card digits are sent as they are; only full card numbers are detected. To hide the customer too, set `targets = "surrogate"` under `[policy.act]` in `shadow.toml`.

## Rollout checklist

- [ ] Choose one approved, API-based AI tool
- [ ] Sign the AI provider's business terms or data processing agreement
- [ ] Install Shadow on each computer that uses the tool
- [ ] Copy the shared `shadow.toml` into each data folder
- [ ] Run `shadow scan` on a real customer email to see what is masked
- [ ] Write and share a one-page AI policy
- [ ] Recheck every few months: new staff, new computers, new tools

## Honest limits

:::caution[Know these before you rely on Shadow]
- **The chatgpt.com and claude.ai websites are not covered.** Nor are AI apps on phones.
- **Photos and scanned documents are not masked.** Text inside images is not read.
- **Unusual details are missed.** Order numbers, loyalty IDs and nicknames are not detected unless you add them as terms.
- **No admin console.** Each computer is set up individually; there is no central dashboard.
- **Not legal advice.** Shadow helps reduce what reaches AI providers. It does not make you compliant with GDPR, PCI DSS or any other law on its own.
:::

## Related

- [How to stop employees pasting customer data into ChatGPT](/shadow/use-cases/stop-pii-leaks-to-chatgpt/)
- [E-commerce and retail](/shadow/industries/ecommerce-retail/) and [nonprofits](/shadow/industries/nonprofit/)
- [Privacy for AI in customer support](/shadow/use-cases/privacy-for-ai-in-customer-support/)
- [Quick start](/shadow/getting-started/quick-start/)
- [How Shadow compares](/shadow/compare/)
