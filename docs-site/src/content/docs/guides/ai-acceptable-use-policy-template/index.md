---
title: "Free AI acceptable use policy template (with enforcement)"
seoTitle: "Free AI acceptable use policy template"
description: "A free AI acceptable use policy template for companies, with the controls that make each rule enforceable rather than a document nobody reads."
editUrl: false
keywords:
  - ai acceptable use policy
  - ai acceptable use policy template
  - ai usage policy template
  - generative ai policy template
  - chatgpt policy for employees
  - company ai policy
faq:
  - q: "What is an AI acceptable use policy?"
    a: "It is a short company policy that says which AI tools employees may use, what data they may and may not put into them, how the rules are enforced, and how to ask for a new tool. It is the AI equivalent of an IT acceptable use policy."
  - q: "What should an AI acceptable use policy include?"
    a: "Who it covers, the approved and prohibited tools, the data that must never be entered, rules for checking AI output, how the rules are enforced and monitored, how to request exceptions and new tools, and what happens when the policy is broken."
  - q: "Can we use this template for free?"
    a: "Yes. Copy it, change it and use it in your organisation without attribution. Have your legal and privacy teams review it before you adopt it."
  - q: "How do we enforce an AI policy?"
    a: "Pair each rule with a control. Approved tools come with business agreements; sensitive data is masked or blocked before it leaves the laptop, for example with Shadow Core; usage is monitored per team with a dashboard such as Shadow Hub; and the approved list is reviewed every quarter."
  - q: "Should the policy ban ChatGPT?"
    a: "Usually not. A ban without an approved alternative pushes usage onto personal devices. Most policies approve a set of business-plan tools, require that sensitive data be masked, and prohibit only personal accounts and unvetted tools for company data."
---

**An AI acceptable use policy tells employees which AI tools they may use, what data they must never put into them, and how the rules are enforced.** Below is a free template you can copy and adapt. Unlike most templates, each rule is paired with the control that enforces it, because a policy nobody can check becomes a document nobody reads.

[Download the template as Markdown](/shadow/templates/ai-acceptable-use-policy-template.md), or copy it from this page.

## How to use this template

1. Replace everything in `[square brackets]` with your organisation's details.
2. Fill in the approved tools table with what you actually provide, and only list tools you have a business agreement for.
3. Decide how you will enforce each rule (see [Making it enforceable](#making-it-enforceable)) before you publish it, so the policy doesn't promise controls you don't have.
4. Have legal, privacy and HR review it, then publish it where people will find it and walk teams through it. Owners: usually the CISO or IT lead, with legal and the data protection officer.

## The template

```markdown
# [Company] AI acceptable use policy

Version [1.0], effective [date]. Owner: [role]. Review: every [3] months.

## 1. Purpose

AI tools help us work faster. This policy lets everyone use them while
keeping customer, employee and company data safe and meeting our legal
and contractual obligations.

## 2. Scope

This policy covers all employees, contractors and interns, on company
and personal devices, whenever they use AI for [Company] work. "AI tools"
means chat assistants, coding assistants and agents, AI features in other
software, browser extensions and any service that sends data to an AI model.

## 3. Approved tools

You may use these tools for company work, with the accounts listed:

| Tool | Use for | Account | Data allowed |
|------|---------|---------|--------------|
| [ChatGPT Business / Enterprise] | Writing, analysis | Company SSO | Internal, with masking on |
| [Claude Code / Codex / Cursor] | Software development | Company key | Code, with masking on |
| [Internal assistant] | [Use] | Company SSO | [Level] |

Approved tools run with [Shadow] on company laptops, which replaces
sensitive values with placeholders before a request leaves the laptop.
Do not turn it off.

## 4. Prohibited

- Personal AI accounts (free or paid) for company work.
- AI tools, extensions or plugins not on the approved list.
- Entering the data in section 5 into any AI tool unless it is masked.
- Using AI to make final decisions about people (hiring, performance,
  credit, access) without human review.
- Presenting AI output as your own professional judgement without
  checking it.

## 5. Data you must never enter unmasked

- Customer and employee personal data: names, emails, phone numbers,
  addresses, dates of birth, national ID numbers.
- Payment and financial data: card numbers, bank accounts and IBANs.
- Health, legal and other special category data.
- Credentials: passwords, API keys, tokens, private keys, connection
  strings.
- Confidential business information: unreleased financials, contracts,
  M&A, board papers, [project code names].
- Source code from [restricted repositories].

If you are unsure, treat it as sensitive.

## 6. Using AI output

- Check facts, figures, citations and code before you use them.
- Don't paste AI output into customer communications, contracts or
  production code without review.
- Label AI-generated content where [customers / law] expect it.

## 7. Monitoring

[Company] records which AI services are used from company devices and
when sensitive data is masked or blocked, to protect data and improve
this policy. [Sharing level: counts only / prompts as the AI received
them / full detail.] Employees can see what is shared on their own
machine. We do not use this data for [performance reviews].

## 8. New tools and exceptions

Request a new tool or an exception at [link / channel]. [Security] aims
to answer within [5] working days. Until approved, don't use the tool
for company data.

## 9. Incidents

If you entered sensitive data into an unapproved tool, or think a tool
leaked data, tell [security@company] straight away. Reporting quickly
is never a disciplinary matter in itself.

## 10. Breaches

Breaking this policy may lead to loss of access and to disciplinary
action under [policy name].

## 11. Training

Everyone who uses AI for work completes [AI training] when they join and
[every year].
```

## Making it enforceable

Each section of the policy needs a control behind it. Here is what to pair with what.

| Policy section | Control that enforces it | How Shadow helps |
|---|---|---|
| 3. Approved tools | Business agreements and company accounts with each provider | Masks data in approved tools, so more tools are safe to approve |
| 4. Prohibited tools | Web filter or CASB for unsanctioned AI sites; extension allow-list | [Shadow Hub](/shadow/hub/) lists newly seen AI services and who uses them |
| 5. Data never entered | Technical masking or blocking before data leaves the laptop | Shadow Core replaces names, emails, phones, card numbers, IBANs, national IDs, addresses and secrets with realistic fakes and restores them in the answer; it can block image uploads |
| 6. Using AI output | Review steps in existing processes (code review, legal sign-off) | Not a Shadow feature |
| 7. Monitoring | A dashboard of AI usage and incidents, with a clear sharing level | Hub shows masked and blocked requests per person, app and AI service, at the sharing level you choose |
| 8. New tools | A request form and an owner | Hub's newly seen services show which requests to expect |
| 9. Incidents | An incident channel and a record of what was sent | Hub incidents show what was caught and what the AI received instead |

### Rule 5 is where most policies fail

People don't paste customer data into a chatbot to break a rule; they do it because it's the fastest way to get the job done. "Never enter personal data" asks them to stop doing the useful part. Masking keeps the useful part: the model still sees *a* name, *an* email and *a* card number, just not the real ones, and the real values come back in the answer. The rule becomes something the laptop enforces rather than something people have to remember.

To try it on one laptop:

```sh
curl -fsSL https://hub.getshadow.dev/core/install.sh | sh
```

Then turn on **Protect all apps and browsers** in the menu-bar app. For a whole team, [start Shadow Hub](https://hub.getshadow.dev/signup): free for up to 3 people, then per monitored user.

## Tips for a policy people follow

- **Keep it short.** Two pages people read beat twenty they don't.
- **Say yes first.** Lead with what is approved. A policy that only lists bans reads as "don't use AI", and people will use it anyway.
- **Name real tools.** "Approved generative AI solutions" means nothing to a support agent; "ChatGPT Business with your company login" does.
- **Be open about monitoring.** Tell people what is recorded and what isn't. Covert monitoring of employees is restricted in many countries, and in some (Germany, for example) a works council has to agree first.
- **Review it every quarter.** AI tools change monthly. Use what you have seen (new services, blocked requests, exception requests) to update the approved list.

## Laws and frameworks to check against

- **GDPR**: a legal basis and a data processing agreement for personal data sent to an AI provider; data protection by design (Article 25) and appropriate security (Article 32).
- **EU AI Act**: AI literacy for staff who use AI (Article 4) and duties for deployers of high-risk systems. Check the current text: the 2026 Digital Omnibus amended several provisions and timelines.
- **ISO/IEC 42001** and the **NIST AI Risk Management Framework** if you follow a management-system or risk framework.
- **Sector rules** such as HIPAA, financial secrecy, or client confidentiality for law firms.

:::caution[Not legal advice]
This template is a starting point, not legal advice. Have your legal, privacy and HR teams review it for your organisation and jurisdiction before you adopt it.
:::

## Related

- [What is shadow AI?](/shadow/guides/what-is-shadow-ai/)
- [Block ChatGPT or govern it?](/shadow/guides/block-chatgpt-or-govern-it/)
- [How to stop employees pasting customer data into ChatGPT](/shadow/use-cases/stop-pii-leaks-to-chatgpt/)
- [Shadow Hub](/shadow/hub/)
