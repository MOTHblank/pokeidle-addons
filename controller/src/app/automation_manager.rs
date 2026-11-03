//! Read-only account, hunt and roster diagnostics built from cached native snapshots.
use super::{automation_badge, detail_stat_line, egui, format_hunt_amount,
    format_number, localize_status, request_item_catalog, tr, tr_dynamic,
    ControllerApp, PanelTab, RichText, SpriteKind, DIM, GOOD, MUTED, PANEL,
    PANEL_ALT, TEXT, WARN};

pub(super) fn draw_native_manager_details(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize) {
    // Take one read-only snapshot for the entire render. All network parsing
    // and commands remain on the game worker, never the egui render thread.
    let (status, view, twitch) = {
        let native = app.games[index].native.as_ref().expect("native details require a session");
        (native.snapshot(), native.hunt_snapshot(), native.twitch_snapshot())
    };
    let phase_online = status.phase == "Online";
    request_item_catalog(app);
    let item_catalog = app.inventory_catalog.clone();
    let hunt_name = view.hunts.iter()
        .find(|hunt| Some(hunt.slug.as_str()) == view.hunt_slug.as_deref())
        .map(|hunt| hunt.name.as_str())
        .or(view.hunt_slug.as_deref()).unwrap_or("—");
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .inner_margin(egui::Margin::same(14)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("ACCOUNT DETAILS")).size(12.0)
                    .strong().color(TEXT));
                automation_badge(ui, "PokéIdle connection", &status.phase, phase_online);
                automation_badge(ui, "Twitch", &twitch.phase,
                    twitch.phase == "Online");
                if let Some(last) = status.last_received {
                    ui.label(RichText::new(tr_dynamic(format!("Last game update: {}s ago",
                        last.elapsed().as_secs()))).size(10.0).color(MUTED));
                }
            });
            if !phase_online {
                ui.colored_label(WARN, tr("Live game state is not confirmed; values below may be stale."));
            }
            if let Some(error) = status.error.as_deref() {
                ui.label(RichText::new(tr_dynamic(localize_status(error.to_owned()))).size(10.0).color(WARN));
            }
            ui.add_space(9.0);
            ui.columns(3, |cols| {
                detail_stat_line(&mut cols[0], tr("Hunt"),
                    if view.in_center { tr("Pokémon Center") } else { hunt_name });
                detail_stat_line(&mut cols[0], tr("In-game nickname"),
                    view.account_nick.as_deref().unwrap_or("—"));
                detail_stat_line(&mut cols[0], tr("Trainer level"),
                    &view.trainer_level.map(|n| n.to_string()).unwrap_or_else(|| "—".into()));
                detail_stat_line(&mut cols[0], tr("Trainer XP"),
                    &view.trainer_xp.map(format_number).unwrap_or_else(|| "—".into()));

                detail_stat_line(&mut cols[1], tr("Active Pokémon"),
                    view.active_pokemon.as_ref().map(|p| p.name.as_str()).unwrap_or("—"));
                detail_stat_line(&mut cols[1], tr("Pokémon level"),
                    &view.active_pokemon.as_ref().map(|p|p.level.to_string())
                        .unwrap_or_else(|| "—".into()));
                detail_stat_line(&mut cols[1], tr("Pokémon XP"),
                    &view.active_pokemon.as_ref().and_then(|p|p.xp)
                        .map(format_number).unwrap_or_else(|| "—".into()));

                detail_stat_line(&mut cols[2], tr("Gold"),
                    &view.gold.map(format_number).unwrap_or_else(|| "—".into()));
                detail_stat_line(&mut cols[2], tr("Orbs"),
                    &view.orbs.map(format_number).unwrap_or_else(|| "—".into()));
                detail_stat_line(&mut cols[2], tr("Diamonds"),
                    &view.diamonds.map(format_number).unwrap_or_else(|| "—".into()));
            });
            if let Some(hunt) = view.hunts.iter().find(|hunt|
                Some(hunt.slug.as_str()) == view.hunt_slug.as_deref()) {
                ui.label(RichText::new(tr_dynamic(format!("{} · {} · Lv {} · {} spawn slots",
                    hunt.area, hunt.region, hunt.level, hunt.total_spawns)))
                    .size(10.0).color(DIM));
            }
            if let Some(map) = view.field_map.as_deref() {
                ui.label(RichText::new(format!("{}: {}",tr("Field map"),map))
                    .size(10.0).color(DIM));
            }
            if let Some(wild) = view.wild.as_ref() {
                ui.label(RichText::new(tr_dynamic(format!("Wild: {} · Lv {} · HP {}/{}",
                    wild.name, wild.level, wild.hp, wild.max_hp)))
                    .size(10.0).color(MUTED));
            }
            ui.horizontal_wrapped(|ui| {
                let bonus = &view.bonuses;
                for (label, percent) in [
                    ("Twitch", bonus.twitch_pct),
                    ("Guild", bonus.guild_pct),
                    ("Event trainer", bonus.trainer_event_pct),
                    ("Event Pokémon", bonus.pokemon_event_pct),
                ] {
                    ui.label(RichText::new(format!("{label} +{percent:.0}%"))
                        .size(10.0).color(DIM));
                }
                if bonus.vip_active { ui.label(RichText::new("VIP").color(GOOD)); }
                if bonus.guild_boost_active { ui.label(RichText::new(tr("Guild boost")).color(GOOD)); }
            });
        });
    ui.add_space(9.0);
    egui::Frame::new().fill(PANEL_ALT).corner_radius(10.0)
        .inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("HUNT SESSION")).size(12.0)
                    .strong().color(TEXT));
                for text in [
                    format!("{} {}",view.session.kills,tr("kills")),
                    format!("{} {}",view.session.captures,tr("captures")),
                    format!("{} {}",view.session.shiny_captured,tr("shinies caught")),
                    format!("{:02}:{:02}:{:02}",view.session.duration_secs/3600,
                        (view.session.duration_secs/60)%60,view.session.duration_secs%60),
                ] {
                    ui.label(RichText::new(text).size(10.0).color(MUTED));
                }
                if ui.small_button(tr("Open Hunt Atlas")).clicked() {
                    app.workspace.hunt_account = index;
                    app.workspace.panel_tab = PanelTab::Hunts;
                }
            });
            ui.columns(3, |cols| {
                let m = &view.measurements;
                detail_stat_line(&mut cols[0], tr("Trainer XP/h"),
                    &if m.valid_trainer { format_hunt_amount(m.trainer_xp_h) } else { "—".into() });
                detail_stat_line(&mut cols[1], tr("Pokémon XP/h"),
                    &if m.valid_pokemon { format_hunt_amount(m.pokemon_xp_h) } else { "—".into() });
                detail_stat_line(&mut cols[2], tr("Net gold/h"),
                    &if m.valid_gold { format_hunt_amount(m.gold_net_h) } else { "—".into() });
            });
            ui.label(RichText::new(format!("{} · {}s · {} {}",
                view.sample_quality, view.measurements.seconds.round() as u64,
                format_hunt_amount(view.measurements.kills_h),tr("kills/h")))
                .size(10.0).color(MUTED));
            ui.columns(2, |cols| {
                detail_stat_line(&mut cols[0], tr("Trainer XP earned"),
                    &format_number(view.session.trainer_xp));
                detail_stat_line(&mut cols[0], tr("Pokémon XP earned"),
                    &format_number(view.session.pokemon_xp));
                detail_stat_line(&mut cols[1], tr("Combat gold"),
                    &format_number(view.session.combat_gold));
                detail_stat_line(&mut cols[1], tr("Auto-sale gold"),
                    &format_number(view.session.auto_sale_gold));
            });
            if !view.session.balls_used.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(tr("Balls used")).size(10.0).color(DIM));
                    for (id,quantity) in &view.session.balls_used {
                        ui.horizontal(|ui| {
                            app.visuals.show(ui, SpriteKind::Ball, *id, None, 16.0);
                            ui.label(RichText::new(format!("{} ×{}",view.ball_names.get(id)
                                .map(String::as_str).unwrap_or("Ball"),quantity))
                                .size(10.0).color(TEXT));
                        });
                    }
                });
            }
            if !view.session.recent_drops.is_empty() {
                ui.collapsing(format!("{} ({})",tr("Recent drops"),
                    view.session.recent_drops.len()), |ui| {
                    for (name,qty) in view.session.recent_drops.iter().take(15) {
                        ui.label(format!("{} ×{}", name, qty));
                    }
                });
            }
        });
    ui.add_space(9.0);
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("POKÉMON DEPOT")).size(12.0)
                    .strong().color(TEXT));
                ui.label(RichText::new(format!("{} {}",view.pokemon_roster.len(),
                    tr("Pokémon"))).size(10.0).color(MUTED));
                ui.label(RichText::new(format!("{} / {}",
                    view.pokemon_roster.iter().filter(|p|p.hp==Some(0)).count(),
                    tr("fallen"))).size(10.0).color(MUTED));
            });
            if view.pokemon_roster.is_empty() {
                ui.label(RichText::new(tr("Loading your Pokémon"))
                    .size(11.0).color(DIM));
            } else {
                egui::ScrollArea::vertical().id_salt(("native-roster",index))
                    .max_height(185.0).show(ui, |ui| {
                    for pokemon in view.pokemon_roster.iter() {
                        ui.horizontal_wrapped(|ui| {
                            let active = view.active_pokemon.as_ref().is_some_and(|p|p.id==pokemon.id);
                            let name = format!("{}{} · Lv {}{}",
                                if active {"▶ "} else {""},pokemon.name,pokemon.level,
                                if pokemon.shiny {" ✨"} else {""});
                            let detail = format!(
                                "ID #{} · Species #{} · XP {} · Next {}\nTypes: {} · Power: {} · Held item #{}\nGrade: {} · Looktype: {} / {}\nIVs: {}",
                                pokemon.id,
                                pokemon.species_id.map(|id|id.to_string())
                                    .unwrap_or_else(|| "—".into()),
                                pokemon.xp.map(format_number).unwrap_or_else(|| "—".into()),
                                pokemon.xp_next.map(format_number).unwrap_or_else(|| "—".into()),
                                pokemon.types.join(", "),
                                pokemon.power.map(|n|n.to_string())
                                    .unwrap_or_else(|| "—".into()),
                                pokemon.held_item_id.map(|n|n.to_string())
                                    .unwrap_or_else(|| "—".into()),
                                pokemon.grade.map(|n|format!("{n:.2}"))
                                    .unwrap_or_else(|| "—".into()),
                                pokemon.looktype.map(|n|n.to_string())
                                    .unwrap_or_else(|| "—".into()),
                                pokemon.shiny_looktype.map(|n|n.to_string())
                                    .unwrap_or_else(|| "—".into()),
                                pokemon.ivs.iter().map(|value|value.map(|n|n.to_string())
                                    .unwrap_or_else(|| "—".into()))
                                    .collect::<Vec<_>>().join(" / "),
                            );
                            ui.label(RichText::new(name).size(11.0)
                                .color(if active { GOOD } else { TEXT }))
                                .on_hover_text(detail);
                            ui.label(RichText::new(format!(
                                "HP {} / {} · Q {} · P{}",
                                pokemon.hp.map(|n|n.to_string()).unwrap_or_else(|| "—".into()),
                                pokemon.max_hp.map(|n|n.to_string()).unwrap_or_else(|| "—".into()),
                                pokemon.quality.map(|n|format!("{n:.2}")).unwrap_or_else(|| "—".into()),
                                pokemon.potency.unwrap_or_default()
                            )).size(10.0).color(DIM));
                        });
                        ui.separator();
                    }
                });
            }
        });
    ui.add_space(9.0);
    egui::Frame::new().fill(PANEL_ALT).corner_radius(10.0)
        .inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            let item_count = view.items.values().filter(|n|**n>0).count();
            let ball_count = view.balls.values().filter(|n|**n>0).count();
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("INVENTORY")).size(12.0)
                    .strong().color(TEXT));
                ui.label(format!("{} {} · {} {}",item_count,tr("items"),
                    ball_count,tr("ball types")));
            });
            ui.horizontal_wrapped(|ui| {
                for (id,qty) in view.balls.iter().filter(|(_,qty)|**qty>0) {
                    ui.horizontal(|ui| {
                        app.visuals.show(ui, SpriteKind::Ball, *id, None, 20.0);
                        ui.label(RichText::new(format!("{} ×{}",view.ball_names.get(id)
                            .map(String::as_str).unwrap_or("Ball"),qty))
                            .size(10.0).color(TEXT));
                    });
                }
            });
            let mut items = view.items.iter().filter(|(_,qty)| **qty>0)
                .collect::<Vec<_>>();
            items.sort_by(|a,b|b.1.cmp(a.1).then_with(||a.0.cmp(b.0)));
            ui.horizontal_wrapped(|ui| {
                for (id,qty) in items.into_iter().take(10) {
                    let name = item_catalog.as_deref().map(|catalog|catalog.name(*id))
                        .unwrap_or_else(||format!("Item #{id}"));
                    let hint = item_catalog.as_deref().and_then(|catalog|
                        catalog.items.get(id).and_then(|item|item.icon.as_deref()));
                    ui.horizontal(|ui| {
                        app.visuals.show(ui, SpriteKind::Item, *id, hint, 20.0);
                        ui.label(RichText::new(format!("{name} ×{qty}"))
                            .size(10.0).color(DIM));
                    });
                }
            });
        });
    // Automation has its own sidebar page. Keep the dashboard for game data.
    ui.add_space(7.0);
    if ui.small_button(tr("Open Automation")).clicked() {
        app.workspace.automation_account = index;
        app.workspace.panel_tab = PanelTab::Automation;
    }
}

