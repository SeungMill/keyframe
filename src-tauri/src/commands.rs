use crate::{driver, hook, hotkeys, model::*, player, procs, recorder, state::*};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    macros: Vec<Macro>,
    settings: Settings,
    recording: RecordingPayload,
    playing: Vec<String>,
    selected: Option<String>,
}

#[tauri::command]
pub async fn get_state(app: AppHandle) -> Snapshot {
    let st = app.state::<AppState>();
    let recording = {
        let r = st.recorder.lock();
        RecordingPayload { state: r.state, countdown: 0, macro_id: r.target.clone() }
    };
    let snap = Snapshot {
        macros: st.macros.lock().clone(),
        settings: st.settings.lock().clone(),
        recording,
        playing: st.playback.lock().keys().cloned().collect(),
        selected: st.selected.lock().clone(),
    };
    snap
}

#[tauri::command]
pub async fn create_macro(app: AppHandle, name: Option<String>) -> Macro {
    let st = app.state::<AppState>();
    let m = {
        let mut ms = st.macros.lock();
        let name = name.unwrap_or_else(|| format!("Macro {}", ms.len() + 1));
        let m = Macro::new(name);
        ms.push(m.clone());
        m
    };
    persist(&app);
    m
}

#[tauri::command]
pub async fn save_macro(app: AppHandle, data: Macro) {
    let st = app.state::<AppState>();
    let hotkeys_changed = {
        let mut ms = st.macros.lock();
        match ms.iter_mut().find(|x| x.id == data.id) {
            Some(slot) => {
                let changed = slot.trigger != data.trigger || slot.enabled != data.enabled;
                *slot = data;
                changed
            }
            None => {
                ms.push(data);
                true
            }
        }
    };
    persist(&app);
    if hotkeys_changed {
        hotkeys::rebuild(&app);
    }
}

#[tauri::command]
pub async fn delete_macro(app: AppHandle, id: String) {
    player::stop(&app, &id);
    let st = app.state::<AppState>();
    st.macros.lock().retain(|m| m.id != id);
    {
        let mut sel = st.selected.lock();
        if sel.as_deref() == Some(id.as_str()) {
            *sel = None;
        }
    }
    persist(&app);
    hotkeys::rebuild(&app);
}

#[tauri::command]
pub async fn select_macro(app: AppHandle, id: Option<String>) {
    *app.state::<AppState>().selected.lock() = id;
}

#[tauri::command]
pub async fn update_settings(app: AppHandle, settings: Settings) {
    let st = app.state::<AppState>();
    let launch = settings.launch_at_login;
    let (hotkey_changed, launch_changed) = {
        let mut cur = st.settings.lock();
        let changed = (cur.record_hotkey != settings.record_hotkey, cur.launch_at_login != launch);
        *cur = settings;
        changed
    };
    persist(&app);
    if hotkey_changed {
        hotkeys::rebuild(&app);
    }
    if launch_changed {
        sync_autostart(&app, launch, true);
    }
}

#[tauri::command]
pub async fn start_recording(app: AppHandle, macro_id: Option<String>) -> Result<(), String> {
    recorder::start(&app, macro_id)
}

#[tauri::command]
pub async fn stop_recording(app: AppHandle) {
    recorder::stop(&app, recorder::StopSource::Ui);
}

#[tauri::command]
pub async fn play_macro(app: AppHandle, id: String) -> Result<(), String> {
    let m = app
        .state::<AppState>()
        .macros
        .lock()
        .iter()
        .find(|m| m.id == id)
        .cloned()
        .ok_or("Macro not found.")?;
    player::start(&app, m.clone(), player::passes_for(&m))
}

#[tauri::command]
pub async fn stop_playback(app: AppHandle, id: Option<String>) {
    match id {
        Some(id) => player::stop(&app, &id),
        None => player::stop_all(&app),
    }
}

#[tauri::command]
pub async fn start_capture(app: AppHandle, kind: String) -> Result<(), String> {
    let kind = match kind.as_str() {
        "trigger" => CaptureKind::Trigger,
        "key" => CaptureKind::Key,
        other => return Err(format!("Unknown capture kind '{other}'.")),
    };
    let st = app.state::<AppState>();
    if matches!(st.recorder.lock().state, RecState::Countdown | RecState::Recording) {
        return Err("Stop recording first.".into());
    }
    *st.capture.lock() =
        Some(Capture { kind, ctrl: false, shift: false, alt: false, sup: false });
    hook::sync_key_polling(&app);
    hotkeys::rebuild(&app);
    Ok(())
}

#[tauri::command]
pub async fn cancel_capture(app: AppHandle) {
    hook::finish_capture(&app, CapturePayload::Cancelled);
}

fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect();
    let s = s.trim().to_string();
    if s.is_empty() { "macro".into() } else { s }
}

#[tauri::command]
pub async fn export_macros(app: AppHandle, ids: Option<Vec<String>>) -> Result<bool, String> {
    let list: Vec<Macro> = {
        let st = app.state::<AppState>();
        let ms = st.macros.lock();
        match &ids {
            Some(ids) => ms.iter().filter(|m| ids.contains(&m.id)).cloned().collect(),
            None => ms.clone(),
        }
    };
    if list.is_empty() {
        return Err("Nothing to export.".into());
    }
    let file_name = if list.len() == 1 {
        format!("{}.json", sanitize(&list[0].name))
    } else {
        "keyframe-macros.json".to_string()
    };
    let Some(picked) = app
        .dialog()
        .file()
        .add_filter("Keyframe macros", &["json"])
        .set_file_name(file_name)
        .blocking_save_file()
    else {
        return Ok(false);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let doc = serde_json::json!({ "format": "keyframe", "version": 1, "macros": list });
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn import_macros(app: AppHandle) -> Result<usize, String> {
    let Some(picked) = app
        .dialog()
        .file()
        .add_filter("Keyframe macros", &["json"])
        .blocking_pick_file()
    else {
        return Ok(0);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Not valid JSON: {e}"))?;
    let items: Vec<serde_json::Value> = match value {
        serde_json::Value::Array(a) => a,
        serde_json::Value::Object(mut o) => match o.remove("macros") {
            Some(serde_json::Value::Array(a)) => a,
            _ => vec![serde_json::Value::Object(o)],
        },
        _ => return Err("Unrecognised macro file.".into()),
    };

    let mut imported: Vec<Macro> = vec![];
    let mut risky = false;
    for item in items {
        let mut m: Macro =
            serde_json::from_value(item).map_err(|e| format!("Invalid macro in file: {e}"))?;
        m.id = new_id();
        for e in &mut m.events {
            e.id = new_id();
        }
        if m.events.iter().any(|e| matches!(e.kind, EventKind::RunProgram { .. })) {
            m.enabled = false;
            risky = true;
        }
        imported.push(m);
    }
    let count = imported.len();
    if count == 0 {
        return Ok(0);
    }
    app.state::<AppState>().macros.lock().extend(imported);
    persist(&app);
    hotkeys::rebuild(&app);
    emit_macros(&app);
    if risky {
        toast(
            &app,
            "warn",
            "Some imported macros contain Run Program actions and were disabled. Review them, then enable.",
        );
    }
    Ok(count)
}

#[tauri::command]
pub async fn reorder_macros(app: AppHandle, ids: Vec<String>) {
    {
        let st = app.state::<AppState>();
        let mut ms = st.macros.lock();
        ms.sort_by_key(|m| ids.iter().position(|i| i == &m.id).unwrap_or(usize::MAX));
    }
    persist(&app);
}

#[tauri::command]
pub async fn window_minimize(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.minimize();
    }
}

#[tauri::command]
pub async fn window_toggle_maximize(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_maximized().unwrap_or(false) {
            let _ = w.unmaximize();
        } else {
            let _ = w.maximize();
        }
    }
}

#[tauri::command]
pub async fn window_close(app: AppHandle) {
    let to_tray = app.state::<AppState>().settings.lock().close_to_tray;
    if to_tray {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
    } else {
        player::stop_all(&app);
        app.exit(0);
    }
}

pub fn sync_autostart(app: &AppHandle, on: bool, notify: bool) {
    use tauri_plugin_autostart::ManagerExt;
    if cfg!(debug_assertions) && !notify {
        return;
    }
    let launcher = app.autolaunch();
    let res = if on { launcher.enable() } else { launcher.disable() };
    if let Err(e) = res {
        if notify {
            toast(app, "error", format!("Could not change start-with-Windows: {e}"));
        }
    }
}

#[tauri::command]
pub async fn driver_status(app: AppHandle) -> driver::DriverStatus {
    driver::status(&app)
}

#[tauri::command]
pub async fn driver_install(app: AppHandle, uninstall: bool) -> Result<String, String> {
    let a = app.clone();
    let flag = if uninstall { "/uninstall" } else { "/install" };
    tauri::async_runtime::spawn_blocking(move || driver::run_installer(&a, flag))
        .await
        .map_err(|e| e.to_string())??;
    let msg = if uninstall { "Driver removed. Restart Windows to finish." } else { "Driver installed. Restart Windows to activate it." };
    Ok(msg.to_string())
}

#[tauri::command]
pub async fn list_processes() -> Vec<procs::ProcInfo> {
    procs::list_windows()
}
