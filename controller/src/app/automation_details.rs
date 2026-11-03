//! Expanded browser/account details; native diagnostics route to automation_manager.
use super::*;
use super::automation_manager::draw_native_manager_details;

pub(super) fn draw_game_details(
    app: &mut ControllerApp,
    ui: &mut egui::Ui,
    index: usize,
    width: f32,
) {
    ui.set_width(width);
    if app.games[index].native.is_some() {
        draw_native_manager_details(app, ui, index);
        return;
    }

    let profile = app.games[index].profile;
    let account_name = app.account_name(profile);
    let health = app.games[index].health();
    let running = app.games[index].is_running();

    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(12.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            // Browser controls and warnings live in the compact overview.
            // Keep the full-width detail panel dedicated to game data.
            ui.add_space(2.0);

            ui.add_space(14.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(
                            if running && health.last_successful_probe_at.is_some() && !health.telemetry_is_fresh() {
                                tr("LAST OBSERVED STATE")
                            } else if health.activity == "Hunting" {
                                tr("CURRENT ACTIVITY")
                            } else {
                                tr("STATE")
                            },
                        )
                        .size(9.0)
                        .strong()
                        .color(DIM),
                    );

                    ui.add_space(3.0);

                    let headline = match health.activity.as_str() {
                        "Hunting" if !health.hunt.is_empty() => {
                            format!("{} · {}", tr("Hunting"), compact_text(&health.hunt, 34))
                        }
                        "Center" => tr("Online · Center").to_string(),
                        "Login" => tr("Waiting for login").to_string(),
                        _ => localize_status(health.summary()),
                    };

                    ui.add(
                        egui::Label::new(
                            RichText::new(compact_text(&headline, 52))
                                .size(18.0)
                                .strong()
                                .color(TEXT),
                        )
                        .truncate(),
                    );

                    if !health.active_pokemon.is_empty() {
                        ui.add_space(3.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(compact_text(&health.active_pokemon, 54))
                                    .size(11.0)
                                    .color(MUTED),
                            )
                            .truncate(),
                        );
                    }

                    ui.add_space(9.0);

                    let trainer_level = if health.player_level == 0 {
                        "—".to_string()
                    } else {
                        health.player_level.to_string()
                    };
                    let fallen = health.fallen_count.to_string();

                    // Stable two-column progress summary: trainer and Pokémon
                    // stats no longer wrap into an ambiguous sequence of labels.
                    ui.columns(2, |columns| {
                        columns[0].label(RichText::new(tr("TRAINER")).size(9.0).strong().color(DIM));
                        columns[0].add_space(3.0);
                        detail_stat_line(&mut columns[0], "Level", &trainer_level);
                        detail_stat_line(&mut columns[0], "XP", if health.player_xp.is_empty() { "—" } else { &health.player_xp });

                        columns[1].label(RichText::new(tr("POKÉMON")).size(9.0).strong().color(DIM));
                        columns[1].add_space(3.0);
                        detail_stat_line(&mut columns[1], "Level", if health.pokemon_level.is_empty() { "—" } else { &health.pokemon_level });
                        detail_stat_line(&mut columns[1], "XP", if health.pokemon_xp.is_empty() { "—" } else { &health.pokemon_xp });
                        detail_stat_line(&mut columns[1], "FALLEN", &fallen);
                    });
                });

            // HuntAtlas shares already-calculated results through the Bridge.
            // An absent or mismatched snapshot is not a rate of zero.
            let atlas = &health.hunt_atlas;
            let matches_hunt = atlas.get("huntSlug").and_then(|v| v.as_str())
                .is_some_and(|slug| !slug.is_empty() && slug == health.hunt_slug);
            if matches_hunt {
                let rate = |key: &str| -> String {
                    atlas.get(key).and_then(|v| v.as_f64())
                        .filter(|n| n.is_finite() && *n >= 0.0)
                        .map(|n| if n < 1.0 { format!("{n:.3}") } else { format!("{n:.1}") })
                        .unwrap_or_else(|| "—".to_string())
                };
                ui.add_space(10.0);
                ui.label(RichText::new(tr("HUNT ATLAS")).size(10.0).strong().color(DIM));
                egui::Frame::new().fill(PANEL_ALT).corner_radius(10.0)
                    .inner_margin(10.0).show(ui, |ui| {
                        ui.columns(2, |cols| {
                            detail_stat_line(&mut cols[0], tr("Trainer XP/h"), &rate("xpH"));
                            let eta = match (health.player_xp_to_next,
                                atlas.get("xpH").and_then(|v| v.as_f64())) {
                                (Some(remaining), Some(xp)) if xp > 0.0 =>
                                    format!("{:.1} h", remaining as f64 / xp),
                                _ => "—".to_string(),
                            };
                            detail_stat_line(&mut cols[0], tr("Next level"), &eta);
                            detail_stat_line(&mut cols[0], tr("Coins/h"), &rate("coinsH"));
                            detail_stat_line(&mut cols[1], tr("Captures/h"), &rate("capturesH"));
                            let shiny_interval = atlas.get("shinyCaughtH").and_then(|v| v.as_f64())
                                .filter(|rate| *rate > 0.0).map(|rate| {
                                    let hours = 1.0 / rate;
                                    if hours >= 48.0 { format!("{:.1} days", hours / 24.0) }
                                    else { format!("{hours:.1} h") }
                                }).unwrap_or_else(|| "—".to_string());
                            detail_stat_line(&mut cols[1], tr("Shiny wait"), &shiny_interval);
                            detail_stat_line(&mut cols[1], tr("Balls/capture"), &rate("ballsPerCapture"));
                        });
                        ui.separator();
                        let chance = |key: &str| -> String {
                            atlas.get(key).and_then(|v| v.as_f64())
                                .filter(|n| n.is_finite() && *n >= 0.0)
                                .map(|n| format!("{:.2}%", n * 100.0))
                                .unwrap_or_else(|| "—".to_string())
                        };
                        detail_stat_line(ui, tr("Ultra Ball"), &chance("ultraChance"));
                        detail_stat_line(ui, tr("Beast Ball"), &chance("beastChance"));
                        if let Some(tier) = atlas.get("activeOutlandTier").and_then(|v| v.as_u64()) {
                            detail_stat_line(ui, tr("Outland tier"), &tier.to_string());
                        }
                        if let Some(drops) = atlas.get("rareDrops").and_then(|v| v.as_array()) {
                            for drop in drops {
                                if let (Some(name), Some(rate)) = (drop.get("name").and_then(|v| v.as_str()),
                                    drop.get("unitsH").and_then(|v| v.as_f64())) {
                                    let label = if rate > 0.0 && rate < 0.01 {
                                        format!("{:.1} days/drop", 1.0 / (rate * 24.0))
                                    } else { format!("{rate:.3}/h") };
                                    detail_stat_line(ui, name, &label);
                                }
                            }
                        }
                        ui.label(RichText::new(if atlas.get("xpMeasured").and_then(|v| v.as_bool()) == Some(true) {
                            tr("Measured XP/h · other rates may be estimated")
                        } else { tr("Estimated rates · not measured") }).size(9.0).color(DIM));
                    });
            }

            ui.add_space(12.0);

            ui.label(
                RichText::new(tr("RESOURCES"))
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
                    let gold = format_number(health.gold);
                    let orbs = format_number(health.orbs);
                    let balls_used = health.autocatch_balls_used.to_string();
                    let captures = health.autocatch_captures.to_string();
                    let bridge_ok = health.bridge_snapshot_ok;

                    // Group balances and capture statistics independently.
                    // Auto Catch ON/OFF is already shown under Automation.
                    ui.columns(2, |columns| {
                        columns[0].label(RichText::new(tr("CURRENCY")).size(9.0).strong().color(DIM));
                        columns[0].add_space(3.0);
                        detail_stat_line(&mut columns[0], "GOLD", if bridge_ok { &gold } else { "—" });
                        detail_stat_line(&mut columns[0], "GEMS", if bridge_ok { &orbs } else { "—" });

                        columns[1].label(RichText::new(tr("CAPTURE RESULTS")).size(9.0).strong().color(DIM));
                        columns[1].add_space(3.0);
                        detail_stat_line(&mut columns[1], "BALLS USED", if bridge_ok { &balls_used } else { "—" });
                        detail_stat_line(&mut columns[1], "CAPTURES", if bridge_ok { &captures } else { "—" });
                        detail_stat_line(&mut columns[1], "SUCCESS", if bridge_ok && !health.autocatch_rate.is_empty() { &health.autocatch_rate } else { "—" });
                    });

                    ui.add_space(9.0);
                    ui.separator();
                    ui.add_space(5.0);
                    ui.label(RichText::new(tr("BALL STOCK")).size(9.0).strong().color(DIM));
                    ui.add_space(5.0);
                    if health.ball_stock.is_empty() {
                        ui.label(RichText::new(tr("Pokéballs: unavailable")).size(10.0).color(MUTED));
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for item in &health.ball_stock {
                                resource_chip(ui, item);
                            }
                        });
                    }
                });

            ui.add_space(12.0);

            ui.label(
                RichText::new(tr("AUTOMATION"))
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            if app.games[index].native.is_some() {
                draw_native_automation(app, ui, index);
            } else {
                ui.label(RichText::new(tr(
                    "Automation is managed in the controller. Start this account to see its current settings."
                )).size(10.0).color(MUTED));
            }

            // Browser-only controls are irrelevant after successful native
            // takeover. The runtime and saved session now own game + IRC.
            if app.games[index].native.is_none() {
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
                            app.tab_url_open[index] = !app.tab_url_open[index];
                        }
                    });
                });
                ui.add_space(6.0);

                egui::Frame::new()
                    .fill(PANEL_ALT)
                    .corner_radius(10.0)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        if app.tab_url_open[index] {
                            ui.horizontal(|ui| {
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut app.tab_url_input[index])
                                        .hint_text(tr("https://example.com"))
                                        .desired_width((width - 150.0).max(180.0))
                                );

                                let enter = response.lost_focus()
                                    && ui.input(|input| input.key_pressed(egui::Key::Enter));

                                if ui.button(tr("Open")).clicked() || enter {
                                    let raw = app.tab_url_input[index].trim();
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
                                            if let Some(monitor) = app.games[index].monitor.as_ref() {
                                                match monitor.send(json!({
                                                    "t": "browser.openTab",
                                                    "url": candidate
                                                })) {
                                                    Ok(id) => {
                                                        app.tab_url_input[index].clear();
                                                        app.tab_url_open[index] = false;
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
                                    app.tab_url_open[index] = false;
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
                                    let marker = if tab.kind == "Twitch" {
                                        if tab.low_resource { "●" } else { "○" }
                                    } else {
                                        "●"
                                    };

                                    ui.label(
                                        RichText::new(marker)
                                            .size(9.0)
                                            .color(if tab.kind == "Twitch" && tab.low_resource { GOOD } else { MUTED }),
                                    );

                                    ui.label(
                                        RichText::new(if tab.kind == "Twitch" && tab.twitch_channels.is_some() { tr("Shared Twitch chat") } else { tr(&tab.kind) })
                                            .size(9.0)
                                            .strong()
                                            .color(TEXT),
                                    );

                                    ui.add_space(5.0);

                                    let channel_label = tab.twitch_channels.map(|n| format!("{} {}", n, tr("channels in this tab")));
                                    let title = channel_label.as_deref().unwrap_or_else(|| if tab.title.is_empty() { &tab.url } else { &tab.title });

                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(compact_text(title, 54))
                                                .size(9.0)
                                                .color(MUTED),
                                        )
                                        .truncate(),
                                    );

                                    if tab.kind == "Twitch" {
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            ui.label(
                                                RichText::new(if tab.low_resource { tr("LOW RESOURCE") } else { tr("Not optimized") })
                                                    .size(8.0)
                                                    .strong()
                                                    .color(if tab.low_resource { GOOD } else { WARN }),
                                            );
                                        });
                                    }
                                });
                            }
                        }
                    });

            }

            ui.add_space(12.0);
            ui.label(
                RichText::new(tr("STREAMS & BONUS"))
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
                    if !health.bridge_snapshot_ok {
                        ui.label(RichText::new(tr("Twitch XP bonus unavailable until game data loads")).size(10.0).color(DIM));
                    } else if !health.stream_watching.is_empty() && health.stream_bonus_pct > 0.0 {
                        ui.label(
                            RichText::new(format!(
                                "{} · +{}% XP",
                                tr("ACTIVE"),
                                format_pct(health.stream_bonus_pct)
                            ))
                            .size(11.0)
                            .strong()
                            .color(GOOD),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                tr("Watching:"),
                                health.stream_watching.join(", ")
                            ))
                            .size(10.0)
                            .color(MUTED),
                        );
                    } else if !health.stream_missing.is_empty() {
                        ui.label(
                            RichText::new(tr("LIVE BONUS AVAILABLE"))
                                .size(10.0)
                                .strong()
                                .color(WARN),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} {}{}",
                                tr("Bonus not registered for:"),
                                health.stream_missing.join(", "),
                                if health.stream_bonus_pct > 0.0 {
                                    format!(" · +{}% XP", format_pct(health.stream_bonus_pct))
                                } else {
                                    String::new()
                                }
                            ))
                            .size(10.0)
                            .color(TEXT),
                        );
                    } else {
                        ui.label(
                            RichText::new(tr(if health.stream_watching.is_empty() { "No Twitch XP bonus confirmed by the game" } else { "Chat activity reported; XP bonus not confirmed" }))
                                .size(10.0)
                                .color(DIM),
                        );
                    }
                    ui.label(RichText::new(tr("Twitch channels and game-reported XP bonuses are different signals.")).size(9.0).color(DIM));

                    ui.add_space(6.0);

                    ui.horizontal_wrapped(|ui| {
                        for tab in health.tabs.iter().filter(|tab| {
                            tab.kind == "Twitch" || tab.kind == "KICK"
                        }) {
                            let state = if tab.kind == "Twitch" {
                                if tab.low_resource { tr("LOW") } else { tr("not optimized") }
                            } else {
                                tr("OPEN")
                            };

                            ui.label(
                                RichText::new(format!(
                                    "{} · {} · {}",
                                    tab.kind,
                                    compact_text(
                                        tab.title.rsplit('/').next().unwrap_or(&tab.title),
                                        24
                                    ),
                                    state
                                ))
                                .size(9.0)
                                .color(if tab.kind == "Twitch" && tab.low_resource {
                                    GOOD
                                } else {
                                    MUTED
                                }),
                            );
                        }
                    });
                });

            ui.add_space(12.0);
            ui.label(
                RichText::new(tr("XP BONUS SOURCES"))
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
                    let mut sources = health.xp_sources.clone();
                    for bonus in &health.xp_bonuses {
                        if !sources.iter().any(|source| source == bonus) {
                            sources.push(bonus.clone());
                        }
                    }

                    if sources.is_empty() {
                        ui.label(
                            RichText::new(tr(if health.bridge_snapshot_ok { "No active XP bonus detected" } else { "XP bonus data unavailable" }))
                                .size(10.0)
                                .color(DIM),
                        );
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for source in sources {
                                bonus_chip(ui, &source);
                            }
                        });
                    }
                });

        });
}

