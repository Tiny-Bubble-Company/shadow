---
title: "Secure LLM use in media and entertainment: keep unreleased titles and talent names out of AI prompts"
seoTitle: "Secure LLM use in media: keep unreleased titles private"
description: "Mask unreleased titles, codenames, talent and deal terms before writers, producers and marketers use OpenAI, Anthropic or Gemini. Scripts masked in place."
keywords:
  - studio ai content security
  - unreleased content chatgpt leak
  - protect scripts from ai
  - tpn generative ai
  - entertainment industry llm privacy
  - mask talent names ai
faq:
  - q: "Can writers and producers use AI on unreleased scripts without leaking the title?"
    a: "Shadow replaces the titles, production codenames, character and talent names you list as terms, plus detected people, emails and phone numbers, before the request leaves. The story itself is content and still reaches the model. Whether a script may be sent to an external AI service at all is a decision for the production and its contracts."
  - q: "Does Shadow mask screeners, stills or video?"
    a: "No. Shadow masks text and text inside DOCX, XLSX, PPTX and PDF uploads. Images and video are not masked and there is no OCR; set images = \"block\" so stills and screenshots cannot be uploaded through Shadow."
  - q: "Does Shadow help with TPN or studio content security requirements?"
    a: "It can be a documented technical control for AI tools: it runs on your machines, has no telemetry, decrypts only known AI hosts and keeps the mapping in an encrypted local vault. It does not give you a TPN assessment result or satisfy a studio's requirements on its own."
  - q: "How do we protect a production codename?"
    a: "Add it as a dictionary term, for example shadow terms add \"Project Nightjar\" \"person:Ava Brennan\". The term always maps to the same fake, so long conversations and agents stay consistent, and the answer comes back with the real name."
  - q: "Will AI feedback on a script get worse if names are replaced?"
    a: "Usually not much. Fakes are realistic names and titles from one coherent persona, so dialogue and structure still read naturally. Feedback that depends on a real-world person or franchise will lose that context."
sidebar:
  order: 13
---

Shadow is for studios, broadcasters, publishers, music labels, games companies and agencies whose creative, marketing and business teams use large language models on material that has not been released. It runs on your own laptops and servers, replaces working titles, production codenames, talent and character names, deal parties and contact details with consistent fakes before a request reaches the AI provider, and restores them in the answer. It hides what a project is and who is on it; it does not hide the story.

## What data leaks into AI tools in media and entertainment

| Workflow | What typically ends up in the prompt |
|---|---|
| Script coverage, notes and rewrites | Working title, character names, attached talent, full scenes |
| Marketing and release planning | Unannounced titles, release windows, campaign codenames, partner brands |
| Talent and business affairs | Talent names, agents, fees, contract terms, personal contact details |
| Localisation and subtitles | Dialogue from unreleased episodes, episode titles |
| Games and music | Unannounced game titles, track lists, featured artists, label deals |
| Audience and subscriber analytics | Subscriber names, emails, viewing histories in exports |

The risk is a spoiler or an early announcement, a broken NDA, or a leaked deal, often with direct financial consequences.

## How Shadow protects it

Shadow detects sensitive values in every outbound AI request and uploaded file, swaps them for fakes from a persistent [persona](/shadow/concepts/personas/), keeps the mapping in an [encrypted local vault](/shadow/concepts/vault/), and restores the answer, including streamed responses.

| Data | How Shadow handles it |
|---|---|
| Working titles, production and campaign codenames | Dictionary `terms`, consistent fakes |
| Talent, characters, crew | `person:` terms for known names; the name detector, NER sidecar or Presidio for free text |
| Studios, distributors, brand partners | `org:` terms |
| Agents' and talent emails, phones, addresses | Built-in detectors |
| Scripts, treatments, decks, budgets | DOCX, PPTX and XLSX masked in place; PDF text extracted and masked |
| Subscriber exports | `report` intent with typed tokens such as `[PERSON_0042]` |
| Internal hostnames and API keys | `hostname` and `secret` detectors |

```toml
organization = ["Northlight Pictures LLC"]
terms = [
  "Project Nightjar",                      # production codename
  "term:The Long Winter",                  # working title
  "person:Ava Brennan", "person:Marcus Hale", # attached talent
  "person:Captain Ilsa Varga",             # character
  "org:Globex Streaming",                  # distributor
]
internal_domains = ["northlight.example"]

[files]
pdf = "text"
images = "block"     # stills, storyboards and screenshots are not masked
```

Add names as they are attached to a project:

```sh
shadow terms add "person:Jun Park" "term:Episode 104 - Glasshouse"
```

:::caution[What masking does not hide]
- **The story, dialogue and plot** are content. A distinctive premise can identify a project even with every name replaced.
- **Images, video and audio** are not masked. No OCR.
- **Names not in your term list** depend on detection. Free-text names are found by the name detector and, with better recall, the NER sidecar or Presidio, but detection is best effort. Run `shadow scan` on a sample script before relying on it.
:::

## Regulations and frameworks

| Requirement | How Shadow helps |
|---|---|
| Studio and production content security programmes, TPN | A technical control for AI tools: local, no telemetry, minimal interception, encrypted vault. Not an assessment or a substitute for studio requirements |
| NDAs and talent agreements | Keeps talent, title and deal-party names out of third-party AI services |
| GDPR and US state privacy laws for subscriber data | Data minimisation and pseudonymisation before data leaves; pseudonymised data remains personal data for you (Recital 26) |
| ISO/IEC 27001, SOC 2 | Evidence of a control on AI data flows |

:::caution[Not legal advice]
This page describes technical capabilities, not legal conclusions. Shadow does not satisfy content security, contractual or privacy obligations on its own. Check production contracts and studio policies before sending unreleased material to any AI service.
:::

## Example workflows

### Script notes on a scene

What the writer sends (all data fictional):

```text
Give notes on this scene from The Long Winter (Project Nightjar). Ava Brennan plays
Captain Ilsa Varga; we want it tighter before Globex Streaming sees the cut.
```

What the provider sees (your fakes will differ):

```text
Give notes on this scene from Lumeth Draane (Velora Quilent). Ying Chen plays
Captain Hui Yang; we want it tighter before Zhilan Media Co., Ltd. sees the cut.
```

The notes come back referring to Captain Ilsa Varga and The Long Winter. The scene text itself is unchanged.

### Marketing deck review

A marketer uploads a PPTX for an unannounced campaign. Title, codename, partner brand and talent names in the slide text runs are replaced, the deck stays a valid presentation, and suggestions come back with the real names. Images on the slides are not masked, so with `images = "block"` any embedded screenshot uploaded separately is refused.

### Subscriber churn analysis

An analyst attaches a CSV of subscribers and viewing data and asks "Analyse these subscriber records: which titles drive cancellations?". The request reads as a report, so the `report` intent tokenises subscriber names and emails; title names in your term list get consistent fakes. The analysis is restored locally.

## Deployment patterns

| Pattern | When to use it |
|---|---|
| **Endpoint install** for writers, producers and marketers: `shadow install --apply` | API-backed chat clients and desktop apps that honour the system proxy or PAC file |
| **Gateway** for internal tools: `http://127.0.0.1:8877/gw/anthropic` and similar | Coverage, localisation and marketing assistants you build |
| **Sidecar** in Kubernetes | Product features such as recommendations or support bots. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |

Consumer web chat apps such as chatgpt.com are not built-in intercepted hosts, so give teams an approved API-backed client. There is no central admin console yet; distribute `shadow.toml` and term lists through your existing tooling. Shared vaults for teams working on one production are on the [roadmap](/shadow/project/roadmap/); today you copy `persona.json`, `vault.bin` and `vault.key`.

## Related

- [Redact documents before uploading to AI](/shadow/use-cases/redact-documents-before-uploading-to-ai/) and [shadow AI governance](/shadow/use-cases/shadow-ai-governance/)
- [Consulting and professional services](/shadow/industries/consulting-professional-services/) and [legal](/shadow/industries/legal/)
- [File masking](/shadow/concepts/file-masking/), [detection](/shadow/concepts/detection/), [security model](/shadow/reference/security/)
- Detection quality: [benchmarks](/shadow/benchmarks/). Other approaches: [compare](/shadow/compare/)
