//! Incident details on disk, for auditing: every masked or blocked request's prompt, values
//! and process, kept as long as its event, encrypted with the vault key.
//!
//! `activity/details.log` is append-only, one line per incident:
//! `<id> <ts> <base64(nonce(12) || ChaCha20-Poly1305(JSON detail))>`, mode 0600. The id and
//! timestamp are in clear (the activity log already holds them) and bound into the
//! ciphertext as associated data, so a line cannot be moved to another incident. Lines are
//! read back on demand through an in-memory index of offsets. Expired incidents and, past
//! `max_bytes`, the oldest ones are dropped by rewriting the file.
//!
//! `activity/receipts.log` uses the same layout without encryption for scan receipts
//! ([`crate::receipt`]), which hold hashes and counts only.

use std::collections::BTreeMap;
use std::io::{BufRead, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use base64::Engine;
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use rand::RngCore;

use crate::activity::Detail;

/// File names inside the activity dir.
pub const DETAILS_LOG: &str = "details.log";
pub const RECEIPTS_LOG: &str = "receipts.log";

/// Where one incident's line is: byte offset, length (without the newline) and time.
#[derive(Clone, Copy, Debug)]
struct Entry {
    offset: u64,
    len: u32,
    ts: i64,
}

pub(crate) struct DetailStore {
    path: PathBuf,
    /// `None` for files whose lines need no encryption (scan receipts).
    cipher: Option<ChaCha20Poly1305>,
    max_bytes: u64,
    retention_ms: i64,
    keep_last: bool,
    inner: Mutex<Inner>,
}

struct Inner {
    index: BTreeMap<u64, Entry>,
    size: u64,
}

fn aad(id: u64, ts: i64) -> Vec<u8> {
    format!("shadow-details-v2:{id}:{ts}").into_bytes()
}

impl DetailStore {
    /// Open `<dir>/details.log`, keeping the lines for which `keep(id)` holds and that are
    /// newer than `retention_ms`; rewrites the file when anything was dropped.
    pub(crate) fn open(
        dir: &Path,
        cipher: ChaCha20Poly1305,
        max_bytes: u64,
        retention_ms: i64,
        keep: impl Fn(u64) -> bool,
    ) -> anyhow::Result<Arc<Self>> {
        Self::open_file(
            dir,
            DETAILS_LOG,
            Some(cipher),
            max_bytes,
            retention_ms,
            false,
            keep,
        )
    }

    /// [`DetailStore::open`] for any file name, with or without encryption. With
    /// `keep_last` the newest line survives expiry (a receipt chain continues from it).
    pub(crate) fn open_file(
        dir: &Path,
        name: &str,
        cipher: Option<ChaCha20Poly1305>,
        max_bytes: u64,
        retention_ms: i64,
        keep_last: bool,
        keep: impl Fn(u64) -> bool,
    ) -> anyhow::Result<Arc<Self>> {
        let path = dir.join(name);
        let store = Arc::new(Self {
            path,
            cipher,
            max_bytes: max_bytes.max(1024 * 1024),
            retention_ms,
            keep_last,
            inner: Mutex::new(Inner {
                index: BTreeMap::new(),
                size: 0,
            }),
        });
        let mut dropped = false;
        if let Ok(f) = std::fs::File::open(&store.path) {
            let floor = crate::activity::now_ms() - retention_ms;
            let mut reader = std::io::BufReader::new(f);
            let mut offset = 0u64;
            let mut line = Vec::new();
            let mut inner = store.lock();
            let mut newest: Option<(u64, Entry)> = None;
            loop {
                line.clear();
                let n = reader.read_until(b'\n', &mut line)?;
                if n == 0 {
                    break;
                }
                let body = line.strip_suffix(b"\n").unwrap_or(&line);
                if let Some((id, ts)) = parse_head(body) {
                    if newest.is_none_or(|(n, _)| id > n) {
                        let len = body.len() as u32;
                        newest = Some((id, Entry { offset, len, ts }));
                    }
                }
                match parse_head(body) {
                    Some((id, ts)) if ts >= floor && keep(id) => {
                        inner.index.insert(
                            id,
                            Entry {
                                offset,
                                len: body.len() as u32,
                                ts,
                            },
                        );
                    }
                    _ => dropped = true,
                }
                offset += n as u64;
            }
            if let Some((id, e)) = newest.filter(|_| store.keep_last) {
                inner.index.entry(id).or_insert(e);
            }
            inner.size = offset;
        }
        if dropped || store.lock().size > store.max_bytes {
            store.compact()?;
        }
        Ok(store)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn contains(&self, id: u64) -> bool {
        self.lock().index.contains_key(&id)
    }

    /// Encrypt and append one incident.
    pub(crate) fn append(&self, id: u64, ts: i64, detail: &Detail) -> anyhow::Result<()> {
        self.append_bytes(id, ts, &serde_json::to_vec(detail)?)
    }

    /// Append `plain` for `id`, encrypted when this store has a cipher.
    pub(crate) fn append_bytes(&self, id: u64, ts: i64, plain: &[u8]) -> anyhow::Result<()> {
        let Some(cipher) = &self.cipher else {
            return self.append_line(id, ts, plain);
        };
        let mut nonce = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);
        let ct = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: plain,
                    aad: &aad(id, ts),
                },
            )
            .map_err(|_| anyhow::anyhow!("details encryption failed"))?;
        let mut sealed = nonce.to_vec();
        sealed.extend_from_slice(&ct);
        self.append_line(id, ts, &sealed)
    }

    fn append_line(&self, id: u64, ts: i64, sealed: &[u8]) -> anyhow::Result<()> {
        let line = format!(
            "{id} {ts} {}",
            base64::engine::general_purpose::STANDARD_NO_PAD.encode(sealed)
        );
        {
            let mut inner = self.lock();
            let mut f = open_append(&self.path)?;
            f.write_all(line.as_bytes())?;
            f.write_all(b"\n")?;
            let offset = inner.size;
            inner.index.insert(
                id,
                Entry {
                    offset,
                    len: line.len() as u32,
                    ts,
                },
            );
            inner.size += line.len() as u64 + 1;
        }
        if self.lock().size > self.max_bytes + self.max_bytes / 4 {
            self.compact()?;
        }
        Ok(())
    }

    /// Read and decrypt one incident.
    pub(crate) fn get(&self, id: u64) -> Option<Detail> {
        serde_json::from_slice(&self.get_bytes(id)?).ok()
    }

    /// Ids of the entries saved at or after `since` (unix ms), oldest first.
    pub(crate) fn ids_since(&self, since: i64) -> Vec<u64> {
        let inner = self.lock();
        inner
            .index
            .iter()
            .filter(|(_, e)| e.ts >= since)
            .map(|(id, _)| *id)
            .collect()
    }

    /// The newest entry: its id and bytes.
    pub(crate) fn last(&self) -> Option<(u64, Vec<u8>)> {
        let id = *self.lock().index.keys().next_back()?;
        Some((id, self.get_bytes(id)?))
    }

    /// Read (and decrypt) the bytes saved for `id`.
    pub(crate) fn get_bytes(&self, id: u64) -> Option<Vec<u8>> {
        let inner = self.lock();
        let e = *inner.index.get(&id)?;
        let mut f = std::fs::File::open(&self.path).ok()?;
        f.seek(SeekFrom::Start(e.offset)).ok()?;
        let mut buf = vec![0u8; e.len as usize];
        f.read_exact(&mut buf).ok()?;
        drop(inner);
        self.open_line(&buf, id)
    }

    fn open_line(&self, line: &[u8], want: u64) -> Option<Vec<u8>> {
        let (id, ts) = parse_head(line)?;
        if id != want {
            return None;
        }
        let b64 = std::str::from_utf8(line).ok()?.splitn(3, ' ').nth(2)?;
        let sealed = base64::engine::general_purpose::STANDARD_NO_PAD
            .decode(b64)
            .ok()?;
        let Some(cipher) = &self.cipher else {
            return Some(sealed);
        };
        if sealed.len() < 12 {
            return None;
        }
        cipher
            .decrypt(
                Nonce::from_slice(&sealed[..12]),
                Payload {
                    msg: &sealed[12..],
                    aad: &aad(id, ts),
                },
            )
            .ok()
    }

    /// Rewrite the file with the incidents still within retention, newest first up to
    /// three quarters of `max_bytes`, so it does not have to be rewritten again soon.
    fn compact(&self) -> anyhow::Result<()> {
        let mut inner = self.lock();
        let floor = crate::activity::now_ms() - self.retention_ms;
        let budget = self.max_bytes - self.max_bytes / 4;
        let mut keep = vec![];
        let mut total = 0u64;
        for (i, (id, e)) in inner.index.iter().rev().enumerate() {
            let pinned = i == 0 && self.keep_last;
            if !pinned && (e.ts < floor || total + e.len as u64 + 1 > budget) {
                break;
            }
            total += e.len as u64 + 1;
            keep.push((*id, *e));
        }
        keep.reverse();
        let dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        let name = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
        let mut index = BTreeMap::new();
        {
            let mut src = std::fs::File::open(&self.path).ok();
            let mut out = BufWriter::new(open_new(&tmp)?);
            let mut offset = 0u64;
            for (id, e) in keep {
                let Some(src) = src.as_mut() else { break };
                let mut buf = vec![0u8; e.len as usize];
                src.seek(SeekFrom::Start(e.offset))?;
                src.read_exact(&mut buf)?;
                out.write_all(&buf)?;
                out.write_all(b"\n")?;
                index.insert(id, Entry { offset, ..e });
                offset += e.len as u64 + 1;
            }
            out.flush()?;
            out.get_ref().sync_all()?;
            inner.size = offset;
        }
        std::fs::rename(&tmp, &self.path)?;
        inner.index = index;
        Ok(())
    }
}

/// `"<id> <ts> ..."` -> (id, ts).
fn parse_head(line: &[u8]) -> Option<(u64, i64)> {
    let s = std::str::from_utf8(line.get(..line.len().min(48))?).ok()?;
    let mut it = s.splitn(3, ' ');
    let id = it.next()?.parse().ok()?;
    let ts = it.next()?.parse().ok()?;
    Some((id, ts))
}

fn open_append(path: &Path) -> std::io::Result<std::fs::File> {
    let mut o = std::fs::OpenOptions::new();
    o.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)
}

fn open_new(path: &Path) -> std::io::Result<std::fs::File> {
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)
}
