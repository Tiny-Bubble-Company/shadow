//! Scan receipts: signed, chained evidence of what Shadow did with each request.
//!
//! Every request the proxy inspects gets a receipt: a COSE_Sign1 (RFC 9052) envelope,
//! tagged 18, whose payload is a CWT (RFC 8392) claims set profiled as an EAT (RFC 9711),
//! signed with the machine's Ed25519 key. The format follows the Attested Inference Receipt
//! (AIR v1, draft-tsyrulnikov-rats-attested-inference-receipt): Ed25519 only, protected
//! header `{1: -8, 3: 61}`, empty unprotected header, deterministic CBOR, a closed claims
//! map and the same four verification layers. AIR describes an inference inside a TEE; a
//! scan receipt describes what a data-protection proxy observed and changed, so it has its
//! own profile and claims. It also chains receipts (`prev_receipt`, `sequence`), which AIR
//! v1 leaves for later, so a deleted or edited receipt shows up as a break.
//!
//! A receipt never holds prompt text or values: hashes, counts, categories and the
//! provider host only. The original text is bound with a salted commitment so the hash
//! cannot be brute-forced from a short prompt; the salt stays with the incident details.
//!
//! What a receipt proves: this machine's key signed these claims, in this order. It does
//! not prove the machine was honest (a compromised machine can sign anything) and it is
//! not hardware-attested.

use std::collections::BTreeMap;
use std::path::Path;

use ed25519_dalek::{Signature, Signer as _, SigningKey, VerifyingKey};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// EAT profile of scan receipts.
pub const PROFILE: &str = "https://getshadow.dev/spec/scan-receipt/v1";
/// Largest receipt a verifier accepts, in bytes (as in AIR v1).
pub const MAX_RECEIPT: usize = 65_536;
/// File name of the signing key inside the data dir.
pub const KEY_FILE: &str = "receipt.key";

const TAG_COSE_SIGN1: u64 = 18;
const ALG_EDDSA: i64 = -8;
const CT_CWT: u64 = 61;

// Standard CWT / EAT claim keys.
const ISS: i64 = 1;
const IAT: i64 = 6;
const CTI: i64 = 7;
const EAT_PROFILE: i64 = 265;

// Scan receipt private claim keys (closed set; unknown keys are rejected).
const EVENT_ID: i64 = -70001;
const EVENT_TS_MS: i64 = -70002;
const SEQUENCE: i64 = -70003;
const PREV_RECEIPT: i64 = -70004;
const ACTION: i64 = -70005;
const PROVIDER: i64 = -70006;
const HOST: i64 = -70007;
const CLIENT: i64 = -70008;
const ENDPOINT: i64 = -70009;
const FINDINGS: i64 = -70010;
const DETECTORS: i64 = -70011;
const SENT_BODY_HASH: i64 = -70012;
const RESPONSE_HASH: i64 = -70013;
const DELIVERED_HASH: i64 = -70014;
const SENT_TEXT_HASH: i64 = -70015;
const ORIGINAL_TEXT_COMMITMENT: i64 = -70016;
const POLICY_HASH: i64 = -70017;
const SHADOW_VERSION: i64 = -70018;
const STATUS: i64 = -70019;
const RESTORED: i64 = -70020;
const BLOCK_REASON: i64 = -70021;
const INTENT: i64 = -70022;

const REQUIRED: &[i64] = &[
    ISS,
    IAT,
    CTI,
    EAT_PROFILE,
    EVENT_ID,
    EVENT_TS_MS,
    SEQUENCE,
    PREV_RECEIPT,
    ACTION,
    PROVIDER,
    HOST,
    CLIENT,
    ENDPOINT,
    FINDINGS,
    DETECTORS,
    POLICY_HASH,
    SHADOW_VERSION,
    STATUS,
    RESTORED,
];
const OPTIONAL: &[i64] = &[
    SENT_BODY_HASH,
    RESPONSE_HASH,
    DELIVERED_HASH,
    SENT_TEXT_HASH,
    ORIGINAL_TEXT_COMMITMENT,
    BLOCK_REASON,
    INTENT,
];
const ACTIONS: &[&str] = &["clean", "masked", "blocked", "passed"];
/// Longest text claim accepted (AIR recommends 1024 bytes).
const MAX_TEXT: usize = 1024;

pub type Hash = [u8; 32];

/// What one receipt says. Hashes are SHA-256.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Claims {
    /// `shadow-device:<key id>`.
    pub iss: String,
    /// Signing time, unix seconds.
    pub iat: u64,
    /// Random receipt id.
    #[serde(serialize_with = "ser_hex")]
    pub cti: [u8; 16],
    /// Local activity event id.
    pub event_id: u64,
    /// When the request was received, unix milliseconds.
    pub event_ts_ms: u64,
    /// Position in this machine's chain, from 1.
    pub sequence: u64,
    /// Hash of the previous receipt (all zeros for the first one).
    #[serde(serialize_with = "ser_hex")]
    pub prev_receipt: Hash,
    pub action: String,
    pub provider: String,
    pub host: String,
    pub client: String,
    pub endpoint: String,
    /// Values found per kind.
    pub findings: BTreeMap<String, u64>,
    pub detectors: Vec<String>,
    /// Exact bytes forwarded to the AI provider (absent when nothing was sent).
    #[serde(
        serialize_with = "ser_hex_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub sent_body_hash: Option<Hash>,
    /// Bytes the provider answered with, as received.
    #[serde(
        serialize_with = "ser_hex_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub response_hash: Option<Hash>,
    /// Bytes handed back to the app, after restoring real values.
    #[serde(
        serialize_with = "ser_hex_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub delivered_hash: Option<Hash>,
    /// The request text the provider received, as shown in the incident ([`sent_text_hash`]).
    #[serde(
        serialize_with = "ser_hex_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub sent_text_hash: Option<Hash>,
    /// Salted commitment to the text as the app wrote it ([`original_text_commitment`]).
    #[serde(
        serialize_with = "ser_hex_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub original_text_commitment: Option<Hash>,
    /// Hash of the effective Shadow configuration.
    #[serde(serialize_with = "ser_hex")]
    pub policy_hash: Hash,
    pub shadow_version: String,
    /// HTTP status returned to the app.
    pub status: u64,
    /// Values restored in the answer.
    pub restored: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
}

fn ser_hex<S: serde::Serializer, const N: usize>(v: &[u8; N], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&hex::encode(v))
}

fn ser_hex_opt<S: serde::Serializer>(v: &Option<Hash>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(h) => s.serialize_str(&hex::encode(h)),
        None => s.serialize_none(),
    }
}

/// SHA-256 of `bytes`.
pub fn sha256(bytes: &[u8]) -> Hash {
    Sha256::digest(bytes).into()
}

/// Hash that identifies a receipt (and links the next one to it): SHA-256 of its bytes.
pub fn receipt_hash(receipt: &[u8]) -> Hash {
    sha256(receipt)
}

/// Hash of the text the provider received: SHA-256 of the compact JSON array
/// `[[field, sent], ...]` over the parts that were sent, in order.
pub fn sent_text_hash<'a>(parts: impl IntoIterator<Item = (&'a str, Option<&'a str>)>) -> Hash {
    let list: Vec<(&str, &str)> = parts
        .into_iter()
        .filter_map(|(f, s)| Some((f, s?)))
        .collect();
    sha256(&serde_json::to_vec(&list).unwrap_or_default())
}

/// Commitment to the text as written: SHA-256 of `salt || [[field, original], ...]`.
pub fn original_text_commitment<'a>(
    salt: &[u8],
    parts: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Hash {
    let list: Vec<(&str, &str)> = parts.into_iter().collect();
    let mut h = Sha256::new();
    h.update(salt);
    h.update(serde_json::to_vec(&list).unwrap_or_default());
    h.finalize().into()
}

/// Key id of a public key: the first 8 bytes of its SHA-256, in hex.
pub fn key_id(public: &[u8; 32]) -> String {
    hex::encode(&sha256(public)[..8])
}

/// This machine's receipt signing key.
pub struct Signer {
    key: SigningKey,
    iss: String,
}

impl Signer {
    /// Load `<data_dir>/receipt.key`, creating it (0600) on first use.
    pub fn load_or_create(path: &Path) -> anyhow::Result<Self> {
        Ok(Self::from_seed(crate::vault::load_or_create_key(path)?))
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        let key = SigningKey::from_bytes(&seed);
        let iss = format!("shadow-device:{}", key_id(key.verifying_key().as_bytes()));
        Self { key, iss }
    }

    pub fn public_key(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }

    /// `shadow-device:<key id>`, the issuer of every receipt this key signs.
    pub fn issuer(&self) -> &str {
        &self.iss
    }

    /// Sign `claims` (its `iss` is set to this key's issuer) into a tagged COSE_Sign1.
    pub fn sign(&self, claims: &mut Claims) -> Vec<u8> {
        claims.iss = self.iss.clone();
        let payload = encode(&claims_to_cbor(claims));
        let protected = encode(&protected_header());
        let sig = self.key.sign(&sig_structure(&protected, &payload));
        encode(&Cbor::Tag(
            TAG_COSE_SIGN1,
            Box::new(Cbor::Array(vec![
                Cbor::Bytes(protected),
                Cbor::Map(vec![]),
                Cbor::Bytes(payload),
                Cbor::Bytes(sig.to_bytes().to_vec()),
            ])),
        ))
    }
}

fn protected_header() -> Cbor {
    Cbor::Map(vec![
        (Cbor::int(1), Cbor::int(ALG_EDDSA)),
        (Cbor::int(3), Cbor::Uint(CT_CWT)),
    ])
}

fn sig_structure(protected: &[u8], payload: &[u8]) -> Vec<u8> {
    encode(&Cbor::Array(vec![
        Cbor::Text("Signature1".into()),
        Cbor::Bytes(protected.to_vec()),
        Cbor::Bytes(vec![]),
        Cbor::Bytes(payload.to_vec()),
    ]))
}

fn claims_to_cbor(c: &Claims) -> Cbor {
    let text = |s: &str| Cbor::Text(s.to_string());
    let bytes = |b: &[u8]| Cbor::Bytes(b.to_vec());
    let mut m = vec![
        (Cbor::int(ISS), text(&c.iss)),
        (Cbor::int(IAT), Cbor::Uint(c.iat)),
        (Cbor::int(CTI), bytes(&c.cti)),
        (Cbor::int(EAT_PROFILE), text(PROFILE)),
        (Cbor::int(EVENT_ID), Cbor::Uint(c.event_id)),
        (Cbor::int(EVENT_TS_MS), Cbor::Uint(c.event_ts_ms)),
        (Cbor::int(SEQUENCE), Cbor::Uint(c.sequence)),
        (Cbor::int(PREV_RECEIPT), bytes(&c.prev_receipt)),
        (Cbor::int(ACTION), text(&c.action)),
        (Cbor::int(PROVIDER), text(&c.provider)),
        (Cbor::int(HOST), text(&c.host)),
        (Cbor::int(CLIENT), text(&c.client)),
        (Cbor::int(ENDPOINT), text(&c.endpoint)),
        (
            Cbor::int(FINDINGS),
            Cbor::Map(
                c.findings
                    .iter()
                    .map(|(k, v)| (text(k), Cbor::Uint(*v)))
                    .collect(),
            ),
        ),
        (
            Cbor::int(DETECTORS),
            Cbor::Array(c.detectors.iter().map(|d| text(d)).collect()),
        ),
        (Cbor::int(POLICY_HASH), bytes(&c.policy_hash)),
        (Cbor::int(SHADOW_VERSION), text(&c.shadow_version)),
        (Cbor::int(STATUS), Cbor::Uint(c.status)),
        (Cbor::int(RESTORED), Cbor::Uint(c.restored)),
    ];
    for (k, h) in [
        (SENT_BODY_HASH, &c.sent_body_hash),
        (RESPONSE_HASH, &c.response_hash),
        (DELIVERED_HASH, &c.delivered_hash),
        (SENT_TEXT_HASH, &c.sent_text_hash),
        (ORIGINAL_TEXT_COMMITMENT, &c.original_text_commitment),
    ] {
        if let Some(h) = h {
            m.push((Cbor::int(k), bytes(h)));
        }
    }
    for (k, s) in [(BLOCK_REASON, &c.block_reason), (INTENT, &c.intent)] {
        if let Some(s) = s {
            m.push((Cbor::int(k), text(s)));
        }
    }
    Cbor::Map(m)
}

// ----------------------------------------------------------------------------------------
// Verification

/// Which verification layer failed (the AIR v1 layers).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Parse,
    Signature,
    Claims,
    Policy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VerifyError {
    pub layer: Layer,
    pub message: String,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.layer, self.message)
    }
}

impl std::error::Error for VerifyError {}

fn fail<T>(layer: Layer, message: impl Into<String>) -> Result<T, VerifyError> {
    Err(VerifyError {
        layer,
        message: message.into(),
    })
}

/// Layers 1-3: parse the receipt, check its Ed25519 signature against `public_key`
/// (strict: non-canonical signatures are rejected) and validate every claim. Contextual
/// checks (layer 4: chain, content, key pinning) are up to the caller; see
/// [`check_chain`] and [`check_content`].
pub fn verify(receipt: &[u8], public_key: &[u8; 32]) -> Result<Claims, VerifyError> {
    // Layer 1: parse.
    if receipt.len() > MAX_RECEIPT {
        return fail(Layer::Parse, "receipt is larger than 64 KiB");
    }
    let outer = decode(receipt).or_else(|e| fail(Layer::Parse, e))?;
    let Cbor::Tag(TAG_COSE_SIGN1, inner) = outer else {
        return fail(Layer::Parse, "not a tagged COSE_Sign1 (tag 18)");
    };
    let Cbor::Array(items) = *inner else {
        return fail(Layer::Parse, "COSE_Sign1 is not an array");
    };
    let [Cbor::Bytes(protected), Cbor::Map(unprotected), Cbor::Bytes(payload), Cbor::Bytes(sig)] =
        items.as_slice()
    else {
        return fail(Layer::Parse, "COSE_Sign1 must have 4 elements");
    };
    let header =
        decode(protected).or_else(|e| fail(Layer::Parse, format!("protected header: {e}")))?;
    if encode(&header) != *protected {
        return fail(
            Layer::Parse,
            "protected header is not deterministically encoded",
        );
    }
    let Cbor::Map(h) = &header else {
        return fail(Layer::Parse, "protected header is not a map");
    };
    if map_get(h, 1) != Some(&Cbor::int(ALG_EDDSA)) || h.len() != 2 {
        return fail(Layer::Parse, "algorithm must be EdDSA (-8)");
    }
    if map_get(h, 3) != Some(&Cbor::Uint(CT_CWT)) {
        return fail(Layer::Parse, "content type must be application/cwt (61)");
    }
    if !unprotected.is_empty() {
        return fail(Layer::Parse, "unprotected header must be empty");
    }
    let claims = decode(payload).or_else(|e| fail(Layer::Parse, format!("payload: {e}")))?;
    let Cbor::Map(m) = &claims else {
        return fail(Layer::Parse, "payload is not a map");
    };
    if map_get(m, EAT_PROFILE) != Some(&Cbor::Text(PROFILE.into())) {
        return fail(Layer::Parse, format!("eat_profile must be {PROFILE}"));
    }

    // Layer 2: signature.
    let Ok(sig) = <[u8; 64]>::try_from(sig.as_slice()) else {
        return fail(Layer::Signature, "signature must be 64 bytes");
    };
    let Ok(key) = VerifyingKey::from_bytes(public_key) else {
        return fail(Layer::Signature, "public key is not a valid Ed25519 key");
    };
    if key
        .verify_strict(
            &sig_structure(protected, payload),
            &Signature::from_bytes(&sig),
        )
        .is_err()
    {
        return fail(Layer::Signature, "signature does not match this device key");
    }

    // Layer 3: claims.
    if encode(&claims) != *payload {
        return fail(Layer::Claims, "payload is not deterministically encoded");
    }
    let c = claims_from_map(m)?;
    if c.iss != format!("shadow-device:{}", key_id(public_key)) {
        return fail(Layer::Claims, "issuer does not name this device key");
    }
    Ok(c)
}

fn claims_from_map(m: &[(Cbor, Cbor)]) -> Result<Claims, VerifyError> {
    let mut seen = std::collections::BTreeSet::new();
    for (k, _) in m {
        let Some(k) = k.as_int() else {
            return fail(Layer::Claims, "claim keys must be integers");
        };
        if !REQUIRED.contains(&k) && !OPTIONAL.contains(&k) {
            return fail(Layer::Claims, format!("unknown claim {k}"));
        }
        if !seen.insert(k) {
            return fail(Layer::Claims, format!("duplicate claim {k}"));
        }
    }
    if let Some(k) = REQUIRED.iter().find(|k| !seen.contains(k)) {
        return fail(Layer::Claims, format!("missing claim {k}"));
    }
    let get = |k| map_get(m, k).expect("checked above");
    let text = |k: i64| -> Result<String, VerifyError> {
        match get(k) {
            Cbor::Text(s) if s.len() <= MAX_TEXT => Ok(s.clone()),
            _ => fail(
                Layer::Claims,
                format!("claim {k} must be text of at most {MAX_TEXT} bytes"),
            ),
        }
    };
    let uint = |k: i64| -> Result<u64, VerifyError> {
        match get(k) {
            Cbor::Uint(n) => Ok(*n),
            _ => fail(
                Layer::Claims,
                format!("claim {k} must be an unsigned integer"),
            ),
        }
    };
    let hash = |k: i64| -> Result<Hash, VerifyError> {
        match get(k) {
            Cbor::Bytes(b) => b
                .as_slice()
                .try_into()
                .or_else(|_| fail(Layer::Claims, format!("claim {k} must be 32 bytes"))),
            _ => fail(Layer::Claims, format!("claim {k} must be bytes")),
        }
    };
    let opt_hash = |k: i64| seen.contains(&k).then(|| hash(k)).transpose();
    let opt_text = |k: i64| seen.contains(&k).then(|| text(k)).transpose();
    let cti = match get(CTI) {
        Cbor::Bytes(b) if b.len() == 16 => {
            let mut a = [0u8; 16];
            a.copy_from_slice(b);
            a
        }
        _ => return fail(Layer::Claims, "cti must be 16 bytes"),
    };
    let findings = match get(FINDINGS) {
        Cbor::Map(f) => {
            let mut out = BTreeMap::new();
            for (k, v) in f {
                match (k, v) {
                    (Cbor::Text(k), Cbor::Uint(n)) if k.len() <= 64 => {
                        out.insert(k.clone(), *n);
                    }
                    _ => return fail(Layer::Claims, "findings must map text to counts"),
                }
            }
            out
        }
        _ => return fail(Layer::Claims, "findings must be a map"),
    };
    let detectors = match get(DETECTORS) {
        Cbor::Array(a) => a
            .iter()
            .map(|d| match d {
                Cbor::Text(s) if s.len() <= 64 => Ok(s.clone()),
                _ => fail(Layer::Claims, "detectors must be short text"),
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return fail(Layer::Claims, "detectors must be an array"),
    };
    let c = Claims {
        iss: text(ISS)?,
        iat: uint(IAT)?,
        cti,
        event_id: uint(EVENT_ID)?,
        event_ts_ms: uint(EVENT_TS_MS)?,
        sequence: uint(SEQUENCE)?,
        prev_receipt: hash(PREV_RECEIPT)?,
        action: text(ACTION)?,
        provider: text(PROVIDER)?,
        host: text(HOST)?,
        client: text(CLIENT)?,
        endpoint: text(ENDPOINT)?,
        findings,
        detectors,
        sent_body_hash: opt_hash(SENT_BODY_HASH)?,
        response_hash: opt_hash(RESPONSE_HASH)?,
        delivered_hash: opt_hash(DELIVERED_HASH)?,
        sent_text_hash: opt_hash(SENT_TEXT_HASH)?,
        original_text_commitment: opt_hash(ORIGINAL_TEXT_COMMITMENT)?,
        policy_hash: hash(POLICY_HASH)?,
        shadow_version: text(SHADOW_VERSION)?,
        status: uint(STATUS)?,
        restored: uint(RESTORED)?,
        block_reason: opt_text(BLOCK_REASON)?,
        intent: opt_text(INTENT)?,
    };
    if c.iat == 0 || c.sequence == 0 || c.event_id == 0 {
        return fail(Layer::Claims, "iat, sequence and event id must be non-zero");
    }
    if !ACTIONS.contains(&c.action.as_str()) {
        return fail(Layer::Claims, format!("unknown action {}", c.action));
    }
    if c.iss.is_empty() || c.shadow_version.is_empty() {
        return fail(Layer::Claims, "iss and shadow version must not be empty");
    }
    if c.action == "blocked" && (c.sent_body_hash.is_some() || c.response_hash.is_some()) {
        return fail(
            Layer::Claims,
            "a blocked request cannot have sent or response hashes",
        );
    }
    if c.sequence == 1 && c.prev_receipt != [0; 32] {
        return fail(
            Layer::Claims,
            "the first receipt must not link to a previous one",
        );
    }
    Ok(c)
}

/// Layer 4, CHAIN: `next` directly follows the receipt `prev` (whose hash is `prev_hash`).
pub fn check_chain(prev: &Claims, prev_hash: &Hash, next: &Claims) -> Result<(), VerifyError> {
    if next.sequence != prev.sequence + 1 {
        return fail(
            Layer::Policy,
            format!(
                "sequence jumps from {} to {}: {} receipt(s) missing",
                prev.sequence,
                next.sequence,
                next.sequence.saturating_sub(prev.sequence + 1)
            ),
        );
    }
    if &next.prev_receipt != prev_hash {
        return fail(Layer::Policy, "does not link to the previous receipt");
    }
    if next.event_ts_ms < prev.event_ts_ms.saturating_sub(60_000) {
        return fail(Layer::Policy, "goes back in time by more than a minute");
    }
    Ok(())
}

/// Layer 4, CONTENT: the incident text shown matches what was signed. `parts` are
/// `(field, original, sent)`; pass `salt` when the original text is available.
pub fn check_content(
    c: &Claims,
    parts: &[(String, Option<String>, Option<String>)],
    salt: Option<&[u8]>,
) -> Result<ContentCheck, VerifyError> {
    let mut out = ContentCheck::default();
    if let Some(expected) = c.sent_text_hash {
        let got = sent_text_hash(parts.iter().map(|(f, _, s)| (f.as_str(), s.as_deref())));
        if got != expected {
            return fail(
                Layer::Policy,
                "the text the AI received differs from what was signed",
            );
        }
        out.sent_text = true;
    }
    if let (Some(expected), Some(salt)) = (c.original_text_commitment, salt) {
        let originals: Option<Vec<(&str, &str)>> = parts
            .iter()
            .map(|(f, o, _)| Some((f.as_str(), o.as_deref()?)))
            .collect();
        if let Some(originals) = originals.filter(|o| !o.is_empty()) {
            if original_text_commitment(salt, originals) != expected {
                return fail(
                    Layer::Policy,
                    "the original prompt differs from what was signed",
                );
            }
            out.original_text = true;
        }
    }
    Ok(out)
}

/// Which content checks ran (and passed).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ContentCheck {
    pub sent_text: bool,
    pub original_text: bool,
}

/// Read the claims without checking the signature (for display and chain bookkeeping only).
pub fn peek(receipt: &[u8]) -> Option<Claims> {
    let Cbor::Tag(TAG_COSE_SIGN1, inner) = decode(receipt).ok()? else {
        return None;
    };
    let Cbor::Array(items) = *inner else {
        return None;
    };
    let Some(Cbor::Bytes(payload)) = items.get(2) else {
        return None;
    };
    let Cbor::Map(m) = decode(payload).ok()? else {
        return None;
    };
    claims_from_map(&m).ok()
}

// ----------------------------------------------------------------------------------------
// Minimal deterministic CBOR (RFC 8949 §4.2.1): definite lengths, shortest heads, map keys
// sorted by their encoded bytes. Only the types receipts use; anything else is rejected.

#[derive(Clone, Debug, PartialEq, Eq)]
enum Cbor {
    Uint(u64),
    /// Negative integer `-1 - n`.
    Nint(u64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<Cbor>),
    Map(Vec<(Cbor, Cbor)>),
    Tag(u64, Box<Cbor>),
}

impl Cbor {
    fn int(n: i64) -> Cbor {
        if n >= 0 {
            Cbor::Uint(n as u64)
        } else {
            Cbor::Nint((-1 - n) as u64)
        }
    }

    fn as_int(&self) -> Option<i64> {
        match self {
            Cbor::Uint(n) => i64::try_from(*n).ok(),
            Cbor::Nint(n) => i64::try_from(*n).ok().map(|n| -1 - n),
            _ => None,
        }
    }
}

fn map_get(m: &[(Cbor, Cbor)], key: i64) -> Option<&Cbor> {
    m.iter()
        .find(|(k, _)| k.as_int() == Some(key))
        .map(|(_, v)| v)
}

fn head(out: &mut Vec<u8>, major: u8, n: u64) {
    let m = major << 5;
    match n {
        0..=23 => out.push(m | n as u8),
        24..=0xff => out.extend([m | 24, n as u8]),
        0x100..=0xffff => {
            out.push(m | 25);
            out.extend((n as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(m | 26);
            out.extend((n as u32).to_be_bytes());
        }
        _ => {
            out.push(m | 27);
            out.extend(n.to_be_bytes());
        }
    }
}

fn encode(v: &Cbor) -> Vec<u8> {
    let mut out = vec![];
    encode_into(v, &mut out);
    out
}

fn encode_into(v: &Cbor, out: &mut Vec<u8>) {
    match v {
        Cbor::Uint(n) => head(out, 0, *n),
        Cbor::Nint(n) => head(out, 1, *n),
        Cbor::Bytes(b) => {
            head(out, 2, b.len() as u64);
            out.extend(b);
        }
        Cbor::Text(s) => {
            head(out, 3, s.len() as u64);
            out.extend(s.as_bytes());
        }
        Cbor::Array(a) => {
            head(out, 4, a.len() as u64);
            for x in a {
                encode_into(x, out);
            }
        }
        Cbor::Map(m) => {
            let mut entries: Vec<(Vec<u8>, Vec<u8>)> =
                m.iter().map(|(k, v)| (encode(k), encode(v))).collect();
            entries.sort();
            head(out, 5, entries.len() as u64);
            for (k, v) in entries {
                out.extend(k);
                out.extend(v);
            }
        }
        Cbor::Tag(t, x) => {
            head(out, 6, *t);
            encode_into(x, out);
        }
    }
}

fn decode(bytes: &[u8]) -> Result<Cbor, String> {
    let mut pos = 0;
    let v = decode_at(bytes, &mut pos, 0)?;
    if pos != bytes.len() {
        return Err("trailing bytes after the CBOR item".into());
    }
    Ok(v)
}

fn decode_at(b: &[u8], pos: &mut usize, depth: u32) -> Result<Cbor, String> {
    if depth > 16 {
        return Err("CBOR nested too deeply".into());
    }
    let first = *b.get(*pos).ok_or("truncated CBOR")?;
    *pos += 1;
    let major = first >> 5;
    let info = first & 0x1f;
    let mut take = |n: usize| -> Result<&[u8], String> {
        let s = b.get(*pos..*pos + n).ok_or("truncated CBOR")?;
        *pos += n;
        Ok(s)
    };
    let n: u64 = match info {
        0..=23 => info as u64,
        24 => take(1)?[0] as u64,
        25 => u16::from_be_bytes(take(2)?.try_into().unwrap()) as u64,
        26 => u32::from_be_bytes(take(4)?.try_into().unwrap()) as u64,
        27 => u64::from_be_bytes(take(8)?.try_into().unwrap()),
        _ => return Err("indefinite lengths and reserved values are not allowed".into()),
    };
    let len = |n: u64| -> Result<usize, String> {
        usize::try_from(n)
            .ok()
            .filter(|n| *n <= MAX_RECEIPT)
            .ok_or_else(|| "CBOR length too large".to_string())
    };
    Ok(match major {
        0 => Cbor::Uint(n),
        1 => Cbor::Nint(n),
        2 => Cbor::Bytes(take(len(n)?)?.to_vec()),
        3 => Cbor::Text(
            String::from_utf8(take(len(n)?)?.to_vec()).map_err(|_| "invalid UTF-8 text")?,
        ),
        4 => {
            let n = len(n)?;
            let mut a = Vec::with_capacity(n.min(64));
            for _ in 0..n {
                a.push(decode_at(b, pos, depth + 1)?);
            }
            Cbor::Array(a)
        }
        5 => {
            let n = len(n)?;
            let mut m = Vec::with_capacity(n.min(64));
            for _ in 0..n {
                let k = decode_at(b, pos, depth + 1)?;
                let v = decode_at(b, pos, depth + 1)?;
                m.push((k, v));
            }
            Cbor::Map(m)
        }
        6 => Cbor::Tag(n, Box::new(decode_at(b, pos, depth + 1)?)),
        _ => return Err("floats and simple values are not allowed".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(seq: u64, prev: Hash) -> Claims {
        Claims {
            iat: 1_780_000_000,
            cti: [7; 16],
            event_id: seq + 100,
            event_ts_ms: 1_780_000_000_000 + seq,
            sequence: seq,
            prev_receipt: prev,
            action: "masked".into(),
            provider: "openai".into(),
            host: "api.openai.com".into(),
            client: "curl".into(),
            endpoint: "chat".into(),
            findings: [("email".to_string(), 2)].into(),
            detectors: vec!["email".into()],
            sent_body_hash: Some(sha256(b"sent")),
            response_hash: Some(sha256(b"answer")),
            delivered_hash: Some(sha256(b"restored")),
            sent_text_hash: Some(sent_text_hash([(
                "messages[0].content",
                Some("hi bob@x.cn"),
            )])),
            original_text_commitment: Some(original_text_commitment(
                b"salt",
                [("messages[0].content", "hi ann@acme.com")],
            )),
            policy_hash: sha256(b"config"),
            shadow_version: "0.1.0".into(),
            status: 200,
            restored: 1,
            intent: Some("converse".into()),
            ..Claims::default()
        }
    }

    #[test]
    fn signed_receipts_verify_and_chain() {
        let s = Signer::from_seed([1; 32]);
        let mut a = claims(1, [0; 32]);
        let ra = s.sign(&mut a);
        let mut b = claims(2, receipt_hash(&ra));
        let rb = s.sign(&mut b);
        let va = verify(&ra, &s.public_key()).unwrap();
        let vb = verify(&rb, &s.public_key()).unwrap();
        assert_eq!(va, a);
        assert_eq!(vb, b);
        check_chain(&va, &receipt_hash(&ra), &vb).unwrap();
        assert_eq!(peek(&rb), Some(b.clone()));
        let parts = vec![(
            "messages[0].content".to_string(),
            Some("hi ann@acme.com".to_string()),
            Some("hi bob@x.cn".to_string()),
        )];
        let ok = check_content(&vb, &parts, Some(b"salt")).unwrap();
        assert!(ok.sent_text && ok.original_text);

        // A gap, a wrong link, edited text and a wrong salt are all caught.
        let mut c = claims(4, receipt_hash(&rb));
        let rc = s.sign(&mut c);
        let vc = verify(&rc, &s.public_key()).unwrap();
        assert!(check_chain(&vb, &receipt_hash(&rb), &vc)
            .unwrap_err()
            .message
            .contains("1 receipt(s) missing"));
        assert!(check_chain(&va, &receipt_hash(&ra), &vc).is_err());
        let mut edited = parts.clone();
        edited[0].2 = Some("hi ann@acme.com".into());
        assert!(check_content(&vb, &edited, None).is_err());
        assert!(check_content(&vb, &parts, Some(b"pepper")).is_err());
    }

    #[test]
    fn tampering_is_rejected_at_the_right_layer() {
        let s = Signer::from_seed([2; 32]);
        let mut c = claims(1, [0; 32]);
        let r = s.sign(&mut c);
        let other = Signer::from_seed([3; 32]);
        assert_eq!(
            verify(&r, &other.public_key()).unwrap_err().layer,
            Layer::Signature
        );

        // Flip a byte inside the payload (the host name).
        let mut bad = r.clone();
        let i = bad.windows(6).position(|w| w == b"openai").unwrap();
        bad[i] = b'O';
        assert_eq!(
            verify(&bad, &s.public_key()).unwrap_err().layer,
            Layer::Signature
        );

        // Untagged, wrong algorithm, unprotected header, unknown claim.
        assert_eq!(
            verify(&r[1..], &s.public_key()).unwrap_err().layer,
            Layer::Parse
        );
        let Cbor::Tag(_, inner) = decode(&r).unwrap() else {
            panic!()
        };
        let Cbor::Array(items) = *inner else { panic!() };
        let rebuild = |items: Vec<Cbor>| encode(&Cbor::Tag(18, Box::new(Cbor::Array(items))));
        let mut wrong_alg = items.clone();
        wrong_alg[0] = Cbor::Bytes(encode(&Cbor::Map(vec![
            (Cbor::int(1), Cbor::int(-7)),
            (Cbor::int(3), Cbor::Uint(61)),
        ])));
        assert_eq!(
            verify(&rebuild(wrong_alg), &s.public_key())
                .unwrap_err()
                .layer,
            Layer::Parse
        );
        let mut kid = items.clone();
        kid[1] = Cbor::Map(vec![(Cbor::int(4), Cbor::Bytes(vec![1]))]);
        assert_eq!(
            verify(&rebuild(kid), &s.public_key()).unwrap_err().layer,
            Layer::Parse
        );

        let Cbor::Map(mut m) = claims_to_cbor(&c) else {
            panic!()
        };
        m.push((Cbor::int(-70099), Cbor::Uint(1)));
        let payload = encode(&Cbor::Map(m));
        let protected = encode(&protected_header());
        let sig = s.key.sign(&sig_structure(&protected, &payload));
        let extra = rebuild(vec![
            Cbor::Bytes(protected),
            Cbor::Map(vec![]),
            Cbor::Bytes(payload),
            Cbor::Bytes(sig.to_bytes().to_vec()),
        ]);
        let e = verify(&extra, &s.public_key()).unwrap_err();
        assert_eq!(e.layer, Layer::Claims);
        assert!(e.message.contains("unknown claim"), "{e}");
    }

    #[test]
    fn encoding_is_deterministic_and_minimal() {
        assert_eq!(encode(&Cbor::Uint(23)), [0x17]);
        assert_eq!(encode(&Cbor::Uint(24)), [0x18, 24]);
        assert_eq!(encode(&Cbor::int(-8)), [0x27]);
        assert_eq!(encode(&Cbor::int(-70001)), [0x3a, 0, 1, 0x11, 0x70]);
        // Keys are ordered by their encoding: 1, 3, then the negative ones.
        let m = Cbor::Map(vec![
            (Cbor::int(-1), Cbor::Uint(0)),
            (Cbor::int(3), Cbor::Uint(0)),
            (Cbor::int(1), Cbor::Uint(0)),
        ]);
        assert_eq!(encode(&m), [0xa3, 0x01, 0x00, 0x03, 0x00, 0x20, 0x00]);
        assert_eq!(decode(&encode(&m)).map(|v| encode(&v)), Ok(encode(&m)));
        assert!(decode(&[0x5f]).is_err());
        assert!(decode(&[0xf9, 0, 0]).is_err());
    }
}
