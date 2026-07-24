//! Windows FFI declarations, HWND utilities, and raw-window-handle bridge.

#![allow(unsafe_code)]

// ── Windows FFI structs ─────────────────────────────────────────────────

#[repr(C)]
pub(crate) struct NativeRect {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

#[repr(C)]
pub(crate) struct NativePoint {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[repr(C)]
pub(crate) struct KBDLLHOOKSTRUCT {
    pub(crate) vk_code: u32,
    pub(crate) scan_code: u32,
    pub(crate) flags: u32,
    pub(crate) time: u32,
    pub(crate) dw_extra_info: usize,
}

unsafe extern "system" {
    pub(crate) fn FindWindowW(class: *const u16, window: *const u16) -> isize;
    pub(crate) fn GetClientRect(hwnd: isize, rect: *mut NativeRect) -> i32;
    pub(crate) fn GetDpiForWindow(hwnd: isize) -> u32;
    pub(crate) fn EnumWindows(
        callback: Option<unsafe extern "system" fn(isize, *mut isize) -> i32>,
        lparam: *mut isize,
    ) -> i32;
    pub(crate) fn GetWindowTextLengthW(hwnd: isize) -> i32;
    pub(crate) fn GetWindowTextW(hwnd: isize, lpstring: *mut u16, cch: i32) -> i32;
    pub(crate) fn SetWindowPos(
        hwnd: isize,
        hwndinsertafter: isize,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        uflags: u32,
    ) -> i32;
    pub(crate) fn ClientToScreen(hwnd: isize, lppoint: *mut NativePoint) -> i32;
    pub(crate) fn PostMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> i32;
    pub(crate) fn GetForegroundWindow() -> isize;
    pub(crate) fn IsChild(parent: isize, child: isize) -> i32;
    pub(crate) fn SetWindowsHookExW(
        idhook: i32,
        lpfn: Option<unsafe extern "system" fn(i32, usize, isize) -> isize>,
        hmod: isize,
        dwthreadid: u32,
    ) -> isize;
    pub(crate) fn UnhookWindowsHookEx(hhook: isize) -> i32;
    pub(crate) fn CallNextHookEx(hhook: isize, ncode: i32, wparam: usize, lparam: isize) -> isize;
    pub(crate) fn CreateWindowExW(
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
    pub(crate) fn DestroyWindow(hwnd: isize) -> i32;
}

// ── Win32 constants ─────────────────────────────────────────────────────

pub(crate) const WM_CLOSE: u32 = 0x0010;
pub(crate) const WH_KEYBOARD_LL: i32 = 13;
pub(crate) const WM_SYSKEYDOWN: usize = 0x0104;
pub(crate) const VK_F4: u32 = 0x73;

pub(crate) const SWP_SHOWWINDOW: u32 = 0x0040;
pub(crate) const SWP_FRAMECHANGED: u32 = 0x0020;
pub(crate) const HWND_TOP: isize = 0;
/// Borderless popup window.
pub(crate) const WS_POPUP: u32 = 0x8000_0000;
pub(crate) const WS_VISIBLE: u32 = 0x1000_0000;
/// Don't show in the taskbar.
pub(crate) const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
/// Don't activate when shown.
pub(crate) const WS_EX_NOACTIVATE: u32 = 0x0800_0000;

// ── Wide-string helper ──────────────────────────────────────────────────

pub(crate) fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ── Window finding ──────────────────────────────────────────────────────

pub(crate) fn find_window_by_title(title: &str) -> Option<isize> {
    let wide = to_wide(title);
    let hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
    if hwnd != 0 {
        super::debug_log(&format!(
            "found window by exact title '{}' hwnd={}",
            title, hwnd
        ));
        return Some(hwnd);
    }

    // Fallback: search for a window whose title contains "ELP11".
    let mut result: Option<isize> = None;
    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            &mut result as *mut _ as *mut isize,
        );
    }
    if let Some(hwnd) = result {
        super::debug_log(&format!(
            "found window by fallback title containing 'ELP11' hwnd={}",
            hwnd
        ));
    }
    result
}

pub(crate) fn window_title_contains(hwnd: isize, needle: &[u16]) -> bool {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return false;
        }
        let mut buf = vec![0u16; len as usize + 1];
        GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
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

const APP_NAME_WIDE: &[u16] = &[
    0x0045, 0x004c, 0x0050, 0x0031, 0x0031,
];

unsafe extern "system" fn enum_windows_callback(hwnd: isize, lparam: *mut isize) -> i32 {
    let result = unsafe { &mut *(lparam as *mut Option<isize>) };
    if result.is_none() && window_title_contains(hwnd, APP_NAME_WIDE) {
        *result = Some(hwnd);
        return 0;
    }
    1
}

// ── raw-window-handle bridge (WebView2 requires HasWindowHandle) ─────────

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};

pub(crate) struct HwndWrap(pub(crate) isize);

impl HasWindowHandle for HwndWrap {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let hwnd = std::num::NonZeroIsize::new(self.0).ok_or(HandleError::Unavailable)?;
        let raw = RawWindowHandle::Win32(Win32WindowHandle::new(hwnd));
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

impl HasDisplayHandle for HwndWrap {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let raw = RawDisplayHandle::Windows(WindowsDisplayHandle::new());
        Ok(unsafe { DisplayHandle::borrow_raw(raw) })
    }
}
