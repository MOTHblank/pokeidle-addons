//! Reusable compact runtime and connection status widgets.
use super::{egui, tr, Align, Color32, Health, Layout, Margin, RichText, Stroke,
    ACCENT, BAD, BORDER, DIM, GOOD, MUTED, PANEL_ALT, TEXT, WARN};

pub(super) fn status_chip(ui: &mut egui::Ui, name: &str, state: &str, good: bool) -> egui::Response {
    let waiting = [
        tr("Connecting"), tr("Reconnecting"), tr("Sign-in required"),
        tr("Waiting for login"),
    ].contains(&state);
    let color = if good { GOOD } else if waiting { WARN } else { DIM };
    ui.label(
        RichText::new(format!("● {}: {}", tr(name), tr(state)))
            .size(10.0)
            .color(color),
    )
}



pub(super) fn status_badge(ui: &mut egui::Ui, health: &Health, running: bool) {
    // Loss of browser monitoring does not imply that the game stopped.
    // Keep the state accessible without a constantly growing, alarm-colored warning.
    let no_readings = running && health.state == "Running"
        && health.last_heartbeat_at.is_some()
        && !health.heartbeat_is_fresh();
    let session_blocked = running && health.last_error.as_deref()
        .is_some_and(|error| error.contains("Maximum number of active sessions"));
    let (label, color) = if !running {
        ("STOPPED", DIM)
    } else if session_blocked {
        ("FIREFOX CONNECTION BLOCKED", WARN)
    } else if health.state == "Reconnecting" {
        ("RECONNECTING", WARN)
    } else if no_readings {
        ("UPDATES DELAYED", DIM)
    } else if health.telemetry_is_fresh()
        && health.heartbeat_is_fresh()
        && health.logged_in
        && health.activity == "Hunting" {
        ("HUNTING", GOOD)
    } else if health.state == "Running" && health.logged_in {
        ("ONLINE", GOOD)
    } else if health.state == "Existing Firefox" {
        ("EXISTING", WARN)
    } else if health.state == "Connecting" || health.state == "Reconnecting" {
        ("CONNECTING", WARN)
    } else if health.state == "Running" && !health.logged_in {
        ("LOGIN", WARN)
    } else {
        ("ERROR", BAD)
    };

    let response = ui.label(
        RichText::new(format!("● {}", tr(label)))
            .size(12.0)
            .strong()
            .color(color),
    );
    if no_readings || session_blocked || health.last_error.is_some() {
        response.on_hover_ui(|ui| {
            if session_blocked {
                ui.label(tr("Firefox isn't responding for this account. Restart Firefox to reconnect."));
            } else {
                ui.label(tr("Game updates are temporarily unavailable. Firefox may still be running."));
            }
            ui.label(RichText::new(tr("The displayed metrics are from the last successful reading.")).color(DIM));
            if let Some(error) = &health.last_error {
                ui.label(RichText::new(error).color(DIM));
            }
        });
    }
}


pub(super) fn runtime_chip(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    egui::Frame::new()
        .fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8.0)
        .inner_margin(Margin::symmetric(11, 8))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(tr(label)).size(9.0).strong().color(DIM));
                ui.label(RichText::new(tr(value)).size(11.0).color(color));
            });
        });
}

pub(super) fn runtime_row(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr(label)).size(10.0).color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(tr(value)).size(10.0).strong().color(color));
        });
    });
    ui.add_space(5.0);
}

pub(super) fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(9.0)
            .strong()
            .color(DIM),
    );
    ui.add_space(5.0);
}

pub(super) fn sidebar_button<'a>(ui: &mut egui::Ui, text: &'a str, selected: bool) -> egui::Response {
    let fill = if selected {
        ACCENT.linear_multiply(0.18)
    } else {
        Color32::TRANSPARENT
    };

    ui.add_sized(
        [ui.available_width().max(80.0), 34.0],
        egui::Button::new(
            RichText::new(text)
                .size(11.0)
                .strong()
                .color(if selected { TEXT } else { MUTED }),
        )
        .fill(fill)
        .stroke(if selected {
            Stroke::new(1.0, ACCENT.linear_multiply(0.6))
        } else {
            Stroke::NONE
        })
        .corner_radius(7.0),
    )
}

pub(super) fn compact_text(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let mut out: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        out.push('…');
    }

    out
}

