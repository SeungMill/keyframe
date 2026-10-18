use crate::{player, recorder, state::*};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub fn rebuild(app: &AppHandle) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let st = app.state::<AppState>();
    st.held.lock().clear();
    if st.capture.lock().is_some() {
        return;
    }

    let hotkey = st.settings.lock().record_hotkey.clone();
    if let Err(e) = gs.on_shortcut(hotkey.as_str(), |app, _s, ev| {
        if ev.state() == ShortcutState::Pressed {
            recorder::toggle(app, recorder::StopSource::Hotkey);
        }
    }) {
        toast(app, "error", format!("Record hotkey '{hotkey}' is unavailable: {e}"));
    }

    let fg = crate::procs::foreground_exe();
    let cfg = st.settings.lock().clone();
    let triggers: Vec<(String, String, String)> = st
        .macros
        .lock()
        .iter()
        .filter(|m| is_armed(m, &cfg) && crate::procs::scope_ok(m.process.as_deref(), fg.as_deref()))
        .filter_map(|m| match &m.trigger {
            Some(crate::model::Trigger::Keyboard { accelerator }) => {
                Some((m.id.clone(), m.name.clone(), accelerator.clone()))
            }
            _ => None,
        })
        .collect();

    for (id, name, accel) in triggers {
        let res = gs.on_shortcut(accel.as_str(), move |app, _s, ev| {
            player::trigger(app, &id, ev.state() == ShortcutState::Pressed);
        });
        if let Err(e) = res {
            toast(app, "error", format!("Trigger '{accel}' for \"{name}\" could not be registered: {e}"));
        }
    }
}

fn signature(app: &AppHandle) -> String {
    let fg = crate::procs::foreground_exe();
    let st = app.state::<AppState>();
    let cfg = st.settings.lock().clone();
    let ms = st.macros.lock();
    ms.iter()
        .filter(|m| is_armed(m, &cfg) && matches!(m.trigger, Some(crate::model::Trigger::Keyboard { .. })))
        .filter(|m| crate::procs::scope_ok(m.process.as_deref(), fg.as_deref()))
        .map(|m| m.id.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

pub fn start_watcher(app: AppHandle) {
    let _ = std::thread::Builder::new().name("scope-watch".into()).spawn(move || {
        let mut last = signature(&app);
        loop {
            std::thread::sleep(Duration::from_millis(300));
            let sig = signature(&app);
            if sig != last {
                last = sig;
                rebuild(&app);
            }
        }
    });
}
