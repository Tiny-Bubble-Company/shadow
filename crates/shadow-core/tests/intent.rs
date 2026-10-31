//! Intent-aware masking end to end: an action still reaches the right person, a report over
//! customer data is tokenised, and the provider never sees what it doesn't need.

use serde_json::json;
use shadow_core::{Intent, Shadow, ShadowConfig};

fn shadow() -> (Shadow, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let s = Shadow::open(ShadowConfig::from_dir(dir.path()).with_terms(vec!["Acme GmbH".into()]))
        .unwrap();
    (s, dir)
}

#[test]
fn an_email_action_with_a_local_tool_reaches_the_real_recipient() {
    let (shadow, _d) = shadow();
    let req = json!({
        "model": "gpt-5",
        "messages": [{ "role": "user", "content": "Email jane.doe@acme.com that the Acme GmbH renewal is signed" }],
        "tools": [{ "type": "function", "function": { "name": "send_email", "parameters": {} } }]
    });
    let session = shadow.session_for(&req);
    assert_eq!(session.classification().intent, Intent::Act);

    // The provider sees a realistic fake address…
    let mut out = req.clone();
    session.mask_json(&mut out);
    let sent = out.to_string();
    assert!(
        !sent.contains("jane.doe@acme.com") && !sent.contains("Acme GmbH"),
        "{sent}"
    );
    let fake = out["messages"][0]["content"]
        .as_str()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .to_string();
    assert!(fake.contains('@'), "still an email: {fake}");

    // …the model calls the tool with it, and Shadow restores the real recipient on this machine.
    let mut call = json!({ "name": "send_email", "arguments": { "to": fake, "body": "Signed!" } });
    session.unmask_json(&mut call);
    assert_eq!(call["arguments"]["to"], "jane.doe@acme.com");
    assert_eq!(session.summary().kept, 0);
}

#[test]
fn a_provider_hosted_tool_gets_the_real_target_and_nothing_else() {
    let (shadow, _d) = shadow();
    let req = json!({
        "input": [{ "role": "user", "content": [{ "type": "input_text", "text": "Send jane.doe@acme.com the Acme GmbH contract summary" }] }],
        "tools": [{ "type": "mcp", "server_label": "mail", "server_url": "https://mcp.mail.example.com" }]
    });
    let session = shadow.session_for(&req);
    assert!(session.classification().remote_actions);
    let mut out = req.clone();
    session.mask_json(&mut out);
    let text = out
        .pointer("/input/0/content/0/text")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(
        text.contains("jane.doe@acme.com"),
        "the provider runs the tool, so the target is kept: {text}"
    );
    assert!(
        !text.contains("Acme GmbH"),
        "everything that isn't the target is still masked: {text}"
    );
    assert_eq!(session.summary().kept, 1);
}

#[test]
fn a_report_over_customers_uses_tokens_and_reads_back_in_full() {
    let (shadow, _d) = shadow();
    let rows = "name,email,plan\nJane Doe,jane.doe@acme.com,pro\nMax Weber,max.weber@globex.de,free\nPriya Patel,priya@initech.io,pro\nLuis Ortega,luis@hooli.com,pro\nAna Silva,ana@umbrella.co,free";
    let req = json!({ "messages": [{ "role": "user", "content": format!("Build a churn report for these customers:\n{rows}") }] });
    let session = shadow.session_for(&req);
    assert_eq!(session.classification().intent, Intent::Report);

    let mut out = req.clone();
    session.mask_json(&mut out);
    let text = out["messages"][0]["content"].as_str().unwrap().to_string();
    assert!(
        text.contains("[EMAIL_0001]") && !text.contains("@"),
        "{text}"
    );
    assert!(
        !text.contains("jane.doe") && !text.contains("Weber"),
        "{text}"
    );

    // The answer refers to tokens; the reader sees the real customers.
    let answer = "Highest churn risk: [EMAIL_0002] (free plan).";
    assert_eq!(
        session.unmask(answer),
        "Highest churn risk: max.weber@globex.de (free plan)."
    );
    let mut stream = session.stream_unmasker();
    let mut got = stream.push("Highest churn risk: [EMA");
    got += &stream.push("IL_0002] (free plan).");
    got += &stream.finish();
    assert_eq!(
        got, "Highest churn risk: max.weber@globex.de (free plan).",
        "tokens split across stream chunks"
    );
    assert!(session.summary().tokens >= 5);
}

#[test]
fn intent_can_be_switched_off() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = ShadowConfig::from_dir(dir.path());
    cfg.policy.intent = false;
    let shadow = Shadow::open(cfg).unwrap();
    let req = json!({ "messages": [{ "role": "user", "content": "Summarise these customers: jane.doe@acme.com, max@globex.de, priya@initech.io" }] });
    let session = shadow.session_for(&req);
    let mut out = req.clone();
    session.mask_json(&mut out);
    assert!(
        !out.to_string().contains("[EMAIL_"),
        "plain surrogates when intent is off"
    );
}

#[test]
fn ids_and_wallets_keep_their_shape_without_leaking() {
    let (shadow, _d) = shadow();
    let pan = "PAN ABCPD1234F on file";
    let m = shadow.mask(pan);
    let fake = m.text.split_whitespace().nth(1).unwrap();
    assert_ne!(fake, "ABCPD1234F");
    assert!(
        !fake.contains("ABCPD"),
        "the letters carry name initials: {fake}"
    );
    let eth = "send to 0x52908400098527886E0F7030069857D2E4169EE7";
    let m = shadow.mask(eth);
    let w = m.text.rsplit(' ').next().unwrap();
    assert!(
        w.starts_with("0x") && w.len() == 42 && w[2..].chars().all(|c| c.is_ascii_hexdigit()),
        "{w}"
    );
    assert_eq!(shadow.unmask(&m.text), eth);
}
