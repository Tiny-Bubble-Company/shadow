//! Checksums used to validate (and generate) cards and IBANs.

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

#[cfg(test)]
mod tests {
    use super::*;

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
