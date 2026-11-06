---
title: "Is ChatGPT Safe for Company Data? What Happens to Your Prompts"
seoTitle: "Is ChatGPT safe for company data?"
description: "Whether ChatGPT is safe for work data depends on the plan. Free, Plus and Pro chats can train models by default; Business, Enterprise and the API do not. Here is what is stored, for how long, and how to keep sensitive data out."
publishedDate: 2026-08-25
updatedDate: 2026-11-06
topics:
  - AI Privacy
  - Enterprise AI
  - ChatGPT
  - Data Security
author: Shadow Research Team
readingTime: 9
keywords:
  - is chatgpt safe
  - is chatgpt safe for confidential information
  - is chatgpt safe to use
  - is chatgpt safe to use for work
  - chatgpt data privacy
  - chatgpt enterprise security
faq:
  - q: "Does ChatGPT train on my data?"
    a: 'On personal Free, Plus and Pro accounts, yes by default, unless you turn off "Improve the model for everyone" in Settings, Data controls. Business, Enterprise, Edu, Healthcare and API data is not used for training by default.'
  - q: "Is ChatGPT Enterprise safe for confidential data?"
    a: "It has SOC 2 Type 2, admin-controlled retention, SSO and no training by default, per OpenAI. Whether it is safe for your data depends on your contracts and data classes; regulated data needs a specific agreement."
  - q: "Does temporary chat keep my data private?"
    a: "Temporary chats are not used for training and do not appear in history, but OpenAI may keep them for up to 30 days for safety purposes."
  - q: "Can my employer see my ChatGPT chats?"
    a: "On ChatGPT Business, workspace admins can view, export and delete members' conversations; on Enterprise, admins can access an audit log through the Compliance API. On a personal account, your employer cannot see them, which is exactly why personal accounts are a data-leak risk."
  - q: "Is it safe to paste API keys or passwords into ChatGPT?"
    a: "No. Treat any secret that reached an AI tool as exposed and rotate it. Shadow detects common secret formats and masks them before they leave your machine."
cta:
  label: "Try Shadow free"
  href: /getting-started/quick-start/
---

**ChatGPT can be safe for company data, but only on the right plan and with the right settings.** On personal Free, Plus and Pro accounts, conversations can be used to train OpenAI models by default unless the user turns that off. ChatGPT Business, Enterprise and the API are not used for training by default. On every plan, the safest rule is the same: keep customer data, credentials and secrets out of the prompt, or mask them before they leave the laptop.

## The short answer by plan

| Plan | Used to train models by default? | Retention (per OpenAI) | Who controls it |
|---|---|---|---|
| ChatGPT Free, Plus, Pro (personal) | Yes, unless "Improve the model for everyone" is turned off | Chats stay in history until deleted | The individual user |
| Temporary chat (any personal plan) | No | Up to 30 days for safety purposes | The individual user |
| ChatGPT Business | No | Admin-controlled; deleted chats removed within 30 days unless law requires longer | Workspace admin |
| ChatGPT Enterprise / Edu / Healthcare | No | Admin-controlled; deleted chats removed within 30 days unless legally required | Workspace admin |
| OpenAI API | No | Up to 30 days for abuse monitoring, except listed endpoints; zero data retention available for eligible use cases | The developer or company |

Sources: [OpenAI enterprise privacy](https://openai.com/enterprise-privacy/) and [Data controls in ChatGPT](https://help.openai.com/en/articles/7730893-data-controls-faq).

## What actually happens to a prompt

1. The text you type (and any file you upload) is sent to OpenAI's servers over TLS.
2. It is processed by the model and stored as part of the conversation, unless it is a temporary chat.
3. Depending on plan and settings, it may be used to improve models. Giving thumbs-up or thumbs-down feedback can make the whole conversation eligible for training, even after opting out, according to OpenAI's FAQ.
4. On business plans and the API, OpenAI says data may be run through automated safety classifiers, and authorized staff or contractors may access it for abuse investigation, support or legal reasons.

None of this is unusual for a cloud service. The risk is not that OpenAI is careless; it is that **once a value leaves your machine, you no longer control where it is stored, who can be compelled to produce it, or how long it lives.**

## Is ChatGPT safe for confidential information?

Treat it like any external SaaS vendor:

- **Personal accounts:** not suitable for customer data, health data, payment data, credentials, unreleased financials or source code with secrets. Even with training turned off, the data is stored in a personal account the company cannot see or delete.
- **ChatGPT Business or Enterprise:** acceptable for much internal work once legal has reviewed the DPA and retention settings. Regulated data (PHI, cardholder data) still needs a specific agreement, such as a BAA, and a policy.
- **API:** good controls, but your developers decide what goes into each request. One log line or support ticket pasted into a prompt can carry hundreds of customer emails.

## The real risk at most companies: shadow AI

Most leaks do not come from the official ChatGPT Enterprise rollout. They come from people using personal accounts, browser extensions and coding assistants the company never approved. See [what is shadow AI](/research/what-is-shadow-ai/).

## How to use ChatGPT safely at work

1. **Pick a business plan** (Business, Enterprise or the API) so training is off by default and an admin controls retention.
2. **Write an AI acceptable use policy** that lists approved tools and data that may never be shared. Start from our [free template](/research/ai-acceptable-use-policy-template/).
3. **Turn off training on personal accounts** if people use them at all, and prefer temporary chats for one-off questions.
4. **Mask sensitive data before it leaves the device.** This is the one control that works on every plan and every tool.
5. **Get visibility.** Know which AI tools your team uses and whether sensitive data reached them.

## Mask data before it reaches ChatGPT with Shadow

[Shadow](/) is a free, open-source privacy layer that runs on your machine. It detects names, emails, phone numbers, card numbers, IBANs, secrets and your own organisation terms in prompts to ChatGPT, Claude, Gemini and AI APIs, replaces them with realistic, reversible fakes, and restores the real values in the answer. In the ChatGPT app it masks only the prompt text, and TLS is terminated only on your own machine ([how proxy mode works](/integrate/proxy-mode/)).

On a Mac or Linux laptop:

```sh
curl -fsSL https://hub.getshadow.dev/core/install.sh | sh
```

For teams, [Shadow Hub](/hub/) shows which AI tools your people use and what Shadow masked or blocked, per user and per tool, without collecting prompt text. Hub is free for up to 3 users, then $3 per user per month up to 500 users. See [Pricing](/pricing/).

---

**Disclaimer**: This guide describes risks and technical controls. It does not make your AI use compliant with any law or standard. Involve your security, privacy and legal teams.

## Related research

- [Private ChatGPT: how to use AI without sharing your data](/research/private-chatgpt/)
- [Does Cursor train on my code?](/research/does-cursor-train-on-my-code/)
- [What is shadow AI?](/research/what-is-shadow-ai/)
- [Free AI acceptable use policy template](/research/ai-acceptable-use-policy-template/)
