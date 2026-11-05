use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rand::RngCore;
use shadow_core::Session;

/// Sessions kept between calls, so a protect call and the restore of its answer share the
/// same typed tokens. Persona fakes live in the vault and restore without a session.
pub struct SessionStore {
    ttl: Duration,
    max: usize,
    map: Mutex<HashMap<String, Entry>>,
}

struct Entry {
    session: Arc<Session>,
    used: Instant,
}

impl SessionStore {
    pub fn new(ttl: Duration, max: usize) -> Self {
        SessionStore {
            ttl,
            max: max.max(1),
            map: Mutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Entry>> {
        self.map.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Store a new session and return its id.
    pub fn insert(&self, session: Session) -> (String, Arc<Session>) {
        let mut id = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut id);
        let id = format!("ses_{}", hex(&id));
        let session = Arc::new(session);
        let mut map = self.lock();
        let now = Instant::now();
        map.retain(|_, e| now.duration_since(e.used) < self.ttl);
        while map.len() >= self.max {
            let Some(oldest) = map
                .iter()
                .min_by_key(|(_, e)| e.used)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            map.remove(&oldest);
        }
        map.insert(
            id.clone(),
            Entry {
                session: session.clone(),
                used: now,
            },
        );
        (id, session)
    }

    /// The session with this id, if it has not expired; marks it used.
    pub fn get(&self, id: &str) -> Option<Arc<Session>> {
        let mut map = self.lock();
        let now = Instant::now();
        let e = map.get_mut(id)?;
        if now.duration_since(e.used) >= self.ttl {
            map.remove(id);
            return None;
        }
        e.used = now;
        Some(e.session.clone())
    }

    pub fn remove(&self, id: &str) -> bool {
        self.lock().remove(id).is_some()
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
