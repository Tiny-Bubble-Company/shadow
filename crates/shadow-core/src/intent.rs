//! What a request is trying to do, and what that means for each sensitive value in it.
//!
//! Masking everything the same way breaks real work. "Email jane@acme.com about the renewal"
//! needs the recipient to survive; "build a churn report from this customer list" needs the
//! opposite. Shadow classifies each request from its latest instruction and the tools on offer:
//!
//! | Intent     | Typical request                                   | Default treatment                        |
//! |------------|---------------------------------------------------|------------------------------------------|
//! | `act`      | send, email, call, invite, book, pay, post        | targets: surrogate (restored in the tool call locally), or keep when the provider runs the tool |
//! | `report`   | report, summarise, analyse, export over many records | personal data → typed tokens (`[PERSON_0007]`) |
//! | `code`     | code, stack traces, configs                       | surrogate                                |
//! | `converse` | everything else                                   | surrogate                                |
//!
//! `[policy.always]` forces a treatment whatever the intent, e.g. `secret = "redact"`.

use serde::Serialize;
use serde_json::Value;

use crate::config::{PolicyConfig, Treatment};
use crate::detect::{Finding, Kind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    Act,
    Report,
    Code,
    Converse,
}

impl Intent {
    pub fn as_str(self) -> &'static str {
        match self {
            Intent::Act => "act",
            Intent::Report => "report",
            Intent::Code => "code",
            Intent::Converse => "converse",
        }
    }
}

/// A tool the model may call, and where it runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ToolInfo {
    pub name: String,
    /// Executed by the provider (hosted MCP servers, server-side tools), so Shadow can't
    /// restore values in its arguments before it runs.
    pub remote: bool,
}

/// What Shadow knows about a request before masking it.
#[derive(Clone, Debug, Default, Serialize)]
pub struct RequestContext {
    /// The latest instruction from the user.
    pub instruction: String,
    pub tools: Vec<ToolInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Classification {
    pub intent: Intent,
    /// An action would be carried out by the provider itself.
    pub remote_actions: bool,
    /// Why, in a few words each (for logs and `shadow explain`).
    pub reasons: Vec<String>,
}

impl Default for Classification {
    fn default() -> Self {
        Classification {
            intent: Intent::Converse,
            remote_actions: false,
            reasons: vec![],
        }
    }
}

const ACT_VERBS: &[&str] = &[
    "send",
    "email",
    "e-mail",
    "mail",
    "message",
    "text",
    "sms",
    "whatsapp",
    "call",
    "phone",
    "ring",
    "dial",
    "invite",
    "book",
    "schedule",
    "reschedule",
    "reserve",
    "pay",
    "transfer",
    "wire",
    "refund",
    "post",
    "publish",
    "tweet",
    "reply",
    "respond",
    "forward",
    "notify",
    "ping",
    "dm",
    "remind",
    "share",
    "assign",
    "order",
    "cancel",
    "submit",
    "file",
];
const ACT_OBJECTS: &[&str] = &[
    "ticket",
    "issue",
    "event",
    "meeting",
    "invoice",
    "appointment",
    "calendar",
    "pull request",
    "task",
];
const REPORT_WORDS: &[&str] = &[
    "report",
    "summary",
    "summarise",
    "summarize",
    "analyse",
    "analyze",
    "analysis",
    "breakdown",
    "aggregate",
    "statistics",
    "stats",
    "trend",
    "trends",
    "dashboard",
    "churn",
    "cohort",
    "segment",
    "segments",
    "kpi",
    "kpis",
    "metrics",
    "export",
    "spreadsheet",
    "pivot",
    "insights",
    "distribution",
    "average",
    "median",
    "top",
    "rank",
    "ranking",
];
const RECORD_NOUNS: &[&str] = &[
    "customers",
    "clients",
    "users",
    "employees",
    "patients",
    "members",
    "accounts",
    "leads",
    "contacts",
    "records",
    "rows",
];

fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '-'))
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Words that can come before the verb of a request: "please", "can you", "go ahead and".
const LEAD_WORDS: &[&str] = &[
    "please", "can", "could", "would", "will", "you", "kindly", "go", "ahead", "and", "now",
    "then", "also", "pls", "hey", "just",
];
/// Words after which an action verb is still the thing being asked for: "I need you **to**
/// email", "write it **and** send it", "**then** book".
const VERB_LINKS: &[&str] = &["to", "and", "then", "please", "you", "also", "&"];

/// Does the instruction ask for an action (rather than mention one)?
fn asks_for_action(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for clause in lower.split(['.', '\n', '!', '?', ';']) {
        let ws = words(clause);
        let start = ws
            .iter()
            .position(|w| !LEAD_WORDS.contains(&w.as_str()))
            .unwrap_or(ws.len());
        if let Some(first) = ws.get(start) {
            if ACT_VERBS.contains(&first.as_str()) {
                return Some(format!("instruction starts with \"{first}\""));
            }
            if (first == "create" || first == "open" || first == "add" || first == "set")
                && ws[start..]
                    .iter()
                    .take(5)
                    .any(|w| ACT_OBJECTS.iter().any(|o| o.starts_with(w.as_str())))
            {
                return Some(format!("asks to {first} a record elsewhere"));
            }
        }
        // "I need you to email Jane", "draft the reply and send it".
        if let Some(w) = ws.windows(2).find_map(|p| {
            (VERB_LINKS.contains(&p[0].as_str()) && ACT_VERBS.contains(&p[1].as_str()))
                .then(|| p[1].clone())
        }) {
            return Some(format!("asks to \"{w}\""));
        }
        // "Draft a renewal reminder to Jane", "write a letter for Mr Okafor": the goal is a
        // message to a person, so it is treated like sending one.
        if let Some(i) = ws.iter().position(|w| COMPOSE_VERBS.contains(&w.as_str())) {
            if ws[i + 1..]
                .iter()
                .take(6)
                .any(|w| MESSAGE_NOUNS.contains(&w.as_str()))
            {
                return Some(format!("asks to {} a message", ws[i]));
            }
        }
    }
    None
}

const COMPOSE_VERBS: &[&str] = &["draft", "write", "compose", "prepare", "create", "redraft"];
const MESSAGE_NOUNS: &[&str] = &[
    "email",
    "e-mail",
    "mail",
    "letter",
    "message",
    "reply",
    "response",
    "note",
    "reminder",
    "invitation",
    "invite",
    "follow-up",
    "followup",
    "text",
    "sms",
    "dm",
    "memo",
    "notice",
    "card",
    "post",
    "announcement",
    "apology",
    "thank-you",
    "thanks",
    "congratulations",
    "condolences",
    "proposal",
];

/// Words that end the recipient part of an instruction: what follows is the content.
const RECIPIENT_END: &[&str] = &[
    " about ",
    " regarding ",
    " re: ",
    " re ",
    " that ",
    " saying ",
    " telling ",
    " asking ",
    " to say",
    " to tell",
    " to ask",
    " with the ",
    " with a ",
    " with our ",
    " explaining ",
    " to confirm",
    " to let ",
    " so that ",
    " because ",
    " if ",
    " when ",
];

/// The parts of an instruction that name who an action or message is for: "Draft a reminder
/// to **Jane Doe (jane@acme.com) at Acme GmbH**. Their plan renews...", "Email
/// **jane@acme.com** that the renewal is signed". Values found here are the recipient: the
/// model has to see them as they are to write to, or act on, the right person.
pub fn recipient_phrases(instruction: &str) -> Vec<String> {
    let lower = instruction.to_lowercase();
    let mut out = vec![];
    // Sentence by sentence; a "." inside an address or a number doesn't end one.
    let mut sentences = vec![];
    let mut start = 0;
    for (i, c) in lower.char_indices() {
        let next = lower[i + c.len_utf8()..].chars().next();
        if matches!(c, '\n' | '!' | '?' | ';') || (c == '.' && next.is_none_or(char::is_whitespace))
        {
            sentences.push((start, i));
            start = i + c.len_utf8();
        }
    }
    sentences.push((start, lower.len()));
    for (s, e) in sentences {
        let sentence = &lower[s..e];
        if asks_for_action(sentence).is_none() {
            continue;
        }
        let ws: Vec<(usize, &str)> = sentence
            .split_whitespace()
            .map(|w| (w.as_ptr() as usize - sentence.as_ptr() as usize, w))
            .collect();
        let bare = |w: &str| {
            w.trim_matches(|c: char| !(c.is_alphanumeric() || c == '-'))
                .to_string()
        };
        // Where the recipient starts: after "to"/"for" once the verb has appeared, else right
        // after a verb that takes the person as its object ("email jane@...", "call Jo").
        let verb = ws.iter().position(|(_, w)| {
            let w = bare(w);
            ACT_VERBS.contains(&w.as_str()) || COMPOSE_VERBS.contains(&w.as_str())
        });
        let Some(v) = verb else { continue };
        let after_prep = ws[v + 1..]
            .iter()
            .position(|(_, w)| matches!(bare(w).as_str(), "to" | "for" | "at"))
            .map(|i| v + 1 + i + 1);
        let direct = ACT_VERBS.contains(&bare(ws[v].1).as_str())
            && !COMPOSE_VERBS.contains(&bare(ws[v].1).as_str());
        let from = match (after_prep, direct) {
            (Some(i), false) => i,
            (Some(i), true) if i == v + 2 => i, // "send to Jane"
            (_, true) => v + 1,
            (None, false) => continue,
        };
        let Some(&(offset, _)) = ws.get(from) else {
            continue;
        };
        let rest = &sentence[offset..];
        let end = RECIPIENT_END
            .iter()
            .filter_map(|m| rest.find(m))
            .min()
            .unwrap_or(rest.len());
        let (a, b) = (s + offset, s + offset + end);
        let phrase = instruction[a..b].trim();
        if !phrase.is_empty() {
            out.push(phrase.to_string());
        }
    }
    out
}

/// Many records, or a table of personal data, is a report whatever the wording.
fn looks_tabular(text: &str) -> bool {
    let rows = text
        .lines()
        .filter(|l| l.matches([',', '\t', '|', ';']).count() >= 2)
        .count();
    rows >= 5
}

/// Classify a request. `personal_values` is how many person/contact/ID findings it contains.
pub fn classify(ctx: &RequestContext, personal_values: usize) -> Classification {
    let mut reasons = vec![];
    let remote_actions = ctx.tools.iter().any(|t| t.remote);
    let lower = ctx.instruction.to_lowercase();
    let ws = words(&ctx.instruction);

    if let Some(why) = asks_for_action(&ctx.instruction) {
        reasons.push(why);
        if !ctx.tools.is_empty() {
            reasons.push(format!(
                "{} tool(s) available{}",
                ctx.tools.len(),
                if remote_actions {
                    ", some run by the provider"
                } else {
                    ""
                }
            ));
        }
        return Classification {
            intent: Intent::Act,
            remote_actions,
            reasons,
        };
    }

    let report_word = ws.iter().find(|w| REPORT_WORDS.contains(&w.as_str()));
    let about_records = ws.iter().any(|w| RECORD_NOUNS.contains(&w.as_str()));
    if let Some(w) = report_word
        .filter(|_| about_records || personal_values >= 3 || looks_tabular(&ctx.instruction))
    {
        reasons.push(format!("asks for \"{w}\" over personal data"));
        return Classification {
            intent: Intent::Report,
            remote_actions,
            reasons,
        };
    }
    if personal_values >= 10 || (looks_tabular(&ctx.instruction) && personal_values >= 3) {
        reasons.push(format!("{personal_values} personal values in bulk"));
        return Classification {
            intent: Intent::Report,
            remote_actions,
            reasons,
        };
    }

    let code_signals = [
        "```",
        "fn ",
        "def ",
        "function ",
        "class ",
        "import ",
        "traceback",
        "stack trace",
        "exception",
        "error[",
        "panicked at",
        "=> {",
        "#include",
    ];
    if code_signals.iter().any(|s| lower.contains(s)) {
        reasons.push("contains code".into());
        return Classification {
            intent: Intent::Code,
            remote_actions,
            reasons,
        };
    }
    Classification {
        intent: Intent::Converse,
        remote_actions,
        reasons,
    }
}

/// Whether a finding is something an action needs as it is.
///
/// When a request is meant to reach a person (an email, letter, message, call, invite or
/// payment), the model has to see who it is for: a stand-in recipient makes it address,
/// research or act on the wrong person. So the recipient (`in_recipient`: the value is in a
/// [`recipient_phrases`] span) is a target: name, contact details, company, address. The rest
/// of the request, such as other people, health data, IDs, cards and secrets, is not.
///
/// When the provider carries out the action itself (a hosted MCP server, a server-side tool),
/// Shadow never sees the result, so more is a target: the people, companies, addresses and
/// contact details named anywhere in the instruction, and, for a follow-up that names no
/// email or phone ("now send her the invite"), emails, phones and people anywhere in the
/// request. `instruction_has_contact` says whether the instruction names an email or phone.
pub fn is_action_target(
    c: &Classification,
    in_instruction: bool,
    in_recipient: bool,
    instruction_has_contact: bool,
    kind: Kind,
) -> bool {
    if c.intent != Intent::Act {
        return false;
    }
    let contact = matches!(kind, Kind::Email | Kind::Phone);
    let about = contact
        || matches!(
            kind,
            Kind::Person | Kind::Org | Kind::Address | Kind::Iban | Kind::Username
        );
    if in_recipient && about {
        return true;
    }
    if !c.remote_actions {
        return false;
    }
    (in_instruction && about) || (!instruction_has_contact && (contact || kind == Kind::Person))
}

/// The treatment for one value.
pub fn treatment(cfg: &PolicyConfig, c: &Classification, kind: Kind, target: bool) -> Treatment {
    if let Some(t) = cfg.always.get(kind.as_str()) {
        return *t;
    }
    if !cfg.intent {
        return Treatment::Surrogate;
    }
    match c.intent {
        Intent::Act if target => {
            if c.remote_actions {
                cfg.act.remote_tool_targets
            } else {
                cfg.act.targets
            }
        }
        Intent::Report => cfg
            .report
            .get(kind.as_str())
            .or_else(|| cfg.report.get("default"))
            .copied()
            .unwrap_or(Treatment::Surrogate),
        _ => Treatment::Surrogate,
    }
}

/// Counts personal values among findings (for classification).
pub fn personal(findings: &[Finding]) -> usize {
    findings
        .iter()
        .filter(|f| {
            matches!(
                f.kind,
                Kind::Person
                    | Kind::Email
                    | Kind::Phone
                    | Kind::Address
                    | Kind::NationalId
                    | Kind::Iban
                    | Kind::CreditCard
                    | Kind::Date
            )
        })
        .count()
}

// ------------------------------------------------------------------ provider request shapes

fn text_of(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(Value::as_str).or_else(|| p.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// The latest user instruction and the tools on offer, from an OpenAI (Chat Completions or
/// Responses), Anthropic Messages or Gemini request body.
pub fn context_from_json(v: &Value) -> RequestContext {
    let mut ctx = RequestContext::default();
    // Latest user message.
    if let Some(msgs) = v.get("messages").and_then(Value::as_array) {
        if let Some(m) = msgs.iter().rev().find(|m| m["role"] == "user") {
            ctx.instruction = text_of(&m["content"]);
        }
    } else if let Some(input) = v.get("input") {
        ctx.instruction = match input {
            Value::String(s) => s.clone(),
            Value::Array(items) => items
                .iter()
                .rev()
                .find(|i| i["role"] == "user" || i.is_string())
                .map(|i| {
                    if let Some(s) = i.as_str() {
                        s.to_string()
                    } else {
                        text_of(&i["content"])
                    }
                })
                .unwrap_or_default(),
            _ => String::new(),
        };
    } else if let Some(contents) = v.get("contents").and_then(Value::as_array) {
        if let Some(c) = contents
            .iter()
            .rev()
            .find(|c| c["role"] == "user" || c.get("role").is_none())
        {
            ctx.instruction = text_of(&c["parts"]);
        }
    }
    // Tools, and where they run.
    if let Some(tools) = v.get("tools").and_then(Value::as_array) {
        for t in tools {
            let ty = t["type"].as_str().unwrap_or("");
            if let Some(decls) = t
                .get("functionDeclarations")
                .or_else(|| t.get("function_declarations"))
                .and_then(Value::as_array)
            {
                for d in decls {
                    ctx.tools.push(ToolInfo {
                        name: d["name"].as_str().unwrap_or("function").into(),
                        remote: false,
                    });
                }
                continue;
            }
            let name = t["name"]
                .as_str()
                .or_else(|| t.pointer("/function/name").and_then(Value::as_str))
                .unwrap_or(ty)
                .to_string();
            // Client-executed: plain functions, Anthropic custom tools, computer/bash/editor tools,
            // OpenAI local_shell. Everything else (web search, code execution, file search, hosted MCP,
            // Gemini googleSearch/codeExecution) runs in the provider's cloud.
            let local = ty.is_empty()
                || ty == "function"
                || ty == "custom"
                || ty == "local_shell"
                || ty.starts_with("computer")
                || ty.starts_with("bash")
                || ty.starts_with("text_editor");
            let gemini_remote = t.get("googleSearch").is_some()
                || t.get("google_search").is_some()
                || t.get("codeExecution").is_some()
                || t.get("url_context").is_some();
            ctx.tools.push(ToolInfo {
                name,
                remote: !local || gemini_remote,
            });
        }
    }
    if v.get("mcp_servers")
        .and_then(Value::as_array)
        .is_some_and(|a| !a.is_empty())
    {
        ctx.tools.push(ToolInfo {
            name: "mcp_servers".into(),
            remote: true,
        });
    }
    ctx
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx(s: &str) -> RequestContext {
        RequestContext {
            instruction: s.into(),
            tools: vec![],
        }
    }

    #[test]
    fn classifies_instructions() {
        assert_eq!(
            classify(&ctx("Email jane@acme.com about the renewal"), 1).intent,
            Intent::Act
        );
        assert_eq!(
            classify(&ctx("Could you please send Jo the contract?"), 1).intent,
            Intent::Act
        );
        assert_eq!(
            classify(&ctx("Create a ticket for the outage"), 0).intent,
            Intent::Act
        );
        assert_eq!(
            classify(&ctx("Build a churn report from these customers"), 2).intent,
            Intent::Report
        );
        assert_eq!(
            classify(&ctx("Summarise this list"), 12).intent,
            Intent::Report
        );
        let table = "name,email,plan\na,a@x.com,pro\nb,b@x.com,pro\nc,c@x.com,free\nd,d@x.com,pro\ne,e@x.com,pro";
        assert_eq!(classify(&ctx(table), 5).intent, Intent::Report);
        assert_eq!(
            classify(&ctx("Why does this panic?\n```rust\nfn main() {}\n```"), 0).intent,
            Intent::Code
        );
        assert_eq!(
            classify(&ctx("What did Jane say about the email I got?"), 1).intent,
            Intent::Converse,
            "mentioning email isn't sending one"
        );
        for asks in [
            "I need you to email Jane the contract",
            "Write a short note to Jane and send it",
            "Check the calendar, then book a room for Jane",
            "Hey, can you remind Jane about Friday?",
            "Draft a renewal reminder to Jane Doe",
            "Write a letter to Mr Okafor about his results",
            "Draft an apology to Jane for the delay",
        ] {
            assert_eq!(classify(&ctx(asks), 1).intent, Intent::Act, "{asks}");
        }
        for mentions in [
            "Write a function that parses emails",
            "Write a function to update Jane's record",
            "Summarise the email Jane sent me",
            "How do I send a POST request in Python?",
        ] {
            assert_ne!(
                classify(&ctx(mentions), 1).intent,
                Intent::Act,
                "{mentions}"
            );
        }
    }

    #[test]
    fn finds_who_a_message_is_for() {
        assert_eq!(
            recipient_phrases("Draft a renewal reminder to Jane Doe (jane@acme.com) at Acme GmbH. Their plan renews in June."),
            ["Jane Doe (jane@acme.com) at Acme GmbH"]
        );
        assert_eq!(
            recipient_phrases("Email jane.doe@acme.com that Max approved it"),
            ["jane.doe@acme.com"]
        );
        assert_eq!(
            recipient_phrases("Please write a letter to Mr Okafor about his MRI results"),
            ["Mr Okafor"]
        );
        assert!(recipient_phrases("Summarise Jane's ticket").is_empty());
    }

    #[test]
    fn treatments_follow_policy() {
        let p = PolicyConfig::default();
        let act = Classification {
            intent: Intent::Act,
            remote_actions: false,
            reasons: vec![],
        };
        let act_remote = Classification {
            remote_actions: true,
            ..act.clone()
        };
        let report = Classification {
            intent: Intent::Report,
            ..act.clone()
        };
        assert_eq!(treatment(&p, &act, Kind::Email, true), Treatment::Keep);
        assert_eq!(
            treatment(&p, &act_remote, Kind::Email, true),
            Treatment::Keep
        );
        assert_eq!(
            treatment(&p, &act_remote, Kind::Email, false),
            Treatment::Surrogate,
            "only targets are kept"
        );
        assert_eq!(
            treatment(&p, &report, Kind::Person, false),
            Treatment::Token
        );
        assert_eq!(
            treatment(&p, &report, Kind::Hostname, false),
            Treatment::Surrogate
        );
        let strict = PolicyConfig {
            always: [("secret".to_string(), Treatment::Redact)]
                .into_iter()
                .collect(),
            ..PolicyConfig::default()
        };
        assert_eq!(
            treatment(&strict, &act_remote, Kind::Secret, true),
            Treatment::Redact,
            "always wins over intent"
        );
        let off = PolicyConfig {
            intent: false,
            ..PolicyConfig::default()
        };
        assert_eq!(
            treatment(&off, &report, Kind::Person, false),
            Treatment::Surrogate
        );
    }

    #[test]
    fn reads_provider_shapes() {
        let openai = json!({ "messages": [{ "role": "system", "content": "x" }, { "role": "user", "content": [{ "type": "text", "text": "Email Jo" }] }],
                             "tools": [{ "type": "function", "function": { "name": "send_email" } }] });
        let c = context_from_json(&openai);
        assert_eq!(
            (c.instruction.as_str(), c.tools[0].remote),
            ("Email Jo", false)
        );
        let responses = json!({ "input": [{ "role": "user", "content": [{ "type": "input_text", "text": "Book a room" }] }],
                                "tools": [{ "type": "mcp", "server_label": "cal", "server_url": "https://mcp.example.com" }] });
        let c = context_from_json(&responses);
        assert_eq!(
            (c.instruction.as_str(), c.tools[0].remote),
            ("Book a room", true)
        );
        let anthropic = json!({ "messages": [{ "role": "user", "content": "Ping the team" }], "tools": [{ "name": "slack", "input_schema": {} }, { "type": "web_search_20250305", "name": "web_search" }] });
        let c = context_from_json(&anthropic);
        assert_eq!(
            c.tools.iter().map(|t| t.remote).collect::<Vec<_>>(),
            [false, true]
        );
        let gemini = json!({ "contents": [{ "role": "user", "parts": [{ "text": "Call Ana" }] }], "tools": [{ "functionDeclarations": [{ "name": "dial" }] }] });
        let c = context_from_json(&gemini);
        assert_eq!(
            (c.instruction.as_str(), c.tools[0].name.as_str()),
            ("Call Ana", "dial")
        );
    }
}
