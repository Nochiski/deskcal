mod apple;
mod commands;
mod google;
mod ics;
mod model;
mod notify;
mod state;
mod store;
mod sync;
mod tray;
mod window_mode;

use model::{WindowGeometry, WindowMode};
use state::AppState;
use std::sync::atomic::Ordering;
use tauri::{Manager, WindowEvent};

fn restore_geometry(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let g = store::load_window(&state.paths);
    let Some(w) = app.get_webview_window("main") else { return };
    if let (Some(width), Some(height)) = (g.width, g.height) {
        if width >= 300 && height >= 200 {
            let _ = w.set_size(tauri::PhysicalSize::new(width, height));
        }
    }
    if let (Some(x), Some(y)) = (g.x, g.y) {
        // Only restore if the point is on some monitor (avoids off-screen windows).
        let on_screen = w
            .available_monitors()
            .map(|ms| {
                ms.iter().any(|m| {
                    let p = m.position();
                    let s = m.size();
                    x >= p.x - 50 && y >= p.y - 50 && x < p.x + s.width as i32 && y < p.y + s.height as i32
                })
            })
            .unwrap_or(false);
        if on_screen {
            let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
        }
    }
}

fn schedule_geometry_save(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let Some(w) = app.get_webview_window("main") else { return };
    let (Ok(pos), Ok(size)) = (w.outer_position(), w.outer_size()) else { return };
    if size.width == 0 || size.height == 0 {
        return; // minimized
    }
    *state.pending_geometry.lock().unwrap() = WindowGeometry {
        x: Some(pos.x),
        y: Some(pos.y),
        width: Some(size.width),
        height: Some(size.height),
    };
    let seq = state.geometry_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
        let state = app.state::<AppState>();
        if state.geometry_seq.load(Ordering::SeqCst) == seq {
            let g = state.pending_geometry.lock().unwrap().clone();
            store::save_window(&state.paths, &g);
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_targets = [
        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: Some("deskcal".into()) }),
        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
    ];

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets(log_targets)
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            let handle = app.handle().clone();
            app.manage(AppState::new(&handle));

            restore_geometry(&handle);
            tray::setup(&handle)?;

            let mode = handle.state::<AppState>().settings().window_mode;
            let minimized = std::env::args().any(|a| a == "--minimized");
            if let Some(w) = handle.get_webview_window("main") {
                if !(minimized && mode == WindowMode::Floating) {
                    let _ = w.show();
                }
                if mode != WindowMode::Floating {
                    // Let the window finish creating before re-parenting it.
                    let h = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        commands::apply_window_mode(&h, mode);
                    });
                }
            }

            notify::start(handle.clone());
            sync::start_background(handle.clone());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                schedule_geometry_save(window.app_handle());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_accounts,
            commands::connect_google,
            commands::disconnect_google,
            commands::connect_apple,
            commands::disconnect_apple,
            commands::get_cached,
            commands::sync_now,
            commands::set_window_mode,
            commands::hide_window,
            commands::open_external,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                // Keep running in the tray when the last window closes; exit only via the tray.
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
