use crate::input::{self, InputEvent, Kind};
use crate::{keys, model::*, player, recorder, state::*};
use std::{sync::mpsc, thread};
use tauri::{AppHandle, Emitter, Manager};

pub fn start(app: AppHandle) {
    let (tx, rx) = mpsc::channel();
    input::start(tx);
    thread::Builder::new()
        .name("input-dispatch".into())
        .spawn(move || {
            for raw in rx {
                if let Some(ev) = input::convert(raw) {
                    dispatch(&app, ev);
                }
            }
        })
        .expect("spawn input dispatcher");
}

fn dispatch(app: &AppHandle, ev: InputEvent) {
    if ev.injected {
        return;
    }
    let st = app.state::<AppState>();

    match &ev.kind {
        Kind::ButtonDown(b) if st.consume_inject(b, true) => return,
        Kind::ButtonUp(b) if st.consume_inject(b, false) => return,
        _ => {}
    }

    if handle_capture(app, &st, &ev) {
        return;
    }

    recorder::on_event(app, &st, &ev);

    let (name, pressed) = match &ev.kind {
        Kind::ButtonDown(b) => (b.clone(), true),
        Kind::ButtonUp(b) => (b.clone(), false),
        _ => return,
    };
    if name == "Left" {
        return;
    }
    let ids: Vec<String> = st
        .macros
        .lock()
        .iter()
        .filter(|m| m.enabled && matches!(&m.trigger, Some(Trigger::Mouse { button }) if *button == name))
        .map(|m| m.id.clone())
        .collect();
    for id in ids {
        player::trigger(app, &id, pressed);
    }
}

fn set_mod(c: &mut Capture, m: keys::Mod, down: bool) {
    match m {
        keys::Mod::Ctrl => c.ctrl = down,
        keys::Mod::Shift => c.shift = down,
        keys::Mod::Alt => c.alt = down,
        keys::Mod::Super => c.sup = down,
    }
}

fn build_accel(c: &Capture, token: &str) -> String {
    let mut parts: Vec<&str> = vec![];
    if c.ctrl {
        parts.push("Ctrl");
    }
    if c.alt {
        parts.push("Alt");
    }
    if c.shift {
        parts.push("Shift");
    }
    if c.sup {
        parts.push("Super");
    }
    parts.push(token);
    parts.join("+")
}

fn handle_capture(app: &AppHandle, st: &AppState, ev: &InputEvent) -> bool {
    let result: CapturePayload;
    {
        let mut guard = st.capture.lock();
        let Some(c) = guard.as_mut() else { return false };
        match &ev.kind {
            Kind::KeyDown(name) => {
                if name == "Escape" {
                    result = CapturePayload::Cancelled;
                } else if let Some(m) = keys::modifier(name) {
                    set_mod(c, m, true);
                    return true;
                } else {
                    match c.kind {
                        CaptureKind::Key => result = CapturePayload::Key { key: name.clone() },
                        CaptureKind::Trigger => match keys::accel_key(name) {
                            Some(tok) => {
                                result = CapturePayload::Trigger {
                                    trigger: Trigger::Keyboard { accelerator: build_accel(c, &tok) },
                                }
                            }
                            None => return true,
                        },
                    }
                }
            }
            Kind::KeyUp(name) => {
                if let Some(m) = keys::modifier(name) {
                    set_mod(c, m, false);
                }
                return true;
            }
            Kind::ButtonDown(b) => {
                if c.kind == CaptureKind::Trigger && b != "Left" {
                    result = CapturePayload::Trigger { trigger: Trigger::Mouse { button: b.clone() } };
                } else {
                    return true;
                }
            }
            _ => return true,
        }
    }
    finish_capture(app, result);
    true
}

pub fn sync_key_polling(app: &AppHandle) {
    let st = app.state::<AppState>();
    let recording = st.recorder.lock().state == RecState::Recording;
    let capturing = st.capture.lock().is_some();
    input::set_keys_enabled(recording || capturing);
}

pub fn finish_capture(app: &AppHandle, payload: CapturePayload) {
    {
        *app.state::<AppState>().capture.lock() = None;
    }
    sync_key_polling(app);
    let _ = app.emit("capture-result", payload);
    crate::hotkeys::rebuild(app);
}
