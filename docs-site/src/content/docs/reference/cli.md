---
title: CLI
seoTitle: "Shadow CLI reference: every command, subcommand and flag"
description: "Reference for the shadow CLI: init, run, install, status, mask, unmask, scan, vault, terms, hosts, top, activity and dashboard, with every flag."
faq:
  - q: "How do I see what Shadow would mask without sending anything?"
    a: "Run `shadow scan <FILE>` to list sensitive values without changing anything or touching the vault, or pipe text through `shadow mask` to see the masked version."
  - q: "How do I health-check Shadow in a script?"
    a: "Use `shadow status --quiet`. It prints nothing and exits 0 if the proxy answers, 1 otherwise."
  - q: "How do I see the real-to-fake pairs?"
    a: "`shadow vault export --i-understand` prints every pair as JSON. It shows real values in clear text, so don't paste the output into an AI tool."
  - q: "How do I see what Shadow has masked recently?"
    a: "Use `shadow top` for a live terminal view, `shadow activity` for a summary, or `shadow dashboard` to open the web dashboard. They show counts and categories, never values."
---

```text
shadow [--config <path>] [--data-dir <dir>] <command>
```

| Global flag | Description |
|---|---|
| `--config <path>` | Path to `shadow.toml`. Default `<data dir>/shadow.toml`. Also read from `SHADOW_CONFIG`. An explicit path that doesn't exist is an error |
| `--data-dir <dir>` | Data directory. Default `$SHADOW_HOME` or the platform data directory |
| `-h`, `--help` | Help for any command |
| `-V`, `--version` | Print the version |

Logs go to stderr. `shadow run` logs at `info` by default, every other command at `warn`. Override with `RUST_LOG`.

## Commands at a glance

| Command | Purpose |
|---|---|
| [`init`](#shadow-init) | Create the persona, vault key and local CA |
| [`run`](#shadow-run) | Run the proxy and gateway in the foreground |
| [`install`](#shadow-install) / [`uninstall`](#shadow-uninstall) | Background service and CA trust, optionally the system proxy (dry run by default) |
| [`system-proxy`](#shadow-system-proxy) | Route the machine's traffic through Shadow, or stop |
| [`hub`](#shadow-hub) | Connect this machine to your organisation's Shadow Hub, check it, or disconnect |
| [`status`](#shadow-status) | Configuration, persona, vault and proxy health |
| [`identity`](#shadow-identity) | Show the persona |
| [`mask`](#shadow-mask) / [`unmask`](#shadow-unmask) | Translate stdin to stdout |
| [`scan`](#shadow-scan) | Report sensitive values in a file |
| [`vault`](#shadow-vault) | Inspect, export or edit the vault |
| [`terms`](#shadow-terms) | Manage protected dictionary terms |
| [`hosts`](#shadow-hosts) | List intercepted hosts and gateway routes |
| [`top`](#shadow-top) | Live activity view in the terminal |
| [`activity`](#shadow-activity) | Print an activity summary |
| [`incidents`](#shadow-incidents) | List incidents, or show one with its prompt and source |
| [`receipts`](#shadow-receipts) | Check the evidence receipt chain, show a receipt, or verify a downloaded one |
| [`dashboard`](#shadow-dashboard) | Open the web activity dashboard |

## `shadow init`

Create the persona, vault key and local CA. Writes a minimal `shadow.toml` if none exists.

```sh
shadow init [--region <CODE>] [--org <NAME>]... [--force]
```

| Flag | Description |
|---|---|
| `--region <CODE>` | Fake region: `CN`, `JP`, `KR`, `BR`, `US`, `DE`, `IN`, `AU`, `MX`, `ZA`, `CA`, or `auto`. Ignored if a persona exists (use `--force`) |
| `--org <NAME>` | Organisation name to protect. Repeatable. The first maps to the persona company |
| `--force` | Delete the existing persona and vault and start over |

Also writes `ca/bundle.pem` (system roots + Shadow CA). Safe to run again: existing files are kept unless `--force` is given.

## `shadow run`

Run the forward proxy and the `/gw` gateway in the foreground until Ctrl-C.

```sh
shadow run [--listen <ADDR>]
```

| Flag | Description |
|---|---|
| `--listen <ADDR>` | Listen address. Default `[proxy] listen`, i.e. `127.0.0.1:8877`. Warns if not loopback |

On start it prints the persona, the gateway URL pattern, the proxy setting and the CA path, and the upstream proxy if one is configured.

## `shadow install`

Install the background service and trust the local CA. Prints the plan; performs it only with `--apply`.

```sh
shadow install [--apply] [--listen <ADDR>] [--system-proxy]
```

| Flag | Description |
|---|---|
| `--apply` | Perform the steps. Without it, nothing changes |
| `--listen <ADDR>` | Listen address for the service |
| `--system-proxy` | Also point the system proxy at Shadow, so browsers and desktop apps (ChatGPT, Claude.ai, Gemini, ...) are covered. See [`system-proxy`](#shadow-system-proxy) |

Creates the persona, key and CA first if needed. After the plan, it prints how to point tools at Shadow (gateway URLs, the proxy environment, and the macOS PAC command). With `--apply`, it also writes `env.sh` (`env.ps1` on Windows) to the data directory. See [Installation](/shadow/getting-started/installation/#run-as-a-background-service) for the per-OS steps.

## `shadow uninstall`

Remove the system proxy (where it points at Shadow), the background service and the CA trust. Same flags as `install`, also a dry run unless `--apply`. The data directory is left in place.

## `shadow system-proxy`

Route the machine's traffic through Shadow with the system proxy settings, or stop. `on` and `off` print the steps and perform them only with `--apply`; `status` shows where the system proxy points at Shadow.

```sh
shadow system-proxy status
shadow system-proxy on [--apply]
shadow system-proxy off [--apply]
```

`on` sets `http://<listen>/proxy.pac?scope=all` as the automatic proxy configuration: on every enabled macOS network service (`networksetup`), in Windows Internet Settings (`AutoConfigURL`) or in GNOME (`gsettings`). The PAC file sends all traffic to Shadow except local and private addresses, and falls back to a direct connection when Shadow is not running. Shadow decrypts only AI hosts and chat apps; everything else is tunnelled and not recorded. `off` turns off only settings that point at Shadow. See [Proxy mode](/shadow/integrate/proxy-mode/#system-wide-with-the-system-proxy).

## `shadow status`

```sh
shadow status [--quiet]
```

Shows whether the proxy answers on the configured address, the data directory, the persona, the vault size and creation date, the CA path and SHA-256 fingerprint, the model detector, and the upstream proxy.

| Flag | Description |
|---|---|
| `--quiet` | Print nothing; exit 0 if the proxy answers, 1 otherwise. Useful for health checks |

## `shadow identity`

```sh
shadow identity [--json]
```

Show the persona: company and domain, primary person and email, location, timezone and locale, user id, username, hostname and colleagues. `--json` prints the full `persona.json` record.

## `shadow mask`

Mask stdin to stdout. Uses and updates the same vault as the proxy.

```sh
echo "Mail jane@acme.com" | shadow mask
shadow mask --json < request.json
```

| Flag | Description |
|---|---|
| `--json` | Parse stdin as JSON and mask string values only (protocol keys are skipped). Output is pretty-printed |

## `shadow unmask`

Unmask stdin to stdout.

```sh
shadow unmask < answer.txt
shadow unmask --json < response.json
```

| Flag | Description |
|---|---|
| `--json` | Parse stdin as JSON and unmask every string value |

## `shadow scan`

Report sensitive values in a file without changing anything or touching the vault.

```sh
shadow scan <FILE> [--reveal] [--json]
```

| Flag | Description |
|---|---|
| `--reveal` | Print full values instead of redacted previews (`ja*********om`) |
| `--json` | Output a JSON array of `{kind, line, column, detector, score, value}` |

The table output shows `KIND`, `AT` (line:column), `VALUE` and `DETECTOR`.

## `shadow vault`

```sh
shadow vault stats
shadow vault export --i-understand
shadow vault forget <VALUE>
```

| Subcommand | Description |
|---|---|
| `stats` | Entry counts by kind and the vault's creation time |
| `export --i-understand` | Print every real↔fake pair as JSON. Contains real values in clear text, so the flag is required |
| `forget <VALUE>` | Remove the mapping for a real or a fake value |

## `shadow terms`

```sh
shadow terms add <TERM>...
shadow terms list
```

| Subcommand | Description |
|---|---|
| `add <TERM>...` | Protect one or more terms. Prefix with `person:`, `org:`, `term:`, `hostname:`, `email:` or `username:` to set the kind. Prints the fake each term maps to. Stored in `terms.json` |
| `list` | List all protected terms (from config, `organization` and `terms.json`) with their kinds |

```text
$ shadow terms add "Project Falcon" "person:Jane Doe"
✓ Project Falcon  →  Lumeth Draane
✓ person:Jane Doe  →  Xiu Wang
```

## `shadow hosts`

List the hosts intercepted in proxy mode (built-ins plus `extra_hosts`, with `bypass_hosts` marked) and the gateway routes (built-ins plus `[proxy.routes]`).

```sh
shadow hosts
```

## `shadow top`

A live, full-screen view of the running proxy's [activity](/shadow/concepts/activity-dashboard/): totals, a timeline, and breakdowns by data type, destination and app. `--demo` shows synthetic data instead.

```sh
shadow top [--demo]
```

## `shadow activity`

Print the activity summary as plain tables, or as JSON with `--json` (`shadow stats` is the same command). At most once a week it ends with one line about Shadow Hub; `--no-hints` or `hints = false` in `shadow.toml` turns that off. The hint makes no network call. `--window` (`-w`) is `1h`, `24h` (default), `7d`, `30d` or `today`. `--demo` summarises synthetic data.

```sh
shadow activity [--window <WINDOW>] [--json] [--demo]
```

## `shadow hub`

Connect this machine to your organisation's [Shadow Hub](/shadow/hub/), check the connection, or end it. Core works fully on its own and never phones home: nothing is sent anywhere unless you connect.

```sh
shadow hub connect [<JOIN LINK>] [--yes] [--email <EMAIL>] [--name <NAME>]
shadow hub status
shadow hub disconnect
```

`connect` shows the organisation and exactly what it will see at its sharing level, and asks before anything is sent (`--yes` skips the question, for scripts and MDM). Without a link it explains how to get one and opens [getshadow.dev/hub/connect](/shadow/hub/connect/). `--email` and `--name` are used when the invite was not made out to a named person. The connection is stored in `<data dir>/hub.json` (mode `0600`). `disconnect` stops reporting; Shadow keeps protecting the machine.

The installer connects in one step when run with `SHADOW_ENROLL='<join link>'`. `shadow enroll` and `shadow unenroll` remain as aliases.

## `shadow incidents`

List incidents (masked or blocked requests) with their IDs, or show one in full: the app, process and conversation that sent it, the AI and model it went to, each value caught and what replaced it, and the prompt as written and as sent. Details come from the running proxy, which keeps each incident's prompt for as long as the incident (`retention_days`).

```sh
shadow incidents [-n <N>] [--json]
shadow incidents <ID> [--reveal] [--json]
```

| Flag | Description |
|---|---|
| `-n`, `--limit <N>` | How many incidents to list (default 30) |
| `--reveal` | Print caught values in full instead of shortened (`ma*******om`) |
| `--json` | Print the raw JSON from the proxy, values in full |

## `shadow receipts`

Check this machine's [evidence receipts](/shadow/concepts/evidence-receipts/): every signature, and that each receipt links to the one before. Exits with an error when a receipt is invalid or missing.

```sh
shadow receipts [-w <WINDOW>] [--json]
shadow receipts show <ID> [--json]
shadow receipts verify <FILE>
```

| Flag | Description |
|---|---|
| `-w`, `--window <WINDOW>` | `1h`, `24h`, `7d`, `30d`, `today` or `all` (default) |
| `--json` | Print the raw result; with `show`, the receipt and public key as a file `verify` accepts |

`verify` checks a receipt downloaded from the dashboard or Shadow Hub without the proxy running.

## `shadow dashboard`

Open the web dashboard (`http://127.0.0.1:8877/shadow/ui`) in your browser. `--print` prints the URL instead; `--demo` opens it with synthetic data.

```sh
shadow dashboard [--demo] [--print]
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Error (message printed to stderr as `error: ...`), or `status --quiet` with the proxy down |
| `2` | Invalid command-line usage |
