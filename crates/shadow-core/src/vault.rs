//! The vault: the real <-> fake mapping, encrypted at rest with ChaCha20-Poly1305.
//!
//! File format of `vault.bin`: `b"SHV1" || nonce(12) || ciphertext(JSON)`.
//! The 32-byte key lives in `vault.key` (hex, mode 0600). Every write is atomic
//! (temp file + rename). Several processes may share one data dir: before each
//! save the on-disk vault is merged in (union of pairs), so no mapping is lost.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{bail, Context};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};

const MAGIC: &[u8; 4] = b"SHV1";
const AAD: &[u8] = b"shadow-vault-v1";

/// One real <-> fake pair.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VaultEntry {
    /// Detector kind (`email`, `person`, ...).
    pub kind: String,
    pub real: String,
    pub fake: String,
    /// Unix seconds.
    pub created: i64,
}

/// Summary statistics of a vault.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VaultStats {
    pub entries: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Default)]
struct VaultFile {
    version: u32,
    created_at: String,
    entries: Vec<VaultEntry>,
}

/// In-memory vault.
pub struct Vault {
    path: PathBuf,
    cipher: ChaCha20Poly1305,
    created_at: String,
    entries: Vec<VaultEntry>,
    by_real: HashMap<String, usize>,
    by_fake: HashMap<String, usize>,
    /// Bumped on every change (used to invalidate automata).
    pub generation: u64,
    /// Bumped when entries are removed or reordered; between bumps, entries are only appended.
    pub epoch: u64,
    disk_stamp: Option<(SystemTime, u64)>,
}

impl Vault {
    /// Open (or create) the vault at `path` using the key at `key_path`.
    pub fn open(path: &Path, key_path: &Path) -> anyhow::Result<Vault> {
        let key = load_or_create_key(key_path)?;
        let mut v = Vault {
            path: path.to_path_buf(),
            cipher: ChaCha20Poly1305::new(Key::from_slice(&key)),
            created_at: chrono::Utc::now().to_rfc3339(),
            entries: vec![],
            by_real: HashMap::new(),
            by_fake: HashMap::new(),
            generation: 1,
            epoch: 0,
            disk_stamp: None,
        };
        if path.exists() {
            let file = v.read_disk()?;
            v.created_at = file.created_at;
            for e in file.entries {
                v.insert_entry(e);
            }
            v.disk_stamp = stamp(path);
        } else {
            v.save()?;
        }
        Ok(v)
    }

    fn read_disk(&self) -> anyhow::Result<VaultFile> {
        let raw = std::fs::read(&self.path)
            .with_context(|| format!("reading {}", self.path.display()))?;
        if raw.len() < 16 || &raw[..4] != MAGIC {
            bail!("{} is not a Shadow vault", self.path.display());
        }
        let nonce = Nonce::from_slice(&raw[4..16]);
        let plain = self
            .cipher
            .decrypt(
                nonce,
                Payload {
                    msg: &raw[16..],
                    aad: AAD,
                },
            )
            .map_err(|_| anyhow::anyhow!("vault decryption failed (wrong vault.key?)"))?;
        Ok(serde_json::from_slice(&plain)?)
    }

    /// Insert a pair unless it conflicts (fake already bound to a different real).
    /// Returns true when inserted.
    pub fn insert_entry(&mut self, e: VaultEntry) -> bool {
        if self.by_fake.contains_key(&e.fake) {
            return false;
        }
        let idx = self.entries.len();
        self.by_real.entry(e.real.clone()).or_insert(idx);
        self.by_fake.insert(e.fake.clone(), idx);
        self.entries.push(e);
        self.generation += 1;
        true
    }

    /// Insert a new mapping.
    pub fn insert(&mut self, kind: &str, real: &str, fake: &str) -> bool {
        self.insert_entry(VaultEntry {
            kind: kind.into(),
            real: real.into(),
            fake: fake.into(),
            created: chrono::Utc::now().timestamp(),
        })
    }

    /// Fake for a real value.
    pub fn fake_for(&self, real: &str) -> Option<&str> {
        self.by_real
            .get(real)
            .map(|&i| self.entries[i].fake.as_str())
    }

    /// Real value for a fake.
    pub fn real_for(&self, fake: &str) -> Option<&str> {
        self.by_fake
            .get(fake)
            .map(|&i| self.entries[i].real.as_str())
    }

    /// Entry for a real value.
    pub fn entry_for_real(&self, real: &str) -> Option<&VaultEntry> {
        self.by_real.get(real).map(|&i| &self.entries[i])
    }

    /// True if `s` is a fake or a real already in use (fakes must avoid both).
    pub fn is_taken(&self, s: &str) -> bool {
        self.by_fake.contains_key(s) || self.by_real.contains_key(s)
    }

    /// True if `s` is a known fake.
    pub fn is_fake(&self, s: &str) -> bool {
        self.by_fake.contains_key(s)
    }

    /// All entries.
    pub fn entries(&self) -> &[VaultEntry] {
        &self.entries
    }

    /// Remove all pairs whose real or fake equals `value` (case-sensitive). Returns count.
    pub fn forget(&mut self, value: &str) -> usize {
        let before = self.entries.len();
        let target_real: Vec<String> = self
            .entries
            .iter()
            .filter(|e| e.real == value || e.fake == value)
            .map(|e| e.real.clone())
            .collect();
        self.entries.retain(|e| !target_real.contains(&e.real));
        self.reindex();
        before - self.entries.len()
    }

    fn reindex(&mut self) {
        self.by_real.clear();
        self.by_fake.clear();
        for (i, e) in self.entries.iter().enumerate() {
            self.by_real.entry(e.real.clone()).or_insert(i);
            self.by_fake.insert(e.fake.clone(), i);
        }
        self.generation += 1;
        self.epoch += 1;
    }

    /// Statistics.
    pub fn stats(&self) -> VaultStats {
        let mut by_kind = BTreeMap::new();
        for e in &self.entries {
            *by_kind.entry(e.kind.clone()).or_insert(0) += 1;
        }
        VaultStats {
            entries: self.entries.len(),
            by_kind,
            created_at: self.created_at.clone(),
        }
    }

    /// Merge pairs written by other processes if the file changed on disk.
    /// Returns true if anything new was merged.
    pub fn refresh(&mut self) -> anyhow::Result<bool> {
        let now = stamp(&self.path);
        if now.is_none() || now == self.disk_stamp {
            return Ok(false);
        }
        let file = self.read_disk()?;
        let mut merged = false;
        for e in file.entries {
            if !self.by_fake.contains_key(&e.fake) {
                merged |= self.insert_entry(e);
            }
        }
        self.disk_stamp = now;
        Ok(merged)
    }

    /// Encrypt and atomically write the vault (merging concurrent writers first).
    pub fn save(&mut self) -> anyhow::Result<()> {
        if let Err(e) = self.refresh() {
            tracing::warn!("could not merge on-disk vault: {e:#}");
        }
        let file = VaultFile {
            version: 1,
            created_at: self.created_at.clone(),
            entries: self.entries.clone(),
        };
        let plain = serde_json::to_vec(&file)?;
        let mut nonce = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);
        let ct = self
            .cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &plain,
                    aad: AAD,
                },
            )
            .map_err(|_| anyhow::anyhow!("vault encryption failed"))?;
        let mut out = Vec::with_capacity(16 + ct.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        write_atomic(&self.path, &out, true)?;
        self.disk_stamp = stamp(&self.path);
        Ok(())
    }
}

fn stamp(path: &Path) -> Option<(SystemTime, u64)> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.modified().ok()?, m.len()))
}

fn load_or_create_key(path: &Path) -> anyhow::Result<[u8; 32]> {
    if path.exists() {
        let raw = std::fs::read_to_string(path)?;
        let bytes = hex::decode(raw.trim()).context("vault.key is not valid hex")?;
        let key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("vault.key must be 32 bytes"))?;
        return Ok(key);
    }
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    write_atomic(path, format!("{}\n", hex::encode(key)).as_bytes(), true)?;
    Ok(key)
}

/// Create a directory (and parents) that only the current user can open. An existing directory
/// keeps its permissions: it may be one the user chose and shares on purpose.
pub fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    let existed = dir.exists();
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    if !existed {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    let _ = existed;
    Ok(())
}

/// Write `data` to `path` atomically (temp file in the same dir + rename).
/// With `private`, the file is created with mode 0600 on Unix, in a 0700 directory if new.
pub fn write_atomic(path: &Path, data: &[u8], private: bool) -> anyhow::Result<()> {
    use std::io::Write;
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    if private {
        create_private_dir(dir)?;
    } else {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = dir.join(format!(
        ".{}.{}.{:x}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
        std::process::id(),
        rand::random::<u32>()
    ));
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(if private { 0o600 } else { 0o644 });
    }
    #[cfg(not(unix))]
    let _ = private;
    let mut f = opts
        .open(&tmp)
        .with_context(|| format!("creating {}", tmp.display()))?;
    f.write_all(data)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path).with_context(|| format!("renaming into {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encrypted_and_private() {
        let dir = tempfile::tempdir().unwrap();
        let (vp, kp) = (dir.path().join("vault.bin"), dir.path().join("vault.key"));
        {
            let mut v = Vault::open(&vp, &kp).unwrap();
            assert!(v.insert("email", "jane@acme.com", "wei.zhang@zhilan.cn"));
            assert!(!v.insert("email", "other@acme.com", "wei.zhang@zhilan.cn"));
            v.save().unwrap();
        }
        let raw = std::fs::read(&vp).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("acme"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&kp).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        let v = Vault::open(&vp, &kp).unwrap();
        assert_eq!(v.fake_for("jane@acme.com"), Some("wei.zhang@zhilan.cn"));
        assert_eq!(v.real_for("wei.zhang@zhilan.cn"), Some("jane@acme.com"));
    }

    #[test]
    fn concurrent_writers_merge() {
        let dir = tempfile::tempdir().unwrap();
        let (vp, kp) = (dir.path().join("vault.bin"), dir.path().join("vault.key"));
        let mut a = Vault::open(&vp, &kp).unwrap();
        let mut b = Vault::open(&vp, &kp).unwrap();
        a.insert("term", "Falcon", "Velora");
        a.save().unwrap();
        b.insert("term", "Osprey", "Quilent");
        b.save().unwrap();
        let c = Vault::open(&vp, &kp).unwrap();
        assert_eq!(c.entries().len(), 2);
    }
}
