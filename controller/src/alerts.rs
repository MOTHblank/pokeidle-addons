//! Opt-in, deduplicated desktop alerts. Never include raw server errors,
//! usernames, URLs, tokens, or arbitrary status content in OS notifications.
use crate::preferences::AppPreferences;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertKind { Auth = 0, Disconnect = 1, Automation = 2, Boss = 3 }

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
    if ["disconnected", "connection lost", "connection interrupted", "reconnecting",
        "socket closed"].iter().any(|s| text.contains(s)) {
        return Some(AlertKind::Disconnect);
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

fn show_notification(message: &'static str) {
    // No worker exists unless an alert is actually emitted.
    let _ = std::thread::Builder::new().name("moth-desktop-alert".into()).spawn(move || {
        #[cfg(windows)]
        let output = {
            use std::os::windows::process::CommandExt;
            // The message is one of the fixed strings above, never user content.
            // PowerShell's Windows Runtime toast APIs need no external packages.
            let xml = format!("<toast><visual><binding template='ToastGeneric'><text>Moth Controller</text><text>{message}</text></binding></visual></toast>");
            let script = format!(
                "$ErrorActionPreference='Stop'; \
                 [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; \
                 [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom, ContentType = WindowsRuntime] > $null; \
                 $doc = New-Object Windows.Data.Xml.Dom.XmlDocument; \
                 $doc.LoadXml(\"{xml}\"); \
                 $toast = [Windows.UI.Notifications.ToastNotification]::new($doc); \
                 [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Moth Controller').Show($toast)"
            );
            let mut command = std::process::Command::new("powershell.exe");
            command.creation_flags(0x08000000); // no console
            command.args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
                .output()
        };
        #[cfg(not(windows))]
        let output = std::process::Command::new("notify-send")
            .arg("Moth Controller").arg(message).output();
        match output {
            Ok(result) if result.status.success() => {}
            Ok(result) => crate::logging::warn(&format!(
                "Desktop notification unavailable (exit status {})", result.status)),
            Err(error) => crate::logging::warn(&format!("Desktop notification unavailable: {error}")),
        }
    });
}

pub fn maybe_notify(message: &str, error: bool, preferences: AppPreferences,
    last_alert: &mut [Option<Instant>; 4]) {
    let Some(kind) = classify(message, error) else { return; };
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
        assert_eq!(classify("Connection lost", true), Some(AlertKind::Disconnect));
        assert_eq!(classify("password: secret", true), None);
        assert_eq!(classify("Socket closed", false), None);
    }
    #[test]
    fn notifications_are_disabled_by_default() {
        let preferences = AppPreferences::default();
        assert!(!allowed(AlertKind::Auth, preferences));
        assert!(!allowed(AlertKind::Boss, preferences));
    }
}
