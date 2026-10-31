//! Built-in regex + checksum detectors.

use std::sync::LazyLock;

use regex::Regex;

use super::checksum::{
    abn_valid, btc_base58_valid, btc_bech32_valid, iban_valid, luhn_valid, nhs_valid, tfn_valid,
};
use super::names::{FIRST_NAMES, INTERNAL_TLDS, PUBLIC_DOMAINS, TLDS};
use super::{Detector, Finding, Kind};
use crate::config::ShadowConfig;

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($pat).expect("valid regex"));
    };
}

re!(
    EMAIL,
    r"(?i)\b[a-z0-9][a-z0-9._%+\-]*@[a-z0-9](?:[a-z0-9\-]*[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9\-]*[a-z0-9])?)*\.[a-z]{2,24}\b"
);
re!(
    PHONE_INTL,
    r"\+\d{1,3}[\s.\-]?(?:\(\d{1,4}\)[\s.\-]?)?\d{1,5}(?:[\s.\-]?\d{2,5}){1,4}\b"
);
re!(
    PHONE_US,
    r"\(\d{3}\)\s?\d{3}[\s.\-]\d{4}\b|\b\d{3}[.\-]\d{3}[.\-]\d{4}\b"
);
re!(CARD, r"\b(?:\d[ \-]?){12,18}\d\b");
re!(
    IBAN,
    r"\b[A-Z]{2}\d{2}(?: ?[A-Z0-9]{4}){2,7}(?: ?[A-Z0-9]{1,4})?\b"
);
re!(SSN, r"\b(\d{3})-(\d{2})-(\d{4})\b");
re!(
    NINO,
    r"\b[A-CEGHJ-PR-TW-Z][A-CEGHJ-NPR-TW-Z] ?\d{2} ?\d{2} ?\d{2} ?[A-D]\b"
);
re!(AADHAAR, r"\b[2-9]\d{3} \d{4} \d{4}\b");
// US ITIN: 9xx, then a group in 50-65, 70-88, 90-92 or 94-99, then four digits.
re!(
    ITIN,
    r"\b9\d{2}([- ]?)(?:5\d|6[0-5]|7\d|8[0-8]|9[0-2]|9[4-9])([- ]?)\d{4}\b"
);
// India PAN: 3 letters, holder type (A,B,C,F,G,H,J,L,P,T), surname/name initial, 4 digits, letter.
re!(PAN, r"\b[A-Z]{3}[ABCFGHJLPT][A-Z]\d{4}[A-Z]\b");
re!(NHS, r"\b\d{3}([ \-]?)\d{3}([ \-]?)\d{4}\b");
re!(TFN, r"\b\d{3}([ \-]?)\d{3}([ \-]?)\d{3}\b");
re!(ABN, r"\b\d{2}( ?)\d{3}( ?)\d{3}( ?)\d{3}\b");
re!(BTC_BASE58, r"\b[13][1-9A-HJ-NP-Za-km-z]{25,34}\b");
re!(BTC_BECH32, r"(?i)\bbc1[ac-hj-np-z02-9]{11,87}\b");
re!(ETH, r"\b0x[0-9a-fA-F]{40}\b");
re!(
    IPV4,
    r"\b(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\b"
);
re!(IPV6, r"\b(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}\b");
re!(
    MAC,
    r"\b(?:[0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}\b|\b(?:[0-9A-Fa-f]{2}-){5}[0-9A-Fa-f]{2}\b"
);
re!(
    URL_HOST,
    r"(?i)\b[a-z][a-z0-9+.\-]*://(?:[^\s/@:]+(?::[^\s/@]*)?@)?([a-z0-9](?:[a-z0-9\-]*[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9\-]*[a-z0-9])?)+)"
);
re!(
    BARE_HOST,
    r"(?i)\b(?:[a-z0-9](?:[a-z0-9\-]{0,61}[a-z0-9])?\.)+([a-z]{2,12})\b"
);
re!(
    PATH_USER,
    r#"(?:/Users/|/home/|[A-Za-z]:\\Users\\|[A-Za-z]:/Users/)([^/\\\s"'<>:|*?]+)"#
);
re!(
    HONORIFIC,
    r"\b(?:Mr|Mrs|Ms|Mx|Dr|Prof)\.?\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)\b"
);
re!(
    ADDRESS_EN,
    r"\b\d{1,5}[A-Za-z]?\s+(?:[A-Z][a-z]+\s+){1,3}(?:Street|St\.?|Avenue|Ave\.?|Road|Rd\.?|Boulevard|Blvd\.?|Lane|Ln\.?|Drive|Way|Court|Ct\.?|Place|Pl\.?|Square|Terrace|Parkway)"
);
re!(
    ADDRESS_DE,
    r"\b[A-Z][a-zäöüß\-]+(?:strasse|straße|str\.|weg|allee|platz|gasse|ring|damm)\s+\d{1,4}[a-z]?\b"
);
re!(
    PRIVATE_KEY,
    r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----"
);
re!(
    SECRET_KNOWN,
    r"\b(?:sk-ant-[A-Za-z0-9_\-]{20,}|sk-or-v1-[A-Za-z0-9]{20,}|sk-(?:proj-)?[A-Za-z0-9_\-]{20,}|gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}|xox[abposr]-[A-Za-z0-9\-]{10,}|(?:AKIA|ASIA)[0-9A-Z]{16}|AIza[0-9A-Za-z_\-]{35}|glpat-[A-Za-z0-9_\-]{20,}|gsk_[A-Za-z0-9]{30,}|xai-[A-Za-z0-9]{30,}|hf_[A-Za-z0-9]{30,}|npm_[A-Za-z0-9]{30,}|(?:sk|rk)_live_[A-Za-z0-9]{20,}|eyJ[A-Za-z0-9_\-]{8,}\.eyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,})"
);
re!(
    SECRET_ASSIGN,
    r#"(?i)\b(?:api[_\-]?key|secret|token|passwd|password|pwd|access[_\-]?key|client[_\-]?secret|private[_\-]?key)["']?\s*[:=]\s*["']?([A-Za-z0-9_\-./+=!@#$%^&*]{8,})"#
);
re!(BEARER, r"(?i)\bbearer\s+([A-Za-z0-9\-._~+/]{20,}=*)");

static PERSON: LazyLock<Regex> = LazyLock::new(|| {
    let names = FIRST_NAMES.join("|");
    Regex::new(&format!(
        r"\b(?:{names})\s+[A-Z][a-z]+(?:[\-'][A-Z][a-z]+)?\b"
    ))
    .expect("valid regex")
});

const NOT_SURNAMES: &[&str] = &[
    "The", "And", "Is", "Was", "From", "To", "In", "Of", "For", "With", "On", "At", "By", "Or",
    "But", "If", "Then", "This", "That", "Has", "Had", "Will", "Can", "Said", "Says", "Wrote",
    "Asked", "Here", "There", "Please", "Thanks", "Thank", "Hi", "Hello", "Dear", "Team", "Street",
    "Road", "Avenue",
];

fn spans(re: &Regex, text: &str, kind: Kind, det: &'static str) -> Vec<Finding> {
    re.find_iter(text)
        .map(|m| Finding::new(m.start(), m.end(), kind, det))
        .collect()
}

fn group_spans(re: &Regex, text: &str, kind: Kind, det: &'static str) -> Vec<Finding> {
    re.captures_iter(text)
        .filter_map(|c| c.get(1))
        .map(|m| Finding::new(m.start(), m.end(), kind, det))
        .collect()
}

/// Email addresses.
pub struct EmailDetector;
impl Detector for EmailDetector {
    fn name(&self) -> &'static str {
        "email"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        spans(&EMAIL, text, Kind::Email, "email")
    }
}

/// International (`+cc ...`) and North-American phone numbers.
pub struct PhoneDetector;
impl Detector for PhoneDetector {
    fn name(&self) -> &'static str {
        "phone"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = spans(&PHONE_US, text, Kind::Phone, "phone");
        for m in PHONE_INTL.find_iter(text) {
            let digits = m.as_str().chars().filter(char::is_ascii_digit).count();
            let glued = text[..m.start()]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric());
            if (8..=15).contains(&digits) && !glued {
                out.push(Finding::new(m.start(), m.end(), Kind::Phone, "phone"));
            }
        }
        out
    }
}

/// Payment cards (Luhn-validated, known network prefixes).
pub struct CardDetector;
impl Detector for CardDetector {
    fn name(&self) -> &'static str {
        "credit_card"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        CARD.find_iter(text)
            .filter(|m| {
                let d: String = m.as_str().chars().filter(char::is_ascii_digit).collect();
                let network = matches!(d.as_bytes()[0], b'3' | b'4' | b'5' | b'6')
                    || (d.starts_with('2')
                        && (2221..=2720).contains(&d[..4].parse::<u32>().unwrap_or(0)));
                (13..=19).contains(&d.len()) && network && luhn_valid(&d)
            })
            .map(|m| Finding::new(m.start(), m.end(), Kind::CreditCard, "credit_card"))
            .collect()
    }
}

/// IBANs (mod-97 validated). Over-long greedy matches are trimmed group by group.
pub struct IbanDetector;
impl Detector for IbanDetector {
    fn name(&self) -> &'static str {
        "iban"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = vec![];
        for m in IBAN.find_iter(text) {
            let s = m.as_str();
            let mut end = s.len();
            loop {
                if iban_valid(&s[..end]) {
                    out.push(Finding::new(m.start(), m.start() + end, Kind::Iban, "iban"));
                    break;
                }
                match s[..end].trim_end().rfind(' ') {
                    Some(i) if i > 8 => end = i,
                    _ => break,
                }
            }
        }
        out
    }
}

/// National identifiers: US SSN/ITIN, UK NINO/NHS, Indian Aadhaar/PAN, Australian TFN/ABN.
pub struct NationalIdDetector;
impl Detector for NationalIdDetector {
    fn name(&self) -> &'static str {
        "national_id"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out: Vec<Finding> = SSN
            .captures_iter(text)
            .filter(|c| {
                let a = &c[1];
                a != "000" && a != "666" && !a.starts_with('9') && &c[2] != "00" && &c[3] != "0000"
            })
            .map(|c| {
                let m = c.get(0).unwrap();
                Finding::new(m.start(), m.end(), Kind::NationalId, "national_id")
            })
            .collect();
        out.extend(spans(&NINO, text, Kind::NationalId, "national_id"));
        out.extend(spans(&AADHAAR, text, Kind::NationalId, "national_id"));
        out.extend(
            ITIN.captures_iter(text)
                .filter(|c| c[1] == c[2])
                .map(|c| c.get(0).unwrap())
                .map(|m| Finding::new(m.start(), m.end(), Kind::NationalId, "national_id")),
        );
        out.extend(spans(&PAN, text, Kind::NationalId, "national_id"));
        // Bare digit runs pass a mod-11/mod-89 check by chance fairly often, so the
        // unformatted forms need a nearby keyword; the official spaced forms do not.
        let checked =
            |re: &Regex, valid: fn(&str) -> bool, keywords: &[&str], out: &mut Vec<Finding>| {
                for c in re.captures_iter(text) {
                    let m = c.get(0).unwrap();
                    let seps: Vec<&str> = c.iter().skip(1).flatten().map(|g| g.as_str()).collect();
                    let spaced = seps.iter().all(|s| *s == " ");
                    let consistent = seps.windows(2).all(|w| w[0] == w[1]);
                    if consistent
                        && valid(m.as_str())
                        && (spaced || has_context(text, m.start(), keywords))
                    {
                        out.push(Finding::new(
                            m.start(),
                            m.end(),
                            Kind::NationalId,
                            "national_id",
                        ));
                    }
                }
            };
        checked(&NHS, nhs_valid, &["nhs"], &mut out);
        checked(&TFN, tfn_valid, &["tfn", "tax file"], &mut out);
        checked(&ABN, abn_valid, &["abn", "business number"], &mut out);
        out
    }
}

/// True if one of `keywords` (lowercase) appears in the ~48 bytes before `start`.
fn has_context(text: &str, start: usize, keywords: &[&str]) -> bool {
    let mut from = start.saturating_sub(48);
    while !text.is_char_boundary(from) {
        from += 1;
    }
    let window = text[from..start].to_lowercase();
    keywords.iter().any(|k| window.contains(k))
}

/// Cryptocurrency wallet addresses: Bitcoin (base58check legacy/P2SH, bech32/bech32m
/// segwit, all checksum-validated) and Ethereum (`0x` + 40 hex).
pub struct CryptoWalletDetector;
impl Detector for CryptoWalletDetector {
    fn name(&self) -> &'static str {
        "crypto"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out: Vec<Finding> = BTC_BASE58
            .find_iter(text)
            .filter(|m| btc_base58_valid(m.as_str()))
            .map(|m| Finding::new(m.start(), m.end(), Kind::Secret, "crypto"))
            .collect();
        out.extend(
            BTC_BECH32
                .find_iter(text)
                .filter(|m| btc_bech32_valid(m.as_str()))
                .map(|m| Finding::new(m.start(), m.end(), Kind::Secret, "crypto")),
        );
        out.extend(spans(&ETH, text, Kind::Secret, "crypto"));
        out
    }
}

/// IPv4 / IPv6 addresses (loopback and unspecified addresses are ignored).
pub struct IpDetector;
impl Detector for IpDetector {
    fn name(&self) -> &'static str {
        "ip"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out: Vec<Finding> = IPV4
            .find_iter(text)
            .filter(|m| {
                let s = m.as_str();
                let prev = text[..m.start()].chars().next_back();
                let next = text[m.end()..].chars().next();
                !s.starts_with("127.")
                    && s != "0.0.0.0"
                    && s != "255.255.255.255"
                    && !matches!(prev, Some('v' | 'V' | '.'))
                    && !(next == Some('.')
                        && text[m.end() + 1..].starts_with(|c: char| c.is_ascii_digit()))
            })
            .map(|m| Finding::new(m.start(), m.end(), Kind::Ip, "ip"))
            .collect();
        out.extend(spans(&IPV6, text, Kind::Ip, "ip"));
        out
    }
}

/// MAC addresses.
pub struct MacDetector;
impl Detector for MacDetector {
    fn name(&self) -> &'static str {
        "mac"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        spans(&MAC, text, Kind::Mac, "mac")
    }
}

/// Hostnames in URLs and bare domains that are not well-known public services.
pub struct HostnameDetector {
    allow: Vec<String>,
    internal: Vec<String>,
}

impl HostnameDetector {
    /// Build from config allow/internal lists.
    pub fn new(cfg: &ShadowConfig) -> Self {
        let mut allow: Vec<String> = PUBLIC_DOMAINS.iter().map(|s| s.to_string()).collect();
        allow.extend(cfg.allow_domains.iter().map(|s| s.to_ascii_lowercase()));
        Self {
            allow,
            internal: cfg
                .internal_domains
                .iter()
                .map(|s| s.to_ascii_lowercase())
                .collect(),
        }
    }

    fn under(host: &str, list: &[String]) -> bool {
        list.iter()
            .any(|d| host == d || host.ends_with(&format!(".{d}")))
    }

    /// Whether a hostname should be protected.
    pub fn sensitive(&self, host: &str) -> bool {
        let h = host.to_ascii_lowercase();
        if h == "localhost" || h.parse::<std::net::IpAddr>().is_ok() {
            return false;
        }
        if Self::under(&h, &self.internal) {
            return true;
        }
        let tld = h.rsplit('.').next().unwrap_or("");
        if INTERNAL_TLDS.contains(&tld) {
            return true;
        }
        !Self::under(&h, &self.allow)
    }
}

impl Detector for HostnameDetector {
    fn name(&self) -> &'static str {
        "hostname"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = vec![];
        for c in URL_HOST.captures_iter(text) {
            let m = c.get(1).unwrap();
            if self.sensitive(m.as_str()) {
                out.push(Finding::new(m.start(), m.end(), Kind::Hostname, "hostname"));
            }
        }
        for c in BARE_HOST.captures_iter(text) {
            let m = c.get(0).unwrap();
            let tld = c.get(1).unwrap().as_str().to_ascii_lowercase();
            let prev = text[..m.start()].chars().next_back();
            let next = text[m.end()..].chars().next();
            if !TLDS.contains(&tld.as_str())
                || matches!(prev, Some('@' | '.' | '-' | '_' | '\\'))
                || matches!(next, Some('(' | '@'))
                || out.iter().any(|f| f.start < m.end() && m.start() < f.end)
            {
                continue;
            }
            let host = m.as_str();
            if host
                .split('.')
                .next()
                .is_some_and(|l| l.chars().all(|c| c.is_ascii_digit()))
            {
                continue;
            }
            if self.sensitive(host) {
                out.push(Finding::new(m.start(), m.end(), Kind::Hostname, "hostname"));
            }
        }
        out
    }
}

/// API keys, tokens, private keys, bearer tokens and `password=...` style assignments.
pub struct SecretDetector;

const PLACEHOLDERS: &[&str] = &[
    "your",
    "xxxx",
    "example",
    "changeme",
    "placeholder",
    "dummy",
    "redacted",
    "process.env",
    "os.environ",
    "<",
    "${",
    "{{",
];

impl Detector for SecretDetector {
    fn name(&self) -> &'static str {
        "secret"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = spans(&SECRET_KNOWN, text, Kind::Secret, "secret");
        out.extend(spans(&PRIVATE_KEY, text, Kind::Secret, "secret"));
        out.extend(group_spans(&BEARER, text, Kind::Secret, "secret"));
        for c in SECRET_ASSIGN.captures_iter(text) {
            let m = c.get(1).unwrap();
            let v = m.as_str().trim_end_matches(['"', '\'', ',', ';', '.']);
            let lower = v.to_ascii_lowercase();
            let classes = [
                v.chars().any(|c| c.is_ascii_lowercase()),
                v.chars().any(|c| c.is_ascii_uppercase()),
                v.chars().any(|c| c.is_ascii_digit()),
                v.chars().any(|c| !c.is_ascii_alphanumeric()),
            ]
            .iter()
            .filter(|b| **b)
            .count();
            if PLACEHOLDERS.iter().any(|p| lower.contains(p)) || classes < 2 || v.len() < 8 {
                continue;
            }
            out.push(Finding::new(
                m.start(),
                m.start() + v.len(),
                Kind::Secret,
                "secret",
            ));
        }
        out
    }
}

/// Usernames embedded in home-directory paths (`/Users/<u>`, `/home/<u>`, `C:\Users\<u>`).
pub struct PathDetector;
impl Detector for PathDetector {
    fn name(&self) -> &'static str {
        "path"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        PATH_USER
            .captures_iter(text)
            .filter_map(|c| c.get(1))
            .filter(|m| {
                !matches!(
                    m.as_str(),
                    "Shared"
                        | "Public"
                        | "Default"
                        | "All Users"
                        | "runner"
                        | "user"
                        | "username"
                        | "you"
                        | "me"
                )
            })
            .map(|m| Finding::new(m.start(), m.end(), Kind::Username, "path"))
            .collect()
    }
}

/// Person names: "Firstname Lastname" with a known given name, or honorific + name.
pub struct PersonDetector;
impl Detector for PersonDetector {
    fn name(&self) -> &'static str {
        "person"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out: Vec<Finding> = PERSON
            .find_iter(text)
            .filter(|m| {
                let last = m.as_str().split_whitespace().last().unwrap_or("");
                !NOT_SURNAMES.contains(&last)
            })
            .map(|m| Finding {
                score: 0.8,
                ..Finding::new(m.start(), m.end(), Kind::Person, "person")
            })
            .collect();
        out.extend(group_spans(&HONORIFIC, text, Kind::Person, "person"));
        out
    }
}

/// Street addresses (English and German style).
pub struct AddressDetector;
impl Detector for AddressDetector {
    fn name(&self) -> &'static str {
        "address"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = spans(&ADDRESS_EN, text, Kind::Address, "address");
        out.extend(spans(&ADDRESS_DE, text, Kind::Address, "address"));
        out
    }
}

/// The built-in detectors enabled in `cfg`.
pub fn builtin(cfg: &ShadowConfig) -> Vec<Box<dyn Detector>> {
    let t = &cfg.detectors;
    let mut v: Vec<Box<dyn Detector>> = vec![];
    if t.email {
        v.push(Box::new(EmailDetector));
    }
    if t.phone {
        v.push(Box::new(PhoneDetector));
    }
    if t.credit_card {
        v.push(Box::new(CardDetector));
    }
    if t.iban {
        v.push(Box::new(IbanDetector));
    }
    if t.national_id {
        v.push(Box::new(NationalIdDetector));
    }
    if t.ip {
        v.push(Box::new(IpDetector));
    }
    if t.mac {
        v.push(Box::new(MacDetector));
    }
    if t.hostname {
        v.push(Box::new(HostnameDetector::new(cfg)));
    }
    if t.secret {
        v.push(Box::new(SecretDetector));
        v.push(Box::new(CryptoWalletDetector));
    }
    if t.path {
        v.push(Box::new(PathDetector));
    }
    if t.person {
        v.push(Box::new(PersonDetector));
    }
    if t.address {
        v.push(Box::new(AddressDetector));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(d: &dyn Detector, t: &str) -> Vec<String> {
        d.detect(t)
            .iter()
            .map(|f| t[f.start..f.end].to_string())
            .collect()
    }

    #[test]
    fn detects_common_pii() {
        assert_eq!(
            kinds(&EmailDetector, "mail jane.doe@acme-corp.com now"),
            vec!["jane.doe@acme-corp.com"]
        );
        assert_eq!(
            kinds(&PhoneDetector, "call +49 30 1234 5678."),
            vec!["+49 30 1234 5678"]
        );
        assert_eq!(
            kinds(&PhoneDetector, "or (415) 555-0134"),
            vec!["(415) 555-0134"]
        );
        assert_eq!(
            kinds(
                &CardDetector,
                "card 4111 1111 1111 1111 ok, ts 1698765432123"
            ),
            vec!["4111 1111 1111 1111"]
        );
        assert_eq!(
            kinds(&IbanDetector, "IBAN DE89 3704 0044 0532 0130 00 please"),
            vec!["DE89 3704 0044 0532 0130 00"]
        );
        assert_eq!(
            kinds(&NationalIdDetector, "SSN 123-45-6789"),
            vec!["123-45-6789"]
        );
        assert_eq!(
            kinds(&IpDetector, "host 10.1.2.3 and 127.0.0.1 v1.2.3.4"),
            vec!["10.1.2.3"]
        );
        assert_eq!(
            kinds(&MacDetector, "mac 3c:22:fb:12:ab:9e"),
            vec!["3c:22:fb:12:ab:9e"]
        );
        assert_eq!(
            kinds(&PathDetector, "at /Users/jdoe/src and C:\\Users\\bob\\x"),
            vec!["jdoe", "bob"]
        );
        assert_eq!(
            kinds(&PersonDetector, "Ask John Smith and Dr. Miller"),
            vec!["John Smith", "Miller"]
        );
        assert_eq!(
            kinds(
                &SecretDetector,
                "key sk-proj-abcdefghijklmnopqrstuvwx1234 and password=\"Hunter2!xyz\""
            ),
            vec!["sk-proj-abcdefghijklmnopqrstuvwx1234", "Hunter2!xyz"]
        );
    }

    #[test]
    fn presidio_style_national_ids() {
        let d = NationalIdDetector;
        // UK NHS: spaced form needs no context; bare form needs "NHS" nearby.
        assert_eq!(kinds(&d, "patient 943 476 5919 seen"), vec!["943 476 5919"]);
        assert!(kinds(&d, "patient 943 476 5918 seen").is_empty());
        assert_eq!(kinds(&d, "NHS no: 9434765919"), vec!["9434765919"]);
        assert!(kinds(&d, "order 9434765919").is_empty());
        // US ITIN (SSN detector skips 9xx).
        assert_eq!(kinds(&d, "ITIN 912-70-1234"), vec!["912-70-1234"]);
        assert_eq!(kinds(&d, "ITIN 912 94 1234"), vec!["912 94 1234"]);
        assert!(kinds(&d, "id 912-93-1234").is_empty());
        assert!(kinds(&d, "id 912-45-1234").is_empty());
        assert!(kinds(&d, "id 912-70 1234").is_empty());
        // India PAN: 4th char must be a valid holder type.
        assert_eq!(kinds(&d, "PAN ABCPD1234E."), vec!["ABCPD1234E"]);
        assert!(kinds(&d, "code ABCXD1234E").is_empty());
        assert!(kinds(&d, "code abcpd1234e").is_empty());
        // Australian TFN / ABN.
        assert_eq!(kinds(&d, "TFN 123 456 782"), vec!["123 456 782"]);
        assert_eq!(kinds(&d, "my tfn is 123456782"), vec!["123456782"]);
        assert!(kinds(&d, "TFN 123 456 789").is_empty());
        assert!(kinds(&d, "ref 123456782").is_empty());
        assert_eq!(kinds(&d, "ABN 51 824 753 556"), vec!["51 824 753 556"]);
        assert_eq!(kinds(&d, "ABN: 51824753556"), vec!["51824753556"]);
        assert!(kinds(&d, "ABN 51 824 753 557").is_empty());
        assert!(kinds(&d, "id 51824753556").is_empty());
    }

    #[test]
    fn crypto_wallets() {
        let d = CryptoWalletDetector;
        assert_eq!(
            kinds(&d, "send to 1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa now"),
            vec!["1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"]
        );
        assert_eq!(
            kinds(&d, "p2sh 3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy"),
            vec!["3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy"]
        );
        assert!(kinds(&d, "bad 1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb").is_empty());
        assert_eq!(
            kinds(&d, "segwit bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq."),
            vec!["bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"]
        );
        assert!(kinds(&d, "bad bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdp").is_empty());
        assert_eq!(
            kinds(&d, "eth 0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe ok"),
            vec!["0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe"]
        );
        assert!(kinds(&d, "short 0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BA").is_empty());
        let b = builtin(&ShadowConfig::from_dir("/tmp"));
        assert!(b.iter().any(|d| d.name() == "crypto"));
    }

    #[test]
    fn hostnames_skip_public_and_code() {
        let d = HostnameDetector::new(&ShadowConfig::from_dir("/tmp"));
        let found = kinds(
            &d,
            "see https://git.acme.io/x and github.com/rust, main.rs, db.corp.local, os.path",
        );
        assert_eq!(found, vec!["git.acme.io", "db.corp.local"]);
    }
}
