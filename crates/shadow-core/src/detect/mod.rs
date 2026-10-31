//! Pluggable detection of sensitive spans.
//!
//! A [`Detector`] returns [`Finding`]s (byte ranges + [`Kind`]). Shadow ships regex,
//! checksum and dictionary detectors, plus an optional [`model::ModelDetector`] that
//! queries a local NER sidecar.

pub mod checksum;
pub mod dictionary;
pub mod model;
mod names;
pub mod patterns;
pub mod presidio;

use serde::{Deserialize, Serialize};

/// Category of a sensitive value. Determines how the fake is generated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Email,
    Phone,
    Person,
    Org,
    Term,
    Address,
    Iban,
    CreditCard,
    NationalId,
    Ip,
    Mac,
    Hostname,
    Secret,
    Username,
}

impl Kind {
    /// Stable lowercase name (used in stats and `Masked::kinds`).
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Email => "email",
            Kind::Phone => "phone",
            Kind::Person => "person",
            Kind::Org => "org",
            Kind::Term => "term",
            Kind::Address => "address",
            Kind::Iban => "iban",
            Kind::CreditCard => "credit_card",
            Kind::NationalId => "national_id",
            Kind::Ip => "ip",
            Kind::Mac => "mac",
            Kind::Hostname => "hostname",
            Kind::Secret => "secret",
            Kind::Username => "username",
        }
    }

    /// Parse a name produced by [`Kind::as_str`].
    pub fn parse(s: &str) -> Option<Kind> {
        ALL_KINDS.iter().copied().find(|k| k.as_str() == s)
    }

    /// Tie-break priority when two findings have the same length (higher wins).
    pub fn priority(self) -> u8 {
        match self {
            Kind::Secret => 9,
            Kind::Org | Kind::Term | Kind::Username => 8,
            Kind::Email => 7,
            Kind::Iban | Kind::CreditCard | Kind::NationalId => 6,
            Kind::Person => 5,
            _ => 3,
        }
    }
}

/// All kinds.
pub const ALL_KINDS: &[Kind] = &[
    Kind::Email,
    Kind::Phone,
    Kind::Person,
    Kind::Org,
    Kind::Term,
    Kind::Address,
    Kind::Iban,
    Kind::CreditCard,
    Kind::NationalId,
    Kind::Ip,
    Kind::Mac,
    Kind::Hostname,
    Kind::Secret,
    Kind::Username,
];

/// A detected sensitive span (byte offsets into the scanned text, end exclusive).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Finding {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
    /// Confidence in `0.0..=1.0`.
    pub score: f32,
    /// Name of the detector that produced it.
    pub detector: &'static str,
}

impl Finding {
    /// Convenience constructor with score 1.0.
    pub fn new(start: usize, end: usize, kind: Kind, detector: &'static str) -> Self {
        Self {
            start,
            end,
            kind,
            score: 1.0,
            detector,
        }
    }
    /// Length in bytes.
    pub fn len(&self) -> usize {
        self.end - self.start
    }
    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

/// A source of findings. Implement this to plug in custom detection.
pub trait Detector: Send + Sync {
    /// Short identifier.
    fn name(&self) -> &'static str;
    /// Find sensitive spans in `text`.
    fn detect(&self, text: &str) -> Vec<Finding>;
}

/// Choose a non-overlapping subset: longest first, then by kind priority, then leftmost.
pub fn resolve_overlaps(mut findings: Vec<Finding>, text_len: usize) -> Vec<Finding> {
    findings.retain(|f| !f.is_empty() && f.end <= text_len);
    findings.sort_by(|a, b| {
        b.len()
            .cmp(&a.len())
            .then(b.kind.priority().cmp(&a.kind.priority()))
            .then((a.detector == "dictionary").cmp(&(b.detector == "dictionary")))
            .then(a.start.cmp(&b.start))
    });
    let mut taken = vec![false; text_len];
    let mut out = Vec::new();
    for f in findings {
        if taken[f.start..f.end].iter().any(|t| *t) {
            continue;
        }
        taken[f.start..f.end].iter_mut().for_each(|t| *t = true);
        out.push(f);
    }
    out.sort_by_key(|f| f.start);
    out
}

pub(crate) fn names_webmail(domain: &str) -> bool {
    names::WEBMAIL.contains(&domain)
}
pub(crate) fn names_generic_label(label: &str) -> bool {
    names::GENERIC_LABELS.contains(&label)
}
pub(crate) fn names_internal_tld(tld: &str) -> bool {
    names::INTERNAL_TLDS.contains(&tld)
}

/// True if `b` is a word character (used for boundary checks).
pub(crate) fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// True if the span `[start,end)` is not glued to word characters on either side.
pub(crate) fn at_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start]
        .chars()
        .next_back()
        .is_none_or(|c| !is_word_char(c));
    let after = text[end..].chars().next().is_none_or(|c| !is_word_char(c));
    before && after
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_prefers_longest() {
        let f = vec![
            Finding::new(0, 4, Kind::Term, "t"),
            Finding::new(0, 12, Kind::Email, "e"),
            Finding::new(13, 15, Kind::Ip, "i"),
        ];
        let r = resolve_overlaps(f, 20);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].kind, Kind::Email);
    }
}
