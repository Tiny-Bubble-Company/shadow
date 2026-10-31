//! # shadow-core
//!
//! Reversible, format-preserving pseudonymisation for AI traffic.
//!
//! [`Shadow`] detects sensitive values (PII, secrets, organisation terms, local machine
//! facts), replaces them with consistent fakes drawn from a persistent [`Persona`], keeps
//! the real<->fake mapping in an encrypted on-disk vault, and translates responses back.
//!
//! ```no_run
//! use shadow_core::{Shadow, ShadowConfig};
//! let shadow = Shadow::open(ShadowConfig::from_dir("/tmp/shadow").with_terms(vec!["Acme GmbH".into()]))?;
//! let masked = shadow.mask("Email jane@acme.com about the Acme GmbH contract");
//! assert!(!masked.text.contains("acme"));
//! assert_eq!(shadow.unmask(&masked.text), "Email jane@acme.com about the Acme GmbH contract");
//! # anyhow::Ok(())
//! ```

pub mod config;
pub mod detect;
mod fake;
pub mod files;
pub mod intent;
pub mod local;
pub mod persona;
pub mod translate;
pub mod vault;

use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use anyhow::Context;
use serde_json::Value;

pub use config::{ShadowConfig, Treatment};
pub use detect::{Detector, Finding, Kind};
pub use intent::{Classification, Intent, RequestContext, ToolInfo};
pub use local::LocalFacts;
pub use persona::Persona;
pub use translate::{Masked, StreamUnmasker};
pub use vault::{VaultEntry, VaultStats};

use detect::dictionary::{is_org_name, strip_legal_suffix, DictionaryDetector};
use fake::Faker;
use translate::{walk_all_strings_mut, walk_strings_mut, Unmasker};
use vault::{write_atomic, Vault};

/// The Shadow engine. Cheap to clone; `Send + Sync`.
#[derive(Clone)]
pub struct Shadow {
    inner: Arc<Inner>,
}

struct Inner {
    config: ShadowConfig,
    persona: Persona,
    detectors: Vec<Box<dyn Detector>>,
    primary_org: Option<(String, String)>,
    allow: HashSet<String>,
    state: RwLock<State>,
    last_refresh: Mutex<Instant>,
}

struct State {
    vault: Vault,
    terms: Vec<(String, Kind)>,
    terms_gen: u64,
    /// Dictionary over terms and the vault up to `dict_base_len` entries, rebuilt rarely...
    dict_base: Arc<DictionaryDetector>,
    dict_base_key: (u64, u64),
    dict_base_len: usize,
    /// ...plus a small one over entries appended since, rebuilt on every vault change.
    dict_recent: Arc<DictionaryDetector>,
    dict_key: (u64, u64),
    unmasker: Arc<Unmasker>,
    unmasker_gen: u64,
}

/// Model and Presidio spans are coarse: they can carry punctuation ("Garrett Hernandez <") or
/// swallow a value the rules already found precisely (an email, a key). Trim them to word
/// characters and let precise structured findings win; the models only fill gaps.
fn refine_ml_findings(mut all: Vec<Finding>, text: &str) -> Vec<Finding> {
    let is_ml = |f: &Finding| detect::public::ML_DETECTORS.contains(&f.detector);
    for f in all.iter_mut().filter(|f| is_ml(f)) {
        let span = &text[f.start..f.end];
        let lead = span.len()
            - span
                .trim_start_matches(|c: char| !c.is_alphanumeric())
                .len();
        // Organisations keep a final full stop ("Acme Inc.").
        let keep_dot = f.kind == Kind::Org;
        let trail = span.len()
            - span
                .trim_end_matches(|c: char| !(c.is_alphanumeric() || keep_dot && c == '.'))
                .len();
        if lead + trail >= span.len() {
            f.end = f.start;
        } else {
            f.start += lead;
            f.end -= trail;
        }
    }
    let precise: Vec<(usize, usize)> = all
        .iter()
        .filter(|f| {
            !is_ml(f)
                && !matches!(
                    f.kind,
                    Kind::Person | Kind::Org | Kind::Address | Kind::Term
                )
        })
        .map(|f| (f.start, f.end))
        .collect();
    all.retain(|f| {
        f.start < f.end && !(is_ml(f) && precise.iter().any(|&(s, e)| f.start < e && s < f.end))
    });
    all
}

/// Dictionary terms for vault entries: each real value, and each fake too, because real text
/// that happens to equal a fake (a real "Carvalho" when "Carvalho" is someone else's stand-in)
/// must be masked as well, or unmasking would turn it into the wrong person.
fn vault_terms(entries: &[vault::VaultEntry]) -> Vec<(String, Kind)> {
    let mut out = vec![];
    for e in entries {
        if let Some(k) = Kind::parse(&e.kind) {
            if k == Kind::Secret {
                continue;
            }
            if e.real.len() >= 4 {
                out.push((e.real.clone(), k));
            }
            if e.fake.len() >= 3 {
                out.push((e.fake.clone(), k));
            }
        }
    }
    out
}

/// Parse `kind:value` term syntax (`person:Jane Doe`, `org:Globex`); otherwise infer.
pub fn parse_term(raw: &str, orgs: &[String]) -> (String, Kind) {
    let raw = raw.trim();
    if let Some((k, v)) = raw.split_once(':') {
        if let Some(kind) = Kind::parse(k.trim()) {
            return (v.trim().to_string(), kind);
        }
    }
    let kind = if orgs
        .iter()
        .any(|o| o.eq_ignore_ascii_case(raw) || strip_legal_suffix(o).eq_ignore_ascii_case(raw))
        || is_org_name(raw)
    {
        Kind::Org
    } else if raw.contains('@') && !raw.contains(' ') {
        Kind::Email
    } else if raw.contains('.') && !raw.contains(' ') {
        Kind::Hostname
    } else {
        Kind::Term
    };
    (raw.to_string(), kind)
}

impl Shadow {
    /// Open (or create) persona, vault and key in `config.data_dir`.
    pub fn open(config: ShadowConfig) -> anyhow::Result<Shadow> {
        Self::open_with_detectors(config, Vec::new())
    }

    /// Like [`Shadow::open`] with additional custom detectors.
    pub fn open_with_detectors(
        config: ShadowConfig,
        extra: Vec<Box<dyn Detector>>,
    ) -> anyhow::Result<Shadow> {
        vault::create_private_dir(&config.data_dir)
            .with_context(|| format!("creating data dir {}", config.data_dir.display()))?;
        let persona = load_or_create_persona(&config)?;
        let mut vault = Vault::open(&config.vault_path(), &config.key_path())?;

        let mut detectors = detect::patterns::builtin(&config);
        if config.model_detector.enabled {
            match detect::model::ModelDetector::new(&config.model_detector) {
                Ok(d) => detectors.push(Box::new(d)),
                Err(e) => tracing::warn!("model detector disabled: {e:#}"),
            }
        }
        if let Some(d) = detect::presidio::presidio_detector(&config) {
            detectors.push(d);
        }
        detectors.extend(extra);

        // Organisations: explicit list, else terms with a legal suffix.
        let mut orgs = config.organization.clone();
        orgs.extend(config.terms.iter().filter(|t| is_org_name(t)).cloned());
        let primary_org = orgs
            .first()
            .map(|o| (o.trim().to_string(), strip_legal_suffix(o).to_string()));

        let mut raw_terms: Vec<String> = orgs.clone();
        raw_terms.extend(config.terms.iter().cloned());
        raw_terms.extend(read_terms_file(&config.terms_path()));
        let mut terms: Vec<(String, Kind)> = vec![];
        for t in &raw_terms {
            let (v, k) = parse_term(t, &orgs);
            if k == Kind::Org && is_org_name(&v) {
                terms.push((strip_legal_suffix(&v).to_string(), Kind::Org));
            }
            terms.push((v, k));
        }

        // Seed persona primaries so the machine's own identity maps to the persona.
        let before = vault.generation;
        {
            let mut f = Faker::new(&persona, &mut vault, primary_org.clone());
            if let Some((full, stem)) = &primary_org {
                f.fake_for(Kind::Org, full);
                f.fake_for(Kind::Org, stem);
            }
        }
        if config.detectors.local_facts {
            let facts = LocalFacts::discover();
            let seeds = [
                (
                    facts.username.clone(),
                    Kind::Username,
                    persona.username.clone(),
                ),
                (
                    facts.hostname.clone(),
                    Kind::Hostname,
                    persona.hostname.clone(),
                ),
                (
                    facts.git_name.clone(),
                    Kind::Person,
                    persona.full_name.clone(),
                ),
                (facts.git_email.clone(), Kind::Email, persona.email.clone()),
            ];
            for (real, kind, fake) in seeds {
                if let Some(real) = real {
                    if vault.fake_for(&real).is_none() && !vault.is_taken(&fake) {
                        vault.insert(kind.as_str(), &real, &fake);
                    }
                    terms.push((real, kind));
                }
            }
            if let Some(d) = facts.local_domain {
                terms.push((d, Kind::Hostname));
            }
        }
        if vault.generation != before {
            vault.save()?;
        }

        let allow = config
            .allow_terms
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
        let state = State {
            vault,
            terms,
            terms_gen: 1,
            dict_base: Arc::new(DictionaryDetector::new(vec![])),
            dict_base_key: (u64::MAX, 0),
            dict_base_len: 0,
            dict_recent: Arc::new(DictionaryDetector::new(vec![])),
            dict_key: (0, 0),
            unmasker: Arc::new(Unmasker::new(vec![])),
            unmasker_gen: 0,
        };
        let shadow = Shadow {
            inner: Arc::new(Inner {
                config,
                persona,
                detectors,
                primary_org,
                allow,
                state: RwLock::new(state),
                last_refresh: Mutex::new(Instant::now()),
            }),
        };
        // Compile the rule detectors' patterns in the background rather than on the first request.
        let warm = shadow.clone();
        std::thread::spawn(move || {
            const SAMPLE: &str = "Dear Dr. Jane Doe, born 01/02/1980, SSN 078-05-1120, Name: Ana \
                Silva, Tel. +44 20 7946 0958, 12 Elm Street, Leeds. Acme GmbH, IBAN \
                DE89370400440532013000, ssh root@db-1.corp, password=hunter2x, fe80::1:2:3";
            for d in warm.inner.detectors.iter() {
                if !detect::public::ML_DETECTORS.contains(&d.name()) {
                    let fs = d.detect(SAMPLE);
                    for f in &fs {
                        detect::public::suppress(f, &SAMPLE[f.start..f.end]);
                    }
                }
            }
        });
        Ok(shadow)
    }

    /// The persistent persona.
    pub fn persona(&self) -> Persona {
        self.inner.persona.clone()
    }

    /// The configuration this instance was opened with.
    pub fn config(&self) -> &ShadowConfig {
        &self.inner.config
    }

    /// Merge vault entries written by other processes (rate-limited to 1/s).
    fn maybe_refresh(&self) {
        let mut last = self
            .inner
            .last_refresh
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if last.elapsed() < Duration::from_secs(1) {
            return;
        }
        *last = Instant::now();
        drop(last);
        let mut st = self.write();
        if let Err(e) = st.vault.refresh() {
            tracing::warn!("vault refresh failed: {e:#}");
        }
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, State> {
        self.inner.state.read().unwrap_or_else(|e| e.into_inner())
    }
    fn write(&self) -> std::sync::RwLockWriteGuard<'_, State> {
        self.inner.state.write().unwrap_or_else(|e| e.into_inner())
    }

    /// The dictionary detectors (base, recent). Rebuilding one automaton over the whole vault on
    /// every new value costs O(vault) per request, so new entries go to a small second automaton
    /// that is folded into the base once it grows past `RECENT_MAX`.
    fn dictionary(&self) -> (Arc<DictionaryDetector>, Arc<DictionaryDetector>) {
        const RECENT_MAX: usize = 512;
        {
            let st = self.read();
            if st.dict_key == (st.vault.generation, st.terms_gen) {
                return (st.dict_base.clone(), st.dict_recent.clone());
            }
        }
        let mut st = self.write();
        let key = (st.vault.generation, st.terms_gen);
        if st.dict_key != key {
            let entries = st.vault.entries();
            let base_key = (st.vault.epoch, st.terms_gen);
            if st.dict_base_key != base_key
                || entries.len() < st.dict_base_len
                || entries.len() - st.dict_base_len > RECENT_MAX
            {
                let mut all = st.terms.clone();
                all.extend(vault_terms(entries));
                let len = entries.len();
                st.dict_base = Arc::new(DictionaryDetector::new(all));
                st.dict_base_key = base_key;
                st.dict_base_len = len;
                st.dict_recent = Arc::new(DictionaryDetector::new(vec![]));
            } else {
                let recent = vault_terms(&entries[st.dict_base_len..]);
                st.dict_recent = Arc::new(DictionaryDetector::new(recent));
            }
            st.dict_key = key;
        }
        (st.dict_base.clone(), st.dict_recent.clone())
    }

    fn unmasker(&self) -> Arc<Unmasker> {
        self.maybe_refresh();
        {
            let st = self.read();
            if st.unmasker_gen == st.vault.generation {
                return st.unmasker.clone();
            }
        }
        let mut st = self.write();
        if st.unmasker_gen != st.vault.generation {
            let pairs = st
                .vault
                .entries()
                .iter()
                .map(|e| (e.fake.clone(), e.real.clone()))
                .collect();
            st.unmasker = Arc::new(Unmasker::new(pairs));
            st.unmasker_gen = st.vault.generation;
        }
        st.unmasker.clone()
    }

    /// Detect sensitive spans without changing anything (used by `shadow scan`).
    pub fn scan(&self, text: &str) -> Vec<Finding> {
        let (base, recent) = self.dictionary();
        let mut all = base.detect(text);
        all.extend(recent.detect(text));
        for d in &self.inner.detectors {
            all.extend(d.detect(text));
        }
        let mut all = refine_ml_findings(all, text);
        let st = self.read();
        let public_allowlist = self.inner.config.detectors.public_allowlist;
        all.retain(|f| {
            let v = &text[f.start..f.end];
            !self.inner.allow.contains(&v.to_lowercase())
                && v.trim().chars().count() >= 2
                && !(public_allowlist && detect::public::suppress(f, v))
        });
        drop(st);
        detect::resolve_overlaps(all, text.len())
    }

    /// Replace every sensitive value in `text` with its (persisted) fake. Secrets are redacted.
    /// For intent-aware masking of a whole request, use [`Shadow::session`].
    pub fn mask(&self, text: &str) -> Masked {
        self.session(RequestContext::default()).mask(text)
    }

    /// A masking session for one request: classifies its intent and applies the policy.
    pub fn session(&self, ctx: RequestContext) -> Session {
        let personal = intent::personal(&self.scan(&ctx.instruction));
        let class = intent::classify(&ctx, personal);
        Session {
            shadow: self.clone(),
            class,
            instruction: ctx.instruction,
            tokens: Mutex::new(Tokens::default()),
        }
    }

    /// A session for an OpenAI, Anthropic or Gemini request body.
    pub fn session_for(&self, body: &Value) -> Session {
        self.session(intent::context_from_json(body))
    }

    /// The masking loop: each finding is replaced by `decide`'s text, or by its persona fake.
    fn mask_with(
        &self,
        text: &str,
        decide: &mut dyn FnMut(&Finding, &str) -> Option<String>,
    ) -> Masked {
        self.maybe_refresh();
        let findings = self.scan(text);
        if findings.is_empty() {
            return Masked {
                text: text.to_string(),
                replacements: 0,
                kinds: vec![],
            };
        }
        let mut st = self.write();
        let before = st.vault.generation;
        let mut out = String::with_capacity(text.len() + 16);
        let mut kinds = BTreeSet::new();
        let mut last = 0;
        let mut replaced = 0;
        {
            let mut faker = Faker::new(
                &self.inner.persona,
                &mut st.vault,
                self.inner.primary_org.clone(),
            );
            for f in &findings {
                let real = &text[f.start..f.end];
                out.push_str(&text[last..f.start]);
                match decide(f, real) {
                    Some(r) if r == real => out.push_str(real),
                    Some(r) => {
                        out.push_str(&r);
                        replaced += 1;
                        kinds.insert(f.kind.as_str().to_string());
                    }
                    None => {
                        out.push_str(&faker.fake_for(f.kind, real));
                        replaced += 1;
                        kinds.insert(f.kind.as_str().to_string());
                    }
                }
                last = f.end;
            }
        }
        out.push_str(&text[last..]);
        if st.vault.generation != before {
            if let Err(e) = st.vault.save() {
                tracing::error!("failed to persist vault: {e:#}");
            }
        }
        Masked {
            text: out,
            replacements: replaced,
            kinds: kinds.into_iter().collect(),
        }
    }

    /// Replace every known fake in `text` with its real value.
    pub fn unmask(&self, text: &str) -> String {
        self.unmasker().apply(text)
    }

    /// True if `text` contains any known fake.
    pub fn contains_fake(&self, text: &str) -> bool {
        self.unmasker().contains_fake(text)
    }

    /// Mask every string leaf of a JSON value in place (protocol keys such as `model`,
    /// `role` and `type` are skipped). Returns the number of replacements.
    pub fn mask_json(&self, v: &mut Value) -> usize {
        let mut n = 0;
        walk_strings_mut(v, None, &mut |s| {
            let m = self.mask(s);
            if m.replacements > 0 {
                n += m.replacements;
                *s = m.text;
            }
        });
        n
    }

    /// Unmask every string leaf of a JSON value in place.
    pub fn unmask_json(&self, v: &mut Value) {
        let un = self.unmasker();
        walk_all_strings_mut(v, &mut |s| {
            if un.contains_fake(s) {
                *s = un.apply(s);
            }
        });
    }

    /// Replace identifying request fields with persona values: OpenAI `user` /
    /// `safety_identifier`, Anthropic `metadata.user_id`. Returns fields replaced.
    pub fn scrub_identifiers(&self, v: &mut Value) -> usize {
        let id = Value::String(self.inner.persona.user_id.clone());
        let mut n = 0;
        if let Some(o) = v.as_object_mut() {
            for k in ["user", "safety_identifier"] {
                if let Some(x) = o.get_mut(k) {
                    if x.is_string() {
                        *x = id.clone();
                        n += 1;
                    }
                }
            }
            if let Some(Value::Object(meta)) = o.get_mut("metadata") {
                if let Some(x) = meta.get_mut("user_id") {
                    *x = id.clone();
                    n += 1;
                }
            }
        }
        n
    }

    /// A streaming unmasker bound to the current vault snapshot.
    pub fn stream_unmasker(&self) -> StreamUnmasker {
        StreamUnmasker::new(self.unmasker())
    }

    /// Vault statistics.
    pub fn stats(&self) -> VaultStats {
        self.read().vault.stats()
    }

    /// All vault entries (contains real values — handle with care).
    pub fn export(&self) -> Vec<VaultEntry> {
        self.read().vault.entries().to_vec()
    }

    /// Forget every mapping whose real or fake equals `value`. Returns entries removed.
    pub fn forget(&self, value: &str) -> anyhow::Result<usize> {
        let mut st = self.write();
        let n = st.vault.forget(value);
        if n > 0 {
            // Remove the old file first so the save does not merge the entry back in.
            std::fs::remove_file(self.inner.config.vault_path()).ok();
            st.vault.save()?;
        }
        Ok(n)
    }

    /// Protect new dictionary terms at runtime (persisted to `terms.json`).
    pub fn add_terms(&self, terms: &[String]) {
        let orgs: Vec<String> = self
            .inner
            .primary_org
            .iter()
            .map(|(f, _)| f.clone())
            .collect();
        let mut st = self.write();
        let mut changed = false;
        for t in terms.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
            let (v, k) = parse_term(t, &orgs);
            if !st.terms.iter().any(|(x, _)| x.eq_ignore_ascii_case(&v)) {
                if k == Kind::Org && is_org_name(&v) {
                    st.terms
                        .push((strip_legal_suffix(&v).to_string(), Kind::Org));
                }
                st.terms.push((v, k));
                changed = true;
            }
        }
        if changed {
            st.terms_gen += 1;
            let path = self.inner.config.terms_path();
            let mut saved = read_terms_file(&path);
            for t in terms {
                if !saved.iter().any(|s| s.eq_ignore_ascii_case(t.trim())) && !t.trim().is_empty() {
                    saved.push(t.trim().to_string());
                }
            }
            if let Ok(json) = serde_json::to_vec_pretty(&saved) {
                if let Err(e) = write_atomic(&path, &json, true) {
                    tracing::warn!("could not persist terms: {e:#}");
                }
            }
        }
    }

    /// Currently protected dictionary terms with their kinds.
    pub fn terms(&self) -> Vec<(String, Kind)> {
        self.read().terms.clone()
    }
}

/// One request being masked: its intent, the policy decisions, and the typed tokens it handed
/// out (kept only for this request, so the answer can be translated back).
pub struct Session {
    shadow: Shadow,
    class: Classification,
    instruction: String,
    tokens: Mutex<Tokens>,
}

#[derive(Default)]
struct Tokens {
    by_real: std::collections::HashMap<String, String>,
    pairs: Vec<(String, String)>,
    counters: std::collections::HashMap<&'static str, usize>,
    summary: SessionSummary,
    /// Unmasker over vault + tokens, valid while neither changes: (vault generation, tokens).
    cached: Option<(u64, usize, Arc<Unmasker>)>,
}

/// What a session did, for logs: counts only, never values.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct SessionSummary {
    pub surrogates: usize,
    pub tokens: usize,
    pub kept: usize,
    pub redacted: usize,
}

impl Session {
    pub fn classification(&self) -> &Classification {
        &self.class
    }

    pub fn summary(&self) -> SessionSummary {
        self.lock().summary.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Tokens> {
        self.tokens.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Mask `text` under this request's policy.
    pub fn mask(&self, text: &str) -> Masked {
        let cfg = &self.shadow.inner.config.policy;
        let instruction = self.instruction.as_str();
        let class = &self.class;
        let mut toks = self.lock();
        self.shadow.mask_with(text, &mut |f, real| {
            let target = intent::is_action_target(
                class,
                !instruction.is_empty() && instruction.contains(real),
                f.kind,
            );
            match intent::treatment(cfg, class, f.kind, target) {
                Treatment::Surrogate => {
                    toks.summary.surrogates += 1;
                    None
                }
                Treatment::Keep => {
                    toks.summary.kept += 1;
                    tracing::info!(
                        kind = f.kind.as_str(),
                        intent = class.intent.as_str(),
                        "kept an action target unmasked: the provider runs this tool"
                    );
                    Some(real.to_string())
                }
                Treatment::Redact => {
                    toks.summary.redacted += 1;
                    Some(format!("[REDACTED_{}]", f.kind.as_str().to_uppercase()))
                }
                Treatment::Token => {
                    toks.summary.tokens += 1;
                    if let Some(t) = toks.by_real.get(real) {
                        return Some(t.clone());
                    }
                    let label = f.kind.as_str().to_uppercase();
                    let n = toks.counters.entry(f.kind.as_str()).or_default();
                    *n += 1;
                    let token = format!("[{label}_{n:04}]");
                    toks.by_real.insert(real.to_string(), token.clone());
                    toks.pairs.push((token.clone(), real.to_string()));
                    Some(token)
                }
            }
        })
    }

    /// Mask every eligible string leaf of a JSON request body in place.
    pub fn mask_json(&self, v: &mut Value) -> usize {
        let mut n = 0;
        walk_strings_mut(v, None, &mut |s| {
            let m = self.mask(s);
            if m.replacements > 0 {
                n += m.replacements;
                *s = m.text;
            }
        });
        n
    }

    fn unmasker(&self) -> Arc<Unmasker> {
        let base = self.shadow.unmasker();
        let mut toks = self.lock();
        if toks.pairs.is_empty() {
            return base;
        }
        let st = self.shadow.read();
        let key = (st.vault.generation, toks.pairs.len());
        if let Some((g, n, u)) = &toks.cached {
            if (*g, *n) == key {
                return u.clone();
            }
        }
        let mut pairs: Vec<(String, String)> = st
            .vault
            .entries()
            .iter()
            .map(|e| (e.fake.clone(), e.real.clone()))
            .collect();
        drop(st);
        pairs.extend(toks.pairs.iter().cloned());
        let u = Arc::new(Unmasker::new(pairs));
        toks.cached = Some((key.0, key.1, u.clone()));
        u
    }

    /// Translate fakes and this request's tokens back to real values.
    pub fn unmask(&self, text: &str) -> String {
        self.unmasker().apply(text)
    }

    pub fn unmask_json(&self, v: &mut Value) {
        let un = self.unmasker();
        walk_all_strings_mut(v, &mut |s| {
            if un.contains_fake(s) {
                *s = un.apply(s);
            }
        });
    }

    /// A streaming unmasker for this request's response.
    pub fn stream_unmasker(&self) -> StreamUnmasker {
        StreamUnmasker::new(self.unmasker())
    }
}

fn read_terms_file(path: &Path) -> Vec<String> {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn load_or_create_persona(config: &ShadowConfig) -> anyhow::Result<Persona> {
    let path = config.persona_path();
    if path.exists() {
        let raw = std::fs::read(&path)?;
        return serde_json::from_slice(&raw).with_context(|| format!("parsing {}", path.display()));
    }
    let mut p = Persona::generate(config.persona.region.as_deref(), &mut rand::thread_rng());
    p.apply_overrides(&config.persona);
    write_atomic(&path, &serde_json::to_vec_pretty(&p)?, true)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_is_send_sync_clone() {
        fn assert_traits<T: Send + Sync + Clone>() {}
        assert_traits::<Shadow>();
    }
}
