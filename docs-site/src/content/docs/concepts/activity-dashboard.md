---
title: Activity dashboard
seoTitle: "AI activity dashboard: see what your tools send · Shadow"
description: "See every prompt Shadow masked or blocked: the app and process that sent it, the AI it went to, each value caught and what replaced it. Local only."
keywords:
  - AI privacy dashboard
  - monitor ChatGPT data leaks
  - PII masking activity
  - Claude Code privacy
  - AI data loss prevention
  - shadow top
  - menu bar app
  - see which prompt leaked data
faq:
  - q: Can I see which prompt caused an incident?
    a: Yes. Open an incident in the dashboard, the menu-bar app or shadow incidents <ID> to see the prompt as written and as the AI received it, each value caught and what replaced it, the model, and the app and process that sent it.
  - q: Does Shadow store my prompts or the values it masked?
    a: "Only encrypted. The activity log on disk holds counts and categories only. The prompt text and values of the most recent incidents (200 by default) are saved in details.bin, encrypted with your vault key like the vault itself, so you can review them after a restart. Set activity.persist_details = false to keep them in memory only, or activity.details = false to keep none."
  - q: Can someone else on my network see the dashboard?
    a: No. The dashboard and the activity API only answer connections from the same machine, and only when the Host header names a loopback address. You can opt in to remote access with activity.allow_remote.
  - q: How do I turn activity recording off?
    a: Set enabled = false in the [activity] section of shadow.toml. To keep the dashboard but write nothing to disk, set persist = false.
  - q: Where is the activity history stored?
    a: In activity/activity.jsonl inside the Shadow data directory, created with mode 0600 in a 0700 directory. It rotates at 16 MB and drops events older than 30 days by default.
---

Shadow's activity dashboard shows how much AI traffic went through Shadow, what it masked and what it blocked, by data type, destination and app, in a web page, a menu-bar app or `shadow top`. Open any incident to see the prompt that caused it and where it came from. The log on disk holds counts only; prompt text stays in memory.

Shadow works quietly in the background, so it can be hard to tell that it is doing anything. The activity feed answers three questions at a glance:

- **How much AI traffic went through Shadow?** Requests monitored, by destination (OpenAI, Anthropic, ChatGPT, Google Gemini, Azure OpenAI, ...) and by the app that sent them (Claude Code, Codex CLI, the OpenAI Python SDK, Google Chrome, Safari, curl, ...). Each destination is labelled as a model API, an inference cloud or an AI chat app. Only traffic to [intercepted AI hosts](/shadow/reference/providers/) is counted, including chat apps in a browser when the [system proxy](/shadow/integrate/proxy-mode/#system-wide-with-the-system-proxy) is on; nothing else you browse is recorded.
- **What did it protect?** Values masked before they left the machine, by data type (email addresses, names, secrets and API keys, hostnames, card numbers, ...), and how many were restored in the answers.
- **What went by unprotected?** Connections to AI services Shadow recognises but cannot mask (for example Cursor or Windsurf, which use binary protocols), listed under *Seen, not protected* so nothing AI-bound goes unnoticed. Counts only; their traffic is never decrypted.
- **What did it stop?** Requests blocked by policy, such as an image upload when `files.images = "block"`, with the reason.
- **Which prompt was it?** For each incident, the prompt as written and as the AI received it, every value caught, and the app, process and conversation it came from.

You can look at the same feed in three places: a [web dashboard](#web-dashboard), a [terminal view](#terminal-shadow-top) and a [menu-bar app](#menu-bar-and-tray-app).

## What is recorded, and what never is

Every request that Shadow translates produces one small event:

| Field | Example |
|---|---|
| Time | `2026-10-31T14:02:11Z` |
| Destination | provider `anthropic`, host `api.anthropic.com` |
| App | `Claude Code` (from the `User-Agent` header) |
| Endpoint kind | `chat`, `embeddings`, `files` or `other` |
| Route | HTTPS proxy or `/gw` gateway |
| Action | `masked`, `blocked` or `clean` |
| Counts per data type | `{"email": 2, "secret": 1}` |
| Detectors used | `email`, `secret`, `dictionary`, ... |
| Values restored in the response | `3` |
| Block reason | a fixed category such as `image_blocked` |
| Request intent | `code`, `report`, `act` or `converse` |
| Size, status, latency, streamed | `5.1 KB`, `200`, `840 ms`, `true` |

**Never written to disk:** the real values, the fakes or tokens sent in their place, prompts, responses, request paths and query strings, headers, API keys, file names, and the text of block messages (which can contain a file name). Unknown apps are named by the first product token of their `User-Agent` only (`MyTool/1.2 (build-host; jane@acme.com)` becomes `MyTool`), so the comment part, where hostnames and user names tend to appear, is dropped. Responses are never kept anywhere.

### Incident details

So you can see what caused an incident, Shadow also keeps these for the most recent masked or blocked requests:

| Detail | Example |
|---|---|
| Prompt text, as written and as sent | each chat message, with the changed values highlighted |
| Values caught | `email`: `maria.lopez@example.com` → `mei.lin@kaiyuan-health.cn` |
| Model and path | `gpt-4o-mini`, `POST /v1/chat/completions` |
| App and process | `Claude Code`, process `claude` (pid 4211), full `User-Agent` |
| Conversation | session, conversation or thread ids the app sent as headers |
| Block message | `image blocked by policy` |

Details are kept for the last 200 incidents (`details_limit`) and are never written to the log file. They are saved to `<data_dir>/activity/details.bin` (mode 0600), encrypted with ChaCha20-Poly1305 under your `vault.key`, so they survive a restart; set `persist_details = false` to keep them in memory only. Clean requests keep nothing. The process is found from the connection's local port (`lsof` on macOS and Linux, `netstat` on Windows), so it is shown for apps on the same machine only. Tool and function definitions, images and file payloads are left out of the prompt view. Set `details = false` to keep counts only.

Blind tunnels to hosts that Shadow does not intercept are not recorded at all: Shadow cannot see inside them.

## Web dashboard

With Shadow running, open:

```sh
shadow dashboard
# or browse to http://127.0.0.1:8877/shadow/ui
```

The page shows the headline numbers, a traffic timeline (masked, clean and blocked requests), breakdowns by data type, destination and app, and the most recent incidents. The **Incidents** tab lists every incident, with a filter by app, AI or data type.

Click an incident to open its page:

- **Source:** the app, the process and its pid, the conversation or session id, and the full `User-Agent`.
- **Destination:** the AI provider and host, what kind of destination it is (model API, inference cloud or AI chat app), the model, the request path, the HTTP status and timing.
- **What was caught:** each value found, the detector that found it, and what the AI received instead. Values sent unchanged on purpose, such as the recipient of an email you asked the AI to send, are marked as such.
- **Prompt:** every message, as you wrote it or as the AI received it, with the changed values highlighted.

Values are blurred until you press **Show values**, so the page is safe to have on a shared screen. Incident pages have their own address (`/shadow/ui#/incidents/42`). Pick a window of 1 hour, 24 hours, 7 days or 30 days (keys `1` to `4`); the page refreshes every few seconds and follows your system's light or dark theme.

The page is built into the `shadow` binary. It loads nothing from the internet, runs under a strict Content-Security-Policy (`default-src 'none'`, scripts and styles from the page's own origin only) and talks only to the activity API below.

Useful URL parameters:

| Parameter | Effect |
|---|---|
| `?demo=1` | Synthetic demo data, clearly labelled "Demo data". Good for trying the UI or taking screenshots. |
| `?theme=dark` / `?theme=light` | Force a theme. |
| `?w=1h` / `24h` / `7d` / `30d` | Initial time window. |
| `?embed=1` | Compact layout used by the tray app. |

## Terminal: `shadow top`

```sh
shadow top          # live view of the running proxy
shadow top --demo   # synthetic data
```

A full-screen view with the totals, a timeline, bars by data type, destination and app, and a live incident list. Keys: `1`-`4` change the window, `Tab` or arrow keys switch between Overview, Incidents and Breakdown, `↑`/`↓` and `Enter` open an incident, `r` refreshes, `q` quits.

If the proxy is not running, `shadow top` says so, shows the history saved on disk, and keeps retrying.

For servers and scripts, `shadow activity` prints the same summary as plain tables, and `shadow incidents` lists incidents or shows one in full:

```sh
shadow activity                 # last 24 hours
shadow activity --window 7d
shadow activity --json          # the summary object from the API below
shadow incidents                # recent incidents with their IDs
shadow incidents 42             # prompt, values caught, app and process of incident 42
shadow incidents 42 --reveal    # print the caught values in full
```

`shadow incidents` shortens caught values (`ma*******om`) unless you pass `--reveal`.

## Menu-bar and tray app

`apps/shadow-tray` is a small companion app for macOS (menu bar) and Windows (system tray). Its icon shows whether Shadow is protecting the machine (a filled crescent) or not running (an outline). The menu lists today's requests, values protected and requests blocked, and has **Open dashboard** (a popover with the web dashboard), **Incidents and prompts** (the incident list, where each incident opens its prompt and source), **Open in browser**, **Docs** and **Quit**. Clicking the values or blocked line also opens the incident list. On macOS it has no Dock icon; on Windows, left-click opens the dashboard and right-click opens the menu.

The install script puts it in `~/.local/bin/shadow-tray` on macOS, and `shadow install --apply` starts it at login. To add it to an existing install:

```sh
cargo install --locked --git https://github.com/Tiny-Bubble-Company/shadow shadow-tray --root ~/.local
shadow install --apply
```

On Windows, the PowerShell installer builds it when a Rust toolchain is present. It is a separate Cargo workspace built on the system webview (WKWebView, WebView2), so building Shadow itself never needs webview libraries.

## HTTP API

All endpoints are on the proxy's listen address and return JSON.

| Endpoint | Returns |
|---|---|
| `GET /shadow/activity?since=<id>&limit=<n>&action=masked,blocked` | Recent events, newest first. `action=incidents` is short for masked and blocked. |
| `GET /shadow/activity/summary?window=1h\|24h\|7d\|30d\|today` | Totals, a timeline, and breakdowns by data type (`kinds`), destination (`providers`), app (`clients`), endpoint and block reason. |
| `GET /shadow/activity/events/<id>` | One event, plus its incident `detail` (prompt parts, values caught, model, process) while still kept, or `null`. |
| `GET /shadow/activity/stream` | Server-Sent Events: one `activity` event per new request. |
| `GET /shadow/ui` | The dashboard. |

```sh
curl -s 'http://127.0.0.1:8877/shadow/activity/summary?window=24h' | jq .totals
```

```json
{
  "requests": 573,
  "masked_requests": 349,
  "blocked": 11,
  "clean": 213,
  "values_protected": 981,
  "values_restored": 629,
  "providers": 8,
  "clients": 9,
  "avg_latency_ms": 1840
}
```

### Who can reach it

The dashboard and the activity API answer only connections from the same machine, and only when the `Host` header names a loopback address (`127.0.0.1`, `localhost`, `[::1]`). The second check stops a web page you visit from reading the API through DNS rebinding. Other paths on the proxy are unaffected.

When Shadow runs in Docker and listens on `0.0.0.0`, connections from the host arrive from the Docker bridge and are refused. Publish the port on loopback only (`-p 127.0.0.1:8877:8877`) and set `allow_remote = true`, or read the summary from inside the container. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/).

## Configuration

```toml
[activity]
enabled = true          # record activity events at all
persist = true          # keep history in <data_dir>/activity/activity.jsonl
retention_days = 30     # drop older events
max_file_mb = 16        # rotate the log past this size (one previous file is kept)
allow_remote = false    # serve the dashboard and API to non-loopback clients
details = true          # keep prompt text and values of recent incidents
details_limit = 200     # how many incidents keep their details
persist_details = true  # save them encrypted (vault key) so they survive restarts
```

Recording adds one short in-memory update per request; the file is written from a background thread. With `persist = false` the dashboard still works but starts empty after each restart. See the [configuration reference](/shadow/reference/configuration/) for every other option.
