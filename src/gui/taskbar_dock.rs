//! Windows: keep the battery indicator on the taskbar, left of the notification area.
//!
//! The technique is adapted from BatteryDock (https://github.com/jorjanion/BatteryDock, MIT License,
//! Copyright (c) 2026 jorjanion): a top-most tool window whose *owner* is the `Shell_TrayWnd` taskbar
//! window. Windows always keeps an owned window above its owner, so the taskbar can no longer cover it,
//! even after the Start menu or a flyout takes focus. Geometry comes from the live taskbar windows.
#![cfg(windows)]

use std::ffi::c_void;
use windows::core::w;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindow, GetWindowLongW, GetWindowRect, IsWindow, SetWindowLongPtrW,
    SetWindowLongW, SetWindowPos, GWLP_HWNDPARENT, GWL_EXSTYLE, GW_HWNDPREV, GW_OWNER, HWND_TOPMOST,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

pub struct Geometry {
    pub taskbar: RECT,
    pub tray: Option<RECT>,
    pub taskbar_hwnd: isize,
}

/// Live geometry of the primary taskbar (physical pixels) and of its notification area.
pub fn detect() -> Option<Geometry> {
    unsafe {
        let tb = FindWindowW(w!("Shell_TrayWnd"), None).ok()?;
        if tb.0.is_null() || !IsWindow(tb).as_bool() {
            return None;
        }
        let mut taskbar = RECT::default();
        GetWindowRect(tb, &mut taskbar).ok()?;
        if taskbar.right <= taskbar.left || taskbar.bottom <= taskbar.top {
            return None;
        }
        let mut tray = None;
        if let Ok(t) = FindWindowExW(tb, HWND(std::ptr::null_mut()), w!("TrayNotifyWnd"), None) {
            if !t.0.is_null() {
                let mut r = RECT::default();
                if GetWindowRect(t, &mut r).is_ok() && r.right > r.left && r.bottom > r.top {
                    tray = Some(r);
                }
            }
        }
        Some(Geometry { taskbar, tray, taskbar_hwnd: tb.0 as isize })
    }
}

/// Default spot: on the taskbar, just left of the notification area, vertically centred.
/// `w`/`h` are the indicator's size in physical pixels, `scale` the display scale factor.
pub fn default_position(g: &Geometry, w: i32, h: i32, scale: f64) -> (i32, i32) {
    let gap = (6.0 * scale) as i32;
    let tb = g.taskbar;
    let (tb_w, tb_h) = (tb.right - tb.left, tb.bottom - tb.top);
    if tb_w >= tb_h {
        // horizontal taskbar (bottom or top)
        let right_edge = g.tray.map(|t| t.left).unwrap_or(tb.right - (300.0 * scale) as i32);
        (right_edge - w - gap, tb.top + (tb_h - h) / 2)
    } else {
        // vertical taskbar (left or right): above the notification area
        let bottom_edge = g.tray.map(|t| t.top).unwrap_or(tb.bottom - (200.0 * scale) as i32);
        (tb.left + (tb_w - w) / 2, bottom_edge - h - gap)
    }
}

/// Tool window (no taskbar button), top-most.
pub fn apply_overlay_style(hwnd_val: isize) {
    if hwnd_val == 0 {
        return;
    }
    unsafe {
        let hwnd = HWND(hwnd_val as *mut c_void);
        if !IsWindow(hwnd).as_bool() {
            return;
        }
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        let desired = (ex | WS_EX_TOOLWINDOW.0 as i32 | WS_EX_TOPMOST.0 as i32) & !(WS_EX_APPWINDOW.0 as i32);
        if desired != ex {
            SetWindowLongW(hwnd, GWL_EXSTYLE, desired);
            let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED);
        }
    }
}

unsafe fn is_above(our: HWND, tb: HWND) -> bool {
    let mut cur = GetWindow(tb, GW_HWNDPREV).unwrap_or(HWND(std::ptr::null_mut()));
    let mut steps = 0usize;
    while !cur.0.is_null() && steps < 64 {
        if cur == our {
            return true;
        }
        cur = GetWindow(cur, GW_HWNDPREV).unwrap_or(HWND(std::ptr::null_mut()));
        steps += 1;
    }
    false
}

/// Make the taskbar the owner of our window and keep it top-most and above the taskbar in Z-order.
pub fn ensure_above_taskbar(our_val: isize, taskbar_val: isize) {
    if our_val == 0 {
        return;
    }
    unsafe {
        let our = HWND(our_val as *mut c_void);
        if !IsWindow(our).as_bool() {
            return;
        }
        let tb = HWND(taskbar_val as *mut c_void);
        let have_tb = !tb.0.is_null() && IsWindow(tb).as_bool();
        if have_tb && GetWindow(our, GW_OWNER) != Ok(tb) {
            let _ = SetWindowLongPtrW(our, GWLP_HWNDPARENT, tb.0 as isize);
        }
        let topmost = (GetWindowLongW(our, GWL_EXSTYLE) & WS_EX_TOPMOST.0 as i32) != 0;
        let above = if have_tb { is_above(our, tb) } else { topmost };
        if !topmost || !above {
            let _ = SetWindowPos(our, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }
}

/// True while a full-screen game / presentation is running: the indicator should stay out of the way.
pub fn fullscreen_app_running() -> bool {
    unsafe {
        matches!(SHQueryUserNotificationState(), Ok(s) if s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE)
    }
}

/// Floating mode: drop the taskbar as owner so the indicator behaves like a normal top-most window.
pub fn detach_from_taskbar(our_val: isize) {
    if our_val == 0 {
        return;
    }
    unsafe {
        let our = HWND(our_val as *mut c_void);
        if IsWindow(our).as_bool() && GetWindow(our, GW_OWNER).is_ok_and(|o| !o.0.is_null()) {
            let _ = SetWindowLongPtrW(our, GWLP_HWNDPARENT, 0);
        }
    }
}

/// Find a top-level window by its exact title (the egui dock viewport has no accessible raw handle here).
pub fn find_by_title(title: &str) -> isize {
    use windows::core::PCWSTR;
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        match FindWindowW(PCWSTR::null(), PCWSTR(wide.as_ptr())) {
            Ok(h) if !h.0.is_null() => h.0 as isize,
            _ => 0,
        }
    }
}
