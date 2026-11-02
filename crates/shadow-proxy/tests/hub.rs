//! Enrolment with a Shadow Hub and uploads, against a mock hub: credentials stored
//! privately, events sent once, and incident details trimmed on this machine to the
//! organisation's sharing level before they leave it.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use shadow_core::activity::{Action, ActivityLog, Detail, Event, TextPart};
use shadow_core::Change;
use shadow_proxy::hub::{enroll, upload_pending, Enrollment};

#[derive(Default)]
struct Mock {
    level: String,
    enrol: Option<Value>,
    batches: Vec<Value>,
}
type Shared = Arc<Mutex<Mock>>;

async fn mock_enroll(State(m): State<Shared>, Json(b): Json<Value>) -> Json<Value> {
    let mut m = m.lock().unwrap();
    m.enrol = Some(b);
    Json(
        json!({"device_id": "d1", "device_token": "dev_secret", "org": {"name": "Acme", "share_level": m.level},
                "user": {"id": "u1", "email": "jane@acme.com", "name": "Jane"}}),
    )
}

async fn mock_ingest(
    State(m): State<Shared>,
    headers: axum::http::HeaderMap,
    Json(b): Json<Value>,
) -> Json<Value> {
    assert_eq!(headers["authorization"], "Bearer dev_secret");
    let mut m = m.lock().unwrap();
    m.batches.push(b);
    Json(json!({"accepted": 1, "share_level": m.level}))
}

fn incident(log: &ActivityLog) -> u64 {
    let mut e = Event {
        provider: "openai".into(),
        client: "Claude Code".into(),
        action: Action::Masked,
        ..Event::default()
    };
    e.kinds.insert("email".into(), 1);
    e.detail = Some(Box::new(Detail {
        parts: vec![TextPart {
            role: Some("user".into()),
            field: "messages[0].content".into(),
            original: "mail jane@acme.com".into(),
            sent: Some("mail mei@lin.cn".into()),
        }],
        changes: vec![Change {
            kind: "email".into(),
            detector: "email".into(),
            original: "jane@acme.com".into(),
            sent: "mei@lin.cn".into(),
        }],
        ..Detail::default()
    }));
    log.record(e)
}

#[tokio::test]
async fn enrol_then_upload_at_each_sharing_level() {
    let mock: Shared = Arc::new(Mutex::new(Mock {
        level: "full".into(),
        ..Mock::default()
    }));
    let app = Router::new()
        .route("/api/enroll", post(mock_enroll))
        .route("/api/ingest", post(mock_ingest))
        .with_state(mock.clone());
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let hub = format!("http://{}", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });

    let dir = tempfile::tempdir().unwrap();
    let en = enroll(
        &format!("{hub}/join/inv_abc"),
        dir.path(),
        Some("jane@acme.com"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(en.org_name, "Acme");
    assert_eq!(
        mock.lock().unwrap().enrol.as_ref().unwrap()["token"],
        "inv_abc"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(Enrollment::path(dir.path()))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    let log = ActivityLog::in_memory(1);
    let client = reqwest::Client::new();
    // Activity from before the machine joined is never sent.
    log.record(Event {
        ts: 1,
        ..Event::default()
    });
    let id = incident(&log);
    log.record(Event::default());
    assert_eq!(upload_pending(&client, &log, dir.path()).await.unwrap(), 2);
    assert_eq!(
        upload_pending(&client, &log, dir.path()).await.unwrap(),
        0,
        "sent once"
    );
    {
        let m = mock.lock().unwrap();
        let first = &m.batches[0]["events"][0];
        assert_eq!(first["local_id"], id);
        assert_eq!(m.batches[0]["events"].as_array().unwrap().len(), 2);
        assert!(first.get("id").is_none());
        assert_eq!(
            first["detail"]["parts"][0]["original"],
            "mail jane@acme.com"
        );
    }

    // The organisation lowers its sharing level; the device follows it.
    mock.lock().unwrap().level = "sanitised".into();
    incident(&log);
    upload_pending(&client, &log, dir.path()).await.unwrap();
    // This upload was still made at "full" (the level is learnt from the reply), so the
    // next one is the first at "sanitised".
    incident(&log);
    upload_pending(&client, &log, dir.path()).await.unwrap();
    {
        let m = mock.lock().unwrap();
        let text = m.batches.last().unwrap().to_string();
        assert!(!text.contains("jane@acme.com"), "{text}");
        assert!(text.contains("mei@lin.cn"));
    }
    mock.lock().unwrap().level = "counts".into();
    incident(&log);
    upload_pending(&client, &log, dir.path()).await.unwrap();
    incident(&log);
    upload_pending(&client, &log, dir.path()).await.unwrap();
    let m = mock.lock().unwrap();
    let last = &m.batches.last().unwrap()["events"][0];
    assert!(last.get("detail").is_none(), "{last}");
    assert_eq!(last["kinds"]["email"], 1);
}

#[tokio::test]
async fn not_enrolled_sends_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let log = ActivityLog::in_memory(1);
    log.record(Event::default());
    let n = upload_pending(&reqwest::Client::new(), &log, dir.path())
        .await
        .unwrap();
    assert_eq!(n, 0);
}
