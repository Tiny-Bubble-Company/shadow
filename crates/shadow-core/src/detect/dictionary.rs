//! Dictionary detector: organisation names, codenames, local machine facts and every
//! real value already in the vault (so once-seen values stay protected).

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

use super::{at_word_boundary, Detector, Finding, Kind};

const LEGAL_SUFFIXES: &[&str] = &[
    "GmbH & Co. KG",
    "Co., Ltd.",
    "Pvt. Ltd.",
    "Pty Ltd",
    "S.A. de C.V.",
    "GmbH",
    "AG",
    "KG",
    "SE",
    "Inc.",
    "Inc",
    "LLC",
    "L.L.C.",
    "Ltd.",
    "Ltd",
    "Limited",
    "Corp.",
    "Corp",
    "Corporation",
    "Co.",
    "PLC",
    "plc",
    "S.A.",
    "SA",
    "SAS",
    "S.p.A.",
    "B.V.",
    "BV",
    "N.V.",
    "NV",
    "AB",
    "Oy",
    "ApS",
    "K.K.",
    "LLP",
];

/// Remove a trailing legal-form suffix (`Acme GmbH` -> `Acme`).
pub fn strip_legal_suffix(name: &str) -> &str {
    let t = name.trim();
    for suf in LEGAL_SUFFIXES {
        if let Some(rest) = t.strip_suffix(suf) {
            if rest.ends_with([' ', ',']) {
                let stem = rest.trim_end_matches([' ', ',']);
                if !stem.is_empty() {
                    return stem;
                }
            }
        }
    }
    t
}

/// True if the term carries a legal-form suffix.
pub fn is_org_name(name: &str) -> bool {
    strip_legal_suffix(name).len() != name.trim().len()
}

/// Case-insensitive, word-boundary dictionary matcher.
pub struct DictionaryDetector {
    ac: Option<AhoCorasick>,
    kinds: Vec<Kind>,
}

impl DictionaryDetector {
    /// Build from `(term, kind)` pairs. Terms shorter than 3 chars are ignored.
    pub fn new(terms: Vec<(String, Kind)>) -> Self {
        let mut seen = std::collections::HashSet::new();
        let (mut pats, mut kinds) = (vec![], vec![]);
        for (t, k) in terms {
            let t = t.trim().to_string();
            if t.chars().count() < 3 || !seen.insert(t.to_lowercase()) {
                continue;
            }
            pats.push(t);
            kinds.push(k);
        }
        let ac = if pats.is_empty() {
            None
        } else {
            AhoCorasickBuilder::new()
                .ascii_case_insensitive(true)
                .match_kind(MatchKind::LeftmostLongest)
                .build(&pats)
                .ok()
        };
        Self { ac, kinds }
    }

    /// Number of patterns.
    pub fn len(&self) -> usize {
        self.kinds.len()
    }
    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}

impl Detector for DictionaryDetector {
    fn name(&self) -> &'static str {
        "dictionary"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let Some(ac) = &self.ac else { return vec![] };
        ac.find_iter(text)
            .filter(|m| at_word_boundary(text, m.start(), m.end()))
            .map(|m| {
                Finding::new(
                    m.start(),
                    m.end(),
                    self.kinds[m.pattern().as_usize()],
                    "dictionary",
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffixes() {
        assert_eq!(strip_legal_suffix("Acme GmbH"), "Acme");
        assert_eq!(strip_legal_suffix("Globex Corp."), "Globex");
        assert_eq!(strip_legal_suffix("Initech"), "Initech");
        assert!(is_org_name("Hooli, Inc."));
    }

    #[test]
    fn matches_case_insensitive_on_boundaries() {
        let d = DictionaryDetector::new(vec![
            ("Acme".into(), Kind::Org),
            ("Project Falcon".into(), Kind::Term),
        ]);
        let t = "ACME ships project falcon; acmeology is fine";
        let f: Vec<&str> = d.detect(t).iter().map(|f| &t[f.start..f.end]).collect();
        assert_eq!(f, vec!["ACME", "project falcon"]);
    }
}
