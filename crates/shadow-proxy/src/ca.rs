//! Local certificate authority used to terminate TLS for intercepted AI hosts.
//!
//! The CA (`ca/ca.pem` + `ca/ca.key`, key mode 0600) is created once per machine and
//! must be trusted by the OS / runtimes (`shadow install`). Leaf certificates are minted
//! on demand per host and cached in memory.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType,
    ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::ServerConfig;
use shadow_core::vault::write_atomic;
use time::{Duration, OffsetDateTime};

/// The local CA plus a per-host leaf cache.
pub struct CertAuthority {
    cert: Certificate,
    key: KeyPair,
    ca_der: CertificateDer<'static>,
    pem_path: PathBuf,
    cache: Mutex<HashMap<String, Arc<ServerConfig>>>,
}

fn ca_params() -> CertificateParams {
    let mut p = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "Shadow Local CA");
    dn.push(
        DnType::OrganizationName,
        "Shadow (local endpoint privacy proxy)",
    );
    p.distinguished_name = dn;
    p.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    p.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    p.not_before = OffsetDateTime::now_utc() - Duration::days(1);
    p.not_after = OffsetDateTime::now_utc() + Duration::days(3650);
    p
}

/// Rustls crypto provider used everywhere in Shadow.
pub fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

impl CertAuthority {
    /// Load the CA from `<dir>/ca.pem` + `<dir>/ca.key`, creating it if missing.
    pub fn load_or_create(dir: &Path) -> anyhow::Result<Self> {
        let (pem_path, key_path) = (dir.join("ca.pem"), dir.join("ca.key"));
        let (key, ca_pem) = if pem_path.exists() && key_path.exists() {
            let key = KeyPair::from_pem(&std::fs::read_to_string(&key_path)?)
                .context("parsing ca.key")?;
            (key, std::fs::read_to_string(&pem_path)?)
        } else {
            let key = KeyPair::generate()?;
            let cert = ca_params().self_signed(&key)?;
            write_atomic(&key_path, key.serialize_pem().as_bytes(), true)?;
            write_atomic(&pem_path, cert.pem().as_bytes(), false)?;
            (key, cert.pem())
        };
        // Re-create an issuer object with the same DN and key; leaves validate against
        // the on-disk certificate because subject and key identifier are identical.
        let cert = ca_params().self_signed(&key)?;
        let parsed = pem::parse(ca_pem.as_bytes()).context("parsing ca.pem")?;
        Ok(Self {
            cert,
            key,
            ca_der: CertificateDer::from(parsed.contents().to_vec()),
            pem_path,
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// Path to the CA certificate (PEM) that clients must trust.
    pub fn pem_path(&self) -> &Path {
        &self.pem_path
    }

    /// CA certificate as PEM.
    pub fn pem(&self) -> String {
        pem::encode(&pem::Pem::new("CERTIFICATE", self.ca_der.to_vec()))
    }

    /// SHA-256 fingerprint (hex, colon-separated) of the CA certificate.
    pub fn fingerprint(&self) -> String {
        use sha2::Digest;
        let digest = sha2::Sha256::digest(self.ca_der.as_ref());
        digest
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(":")
    }

    /// TLS server config presenting a leaf certificate for `host`.
    pub fn server_config(&self, host: &str) -> anyhow::Result<Arc<ServerConfig>> {
        let host = host.to_ascii_lowercase();
        if let Some(c) = self
            .cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&host)
        {
            return Ok(c.clone());
        }
        let leaf_key = KeyPair::generate()?;
        let mut p = CertificateParams::new(vec![host.clone()])?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, host.clone());
        p.distinguished_name = dn;
        p.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        p.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];
        p.not_before = OffsetDateTime::now_utc() - Duration::days(1);
        p.not_after = OffsetDateTime::now_utc() + Duration::days(390);
        let leaf = p.signed_by(&leaf_key, &self.cert, &self.key)?;
        let chain = vec![leaf.der().clone(), self.ca_der.clone()];
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der()));
        let mut cfg = ServerConfig::builder_with_provider(provider())
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(chain, key)?;
        cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
        let cfg = Arc::new(cfg);
        self.cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(host, cfg.clone());
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_reloads_and_mints_leaves() {
        let dir = tempfile::tempdir().unwrap();
        let a = CertAuthority::load_or_create(dir.path()).unwrap();
        let fp = a.fingerprint();
        a.server_config("api.openai.com").unwrap();
        let b = CertAuthority::load_or_create(dir.path()).unwrap();
        assert_eq!(fp, b.fingerprint());
        assert!(b.pem().starts_with("-----BEGIN CERTIFICATE-----"));
    }
}
