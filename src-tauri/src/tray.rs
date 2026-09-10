//! System tray icon + menu. The app has no taskbar button; the tray is the main entry point.
use crate::commands::{apply_window_mode, toggle_main_window};
use crate::model::WindowMode;
use crate::state::AppState;
use crate::sync;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

pub struct TrayItems {
    pub floating: CheckMenuItem<Wry>,
    pub desktop: CheckMenuItem<Wry>,
    pub wallpaper: CheckMenuItem<Wry>,
}

impl TrayItems {
    pub fn set_mode(&self, mode: WindowMode) {
        let _ = self.floating.set_checked(mode == WindowMode::Floating);
        let _ = self.desktop.set_checked(mode == WindowMode::Desktop);
        let _ = self.wallpaper.set_checked(mode == WindowMode::Wallpaper);
    }
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let mode = app.state::<AppState>().settings().window_mode;

    let toggle = MenuItem::with_id(app, "toggle", "열기 / 숨기기", true, None::<&str>)?;
    let floating = CheckMenuItem::with_id(app, "mode_floating", "일반 창", true, mode == WindowMode::Floating, None::<&str>)?;
    let desktop = CheckMenuItem::with_id(app, "mode_desktop", "바탕화면 위젯 (아이콘 위)", true, mode == WindowMode::Desktop, None::<&str>)?;
    let wallpaper = CheckMenuItem::with_id(app, "mode_wallpaper", "배경화면 (아이콘 뒤, 보기 전용)", true, mode == WindowMode::Wallpaper, None::<&str>)?;
    let sync_item = MenuItem::with_id(app, "sync", "지금 동기화", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "설정…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &toggle,
            &PredefinedMenuItem::separator(app)?,
            &floating,
            &desktop,
            &wallpaper,
            &PredefinedMenuItem::separator(app)?,
            &sync_item,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    app.manage(TrayItems { floating, desktop, wallpaper });

    let icon = app.default_window_icon().cloned().expect("default icon");
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("DeskCal")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => toggle_main_window(app),
            "mode_floating" => apply_window_mode(app, WindowMode::Floating),
            "mode_desktop" => apply_window_mode(app, WindowMode::Desktop),
            "mode_wallpaper" => apply_window_mode(app, WindowMode::Wallpaper),
            "sync" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let (s, e) = *app.state::<AppState>().last_range.lock().unwrap();
                    let _ = sync::sync_range(&app, s, e).await;
                });
            }
            "settings" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
                let _ = app.emit("open-settings", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
