//! Youdao Dictionary webview manager.
//!
//! Manages a native child webview (via `wry` on Windows) positioned over the
//! right sidebar's Dictionary tab body. The webview loads the Youdao mobile
//! dictionary page and injects dark-mode styles + node-removal rules on page
//! load, preserving all features from the reference `test-iced` example:
//! darkreader.js, aggressive CSS override, class-node removal, and mobile UA.

#![allow(unsafe_code)]

use std::sync::atomic::{AtomicBool, Ordering};

// ── Debug logging ────────────────────────────────────────────────────────

fn debug_log(msg: &str) {
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

static mut WEBVIEW: Option<wry::WebView> = None;
static mut PARENT_HWND: isize = 0;
static mut POPUP_HWND: isize = 0;
static PAGE_LOADED: AtomicBool = AtomicBool::new(false);
static mut CURRENT_WORD: String = String::new();
static mut LAST_BOUNDS_LOG: Option<(i32, i32, i32, i32)> = None;
/// Track whether the webview was created for a word search.
/// This allows us to hide/show when switching tabs without destroying the webview.
static mut DICT_SEARCH_ACTIVE: bool = false;

// ── Constants ────────────────────────────────────────────────────────────

/// The darkreader.js library (embedded at compile time).
const DARKREADER_JS: &str = include_str!("../darkreader.js");

/// Aggressive dark CSS injected via DOM after page load (bypasses CSP).
/// Also calls DarkReader.enable() once the page DOM is fully populated.
/// Includes repeated removal of unwanted class nodes (top nav, ads).
const DARK_OVERRIDE_JS: &str = r#"
(function(){
    var s=document.createElement('style');
    s.textContent=
        'html{background:#181818!important;color:#e0e0e0!important;}'+
        'body{background:#181818!important;color:#e0e0e0!important;}'+
        'div,section,article,header,footer,main,nav,aside,'+
        'ul,ol,li,p,h1,h2,h3,h4,h5,h6,a,span,b,i,em,strong,small,big,'+
        'table,thead,tbody,tr,td,th,dl,dt,dd,blockquote,pre,code,hr,br,'+
        'label,fieldset,legend,figure,figcaption,details,summary,address,'+
        '[class],[id],[role]{background-color:#181818!important;color:#e0e0e0!important;border-color:#333!important;}'+
        'input,button,textarea,select{background-color:#2a2a2a!important;color:#e0e0e0!important;border-color:#444!important;}'+
        'img,video,canvas,svg,picture,iframe{filter:invert(1) hue-rotate(180deg)!important;}';
    (document.head||document.documentElement).appendChild(s);
    document.documentElement.style.backgroundColor='#181818';
    document.documentElement.style.color='#e0e0e0';
    if(document.body){
        document.body.style.backgroundColor='#181818';
        document.body.style.color='#e0e0e0';
    }
    if(typeof DarkReader!=='undefined'){
        try{DarkReader.enable({brightness:100,contrast:90,sepia:10});}catch(e){}
    }

    // Remove unwanted elements (top nav, ads). Run three times to catch
    // elements that may appear at different times during page load.
    function removeUnwanted(){
        var classes=['content','m-top_vav','promo-ad'];
        for(var i=0;i<classes.length;i++){
            var els=document.getElementsByClassName(classes[i]);
            while(els.length>0){els[0].remove();}
        }
    }
    removeUnwanted();
    setTimeout(removeUnwanted,5000);
    setTimeout(removeUnwanted,20000);
})();
"#;

/// Mobile user-agent so Youdao serves the compact mobile page (better fit
/// for the narrow 360px sidebar).
const MOBILE_UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) \
    AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";

// ── Layout constants (logical pixels) ────────────────────────────────────

/// Estimated height of the main toolbar (File, Subtitle, time display).
const TOOLBAR_HEIGHT: f64 = 32.0;
/// Estimated height of the tab bar row inside the right sidebar.
const TAB_BAR_HEIGHT: f64 = 34.0;
/// Width of the right sidebar panel.
const SIDEBAR_WIDTH: f64 = 360.0;
/// Base DPI for logical-to-physical conversion.
const BASE_DPI: f64 = 96.0;

// ── Windows FFI ──────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
#[repr(C)]
struct NativeRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct NativePoint {
    x: i32,
    y: i32,
}

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn FindWindowW(class: *const u16, window: *const u16) -> isize;
    fn GetClientRect(hwnd: isize, rect: *mut NativeRect) -> i32;
    fn GetDpiForWindow(hwnd: isize) -> u32;
    fn EnumWindows(callback: Option<unsafe extern "system" fn(isize, *mut isize) -> i32>, lparam: *mut isize) -> i32;
    fn GetWindowTextLengthW(hwnd: isize) -> i32;
    fn GetWindowTextW(hwnd: isize, lpstring: *mut u16, cch: i32) -> i32;
    fn SetWindowPos(hwnd: isize, hwndinsertafter: isize, x: i32, y: i32, cx: i32, cy: i32, uflags: u32) -> i32;
    fn ClientToScreen(hwnd: isize, lppoint: *mut NativePoint) -> i32;
    fn CreateWindowExW(
        dwexstyle: u32,
        lpclassname: *const u16,
        lpwindowname: *const u16,
        dwstyle: u32,
        x: i32,
        y: i32,
        nwidth: i32,
        nheight: i32,
        hwndparent: isize,
        hmenu: isize,
        hinstance: isize,
        lpparam: isize,
    ) -> isize;
    fn DestroyWindow(hwnd: isize) -> i32;
}

#[cfg(target_os = "windows")]
const SWP_SHOWWINDOW: u32 = 0x0040;
#[cfg(target_os = "windows")]
const SWP_FRAMECHANGED: u32 = 0x0020;
#[cfg(target_os = "windows")]
const HWND_TOP: isize = 0;
/// Borderless popup window.
#[cfg(target_os = "windows")]
const WS_POPUP: u32 = 0x8000_0000;
#[cfg(target_os = "windows")]
const WS_VISIBLE: u32 = 0x1000_0000;
/// Don't show in the taskbar.
#[cfg(target_os = "windows")]
const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
/// Don't activate when shown.
#[cfg(target_os = "windows")]
const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
/// Stay above sibling windows (keeps it above the Iced window).
#[cfg(target_os = "windows")]
const WS_EX_TOPMOST: u32 = 0x0000_0008;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn find_window_by_title(title: &str) -> Option<isize> {
    let wide = to_wide(title);
    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if hwnd != 0 {
        debug_log(&format!("found window by exact title '{}' hwnd={}", title, hwnd));
        return Some(hwnd);
    }

    // Fallback: search for a window whose title contains "Video Player".
    // This keeps working even if Iced adds/changes title formatting.
    let mut result: Option<isize> = None;
    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            &mut result as *mut _ as *mut isize,
        );
    }
    if let Some(hwnd) = result {
        debug_log(&format!(
            "found window by fallback title containing 'Video Player' hwnd={}",
            hwnd
        ));
    }
    result
}

#[cfg(target_os = "windows")]
fn window_title_contains(hwnd: isize, needle: &[u16]) -> bool {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return false;
        }
        let mut buf = vec![0u16; len as usize + 1];
        GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
        // simple substring search on UTF-16 code units
        'outer: for i in 0..=buf.len().saturating_sub(needle.len()) {
            for (j, &n) in needle.iter().enumerate() {
                if buf.get(i + j) != Some(&n) {
                    continue 'outer;
                }
            }
            return true;
        }
        false
    }
}

#[cfg(target_os = "windows")]
const VIDEO_PLAYER_WIDE: &[u16] = &[
    0x0056, 0x0069, 0x0064, 0x0065, 0x006f, 0x0020, 0x0050, 0x006c, 0x0061, 0x0079, 0x0065, 0x0072,
];

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_callback(hwnd: isize, lparam: *mut isize) -> i32 {
    let result = unsafe { &mut *(lparam as *mut Option<isize>) };
    if result.is_none() && window_title_contains(hwnd, VIDEO_PLAYER_WIDE) {
        *result = Some(hwnd);
        return 0; // stop enumerating
    }
    1
}

// ── raw-window-handle bridge (WebView2 requires HasWindowHandle) ─────────

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowsDisplayHandle, Win32WindowHandle, WindowHandle,
};

#[cfg(target_os = "windows")]
struct HwndWrap(isize);

#[cfg(target_os = "windows")]
impl HasWindowHandle for HwndWrap {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let hwnd = std::num::NonZeroIsize::new(self.0).ok_or(HandleError::Unavailable)?;
        let raw = RawWindowHandle::Win32(Win32WindowHandle::new(hwnd));
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

#[cfg(target_os = "windows")]
impl HasDisplayHandle for HwndWrap {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let raw = RawDisplayHandle::Windows(WindowsDisplayHandle::new());
        Ok(unsafe { DisplayHandle::borrow_raw(raw) })
    }
}

// ── Public API ───────────────────────────────────────────────────────────

/// Drive the webview state machine. Call this on every tick.
///
/// * `is_dict_active` – whether the Dictionary sidebar tab is currently selected.
/// * `dict_word` – the word being looked up (None/empty = no lookup active).
/// * `window_title` – the current Iced window title (used to find the HWND).
pub fn tick(is_dict_active: bool, dict_word: Option<&str>, window_title: &str) {
    #[cfg(not(target_os = "windows"))]
    {
        // WebView2 is Windows-only. On other platforms the webview is
        // unsupported and the dictionary tab falls back to empty content.
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
    unsafe {
        WEBVIEW = None;
        CURRENT_WORD.clear();
        PAGE_LOADED.store(false, Ordering::SeqCst);
        PARENT_HWND = 0;
    }
}

/// Return `true` when a webview is currently alive and positioned.
pub fn has_webview() -> bool {
    unsafe { WEBVIEW.is_some() }
}

// ── Popup-window host ────────────────────────────────────────────────────
//
// We host the WebView in a separate top-level `WS_POPUP` window owned by the
// main Iced window. The webview's compose layer (DXGI/DirectComposition) then
// lives in its own surface and is never touched by Iced's wgpu clear pass.
// This is the only arrangement that survives a continuously-running video
// widget painting into the parent every frame.

#[cfg(target_os = "windows")]
fn create_popup(owner: isize, screen_x: i32, screen_y: i32, w: i32, h: i32) -> isize {
    if owner == 0 {
        return 0;
    }
    let class = to_wide("Static");
    let title = to_wide("");
    let hinstance = 0;
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_VISIBLE,
            screen_x,
            screen_y,
            w,
            h,
            owner, // hwndParent == owner → owned popup that stays above Iced
            0,
            hinstance as isize,
            std::ptr::null::<isize>() as isize,
        )
    }
}

#[cfg(target_os = "windows")]
fn destroy_popup(hwnd: isize) {
    if hwnd != 0 {
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }
}

#[cfg(target_os = "windows")]
fn move_popup(hwnd: isize, screen_x: i32, screen_y: i32, w: i32, h: i32) {
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

/// Compute the screen-space rectangle the popup must cover, in physical pixels.
/// Returns (x, y, w, h). `(0,0,0,0)` if the owner window is gone.
#[cfg(target_os = "windows")]
fn compute_popup_rect(owner: isize) -> (i32, i32, i32, i32) {
    if owner == 0 {
        return (0, 0, 0, 0);
    }
    let mut client = NativeRect { left: 0, top: 0, right: 0, bottom: 0 };
    let mut origin = NativePoint { x: 0, y: 0 };
    unsafe {
        if GetClientRect(owner, &mut client) == 0 {
            return (0, 0, 0, 0);
        }
        // Screen position of the client area's top-left corner. Using
        // GetWindowRect + title-bar math would be inaccurate across DPI scales
        // and snapped windows; ClientToScreen is the canonical way.
        if ClientToScreen(owner, &mut origin) == 0 {
            return (0, 0, 0, 0);
        }
    }
    let client_w = (client.right - client.left) as f64;
    let client_h = (client.bottom - client.top) as f64;
    let dpi = unsafe { GetDpiForWindow(owner) };
    let scale = dpi as f64 / BASE_DPI;

    let panel_w = (SIDEBAR_WIDTH * scale).round() as i32;
    let toolbar_h = (TOOLBAR_HEIGHT * scale).round() as i32;
    let tab_h = (TAB_BAR_HEIGHT * scale).round() as i32;

    let popup_x = origin.x + (client_w as i32 - panel_w);
    let popup_y = origin.y + toolbar_h + tab_h;
    let popup_w = panel_w;
    let popup_h = (client_h as i32 - toolbar_h - tab_h).max(1);

    unsafe {
        let current = (popup_x, popup_y, popup_w, popup_h);
        if LAST_BOUNDS_LOG != Some(current) {
            LAST_BOUNDS_LOG = Some(current);
            debug_log(&format!(
                "compute_popup_rect owner={} dpi={} scale={:.2} screen_rect=({},{},{}x{})",
                owner, dpi, scale, popup_x, popup_y, popup_w, popup_h
            ));
        }
    }

    (popup_x, popup_y, popup_w, popup_h)
}



#[cfg(target_os = "windows")]
use wry::WebViewBuilderExtWindows;

fn tick_impl(is_dict_active: bool, dict_word: Option<&str>, window_title: &str) {
    let word = dict_word.unwrap_or("");

    // --- Find the Iced window (owner) if needed. ---
    unsafe {
        if PARENT_HWND == 0 {
            if let Some(hwnd) = find_window_by_title(window_title) {
                PARENT_HWND = hwnd;
                debug_log(&format!("owner Iced window hwnd={hwnd} title='{window_title}'"));
            } else {
                debug_log(&format!(
                    "could not find window for title '{window_title}' (word='{word}')"
                ));
            }
        }
        if PARENT_HWND == 0 {
            return;
        }
    }

    // --- Word changed? Tear down so we can recreate with the new word. ---
    // Only destroy if there was an active search and the word is different/cleared.
    unsafe {
        let word_changed = CURRENT_WORD != word;
        if word_changed {
            // If the new word is empty, clear everything and hide.
            if word.is_empty() {
                if WEBVIEW.is_some() {
                    WEBVIEW = None;
                    PAGE_LOADED.store(false, Ordering::SeqCst);
                }
                if POPUP_HWND != 0 {
                    destroy_popup(POPUP_HWND);
                    POPUP_HWND = 0;
                    debug_log("popup destroyed (word cleared)");
                }
                CURRENT_WORD.clear();
                DICT_SEARCH_ACTIVE = false;
                return;
            }
            // Word changed to a new non-empty word: rebuild webview.
            CURRENT_WORD = word.to_string();
            DICT_SEARCH_ACTIVE = true;
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

    // --- If no word, nothing to show. ---
    if word.is_empty() {
        unsafe {
            if POPUP_HWND != 0 {
                // Hide the popup off-screen instead of destroying.
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
        return;
    }

    // --- Mark that we have an active dictionary search. ---
    unsafe {
        DICT_SEARCH_ACTIVE = true;
    }

    // --- Compute desired popup rect (screen coords, physical pixels). ---
    let (popup_x, popup_y, popup_w, popup_h) =
        unsafe { compute_popup_rect(PARENT_HWND) };
    if popup_w <= 0 || popup_h <= 0 {
        return;
    }

    // --- Determine if popup should be visible. ---
    // Hide off-screen when dictionary tab is not active, but keep webview alive.
    let should_show = is_dict_active;

    // --- Create the popup + webview if needed. ---
    let popup = unsafe { POPUP_HWND };
    if popup == 0 {
        // No popup exists yet, create it.
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

        let wrapper = HwndWrap(new_popup);
        let initial_rect = wry::Rect {
            position: wry::dpi::Position::Physical(wry::dpi::PhysicalPosition::new(0, 0)),
            size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(popup_w as u32, popup_h as u32)),
        };
        let builder = wry::WebViewBuilder::new_as_child(&wrapper)
            .with_url(&url)
            .with_user_agent(MOBILE_UA)
            .with_theme(wry::Theme::Dark)
            .with_bounds(initial_rect)
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
                unsafe { WEBVIEW = Some(wv); }
            }
            Err(e) => {
                debug_log(&format!("failed to create webview: {e}"));
                eprintln!("dict_webview: failed to create webview: {e}");
                // Bail and leave the popup around; next tick will retry the webview.
            }
        }
        return;
    }

    // --- Popup exists: position it (on-screen if active, off-screen if inactive). ---
    unsafe {
        let (target_x, target_y) = if should_show {
            (popup_x, popup_y)
        } else {
            // Move off-screen to hide without destroying the webview.
            (-32000, -32000)
        };
        move_popup(popup, target_x, target_y, popup_w, popup_h);
        if let Some(ref wv) = WEBVIEW {
            // When the page finishes loading, inject dark styling via DOM.
            if PAGE_LOADED.swap(false, Ordering::SeqCst) {
                let _ = wv.evaluate_script(DARK_OVERRIDE_JS);
            }
        }
    }

    // --- Compute desired popup rect (screen coords, physical pixels). ---
    let (popup_x, popup_y, popup_w, popup_h) =
        unsafe { compute_popup_rect(PARENT_HWND) };
    if popup_w <= 0 || popup_h <= 0 {
        return;
    }

    // --- Create the popup + webview if needed. ---
    let popup = unsafe { POPUP_HWND };
    if popup == 0 {
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

        let wrapper = HwndWrap(new_popup);
        let initial_rect = wry::Rect {
            position: wry::dpi::Position::Physical(wry::dpi::PhysicalPosition::new(0, 0)),
            size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(popup_w as u32, popup_h as u32)),
        };
        let builder = wry::WebViewBuilder::new_as_child(&wrapper)
            .with_url(&url)
            .with_user_agent(MOBILE_UA)
            .with_theme(wry::Theme::Dark)
            .with_bounds(initial_rect)
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
                unsafe { WEBVIEW = Some(wv); }
            }
            Err(e) => {
                debug_log(&format!("failed to create webview: {e}"));
                eprintln!("dict_webview: failed to create webview: {e}");
                // Bail and leave the popup around; next tick will retry the webview.
            }
        }
        return;
    }

    // --- Popup exists: track the Iced window. ---
    unsafe {
        move_popup(popup, popup_x, popup_y, popup_w, popup_h);
        if let Some(ref wv) = WEBVIEW {
            // When the page finishes loading, inject dark styling via DOM.
            if PAGE_LOADED.swap(false, Ordering::SeqCst) {
                let _ = wv.evaluate_script(DARK_OVERRIDE_JS);
            }
        }
    }
}

// ── URL encoding ─────────────────────────────────────────────────────────

/// Minimal percent-encoding for the word in the Youdao URL query.
fn url_encode_word(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric()
                || c == '-'
                || c == '_'
                || c == '.'
                || c == '~'
            {
                c.to_string()
            } else {
                let mut buf = [0u8; 4];
                let enc = c.encode_utf8(&mut buf);
                enc.bytes()
                    .map(|b| format!("%{:02X}", b))
                    .collect::<Vec<_>>()
                    .join("")
            }
        })
        .collect()
}
