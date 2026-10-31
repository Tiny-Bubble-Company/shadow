---
title: CLI
description: "Reference for the shadow command line: init, run, install, status, mask, unmask, scan, vault, terms, hosts, top, activity and dashboard, with every subcommand and flag."
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
| [`install`](#shadow-install) / [`uninstall`](#shadow-uninstall) | Background service and CA trust (dry run by default) |
| [`status`](#shadow-status) | Configuration, persona, vault and proxy health |
| [`identity`](#shadow-identity) | Show the persona |
| [`mask`](#shadow-mask) / [`unmask`](#shadow-unmask) | Translate stdin to stdout |
| [`scan`](#shadow-scan) | Report sensitive values in a file |
| [`vault`](#shadow-vault) | Inspect, export or edit the vault |
| [`terms`](#shadow-terms) | Manage protected dictionary terms |
| [`hosts`](#shadow-hosts) | List intercepted hosts and gateway routes |
| [`top`](#shadow-top) | Live activity view in the terminal |
| [`activity`](#shadow-activity) | Print an activity summary |
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
shadow install [--apply] [--listen <ADDR>]
```

| Flag | Description |
|---|---|
| `--apply` | Perform the steps. Without it, nothing changes |
| `--listen <ADDR>` | Listen address for the service |

Creates the persona, key and CA first if needed. After the plan, it prints how to point tools at Shadow (gateway URLs, the proxy environment, and the macOS PAC command). With `--apply`, it also writes `env.sh` (`env.ps1` on Windows) to the data directory. See [Installation](/shadow/getting-started/installation/#run-as-a-background-service) for the per-OS steps.

## `shadow uninstall`

Remove the background service and the CA trust. Same flags as `install`, also a dry run unless `--apply`. The data directory is left in place.

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

Print the activity summary as plain tables, or as JSON with `--json`. `--window` (`-w`) is `1h`, `24h` (default), `7d`, `30d` or `today`. `--demo` summarises synthetic data.

```sh
shadow activity [--window <WINDOW>] [--json] [--demo]
```

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
