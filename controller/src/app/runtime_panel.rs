//! Runtime information and controller credits presentation.
use super::{egui, localize_status, runtime_chip, tr, update, Align, Color32,
    ControllerApp, FontId, Layout, Margin, PanelTab, RichText, SharedUpdateStatus,
    Stroke, BAD, BORDER, DIM, GOOD, MONITOR_HEARTBEAT_INTERVAL,
    MONITOR_POLL_INTERVAL, MONITOR_TAB_INTERVAL, MUTED, PANEL, PANEL_ALT,
    TEXT, UI_REPAINT_INTERVAL, WARN};

pub(super) fn draw_runtime_section(app: &mut ControllerApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(tr("Runtime"))
                .font(FontId::proportional(17.0))
                .strong()
                .color(TEXT),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(tr("Controller-side diagnostics"))
                .size(11.0)
                .color(DIM),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button(tr("Open Profiles")).clicked() {
                app.workspace.panel_tab = PanelTab::Accounts;
            }
            if ui.button(tr("Open Logs")).clicked() {
                app.open_logs();
            }
        });
    });

    ui.add_space(10.0);

    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let running = app.runtimes.iter().map(|runtime| &runtime.game)
                    .filter(|game| game.child.is_some() || game.orphaned_pid.is_some())
                    .count();
                let headless = app.runtimes.iter().map(|runtime| &runtime.game)
                    .filter(|game| (game.child.is_some() || game.orphaned_pid.is_some()) && game.headless)
                    .count();
                let bidi_connected = app.runtimes.iter().map(|runtime| &runtime.game)
                    .filter(|game| game.monitor.as_ref().is_some_and(|monitor| monitor.health().heartbeat_is_fresh()))
                    .count();
                let native_online = app.runtimes.iter().map(|runtime| &runtime.game).filter(|game|
                    game.native.as_ref().is_some_and(|native| native.snapshot().phase == "Online")
                ).count();
                let native_twitch_online = app.runtimes.iter().map(|runtime| &runtime.game).filter(|game|
                    game.native.as_ref().is_some_and(|native| native.twitch_snapshot().phase == "Online")
                ).count();
                let firefox_state = format!("{running} {} · {headless} {}", tr("running"), tr("Hidden Firefox"));
                let bidi_state = format!("{bidi_connected}/{running} {}", tr("Connected"));
                runtime_chip(ui, tr("Firefox"), &firefox_state, if running > 0 { WARN } else { GOOD });
                runtime_chip(ui, tr("PokéIdle"), &format!("{native_online} online"), if native_online > 0 { GOOD } else { MUTED });
                runtime_chip(ui, tr("Twitch"), &format!("{native_twitch_online} online"), if native_twitch_online > 0 { GOOD } else { MUTED });
                runtime_chip(ui, tr("Firefox connection"), &bidi_state, if running == 0 { MUTED } else if bidi_connected == running { GOOD } else { WARN });
                let enabled_profiles = app.accounts.iter().filter(|account| account.enabled).count();
                runtime_chip(
                    ui,
                    tr("Profiles"),
                    &format!("{} {}", enabled_profiles, tr("isolated")),
                    MUTED,
                );
                runtime_chip(ui, tr("Game heartbeat"), &format!("{} s", MONITOR_HEARTBEAT_INTERVAL.as_secs()), MUTED);
                runtime_chip(ui, tr("Game data"), &format!("{} min", MONITOR_POLL_INTERVAL.as_secs() / 60), MUTED);
                runtime_chip(ui, tr("Tab inventory"), &format!("{} min", MONITOR_TAB_INTERVAL.as_secs() / 60), MUTED);
                runtime_chip(ui, tr("Twitch"), tr("Connected in app"), MUTED);
                runtime_chip(ui, tr("UI repaint"), &format!("{} s", UI_REPAINT_INTERVAL.as_secs()), MUTED);
            });
        });

    ui.add_space(12.0);
    ui.collapsing(tr("Recent events"), |ui| {
        if app.events.is_empty() {
            ui.label(RichText::new(tr("No events yet")).color(DIM));
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("controller_event_history")
            .max_height(190.0)
            .show(ui, |ui| {
                for event in &app.events {
                    let age = event.at.elapsed().as_secs();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(format!("{age}s {}", tr("ago")))
                            .size(9.0).color(DIM));
                        ui.label(RichText::new(localize_status(event.message.clone()))
                            .size(10.0).color(if event.is_error { BAD } else { MUTED }));
                    });
                }
            });
    });
}


pub(super) fn draw_credits(ui: &mut egui::Ui, update_status: &SharedUpdateStatus) {
    egui::Frame::new()
        .fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8.0)
        .inner_margin(Margin::symmetric(9, 7))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(tr("Credits"))
                        .size(9.0)
                        .strong()
                        .color(DIM),
                );

                ui.add_space(3.0);

                ui.label(
                    RichText::new(tr("by MOTHblank"))
                        .size(10.0)
                        .color(MUTED),
                );

                // Keep the links in aligned rows. A wrapped run of independent
                // separators and default-sized hyperlinks breaks in narrow sidebars.
                egui::Grid::new("sidebar_credits_links")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.hyperlink_to(
                            RichText::new(tr("Google Play")).size(10.0),
                            "https://play.google.com/store/apps/developer?id=MOTHblank",
                        );
                        ui.hyperlink_to(
                            RichText::new(tr("X")).size(10.0),
                            "https://x.com/MOTHblank",
                        );
                        ui.end_row();

                        ui.hyperlink_to(
                            RichText::new(tr("WhatsApp / Pix")).size(10.0),
                            "https://wa.me/+5537999933376",
                        );
                        ui.hyperlink_to(
                            RichText::new(tr("source code")).size(10.0),
                            "https://github.com/MOTHblank/pokeidle-addons",
                        );
                        ui.end_row();
                    });

                ui.add_space(5.0);
                ui.label(
                    RichText::new(format!("{} {}", tr("Version"), env!("CARGO_PKG_VERSION")))
                        .size(9.0)
                        .color(DIM),
                );

                let latest_version = update_status
                    .lock()
                    .ok()
                    .and_then(|state| state.latest_version.clone());
                if let Some(latest_version) = latest_version
                    .filter(|latest| update::is_newer_version(latest, env!("CARGO_PKG_VERSION")))
                {
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(Color32::from_rgb(57, 255, 20))
                        .corner_radius(5.0)
                        .inner_margin(Margin::symmetric(6, 3))
                        .show(ui, |ui| {
                            ui.hyperlink_to(
                                RichText::new(format!(
                                    "{} · v{}",
                                    tr("Update available"),
                                    latest_version
                                ))
                                .size(10.0)
                                .strong()
                                .color(Color32::from_rgb(12, 38, 10)),
                                update::RELEASES_URL,
                            );
                        });
                }
            });
        });
}


