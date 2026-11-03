---
title: "What is shadow AI? Risks, examples and how to detect it"
seoTitle: "What is shadow AI? Risks and how to detect it"
description: "Shadow AI is AI use your IT team doesn't know about. What it is, the risks, real examples and how to detect it without blocking work."
editUrl: false
keywords:
  - shadow ai
  - what is shadow ai
  - shadow ai risks
  - shadow ai examples
  - detect shadow ai
  - shadow ai vs shadow it
faq:
  - q: "What is shadow AI?"
    a: "Shadow AI is the use of AI tools at work without the knowledge or approval of IT and security: personal ChatGPT or Claude accounts, AI browser extensions, coding agents run with a personal API key, or AI features switched on inside SaaS apps. The risk is not the AI itself but company and customer data reaching providers outside any contract, review or control."
  - q: "How is shadow AI different from shadow IT?"
    a: "Shadow IT is any unapproved software or service. Shadow AI is the part of it that sends data to an AI model. It spreads faster, because most AI tools need no install and no procurement, and every prompt can carry data out: a paste, a file upload or a whole code repository read by an agent."
  - q: "Is shadow AI illegal?"
    a: "Using an AI tool is not illegal in itself. Sending personal data to a provider without a legal basis, a data processing agreement or the transparency GDPR requires can be, and so can breaching confidentiality obligations to customers. That is why shadow AI is a compliance problem as well as a security one."
  - q: "How do you detect shadow AI?"
    a: "Look at the traffic. Network and DNS logs, a CASB or SASE product, SSO and expense reports show which AI services are used. An endpoint layer such as Shadow Core sees each request on the laptop, including the app that sent it and what sensitive data it carried, and Shadow Hub lists newly seen AI services and who uses them."
  - q: "Should we ban AI tools to stop shadow AI?"
    a: "Bans rarely hold. People move to phones and personal laptops, where you see nothing. Most organisations do better with an approved set of tools, a short acceptable use policy, and a technical control that keeps sensitive data out of the prompts."
---

**Shadow AI is the use of AI tools at work without IT or security knowing about it or approving it.** A support agent pastes a customer thread into a personal ChatGPT account, an engineer runs a coding agent with their own API key, a marketer installs an AI browser extension that reads every page. None of it is malicious. All of it can send company and customer data to a provider your organisation has no contract with, no review of and no visibility into.

This guide explains what counts as shadow AI, why it happens, the risks, real examples, how to detect it, and how to govern it without banning AI.

## Shadow AI vs shadow IT

Shadow IT is any software or service used without IT's approval: a personal Dropbox, an unapproved project tool. Shadow AI is the part of shadow IT that sends data to an AI model, and it differs in three ways:

| | Shadow IT | Shadow AI |
|---|---|---|
| Adoption | Usually needs a signup, sometimes a purchase | A browser tab or a free account; often already built into tools you approved |
| Data flow | Files stored somewhere | Every prompt carries data out: pastes, uploads, whole repositories read by agents |
| What the provider does with it | Stores it | May log it, keep it for abuse review, or use it for training, depending on plan and settings |

The last row matters. On consumer plans, providers' terms commonly allow conversations to be used to improve their models unless the user opts out, and some content is kept for review even then. A business agreement usually changes that, which is exactly what an unapproved personal account lacks.

## Why shadow AI happens

People use AI because it saves them hours. They use *unapproved* AI when:

- **there is no approved tool**, or approval takes months;
- **the approved tool is too restricted** to do the real work, for example a DLP rule that blocks any prompt mentioning a customer;
- **AI arrives inside tools they already use**: an AI assistant switched on in a SaaS app, an editor plugin, a meeting recorder;
- **nobody said what is allowed.** Without a policy, using a free account feels no different from using a search engine.

Shadow AI is a sign of demand, not of bad intent. That shapes how you fix it.

## The risks of shadow AI

**Data leaving your control.** Customer names, emails, contracts, health and financial details, source code, credentials and internal hostnames end up with a provider outside any data processing agreement. Once sent, you cannot recall it.

**Training and retention.** Depending on the plan and the user's settings, prompts may be kept, reviewed by people, or used to train future models.

**Secrets in prompts.** Stack traces, config files and `.env` contents pasted for debugging carry API keys and database passwords. Coding agents read whole repositories, including files nobody meant to share.

**Compliance exposure.** GDPR expects a legal basis, a processor agreement and transparency for personal data sent to a third party. Sector rules (HIPAA, financial secrecy, client confidentiality in law firms) add their own limits. The EU AI Act adds AI literacy obligations for organisations that use AI. Unapproved tools fail all of these by default.

**No audit trail.** If a customer or regulator asks what was sent to which AI service, shadow AI leaves you with no answer.

**Unvetted tools.** Browser extensions and small AI apps can request broad permissions, read every page and send it to servers you have never assessed.

## Shadow AI examples

- **Pasting customer data into a chatbot.** "Reply politely to this complaint" followed by the whole ticket: name, email, order number, address. The most common case by far.
- **Source code in a personal account.** In 2023 Samsung engineers pasted internal source code and meeting notes into ChatGPT; the company then restricted generative AI tools on work devices. Many other companies issued similar restrictions that year.
- **Coding agents with personal keys.** An engineer runs an agent against the company repository with their own API key. Every file it reads goes to the provider under a personal account.
- **Documents for summarising.** Contracts, board papers or HR cases uploaded to "summarise this".
- **AI browser extensions.** Writing assistants and "chat with this page" extensions that read the content of internal apps.
- **AI features in approved SaaS.** A meeting tool's AI notes or a CRM's assistant, switched on by a user and sending data to a model provider you never reviewed.

## How to detect shadow AI

You can't govern what you can't see. Detection works best from the traffic, not from surveys.

1. **Network and DNS logs.** Look for requests to AI providers and chat apps. Shadow keeps a [catalog of 288 AI services](/shadow/reference/providers/) (model APIs, inference clouds, routers, coding agents and chat apps) that you can use as a list of domains to look for.
2. **CASB, SASE or secure web gateway reports.** Most have an "AI" or "generative AI" category that shows usage by user and app.
3. **SSO and expense reports.** Sign-ins with company accounts and AI subscriptions on expense claims show which tools people pay for.
4. **The endpoint.** Network logs show *where* traffic went, not *what* was in it. An endpoint layer sees each AI request on the laptop itself: which app sent it, which AI service received it, and whether it carried names, card numbers, national IDs or secrets.

[Shadow Core](/shadow/getting-started/quick-start/) is that endpoint layer, open source and free. It recognises AI traffic from browsers, desktop apps, SDKs and coding agents, replaces sensitive values with realistic fakes before the request leaves the machine, and shows each request in a local dashboard. [Shadow Hub](/shadow/hub/) collects that activity from every enrolled laptop, lists **newly seen AI services** each week and who uses them, and flags services Shadow recognises but cannot protect yet as *seen, not protected*.

## How to govern shadow AI without banning AI

Blocking every AI site is tempting and rarely works: the work moves to phones and personal laptops, where you see nothing. A model that holds up:

1. **Discover** which AI tools are in use, with the methods above.
2. **Decide** which tools are approved, sign business agreements with those providers, and write a short [AI acceptable use policy](/shadow/guides/ai-acceptable-use-policy-template/).
3. **Provide** approved tools that are good enough for real work, so people don't need the unapproved ones.
4. **Protect** what leaves. Shadow masks names, emails, phone numbers, card numbers, IBANs, national IDs, addresses and secrets in every AI request and restores the real values in the answer, so people can use AI on real work without the data reaching the provider.
5. **Monitor and review.** Watch new AI services and policy violations, and update the approved list every quarter.

The [shadow AI governance](/shadow/use-cases/shadow-ai-governance/) page goes through the technical rollout step by step. If you are still deciding between blocking and governing, read [Block ChatGPT or govern it?](/shadow/guides/block-chatgpt-or-govern-it/).

## A five-minute start

On a Mac or Linux laptop:

```sh
curl -fsSL https://hub.getshadow.dev/core/install.sh | sh
```

Then click the menu-bar icon and choose **Protect all apps and browsers**. Paste a prompt with a test card number such as `4111 1111 1111 1111` into ChatGPT and open the dashboard (`shadow dashboard`) to see what the AI received instead. For a team view, [start Shadow Hub](https://hub.getshadow.dev/signup): free for up to 3 people.

:::caution[Not legal advice]
This guide describes risks and technical controls. It does not make your AI use compliant with any law or standard. Involve your security, privacy and legal teams.
:::

## Related

- [Free AI acceptable use policy template](/shadow/guides/ai-acceptable-use-policy-template/)
- [How to stop employees pasting customer data into ChatGPT](/shadow/use-cases/stop-pii-leaks-to-chatgpt/)
- [Shadow AI governance](/shadow/use-cases/shadow-ai-governance/)
- [Shadow Hub: AI visibility for teams](/shadow/hub/)
