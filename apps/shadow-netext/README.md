# Shadow capture (macOS Network Extension)

A transparent proxy that sends **every outbound TCP connection on the Mac, from every app**, through the local Shadow proxy. It covers what the system proxy misses: apps that ignore the system proxy settings, command-line tools without `HTTPS_PROXY`, and desktop apps with their own network stack.

> **Status: scaffold.** The sources type-check against the macOS 13 SDK, but the extension has not been built, signed or run yet. A system extension can only be built and run with an Apple Developer ID, the Network Extension entitlement and notarisation. Until then, use the system proxy: `shadow install --system-proxy --apply`.

## How it works

```text
any app ──TCP──▶ Shadow capture (NETransparentProxyProvider)
                   │  CONNECT host:443 to 127.0.0.1:8877
                   ▼
                 Shadow ── AI host? decrypt, mask, restore ──▶ AI provider
                        └─ anything else: tunnel byte for byte, not recorded ──▶ internet
```

- **All outbound TCP** is claimed by the extension and handed to Shadow as an HTTP `CONNECT` tunnel, so Shadow's existing rules apply unchanged: AI hosts are decrypted and masked, everything else is passed through and never recorded.
- **QUIC is refused** (outbound UDP 443), so browsers fall back to TCP, which Shadow can see.
- **Shadow's own connections are never redirected** (matched by code-signing identifier `dev.shadow.*` or the ad-hoc `shadow-<hash>` of a `cargo` build), which would loop.
- **Local and private networks** (`127/8`, `10/8`, `172.16/12`, `192.168/16`, `169.254/16`) are excluded.
- **Fail-open:** if Shadow is not answering, each connection goes direct. Stopping Shadow never cuts the Mac off.
- Apps that pin certificates will still refuse Shadow's local CA for the AI hosts it decrypts; their other traffic is unaffected.

## Files

| Path | What it is |
|---|---|
| `Extension/TransparentProxyProvider.swift` | The system extension: flow handling, CONNECT to Shadow, fail-open, QUIC refusal |
| `Extension/Info.plist`, `Extension/Extension.entitlements` | Bundle id `dev.shadow.netext.extension`, `app-proxy-provider-systemextension` |
| `HostApp/ShadowNetExtApp.swift` | A small SwiftUI app that installs the extension and turns it on or off |
| `HostApp/HostApp.entitlements` | Network Extension and system-extension install entitlements |

## Build it

You need Xcode 15 or later and an Apple Developer account (paid) with the **Network Extensions** capability enabled for your team.

1. In Xcode, create a **macOS App** named `ShadowNetExtApp` (bundle id `dev.shadow.netext`), SwiftUI interface. Replace its source with `HostApp/ShadowNetExtApp.swift` and use `HostApp/HostApp.entitlements`.
2. Add a target: **System Extension → Network Extension**, named `Extension`, bundle id `dev.shadow.netext.extension`, provider type **App Proxy**. Replace its source with `Extension/TransparentProxyProvider.swift`, and use the `Info.plist` and entitlements from `Extension/`.
3. Make the app embed the extension in `Contents/Library/SystemExtensions` (Xcode does this when you add the target).
4. Sign both targets with your **Developer ID Application** certificate and the same team. In your developer account, enable **Network Extensions** and **System Extension** for both identifiers and create Developer ID provisioning profiles that include `app-proxy-provider-systemextension`.
5. Archive, then notarise and staple:

   ```sh
   xcrun notarytool submit ShadowNetExtApp.zip --keychain-profile <profile> --wait
   xcrun stapler staple ShadowNetExtApp.app
   ```

6. Move the app to `/Applications` (system extensions only load from there), open it, press **Install extension**, approve it in **System Settings → General → Login Items & Extensions → Network Extensions**, then press **Turn on**.

During development you can skip notarisation with `systemextensionsctl developer on` (requires turning off System Integrity Protection on a test Mac only).

## Before shipping

- Run it with Shadow stopped and confirm that apps still connect (fail-open).
- Check that Shadow's own process is exempt on a Developer ID build (its signing identifier).
- Test the six supported chat apps (ChatGPT, Claude.ai, Gemini, Perplexity, DeepSeek, Grok) in Safari and Chrome, and an API client such as the OpenAI Python SDK without `HTTPS_PROXY`.
- `NWHostEndpoint` and `remoteEndpoint` are deprecated in macOS 15 in favour of `remoteFlowEndpoint`; switch when the minimum macOS version allows.
