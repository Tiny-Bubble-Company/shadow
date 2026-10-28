use shadow_core::detect::checksum::{iban_valid, luhn_valid};
use shadow_core::{Kind, Shadow, ShadowConfig};

fn config(dir: &std::path::Path) -> ShadowConfig {
    let mut c = ShadowConfig::from_dir(dir)
        .with_region("CN")
        .with_terms(vec![
            "Acme GmbH".into(),
            "Project Falcon".into(),
            "Globex".into(),
        ]);
    c.detectors.local_facts = false;
    c
}

const SECRETS: &[&str] = &[
    "jane.doe@acme.com",
    "+49 30 1234 5678",
    "John Smith",
    "Acme GmbH",
    "Acme",
    "Project Falcon",
    "Globex",
    "DE89 3704 0044 0532 0130 00",
    "4111 1111 1111 1111",
    "sk-proj-Abcdefghijklmnopqrstuvwx12345",
    "/Users/jdoe/",
    "10.20.30.40",
    "git.acme.com",
    "123-45-6789",
];

const PROMPT: &str = "Hi, I'm John Smith from Acme GmbH (jane.doe@acme.com, +49 30 1234 5678). \
Acme wants Project Falcon shipped before Globex notices. Wire the deposit to IBAN DE89 3704 0044 0532 0130 00 \
or charge 4111 1111 1111 1111. My key is sk-proj-Abcdefghijklmnopqrstuvwx12345 and the repo lives at \
/Users/jdoe/src/falcon on 10.20.30.40, mirrored at https://git.acme.com/falcon. SSN 123-45-6789. \
Also see https://github.com/rust-lang/rust and docs.rs.";

#[test]
fn mask_unmask_roundtrip_and_no_leaks() {
    let dir = tempfile::tempdir().unwrap();
    let s = Shadow::open(config(dir.path())).unwrap();
    let m = s.mask(PROMPT);
    for secret in SECRETS {
        assert!(!m.text.contains(secret), "{secret:?} leaked in {}", m.text);
    }
    assert!(!m.text.to_lowercase().contains("acme"), "{}", m.text);
    assert!(
        m.text.contains("github.com/rust-lang/rust"),
        "public domains are kept"
    );
    assert!(m.replacements >= 12, "{m:?}");
    for k in [
        "email",
        "phone",
        "person",
        "org",
        "term",
        "iban",
        "credit_card",
        "secret",
        "username",
        "ip",
        "hostname",
        "national_id",
    ] {
        assert!(
            m.kinds.iter().any(|x| x == k),
            "missing kind {k}: {:?}",
            m.kinds
        );
    }
    assert_eq!(s.unmask(&m.text), PROMPT);
    // The persona's company stands in for the primary org.
    assert!(m.text.contains(&s.persona().company), "{}", m.text);
}

#[test]
fn fakes_are_consistent_and_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let first = {
        let s = Shadow::open(config(dir.path())).unwrap();
        let a = s.mask(PROMPT).text;
        assert_eq!(a, s.mask(PROMPT).text, "same input, same output");
        a
    };
    let s = Shadow::open(config(dir.path())).unwrap();
    assert_eq!(
        s.mask(PROMPT).text,
        first,
        "consistent across process restarts"
    );
    assert_eq!(s.unmask(&first), PROMPT);
    assert!(s.stats().entries >= 12);
}

#[test]
fn fakes_keep_checksums_and_formats() {
    let dir = tempfile::tempdir().unwrap();
    let s = Shadow::open(config(dir.path())).unwrap();
    for card in [
        "4111 1111 1111 1111",
        "5500-0000-0000-0004",
        "378282246310005",
    ] {
        let f = s.mask(card).text;
        assert_ne!(f, card);
        assert_eq!(f.len(), card.len());
        assert!(luhn_valid(&f), "{f} must pass Luhn");
    }
    for iban in ["DE89 3704 0044 0532 0130 00", "GB82WEST12345698765432"] {
        let f = s.mask(iban).text;
        assert_ne!(f, iban);
        assert_eq!(f.len(), iban.len());
        assert!(iban_valid(&f), "{f} must be a valid IBAN");
    }
    let phone = s.mask("+49 30 1234 5678").text;
    assert!(phone.starts_with("+86 "), "{phone}");
    let mac = s.mask("3c:22:fb:12:ab:9e").text;
    assert_eq!(mac.matches(':').count(), 5);
}

#[test]
fn fakes_are_unique() {
    let dir = tempfile::tempdir().unwrap();
    let s = Shadow::open(config(dir.path())).unwrap();
    let text: String = (0..60)
        .map(|i| format!("user{i}@corp{i}.example-internal.io "))
        .collect();
    let m = s.mask(&text);
    assert_eq!(s.unmask(&m.text), text);
    let entries = s.export();
    let mut fakes: Vec<&str> = entries.iter().map(|e| e.fake.as_str()).collect();
    fakes.sort();
    let n = fakes.len();
    fakes.dedup();
    assert_eq!(n, fakes.len());
}

#[test]
fn case_variants_and_runtime_terms() {
    let dir = tempfile::tempdir().unwrap();
    let s = Shadow::open(config(dir.path())).unwrap();
    let m = s.mask("ACME and acme and Acme");
    assert!(!m.text.to_lowercase().contains("acme"), "{}", m.text);
    assert_eq!(s.unmask(&m.text), "ACME and acme and Acme");

    assert_eq!(s.mask("Codename Nightjar").replacements, 0);
    s.add_terms(&["Nightjar".into()]);
    let m = s.mask("Codename Nightjar");
    assert!(!m.text.contains("Nightjar"));
    // persisted for the next process
    let s2 = Shadow::open(config(dir.path())).unwrap();
    assert!(s2
        .terms()
        .iter()
        .any(|(t, k)| t == "Nightjar" && *k == Kind::Term));
    assert_eq!(s2.mask("Codename Nightjar").text, m.text);
}

#[test]
fn json_masking_skips_protocol_fields() {
    let dir = tempfile::tempdir().unwrap();
    let s = Shadow::open(config(dir.path())).unwrap();
    let original = serde_json::json!({
        "model": "gpt-4o",
        "user": "jane@acme.com",
        "metadata": {"user_id": "u-123"},
        "messages": [
            {"role": "system", "content": "You work for Acme GmbH."},
            {"role": "user", "content": [{"type": "text", "text": "Email John Smith at jane.doe@acme.com"}]}
        ]
    });
    let mut v = original.clone();
    let n = s.mask_json(&mut v);
    assert!(n >= 3);
    assert_eq!(s.scrub_identifiers(&mut v), 2);
    assert_eq!(v["model"], "gpt-4o");
    assert_eq!(v["messages"][0]["role"], "system");
    assert_eq!(v["messages"][1]["content"][0]["type"], "text");
    assert_eq!(v["user"], s.persona().user_id);
    let dumped = v.to_string();
    assert!(
        !dumped.contains("acme") && !dumped.contains("Acme") && !dumped.contains("John Smith"),
        "{dumped}"
    );
    s.unmask_json(&mut v);
    assert_eq!(v["messages"], original["messages"]);
}

#[test]
fn streaming_unmask_restores_fakes_split_at_every_offset() {
    let dir = tempfile::tempdir().unwrap();
    let s = Shadow::open(config(dir.path())).unwrap();
    let masked = s.mask(PROMPT).text;
    for i in 0..=masked.len() {
        if !masked.is_char_boundary(i) {
            continue;
        }
        let mut u = s.stream_unmasker();
        let mut out = u.push(&masked[..i]);
        out += &u.push(&masked[i..]);
        out += &u.finish();
        assert_eq!(out, PROMPT, "split at {i}");
    }
    // byte-by-byte (char-by-char) streaming
    let mut u = s.stream_unmasker();
    let mut out = String::new();
    for ch in masked.chars() {
        out += &u.push(&ch.to_string());
    }
    out += &u.finish();
    assert_eq!(out, PROMPT);
}

#[test]
fn allow_terms_and_forget() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = config(dir.path());
    c.allow_terms = vec!["john smith".into()];
    let s = Shadow::open(c).unwrap();
    assert_eq!(s.mask("Ask John Smith").replacements, 0);
    let m = s.mask("ping 10.9.8.7");
    assert_eq!(s.forget("10.9.8.7").unwrap(), 1);
    assert_eq!(s.unmask(&m.text), m.text, "forgotten mapping is gone");
}
