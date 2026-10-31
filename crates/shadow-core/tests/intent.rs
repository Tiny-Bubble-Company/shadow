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
fn an_email_action_shows_the_real_recipient_and_masks_the_content() {
    let (shadow, _d) = shadow();
    let req = json!({
        "model": "gpt-5",
        "messages": [{ "role": "user", "content": "Email jane.doe@acme.com that the Acme GmbH renewal is signed" }],
        "tools": [{ "type": "function", "function": { "name": "send_email", "parameters": {} } }]
    });
    let session = shadow.session_for(&req);
    assert_eq!(session.classification().intent, Intent::Act);
    let mut out = req.clone();
    session.mask_json(&mut out);
    let sent = out["messages"][0]["content"].as_str().unwrap();
    assert!(sent.contains("jane.doe@acme.com"), "the recipient: {sent}");
    assert!(!sent.contains("Acme GmbH"), "the content is masked: {sent}");
    assert_eq!(session.summary().kept, 1);
}

#[test]
fn recipients_can_be_hidden_and_restored_in_the_tool_call_instead() {
    // [policy.act] targets = "surrogate": the model sees a stand-in, and Shadow restores the
    // real recipient in the tool call on this machine.
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = ShadowConfig::from_dir(dir.path());
    cfg.policy.act.targets = shadow_core::Treatment::Surrogate;
    let shadow = Shadow::open(cfg).unwrap();
    let req = json!({ "messages": [{ "role": "user", "content": "Email jane.doe@acme.com that the renewal is signed" }] });
    let session = shadow.session_for(&req);
    let mut out = req.clone();
    session.mask_json(&mut out);
    let fake = out["messages"][0]["content"]
        .as_str()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .to_string();
    assert!(fake.contains('@') && fake != "jane.doe@acme.com", "{fake}");
    let mut call = json!({ "name": "send_email", "arguments": { "to": fake } });
    session.unmask_json(&mut call);
    assert_eq!(call["arguments"]["to"], "jane.doe@acme.com");
}

#[test]
fn a_provider_hosted_tool_gets_what_the_action_needs_and_nothing_else() {
    // The provider sends this email itself, so Shadow never sees the result: a fake address
    // would reach someone else and a fake company name would end up in Jane's inbox.
    let (shadow, _d) = shadow();
    let req = json!({
        "input": [{ "role": "user", "content": [{ "type": "input_text", "text": "Send jane.doe@acme.com the Acme GmbH contract summary. Our deploy key is ghp_Q8v2Lr7XcN4mT9wK1sB6yH3dF5gJ0pZaE2uR" }] }],
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
        text.contains("jane.doe@acme.com") && text.contains("Acme GmbH"),
        "the provider runs the tool, so what the email is about stays real: {text}"
    );
    assert!(
        !text.contains("ghp_Q8v2Lr7XcN4mT9wK1sB6yH3dF5gJ0pZaE2uR"),
        "a secret is never part of what the action needs: {text}"
    );
    assert!(session.summary().kept >= 2);
}

#[test]
fn a_follow_up_action_by_the_provider_keeps_the_earlier_recipient() {
    // "Now send her the invite": the address is in an earlier turn, not the instruction.
    let (shadow, _d) = shadow();
    let req = json!({
        "input": [
            { "role": "user", "content": "Jane Doe (jane.doe@acme.com) is joining the launch call." },
            { "role": "assistant", "content": "Noted." },
            { "role": "user", "content": "Now send her the invite." }
        ],
        "tools": [{ "type": "mcp", "server_label": "calendar", "server_url": "https://mcp.calendar.example.com" }]
    });
    let session = shadow.session_for(&req);
    assert_eq!(session.classification().intent, Intent::Act);
    let mut out = req.clone();
    session.mask_json(&mut out);
    let first = out.pointer("/input/0/content").unwrap().as_str().unwrap();
    assert!(first.contains("jane.doe@acme.com"), "{first}");
}

#[test]
fn drafting_an_email_shows_the_model_the_real_recipient_and_masks_the_rest() {
    // The goal is a message to Jane: the model must see Jane, or it writes to someone else.
    // What the message is about (another person, a health detail, a card) is still masked.
    let (shadow, _d) = shadow();
    let req = json!({ "messages": [{ "role": "user", "content": "Draft a renewal reminder to Jane Doe (jane.doe@acme.com) at Acme GmbH. Mention that Max Weber approved it and the card 4111 1111 1111 1111 will be charged." }] });
    let session = shadow.session_for(&req);
    assert_eq!(session.classification().intent, Intent::Act);
    let mut out = req.clone();
    session.mask_json(&mut out);
    let sent = out["messages"][0]["content"].as_str().unwrap();
    assert!(
        sent.contains("Jane Doe")
            && sent.contains("jane.doe@acme.com")
            && sent.contains("Acme GmbH"),
        "the recipient is passed through: {sent}"
    );
    assert!(
        !sent.contains("Max Weber") && !sent.contains("4111 1111 1111 1111"),
        "everything that isn't the recipient is masked: {sent}"
    );
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

#[test]
fn real_text_equal_to_a_fake_survives_the_round_trip() {
    let (shadow, _d) = shadow();
    // Make "fake_of_jane" exist, then send a prompt that contains that fake as real text.
    let first = shadow.mask("Email jane.doe@acme.com");
    let fake = first.text.trim_start_matches("Email ").to_string();
    let text = format!("Forward {fake}'s note to jane.doe@acme.com");
    let m = shadow.mask(&text);
    assert!(
        !m.text.starts_with(&format!("Forward {fake}")),
        "the colliding real value gets its own fake: {}",
        m.text
    );
    assert_eq!(shadow.unmask(&m.text), text);
}

#[test]
fn an_answer_that_uses_only_a_first_name_still_names_the_real_person() {
    // Jane is who the text is about, not who it is for, so she is masked. Models refer to
    // people by first name: "Hui said" must come back as "Jane said".
    // Without this machine's own identity in the vault, so no other fake person can share
    // the first name (a shared part is ambiguous and deliberately not restored).
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = ShadowConfig::from_dir(dir.path());
    cfg.detectors.local_facts = false;
    let shadow = Shadow::open(cfg).unwrap();
    let req = json!({ "messages": [{ "role": "user", "content": "Summarise the workshop feedback from Jane Doe for the team." }] });
    let session = shadow.session_for(&req);
    let mut out = req.clone();
    session.mask_json(&mut out);
    let sent = out["messages"][0]["content"].as_str().unwrap().to_string();
    let fake = sent
        .trim_start_matches("Summarise the workshop feedback from ")
        .trim_end_matches(" for the team.")
        .to_string();
    assert!(!fake.contains("Jane"), "{sent}");
    let parts: Vec<&str> = fake.split_whitespace().collect();
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    let mut answer = json!({ "content": format!("Dear {first},\n\nThank you for the workshop. Best wishes to the {last} family.") });
    session.unmask_json(&mut answer);
    let back = answer["content"].as_str().unwrap();
    assert!(back.starts_with("Dear Jane,"), "{back}");
    assert!(back.contains("the Doe family"), "{back}");
}

#[test]
fn a_real_person_who_shares_a_fake_first_name_is_not_renamed() {
    let (shadow, _d) = shadow();
    let fake = shadow.mask("Jane Doe called.").text;
    let first = fake.split_whitespace().next().unwrap().to_string();
    // Later, a real person with that first name appears. Their name must survive the round trip.
    let text = format!("{first} Example from accounting joined the call.");
    let masked = shadow.mask(&text).text;
    assert_eq!(shadow.unmask(&masked), text);
}
