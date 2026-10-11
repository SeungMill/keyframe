use crate::{keys, model::*, state::*, util};
use crate::input::{InputEvent, Kind};
use std::{collections::HashSet, time::{Duration, SystemTime}};
use tauri::{AppHandle, Emitter, Manager};

const MOVE_INTERVAL: Duration = Duration::from_millis(8);

#[derive(Clone, Copy, PartialEq)]
pub enum StopSource {
    Hotkey,
    Ui,
}

fn emit_state(app: &AppHandle, state: RecState, countdown: u8, macro_id: Option<String>) {
    let _ = app.emit("recording-state", RecordingPayload { state, countdown, macro_id });
}

pub fn toggle(app: &AppHandle, src: StopSource) {
    let state = app.state::<AppState>().recorder.lock().state;
    match state {
        RecState::Countdown | RecState::Recording => stop(app, src),
        _ => {
            if let Err(e) = start(app, None) {
                toast(app, "error", e);
            }
        }
    }
}

pub fn start(app: &AppHandle, macro_id: Option<String>) -> Result<(), String> {
    let st = app.state::<AppState>();
    if !st.playback.lock().is_empty() {
        return Err("Stop macro playback before recording.".into());
    }
    if st.capture.lock().is_some() {
        return Err("Finish the key capture first.".into());
    }
    let selected = st.selected.lock().clone();
    let (generation, target) = {
        let mut r = st.recorder.lock();
        if matches!(r.state, RecState::Countdown | RecState::Recording) {
            return Err("Already recording.".into());
        }
        r.generation += 1;
        r.state = RecState::Countdown;
        r.target = macro_id.or(selected);
        r.events.clear();
        r.down_keys.clear();
        r.last_time = None;
        r.last_pos = None;
        r.pending = None;
        r.last_move_emit = None;
        (r.generation, r.target.clone())
    };

    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let st = app2.state::<AppState>();
        for n in (1..=3u8).rev() {
            {
                let r = st.recorder.lock();
                if r.generation != generation || r.state != RecState::Countdown {
                    return;
                }
            }
            emit_state(&app2, RecState::Countdown, n, target.clone());
            util::beep(700, 80);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        {
            let mut r = st.recorder.lock();
            if r.generation != generation || r.state != RecState::Countdown {
                return;
            }
            r.state = RecState::Recording;
        }
        crate::hook::sync_key_polling(&app2);
        util::beep(1200, 180);
        emit_state(&app2, RecState::Recording, 0, target);
    });
    Ok(())
}

pub fn stop(app: &AppHandle, src: StopSource) {
    let st = app.state::<AppState>();
    let settings = st.settings.lock().clone();

    enum Outcome {
        Cancelled,
        Finished(Vec<MacroEvent>, Option<String>),
    }
    let outcome = {
        let mut r = st.recorder.lock();
        match r.state {
            RecState::Countdown => {
                r.generation += 1;
                r.state = RecState::Idle;
                Outcome::Cancelled
            }
            RecState::Recording => {
                flush(app, &mut r, &settings);
                r.state = RecState::Stopped;
                Outcome::Finished(std::mem::take(&mut r.events), r.target.clone())
            }
            _ => return,
        }
    };

    crate::hook::sync_key_polling(app);

    match outcome {
        Outcome::Cancelled => {
            util::beep(500, 120);
            emit_state(app, RecState::Idle, 0, None);
        }
        Outcome::Finished(mut events, target) => {
            trim(&mut events, src, &settings.record_hotkey);
            let id = commit(app, target, events);
            util::beep(500, 200);
            emit_state(app, RecState::Stopped, 0, id);
        }
    }
}

fn trim(events: &mut Vec<MacroEvent>, src: StopSource, hotkey: &str) {
    let hk: HashSet<String> = keys::hotkey_key_names(hotkey).into_iter().collect();
    while let Some(last) = events.last() {
        let drop_it = match (&last.kind, src) {
            (EventKind::KeyPress { key } | EventKind::KeyRelease { key }, StopSource::Hotkey) => {
                hk.contains(key)
            }
            (EventKind::MouseMove { .. }, _) => true,
            (EventKind::MouseDown { button } | EventKind::MouseUp { button }, StopSource::Ui) => {
                button.as_str() == "Left"
            }
            _ => false,
        };
        if drop_it {
            events.pop();
        } else {
            break;
        }
    }
}

fn commit(app: &AppHandle, target: Option<String>, events: Vec<MacroEvent>) -> Option<String> {
    if events.is_empty() {
        return target;
    }
    let st = app.state::<AppState>();
    let id = {
        let mut ms = st.macros.lock();
        let idx = target.as_ref().and_then(|t| ms.iter().position(|m| &m.id == t));
        match idx {
            Some(i) => {
                ms[i].events.extend(events);
                ms[i].id.clone()
            }
            None => {
                let mut m = Macro::new(format!("Macro {}", ms.len() + 1));
                m.events = events;
                let id = m.id.clone();
                ms.push(m);
                id
            }
        }
    };
    persist(app);
    emit_macros(app);
    Some(id)
}

fn push(app: &AppHandle, r: &mut Recorder, s: &Settings, kind: EventKind, t: SystemTime) {
    let delay_ms = match s.delay_mode {
        DelayMode::Record => r
            .last_time
            .map(|lt| t.duration_since(lt).unwrap_or_default().as_millis() as u64)
            .unwrap_or(0),
        DelayMode::Fixed => {
            if r.last_time.is_some() {
                s.fixed_delay_ms
            } else {
                0
            }
        }
        DelayMode::None => 0,
    };
    r.last_time = Some(t);
    let ev = MacroEvent { id: new_id(), delay_ms, kind };
    r.events.push(ev.clone());
    let _ = app.emit("recording-event", ev);
}

fn flush(app: &AppHandle, r: &mut Recorder, s: &Settings) {
    if let Some((x, y, t)) = r.pending.take() {
        let relative = s.mouse_mode == MouseMode::Relative;
        if relative && x == 0.0 && y == 0.0 {
            return;
        }
        r.last_move_emit = Some(t);
        push(app, r, s, EventKind::MouseMove { x: x.round() as i32, y: y.round() as i32, relative }, t);
    }
}

fn on_move(app: &AppHandle, r: &mut Recorder, s: &Settings, x: f64, y: f64, t: SystemTime) {
    let prev = r.last_pos.replace((x, y));
    match s.mouse_mode {
        MouseMode::None => return,
        MouseMode::Absolute => r.pending = Some((x, y, t)),
        MouseMode::Relative => {
            if let Some((px, py)) = prev {
                let (ax, ay) = r.pending.map(|p| (p.0, p.1)).unwrap_or((0.0, 0.0));
                r.pending = Some((ax + x - px, ay + y - py, t));
            }
        }
    }
    let due = match r.last_move_emit {
        None => true,
        Some(l) => t.duration_since(l).map(|d| d >= MOVE_INTERVAL).unwrap_or(false),
    };
    if due {
        flush(app, r, s);
    }
}

pub fn on_event(app: &AppHandle, st: &AppState, ev: &InputEvent) {
    if st.recorder.lock().state != RecState::Recording {
        return;
    }
    let settings = st.settings.lock().clone();
    let mut r = st.recorder.lock();
    if r.state != RecState::Recording {
        return;
    }
    let t = ev.time;
    let kind = match &ev.kind {
        Kind::Move { x, y } => {
            on_move(app, &mut r, &settings, *x as f64, *y as f64, t);
            return;
        }
        Kind::KeyDown(key) => {
            if !r.down_keys.insert(key.clone()) {
                return;
            }
            EventKind::KeyPress { key: key.clone() }
        }
        Kind::KeyUp(key) => {
            r.down_keys.remove(key);
            EventKind::KeyRelease { key: key.clone() }
        }
        Kind::ButtonDown(b) => EventKind::MouseDown { button: b.clone() },
        Kind::ButtonUp(b) => EventKind::MouseUp { button: b.clone() },
        Kind::Wheel { dx, dy } => EventKind::Scroll { dx: *dx, dy: *dy },
    };
    flush(app, &mut r, &settings);
    push(app, &mut r, &settings, kind, t);
}
