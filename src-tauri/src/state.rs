use crate::model::*;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{atomic::AtomicBool, Arc};
use std::time::{Duration, Instant, SystemTime};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;

pub const STORE_FILE: &str = "macros.json";

#[derive(Default)]
pub struct Recorder {
    pub state: RecState,
    pub target: Option<String>,
    pub generation: u64,
    pub events: Vec<MacroEvent>,
    pub down_keys: HashSet<String>,
    pub last_time: Option<SystemTime>,
    pub last_pos: Option<(f64, f64)>,
    pub pending: Option<(f64, f64, SystemTime)>,
    pub last_move_emit: Option<SystemTime>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CaptureKind {
    Trigger,
    Key,
    Input,
}

#[derive(Default)]
pub struct Prio {
    pub held: HashSet<String>,
    pub generation: u64,
}

pub struct Capture {
    pub kind: CaptureKind,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub sup: bool,
}

#[derive(Default)]
pub struct AppState {
    pub macros: Mutex<Vec<Macro>>,
    pub settings: Mutex<Settings>,
    pub selected: Mutex<Option<String>>,
    pub recorder: Mutex<Recorder>,
    pub playback: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub capture: Mutex<Option<Capture>>,
    pub held: Mutex<HashSet<String>>,
    pub inject: Mutex<VecDeque<(String, bool, Instant)>>,
    pub paused: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub prio: Mutex<HashMap<String, Prio>>,
    pub keys_out: Mutex<HashMap<String, (bool, Instant)>>,
}

impl AppState {
    pub fn note_inject(&self, button: &str, down: bool) {
        let mut q = self.inject.lock();
        q.push_back((button.to_string(), down, Instant::now()));
        while q.len() > 64 {
            q.pop_front();
        }
    }

    pub fn note_key_out(&self, key: &str, down: bool) {
        self.keys_out.lock().insert(key.to_string(), (down, Instant::now()));
    }

    pub fn is_key_ours(&self, key: &str) -> bool {
        self.keys_out
            .lock()
            .get(key)
            .map(|(down, at)| *down || at.elapsed() < Duration::from_millis(80))
            .unwrap_or(false)
    }

    pub fn consume_inject(&self, button: &str, down: bool) -> bool {
        let mut q = self.inject.lock();
        let now = Instant::now();
        while let Some(front) = q.front() {
            if now.duration_since(front.2) > Duration::from_millis(300) {
                q.pop_front();
            } else {
                break;
            }
        }
        if let Some(pos) = q.iter().position(|e| e.0 == button && e.1 == down) {
            q.remove(pos);
            true
        } else {
            false
        }
    }
}

fn migrate_old_data(app: &AppHandle) {
    let Ok(dir) = app.path().app_data_dir() else { return };
    let Some(parent) = dir.parent() else { return };
    let (new, old) = (dir.join(STORE_FILE), parent.join("com.macroforge.app").join(STORE_FILE));
    if !new.exists() && old.exists() {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::copy(&old, &new);
    }
}

pub fn load_into(app: &AppHandle) {
    migrate_old_data(app);
    let st = app.state::<AppState>();
    match app.store(STORE_FILE) {
        Ok(store) => {
            if let Some(v) = store.get("macros") {
                match serde_json::from_value::<Vec<Macro>>(v) {
                    Ok(m) => *st.macros.lock() = m,
                    Err(e) => eprintln!("could not parse stored macros: {e}"),
                }
            }
            if let Some(v) = store.get("settings") {
                if let Ok(s) = serde_json::from_value::<Settings>(v) {
                    *st.settings.lock() = s;
                }
            }
        }
        Err(e) => eprintln!("could not open store: {e}"),
    }
}

pub fn persist(app: &AppHandle) {
    let st = app.state::<AppState>();
    let macros = st.macros.lock().clone();
    let settings = st.settings.lock().clone();
    match app.store(STORE_FILE) {
        Ok(store) => {
            store.set("macros", serde_json::to_value(&macros).unwrap_or_default());
            store.set("settings", serde_json::to_value(&settings).unwrap_or_default());
            if let Err(e) = store.save() {
                eprintln!("store save failed: {e}");
            }
        }
        Err(e) => eprintln!("could not open store: {e}"),
    }
}

pub fn toast(app: &AppHandle, level: &str, message: impl Into<String>) {
    let _ = app.emit(
        "toast",
        ToastPayload { level: level.to_string(), message: message.into() },
    );
}

pub fn emit_macros(app: &AppHandle) {
    let list = app.state::<AppState>().macros.lock().clone();
    let _ = app.emit("macros-changed", list);
}
