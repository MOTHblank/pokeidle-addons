//! Workspace navigation, toolbar actions and account switching presentation.
use super::*;

/// Shared compact toolbar for both single-row and wrapped headers.
pub(super) fn draw_header_actions(app: &mut ControllerApp, ui: &mut egui::Ui) {
    if ui.add(
        egui::Button::new(RichText::new(tr("Launch enabled")).strong().size(11.0))
            .fill(ACCENT.linear_multiply(0.8))
            .stroke(Stroke::new(1.0, ACCENT))
            .corner_radius(8.0),
    ).on_hover_text(tr("Start all enabled accounts")).clicked() {
        app.launch_enabled();
    }
    if ui.add(
        egui::Button::new(RichText::new(tr("Stop all")).strong().size(11.0))
            .fill(BAD.linear_multiply(0.14))
            .stroke(Stroke::new(1.0, BAD.linear_multiply(0.55)))
            .corner_radius(8.0),
    ).on_hover_text(tr("Stop all running accounts")).clicked() {
        app.stop_all();
    }
    #[cfg(windows)]
    if ui.add(
        egui::Button::new(RichText::new(tr("To tray")).size(11.0))
            .fill(PANEL)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(8.0),
    ).on_hover_text(tr("Hide this window in the system tray; click the tray icon to restore it"))
    .clicked() {
        if app.tray.is_some() {
            app.tray_hidden.store(true, Ordering::Relaxed);
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(false));
            logging::info("controller window hidden to system tray");
        } else {
            app.set_status(tr("System tray is unavailable; the window cannot be hidden.").to_owned(), true);
        }
    }
}

/// Same compact account navigation is reused by every native data page and
/// the account manager. Clicking one account changes the workspace selection,
/// not the running game. Status always reflects the real native session.
pub(super) fn draw_account_switcher(
    app: &ControllerApp,
    ui: &mut egui::Ui,
    selected: Option<usize>,
    include_all: bool,
) -> Option<usize> {
    let mut next = selected;
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8.0)
        .inner_margin(Margin::symmetric(9, 7))
        .show(ui, |ui| {
            let button_width = if ui.available_width() < 780.0 { 112.0 } else { 128.0 };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("ACCOUNT")).size(10.0).strong().color(DIM));
                if include_all {
                    let is_all = selected.is_none();
                    if ui.add_sized([105.0, 34.0],
                        egui::Button::new(RichText::new(tr("All accounts")).size(11.0))
                            .fill(if is_all { ACCENT.linear_multiply(0.20) } else { PANEL_ALT })
                            .stroke(Stroke::new(1.0, if is_all { ACCENT } else { BORDER }))
                            .corner_radius(6.0)).clicked() {
                        next = None;
                    }
                }
                for index in 0..app.runtimes.len() {
                    let slot = &app.runtimes[index].game;
                    let profile = slot.profile;
                    let (state, color) = if let Some(native) = slot.native.as_ref() {
                        let snapshot = native.snapshot();
                        match snapshot.phase.as_str() {
                            "Online" if snapshot.last_received
                                .is_some_and(|at| at.elapsed() <= Duration::from_secs(90)) =>
                                (tr("Online"), GOOD),
                            "Online" => (tr("Stale"), WARN),
                            "NeedsLogin" => (tr("Sign-in required"), WARN),
                            "Reconnecting" => (tr("Reconnecting"), WARN),
                            "Offline" => (tr("Offline"), DIM),
                            _ => (tr("Waiting for data"), WARN),
                        }
                    } else if slot.stopping {
                        (tr("Stopping"), WARN)
                    } else if slot.shutdown_failed {
                        (tr("Stop failed"), BAD)
                    } else if app.runtimes[index].session.cached_native_busy || app.runtimes[index].session.native_verifying
                        || app.runtimes[index].session.native_handoff_busy || slot.handoff_closing {
                        (tr("Starting"), WARN)
                    } else if slot.child.is_some() || slot.orphaned_pid.is_some() {
                        (tr("Firefox"), WARN)
                    } else if !app.accounts[index].enabled {
                        (tr("Disabled"), DIM)
                    } else {
                        (tr("Offline"), DIM)
                    };
                    let selected_here = selected == Some(index);
                    let name = compact_text(&app.accounts[index].name, 15);
                    let text = format!("{} {}\n{}", if color == GOOD { "●" } else if color == WARN { "◐" } else { "○" },
                        name, state);
                    let response = ui.add_sized([button_width, 40.0],
                        egui::Button::new(RichText::new(text).size(10.0)
                            .color(if selected_here { TEXT } else { MUTED }))
                            .fill(if selected_here {
                                ACCENT.linear_multiply(0.20)
                            } else {
                                PANEL_ALT
                            })
                            .stroke(Stroke::new(1.0, if selected_here { ACCENT }
                                else if color == GOOD { GOOD.linear_multiply(0.45) }
                                else if color == WARN { WARN.linear_multiply(0.45) }
                                else { BORDER }))
                            .corner_radius(6.0));
                    if response.on_hover_text(format!("{} · {}", profile.label(), state)).clicked() {
                        next = Some(index);
                    }
                }
            });
        });
    next
}

/// Performance preferences persist independently of accounts and profiles.
pub(super) fn draw_options_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_salt("options-page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let width = ui.available_width().min(720.0);
            ui.allocate_ui_with_layout(
                egui::vec2(width, 0.0), Layout::top_down(Align::Min),
                |ui| draw_options_contents(app, ui),
            );
        });
}

fn draw_options_contents(app: &mut ControllerApp, ui: &mut egui::Ui) {
    ui.label(RichText::new(tr("Performance and updates"))
        .size(17.0).strong().color(TEXT));
    ui.add_space(5.0);
    ui.label(RichText::new(tr("Choose when Moth Controller uses fewer resources in the background."))
        .size(11.0).color(MUTED));
    ui.add_space(12.0);

    let mut next = app.ui_options;
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(egui::Margin::same(16)).show(ui, |ui| {
            ui.label(RichText::new(tr("Background refresh"))
                .size(13.0).strong().color(TEXT));
            ui.add_space(8.0);

            ui.checkbox(
                &mut next.reduce_updates_unfocused,
                tr("Reduce updates when the window is not focused"),
            );
            ui.label(RichText::new(tr(
                "When enabled, refresh this window less often while you use other apps. The game keeps running normally."
            )).size(10.0).color(MUTED));

            ui.add_space(10.0);
            ui.checkbox(
                &mut next.reduce_updates_in_tray,
                tr("Reduce updates when minimized to the system tray"),
            );
            ui.label(RichText::new(tr(
                "On: slower refresh while hidden in the tray. Off: normal foreground refresh continues while hidden."
            )).size(10.0).color(MUTED));

            ui.add_space(10.0);
            ui.separator();
            ui.label(RichText::new(tr(
                "PokéIdle and Twitch keep running normally. These options only change how often the window and Firefox update."
            )).size(10.0).color(DIM));
        });

    if next != app.ui_options {
        // The change is applied only after a successful disk write. If the
        // options directory is unavailable, keep the previous live setting.
        app.save_ui_options(next);
        ui.ctx().request_repaint();
    }

    ui.add_space(12.0);
    let slowed=app.controller_in_background(ui.ctx());
    ui.label(RichText::new(format!("{}: {}",
        tr("Current refresh mode"),
        tr(if slowed {"Reduced background updates"} else {"Normal foreground updates"})))
        .size(11.0).color(if slowed { WARN } else { GOOD }));
    if ui.small_button(tr("Restore default performance settings")).clicked() {
        // Performance reset must not wipe per-account Hunt Atlas choices.
        app.save_ui_options(UiOptions {
            atlas_aoe_enabled: app.ui_options.atlas_aoe_enabled,
            ..UiOptions::default()
        });
    }
    ui.label(RichText::new(tr("Changes are saved automatically and restored on the next launch."))
        .size(11.0).color(DIM));
}

pub(super) fn draw_sidebar(app: &mut ControllerApp, ui: &mut egui::Ui) {
    egui::Panel::left("sidebar")
        .resizable(false)
        .default_size(206.0)
        .frame(
            egui::Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER))
                .inner_margin(16.0),
        )
        .show(ui, |ui| {
            ui.label(
                RichText::new("MOTH")
                    .font(FontId::proportional(22.0))
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new("POKEIDLE")
                    .font(FontId::proportional(10.0))
                    .strong()
                    .color(ACCENT),
            );
            ui.add_space(22.0);

            section_label(ui, tr("WORKSPACE"));

            if sidebar_button(ui, tr("Dashboard"), app.workspace.panel_tab == PanelTab::Overview).clicked() {
                app.workspace.panel_tab = PanelTab::Overview;
            }

            if sidebar_button(ui, &format!("  ·  {}", tr("Hunt Atlas")), app.workspace.panel_tab == PanelTab::Hunts).clicked() {
                app.workspace.panel_tab = PanelTab::Hunts;
            }
            if sidebar_button(ui, &format!("  ·  {}", tr("Boss Manager")), app.workspace.panel_tab == PanelTab::Boss).clicked() {
                app.workspace.panel_tab = PanelTab::Boss;
            }

            if sidebar_button(ui, tr("Accounts"),
                app.workspace.panel_tab == PanelTab::Accounts &&
                app.workspace.accounts_section == AccountsSection::Profiles).clicked() {
                app.workspace.panel_tab = PanelTab::Accounts;
                app.workspace.accounts_section = AccountsSection::Profiles;
            }
            if sidebar_button(ui, "  ·  Kick",
                app.workspace.panel_tab == PanelTab::Accounts &&
                app.workspace.accounts_section == AccountsSection::Kick).clicked() {
                app.workspace.panel_tab = PanelTab::Accounts;
                app.workspace.accounts_section = AccountsSection::Kick;
            }

            if sidebar_button(ui, tr("Automation"), app.workspace.panel_tab == PanelTab::Automation).clicked() {
                app.workspace.panel_tab = PanelTab::Automation;
            }

            if sidebar_button(ui, tr("Logs"), false).clicked() {
                app.open_logs();
            }

            if sidebar_button(ui, tr("Diagnostics"),
                app.workspace.panel_tab == PanelTab::Diagnostics).clicked() {
                app.workspace.panel_tab = PanelTab::Diagnostics;
            }

            if sidebar_button(ui, tr("Options"), app.workspace.panel_tab == PanelTab::Options).clicked() {
                app.workspace.panel_tab = PanelTab::Options;
            }

            // Global start/stop actions are in the header; avoid a duplicate
            // control set that crowds the persistent navigation.
            ui.add_space(18.0);
            section_label(ui, tr("LANGUAGE"));

            ui.horizontal(|ui| {
                let en_selected = !pt_br();
                let pt_selected = pt_br();

                let en_button = egui::Button::new(
                    RichText::new("EN").size(10.0).strong(),
                )
                .fill(if en_selected {
                    ACCENT.linear_multiply(0.22)
                } else {
                    PANEL_ALT
                })
                .stroke(Stroke::new(
                    1.0,
                    if en_selected { ACCENT } else { BORDER },
                ))
                .corner_radius(7.0);

                if ui.add_sized([78.0, 30.0], en_button).clicked() {
                    PT_BR.store(false, Ordering::Relaxed);
                }

                let pt_button = egui::Button::new(
                    RichText::new("PT-BR").size(10.0).strong(),
                )
                .fill(if pt_selected {
                    ACCENT.linear_multiply(0.22)
                } else {
                    PANEL_ALT
                })
                .stroke(Stroke::new(
                    1.0,
                    if pt_selected { ACCENT } else { BORDER },
                ))
                .corner_radius(7.0);

                if ui.add_sized([78.0, 30.0], pt_button).clicked() {
                    PT_BR.store(true, Ordering::Relaxed);
                }
            });

            ui.add_space(20.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(10.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(tr("CONNECTIONS"))
                            .size(10.0)
                            .strong()
                            .color(DIM),
                    );
                    ui.add_space(7.0);
                    let active: Vec<_> = app.runtimes.iter().map(|runtime| &runtime.game)
                        .filter(|game| game.child.is_some() || game.orphaned_pid.is_some())
                        .collect();
                    let native_online = app.runtimes.iter().map(|runtime| &runtime.game).filter(|game|
                        game.native.as_ref().is_some_and(|native|
                            native.snapshot().phase == "Online")).count();
                    let mode = if active.is_empty() {
                        tr("STOPPED")
                    } else if active.iter().all(|game| game.headless) {
                        tr("HIDDEN")
                    } else if active.iter().all(|game| !game.headless) {
                        tr("VISIBLE")
                    } else {
                        tr("MIXED")
                    };
                    let connected = active.iter()
                        .filter(|game| game.monitor.as_ref().is_some_and(|monitor| monitor.health().heartbeat_is_fresh()))
                        .count();
                    runtime_row(ui, tr("PokéIdle"), &format!("{native_online}/4 online"),
                        if native_online>0 {GOOD} else {MUTED});
                    runtime_row(ui, "Firefox", mode, if active.is_empty() { MUTED } else { GOOD });
                    if !active.is_empty() {
                        runtime_row(ui, tr("Firefox login"), &format!("{connected}/{} {}", active.len(), tr("Connected")),
                            if connected == active.len() { GOOD } else { MUTED });
                    }
                });

            ui.add_space(12.0);
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.add_space(6.0);
                draw_credits(ui, &app.update_status);
                ui.add_space(10.0);
                ui.label(
                    RichText::new(if cfg!(windows) { tr("Moth Controller · Windows") } else { tr("Moth Controller · Linux") })
                        .size(10.0)
                        .color(DIM),
                );
            });
        });
}

pub(super) fn draw_instance_section(app: &mut ControllerApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("ACCOUNT OVERVIEW"))
            .font(FontId::proportional(17.0)).strong().color(TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.selectable_label(app.workspace.dashboard_session_view, tr("Session")).clicked() {
                app.workspace.dashboard_session_view = true;
            }
            if ui.selectable_label(!app.workspace.dashboard_session_view, tr("Summary")).clicked() {
                app.workspace.dashboard_session_view = false;
            }
        });
    });
    ui.add_space(9.0);

    egui::ScrollArea::vertical()
        .id_salt("instances_page")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let profiles: Vec<usize> = GameProfile::ALL.iter()
                .map(|profile| profile.index())
                .filter(|index| app.accounts[*index].enabled)
                .collect();

            if profiles.is_empty() && app.workspace.selected_details.is_none() {
                ui.label(RichText::new(tr(
                    "No accounts enabled. Open Account Manager to add or enable one."
                )).size(12.0).color(DIM));
            } else {
                // The dashboard matches the React manager: aggregate metrics
                // above a responsive grid of account cards. Details replace
                // the grid rather than leaving an empty two-column workspace.
                if app.workspace.selected_details.is_none() {
                    // Freeze one display sample per enabled account for this
                    // frame. Totals, cards and status widgets share it, rather
                    // than each cloning a large live hunt/roster snapshot.
                    let samples: [Option<OverviewAccountSample>; 4] =
                        std::array::from_fn(|index| {
                            app.accounts[index].enabled.then(||
                                OverviewAccountSample::capture(&app.runtimes[index].game))
                        });
                    let totals = collect_overview_totals(app, &profiles, &samples);
                    draw_overview_totals(ui, &totals);
                    ui.add_space(12.0);
                    let gap = 12.0;
                    let available = ui.available_width();
                    let columns = if available >= 800.0 { 2 } else { 1 };
                    let card_width = if columns == 2 {
                        (available - gap) / 2.0
                    } else { available };
                    for chunk in profiles.chunks(columns) {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for index in chunk {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(card_width, 0.0),
                                    Layout::top_down(Align::Min),
                                    |ui| draw_overview_card(
                                        app, ui, *index, card_width,
                                        samples[*index].as_ref().expect("enabled account sampled"),
                                    ),
                                );
                            }
                        });
                        ui.add_space(10.0);
                    }
                }

                if let Some(index) = app.workspace.selected_details {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(format!(
                            "{} {}", tr("Details for"),
                            app.account_name(app.runtimes[index].game.profile)
                        )).size(15.0).strong().color(TEXT));
                        ui.add_space(8.0);
                        if ui.selectable_label(!app.workspace.dashboard_show_history, tr("Live details")).clicked() {
                            app.workspace.dashboard_show_history = false;
                        }
                        if ui.selectable_label(app.workspace.dashboard_show_history, tr("History")).clicked() {
                            app.workspace.dashboard_show_history = true;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button(tr("Hide details")).clicked() {
                                app.workspace.selected_details = None;
                            }
                        });
                    });
                    ui.add_space(8.0);
                    if app.workspace.selected_details == Some(index) {
                        let width = ui.available_width().min(900.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(width, 0.0),
                            Layout::top_down(Align::Min),
                            |ui| {
                                if app.workspace.dashboard_show_history {
                                    draw_history_panel(app, ui, index);
                                } else {
                                    draw_game_details(app, ui, index, width);
                                }
                            },
                        );
                    }
                }
            }

            ui.add_space(10.0);
            ui.collapsing(tr("Technical details"), |ui| {
                draw_runtime_section(app, ui);
            });
        });
}

