//! Firefox-only Kick and tab controls, using the already cached Health snapshot.
//! Called from the expanded account view. No extra monitoring or polling.
use super::{compact_text, egui, json, multikick_link,
    open_kick_channels_in_regular_browser, tr, Align, ControllerApp, Health,
    Layout, RichText, ACCENT, DIM, GOOD, MUTED, PANEL_ALT, TEXT};

pub(super) fn draw_browser_controls(
    app: &mut ControllerApp,
    ui: &mut egui::Ui,
    index: usize,
    width: f32,
    account_name: &str,
    health: &Health,
) {
            // The extracted browser-only panel still uses the managed
            // Firefox process state to distinguish closed from stale tabs.
            let running=app.runtimes[index].game.is_running();
            // Browser-only controls are irrelevant after successful native
            // takeover. The runtime and saved session now own game + IRC.
            if app.runtimes[index].game.native.is_none() {
                ui.add_space(12.0);
                ui.label(
                    RichText::new(tr("KICK MULTISTREAM"))
                        .size(9.0)
                        .strong()
                        .color(DIM),
                );
                ui.add_space(6.0);

                egui::Frame::new()
                    .fill(PANEL_ALT)
                    .corner_radius(10.0)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        if let Some((link, channels)) = multikick_link(&health.kick_streams) {
                            ui.label(
                                RichText::new(format!(
                                    "{} {}",
                                    channels.len(),
                                    tr("live KICK channels:")
                                ))
                                .size(10.0)
                                .strong()
                                .color(GOOD),
                            );
                            ui.label(
                                RichText::new(channels.join(" · "))
                                    .size(10.0)
                                    .color(TEXT),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(link.as_str())
                                    .size(10.0)
                                    .color(ACCENT),
                            );
                            ui.horizontal_wrapped(|ui| {
                                if ui.button(tr("Copy link")).clicked() {
                                    ui.ctx().copy_text(link.clone());
                                    app.set_status(
                                        tr("Copied MultiKick link. Open it in a regular browser, outside the controller-managed Firefox.").to_string(),
                                        false,
                                    );
                                }
                                if ui.button(tr("Open live channels")).clicked() {
                                    match open_kick_channels_in_regular_browser(&channels) {
                                        Ok(_) => app.set_status(
                                            tr("Opened live KICK channels in the regular browser.").to_string(),
                                            false,
                                        ),
                                        Err(error) => app.set_status(error, true),
                                    }
                                }
                            });
                            ui.label(
                                RichText::new(tr("Copy link copies a MultiKick URL; Open live channels opens each stream individually in your regular browser."))
                                    .size(9.0)
                                    .color(MUTED),
                            );
                        } else {
                            ui.label(
                                RichText::new(tr("No live KICK channels detected."))
                                    .size(10.0)
                                    .color(MUTED),
                            );
                        }
                    });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr(if health.telemetry_is_fresh() || health.last_successful_probe_at.is_none() { "OPEN TABS" } else { "Last reported tabs" }))
                            .size(9.0)
                            .strong()
                            .color(DIM),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let Some(age) = health.tab_inventory_age() {
                            ui.label(
                                RichText::new(format!("{} · {} {}",
                                    tr("Tabs checked"), age.as_secs() / 60, tr("min ago")))
                                    .size(9.0).color(DIM),
                            );
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("+")
                                        .size(13.0)
                                        .strong()
                                )
                                .min_size(egui::vec2(24.0, 22.0)),
                            )
                            .on_hover_text(tr("Open a custom URL in this Firefox profile"))
                            .clicked()
                        {
                            app.workspace.browser_tabs[index].open = !app.workspace.browser_tabs[index].open;
                        }
                    });
                });
                ui.add_space(6.0);

                egui::Frame::new()
                    .fill(PANEL_ALT)
                    .corner_radius(10.0)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        if app.workspace.browser_tabs[index].open {
                            ui.horizontal(|ui| {
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut app.workspace.browser_tabs[index].input)
                                        .hint_text(tr("https://example.com"))
                                        .desired_width((width - 150.0).max(180.0))
                                );

                                let enter = response.lost_focus()
                                    && ui.input(|input| input.key_pressed(egui::Key::Enter));

                                if ui.button(tr("Open")).clicked() || enter {
                                    let raw = app.workspace.browser_tabs[index].input.trim();
                                    if raw.is_empty() {
                                        app.set_status(
                                            format!(
                                                "{}: {}",
                                                &account_name,
                                                tr("Enter a URL.")
                                            ),
                                            true,
                                        );
                                    } else {
                                        // Accept bare domains, pasted URLs, and mixed-case schemes.
                                        // A malformed URL should not be reported as successfully queued.
                                        let candidate = if raw
                                            .get(..7)
                                            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
                                            || raw
                                                .get(..8)
                                                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
                                        {
                                            raw.to_string()
                                        } else if raw.contains("://") {
                                            String::new()
                                        } else {
                                            format!("https://{}", raw)
                                        };
                                        let has_http_scheme = candidate
                                            .get(..7)
                                            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
                                            || candidate
                                                .get(..8)
                                                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"));
                                        let host = candidate
                                            .split_once("://")
                                            .map(|(_, rest)| rest.split(['/', '?', '#']).next().unwrap_or_default())
                                            .unwrap_or_default();
                                        let valid_url = has_http_scheme
                                            && !host.is_empty()
                                            && !host.contains(char::is_whitespace);

                                        if valid_url {
                                            if let Some(monitor) = app.runtimes[index].game.monitor.as_ref() {
                                                match monitor.send(json!({
                                                    "t": "browser.openTab",
                                                    "url": candidate
                                                })) {
                                                    Ok(id) => {
                                                        app.workspace.browser_tabs[index].input.clear();
                                                        app.workspace.browser_tabs[index].open = false;
                                                        app.set_status(
                                                            format!("{} · {} #{}", &account_name, tr("custom tab queued"), id),
                                                            false,
                                                        );
                                                    }
                                                    Err(error) => app.set_status(
                                                        format!("{}: {}", &account_name, error),
                                                        true,
                                                    ),
                                                }
                                            } else {
                                                app.set_status(
                                                    format!("{}: {}", &account_name, tr("Browser monitor is not connected; tab was not queued.")),
                                                    true,
                                                );
                                            }
                                        } else {
                                            app.set_status(
                                                format!("{}: {}", &account_name, tr("Enter a valid http:// or https:// URL, or a domain such as example.com.")),
                                                true,
                                            );
                                        }
                                    }
                                }

                                if ui.button("×").clicked() {
                                    app.workspace.browser_tabs[index].open = false;
                                }
                            });

                            ui.add_space(6.0);
                        }

                        if health.tabs.is_empty() {
                            ui.label(
                                RichText::new(tr(if !running { "No active browser tabs" } else if !health.telemetry_is_fresh() { "Tab list not currently verified" } else if !health.game_ready { "Tab list unavailable while game loads" } else { "No top-level tabs reported" }))
                                    .size(10.0)
                                    .color(DIM),
                            );
                        } else {
                            for tab in &health.tabs {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("●").size(9.0).color(MUTED));
                                    ui.label(
                                        RichText::new(tr(&tab.kind))
                                            .size(9.0)
                                            .strong()
                                            .color(TEXT),
                                    );
                                    ui.add_space(5.0);
                                    let title=if tab.title.is_empty(){&tab.url}else{&tab.title};
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(compact_text(title,54))
                                                .size(9.0)
                                                .color(MUTED),
                                        )
                                        .truncate(),
                                    );
                                });
                            }
                        }
                    });

            }

}
