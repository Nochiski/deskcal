//! Window placement modes (Windows only):
//!  - Floating : ordinary top-level window (no taskbar button).
//!  - Desktop  : child of Progman, above the desktop icons, below every app window.
//!               Survives Win+D and stays interactive ("desktop widget").
//!  - Wallpaper: parented into the WorkerW *behind* the desktop icons (Wallpaper Engine style).
//!               View-only, because the icon layer receives the input.
use crate::model::WindowMode;
use tauri::WebviewWindow;

#[cfg(windows)]
mod win {
    use super::*;
    use std::ffi::c_void;
    use windows::core::{w, BOOL, PCWSTR};
    use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    use windows::Win32::UI::WindowsAndMessaging::*;

    fn hwnd_of(window: &WebviewWindow) -> Result<HWND, String> {
        let h = window.hwnd().map_err(|e| e.to_string())?;
        Ok(HWND(h.0 as *mut c_void))
    }

    fn find_progman() -> Result<HWND, String> {
        unsafe { FindWindowW(w!("Progman"), PCWSTR::null()).map_err(|e| format!("Progman not found: {e}")) }
    }

    struct EnumState {
        found: Option<HWND>,
    }

    unsafe extern "system" fn enum_proc(top: HWND, lparam: LPARAM) -> BOOL {
        let state = &mut *(lparam.0 as *mut EnumState);
        // A top-level window that hosts SHELLDLL_DefView; the WorkerW right after it is the
        // wallpaper layer (classic Windows 10 / early Windows 11 layout).
        if FindWindowExW(Some(top), None, w!("SHELLDLL_DefView"), PCWSTR::null()).is_ok() {
            if let Ok(w) = FindWindowExW(None, Some(top), w!("WorkerW"), PCWSTR::null()) {
                state.found = Some(w);
                return BOOL(0);
            }
        }
        BOOL(1)
    }

    /// Asks Progman to spawn the wallpaper WorkerW and returns it (or the best fallback).
    fn find_wallpaper_layer(progman: HWND) -> (HWND, bool) {
        unsafe {
            let _ = SendMessageTimeoutW(
                progman,
                0x052C,
                WPARAM(0xD),
                LPARAM(0x1),
                SMTO_NORMAL,
                1000,
                None,
            );
            let mut st = EnumState { found: None };
            let _ = EnumWindows(Some(enum_proc), LPARAM(&mut st as *mut EnumState as isize));
            if let Some(w) = st.found {
                return (w, true);
            }
            // Windows 11 24H2+: WorkerW is created as a child of Progman, beneath SHELLDLL_DefView.
            if let Ok(w) = FindWindowExW(Some(progman), None, w!("WorkerW"), PCWSTR::null()) {
                return (w, true);
            }
            (progman, false)
        }
    }

    /// Moves `hwnd` so that it keeps its on-screen rect after being re-parented into `parent`.
    unsafe fn keep_screen_position(hwnd: HWND, parent: Option<HWND>, rect: RECT, insert_after: Option<HWND>) {
        let mut pt = POINT { x: rect.left, y: rect.top };
        if let Some(p) = parent {
            let _ = ScreenToClient(p, &mut pt);
        }
        let mut flags = SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED;
        if insert_after.is_none() {
            flags |= SWP_NOZORDER;
        }
        let _ = SetWindowPos(hwnd, insert_after, pt.x, pt.y, 0, 0, flags);
    }

    /// True when the window is still parented where `mode` expects it. After an Explorer
    /// restart Progman/WorkerW are recreated and the window ends up orphaned (invisible).
    pub fn is_attached(window: &WebviewWindow, mode: WindowMode) -> bool {
        unsafe {
            let Ok(hwnd) = hwnd_of(window) else { return true };
            let parent = GetAncestor(hwnd, GA_PARENT);
            let Ok(progman) = find_progman() else { return mode == WindowMode::Floating };
            match mode {
                WindowMode::Floating => true,
                WindowMode::Desktop => parent == progman,
                WindowMode::Wallpaper => {
                    if parent.0.is_null() || !IsWindow(Some(parent)).as_bool() {
                        return false;
                    }
                    // WorkerW under Progman, a top-level WorkerW, or Progman itself (fallback).
                    parent == progman || GetAncestor(parent, GA_PARENT) == progman || {
                        let mut cls = [0u16; 16];
                        let n = GetClassNameW(parent, &mut cls) as usize;
                        String::from_utf16_lossy(&cls[..n]) == "WorkerW"
                    }
                }
            }
        }
    }

    /// Floating mode: real acrylic blur of whatever is behind the window (the "liquid glass"
    /// look) plus DWM-rounded corners. Reparented modes sit on the static wallpaper, so the page's
    /// own translucent CSS is enough there and the system effects are turned off.
    fn set_glass(window: &WebviewWindow, hwnd: HWND, enabled: bool) {
        use windows::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWCP_ROUND,
        };
        let pref = if enabled { DWMWCP_ROUND } else { DWMWCP_DONOTROUND };
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &pref as *const _ as *const c_void,
                std::mem::size_of_val(&pref) as u32,
            );
        }
        if enabled {
            if let Err(e) = window_vibrancy::apply_acrylic(window, Some((255, 255, 255, 12))) {
                log::warn!("acrylic unavailable ({e}); trying blur");
                let _ = window_vibrancy::apply_blur(window, Some((255, 255, 255, 12)));
            }
        } else {
            let _ = window_vibrancy::clear_acrylic(window);
            let _ = window_vibrancy::clear_blur(window);
        }
    }

    /// SetParent does not touch WS_CHILD/WS_POPUP by itself (documented), but input routing and
    /// mouse activation only work correctly for a re-parented window when it really is a child
    /// window. Rainmeter's "on desktop" mode does the same. Restored when floating again.
    unsafe fn set_child_style(hwnd: HWND, child: bool) {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let new_style = if child {
            (style & !WS_POPUP.0) | WS_CHILD.0
        } else {
            style & !WS_CHILD.0
        };
        if new_style != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);
        }
    }

    pub fn apply(window: &WebviewWindow, mode: WindowMode) -> Result<(), String> {
        unsafe {
            let hwnd = hwnd_of(window)?;
            // A maximized window cannot live inside Progman sensibly; restore it first.
            if mode != WindowMode::Floating && IsZoomed(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).map_err(|e| e.to_string())?;
            set_glass(window, hwnd, mode == WindowMode::Floating);
            match mode {
                WindowMode::Floating => {
                    let _ = SetParent(hwnd, None);
                    set_child_style(hwnd, false);
                    keep_screen_position(hwnd, None, rect, Some(HWND_NOTOPMOST));
                }
                WindowMode::Desktop => {
                    let progman = find_progman()?;
                    set_child_style(hwnd, true);
                    SetParent(hwnd, Some(progman)).map_err(|e| e.to_string())?;
                    keep_screen_position(hwnd, Some(progman), rect, Some(HWND_TOP));
                }
                WindowMode::Wallpaper => {
                    let progman = find_progman()?;
                    let (layer, dedicated) = find_wallpaper_layer(progman);
                    set_child_style(hwnd, true);
                    SetParent(hwnd, Some(layer)).map_err(|e| e.to_string())?;
                    // In a dedicated WorkerW anything goes; when we had to fall back to Progman,
                    // push below SHELLDLL_DefView so the icons stay on top.
                    let z = if dedicated { Some(HWND_TOP) } else { Some(HWND_BOTTOM) };
                    keep_screen_position(hwnd, Some(layer), rect, z);
                }
            }
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        log::info!("window mode applied: {mode:?}");
        Ok(())
    }
}

#[cfg(windows)]
pub fn apply(window: &WebviewWindow, mode: WindowMode) -> Result<(), String> {
    win::apply(window, mode)
}

#[cfg(windows)]
pub fn is_attached(window: &WebviewWindow, mode: WindowMode) -> bool {
    win::is_attached(window, mode)
}

#[cfg(not(windows))]
pub fn is_attached(_window: &WebviewWindow, _mode: WindowMode) -> bool {
    true
}

#[cfg(not(windows))]
pub fn apply(_window: &WebviewWindow, mode: WindowMode) -> Result<(), String> {
    if mode == WindowMode::Floating {
        Ok(())
    } else {
        Err("이 창 모드는 Windows에서만 지원됩니다.".into())
    }
}
