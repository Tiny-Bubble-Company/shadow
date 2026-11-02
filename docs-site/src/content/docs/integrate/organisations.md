---
title: Teams and organisations
seoTitle: "Shadow for teams: see AI data leaks across your org"
description: "Enrol employees' machines with one command and see what sensitive data your workforce sends to AI, per person, per machine and across the organisation."
faq:
  - q: "How do employees join?"
    a: "An admin creates an invite in Shadow Hub and shares the join link. The employee runs the one-line command on the join page: it installs Shadow, enrols the machine with the organisation and turns on protection, including the system proxy."
  - q: "What does the organisation see?"
    a: "It depends on the sharing level the organisation sets: full (incidents with prompts and the values caught), sanitised (prompts only as the AI received them, real values never leave the laptop) or counts only. The level is applied on the laptop before anything is sent."
  - q: "Do employees know their machine is managed?"
    a: "Yes. The menu-bar app, the local dashboard and `shadow status` all say which organisation manages the machine and what is shared with its admins."
  - q: "Is Shadow Hub open source?"
    a: "No. Shadow, which protects each machine, is open source under Apache-2.0. Shadow Hub, the central dashboard for organisations, is a commercial service, also available self-hosted. Contact support@getshadow.dev."
---

Shadow protects one machine at a time. **Shadow Hub** brings those machines together for an organisation: admins invite employees, every enrolled laptop and server reports what Shadow protected, and the hub shows it per person, per machine and across the workforce, including what kinds of data leak most and to which AI services.

Shadow itself (everything in this documentation) stays open source and works on its own. Shadow Hub is a separate, commercial product. To try it, write to [support@getshadow.dev](mailto:support@getshadow.dev).

## Joining an organisation

An admin creates an invite in Shadow Hub and shares the join link. On the join page the employee sees what the organisation will be able to see, and a one-line command:

```sh
# macOS and Linux
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | SHADOW_ENROLL='https://hub.example.com/join/inv_...' sh
```

```powershell
# Windows
$env:SHADOW_ENROLL='https://hub.example.com/join/inv_...'; irm https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.ps1 | iex
```

With `SHADOW_ENROLL` set, the installer:

1. installs Shadow (and the menu-bar app on macOS);
2. enrols the machine with the organisation (`shadow enroll <join link>`);
3. installs the background service, trusts the local CA and turns on the [system proxy](/shadow/integrate/proxy-mode/#system-wide-with-the-system-proxy) (`shadow install --apply --system-proxy`), so browsers and chat apps are covered too.

On a machine where Shadow is already installed, enrolling is one command:

```sh
shadow enroll 'https://hub.example.com/join/inv_...'
shadow status        # shows the organisation and what is shared
shadow unenroll      # stop reporting from this machine
```

## What is shared

The organisation chooses a sharing level in Shadow Hub. Each machine fetches it with every upload and trims what it sends **before** anything leaves the machine; the hub enforces it again on arrival.

| Level | Sent to the organisation |
|---|---|
| **Full** | Activity counts, plus each incident's prompt as written and as the AI received it, the values caught and what replaced them, the app and process, and the machine |
| **Sanitised** | Activity counts, plus each incident's prompt only as the AI received it (fakes). Real values never leave the laptop |
| **Counts** | Activity counts and categories: which AI services, which apps, how many values of each type, what was blocked |

Every level includes the machine's hostname, operating system and user name, and the person it is enrolled to. Traffic that is not going to an AI service is never recorded, at any level.

People using an enrolled machine are told, in the menu-bar app ("Shadow is protecting this Mac · managed by Acme"), on the local dashboard and in `shadow status`, which organisation manages it and what is shared. Depending on where you operate, monitoring employees' use of AI may require telling them in writing or consulting a works council. Choose the sharing level with that in mind.

## How it works

- Enrolment stores the hub address and a device token in `<data dir>/hub.json`, readable only by the user (`0600`).
- While Shadow runs, it uploads new activity events every few seconds over HTTPS, with the device token. Uploads are idempotent, so a lost connection never duplicates or loses events; they resume from where they stopped.
- Incident details (prompt text) are kept in Shadow's memory for the last 200 incidents. An enrolled machine uploads them within seconds, so the hub keeps them under the organisation's retention policy.
- The local dashboard at `http://127.0.0.1:8877/shadow/ui` keeps working as before.
