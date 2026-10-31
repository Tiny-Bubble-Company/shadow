//! Checksums used to validate (and generate) cards, IBANs, national IDs and wallet addresses.

/// Luhn check over the digits of `s` (non-digits ignored).
pub fn luhn_valid(s: &str) -> bool {
    let digits: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() < 12 {
        return false;
    }
    luhn_sum(&digits) % 10 == 0
}

fn luhn_sum(digits: &[u32]) -> u32 {
    digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            if i % 2 == 1 {
                let x = d * 2;
                if x > 9 {
                    x - 9
                } else {
                    x
                }
            } else {
                d
            }
        })
        .sum()
}

/// Check digit that makes `payload ++ check` Luhn-valid.
pub fn luhn_check_digit(payload: &[u32]) -> u32 {
    let mut with = payload.to_vec();
    with.push(0);
    (10 - luhn_sum(&with) % 10) % 10
}

/// Expected IBAN lengths for common countries (unknown countries accept 15..=34).
fn iban_len(cc: &str) -> Option<usize> {
    Some(match cc {
        "DE" => 22,
        "GB" => 22,
        "FR" => 27,
        "ES" => 24,
        "IT" => 27,
        "NL" => 18,
        "BE" => 16,
        "CH" => 21,
        "AT" => 20,
        "IE" => 22,
        "PT" => 25,
        "SE" => 24,
        "NO" => 15,
        "DK" => 18,
        "FI" => 18,
        "PL" => 28,
        "CZ" => 24,
        "LU" => 20,
        "GR" => 27,
        "AE" => 23,
        "SA" => 24,
        "TR" => 26,
        "BR" => 29,
        "IL" => 23,
        "HU" => 28,
        "RO" => 24,
        _ => return None,
    })
}

/// Mod-97 remainder of an IBAN-style string (country+check rotated to the end).
fn mod97(rearranged: &str) -> Option<u32> {
    let mut rem: u32 = 0;
    for c in rearranged.chars() {
        let v = match c {
            '0'..='9' => c as u32 - '0' as u32,
            'A'..='Z' => c as u32 - 'A' as u32 + 10,
            _ => return None,
        };
        if v >= 10 {
            rem = (rem * 100 + v) % 97;
        } else {
            rem = (rem * 10 + v) % 97;
        }
    }
    Some(rem)
}

/// Mod-97 check only (any length 15-34), for values explicitly labelled "IBAN" whose
/// length does not match the country's registered format.
pub fn iban_checksum_ok(s: &str) -> bool {
    let compact: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    if compact.len() < 15 || compact.len() > 34 || !compact.is_ascii() {
        return false;
    }
    let rearranged = format!("{}{}", &compact[4..], &compact[..4]);
    mod97(&rearranged) == Some(1)
}

/// True if `cc` is a country code that issues IBANs.
pub fn iban_country(cc: &str) -> bool {
    iban_len(&cc.to_ascii_uppercase()).is_some()
}

/// Validate an IBAN (spaces allowed).
pub fn iban_valid(s: &str) -> bool {
    let compact: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    if compact.len() < 15 || compact.len() > 34 || !compact.is_ascii() {
        return false;
    }
    if let Some(n) = iban_len(&compact[..2]) {
        if n != compact.len() {
            return false;
        }
    }
    let rearranged = format!("{}{}", &compact[4..], &compact[..4]);
    mod97(&rearranged) == Some(1)
}

/// Compute the two IBAN check digits for `country` + `bban`.
pub fn iban_check_digits(country: &str, bban: &str) -> String {
    let rem = mod97(&format!("{bban}{country}00")).unwrap_or(0);
    format!("{:02}", 98 - rem)
}

fn digits_of(s: &str) -> Vec<u32> {
    s.chars().filter_map(|c| c.to_digit(10)).collect()
}

/// UK NHS number: 10 digits, mod-11 check digit (separators ignored).
pub fn nhs_valid(s: &str) -> bool {
    let d = digits_of(s);
    if d.len() != 10 || d.iter().all(|&x| x == d[0]) {
        return false;
    }
    let sum: u32 = d[..9].iter().zip((2..=10).rev()).map(|(x, w)| x * w).sum();
    let check = match 11 - sum % 11 {
        11 => 0,
        10 => return false,
        c => c,
    };
    check == d[9]
}

/// Australian Tax File Number (9 digits, weighted mod-11).
pub fn tfn_valid(s: &str) -> bool {
    const W: [u32; 9] = [1, 4, 3, 7, 5, 8, 6, 9, 10];
    let d = digits_of(s);
    if d.len() != 9 || d.iter().all(|&x| x == d[0]) {
        return false;
    }
    d.iter().zip(W).map(|(x, w)| x * w).sum::<u32>() % 11 == 0
}

/// Australian Business Number (11 digits, first digit minus one, weighted mod-89).
pub fn abn_valid(s: &str) -> bool {
    const W: [u32; 11] = [10, 1, 3, 5, 7, 9, 11, 13, 15, 17, 19];
    let mut d = digits_of(s);
    if d.len() != 11 || d[0] == 0 {
        return false;
    }
    d[0] -= 1;
    d.iter().zip(W).map(|(x, w)| x * w).sum::<u32>() % 89 == 0
}

const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Decode a base58 string (Bitcoin alphabet).
fn base58_decode(s: &str) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(s.len());
    for c in s.bytes() {
        let mut carry = BASE58.iter().position(|&b| b == c)? as u32;
        for byte in out.iter_mut().rev() {
            carry += (*byte as u32) * 58;
            *byte = (carry & 0xff) as u8;
            carry >>= 8;
        }
        while carry > 0 {
            out.insert(0, (carry & 0xff) as u8);
            carry >>= 8;
        }
    }
    let zeros = s.bytes().take_while(|&b| b == b'1').count();
    let mut v = vec![0u8; zeros];
    v.extend(out);
    Some(v)
}

/// Legacy (P2PKH, `1...`) or P2SH (`3...`) Bitcoin address with a valid base58check checksum.
pub fn btc_base58_valid(s: &str) -> bool {
    use sha2::{Digest, Sha256};
    let Some(raw) = base58_decode(s) else {
        return false;
    };
    if raw.len() != 25 {
        return false;
    }
    let version_ok = match s.as_bytes().first() {
        Some(b'1') => raw[0] == 0x00,
        Some(b'3') => raw[0] == 0x05,
        _ => false,
    };
    let hash = Sha256::digest(Sha256::digest(&raw[..21]));
    version_ok && hash[..4] == raw[21..]
}

/// Segwit address (`bc1...`) with a valid bech32 (v0) or bech32m (v1+) checksum.
pub fn btc_bech32_valid(s: &str) -> bool {
    const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    const GEN: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
    if s.chars().any(|c| c.is_ascii_uppercase()) && s.chars().any(|c| c.is_ascii_lowercase()) {
        return false;
    }
    let lower = s.to_ascii_lowercase();
    let Some((hrp, data)) = lower.rsplit_once('1') else {
        return false;
    };
    if hrp != "bc" || data.len() < 7 {
        return false;
    }
    let Some(values) = data
        .bytes()
        .map(|c| CHARSET.iter().position(|&x| x == c).map(|v| v as u32))
        .collect::<Option<Vec<u32>>>()
    else {
        return false;
    };
    let mut chk: u32 = 1;
    let hrp_exp = hrp
        .bytes()
        .map(|b| (b >> 5) as u32)
        .chain(std::iter::once(0))
        .chain(hrp.bytes().map(|b| (b & 31) as u32));
    for v in hrp_exp.chain(values.iter().copied()) {
        let top = chk >> 25;
        chk = ((chk & 0x1ffffff) << 5) ^ v;
        for (i, g) in GEN.iter().enumerate() {
            if (top >> i) & 1 == 1 {
                chk ^= g;
            }
        }
    }
    match values[0] {
        0 => chk == 1,
        1..=16 => chk == 0x2bc830a3,
        _ => false,
    }
}

/// US Social Security Number structure: area not 000/666/9xx, group not 00, serial not 0000.
pub fn ssn_valid(s: &str) -> bool {
    let d: String = s.chars().filter(char::is_ascii_digit).collect();
    if d.len() != 9 {
        return false;
    }
    let (area, group, serial) = (&d[..3], &d[3..5], &d[5..]);
    area != "000" && area != "666" && !area.starts_with('9') && group != "00" && serial != "0000"
}

/// UK National Insurance number prefix/suffix rules (separators ignored).
pub fn nino_valid(s: &str) -> bool {
    let c: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    if c.len() != 9 {
        return false;
    }
    let (a, b) = (c[0], c[1]);
    let ok_first = a.is_ascii_uppercase() && !"DFIQUV".contains(a);
    let ok_second = b.is_ascii_uppercase() && !"DFIOQUV".contains(b);
    let prefix: String = [a, b].iter().collect();
    ok_first
        && ok_second
        && !["BG", "GB", "NK", "KN", "TN", "NT", "ZZ"].contains(&prefix.as_str())
        && c[2..8].iter().all(char::is_ascii_digit)
        && "ABCD".contains(c[8])
}

const VERHOEFF_D: [[u8; 10]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 2, 3, 4, 0, 6, 7, 8, 9, 5],
    [2, 3, 4, 0, 1, 7, 8, 9, 5, 6],
    [3, 4, 0, 1, 2, 8, 9, 5, 6, 7],
    [4, 0, 1, 2, 3, 9, 5, 6, 7, 8],
    [5, 9, 8, 7, 6, 0, 4, 3, 2, 1],
    [6, 5, 9, 8, 7, 1, 0, 4, 3, 2],
    [7, 6, 5, 9, 8, 2, 1, 0, 4, 3],
    [8, 7, 6, 5, 9, 3, 2, 1, 0, 4],
    [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
];
const VERHOEFF_P: [[u8; 10]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
    [5, 8, 0, 3, 7, 9, 6, 1, 4, 2],
    [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
    [9, 4, 5, 3, 1, 2, 8, 7, 6, 0],
    [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
    [2, 7, 9, 3, 8, 0, 6, 4, 1, 5],
    [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
];
const VERHOEFF_INV: [u8; 10] = [0, 4, 3, 2, 1, 5, 6, 7, 8, 9];

fn verhoeff_c(digits: &[u32], offset: usize) -> u8 {
    let mut c = 0u8;
    for (i, &d) in digits.iter().rev().enumerate() {
        c = VERHOEFF_D[c as usize][VERHOEFF_P[(i + offset) % 8][d as usize] as usize];
    }
    c
}

/// Verhoeff check digit for `payload`.
pub fn verhoeff_check_digit(payload: &[u32]) -> u32 {
    VERHOEFF_INV[verhoeff_c(payload, 1) as usize] as u32
}

/// India Aadhaar: 12 digits, first digit 2-9, Verhoeff checksum (separators ignored).
pub fn aadhaar_valid(s: &str) -> bool {
    let d = digits_of(s);
    d.len() == 12 && d[0] >= 2 && verhoeff_c(&d, 0) == 0
}

/// The two CPF check digits for the first nine digits.
pub fn cpf_check_digits(d9: &[u32]) -> (u32, u32) {
    let s1: u32 = d9.iter().zip((2..=10).rev()).map(|(x, w)| x * w).sum();
    let c1 = (s1 * 10) % 11 % 10;
    let s2: u32 = d9
        .iter()
        .chain(std::iter::once(&c1))
        .zip((2..=11).rev())
        .map(|(x, w)| x * w)
        .sum();
    (c1, (s2 * 10) % 11 % 10)
}

/// Brazil CPF: 11 digits with two mod-11 check digits (separators ignored).
pub fn cpf_valid(s: &str) -> bool {
    let d = digits_of(s);
    if d.len() != 11 || d.iter().all(|&x| x == d[0]) {
        return false;
    }
    cpf_check_digits(&d[..9]) == (d[9], d[10])
}

/// Netherlands BSN: 9 digits passing the "11-proef" (last digit weighted -1).
pub fn bsn_valid(s: &str) -> bool {
    let d = digits_of(s);
    if d.len() != 9 || d.iter().all(|&x| x == 0) {
        return false;
    }
    let sum: i64 = d[..8]
        .iter()
        .zip((2..=9).rev())
        .map(|(x, w)| (*x as i64) * w)
        .sum::<i64>()
        - d[8] as i64;
    sum % 11 == 0
}

const NIF_LETTERS: &[u8] = b"TRWAGMYFPDXBNJZSQVHLCKE";

/// Spanish NIF/DNI (8 digits + letter) or NIE (X/Y/Z + 7 digits + letter) check letter.
pub fn nif_letter(number: u32) -> char {
    NIF_LETTERS[(number % 23) as usize] as char
}

/// Spain NIF/DNI or NIE with a valid control letter (a `-` or space before the letter is allowed).
pub fn nif_valid(s: &str) -> bool {
    let c: String = s
        .chars()
        .filter(|c| !matches!(c, '-' | ' '))
        .collect::<String>()
        .to_ascii_uppercase();
    if c.len() != 9 {
        return false;
    }
    let (body, letter) = c.split_at(8);
    let body = match body.as_bytes()[0] {
        b'X' => format!("0{}", &body[1..]),
        b'Y' => format!("1{}", &body[1..]),
        b'Z' => format!("2{}", &body[1..]),
        _ => body.to_string(),
    };
    match body.parse::<u32>() {
        Ok(n) if body.chars().all(|c| c.is_ascii_digit()) => letter.starts_with(nif_letter(n)),
        _ => false,
    }
}

/// Italian codice fiscale check character for the first 15 characters.
pub fn codice_fiscale_check(first15: &str) -> Option<char> {
    const ODD: [u32; 26] = [
        1, 0, 5, 7, 9, 13, 15, 17, 19, 21, 2, 4, 18, 20, 11, 3, 6, 8, 12, 14, 16, 10, 22, 25, 24,
        23,
    ];
    let mut sum = 0;
    for (i, c) in first15.chars().enumerate() {
        let idx = match c {
            '0'..='9' => c as u32 - '0' as u32,
            'A'..='Z' => c as u32 - 'A' as u32,
            _ => return None,
        };
        sum += if i % 2 == 0 { ODD[idx as usize] } else { idx };
    }
    char::from_u32('A' as u32 + sum % 26)
}

/// Italy codice fiscale: 16 characters, valid structure and check character.
pub fn codice_fiscale_valid(s: &str) -> bool {
    let s = s.to_ascii_uppercase();
    let b = s.as_bytes();
    if b.len() != 16 {
        return false;
    }
    // Digits may be replaced by LMNPQRSTUV ("omocodia") when codes collide.
    let num = |c: u8| c.is_ascii_digit() || b"LMNPQRSTUV".contains(&c);
    let shape = b[..6].iter().all(u8::is_ascii_alphabetic)
        && num(b[6])
        && num(b[7])
        && b"ABCDEHLMPRST".contains(&b[8])
        && num(b[9])
        && num(b[10])
        && b[11].is_ascii_alphabetic()
        && b[12..15].iter().all(|c| num(*c))
        && b[15].is_ascii_alphabetic();
    shape && codice_fiscale_check(&s[..15]) == Some(b[15] as char)
}

/// France INSEE/NIR: 13 characters (Corsica `2A`/`2B` allowed) + 2-digit key = 97 - n mod 97.
pub fn insee_valid(s: &str) -> bool {
    let c: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if c.len() != 15 {
        return false;
    }
    let (body, key) = c.split_at(13);
    insee_key(body).is_some_and(|k| key.parse::<u64>().ok() == Some(k))
}

/// The INSEE control key for a 13-character body.
pub fn insee_key(body: &str) -> Option<u64> {
    let up = body.to_ascii_uppercase();
    if up.len() != 13 || !matches!(up.as_bytes()[0], b'1' | b'2' | b'3' | b'7' | b'8') {
        return None;
    }
    // Corsica: departments 2A / 2B count as 19 / 18 in the key computation.
    let n = match &up[5..7] {
        "2A" => format!("{}19{}", &up[..5], &up[7..]),
        "2B" => format!("{}18{}", &up[..5], &up[7..]),
        _ => up.clone(),
    };
    let v: u64 = n.parse().ok()?;
    let month: u32 = up[3..5].parse().ok()?;
    if !(1..=12).contains(&month) && !(20..=42).contains(&month) && !(50..=99).contains(&month) {
        return None;
    }
    Some(97 - v % 97)
}

/// Germany Steuer-ID check digit (ISO 7064 MOD 11,10) for the first ten digits.
pub fn steuer_id_check_digit(d10: &[u32]) -> u32 {
    let mut product = 10;
    for &d in d10 {
        let mut sum = (d + product) % 10;
        if sum == 0 {
            sum = 10;
        }
        product = (sum * 2) % 11;
    }
    let c = 11 - product;
    if c == 10 {
        0
    } else {
        c
    }
}

/// Germany Steuerliche Identifikationsnummer: 11 digits, no leading zero, exactly one digit
/// repeated (2-3 times) in the first ten, valid check digit.
pub fn steuer_id_valid(s: &str) -> bool {
    let d = digits_of(s);
    if d.len() != 11 || d[0] == 0 {
        return false;
    }
    let mut counts = [0u8; 10];
    d[..10].iter().for_each(|&x| counts[x as usize] += 1);
    let repeated: Vec<u8> = counts.iter().copied().filter(|&c| c >= 2).collect();
    repeated.len() == 1 && repeated[0] <= 3 && steuer_id_check_digit(&d[..10]) == d[10]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn national_ids() {
        assert!(ssn_valid("123-45-6789"));
        assert!(!ssn_valid("666-45-6789"));
        assert!(!ssn_valid("923-45-6789"));
        assert!(!ssn_valid("123-00-6789"));
        assert!(nino_valid("AB 12 34 56 C"));
        assert!(nino_valid("JG103759A"));
        assert!(!nino_valid("ZZ123456A"));
        assert!(!nino_valid("DA123456A"));
        assert!(!nino_valid("AB123456E"));
        assert!(aadhaar_valid("2341 2341 2346"));
        assert!(!aadhaar_valid("2341 2341 2347"));
        assert!(!aadhaar_valid("1341 2341 2346"));
        let mut p = vec![4, 9, 9, 1, 1, 8, 6, 6, 5, 2, 3];
        p.push(verhoeff_check_digit(&p));
        assert!(aadhaar_valid(
            &p.iter().map(|d| d.to_string()).collect::<String>()
        ));
        assert!(cpf_valid("529.982.247-25"));
        assert!(!cpf_valid("529.982.247-26"));
        assert!(!cpf_valid("111.111.111-11"));
        assert!(bsn_valid("111222333"));
        assert!(bsn_valid("123456782"));
        assert!(!bsn_valid("123456789"));
        assert!(nif_valid("12345678Z"));
        assert!(nif_valid("12345678-Z"));
        assert!(!nif_valid("12345678A"));
        assert!(nif_valid("X1234567L"));
        assert!(!nif_valid("X1234567A"));
        assert!(codice_fiscale_valid("RSSMRA85T10A562S"));
        assert!(!codice_fiscale_valid("RSSMRA85T10A562T"));
        assert!(!codice_fiscale_valid("RSSMRA85X10A562S"));
        assert!(insee_valid("1 84 12 76 451 089 46"));
        assert!(insee_valid("184127645108946"));
        assert!(!insee_valid("184127645108947"));
        assert!(!insee_valid("484127645108946"));
        assert!(steuer_id_valid("86095742719"));
        assert!(!steuer_id_valid("86095742718"));
        assert!(!steuer_id_valid("06095742719"));
    }

    #[test]
    fn nhs_tfn_abn() {
        assert!(nhs_valid("943 476 5919"));
        assert!(nhs_valid("9434765919"));
        assert!(!nhs_valid("943 476 5918"));
        assert!(!nhs_valid("1111111111"));
        assert!(tfn_valid("123 456 782"));
        assert!(!tfn_valid("123 456 789"));
        assert!(abn_valid("51 824 753 556"));
        assert!(!abn_valid("51 824 753 557"));
    }

    #[test]
    fn bitcoin() {
        assert!(btc_base58_valid("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"));
        assert!(btc_base58_valid("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy"));
        assert!(!btc_base58_valid("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb"));
        assert!(!btc_base58_valid("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLz"));
        assert!(btc_bech32_valid(
            "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
        ));
        assert!(btc_bech32_valid(
            "BC1QW508D6QEJXTDG4Y5R3ZARVARY0C5XW7KV8F3T4"
        ));
        assert!(btc_bech32_valid(
            "bc1p5d7rjq7g6rdk2yhzks9smlaqtedr4dekq08ge8ztwac72sfr9rusxg3297"
        ));
        assert!(!btc_bech32_valid(
            "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdp"
        ));
    }

    #[test]
    fn luhn() {
        assert!(luhn_valid("4111 1111 1111 1111"));
        assert!(!luhn_valid("4111 1111 1111 1112"));
        let p = [4, 5, 3, 2, 0, 1, 5, 1, 1, 2, 8, 3, 0, 3, 6];
        let c = luhn_check_digit(&p);
        let s: String = p
            .iter()
            .chain(std::iter::once(&c))
            .map(|d| char::from_digit(*d, 10).unwrap())
            .collect();
        assert!(luhn_valid(&s));
    }

    #[test]
    fn iban() {
        assert!(iban_valid("DE89 3704 0044 0532 0130 00"));
        assert!(iban_valid("GB82WEST12345698765432"));
        assert!(!iban_valid("DE89 3704 0044 0532 0130 01"));
        assert_eq!(iban_check_digits("DE", "370400440532013000"), "89");
    }
}
