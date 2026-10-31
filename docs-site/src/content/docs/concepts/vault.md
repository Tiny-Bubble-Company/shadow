---
title: The vault
seoTitle: "Encrypted vault for reversible PII masking · Shadow"
description: "How Shadow's local vault stores the real-to-fake mapping: ChaCha20-Poly1305 encryption at rest, multi-process sharing, export and forgetting values."
faq:
  - q: "Does the vault ever leave my machine?"
    a: "No. The real-to-fake mapping is stored only in the data directory on your machine and is never sent to a provider or any other service."
  - q: "How is the vault encrypted?"
    a: "With ChaCha20-Poly1305, using a 32-byte key in `vault.key` created with mode `0600` and a fresh nonce on every save. A wrong key or a modified file fails authentication instead of producing garbage."
  - q: "Can several processes share one vault?"
    a: "Yes. Before each save, Shadow merges the on-disk vault in, so the background proxy and a `shadow mask` in your terminal never lose each other's entries."
  - q: "What happens if I lose vault.key?"
    a: "`vault.bin` can no longer be read. Move both files away and Shadow starts a new vault: new requests work normally, but responses that mention old fakes are not restored."
---

The vault is the table of real↔fake pairs. It is what makes translation reversible, and it never leaves your machine.

## Format and encryption

| File | Contents |
|---|---|
| `vault.bin` | `SHV1` magic, a 12-byte nonce, then the JSON list of entries encrypted with **ChaCha20-Poly1305** |
| `vault.key` | The 32-byte key, hex encoded, created with mode `0600` |

Each entry records the kind (`email`, `person`, `secret`, ...), the real value, the fake, and when it was created. A fresh nonce is used for every save. A wrong key or a modified file fails authentication and is reported as an error rather than silently producing garbage.

Typed tokens used by the [intent policy](/shadow/concepts/intent-policy/) (`[PERSON_0001]`) are numbered per session and restored from that session. Redacted values are irreversible by design: the provider receives `[REDACTED_SECRET]` and nothing maps it back.

## Writes and concurrency

- **Atomic.** Every save writes a temporary file and renames it over `vault.bin`, so a crash never leaves a half-written vault.
- **Thread-safe.** One `Shadow` instance is shared by all connections. Lookups take a read lock; adding a new value takes a short write lock.
- **Multi-process.** Several processes can use one data directory, for example the background proxy and a `shadow mask` in your terminal. Before each save, the on-disk vault is merged in (a union of pairs), so no process loses another's entries. Running processes pick up changes from disk when the file changes.

## Inspecting the vault

```sh
shadow vault stats
```

```text
◆ Vault — 13 entries (created 2026-10-31T12:31:29Z)
  KIND         ENTRIES
  credit_card  1
  email        2
  hostname     2
  org          2
  person       2
  secret       1
  term         1
  username     2
```

To see the actual pairs:

```sh
shadow vault export --i-understand
```

This prints real values in clear text, which is why it needs the flag. Don't paste the output into an AI tool.

## Forgetting a value

```sh
shadow vault forget "jane@acme.com"     # by real value
shadow vault forget "hui.zhao@zhilan.cn" # or by fake
```

After forgetting, the next time the value appears it gets a new fake. Responses that still mention the old fake will no longer be restored.

## Backups and moving machines

The vault, its key and the persona belong together. To move an identity to another machine, or to give several machines one identity, copy:

```text
persona.json  vault.bin  vault.key
```

Without `vault.key`, `vault.bin` cannot be read. Without `vault.bin`, old fakes cannot be restored, but new requests work normally.

## Protecting the vault

The vault is only as safe as the account that owns it. Anyone who can read files as your user can read `vault.key` and decrypt the mapping. Use full-disk encryption, keep the data directory private, and treat `vault.key` like any other credential. On servers, run Shadow as a dedicated user and restrict the data directory to it. See the [security model](/shadow/reference/security/).
