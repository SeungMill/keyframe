mod commands;
mod driver;
mod hook;
mod hotkeys;
mod inject;
mod input;
mod keys;
mod model;
mod player;
mod priority;
mod procs;
mod recorder;
mod state;
mod tray;
mod util;

use tauri::{Emitter, Manager};

pub fn run() {
    util::timer_begin(1);

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::create_macro,
            commands::save_macro,
            commands::delete_macro,
            commands::select_macro,
            commands::update_settings,
            commands::start_recording,
            commands::stop_recording,
            commands::play_macro,
            commands::stop_playback,
            commands::start_capture,
            commands::cancel_capture,
            commands::export_macros,
            commands::import_macros,
            commands::reorder_macros,
            commands::window_minimize,
            commands::window_toggle_maximize,
            commands::window_close,
            commands::window_compact,
            commands::window_compact_size,
            commands::driver_status,
            commands::driver_install,
            commands::list_processes,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            state::load_into(&handle);
            tray::build(&handle)?;
            hotkeys::rebuild(&handle);
            hook::start(handle.clone());
            hotkeys::start_watcher(handle.clone());

            let s = handle.state::<state::AppState>().settings.lock().clone();
            commands::sync_autostart(&handle, s.launch_at_login, false);
            if s.compact {
                commands::apply_compact(&handle, true, 220.0);
            }
            if s.always_on_top {
                if let Some(w) = handle.get_webview_window("main") {
                    let _ = w.set_always_on_top(true);
                }
            }
            if !s.start_minimized {
                tray::show_main(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let app = window.app_handle();
                if app.state::<state::AppState>().settings.lock().close_to_tray {
                    let _ = window.hide();
                    api.prevent_close();
                } else {
                    player::stop_all(app);
                    state::persist(app);
                }
            }
            tauri::WindowEvent::Moved(pos) => {
                let app = window.app_handle();
                let st = app.state::<state::AppState>();
                let compact = {
                    let mut s = st.settings.lock();
                    if s.compact {
                        s.compact_x = Some(pos.x);
                        s.compact_y = Some(pos.y);
                    }
                    s.compact
                };
                if !compact {
                    if let Some(g) = st.full_geom.lock().as_mut() {
                        g.2 = pos.x;
                        g.3 = pos.y;
                    }
                }
            }
            tauri::WindowEvent::Resized(_) => {
                let maximized = window.is_maximized().unwrap_or(false);
                let _ = window.emit("window-state", maximized);
                let app = window.app_handle();
                let st = app.state::<state::AppState>();
                let compact = st.settings.lock().compact;
                if !compact && !maximized && !window.is_minimized().unwrap_or(false) {
                    if let (Ok(size), Ok(pos)) = (window.outer_size(), window.outer_position()) {
                        if size.width > 600 {
                            *st.full_geom.lock() = Some((size.width, size.height, pos.x, pos.y));
                        }
                    }
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running Keyframe");
}
