---
title: FAQ
description: Common questions about Shadow.
---

### Does Shadow send anything anywhere?

Only the masked requests you were already sending, to the providers you were already using. There is no telemetry, no account and no hosted service. Optional detectors (the NER sidecar, Presidio) are called on addresses you configure, normally loopback.

### Does the model's answer get worse?

Usually not noticeably, which is the point of realistic fakes. A fake email is still an email, a fake card number still passes Luhn, and the persona keeps names, domains and regions consistent with each other. Answers can shift where the persona's region matters (holidays, phone formats, legal context); pin `[persona] region` if that matters to you. For bulk data in reports, typed tokens avoid the model inventing facts about fake people.

### Can the provider reverse the fakes?

No. Fakes are generated randomly from the persona. The mapping exists only in your encrypted vault.

### What happens if Shadow is down?

In gateway mode, requests to `127.0.0.1:8877` fail. In proxy mode, tools fail to connect. Nothing silently bypasses Shadow. The background services restart on failure. To bypass Shadow on purpose, unset `HTTPS_PROXY` or the base URL.

### How much latency does it add?

Masking is in-process string work on the request: typically a fraction of a millisecond to a few milliseconds per request, small next to model latency. Streams are delayed by at most a few characters (the length of the longest fake). Optional detectors add their own call time: the NER sidecar is capped by `timeout_ms`, and Presidio adds a local HTTP round trip.

### Can I see what was replaced?

`shadow run` logs one line per request with the number of values masked, never the values. `shadow scan <file>` shows what would be replaced in a file. `shadow vault export --i-understand` prints the full mapping.

### Does it work with streaming?

Yes. Server-sent events and NDJSON streams are restored on the fly, including fakes split across events and tool-call arguments streamed in pieces. See [Streaming hold-back](/shadow/concepts/how-it-works/#streaming-hold-back).

### Does it work with agents and tools?

Yes. Tool-call arguments are restored on your machine before your tools run. Tools hosted by the provider are handled by policy. See [Tool calls and agents](/shadow/concepts/tool-calls/).

### Does it work with images, PDFs and Office files?

Text, CSV, JSON, HTML, XML, DOCX, XLSX and PPTX uploads are masked. PDF text is extracted, masked and sent as text. Images pass through unchanged (no OCR yet) unless you block them. See [File masking](/shadow/concepts/file-masking/).

### Can several machines share one identity?

Yes. Copy `persona.json`, `vault.bin` and `vault.key` to each machine, or point several processes at one data directory. Concurrent writers are merged on save.

### Why does the persona live far away?

So that the fake identity is not a near miss of the real one. A machine in Germany presenting as a Berlin company with different names would still leak a lot. You can pin any supported region.

### Why not just use Presidio?

Presidio is an excellent detector, and Shadow can use it. But detection is half the job. Presidio's anonymizer replaces values with placeholders or hashes; Shadow adds consistent, format-preserving fakes from a persona, an encrypted persistent mapping, response restoration (including streams and tool calls), an HTTPS proxy and gateway, and intent-aware policy. Use them together: Presidio for recall, Shadow for everything around it.

### Is this a DLP product?

It overlaps with DLP but behaves differently. DLP gateways usually block or redact. Shadow translates, so the tool keeps working and people have no reason to route around it.

### Can Shadow run in a browser or on a phone?

No. Shadow runs on macOS, Linux and Windows hosts, servers and containers. Browsers on those machines can use it through the system proxy or PAC file.

### What licence is Shadow under?

Apache-2.0.
