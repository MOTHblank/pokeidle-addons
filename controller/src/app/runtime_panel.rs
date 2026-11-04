//! Runtime information and controller credits presentation.
use super::{egui, localize_status, runtime_chip, tr, update, Align,
    ControllerApp, FontId, Layout, Margin, PanelTab, RichText,
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


pub(super) fn draw_credits(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let (latest_version, _notes_url, checking, downloading, staged_version, failure) =
        match app.update_status.lock() {
            Ok(status) => (
                status.latest_version.clone(),
                status.release_notes_url.clone(),
                status.checking,
                status.downloading,
                status.staged_version.clone(),
                status.last_error.clone(),
            ),
            Err(_) => (None, None, false, false, None, Some("Update status unavailable".into())),
        };
    let newer = latest_version.as_deref()
        .is_some_and(|latest| update::is_newer_version(latest, env!("CARGO_PKG_VERSION")));

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
                ui.label(RichText::new(tr("by MOTHblank")).size(10.0).color(MUTED));

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
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{} {}", tr("Version"), env!("CARGO_PKG_VERSION")))
                            .size(9.0)
                            .color(DIM),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let status = if checking {
                            tr("Checking for updates...").to_owned()
                        } else if downloading {
                            tr("Downloading update...").to_owned()
                        } else if let Some(version) = staged_version.as_ref() {
                            format!("✓ v{version}")
                        } else if let Some(version) = latest_version.as_ref().filter(|_| newer) {
                            format!("↑ v{version}")
                        } else if latest_version.is_some() {
                            tr("Up to date").to_owned()
                        } else {
                            String::new()
                        };
                        if !status.is_empty() {
                            ui.label(
                                RichText::new(status)
                                    .size(8.5)
                                    .color(if staged_version.is_some() {
                                        GOOD
                                    } else if newer {
                                        WARN
                                    } else {
                                        DIM
                                    }),
                            );
                        }
                    });
                });

                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    let mut automatic = app.preferences.check_updates_on_start;
                    let automatic_response = ui.add(
                        egui::Checkbox::new(&mut automatic, RichText::new("Auto").size(9.0))
                    ).on_hover_text(tr("Check for new versions on startup"));
                    if automatic_response.changed() {
                        let mut next = app.preferences;
                        next.check_updates_on_start = automatic;
                        app.save_app_preferences(next);
                    }

                    let check_response = ui.add_enabled(
                        !checking && !downloading,
                        egui::Button::new(RichText::new("↻").size(10.0)),
                    ).on_hover_text(tr("Check now"));
                    if check_response.clicked() {
                        match update::check_now(&app.update_status) {
                            Ok(_) => app.set_status(tr("Update check started"), false),
                            Err(error) => app.set_status(error, true),
                        }
                        ui.ctx().request_repaint();
                    }

                    #[cfg(windows)]
                    {
                        if staged_version.is_some() {
                            let active = app.accounts_have_live_work();
                            let response = ui.add_enabled(
                                !active,
                                egui::Button::new(
                                    RichText::new(tr("Restart & install")).size(9.0)
                                ),
                            );
                            let response = if active {
                                response.on_hover_text(tr(
                                    "Stop all active accounts before restarting to install it."
                                ))
                            } else {
                                response
                            };
                            if response.clicked() {
                                match update::request_restart_and_install(&app.update_status) {
                                    Ok(_) => {
                                        app.set_status(tr("Restarting to install update"), false);
                                        app.tray_exit_requested.store(
                                            true,
                                            std::sync::atomic::Ordering::Release,
                                        );
                                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                                    }
                                    Err(error) => app.set_status(error, true),
                                }
                            }
                        } else if newer && !downloading {
                            let response = ui.add_enabled(
                                !checking,
                                egui::Button::new(
                                    RichText::new(tr("Download")).size(9.0)
                                ),
                            );
                            if response.clicked() {
                                match update::download_update(&app.update_status) {
                                    Ok(_) => app.set_status(tr("Update download started"), false),
                                    Err(error) => app.set_status(error, true),
                                }
                                ui.ctx().request_repaint();
                            }
                        }
                    }

                    #[cfg(not(windows))]
                    if newer {
                        ui.hyperlink_to(
                            RichText::new(tr("Release notes")).size(9.0),
                            _notes_url.as_deref().unwrap_or(update::RELEASES_URL),
                        );
                    }

                    if let Some(error) = failure.as_ref() {
                        ui.label(RichText::new("!").size(10.0).strong().color(BAD))
                            .on_hover_text(error.as_str());
                    }
                });
            });
        });
}
