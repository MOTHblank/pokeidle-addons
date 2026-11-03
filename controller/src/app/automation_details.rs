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

            draw_browser_controls(app, ui, index, width, &account_name, &health);
            draw_stream_bonus_details(ui, &health);

        });
}

