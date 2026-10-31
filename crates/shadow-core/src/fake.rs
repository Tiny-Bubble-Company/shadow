//! Format-preserving fake generation, bound to a persona and made unique via the vault.

use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::detect::checksum::{iban_check_digits, luhn_check_digit};
use crate::detect::dictionary::{is_org_name, strip_legal_suffix};
use crate::detect::fields::{month_lookup, parse_date, MONTHS};
use crate::detect::Kind;
use crate::persona::{codeword, Persona};
use crate::vault::Vault;

const SECOND_LEVEL: &[&str] = &[
    "co.uk", "org.uk", "ac.uk", "com.au", "com.br", "co.jp", "com.cn", "co.za", "com.mx", "co.in",
    "co.nz", "com.sg", "co.kr",
];
const SECRET_PREFIXES: &[&str] = &[
    "sk-ant-api03-",
    "sk-ant-",
    "sk-proj-",
    "sk-or-v1-",
    "sk-",
    "github_pat_",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "xoxb-",
    "xoxp-",
    "xoxa-",
    "xoxs-",
    "AKIA",
    "ASIA",
    "AIza",
    "glpat-",
    "gsk_",
    "xai-",
    "hf_",
    "npm_",
    "sk_live_",
    "rk_live_",
    "eyJ",
];

/// Generates fakes and records them in the vault.
pub(crate) struct Faker<'a> {
    pub persona: &'a Persona,
    pub vault: &'a mut Vault,
    /// Primary organisation (full legal name, stem) mapped to the persona company.
    pub primary_org: Option<(String, String)>,
    /// Text being masked: a fake that already appears in it as a whole token would be
    /// unmasked there too, so such candidates are skipped.
    text: &'a str,
    rng: ThreadRng,
}

/// Whether `needle` occurs in `text` as a whole token (not glued to letters or digits), the
/// way the unmasker matches fakes.
fn occurs_as_token(text: &str, needle: &str) -> bool {
    let alnum = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    let (first, last) = (needle.chars().next(), needle.chars().next_back());
    text.match_indices(needle).any(|(i, m)| {
        let glued_before = alnum(first) && alnum(text[..i].chars().next_back());
        let glued_after = alnum(last) && alnum(text[i + m.len()..].chars().next());
        !glued_before && !glued_after
    })
}

/// Re-case `fake` like `real` (UPPER, lower, or as generated).
pub fn case_like(real: &str, fake: &str) -> String {
    let letters: Vec<char> = real.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() > 1 && letters.iter().all(|c| c.is_uppercase()) {
        fake.to_uppercase()
    } else if !letters.is_empty() && letters.iter().all(|c| c.is_lowercase()) {
        fake.to_lowercase()
    } else {
        fake.to_string()
    }
}

impl<'a> Faker<'a> {
    pub fn new(
        persona: &'a Persona,
        vault: &'a mut Vault,
        primary_org: Option<(String, String)>,
    ) -> Self {
        Self {
            persona,
            vault,
            primary_org,
            text: "",
            rng: rand::thread_rng(),
        }
    }

    /// Avoid fakes that already appear in `text`.
    pub fn avoiding(mut self, text: &'a str) -> Self {
        self.text = text;
        self
    }

    fn is_free(&self, fake: &str, real: &str) -> bool {
        fake != real && !self.vault.is_taken(fake) && !occurs_as_token(self.text, fake)
    }

    /// Fake for `real`, creating and storing one if needed.
    pub fn fake_for(&mut self, kind: Kind, real: &str) -> String {
        if let Some(f) = self.vault.fake_for(real) {
            return f.to_string();
        }
        // Case variant of a known value: derive from the canonical fake.
        let canonical = self
            .vault
            .entries()
            .iter()
            .find(|e| e.real.eq_ignore_ascii_case(real) && e.real != real)
            .map(|e| e.fake.clone());
        let mut fake = canonical
            .map(|f| case_like(real, &f))
            .filter(|f| self.is_free(f, real))
            .unwrap_or_default();
        if fake.is_empty() {
            fake = self.generate_unique(kind, real);
        }
        self.vault.insert(kind.as_str(), real, &fake);
        fake
    }

    fn generate_unique(&mut self, kind: Kind, real: &str) -> String {
        if let Some(p) = self.preferred(kind, real) {
            if self.is_free(&p, real) {
                return p;
            }
        }
        for _ in 0..64 {
            let c = self.generate(kind, real);
            if self.is_free(&c, real) {
                return c;
            }
        }
        // The plain first x last combinations are used up: double-barrelled surnames
        // ("Almeida-Rocha") multiply the space while still looking like a name.
        if kind == Kind::Person {
            for _ in 0..256 {
                let c = case_like(real, &self.compound_name(real.split_whitespace().count()));
                if self.is_free(&c, real) {
                    return c;
                }
            }
        }
        loop {
            let c = format!(
                "{}{}",
                self.generate(kind, real),
                self.rng.gen_range(10..9999)
            );
            if self.is_free(&c, real) {
                return c;
            }
        }
    }

    /// Persona-level preferred fakes (primary org -> persona company).
    fn preferred(&self, kind: Kind, real: &str) -> Option<String> {
        let (full, stem) = self.primary_org.as_ref()?;
        if !matches!(kind, Kind::Org | Kind::Term) {
            return None;
        }
        if real.eq_ignore_ascii_case(full) {
            Some(self.persona.company.clone())
        } else if real.eq_ignore_ascii_case(stem) {
            Some(case_like(real, &self.persona.company_short))
        } else {
            None
        }
    }

    fn digit(&mut self) -> char {
        char::from_digit(self.rng.gen_range(0..10), 10).unwrap()
    }

    /// Replace each char by a random one of the same class.
    fn scramble(&mut self, s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '0'..='9' => self.digit(),
                'a'..='z' => (b'a' + self.rng.gen_range(0..26)) as char,
                'A'..='Z' => (b'A' + self.rng.gen_range(0..26)) as char,
                _ => c,
            })
            .collect()
    }

    /// Same length, characters drawn from `alphabet` (keeps a value's character set).
    fn draw_from(&mut self, s: &str, alphabet: &str) -> String {
        let a: Vec<char> = alphabet.chars().collect();
        s.chars()
            .map(|_| a[self.rng.gen_range(0..a.len())])
            .collect()
    }

    fn person_name(&mut self, words: usize) -> String {
        let r = self.persona.region_data();
        let last = *r.last.choose(&mut self.rng).unwrap();
        if words <= 1 {
            last.to_string()
        } else {
            format!("{} {last}", r.first.choose(&mut self.rng).unwrap())
        }
    }

    /// A name with a double-barrelled surname.
    fn compound_name(&mut self, words: usize) -> String {
        let r = self.persona.region_data();
        let mut two = r.last.choose_multiple(&mut self.rng, 2);
        let (a, b) = (two.next().unwrap(), two.next().unwrap_or(&"Lee"));
        if words <= 1 {
            format!("{a}-{b}")
        } else {
            format!("{} {a}-{b}", r.first.choose(&mut self.rng).unwrap())
        }
    }

    fn generate(&mut self, kind: Kind, real: &str) -> String {
        match kind {
            Kind::Email => self.email(real),
            Kind::Hostname => self.hostname(real),
            Kind::Person => {
                let n = self.person_name(real.split_whitespace().count());
                case_like(real, &n)
            }
            Kind::Org => {
                let stem = codeword(&mut self.rng);
                if is_org_name(real) {
                    // Keep the real legal form ("Globex Ltd" -> "Zephyr Ltd"): the text around it
                    // was written for that suffix, so a different one ("Co., Ltd.") can collide
                    // with a following full stop.
                    let t = real.trim();
                    format!("{stem}{}", &t[strip_legal_suffix(t).len()..])
                } else {
                    case_like(real, &stem)
                }
            }
            Kind::Term => {
                let n = real.split_whitespace().count().max(1);
                let words: Vec<String> = (0..n).map(|_| codeword(&mut self.rng)).collect();
                case_like(real, &words.join(" "))
            }
            Kind::Username => {
                let r = self.persona.region_data();
                let f = r.first.choose(&mut self.rng).unwrap();
                let l = r.last.choose(&mut self.rng).unwrap();
                format!("{}{}", f[..1].to_ascii_lowercase(), l.to_ascii_lowercase())
            }
            Kind::Address => {
                let street = *self
                    .persona
                    .region_data()
                    .streets
                    .choose(&mut self.rng)
                    .unwrap();
                format!("{} {street}", self.rng.gen_range(1..400))
            }
            Kind::Phone => self.phone(real),
            Kind::CreditCard => self.card(real),
            Kind::Iban => self.iban(real),
            Kind::NationalId => self.national_id(real),
            Kind::Ip => self.ip(real),
            Kind::Mac => real
                .chars()
                .map(|c| {
                    if c.is_ascii_hexdigit() {
                        let h = "0123456789abcdef".as_bytes()[self.rng.gen_range(0..16)] as char;
                        if c.is_ascii_uppercase() {
                            h.to_ascii_uppercase()
                        } else {
                            h
                        }
                    } else {
                        c
                    }
                })
                .collect(),
            Kind::Secret => self.secret(real),
            Kind::Date => self.date(real),
        }
    }

    fn email(&mut self, real: &str) -> String {
        let (local, domain) = real.rsplit_once('@').unwrap_or((real, "example.com"));
        let dl = domain.to_ascii_lowercase();
        // Webmail and public domains (`gmail.com`, `example.com`) identify nobody: keeping
        // them also keeps them out of the vault, so they are not masked elsewhere.
        let fake_domain =
            if crate::detect::names_webmail(&dl) || crate::detect::names_public_domain(&dl) {
                dl
            } else {
                self.fake_for(Kind::Hostname, domain)
            };
        let sep = local.chars().find(|c| matches!(c, '.' | '_' | '-'));
        let r = self.persona.region_data();
        let f = r.first.choose(&mut self.rng).unwrap().to_ascii_lowercase();
        let l = r.last.choose(&mut self.rng).unwrap().to_ascii_lowercase();
        let fake_local = match sep {
            Some(s) => format!("{f}{s}{l}"),
            None if local.len() <= 3 => f[..1].to_string() + &l[..1.min(l.len())],
            None => format!("{}{l}", &f[..1]),
        };
        let digits: String = local.chars().filter(char::is_ascii_digit).collect();
        let fake_local = if digits.is_empty() {
            fake_local
        } else {
            format!("{fake_local}{}", self.scramble(&digits))
        };
        format!("{}@{fake_domain}", case_like(local, &fake_local))
    }

    fn hostname(&mut self, real: &str) -> String {
        let lower = real.to_ascii_lowercase();
        let labels: Vec<&str> = lower.split('.').collect();
        if labels.len() == 1 {
            return format!(
                "{}-{}",
                codeword(&mut self.rng).to_lowercase(),
                self.rng.gen_range(10..99)
            );
        }
        let base_n = if labels.len() >= 3
            && SECOND_LEVEL.contains(&labels[labels.len() - 2..].join(".").as_str())
        {
            3
        } else {
            2
        };
        if labels.len() > base_n {
            let base = labels[labels.len() - base_n..].join(".");
            let fake_base = self.fake_for(Kind::Hostname, &base);
            let subs: Vec<String> = labels[..labels.len() - base_n]
                .iter()
                .map(|l| {
                    if crate::detect::names_generic_label(l) {
                        l.to_string()
                    } else {
                        codeword(&mut self.rng).to_lowercase()
                    }
                })
                .collect();
            return format!("{}.{fake_base}", subs.join("."));
        }
        let tld = labels[labels.len() - 1];
        let first = labels[0];
        let is_org = self.primary_org.as_ref().is_some_and(|(_, stem)| {
            let s: String = stem
                .to_ascii_lowercase()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect();
            !s.is_empty() && first.replace('-', "").contains(&s)
        });
        if is_org && !self.vault.is_taken(&self.persona.domain) {
            return self.persona.domain.clone();
        }
        let ptld = if crate::detect::names_internal_tld(tld) {
            tld.to_string()
        } else {
            self.persona.region_data().tld.to_string()
        };
        format!("{}.{ptld}", codeword(&mut self.rng).to_lowercase())
    }

    fn phone(&mut self, real: &str) -> String {
        let mut out = String::new();
        let mut rest = real;
        if let Some(r) = real.strip_prefix('+') {
            let cc_len = r
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(2)
                .min(3)
                .min(r.len());
            out.push('+');
            out.push_str(&self.persona.dial_code);
            rest = &r[cc_len..];
        }
        let mut first = true;
        for c in rest.chars() {
            if c.is_ascii_digit() {
                let d = if first {
                    char::from_digit(self.rng.gen_range(2..10), 10).unwrap()
                } else {
                    self.digit()
                };
                first = false;
                out.push(d);
            } else {
                out.push(c);
            }
        }
        out
    }

    fn card(&mut self, real: &str) -> String {
        let digits: Vec<u32> = real.chars().filter_map(|c| c.to_digit(10)).collect();
        if digits.len() < 2 {
            // A model called something a card that has no card number in it.
            return self.scramble(real);
        }
        let keep = if digits[0] == 3 { 2 } else { 1 };
        let mut new: Vec<u32> = digits[..keep].to_vec();
        while new.len() < digits.len() - 1 {
            new.push(self.rng.gen_range(0..10));
        }
        new.push(luhn_check_digit(&new));
        let mut it = new.into_iter();
        real.chars()
            .map(|c| {
                if c.is_ascii_digit() {
                    char::from_digit(it.next().unwrap(), 10).unwrap()
                } else {
                    c
                }
            })
            .collect()
    }

    fn iban(&mut self, real: &str) -> String {
        let compact: String = real
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_uppercase();
        if compact.len() < 5 || !compact.is_ascii() {
            // Not an IBAN after all (a model's guess): keep its shape only.
            return self.scramble(real);
        }
        let cc = &compact[..2];
        let bban = self.scramble(&compact[4..]);
        let full = format!("{cc}{}{bban}", iban_check_digits(cc, &bban));
        let mut it = full.chars();
        real.chars()
            .map(|c| {
                if c.is_whitespace() {
                    c
                } else {
                    it.next().unwrap()
                }
            })
            .collect()
    }

    fn national_id(&mut self, real: &str) -> String {
        loop {
            let mut first = true;
            let s: String = real
                .chars()
                .map(|c| {
                    if c.is_ascii_digit() {
                        let d = if first {
                            char::from_digit(self.rng.gen_range(2..7), 10).unwrap()
                        } else {
                            self.digit()
                        };
                        first = false;
                        d
                    } else if c.is_ascii_uppercase() {
                        // IDs such as India's PAN carry name initials in their letters.
                        (b'A' + self.rng.gen_range(0..26)) as char
                    } else if c.is_ascii_lowercase() {
                        (b'a' + self.rng.gen_range(0..26)) as char
                    } else {
                        c
                    }
                })
                .collect();
            if !s.starts_with("666") {
                return s;
            }
        }
    }

    /// A different, plausible date in the same format: year shifted by 1-3, new month and
    /// day, same separators, zero-padding, order and month-name language/case.
    fn date(&mut self, real: &str) -> String {
        let Some((y, m, d)) = parse_date(real) else {
            return self.scramble(real);
        };
        // Runs of digits / letters / other, with their roles.
        let mut runs: Vec<(String, u8)> = vec![]; // 0 digit, 1 alpha, 2 other
        for c in real.chars() {
            let class = if c.is_ascii_digit() {
                0
            } else if c.is_alphabetic() {
                1
            } else {
                2
            };
            match runs.last_mut() {
                Some((s, k)) if *k == class => s.push(c),
                _ => runs.push((c.to_string(), class)),
            }
        }
        let month_word = runs
            .iter()
            .position(|(s, k)| *k == 1 && month_lookup(s).is_some());
        let nums: Vec<usize> = (0..runs.len()).filter(|i| runs[*i].1 == 0).collect();
        // Role per numeric run: 'y', 'm' or 'd' (mirrors parse_date).
        let mut roles = vec![' '; runs.len()];
        match (month_word, nums.as_slice()) {
            (Some(_), [a, b]) if runs[*a].0.len() == 4 => (roles[*a], roles[*b]) = ('y', 'd'),
            (Some(_), [a, b]) => (roles[*a], roles[*b]) = ('d', 'y'),
            (None, [a, b, c]) if runs[*a].0.len() == 4 => {
                (roles[*a], roles[*b], roles[*c]) = ('y', 'm', 'd')
            }
            (None, [a, b, c]) => {
                let first: u32 = runs[*a].0.parse().unwrap_or(0);
                if first > 12 {
                    (roles[*a], roles[*b], roles[*c]) = ('d', 'm', 'y');
                } else {
                    (roles[*a], roles[*b], roles[*c]) = ('m', 'd', 'y');
                }
            }
            _ => return self.scramble(real),
        }
        // Day/month order is ambiguous when both are <= 12: keep the fake ambiguous too.
        let ambiguous = month_word.is_none() && roles.first() != Some(&'y') && d <= 12;
        let shift = self.rng.gen_range(1..=3) * if self.rng.gen_bool(0.5) { 1 } else { -1 };
        let ny = y + shift;
        let nm = loop {
            let x = self.rng.gen_range(1..=12);
            if x != m {
                break x;
            }
        };
        // ...and an unambiguous numeric date (31/12) stays unambiguous (day above 12).
        let numeric = month_word.is_none() && roles.first() != Some(&'y');
        let days = if ambiguous {
            1..=12
        } else if numeric {
            13..=28
        } else {
            1..=28
        };
        let nd = loop {
            let x = self.rng.gen_range(days.clone());
            if x != d {
                break x;
            }
        };
        let mut out = String::new();
        for (i, (s, k)) in runs.iter().enumerate() {
            let pad = |v: u32| {
                if s.len() >= 2 {
                    format!("{v:02}")
                } else {
                    v.to_string()
                }
            };
            match (k, roles[i]) {
                (0, 'y') if s.len() == 2 => out.push_str(&format!("{:02}", ny.rem_euclid(100))),
                (0, 'y') => out.push_str(&ny.to_string()),
                (0, 'm') => out.push_str(&pad(nm)),
                (0, 'd') => out.push_str(&pad(nd)),
                (1, _) if Some(i) == month_word => {
                    let (row, _) = month_lookup(s).unwrap();
                    let name = MONTHS[row][nm as usize - 1];
                    let name = if s.chars().next().is_some_and(char::is_uppercase)
                        && !s.chars().all(char::is_uppercase)
                    {
                        let mut c = name.chars();
                        c.next()
                            .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                            .unwrap_or_default()
                    } else {
                        case_like(s, name)
                    };
                    out.push_str(&name);
                }
                (1, _) if matches!(s.as_str(), "st" | "nd" | "rd" | "th") => {
                    out.push_str(match nd {
                        1 | 21 | 31 => "st",
                        2 | 22 => "nd",
                        3 | 23 => "rd",
                        _ => "th",
                    })
                }
                _ => out.push_str(s),
            }
        }
        out
    }

    fn ip(&mut self, real: &str) -> String {
        if real.contains(':') {
            return self
                .scramble(&real.to_ascii_lowercase())
                .chars()
                .map(|c| {
                    if c.is_ascii_lowercase() {
                        "abcdef".as_bytes()[self.rng.gen_range(0..6)] as char
                    } else {
                        c
                    }
                })
                .collect();
        }
        let o: Vec<u8> = real.split('.').filter_map(|p| p.parse().ok()).collect();
        if o.len() != 4 {
            // An ML detector called something an IP that isn't dotted IPv4: keep its shape only.
            return self.scramble(real);
        }
        let r = |rng: &mut ThreadRng, lo: u8, hi: u8| rng.gen_range(lo..=hi);
        let rng = &mut self.rng;
        let (a, b) = match (o[0], o[1]) {
            (10, _) => (10, r(rng, 0, 255)),
            (172, 16..=31) => (172, r(rng, 16, 31)),
            (192, 168) => (192, 168),
            (169, 254) => (169, 254),
            (100, 64..=127) => (100, r(rng, 64, 127)),
            _ => {
                let nets = [(203, 0, 113), (198, 51, 100), (192, 0, 2)];
                let (x, y, z) = *nets.choose(rng).unwrap();
                return format!("{x}.{y}.{z}.{}", r(rng, 1, 254));
            }
        };
        format!("{a}.{b}.{}.{}", r(rng, 0, 255), r(rng, 1, 254))
    }

    fn secret(&mut self, real: &str) -> String {
        if real.starts_with("-----BEGIN") {
            let lines: Vec<&str> = real.lines().collect();
            let n = lines.len();
            return lines
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    if i == 0 || i + 1 == n {
                        l.to_string()
                    } else {
                        self.scramble(l)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
        }
        // Wallet addresses keep their shape: 0x + hex, bc1 + bech32, legacy base58.
        if let Some(hex) = real
            .strip_prefix("0x")
            .filter(|h| h.chars().all(|c| c.is_ascii_hexdigit()))
        {
            return format!("0x{}", self.draw_from(hex, "0123456789abcdef"));
        }
        if let Some(rest) = real
            .strip_prefix("bc1")
            .or_else(|| real.strip_prefix("BC1"))
        {
            return format!(
                "{}{}",
                &real[..3],
                self.draw_from(rest, "qpzry9x8gf2tvdw0s3jn54khce6mua7l")
            );
        }
        if (real.starts_with('1') || real.starts_with('3'))
            && (26..=35).contains(&real.len())
            && real
                .chars()
                .all(|c| c.is_ascii_alphanumeric() && !"0OIl".contains(c))
        {
            return format!(
                "{}{}",
                &real[..1],
                self.draw_from(
                    &real[1..],
                    "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
                )
            );
        }
        let prefix = SECRET_PREFIXES
            .iter()
            .find(|p| real.starts_with(*p))
            .copied()
            .unwrap_or("");
        format!("{prefix}{}", self.scramble(&real[prefix.len()..]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_faker(f: impl FnOnce(&mut Faker<'_>)) {
        let dir = tempfile::tempdir().unwrap();
        let persona = Persona::generate(Some("US"), &mut rand::thread_rng());
        let mut vault = Vault::open(&dir.path().join("v"), &dir.path().join("k")).unwrap();
        let mut faker = Faker::new(&persona, &mut vault, None);
        f(&mut faker);
    }

    #[test]
    fn email_fakes_keep_public_domains() {
        with_faker(|f| {
            let fake = f.fake_for(Kind::Email, "jane.roe@example.com");
            assert!(fake.ends_with("@example.com"), "{fake}");
            assert_ne!(fake, "jane.roe@example.com");
            let private = f.fake_for(Kind::Email, "jane.roe@roe-family.net");
            assert!(!private.ends_with("@roe-family.net"), "{private}");
        });
    }

    #[test]
    fn ip_fakes_survive_odd_input() {
        // ML detectors sometimes label things that are not IPs as IP.
        with_faker(|f| {
            for real in ["1.2.3", "Rua XV", "300.1.1.1", "10.0.0.1"] {
                let fake = f.fake_for(Kind::Ip, real);
                assert!(!fake.is_empty() && fake != real, "{real} -> {fake}");
            }
        });
    }

    #[test]
    fn every_kind_survives_odd_input() {
        // Model detectors can label any text with any kind.
        let kinds = [
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
            Kind::Date,
        ];
        let odd = [
            "x",
            "12",
            "-",
            "€",
            "Ü",
            "@",
            "a@",
            "@b",
            ".",
            "..",
            "1.",
            "+",
            "+4",
            "DE",
            "GB1",
            "4111",
            "12/",
            "/12/",
            "1987-13-45",
            "31.02.",
            "Jan",
            "-----BEGIN",
            "sk-",
            "ab:cd",
            "::",
            "Straße 5",
            "日本語",
            "Ab Cd Ef Gh Ij Kl",
            "(555)",
            "0000-0000",
            "a.b.c.d.e",
            "ﬁ",
            "İstanbul",
            "ß@ß.ß",
        ];
        // A fresh vault per kind: a cached fake from an earlier kind would skip the generator.
        for k in kinds {
            with_faker(|f| {
                for real in odd {
                    let _ = f.fake_for(k, real);
                }
            });
        }
    }

    #[test]
    fn date_fakes_keep_format() {
        with_faker(|f| {
            for real in [
                "1987-03-04",
                "04/03/1987",
                "31/12/1990",
                "12.05.1975",
                "March 4, 1987",
                "4 March 1987",
                "4. März 1987",
                "4 de marzo de 1987",
                "June 1st, 1990",
            ] {
                let fake = f.fake_for(Kind::Date, real);
                assert_ne!(fake, real);
                let (y, m, d) = parse_date(real).unwrap();
                let (fy, fm, fd) = parse_date(&fake).unwrap_or_else(|| panic!("{real} -> {fake}"));
                assert!((1..=3).contains(&(fy - y).abs()), "{real} -> {fake}");
                assert!(fm != m && fd != d, "{real} -> {fake}");
                // Same shape: digits stay digits, separators and words stay in place.
                let shape = |s: &str| {
                    s.chars()
                        .map(|c| {
                            if c.is_ascii_digit() {
                                '9'
                            } else if c.is_alphabetic() {
                                'a'
                            } else {
                                c
                            }
                        })
                        .collect::<String>()
                        .split('a')
                        .filter(|p| !p.is_empty())
                        .map(|p| p.replace('9', ""))
                        .collect::<Vec<_>>()
                };
                assert_eq!(shape(real), shape(&fake), "{real} -> {fake}");
                // Persisted: same real, same fake.
                assert_eq!(f.fake_for(Kind::Date, real), fake);
            }
            let iso = f.fake_for(Kind::Date, "2001-09-11");
            assert_eq!(iso.len(), 10);
            assert_eq!(&iso[4..5], "-");
        });
    }
}
