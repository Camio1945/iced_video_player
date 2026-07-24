//! Youdao Dictionary webview manager.
//!
//! Manages a native child webview (via `wry` on Windows) positioned over the
//! right sidebar's Dictionary tab body. The webview loads the Youdao mobile
//! dictionary page once and injects dark-mode styles + node-removal rules on
//! page load, preserving all features from the reference `test-iced` example:
//! darkreader.js, aggressive CSS override, class-node removal, and mobile UA.
//!
//! Subsequent word searches are performed **without reloading the page**: a
//! JavaScript snippet is injected via `evaluate_script` that fills the page's
//! `#search_input` box (using the native value setter so framework bindings
//! stay in sync) and dispatches an Enter keypress, letting Youdao's own search
//! handler do the work.

#![allow(unsafe_code)]

use std::sync::atomic::{AtomicBool, Ordering};

// ── Windows sub-modules ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod ffi_win32;
#[cfg(target_os = "windows")]
mod popup_win32;
#[cfg(target_os = "windows")]
mod scripts;
#[cfg(target_os = "windows")]
use scripts::*;

// ── Debug logging ────────────────────────────────────────────────────────

pub(crate) fn debug_log(msg: &str) {
    use std::io::Write;
    if let Some(temp) = std::env::var_os("TEMP") {
        let path = std::path::Path::new(&temp).join("iced_video_player_dict_webview.log");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let _ = writeln!(f, "[{}] {}", now, msg);
        }
    }
}

// ── Global webview state ─────────────────────────────────────────────────

pub(crate) static mut WEBVIEW: Option<wry::WebView> = None;
pub(crate) static mut PARENT_HWND: isize = 0;
pub(crate) static mut POPUP_HWND: isize = 0;
pub(crate) static mut HOOK_HANDLE: isize = 0;
pub(crate) static PAGE_LOADED: AtomicBool = AtomicBool::new(false);
pub(crate) static mut CURRENT_WORD: String = String::new();
pub(crate) static mut LAST_BOUNDS_LOG: Option<(i32, i32, i32, i32)> = None;

// ── Layout constants (logical pixels) ────────────────────────────────────

/// Estimated height of the main toolbar (File, Subtitle, time display).
pub(crate) const TOOLBAR_HEIGHT: f64 = 32.0;
/// Estimated height of the tab bar row inside the right sidebar.
pub(crate) const TAB_BAR_HEIGHT: f64 = 34.0;
/// Width of the right sidebar panel.
pub(crate) const SIDEBAR_WIDTH: f64 = 360.0;
/// Base DPI for logical-to-physical conversion.
pub(crate) const BASE_DPI: f64 = 96.0;

// ── Public API ───────────────────────────────────────────────────────────

/// Drive the webview state machine. Call this on every tick.
///
/// * `is_dict_active` – whether the Dictionary sidebar tab is currently selected.
/// * `dict_word` – the word being looked up (None/empty = no lookup active).
/// * `window_title` – the current Iced window title (used to find the HWND).
pub fn tick(is_dict_active: bool, dict_word: Option<&str>, window_title: &str) {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (is_dict_active, dict_word, window_title);
        return;
    }

    #[cfg(target_os = "windows")]
    {
        tick_impl(is_dict_active, dict_word, window_title);
    }
}

/// Destroy the webview immediately (called when the application exits).
#[allow(dead_code)]
pub fn destroy() {
    #[cfg(target_os = "windows")]
    {
        popup_win32::uninstall_hook();
        use ffi_win32::DestroyWindow;
        unsafe {
            if POPUP_HWND != 0 {
                let _ = DestroyWindow(POPUP_HWND);
                POPUP_HWND = 0;
            }
            WEBVIEW = None;
            CURRENT_WORD.clear();
            PAGE_LOADED.store(false, Ordering::SeqCst);
            PARENT_HWND = 0;
        }
    }
}

/// Return `true` when a webview is currently alive and positioned.
pub fn has_webview() -> bool {
    unsafe { WEBVIEW.is_some() }
}

// ── Internal tick implementation ─────────────────────────────────────────

#[cfg(target_os = "windows")]
fn tick_impl(is_dict_active: bool, dict_word: Option<&str>, window_title: &str) {
    use self::popup_win32::compute_popup_rect;

    let word = dict_word.unwrap_or("");

    ensure_owner_window(window_title, word);
    if unsafe { PARENT_HWND == 0 } {
        return;
    }

    if handle_word_change(word) {
        return;
    }

    if hide_popup_if_no_word(word) {
        return;
    }

    let (popup_x, popup_y, popup_w, popup_h) = unsafe { compute_popup_rect(PARENT_HWND) };
    if popup_w <= 0 || popup_h <= 0 {
        return;
    }

    let should_show = is_dict_active;
    let popup = unsafe { POPUP_HWND };
    if popup == 0 {
        create_webview_for_word(word, popup_x, popup_y, popup_w, popup_h);
    } else {
        position_popup_and_inject_scripts(popup, should_show, popup_x, popup_y, popup_w, popup_h);
    }
}

#[cfg(target_os = "windows")]
fn ensure_owner_window(window_title: &str, word: &str) {
    use self::ffi_win32::find_window_by_title;
    unsafe {
        if PARENT_HWND == 0 {
            if let Some(hwnd) = find_window_by_title(window_title) {
                PARENT_HWND = hwnd;
                debug_log(&format!(
                    "owner Iced window hwnd={hwnd} title='{window_title}'"
                ));
            } else {
                debug_log(&format!(
                    "could not find window for title '{window_title}' (word='{word}')"
                ));
            }
        }
    }
}

/// Handle word change: search the new word via JavaScript injection when the
/// webview is already alive, instead of tearing it down and reloading the page.
/// Returns `true` if we should early-return from `tick_impl`.
#[cfg(target_os = "windows")]
fn handle_word_change(word: &str) -> bool {
    unsafe {
        let word_changed = CURRENT_WORD != word;
        if !word_changed {
            return false;
        }
        let old_word = std::mem::replace(&mut CURRENT_WORD, word.to_string());

        // If the webview is already alive, search the new word by injecting
        // JavaScript — no page reload. If the page hasn't finished loading
        // yet, the script silently no-ops (the `#search_input` guard) and the
        // race-condition handler in position_popup_and_inject_scripts will
        // search CURRENT_WORD once the page is ready.
        if !word.is_empty() {
            if let Some(ref wv) = WEBVIEW {
                debug_log(&format!(
                    "searching '{}' via JS injection (was '{}')",
                    word, old_word
                ));
                let _ = wv.evaluate_script(&search_script(word));
            }
        } else {
            debug_log("word cleared, hiding popup (keeping webview alive)");
        }

        // Never tear down the webview on a word change — keep it alive so
        // subsequent searches are instant. The normal flow below will
        // position / show / hide the popup as needed.
        false
    }
}

/// Tear down the current webview and popup window, resetting associated state.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn teardown_webview_and_popup() {
    use self::popup_win32::destroy_popup;
    unsafe {
        if WEBVIEW.is_some() {
            WEBVIEW = None;
            PAGE_LOADED.store(false, Ordering::SeqCst);
        }
        if POPUP_HWND != 0 {
            destroy_popup(POPUP_HWND);
            POPUP_HWND = 0;
        }
    }
}

/// Hide the popup off-screen when there's no word. Returns `true` if we should
/// early-return.
#[cfg(target_os = "windows")]
fn hide_popup_if_no_word(word: &str) -> bool {
    use self::ffi_win32::{HWND_TOP, SWP_FRAMECHANGED, SWP_SHOWWINDOW, SetWindowPos};
    if word.is_empty() {
        unsafe {
            if POPUP_HWND != 0 {
                let _ = SetWindowPos(
                    POPUP_HWND,
                    HWND_TOP,
                    -32000,
                    -32000,
                    0,
                    0,
                    SWP_SHOWWINDOW | SWP_FRAMECHANGED,
                );
            }
        }
        return true;
    }
    false
}

#[cfg(target_os = "windows")]
fn create_webview_for_word(word: &str, popup_x: i32, popup_y: i32, popup_w: i32, popup_h: i32) {
    use self::popup_win32::create_popup;
    let new_popup = create_popup(unsafe { PARENT_HWND }, popup_x, popup_y, popup_w, popup_h);
    if new_popup == 0 {
        debug_log("create_popup failed");
        return;
    }
    unsafe {
        POPUP_HWND = new_popup;
    }
    debug_log(&format!(
        "popup created hwnd={new_popup} at ({popup_x},{popup_y}) {popup_w}x{popup_h}"
    ));
    let url = url_encode_word(word);
    let url = format!("https://dict.youdao.com/m/result?word={url}&lang=en");
    debug_log(&format!("creating webview url='{url}'"));
    build_and_attach_webview(new_popup, &url, popup_w, popup_h);
}

#[cfg(target_os = "windows")]
fn build_and_attach_webview(popup: isize, url: &str, w: i32, h: i32) {
    use self::ffi_win32::HwndWrap;
    use wry::WebViewBuilderExtWindows;

    let wrapper = HwndWrap(popup);
    let initial_rect = wry::Rect {
        position: wry::dpi::Position::Physical(wry::dpi::PhysicalPosition::new(0, 0)),
        size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(w as u32, h as u32)),
    };
    let builder = wry::WebViewBuilder::new_as_child(&wrapper)
        .with_url(url)
        .with_user_agent(MOBILE_UA)
        .with_theme(wry::Theme::Dark)
        .with_bounds(initial_rect)
        .with_browser_accelerator_keys(false)
        .with_initialization_script(DARKREADER_JS)
        .with_on_page_load_handler(|event, _url| {
            if let wry::PageLoadEvent::Finished = event {
                debug_log("page load finished");
                PAGE_LOADED.store(true, Ordering::SeqCst);
            }
        });

    match builder.build() {
        Ok(wv) => {
            debug_log("webview created successfully inside popup");
            unsafe {
                WEBVIEW = Some(wv);
            }
        }
        Err(e) => {
            debug_log(&format!("failed to create webview: {e}"));
            eprintln!("dict_webview: failed to create webview: {e}");
        }
    }
}

#[cfg(target_os = "windows")]
fn position_popup_and_inject_scripts(
    popup: isize,
    should_show: bool,
    popup_x: i32,
    popup_y: i32,
    popup_w: i32,
    popup_h: i32,
) {
    use self::popup_win32::move_popup;
    unsafe {
        let (target_x, target_y) = if should_show {
            (popup_x, popup_y)
        } else {
            (-32000, -32000)
        };
        move_popup(popup, target_x, target_y, popup_w, popup_h);
        if let Some(ref wv) = WEBVIEW {
            if PAGE_LOADED.swap(false, Ordering::SeqCst) {
                let _ = wv.evaluate_script(DARK_OVERRIDE_JS);
                // After the initial page load, search the current word via JS
                // injection. This handles the race condition where the word
                // changed while the page was still loading (the URL may have
                // been for a different word).
                let word = CURRENT_WORD.clone();
                if !word.is_empty() {
                    debug_log(&format!(
                        "page loaded, searching current word '{}' via JS injection",
                        word
                    ));
                    let _ = wv.evaluate_script(&search_script(&word));
                }
            }
        }
    }
}
