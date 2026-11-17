//! Opt-in, deduplicated desktop alerts. Never include raw server errors,
//! usernames, URLs, tokens, or arbitrary status content in OS notifications.
use crate::preferences::AppPreferences;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertKind { Auth = 0, Disconnect = 1, Automation = 2, Boss = 3 }

/// A transient socket interruption is not an actionable desktop alert.
/// Only an account that was previously online can enter this grace period.
#[derive(Clone, Copy, Debug, Default)]
pub struct DisconnectAlertState {
    since: Option<Instant>,
    alerted: bool,
}

impl DisconnectAlertState {
    pub fn pending(&self) -> bool { self.since.is_some() }

    pub fn observe(&mut self, was_online: bool, phase: Option<&str>,
        still_active: bool, now: Instant) -> bool {
        if !still_active || matches!(phase, Some("Online" | "NeedsLogin")) {
            *self = Self::default();
            return false;
        }
        if self.since.is_none() && was_online {
            self.since = Some(now);
        }
        if !self.alerted && self.since.is_some_and(|start|
            now.saturating_duration_since(start) >= Duration::from_secs(90)) {
            self.alerted = true;
            return true;
        }
        false
    }
}

pub fn classify(message: &str, error: bool) -> Option<AlertKind> {
    let text = message.to_ascii_lowercase();
    if text.contains("boss run complete") || text.contains("boss farming complete") {
        return Some(AlertKind::Boss);
    }
    if !error { return None; }
    if ["login expired", "sign-in required", "needslogin", "authentication expired",
        "session expired", "invalid credential", "invalid token"].iter().any(|s| text.contains(s)) {
        return Some(AlertKind::Auth);
    }
    // Connection state is observed with a 90-second grace period elsewhere.
    // Text errors must never notify immediately about transient disconnections.
    if ["disconnected", "connection lost", "connection interrupted",
        "reconnecting", "socket closed"].iter().any(|text_part| text.contains(text_part)) {
        return None;
    }
    if ["auto catch", "restock", "automation", "market", "capture settings", "purchase"]
        .iter().any(|s| text.contains(s)) {
        return Some(AlertKind::Automation);
    }
    None
}

fn allowed(kind: AlertKind, preferences: AppPreferences) -> bool {
    if !preferences.notifications_enabled || preferences.quiet_notifications { return false; }
    match kind {
        AlertKind::Auth => preferences.notify_auth,
        AlertKind::Disconnect => preferences.notify_disconnect,
        AlertKind::Automation => preferences.notify_automation,
        AlertKind::Boss => preferences.notify_boss,
    }
}

fn body(kind: AlertKind) -> &'static str {
    match kind {
        AlertKind::Auth => "An account needs you to sign in again.",
        AlertKind::Disconnect => "A game or browser connection needs attention.",
        AlertKind::Automation => "An automation action failed; check Controller logs.",
        AlertKind::Boss => "A boss farming run has finished.",
    }
}

#[cfg(windows)]
fn show_windows_notification(message: &str) -> Result<(), String> {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicU32, Ordering};

    static ALERT_ID: AtomicU32 = AtomicU32::new(1);

    type Handle = *mut c_void;
    const NIM_ADD: u32 = 0x0000_0000;
    const NIM_DELETE: u32 = 0x0000_0002;
    const NIF_ICON: u32 = 0x0000_0002;
    const NIF_TIP: u32 = 0x0000_0004;
    const NIF_INFO: u32 = 0x0000_0010;
    const NIIF_INFO: u32 = 0x0000_0001;
    const IDI_APPLICATION: *const u16 = 32512 as *const u16;
    const WS_OVERLAPPED: u32 = 0;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[cfg_attr(target_arch = "x86", repr(C, packed(1)))]
    #[cfg_attr(not(target_arch = "x86"), repr(C))]
    struct NotifyIconDataW {
        cb_size: u32,
        hwnd: Handle,
        uid: u32,
        u_flags: u32,
        u_callback_message: u32,
        h_icon: Handle,
        sz_tip: [u16; 128],
        dw_state: u32,
        dw_state_mask: u32,
        sz_info: [u16; 256],
        u_timeout_or_version: u32,
        sz_info_title: [u16; 64],
        dw_info_flags: u32,
        guid_item: Guid,
        h_balloon_icon: Handle,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn CreateWindowExW(
            ex_style: u32,
            class_name: *const u16,
            window_name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Handle,
            menu: Handle,
            instance: Handle,
            param: *mut c_void,
        ) -> Handle;
        fn DestroyWindow(hwnd: Handle) -> i32;
        fn LoadIconW(instance: Handle, icon_name: *const u16) -> Handle;
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn Shell_NotifyIconW(message: u32, data: *const NotifyIconDataW) -> i32;
    }

    fn copy_wide<const N: usize>(dest: &mut [u16; N], text: &str) {
        for (slot, unit) in dest.iter_mut().take(N.saturating_sub(1)).zip(text.encode_utf16()) {
            *slot = unit;
        }
    }

    let class_name: Vec<u16> = "STATIC\0".encode_utf16().collect();
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            std::ptr::null(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if hwnd.is_null() {
        return Err(format!("CreateWindowExW failed: {}", std::io::Error::last_os_error()));
    }

    let mut data: NotifyIconDataW = unsafe { std::mem::zeroed() };
    data.cb_size = std::mem::size_of::<NotifyIconDataW>() as u32;
    data.hwnd = hwnd;
    data.uid = ALERT_ID.fetch_add(1, Ordering::Relaxed);
    data.u_flags = NIF_ICON | NIF_TIP | NIF_INFO;
    data.h_icon = unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) };
    data.dw_info_flags = NIIF_INFO;
    copy_wide(&mut data.sz_tip, "Moth Controller");
    copy_wide(&mut data.sz_info_title, "Moth Controller");
    copy_wide(&mut data.sz_info, message);

    let added = unsafe { Shell_NotifyIconW(NIM_ADD, &data) };
    if added == 0 {
        unsafe { DestroyWindow(hwnd) };
        return Err("Shell_NotifyIconW(NIM_ADD) returned FALSE".into());
    }

    std::thread::sleep(Duration::from_secs(6));
    unsafe {
        Shell_NotifyIconW(NIM_DELETE, &data);
        DestroyWindow(hwnd);
    }
    Ok(())
}

fn show_notification(message: &'static str) {
    // No worker exists unless an alert is actually emitted.
    let _ = std::thread::Builder::new().name("moth-desktop-alert".into()).spawn(move || {
        if let Err(error) = show_windows_notification(message) {
            crate::logging::warn(&format!("Desktop notification unavailable: {error}"));
        }
    });
}

pub fn maybe_notify(message: &str, error: bool, preferences: AppPreferences,
    last_alert: &mut [Option<Instant>; 4]) {
    let Some(kind) = classify(message, error) else { return; };
    maybe_notify_kind(kind, preferences, last_alert);
}

pub fn maybe_notify_kind(kind: AlertKind, preferences: AppPreferences,
    last_alert: &mut [Option<Instant>; 4]) {
    if !allowed(kind, preferences) { return; }
    let slot = kind as usize;
    let now = Instant::now();
    if last_alert[slot].is_some_and(|last| now.duration_since(last) < Duration::from_secs(600)) {
        return;
    }
    last_alert[slot] = Some(now);
    show_notification(body(kind));
}

pub fn test_notification(preferences: AppPreferences) {
    if preferences.notifications_enabled && !preferences.quiet_notifications {
        show_notification("Desktop notifications are enabled.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_only_known_statuses_and_reject_other_errors() {
        assert_eq!(classify("Boss run complete at Pokémon Center", false), Some(AlertKind::Boss));
        assert_eq!(classify("Native restock: request failed", true), Some(AlertKind::Automation));
        assert_eq!(classify("session expired", true), Some(AlertKind::Auth));
        assert_eq!(classify("Connection lost", true), None);
        assert_eq!(classify("password: secret", true), None);
        assert_eq!(classify("Socket closed", false), None);
    }
    #[test]
    fn disconnection_must_outlast_grace_period_and_notify_once() {
        let start = Instant::now();
        let mut state = DisconnectAlertState::default();
        assert!(!state.observe(true, Some("Reconnecting"), true, start));
        assert!(!state.observe(false, Some("Offline"), true,
            start + Duration::from_secs(89)));
        assert!(state.observe(false, Some("Offline"), true,
            start + Duration::from_secs(90)));
        assert!(!state.observe(false, Some("Offline"), true,
            start + Duration::from_secs(200)));
        assert!(!state.observe(false, Some("Online"), true,
            start + Duration::from_secs(210)));
        assert!(!state.observe(true, None, true,
            start + Duration::from_secs(215)));
        assert!(state.observe(false, None, true,
            start + Duration::from_secs(306)));
    }

    #[test]
    fn manual_stop_and_new_login_clear_disconnection_grace() {
        let start = Instant::now();
        let mut state = DisconnectAlertState::default();
        assert!(!state.observe(true, Some("Reconnecting"), true, start));
        assert!(!state.observe(false, Some("Reconnecting"), false,
            start + Duration::from_secs(180)));
        assert!(!state.observe(true, Some("NeedsLogin"), true,
            start + Duration::from_secs(181)));
        assert!(!state.observe(false, Some("Offline"), true,
            start + Duration::from_secs(500)));
    }

    #[test]
    fn notifications_are_disabled_by_default() {
        let preferences = AppPreferences::default();
        assert!(!allowed(AlertKind::Auth, preferences));
        assert!(!allowed(AlertKind::Boss, preferences));
    }
}
