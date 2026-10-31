//! Built-in regex + checksum detectors.

use std::sync::LazyLock;

use regex::Regex;

use super::address::{AddressDetector, OrgDetector};
use super::checksum::{
    aadhaar_valid, abn_valid, bsn_valid, btc_base58_valid, btc_bech32_valid, codice_fiscale_valid,
    cpf_valid, iban_checksum_ok, iban_country, iban_valid, insee_valid, luhn_valid, nhs_valid,
    nif_valid, nino_valid, ssn_valid, steuer_id_valid, tfn_valid,
};
use super::fields::{BirthDateDetector, FieldDetector};
use super::names::{self, INTERNAL_TLDS, PUBLIC_DOMAINS, TLDS};
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
// Phone candidates: optional `+`/`(`, digits with common separators, optional extension.
re!(
    PHONE_CAND,
    r"(?:\+|\(\+?)?\d[\d \-./()\u{a0}]{4,26}\d(?:[^\S\n]?(?:x|ext\.?|extn\.?|extension|#)[^\S\n]?\d{1,6})?"
);
// Words that announce a phone number shortly before it (same line).
re!(
    PHONE_CTX,
    r"(?i)(?:\b(?:phone|telephone|tel|tél|mobile|mob|cell|cellphone|fax|call(?:ed|s|ing)?|ring|whatsapp|sms|text|telefon|telefonnummer|handy|mobil|téléphone|portable|teléfono|telefono|móvil|movil|cellulare|telefoon|mobiel|telefone|celular|contato|ligar|llamar|chiamare|appeler|anrufen|bellen)\b|電話|携帯)"
);
re!(CARD, r"\b(?:\d[ \-]?){12,18}\d\b");
re!(
    IBAN,
    r"\b[A-Z]{2}\d{2}(?: ?[A-Z0-9]{4}){2,7}(?: ?[A-Z0-9]{1,4})?\b"
);
// US SSN: dashed or spaced (consistent separators); the bare 9-digit form needs context.
re!(SSN, r"\b\d{3}([- ]?)\d{2}([- ]?)\d{4}\b");
re!(NINO, r"\b[A-Z]{2} ?\d{2} ?\d{2} ?\d{2} ?[A-D]\b");
re!(AADHAAR, r"\b[2-9]\d{3}([ \-]?)\d{4}([ \-]?)\d{4}\b");
re!(CPF, r"\b\d{3}\.\d{3}\.\d{3}-\d{2}\b|\b\d{11}\b");
re!(BSN, r"\b\d{4}\.\d{2}\.\d{3}\b|\b\d{9}\b");
re!(NIF, r"\b(?:\d{8}|[XYZ]\d{7})-?[A-Z]\b");
re!(
    CODICE_FISCALE,
    r"(?i)\b[A-Z]{6}[0-9LMNPQRSTUV]{2}[ABCDEHLMPRST][0-9LMNPQRSTUV]{2}[A-Z][0-9LMNPQRSTUV]{3}[A-Z]\b"
);
re!(
    INSEE,
    r"\b[12378] ?\d{2} ?\d{2} ?(?:\d{2}|2[AB]) ?\d{3} ?\d{3} ?\d{2}\b"
);
re!(STEUER_ID, r"\b[1-9]\d(?: \d{3}){3}\b|\b[1-9]\d{10}\b");
// "<ID label>[ number][:] <value>": a value right after a national-ID label is taken even
// when its format or checksum is unknown.
re!(
    LABELLED_ID,
    r"(?i:\b(social[ -]security(?:[ -]number)?|ssn|national[ -]insurance(?:[ -]number)?|nino|aadhaa?r(?:[ -]card)?|uidai|cpf|bsn|burgerservicenummer|sofinummer|nif|nie|dni|nie/nif|nif/nie|codice[ -]fiscale|cf|insee|nir|num[ée]ro[ -]de[ -]s[ée]curit[ée][ -]sociale|s[ée]curit[ée][ -]sociale|steuer[ -]?id|steuer-?identifikationsnummer|identifikationsnummer|idnr|personalausweis(?:nummer)?|ausweisnummer|passport|passnummer|reisepass(?:nummer)?|pasaporte|passeport|passaporto|paspoort|passaporte|pan|tax[ -]id|tin|itin|nhs|tfn|national[ -]id|id[ -]card|identity[ -]card|carte[ -]d'identit[ée]|carta[ -]d'identit[àa]|documento[ -]de[ -]identidad|rg|driver'?s?[ -]licen[cs]e|driving[ -]licen[cs]e|licen[cs]e[ -]number|f(?:ü|ue)hrerschein(?:nummer)?|rijbewijs(?:nummer)?|k(?:ö|o)rkort(?:snummer)?|permis[ -]de[ -]conduire|licencia[ -]de[ -]conduc(?:ir|ci(?:ó|o)n)|carnet[ -]de[ -]conducir|patente[ -]di[ -]guida|personnummer|paspoortnummer|passaportnummer|numero[ -]di[ -]passaporto|n(?:ú|u)mero[ -]de[ -]pasaporte|num(?:é|e)ro[ -]de[ -]passeport)\b)[^\S\n]*(?i:(number|no\.|no|nr\.?|num\.?|n°|nº|n\.|#)[^\S\n]*)?(?i:(?:of|is|was|è|es|est|ist|är|lautet|luidt|era)[^\S\n]+)?([:#=(])?[^\S\n]*(?i:(?:n\.|no\.|nr\.?)[^\S\n]*)?([A-Z0-9]+(?:[ .\-/][A-Z0-9]+){0,6})"
);
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
re!(
    IPV6,
    r"(?i)[0-9a-f]{0,4}(?::[0-9a-f]{0,4}){2,7}(?:\.\d{1,3}){0,3}"
);
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
    PRIVATE_KEY,
    r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----"
);
re!(
    SECRET_KNOWN,
    r"\b(?:sk-ant-[A-Za-z0-9_\-]{20,}|sk-or-v1-[A-Za-z0-9]{20,}|sk-(?:proj-)?[A-Za-z0-9_\-]{20,}|gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}|xox[abposr]-[A-Za-z0-9\-]{10,}|(?:AKIA|ASIA)[0-9A-Z]{16}|AIza[0-9A-Za-z_\-]{35}|glpat-[A-Za-z0-9_\-]{20,}|gsk_[A-Za-z0-9]{30,}|xai-[A-Za-z0-9]{30,}|hf_[A-Za-z0-9]{30,}|npm_[A-Za-z0-9]{30,}|(?:sk|rk)_live_[A-Za-z0-9]{20,}|(?:sk|rk|pk)_test_[A-Za-z0-9]{20,}|SG\.[A-Za-z0-9_\-]{16,32}\.[A-Za-z0-9_\-]{16,64}|sq0(?:atp|csp)-[A-Za-z0-9_\-]{20,}|sl\.[A-Za-z0-9_\-]{24,})"
);
// `<key-like name> = value` / `"name": "value"` / `NAME: value`.
re!(
    SECRET_ASSIGN,
    r#"(?i)\b([A-Za-z0-9_.\-]*?(?:api[_\-]?key|apikey|secret|token|passwd|password|passphrase|pwd|pass|access[_\-]?key|private[_\-]?key|auth[_\-]?key|client[_\-]?secret|credentials?|signing[_\-]?key|encryption[_\-]?key|_key))["']?[^\S\n]*(?:=>|:=|=|:)[^\S\n]*(["'`]?)([^\s"'`,;)]{4,512})"#
);
// Credentials named in prose, in the supported languages: "my password is X", "API key: X",
// "lösenordet \"X\"", "Passwort: X", "mot de passe X", "contraseña: X".
re!(
    PROSE_SECRET,
    r#"(?i)\b(password|passwort|kennwort|wachtwoord|mot de passe|contraseña|senha|lösenord(?:et)?|passord|api[ _\-]?key|api[ \-]nyckel(?:n)?|api[ \-]?schlüssel|api[ \-]sleutel|clé api|clave api|chiave api|secret key|access token)\b(?:(?:[^\S\n]+[\p{L}]+){0,6}?[^\S\n]*(?:\*\*)?[^\S\n]*(?:[:=]|\b(?:is|was|è|es|est|ist|är|lautet|luidt|era|could be|would be|will be))|[^\S\n]+(?:of|de|da|for|för|für|voor|per))?[^\S\n]*(?:\*\*)?[^\S\n]*(["'“‘(\[]?)([^\s"'”’]{6,128})"#
);
// `<password>…</password>`, `<APIKey>…</APIKey>` elements.
re!(
    XML_SECRET,
    r"(?i)<((?:[\w.\-]+:)?(?:password|passwd|pwd|passwort|wachtwoord|api[_\-]?key|secret|secret[_\-]?key|access[_\-]?token|token))>([^<>\s]{6,128})</"
);
// A long random-looking token standing alone: letters of both cases and digits.
re!(RANDOM_TOKEN, r"\b[A-Za-z0-9]{24,64}\b");
// JSON Web Tokens: base64url header (`{"` = `eyJ`), payload and signature.
re!(
    JWT,
    r"\beyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}"
);
// Basic-auth tuples: auth=("user", "pw"), HTTPBasicAuth('user', 'pw').
re!(
    AUTH_TUPLE,
    r#"(?i)\b(?:auth|basic_?auth|httpbasicauth|httpdigestauth|credentials|login)[^\S\n]*[=(:][^\S\n]*[(\[]?[^\S\n]*["']([^"'\s]{1,64})["'][^\S\n]*,[^\S\n]*["']([^"'\n]{1,128})["']"#
);
// URL userinfo: scheme://user[:password]@host
re!(
    URL_USERINFO,
    r"(?i)\b[a-z][a-z0-9+.\-]*://([^\s/@:]{1,64})(?::([^\s/@]{1,128}))?@[A-Za-z0-9\[]"
);
// `user@host` tokens (emails are told apart by their host and context).
re!(
    USER_AT_HOST,
    r"(?:^|[\s'\x22(=])([a-z_][a-z0-9_.\-]{0,31})@([A-Za-z0-9](?:[A-Za-z0-9.\-]*[A-Za-z0-9])?)"
);
// Shell commands whose `user@host` argument is a login, not an email.
re!(
    LOGIN_CMD,
    r"\b(?:ssh|scp|sftp|rsync|mosh|ssh-copy-id|telnet|ftp)\b"
);
// `username=x`, `"user": "x"`, `login: x`, `--user x`, `-u x` (for a few login commands).
re!(
    USER_ASSIGN,
    r#"(?i)(?:\b(user(?:[_\-]?name)?|login|db[_\-]?user|ssh[_\-]?user|admin[_\-]?user|uid)\b(["']?)[^\S\n]*(=|:)[^\S\n]*(["']?)|(?:^|\s)(?:--user(?:name)?|--login)(?:[^\S\n]+|=)["']?)([A-Za-z0-9_][A-Za-z0-9_.\-]{1,63})"#
);
re!(
    USER_FLAG,
    r#"\b(?:mysql|psql|mongo(?:sh)?|redis-cli|curl|docker\s+login|sudo|su|ssh|smbclient|ftp|htpasswd|influx|clickhouse-client|sqlcmd|mysqldump|pg_dump)\b[^\n]*?\s-(?:u|U|l)[^\S\n]*["']?([A-Za-z0-9_][A-Za-z0-9_.\-]{1,63})(?::([^\s"']{1,128}))?"#
);
re!(BEARER, r"(?i)\bbearer\s+([A-Za-z0-9\-._~+/]{20,}=*)");
// "user name colin64", "Username 'jdoe_7'", "authenticated user, kim.lee", "gebruiker t.jansson".
re!(
    USER_PROSE,
    r#"(?i)\b(?:user ?name|user id|login|authenticated user|gebruiker(?:snaam)?|benutzer(?:name)?|användar(?:namn|e)|usuario|nome utente|utente|nom d'utilisateur|identifiant|user)\b[,:]?[^\S\n]+(["'“‘]?)([A-Za-z0-9][A-Za-z0-9._\-]{2,31})\b"#
);
// A handle made of a given name and digits: "rebecca_282", "simon3334", "anna.k88".
re!(
    NAME_HANDLE,
    r"\b([a-z]{3,12})(?:[._]?[a-z]{0,12})?[._]?\d{2,4}\b"
);

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
        EMAIL
            .find_iter(text)
            .filter(|m| {
                // `ssh user@host` / `scp user@host:/path` are logins, not mailboxes.
                let mut after = text[m.end()..].chars();
                let scp_path = after.next() == Some(':')
                    && after
                        .next()
                        .is_some_and(|c| c == '/' || c == '~' || c.is_alphanumeric());
                !(scp_path || LOGIN_CMD.is_match(line_before(text, m.start())))
            })
            .map(|m| Finding::new(m.start(), m.end(), Kind::Email, "email"))
            .collect()
    }
}

/// True if `v` (a whole value) is shaped like a phone number: 7-15 digits (plus an optional
/// `(0)` trunk and extension) using only phone punctuation. Used for labelled fields.
pub(crate) fn phone_shape(v: &str) -> bool {
    let v = v.trim();
    let core = strip_extension(v);
    let digits = core.chars().filter(char::is_ascii_digit).count();
    (7..=16).contains(&digits)
        && core
            .chars()
            .all(|c| c.is_ascii_digit() || " +-./()\u{a0}".contains(c))
        && !looks_like_date(core)
        && !ip_like(core)
}

fn strip_extension(s: &str) -> &str {
    static EXT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)[^\S\n]?(?:x|ext\.?|extn\.?|extension|#)[^\S\n]?\d{1,6}$").unwrap()
    });
    match EXT.find(s) {
        Some(m) => &s[..m.start()],
        None => s,
    }
}

fn ip_like(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|p| (1..=3).contains(&p.len()) && p.chars().all(|c| c.is_ascii_digit()))
}

/// Digit groups of a phone candidate (separators: space, `-`, `.`, `/`, parentheses).
fn digit_groups(s: &str) -> Vec<&str> {
    s.split(|c: char| !c.is_ascii_digit())
        .filter(|g| !g.is_empty())
        .collect()
}

/// Structural phone cues that need no context. Prices, years, invoice numbers, dates and
/// thousands-separated amounts fail all of them.
fn phone_structure(core: &str) -> bool {
    let digits: usize = core.chars().filter(char::is_ascii_digit).count();
    let groups = digit_groups(core);
    let first = groups[0];
    if core.starts_with('+') {
        return (8..=15).contains(&digits);
    }
    if core.starts_with("00") {
        return (10..=16).contains(&digits);
    }
    if core.starts_with('0') || core.starts_with("(0") {
        // National trunk prefix; bare runs must be a full national number.
        return if groups.len() == 1 {
            (9..=12).contains(&digits) && !core[1..].starts_with('0')
        } else {
            (8..=13).contains(&digits)
        };
    }
    if core.starts_with('(') {
        // (415) 555-0134 / (051) 3785-7359: a bracketed area code then grouped digits.
        return (2..=5).contains(&first.len()) && groups.len() >= 2 && (7..=13).contains(&digits);
    }
    if groups.len() >= 3 && (7..=12).contains(&digits) && first.len() <= 4 {
        let rest = &groups[1..];
        let thousands = first.len() <= 3 && rest.iter().all(|g| g.len() == 3);
        let years = groups.iter().all(|g| g.len() == 4);
        return !thousands
            && !years
            && groups[groups.len() - 1].len() >= 3
            && groups.iter().all(|g| g.len() >= 2 || g == &first);
    }
    false
}

/// Phone numbers: international (E.164 with spaces/dots/dashes/parentheses), national
/// (leading 0 or `(0..)`), North-American `(415) 555-0134 x12`, grouped forms such as
/// `090-1234-5678`, and bare digit runs announced by a word like "phone"/"Tel."/"call".
pub struct PhoneDetector;
impl Detector for PhoneDetector {
    fn name(&self) -> &'static str {
        "phone"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = vec![];
        for m in PHONE_CAND.find_iter(text) {
            let mut start = m.start();
            let mut s = m.as_str();
            // An unmatched "(" belongs to the surrounding prose.
            if s.starts_with('(') && !s.contains(')') {
                start += 1;
                s = &s[1..];
            }
            let s = s.trim_end_matches(|c: char| !c.is_ascii_digit());
            let end = start + s.len();
            let prev = text[..start].chars().next_back();
            let next = text[end..].chars().next();
            let after_next = text[end..].chars().nth(1);
            if prev.is_some_and(|c| c.is_alphanumeric() || "+-./_#$€£¥@".contains(c))
                || next.is_some_and(|c| c.is_alphanumeric() || c == '%' || c == '@')
                || (matches!(next, Some('.' | '-' | '/' | ',' | ':'))
                    && after_next.is_some_and(|c| c.is_ascii_digit()))
            {
                continue;
            }
            let core = strip_extension(s);
            let digits = core.chars().filter(char::is_ascii_digit).count();
            if !(7..=16).contains(&digits) || looks_like_date(core) || ip_like(core) {
                continue;
            }
            // Decimals ("0.01234567" BTC, "0.0000000") and zero-padded record numbers
            // ("00000000123456") are not phone numbers.
            let decimal = core.split('.').count() == 2
                && core.chars().all(|c| c.is_ascii_digit() || c == '.')
                && core.split('.').next().is_some_and(|a| a.len() <= 3);
            // Two amounts ("0.00 125000.00", "155 000 - 17 000") or a reference with a year
            // or date after a slash ("221-2345/2023", "00/20210825") are not phones either.
            let amounts = core.contains(" - ")
                || core
                    .split(' ')
                    .filter(|g| g.split('.').nth(1).is_some_and(|d| d.len() == 2))
                    .count()
                    >= 2;
            let dated = core.rsplit_once('/').is_some_and(|(_, y)| {
                (y.len() == 4 || y.len() == 8) && (y.starts_with("19") || y.starts_with("20"))
            });
            if decimal || amounts || dated || core.starts_with("000") {
                continue;
            }
            let opens = core.matches('(').count();
            if opens != core.matches(')').count() {
                continue;
            }
            let before = line_before(text, start);
            let context = phone_context(before);
            // An unbroken digit run right after another kind of number label ("Partita IVA:",
            // "Account Number:") is that number, whatever its shape.
            let other_label =
                !context && core.chars().all(|c| c.is_ascii_digit()) && other_number_label(before);
            if (phone_structure(core) && !other_label) || context {
                out.push(Finding::new(start, end, Kind::Phone, "phone"));
            }
        }
        out
    }
}

/// A phone keyword shortly before the number, with only a few non-numeric words between
/// ("calls on", "Tel.", "phone number:") and nothing that names another kind of number.
fn phone_context(before: &str) -> bool {
    let Some(k) = PHONE_CTX.find_iter(before).last() else {
        return false;
    };
    let gap = before[k.end()..].to_lowercase();
    const OTHER: &[&str] = &[
        "invoice", "order", "ticket", "ref", "account", "iban", "card", "code", "pin", "zip", "id",
    ];
    gap.len() <= 24
        && !gap.chars().any(|c| c.is_ascii_digit())
        && gap.split_whitespace().count() <= 3
        && !gap
            .split(|c: char| !c.is_alphanumeric())
            .any(|w| OTHER.contains(&w))
}

/// True if `before` (same line) names a non-phone number: an account, tax or record number.
fn other_number_label(before: &str) -> bool {
    const LABELS: &[&str] = &[
        "account",
        "acct",
        "konto",
        "rekening",
        "cuenta",
        "conto",
        "compte",
        "iva",
        "vat",
        "tax",
        "steuer",
        "invoice",
        "order",
        "ref",
        "id",
        "number",
        "nummer",
        "numero",
        "número",
        "numéro",
        "nr",
        "no",
        "policy",
        "polizza",
        "póliza",
        "contract",
        "ticket",
        "transaction",
        "routing",
        "sort",
        "clearing",
        "customer",
        "client",
        "member",
        "bai",
    ];
    before
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| LABELS.contains(&w))
}

/// The text from the start of the line (at most 48 bytes back) to `pos`.
fn line_before(text: &str, pos: usize) -> &str {
    let mut from = pos.saturating_sub(48);
    while !text.is_char_boundary(from) {
        from += 1;
    }
    let w = &text[from..pos];
    match w.rfind('\n') {
        Some(i) => &w[i + 1..],
        None => w,
    }
}

/// Words that announce a payment card number shortly before it.
const CARD_CTX: &[&str] = &[
    "card number",
    "card no",
    "card_number",
    "card#",
    "credit card",
    "debit card",
    "kreditkart",
    "karten",
    "carte",
    "tarjeta",
    "carta",
    "kaartnummer",
    "creditcard",
    "kredietkaart",
    "kortnummer",
    "kreditkort",
    "betalkort",
];

/// Payment cards: Luhn-validated with a known network prefix, or any 13-19 digit number in
/// card grouping right after a card label ("Card number: 4141-6246-9771-0810"), since test
/// and sample documents often carry numbers that fail the checksum.
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
                if !(13..=19).contains(&d.len()) {
                    return false;
                }
                if network && luhn_valid(&d) {
                    return true;
                }
                // Groups of four (the last may be shorter), or one unbroken run.
                let groups: Vec<&str> = m.as_str().split([' ', '-']).collect();
                let grouped = groups.len() == 1
                    || (groups[..groups.len() - 1].iter().all(|g| g.len() == 4)
                        && (1..=4).contains(&groups[groups.len() - 1].len()))
                    || groups.iter().map(|g| g.len()).eq([4, 6, 5]);
                grouped && has_context(text, m.start(), CARD_CTX) && !dummy_number(&d)
            })
            .map(|m| Finding::new(m.start(), m.end(), Kind::CreditCard, "credit_card"))
            .collect()
    }
}

// A value right after an "IBAN" label, in any grouping: "IBAN: IT65Q 01005 03211 000000001234".
re!(
    LABELLED_IBAN,
    r"(?i:\biban\b)[^\S\n]*(?i:code|number|no\.?|nr\.?|nummer|numero|número|controparte)?[^\S\n]*[:#]?[^\S\n]*([A-Z]{2}\d{2}(?:[^\S\n]?[A-Z0-9]){11,32})\b"
);

/// IBANs (mod-97 validated). Over-long greedy matches are trimmed group by group. A value
/// right after an "IBAN" label is taken when its country code and length fit, even with a
/// wrong checksum (sample and test documents are full of those).
pub struct IbanDetector;
impl Detector for IbanDetector {
    fn name(&self) -> &'static str {
        "iban"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = vec![];
        for c in LABELLED_IBAN.captures_iter(text) {
            let m = c.get(1).unwrap();
            let mut v = m.as_str();
            // "IBAN: DE12 3456 ... BIC": a trailing all-letter group is the next word.
            while let Some((head, tail)) = v.rsplit_once(' ') {
                if tail.chars().all(|c| c.is_ascii_alphabetic()) {
                    v = head.trim_end();
                } else {
                    break;
                }
            }
            let compact = v.chars().filter(|c| !c.is_whitespace()).count();
            if (15..=34).contains(&compact) && iban_country(&v[..2]) {
                out.push(Finding::new(
                    m.start(),
                    m.start() + v.len(),
                    Kind::Iban,
                    "iban",
                ));
            }
        }
        for m in IBAN.find_iter(text) {
            let s = m.as_str();
            let mut end = s.len();
            let labelled = has_context(text, m.start(), &["iban"]);
            loop {
                if out.iter().any(|f: &Finding| f.start == m.start()) {
                    break;
                }
                if iban_valid(&s[..end]) || (labelled && iban_checksum_ok(&s[..end])) {
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

/// National identifiers: US SSN/ITIN, UK NINO/NHS, India Aadhaar/PAN, Brazil CPF, NL BSN,
/// Spain NIF/NIE, Italy codice fiscale, France INSEE, Germany Steuer-ID, Australia TFN/ABN,
/// plus any value right after an ID label ("SSN:", "Passport No.", "Steuer-ID", ...).
pub struct NationalIdDetector;

fn nid(start: usize, end: usize) -> Finding {
    Finding::new(start, end, Kind::NationalId, "national_id")
}

impl NationalIdDetector {
    /// Values following an ID label. Short (≤4 char) labels need an explicit `:`/`#`/"No.".
    fn labelled(text: &str, out: &mut Vec<Finding>) {
        for c in LABELLED_ID.captures_iter(text) {
            let label = c.get(1).unwrap().as_str();
            let explicit = c.get(2).is_some() || c.get(3).is_some();
            if label.len() <= 4 && !explicit {
                continue;
            }
            let v = c.get(4).unwrap();
            let (start, mut end) = (v.start(), v.end());
            // A value glued to the following word ended mid-token: drop trailing groups.
            while text[end..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_alphanumeric())
            {
                match text[start..end].rfind([' ', '.', '-', '/']) {
                    Some(i) => end = start + i,
                    None => {
                        end = start;
                        break;
                    }
                }
            }
            // A following all-letter word ("SSN: 123-45-6789 DOB ...") is not part of it.
            let mut off = 0;
            for (i, g) in text[start..end].split(' ').enumerate() {
                if i > 0 && g.len() >= 2 && g.chars().all(|c| c.is_ascii_alphabetic()) {
                    end = start + off - 1;
                    break;
                }
                off += g.len() + 1;
            }
            let value = &text[start..end];
            let digits = value.chars().filter(char::is_ascii_digit).count();
            let alnum = value.chars().filter(char::is_ascii_alphanumeric).count();
            // A label is weak evidence for a template number ("BSN: 123456789").
            if digits >= 4
                && (5..=24).contains(&alnum)
                && !looks_like_date(value)
                && !dummy_number(value)
            {
                out.push(Finding {
                    score: 0.9,
                    ..nid(start, end)
                });
            }
        }
    }
}

/// True for `2024-01-31`, `31/01/2024`, `01.31.24` style values.
pub(crate) fn looks_like_date(s: &str) -> bool {
    static DATE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:\d{4}[-./]\d{1,2}[-./]\d{1,2}|\d{1,2}[-./]\d{1,2}[-./]\d{2,4})$").unwrap()
    });
    DATE.is_match(s.trim())
}

impl Detector for NationalIdDetector {
    fn name(&self) -> &'static str {
        "national_id"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = vec![];
        const SSN_CTX: &[&str] = &["ssn", "social security", "social sec"];
        for c in SSN.captures_iter(text) {
            let m = c.get(0).unwrap();
            let (s1, s2) = (&c[1], &c[2]);
            if s1 != s2 || !ssn_valid(m.as_str()) {
                continue;
            }
            if s1 == "-" || has_context(text, m.start(), SSN_CTX) {
                out.push(nid(m.start(), m.end()));
            }
        }
        out.extend(
            NINO.find_iter(text)
                .filter(|m| nino_valid(m.as_str()))
                .map(|m| nid(m.start(), m.end())),
        );
        const AADHAAR_CTX: &[&str] = &["aadhaar", "aadhar", "uid"];
        for c in AADHAAR.captures_iter(text) {
            let m = c.get(0).unwrap();
            if c[1] == c[2]
                && aadhaar_valid(m.as_str())
                && (!c[1].is_empty() || has_context(text, m.start(), AADHAAR_CTX))
            {
                out.push(nid(m.start(), m.end()));
            }
        }
        for m in CPF.find_iter(text) {
            let formatted = m.as_str().contains('.');
            if cpf_valid(m.as_str()) && (formatted || has_context(text, m.start(), &["cpf"])) {
                out.push(nid(m.start(), m.end()));
            }
        }
        const BSN_CTX: &[&str] = &["bsn", "burgerservice", "sofi"];
        out.extend(
            BSN.find_iter(text)
                .filter(|m| bsn_valid(m.as_str()) && has_context(text, m.start(), BSN_CTX))
                .map(|m| nid(m.start(), m.end())),
        );
        out.extend(
            NIF.find_iter(text)
                .filter(|m| nif_valid(m.as_str()))
                .map(|m| nid(m.start(), m.end())),
        );
        out.extend(
            CODICE_FISCALE
                .find_iter(text)
                .filter(|m| codice_fiscale_valid(m.as_str()))
                .map(|m| nid(m.start(), m.end())),
        );
        out.extend(
            INSEE
                .find_iter(text)
                .filter(|m| insee_valid(m.as_str()))
                .map(|m| nid(m.start(), m.end())),
        );
        const STEUER_CTX: &[&str] = &["steuer", "idnr", "tax id", "identifikationsnummer"];
        out.extend(
            STEUER_ID
                .find_iter(text)
                .filter(|m| steuer_id_valid(m.as_str()) && has_context(text, m.start(), STEUER_CTX))
                .map(|m| nid(m.start(), m.end())),
        );
        // Bare nine-digit runs starting with 9 are as often reference numbers ("999999999",
        // "987654321") as ITINs, so the unformatted form needs a nearby keyword.
        const ITIN_CTX: &[&str] = &["itin", "taxpayer", "tax id"];
        out.extend(
            ITIN.captures_iter(text)
                .filter(|c| c[1] == c[2])
                .filter(|c| {
                    !c[1].is_empty() || has_context(text, c.get(0).unwrap().start(), ITIN_CTX)
                })
                .map(|c| c.get(0).unwrap())
                .map(|m| nid(m.start(), m.end())),
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
                        out.push(nid(m.start(), m.end()));
                    }
                }
            };
        checked(&NHS, nhs_valid, &["nhs"], &mut out);
        checked(&TFN, tfn_valid, &["tfn", "tax file"], &mut out);
        checked(&ABN, abn_valid, &["abn", "business number"], &mut out);
        Self::labelled(text, &mut out);
        out.sort_by_key(|f| (f.start, f.end));
        out.dedup_by_key(|f| (f.start, f.end));
        out
    }
}

/// True for template numbers that identify nobody: a run of consecutive digits
/// ("123456789", "987654321", "12345678901234567890") or one repeated digit ("999999999").
pub(crate) fn dummy_number(v: &str) -> bool {
    let d: Vec<u8> = v
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|b| b - b'0')
        .collect();
    if d.len() < 6 {
        return false;
    }
    let step = |k: u8| d.windows(2).all(|w| w[1] == (w[0] + k) % 10);
    step(0) || step(1) || step(9)
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

/// IPv6 candidate check: parses, is not loopback/unspecified, and has enough structure
/// that times (`12:30:45`) and `a::b` paths are not taken.
fn ipv6_ok(s: &str) -> bool {
    let Ok(ip) = s.parse::<std::net::Ipv6Addr>() else {
        return false;
    };
    if ip.is_loopback() || ip.is_unspecified() {
        return false;
    }
    let groups: Vec<&str> = s.split(':').filter(|g| !g.is_empty()).collect();
    groups.len() >= 3 || (s.contains("::") && groups.iter().any(|g| g.len() >= 3))
}

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
                let before = text[..m.start()].trim_end().to_ascii_lowercase();
                !s.starts_with("127.")
                    && s != "0.0.0.0"
                    && s != "255.255.255.255"
                    && !matches!(prev, Some('v' | 'V' | '.' | '-' | '_'))
                    && !prev.is_some_and(|c| c.is_alphanumeric())
                    && !before.ends_with("version")
                    && !before.ends_with("ver")
                    && !before.ends_with("ver.")
                    && !(next == Some('.')
                        && text[m.end() + 1..].starts_with(|c: char| c.is_ascii_digit()))
            })
            .map(|m| Finding::new(m.start(), m.end(), Kind::Ip, "ip"))
            .collect();
        for m in IPV6.find_iter(text) {
            let prev = text[..m.start()].chars().next_back();
            let next = text[m.end()..].chars().next();
            if prev.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
                || next.is_some_and(|c| c.is_alphanumeric() || c == '_')
            {
                continue;
            }
            let s = m.as_str().trim_end_matches(':');
            let s = if s.ends_with(':') && !s.ends_with("::") {
                &s[..s.len() - 1]
            } else {
                s
            };
            if ipv6_ok(s) {
                out.push(Finding::new(m.start(), m.start() + s.len(), Kind::Ip, "ip"));
            }
        }
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
        let mut url_hosts = vec![];
        for c in URL_HOST.captures_iter(text) {
            let m = c.get(1).unwrap();
            url_hosts.push((m.start(), m.end()));
            if self.sensitive(m.as_str()) && !xml_namespace(text, c.get(0).unwrap().start()) {
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
                || url_hosts.iter().any(|&(a, b)| a < m.end() && m.start() < b)
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

/// True if the URL starting at `pos` is the value of an XML namespace or schema attribute
/// (`xmlns="..."`, `xsi:schemaLocation="... ..."`, `scheme="..."`): an identifier for a
/// vocabulary (FpML, XBRL, ...), not a site anyone visits.
fn xml_namespace(text: &str, pos: usize) -> bool {
    let mut from = pos.saturating_sub(400);
    while !text.is_char_boundary(from) {
        from += 1;
    }
    let w = &text[from..pos];
    let line = w.rfind('\n').map_or(w, |i| &w[i + 1..]);
    let Some(q) = line.rfind(['"', '\'']) else {
        return false;
    };
    let attr = line[..q]
        .trim_end()
        .strip_suffix('=')
        .unwrap_or("")
        .trim_end()
        .rsplit(|c: char| c.is_whitespace() || c == '<')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    attr.starts_with("xmlns") || attr.ends_with("schemalocation") || attr == "scheme"
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

/// Values that are clearly not a real credential.
fn placeholder(v: &str) -> bool {
    let lower = v.to_ascii_lowercase();
    PLACEHOLDERS.iter().any(|p| lower.contains(p))
        || matches!(
            lower.as_str(),
            "password"
                | "passwd"
                | "secret"
                | "token"
                | "null"
                | "none"
                | "true"
                | "false"
                | "string"
                | "str"
                | "required"
                | "optional"
                | "undefined"
                | "empty"
                | "hidden"
                | "pass"
                | "test"
                | "admin"
                | "root"
                | "bytes"
                | "int"
                | "bool"
                | "any"
        )
        || v.chars()
            .all(|c| c == '*' || c == 'x' || c == 'X' || c == '.' || c == '•')
}

/// Shannon entropy in bits per character.
fn entropy(s: &str) -> f64 {
    let mut counts = std::collections::HashMap::new();
    for c in s.chars() {
        *counts.entry(c).or_insert(0usize) += 1;
    }
    let n = s.chars().count() as f64;
    counts
        .values()
        .map(|&c| {
            let p = c as f64 / n;
            -p * p.log2()
        })
        .sum()
}

/// True if an unquoted value is a code reference (`os.getenv(..)`, `config.password`,
/// `secret_var`, `$VAR`) rather than a literal.
fn code_reference(v: &str) -> bool {
    v.starts_with('$')
        || v.contains('(')
        || v.contains("::")
        || (v.contains('.')
            && v.split('.')
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
            && v.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && !v.chars().any(|c| c.is_ascii_digit()))
        || (v.contains('_') && v.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
}

impl SecretDetector {
    fn assignments(text: &str, out: &mut Vec<Finding>) {
        for c in SECRET_ASSIGN.captures_iter(text) {
            let key = c.get(1).unwrap().as_str().to_ascii_lowercase();
            let quoted = !c.get(2).unwrap().as_str().is_empty();
            let m = c.get(3).unwrap();
            let v = m.as_str().trim_end_matches(['.', ']', '}']);
            if v.is_empty() || placeholder(v) || (!quoted && code_reference(v)) {
                continue;
            }
            let password_like = ["passw", "pwd", "pass", "passphrase", "secret"]
                .iter()
                .any(|k| key.ends_with(k) || key.contains("passw"));
            let classes = [
                v.chars().any(|c| c.is_ascii_lowercase()),
                v.chars().any(|c| c.is_ascii_uppercase()),
                v.chars().any(|c| c.is_ascii_digit()),
                v.chars().any(|c| !c.is_ascii_alphanumeric()),
            ]
            .iter()
            .filter(|b| **b)
            .count();
            let ok = if password_like {
                v.chars().count() >= 6 && (classes >= 2 || quoted)
            } else {
                // Keys and tokens: long, mixed and random-looking.
                v.len() >= 16
                    && classes >= 2
                    && entropy(v) >= 3.0
                    && !v
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '-' || c == '.')
            };
            if ok {
                out.push(Finding::new(
                    m.start(),
                    m.start() + v.len(),
                    Kind::Secret,
                    "secret",
                ));
            }
        }
    }

    /// Credentials named in prose ("la mia password è X", "API key: X"). The value must look
    /// generated: mixed character classes, not a word or a template slot.
    fn prose(text: &str, out: &mut Vec<Finding>) {
        for c in PROSE_SECRET.captures_iter(text) {
            let key = c.get(1).unwrap().as_str().to_lowercase();
            let open = c.get(2).unwrap().as_str();
            let m = c.get(3).unwrap();
            let mut v = m.as_str().trim_end_matches(['.', ',', ';', ':', '!', '?']);
            // "(Yml6Ig(h(3PW%p2sE))": the closing brackets belong to the prose.
            if matches!(open, "(" | "[") {
                v = v.trim_end_matches([')', ']']);
            }
            // A bracketed or quoted value is a literal, even with "(" or "$" in it.
            let quoted = !open.is_empty();
            // Generated passwords contain "(" too; only a call shape ("getpass()") is code.
            let code = if v.contains('(') {
                v.ends_with(')')
                    && v.split('(').next().is_some_and(|f| {
                        !f.is_empty()
                            && f.chars()
                                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
                    })
            } else {
                code_reference(v)
            };
            if v.chars().count() < 6 || placeholder(v) || (!quoted && code) {
                continue;
            }
            let classes = [
                v.chars().any(|c| c.is_lowercase()),
                v.chars().any(|c| c.is_uppercase()),
                v.chars().any(|c| c.is_ascii_digit()),
                v.chars().any(|c| !c.is_alphanumeric()),
            ]
            .iter()
            .filter(|b| **b)
            .count();
            let digit = v.chars().any(|c| c.is_ascii_digit());
            let ok = if key.contains("key")
                || key.contains("nyckel")
                || key.contains("token")
                || key.contains("schlüssel")
                || key.contains("sleutel")
                || key.contains("clé")
                || key.contains("clave")
                || key.contains("chiave")
            {
                v.len() >= 8 && classes >= 2 && digit
            } else {
                classes >= 3 || (classes == 2 && digit && !v.chars().all(|c| c.is_alphanumeric()))
            };
            if ok {
                out.push(Finding::new(
                    m.start(),
                    m.start() + v.len(),
                    Kind::Secret,
                    "secret",
                ));
            }
        }
    }

    /// Secret-named XML elements and long random tokens ("SKx2d85AEMNWR7OuvwtHXPWCD1nfqu7Ksf").
    fn tokens(text: &str, out: &mut Vec<Finding>) {
        if text.contains("</") {
            for c in XML_SECRET.captures_iter(text) {
                let m = c.get(2).unwrap();
                if !placeholder(m.as_str()) {
                    out.push(Finding::new(m.start(), m.end(), Kind::Secret, "secret"));
                }
            }
        }
        for m in RANDOM_TOKEN.find_iter(text) {
            // A piece of a longer token (a JWT segment, a path, a URL) is judged as a whole
            // by the other rules.
            let glued = |c: Option<char>| c.is_some_and(|c| ".-_/+=:@".contains(c));
            if glued(text[..m.start()].chars().next_back()) || glued(text[m.end()..].chars().next())
            {
                continue;
            }
            let v = m.as_str();
            let count = |f: fn(&char) -> bool| v.chars().filter(f).count();
            let (upper, lower, digits) = (
                count(char::is_ascii_uppercase),
                count(char::is_ascii_lowercase),
                count(char::is_ascii_digit),
            );
            // Words glued together ("MyCompanyAnnualReport2023") have long lowercase runs.
            let longest_lower = v
                .split(|c: char| !c.is_ascii_lowercase())
                .map(str::len)
                .max()
                .unwrap_or(0);
            if upper >= 3 && lower >= 3 && digits >= 3 && longest_lower <= 6 && entropy(v) >= 3.8 {
                out.push(Finding::new(m.start(), m.end(), Kind::Secret, "secret"));
            }
        }
    }

    fn passwords_in_context(text: &str, out: &mut Vec<Finding>) {
        for c in AUTH_TUPLE.captures_iter(text) {
            let pw = c.get(2).unwrap();
            if !placeholder(pw.as_str()) && pw.as_str().chars().count() >= 3 {
                out.push(Finding::new(pw.start(), pw.end(), Kind::Secret, "secret"));
            }
        }
        for c in URL_USERINFO.captures_iter(text) {
            if let Some(pw) = c.get(2) {
                if !placeholder(pw.as_str()) && pw.as_str().chars().count() >= 3 {
                    out.push(Finding::new(pw.start(), pw.end(), Kind::Secret, "secret"));
                }
            }
        }
        for c in USER_FLAG.captures_iter(text) {
            if let Some(pw) = c.get(2) {
                if !placeholder(pw.as_str()) && pw.as_str().chars().count() >= 3 {
                    out.push(Finding::new(pw.start(), pw.end(), Kind::Secret, "secret"));
                }
            }
        }
        for m in JWT.find_iter(text) {
            let header = m.as_str().split('.').next().unwrap_or("");
            use base64::Engine;
            let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(header.trim_end_matches('='))
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or_default();
            if decoded.starts_with('{')
                && (decoded.contains("\"alg\"") || decoded.contains("\"typ\""))
            {
                out.push(Finding::new(m.start(), m.end(), Kind::Secret, "secret"));
            }
        }
    }
}

impl Detector for SecretDetector {
    fn name(&self) -> &'static str {
        "secret"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = spans(&SECRET_KNOWN, text, Kind::Secret, "secret");
        out.extend(spans(&PRIVATE_KEY, text, Kind::Secret, "secret"));
        out.extend(group_spans(&BEARER, text, Kind::Secret, "secret"));
        Self::assignments(text, &mut out);
        Self::prose(text, &mut out);
        Self::tokens(text, &mut out);
        Self::passwords_in_context(text, &mut out);
        out.sort_by_key(|f| (f.start, f.end));
        out.dedup_by_key(|f| (f.start, f.end));
        out
    }
}

/// Usernames from context: `ssh user@host`, `user@host` where the host is internal or has
/// no domain, `username=`/`"user": "..."`/`login:`, `-u user` for login commands, basic-auth
/// tuples and URL userinfo (`postgres://user:pw@db`). Hosts of login targets are masked too.
pub struct UsernameDetector {
    hosts: HostnameDetector,
}

impl UsernameDetector {
    /// Build with the hostname policy from `cfg`.
    pub fn new(cfg: &ShadowConfig) -> Self {
        Self {
            hosts: HostnameDetector::new(cfg),
        }
    }
}

/// Built-in, service and placeholder account names that identify nobody.
pub(crate) fn generic_user(u: &str) -> bool {
    let l = u.to_ascii_lowercase();
    matches!(
        l.as_str(),
        "root"
            | "admin"
            | "administrator"
            | "user"
            | "username"
            | "guest"
            | "postgres"
            | "mysql"
            | "ubuntu"
            | "ec2-user"
            | "centos"
            | "debian"
            | "fedora"
            | "git"
            | "www-data"
            | "nobody"
            | "test"
            | "demo"
            | "example"
            | "me"
            | "you"
            | "default"
            | "null"
            | "none"
            | "true"
            | "false"
            | "sa"
            | "anonymous"
            | "pi"
            | "vagrant"
            | "docker"
            | "jenkins"
            | "runner"
            | "deploy"
            | "app"
            | "service"
            | "system"
            | "daemon"
            | "oracle"
            | "redis"
            | "elastic"
            | "kibana"
            | "grafana"
            | "string"
            | "str"
            | "name"
            | "login"
            | "id"
            | "uid"
            | "env"
            | "origin"
            | "master"
            | "main"
            | "required"
            | "optional"
            | "foo"
            | "bar"
            | "alice"
            | "bob"
            | "username_here"
            | "your_username"
            | "changeme"
    ) || l.starts_with('$')
        || l.contains("example")
        || l.contains("your")
}

impl Detector for UsernameDetector {
    fn name(&self) -> &'static str {
        "username"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = vec![];
        let user = |m: regex::Match<'_>, out: &mut Vec<Finding>| {
            if !generic_user(m.as_str()) {
                out.push(Finding::new(m.start(), m.end(), Kind::Username, "username"));
            }
        };
        if text.contains('@') {
            for c in USER_AT_HOST.captures_iter(text) {
                let (u, h) = (c.get(1).unwrap(), c.get(2).unwrap());
                let host = h.as_str().to_ascii_lowercase();
                let tld = host.rsplit('.').next().unwrap_or("");
                let next = text[h.end()..].chars().next();
                let login = LOGIN_CMD.is_match(line_before(text, u.start()))
                    || !host.contains('.')
                    || names::INTERNAL_TLDS.contains(&tld)
                    || (next == Some(':')
                        && text[h.end() + 1..]
                            .chars()
                            .next()
                            .is_some_and(|c| c == '/' || c == '~' || c.is_alphanumeric()));
                if !login {
                    continue;
                }
                user(u, &mut out);
                if host.parse::<std::net::IpAddr>().is_err() && self.hosts.sensitive(&host) {
                    out.push(Finding::new(h.start(), h.end(), Kind::Hostname, "username"));
                }
            }
            for c in URL_USERINFO.captures_iter(text) {
                user(c.get(1).unwrap(), &mut out);
            }
        }
        for c in USER_ASSIGN.captures_iter(text) {
            let v = c.get(5).unwrap();
            if let Some(label) = c.get(1) {
                let sep = c.get(3).map_or("", |m| m.as_str());
                let quoted = !c.get(4).map_or("", |m| m.as_str()).is_empty();
                let l = label.as_str();
                let plain_user = l.eq_ignore_ascii_case("user") || l.eq_ignore_ascii_case("uid");
                // "User: How do I..." in a chat transcript is a role, not an account.
                if sep == ":" && !quoted && plain_user {
                    let s = v.as_str();
                    let shaped = s
                        .chars()
                        .any(|c| c.is_ascii_digit() || c == '.' || c == '_');
                    if !shaped || l == "User" {
                        continue;
                    }
                }
                if l.eq_ignore_ascii_case("uid") && v.as_str().chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
            }
            user(v, &mut out);
        }
        for c in USER_FLAG.captures_iter(text) {
            user(c.get(1).unwrap(), &mut out);
        }
        // In prose the value must look like a handle (digit, "_" or "."), unless quoted.
        for c in USER_PROSE.captures_iter(text) {
            let quoted = !c.get(1).unwrap().as_str().is_empty();
            let v = c.get(2).unwrap();
            let s = v.as_str();
            let handle = s
                .chars()
                .any(|c| c.is_ascii_digit() || c == '_' || c == '.')
                && s.chars().any(|c| c.is_ascii_lowercase())
                && !s.ends_with('.');
            let next = text[v.end()..].chars().next();
            if (handle || quoted) && next != Some('@') && !looks_like_date(s) {
                user(v, &mut out);
            }
        }
        for c in NAME_HANDLE.captures_iter(text) {
            let m = c.get(0).unwrap();
            let prev = text[..m.start()].chars().next_back();
            let next = text[m.end()..].chars().next();
            if matches!(prev, Some('@' | '.' | '/' | '-' | '#'))
                || matches!(next, Some('@' | '.' | '/' | '-'))
            {
                continue;
            }
            let stem = c.get(1).unwrap().as_str();
            let mut name = stem.to_string();
            name[..1].make_ascii_uppercase();
            if names::is_first_name(&name) && stem.len() >= 4 {
                user(m, &mut out);
            }
        }
        for c in AUTH_TUPLE.captures_iter(text) {
            user(c.get(1).unwrap(), &mut out);
        }
        out.sort_by_key(|f| (f.start, f.end));
        out.dedup_by_key(|f| (f.start, f.end));
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

/// Person names from rule cues (titles, greetings, `Name <email>`, known given names).
pub struct PersonDetector;
impl Detector for PersonDetector {
    fn name(&self) -> &'static str {
        "person"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        names::detect_people(text)
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
    if t.org {
        v.push(Box::new(OrgDetector));
    }
    if t.username {
        v.push(Box::new(UsernameDetector::new(cfg)));
    }
    if t.date_of_birth {
        v.push(Box::new(BirthDateDetector));
    }
    if t.fields {
        v.push(Box::new(FieldDetector::new(t)));
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
    fn national_ids_by_country() {
        let d = NationalIdDetector;
        // US SSN: dashed form alone; bare/spaced need context; invalid areas rejected.
        assert_eq!(kinds(&d, "ssn 123-45-6789"), vec!["123-45-6789"]);
        assert_eq!(kinds(&d, "Social Security 123456789"), vec!["123456789"]);
        assert!(kinds(&d, "order 123456789").is_empty());
        assert!(kinds(&d, "id 666-45-6789 or 000-12-3456").is_empty());
        // UK NINO incl. spaced; invalid prefixes need a label.
        assert_eq!(kinds(&d, "NI AB 12 34 56 C ok"), vec!["AB 12 34 56 C"]);
        assert_eq!(kinds(&d, "ref JG103759A."), vec!["JG103759A"]);
        assert!(kinds(&d, "ref ZZ123456A.").is_empty());
        assert_eq!(kinds(&d, "UK NINO: ZZ 763112 T\n"), vec!["ZZ 763112 T"]);
        // India Aadhaar (Verhoeff).
        assert_eq!(kinds(&d, "uid 2341 2341 2346"), vec!["2341 2341 2346"]);
        assert!(kinds(&d, "uid 2341 2341 2347").is_empty());
        assert_eq!(kinds(&d, "Aadhaar 234123412346"), vec!["234123412346"]);
        // Brazil CPF.
        assert_eq!(kinds(&d, "CPF 529.982.247-25"), vec!["529.982.247-25"]);
        assert!(kinds(&d, "x 529.982.247-26").is_empty());
        // Netherlands BSN needs context.
        assert_eq!(kinds(&d, "BSN 111222333"), vec!["111222333"]);
        assert!(kinds(&d, "tracking 111222333").is_empty());
        // Spain NIF / NIE.
        assert_eq!(kinds(&d, "DNI 12345678Z."), vec!["12345678Z"]);
        assert_eq!(kinds(&d, "NIE X1234567L"), vec!["X1234567L"]);
        assert!(kinds(&d, "code 12345678A").is_empty());
        // Italy codice fiscale.
        assert_eq!(kinds(&d, "CF RSSMRA85T10A562S"), vec!["RSSMRA85T10A562S"]);
        assert!(kinds(&d, "x RSSMRA85T10A562T").is_empty());
        // France INSEE with spaces.
        assert_eq!(
            kinds(&d, "NIR 1 84 12 76 451 089 46"),
            vec!["1 84 12 76 451 089 46"]
        );
        // Germany Steuer-ID needs context.
        assert_eq!(kinds(&d, "Steuer-ID: 86095742719"), vec!["86095742719"]);
        assert!(kinds(&d, "call 86095742719").is_empty());
        // Label context takes values whose format is unknown.
        assert_eq!(kinds(&d, "Passport No. C01X00T47\n"), vec!["C01X00T47"]);
        assert_eq!(kinds(&d, "SSN: 123-45-6789 DOB"), vec!["123-45-6789"]);
        assert!(kinds(&d, "the passport was renewed in 2020").is_empty());
        assert!(kinds(&d, "PAN card holder").is_empty());
    }

    #[test]
    fn phones_international_and_national() {
        let d = PhoneDetector;
        let one = |t: &str| kinds(&d, t);
        assert_eq!(
            one("call +1 (555) 123-4567 x123 now"),
            vec!["+1 (555) 123-4567 x123"]
        );
        assert_eq!(one("or (555) 123-4567."), vec!["(555) 123-4567"]);
        assert_eq!(one("UK +44 (0)20 7946 0958."), vec!["+44 (0)20 7946 0958"]);
        assert_eq!(one("Büro 030 12345678"), vec!["030 12345678"]);
        assert_eq!(one("DE +49 (0)30 1234567"), vec!["+49 (0)30 1234567"]);
        assert_eq!(one("au 04 27 62 49 86"), vec!["04 27 62 49 86"]);
        assert_eq!(one("IN +91 98765 43210"), vec!["+91 98765 43210"]);
        assert_eq!(one("BR (051) 3785-7359"), vec!["(051) 3785-7359"]);
        assert_eq!(one("JP 090-1234-5678"), vec!["090-1234-5678"]);
        assert_eq!(one("US 555-123-4567"), vec!["555-123-4567"]);
        assert_eq!(one("+31.20.123.4567"), vec!["+31.20.123.4567"]);
        assert_eq!(one("0321215112 is mine"), vec!["0321215112"]);
        // Bare digit runs need a phone word.
        assert_eq!(one("prefers calls on 8199263641."), vec!["8199263641"]);
        assert_eq!(one("Tel. 351338697"), vec!["351338697"]);
        assert!(one("invoice 8199263641").is_empty());
        assert!(one("call about invoice 8199263641").is_empty());
        // Not phones: prices, years, dates, IPs, amounts, versions, IDs.
        assert!(one("costs $1234567 or 1 234 567 EUR").is_empty());
        assert!(one("from 2019 2020 2021 to 2024").is_empty());
        assert!(one("on 2024-01-15 and 15/01/2024 or 15.01.2024").is_empty());
        assert!(one("host 192.168.100.96").is_empty());
        assert!(one("ticket #12345678 and order 98765432").is_empty());
        assert!(one("version 1.2.3.4567").is_empty());
    }

    #[test]
    fn ips_v4_and_v6() {
        let d = IpDetector;
        assert_eq!(
            kinds(&d, "hosts 10.0.0.5, 8.8.8.8 and 2001:db8::8a2e:370:7334."),
            vec!["10.0.0.5", "8.8.8.8", "2001:db8::8a2e:370:7334"]
        );
        assert_eq!(
            kinds(&d, "full 2001:0db8:85a3:0000:0000:8a2e:0370:7334 ok"),
            vec!["2001:0db8:85a3:0000:0000:8a2e:0370:7334"]
        );
        assert_eq!(
            kinds(&d, "link fe80::1ff:fe23:4567:890a%eth0"),
            vec!["fe80::1ff:fe23:4567:890a"]
        );
        assert!(kinds(&d, "at 12:30:45 and ::1 and :: and std::vec").is_empty());
        assert!(kinds(&d, "mac 3c:22:fb:12:ab:9e").is_empty());
        assert!(kinds(&d, "version 1.2.3.4 and v10.0.0.1").is_empty());
        assert!(kinds(&d, "300.1.1.1").is_empty());
    }

    #[test]
    fn secrets_in_context() {
        let d = SecretDetector;
        let one = |t: &str| kinds(&d, t);
        assert_eq!(one(r#"{"password": "s3cr3t-Pass"}"#), vec!["s3cr3t-Pass"]);
        assert_eq!(one("DB_PASSWORD=hunter22!"), vec!["hunter22!"]);
        assert_eq!(one("pwd: Tr0ub4dor&3"), vec!["Tr0ub4dor&3"]);
        assert_eq!(
            one(r#"requests.get(u, auth=("jdoe", "iqQ6ePBsHhHxQO"))"#),
            vec!["iqQ6ePBsHhHxQO"]
        );
        assert_eq!(
            one("postgres://app:Sup3rS3cret@db.internal/x"),
            vec!["Sup3rS3cret"]
        );
        assert_eq!(
            one("STRIPE_SECRET_KEY=Zk3jR8vQ2mN5pL9tW4xY7aB1"),
            vec!["Zk3jR8vQ2mN5pL9tW4xY7aB1"]
        );
        assert_eq!(
            one("token eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U"),
            vec!["eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U"]
        );
        assert_eq!(
            one("slack xoxb-123456789012-123456789012-AbCdEfGhIjKlMnOpQrStUvWx"),
            vec!["xoxb-123456789012-123456789012-AbCdEfGhIjKlMnOpQrStUvWx"]
        );
        // Not secrets: references, placeholders, low-entropy values, prose.
        assert!(one("password = os.getenv(\"DB_PASSWORD\")").is_empty());
        assert!(one("password: <your password>").is_empty());
        assert!(one("primary_key = id_column_name").is_empty());
        assert!(one("API_KEY=xxxxxxxxxxxxxxxx").is_empty());
        assert!(one("the token is valid for one hour").is_empty());
        assert!(one("eyJub3QganNvbg.abcdefghijk.lmnopqrstuv").is_empty());
    }

    #[test]
    fn usernames_in_context() {
        let d = UsernameDetector::new(&ShadowConfig::from_dir("/tmp"));
        let found = |t: &str| {
            d.detect(t)
                .iter()
                .map(|f| (t[f.start..f.end].to_string(), f.kind))
                .collect::<Vec<_>>()
        };
        let users = |t: &str| {
            found(t)
                .into_iter()
                .filter(|(_, k)| *k == Kind::Username)
                .map(|(v, _)| v)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            found("$ ssh jdoe@db-1.internal.acme.io"),
            vec![
                ("jdoe".to_string(), Kind::Username),
                ("db-1.internal.acme.io".to_string(), Kind::Hostname)
            ]
        );
        assert_eq!(users("scp report.pdf mkumar@build01:/tmp/"), vec!["mkumar"]);
        assert_eq!(
            users("login as svc_backup@bastion please"),
            vec!["svc_backup"]
        );
        assert_eq!(
            users("url = \"postgres://jsmith:pw123456@db/app\""),
            vec!["jsmith"]
        );
        assert_eq!(users("username=akemi48&x=1"), vec!["akemi48"]);
        assert_eq!(users(r#"{"user": "marit04"}"#), vec!["marit04"]);
        assert_eq!(users("mysql -u reporting_ro -p"), vec!["reporting_ro"]);
        assert_eq!(users(r#"auth=("kikeda", "pw-123456")"#), vec!["kikeda"]);
        // Emails, generic accounts, chat roles and unrelated -u flags are not usernames.
        assert!(users("mail jane@acme.com today").is_empty());
        assert!(users("ssh root@10.0.0.1 and git@github.com").is_empty());
        assert!(users("User: How do I reset it?").is_empty());
        assert!(users("git push -u origin main").is_empty());
    }

    #[test]
    fn people_from_cues() {
        let d = PersonDetector;
        let one = |t: &str| kinds(&d, t);
        assert_eq!(one("Dear Noëlle Mathurin,"), vec!["Noëlle Mathurin"]);
        assert_eq!(
            one("Ask John Smith and Dr. Miller"),
            vec!["John Smith", "Miller"]
        );
        assert_eq!(one("and cc René De Oliveira."), vec!["René De Oliveira"]);
        assert_eq!(
            one("customer Bo Zwijsen-Chevresson <b@x.nl>"),
            vec!["Bo Zwijsen-Chevresson"]
        );
        assert_eq!(
            one("Marguerite de la Torres signed"),
            vec!["Marguerite de la Torres"]
        );
        assert_eq!(
            one("transfer 500 EUR to Kayla Love, IBAN"),
            vec!["Kayla Love"]
        );
        assert_eq!(one("our client 松本 康弘 lives"), vec!["松本 康弘"]);
        // Not people: headings, products, places, sentence-initial common words.
        assert!(one("Compare Microsoft Azure and Google Cloud in New York.").is_empty());
        assert!(one("Will Smith Street be closed?").is_empty());
        assert!(one("Grace Period Policy").is_empty());
        assert!(one("Acme Widgets GmbH signed").is_empty());
    }

    #[test]
    fn people_in_documents() {
        let d = PersonDetector;
        let one = |t: &str| kinds(&d, t);
        // Middle initials, also before a surname that is a common word.
        assert_eq!(
            one("Name: Melanie J. Hopkins\n"),
            vec!["Melanie J. Hopkins"]
        );
        assert_eq!(one("signed by Tomas K. West."), vec!["Tomas K. West"]);
        assert!(one("See Section A. Overview of the plan.").is_empty());
        assert!(one("Serie B. La Serie A comprende").is_empty());
        // Greetings and pronouns in the supported languages.
        assert_eq!(one("Hej Fenna Doorwerth,\n"), vec!["Fenna Doorwerth"]);
        assert_eq!(one("Geachte Lieke Hoogstraten,"), vec!["Lieke Hoogstraten"]);
        assert_eq!(
            one("Io, Carlotta Bramante-Vesco, confermo"),
            vec!["Carlotta Bramante-Vesco"]
        );
        // Described by what follows: residence, a chat turn, a vocative.
        assert_eq!(
            one("and Sigrun Kallvik, a resident of Umeå"),
            vec!["Sigrun Kallvik"]
        );
        assert_eq!(one("Thank you for your patience, Rachel."), vec!["Rachel"]);
        assert_eq!(one("Astrid: the printer is offline"), vec!["Astrid"]);
        assert_eq!(one("Monsieur Lucien Fabre arrive"), vec!["Lucien Fabre"]);
        // Upper-case names in records need an initial or a whole field.
        assert_eq!(one(":59:/KATE N. WILKES\n"), vec!["KATE N. WILKES"]);
        assert_eq!(one("NAD+BY+ERIC ZANDER+12"), vec!["ERIC ZANDER"]);
        assert_eq!(one("NAD+SH+Tomas Wikberg++4"), vec!["Tomas Wikberg"]);
        assert!(one("THE GRACE PERIOD ENDS").is_empty());
        // Not people: headings after a capitalised role word, accounts, word-like names.
        assert!(one("2.0 Customer Due Diligence\n").is_empty());
        assert!(one("vested in his Defined Contribution account").is_empty());
        assert!(one("Net Cash Flow from Operating Activities").is_empty());
        assert!(one("available to me. However, the cost").is_empty());
        assert!(one("**Loan Details**").is_empty());
        assert!(one("The Grace Period starts").is_empty());
    }

    #[test]
    fn cards_and_ibans_after_labels() {
        // Card numbers that fail Luhn are taken only right after a card label.
        assert_eq!(
            kinds(&CardDetector, "Card number: 4141-6246-9771-0810"),
            vec!["4141-6246-9771-0810"]
        );
        assert_eq!(
            kinds(&CardDetector, "Kreditkartennummer: 3746 0468 6998 5471"),
            vec!["3746 0468 6998 5471"]
        );
        assert!(kinds(&CardDetector, "ref 4141-6246-9771-0810").is_empty());
        assert!(kinds(&CardDetector, "Card number: 1234 5678 9012 3456").is_empty());
        // IBANs after a label: any grouping, checksum not required, known country only.
        assert_eq!(
            kinds(&IbanDetector, "IBAN: DE12 3456 7890 1234 5678 99\nBIC: X"),
            vec!["DE12 3456 7890 1234 5678 99"]
        );
        assert_eq!(
            kinds(&IbanDetector, "IBAN: IT65Q 01005 03211 000000001234"),
            vec!["IT65Q 01005 03211 000000001234"]
        );
        assert!(kinds(&IbanDetector, "IBAN: QQ12 3456 7890 1234 5678 99").is_empty());
    }

    #[test]
    fn labelled_ids_in_prose() {
        let d = NationalIdDetector;
        assert_eq!(
            kinds(&d, "Driver's License Number: F652-2022-025-5"),
            vec!["F652-2022-025-5"]
        );
        assert_eq!(
            kinds(&d, "with a passport number of G40171627 and"),
            vec!["G40171627"]
        );
        assert_eq!(
            kinds(&d, "copia del mio passaporto (n. 289959203)"),
            vec!["289959203"]
        );
        assert_eq!(kinds(&d, "Führerschein: B072RRE2I55"), vec!["B072RRE2I55"]);
        // Template numbers and bare 9xx reference numbers are not IDs.
        assert!(kinds(&d, "BSN: 123456789").is_empty());
        assert!(kinds(&d, "UNB+UNOC:3+999999999:1").is_empty());
        assert!(kinds(&d, "Destination Account Number: 987654321").is_empty());
    }

    #[test]
    fn secrets_in_prose() {
        let one = |t: &str| kinds(&SecretDetector, t);
        assert_eq!(
            one("e la mia password è uaK1YAuLT#g3."),
            vec!["uaK1YAuLT#g3"]
        );
        assert_eq!(
            one("create a secure password (Yml6Ig(h(3PW%p2sE)) now"),
            vec!["Yml6Ig(h(3PW%p2sE"]
        );
        assert_eq!(
            one("- Passwort: cr0KoBh(A&(%ICBC"),
            vec!["cr0KoBh(A&(%ICBC"]
        );
        assert_eq!(one("**Password:** &H3EDLNgsk2n"), vec!["&H3EDLNgsk2n"]);
        assert_eq!(one("and API key: PSCG6TK5. Please"), vec!["PSCG6TK5"]);
        assert_eq!(
            one("API Key for Secure Communication: SK9pC64OLfLeVwtkcQdFB2gv"),
            vec!["SK9pC64OLfLeVwtkcQdFB2gv"]
        );
        assert_eq!(
            one("<password>i$153)C^&G6qakFzWp</password>"),
            vec!["i$153)C^&G6qakFzWp"]
        );
        assert_eq!(
            one("Verzekeringsnummer: SKx2d85AEMNWR7OuvwtHXPWCD1nfqu7Ksf"),
            vec!["SKx2d85AEMNWR7OuvwtHXPWCD1nfqu7Ksf"]
        );
        assert_eq!(
            one("key SG.ab12cd34ef56gh78ij90kl.mn12op34qr56st78uv90wx12yz34"),
            vec!["SG.ab12cd34ef56gh78ij90kl.mn12op34qr56st78uv90wx12yz34"]
        );
        // Words and template text are not secrets.
        assert!(one("The new password must be changed by March").is_empty());
        assert!(one("your password is required").is_empty());
        assert!(one("file MyCompanyAnnualReport2023Final.pdf").is_empty());
    }

    #[test]
    fn usernames_in_prose() {
        let users = |t: &str| kinds(&UsernameDetector::new(&ShadowConfig::default()), t);
        assert_eq!(users("your IP and user name colin64."), vec!["colin64"]);
        assert_eq!(
            users("Username 'kestrelwing' was used"),
            vec!["kestrelwing"]
        );
        assert_eq!(
            users("Authenticated User: hernan9044\n"),
            vec!["hernan9044"]
        );
        assert_eq!(users("|57=rebecca_282|58=Share"), vec!["rebecca_282"]);
        assert!(users("the user is locked out").is_empty());
        assert!(users("uses html5 and sha256 with utf8").is_empty());
    }

    #[test]
    fn phones_are_not_amounts_or_ids() {
        let one = |t: &str| kinds(&PhoneDetector, t);
        assert!(one("de 0.01234567 BTC, et").is_empty());
        assert!(one("Partita IVA: 04204580017\n").is_empty());
        assert!(one("Credit (Dr) Debit (Cr)\n0.00 125000.00\n").is_empty());
        assert!(one("Prospektusnummer: 221-2345/2023\n").is_empty());
        assert_eq!(one("Telefon: 0123 456789\n"), vec!["0123 456789"]);
    }

    #[test]
    fn xml_namespaces_are_not_hosts() {
        let d = HostnameDetector::new(&ShadowConfig::default());
        let t = r#"<FpML xmlns="http://www.vocab-registry.net/FpML-5" xsi:schemaLocation="http://www.vocab-registry.net/a http://www.vocab-registry.net/b.xsd"> see https://portal.vocab-registry.net/x"#;
        assert_eq!(kinds(&d, t), vec!["portal.vocab-registry.net"]);
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
