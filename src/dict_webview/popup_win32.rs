//! Popup-window host for the dictionary WebView.
//!
//! We host the WebView in a separate top-level `WS_POPUP` window owned by the
//! main Iced window. The webview's compose layer (DXGI/DirectComposition) then
//! lives in its own surface and is never touched by Iced's wgpu clear pass.
//!
//! A low-level keyboard hook is installed while the popup is alive so that
//! Alt+F4 (which normally goes to the WebView2 child or popup) is intercepted
//! at the system input level and redirected to the Iced window.

#![allow(unsafe_code)]

use super::ffi_win32::{
    CallNextHookEx, ClientToScreen, CreateWindowExW, DestroyWindow, GetClientRect, GetDpiForWindow,
    GetForegroundWindow, HWND_TOP, IsChild, KBDLLHOOKSTRUCT, NativePoint, NativeRect, PostMessageW,
    SWP_FRAMECHANGED, SWP_SHOWWINDOW, SetWindowPos, VK_F4, WH_KEYBOARD_LL, WM_CLOSE, WM_SYSKEYDOWN,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE, to_wide,
};
use super::{BASE_DPI, SIDEBAR_WIDTH, TAB_BAR_HEIGHT, TOOLBAR_HEIGHT};

// ── Keyboard hook ───────────────────────────────────────────────────────

/// Low-level keyboard hook that intercepts Alt+F4 and forwards it to the
/// owning Iced window when the dictionary popup is active.
pub(crate) unsafe extern "system" fn dict_keyboard_hook(
    ncode: i32,
    wparam: usize,
    lparam: isize,
) -> isize {
    unsafe {
        if ncode >= 0 && wparam == WM_SYSKEYDOWN {
            let ks = &*(lparam as *const KBDLLHOOKSTRUCT);
            if ks.vk_code == VK_F4 {
                let popup = super::POPUP_HWND;
                let owner = super::PARENT_HWND;
                if popup != 0 && owner != 0 {
                    let fg = GetForegroundWindow();
                    let fg_is_ours = fg == owner || fg == popup || IsChild(popup, fg) != 0;
                    if fg_is_ours {
                        super::debug_log("ALT+F4 hook fired; posting WM_CLOSE to Iced window");
                        let _ = PostMessageW(owner, WM_CLOSE, 0, 0);
                        return 1;
                    }
                }
            }
        }
        CallNextHookEx(super::HOOK_HANDLE, ncode, wparam, lparam)
    }
}

// ── Hook installation ───────────────────────────────────────────────────

pub(crate) fn install_hook() {
    use super::ffi_win32::SetWindowsHookExW;
    unsafe {
        if super::HOOK_HANDLE == 0 {
            super::HOOK_HANDLE = SetWindowsHookExW(WH_KEYBOARD_LL, Some(dict_keyboard_hook), 0, 0);
            super::debug_log(&format!(
                "low-level keyboard hook installed h={}",
                super::HOOK_HANDLE
            ));
        }
    }
}

pub(crate) fn uninstall_hook() {
    use super::ffi_win32::UnhookWindowsHookEx;
    unsafe {
        if super::HOOK_HANDLE != 0 {
            UnhookWindowsHookEx(super::HOOK_HANDLE);
            super::debug_log("low-level keyboard hook uninstalled");
            super::HOOK_HANDLE = 0;
        }
    }
}

// ── Popup window management ─────────────────────────────────────────────

pub(crate) fn create_popup(owner: isize, screen_x: i32, screen_y: i32, w: i32, h: i32) -> isize {
    if owner == 0 {
        return 0;
    }
    let class = to_wide("Static");
    let title = to_wide("");
    let hinstance = 0;
    unsafe {
        let popup = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_VISIBLE,
            screen_x,
            screen_y,
            w,
            h,
            owner,
            0,
            hinstance as isize,
            std::ptr::null::<isize>() as isize,
        );
        if popup != 0 {
            install_hook();
        }
        popup
    }
}

pub(crate) fn destroy_popup(hwnd: isize) {
    if hwnd != 0 {
        uninstall_hook();
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }
}

pub(crate) fn move_popup(hwnd: isize, screen_x: i32, screen_y: i32, w: i32, h: i32) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            HWND_TOP,
            screen_x,
            screen_y,
            w,
            h,
            SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );
    }
}

// ── Popup rectangle computation ─────────────────────────────────────────

/// Compute the screen-space rectangle the popup must cover, in physical pixels.
/// Returns (x, y, w, h). `(0,0,0,0)` if the owner window is gone.
pub(crate) fn compute_popup_rect(owner: isize) -> (i32, i32, i32, i32) {
    if owner == 0 {
        return (0, 0, 0, 0);
    }
    let mut client = NativeRect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut origin = NativePoint { x: 0, y: 0 };
    unsafe {
        if GetClientRect(owner, &mut client) == 0 {
            return (0, 0, 0, 0);
        }
        if ClientToScreen(owner, &mut origin) == 0 {
            return (0, 0, 0, 0);
        }
    }
    let client_w = (client.right - client.left) as f64;
    let client_h = (client.bottom - client.top) as f64;
    let dpi = unsafe { GetDpiForWindow(owner) };
    let (panel_w, toolbar_h, tab_h) = calc_popup_dimensions(dpi);
    let popup_x = origin.x + (client_w as i32 - panel_w);
    let popup_y = origin.y + toolbar_h + tab_h;
    let popup_w = panel_w;
    let popup_h = (client_h as i32 - toolbar_h - tab_h).max(1);
    log_popup_rect(owner, dpi, popup_x, popup_y, popup_w, popup_h);
    (popup_x, popup_y, popup_w, popup_h)
}

fn calc_popup_dimensions(dpi: u32) -> (i32, i32, i32) {
    let scale = dpi as f64 / BASE_DPI;
    let panel_w = (SIDEBAR_WIDTH * scale).round() as i32;
    let toolbar_h = (TOOLBAR_HEIGHT * scale).round() as i32;
    let tab_h = (TAB_BAR_HEIGHT * scale).round() as i32;
    (panel_w, toolbar_h, tab_h)
}

fn log_popup_rect(owner: isize, dpi: u32, popup_x: i32, popup_y: i32, popup_w: i32, popup_h: i32) {
    let current = (popup_x, popup_y, popup_w, popup_h);
    unsafe {
        if super::LAST_BOUNDS_LOG != Some(current) {
            super::LAST_BOUNDS_LOG = Some(current);
            let scale = dpi as f64 / BASE_DPI;
            super::debug_log(&format!(
                "compute_popup_rect owner={} dpi={} scale={:.2} screen_rect=({},{},{}x{})",
                owner, dpi, scale, popup_x, popup_y, popup_w, popup_h
            ));
        }
    }
}
