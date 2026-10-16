use crate::{input, keys, state::*};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::{thread, time::Duration};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PausedPayload {
    macro_id: String,
    paused: bool,
}

fn set_paused(app: &AppHandle, id: &str, paused: bool) {
    let flag = app.state::<AppState>().paused.lock().get(id).cloned();
    if let Some(flag) = flag {
        if flag.swap(paused, Ordering::SeqCst) != paused {
            let _ = app.emit("playback-paused", PausedPayload { macro_id: id.to_string(), paused });
        }
    }
}

pub fn on_input(app: &AppHandle, name: &str, down: bool) {
    let st = app.state::<AppState>();
    if st.is_key_ours(name) {
        return;
    }
    let running: Vec<String> = st.playback.lock().keys().cloned().collect();
    if running.is_empty() {
        return;
    }
    let targets: Vec<(String, u64)> = st
        .macros
        .lock()
        .iter()
        .filter(|m| running.contains(&m.id) && m.priority_keys.iter().any(|k| k == name))
        .map(|m| (m.id.clone(), m.priority_resume_ms))
        .collect();
    for (id, resume_ms) in targets {
        if down {
            press(app, &id, name);
        } else {
            release(app, &id, name, resume_ms);
        }
    }
}

fn press(app: &AppHandle, id: &str, name: &str) {
    {
        let st = app.state::<AppState>();
        let mut all = st.prio.lock();
        let p = all.entry(id.to_string()).or_default();
        p.held.insert(name.to_string());
        p.generation += 1;
    }
    set_paused(app, id, true);
}

fn release(app: &AppHandle, id: &str, name: &str, resume_ms: u64) {
    let generation = {
        let st = app.state::<AppState>();
        let mut all = st.prio.lock();
        let p = all.entry(id.to_string()).or_default();
        p.held.remove(name);
        if !p.held.is_empty() {
            return;
        }
        p.generation += 1;
        p.generation
    };
    let app2 = app.clone();
    let id2 = id.to_string();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(resume_ms));
        let still_idle = app2
            .state::<AppState>()
            .prio
            .lock()
            .get(&id2)
            .map(|p| p.generation == generation && p.held.is_empty())
            .unwrap_or(false);
        if still_idle {
            set_paused(&app2, &id2, false);
        }
    });
}

pub fn finish(app: &AppHandle, id: &str) {
    let st = app.state::<AppState>();
    st.prio.lock().remove(id);
    let flag = st.paused.lock().remove(id);
    if let Some(flag) = flag {
        if flag.swap(false, Ordering::SeqCst) {
            let _ = app.emit("playback-paused", PausedPayload { macro_id: id.to_string(), paused: false });
        }
    }
    sync_polling(app);
}

pub fn sync_polling(app: &AppHandle) {
    let st = app.state::<AppState>();
    let running: Vec<String> = st.playback.lock().keys().cloned().collect();
    let mut vks: Vec<u16> = vec![];
    for m in st.macros.lock().iter().filter(|m| running.contains(&m.id)) {
        for k in &m.priority_keys {
            if let Some(v) = keys::vk(k) {
                vks.push(v);
            }
        }
    }
    input::set_priority_vks(&vks);
}
