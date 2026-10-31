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

#[cfg(test)]
mod tests {
    use super::*;

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
