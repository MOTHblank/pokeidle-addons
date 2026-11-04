//! Boss management UI; the game worker owns navigation and all game commands.
//! Reuse the controller's themed cards, badges, resource chips and detail rows.
use super::{
    automation_badge, compact_text, detail_stat_line, draw_native_missing_panel, egui,
    native_atlas, request_item_catalog, resource_chip, tr, AtlasReference,
    ControllerApp, GameCommand, RichText, Stroke, ACCENT, BORDER, DIM, MUTED,
    PANEL, PANEL_ALT, TEXT, WARN,
};
use super::hunt_panel::components::{
    atlas_type_multiplier, atlas_type_name, atlas_types_for_active, draw_atlas_matchup,
};
use crate::native_boss::{strongest_eligible_team_level, BossEntry, BossView};
use crate::native_hunts::HuntView;
use std::collections::BTreeMap;

const BOSS_HISTORY_PAGE_SIZE: usize = 15;

// Favor hover-revealed full names over horizontal overflow on small windows.
fn boss_name_limit(width: f32) -> usize {
    if width < 240.0 { 16 }
    else if width < 340.0 { 24 }
    else { 42 }
}

fn history_range(page: &mut usize, count: usize) -> std::ops::Range<usize> {
    let page_count = count.div_ceil(BOSS_HISTORY_PAGE_SIZE).max(1);
    *page = (*page).min(page_count - 1);
    let start = *page * BOSS_HISTORY_PAGE_SIZE;
    start..(start + BOSS_HISTORY_PAGE_SIZE).min(count)
}

fn leave_confirmation_valid(live: bool, arena: Option<&str>, target: Option<&str>) -> bool {
    live && matches!((arena, target), (Some(arena), Some(target)) if arena == target)
}
use std::time::{SystemTime, UNIX_EPOCH};

fn loot_age(now: u64, at: u64) -> String {
    let elapsed = now.saturating_sub(at);
    if elapsed < 3_600 {
        format!("{}m", elapsed / 60)
    } else if elapsed < 86_400 {
        format!("{}h", elapsed / 3_600)
    } else {
        format!("{}d", elapsed / 86_400)
    }
}


const TYPE_ORDER: [&str; 18] = [
    "NORMAL", "FIRE", "WATER", "ELECTRIC", "GRASS", "ICE", "FIGHTING",
    "POISON", "GROUND", "FLYING", "PSYCHIC", "BUG", "ROCK", "GHOST",
    "DRAGON", "DARK", "STEEL", "FAIRY",
];

pub(super) fn ensure_boss_combat_reference(app: &mut ControllerApp) {
    if app.hunt_reference.is_none() && app.hunt_reference_error.is_none() &&
        !app.hunt_reference_busy {
        app.hunt_reference_busy = true;
        let tx = app.hunt_reference_tx.clone();
        std::thread::Builder::new()
            .name("moth-boss-reference-load".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(native_atlas::load)
                    .unwrap_or_else(|_| Err(tr("Pokémon data unavailable").into()));
                let _ = tx.send(result);
            })
            .expect("could not start Boss Farm reference loader");
    }
}

fn boss_type_label(entry: &BossEntry) -> String {
    if entry.types.is_empty() {
        return tr("Type data unavailable").to_owned();
    }
    entry.types.iter().map(|kind| atlas_type_name(kind))
        .collect::<Vec<_>>().join(" · ")
}

fn draw_boss_selector(
    ui: &mut egui::Ui,
    entry: &BossEntry,
    selected: bool,
    hunt: &HuntView,
) -> bool {
    let tokens = hunt.items.get(&entry.token_id).copied().unwrap_or(0);
    let attempts = tokens / entry.tokens_per_entry.max(1);
    let fight_level = entry.fight_level(hunt)
        .map(|level| level.to_string()).unwrap_or_else(|| "—".into());
    let card = egui::Frame::new()
        .fill(if selected { ACCENT.linear_multiply(0.13) } else { PANEL })
        .stroke(Stroke::new(1.0, if selected { ACCENT } else { BORDER }))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(9, 7))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(entry.name.chars().take(2)
                        .collect::<String>().to_uppercase())
                        .size(11.0).strong().color(ACCENT),
                );
                ui.vertical(|ui| {
                    ui.label(RichText::new(&entry.name).strong().size(12.0).color(TEXT));
                    ui.label(
                        RichText::new(boss_type_label(entry))
                            .size(9.0)
                            .color(MUTED),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} {} · {} {} × {}",
                            tr("Boss combat level"), fight_level,
                            tr("Entry"), entry.tokens_per_entry, entry.token_name,
                        ))
                        .size(9.0)
                        .color(MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(attempts.to_string()).size(12.0).strong().color(TEXT));
                        ui.label(RichText::new(tr("Possible entries")).size(9.0).color(MUTED));
                    });
                });
            });
        });
    let clicked = ui.interact(
        card.response.rect,
        ui.id().with(("boss-selector", &entry.key)),
        egui::Sense::click(),
    ).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
    ui.add_space(5.0);
    clicked
}

fn draw_boss_weaknesses(
    ui: &mut egui::Ui,
    reference: Option<&AtlasReference>,
    types: &[String],
) {
    if types.is_empty() { return; }
    let Some(reference) = reference else {
        ui.label(RichText::new(tr("Loading Pokémon data...")).size(10.0).color(MUTED));
        return;
    };
    let weaknesses = TYPE_ORDER.iter().filter_map(|kind| {
        let multiplier = atlas_type_multiplier(reference, kind, types)?;
        (multiplier > 1.0 + f64::EPSILON).then_some((*kind, multiplier))
    }).collect::<Vec<_>>();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tr("Weak to")).size(10.0).color(MUTED));
        if weaknesses.is_empty() {
            ui.label(RichText::new(tr("No type weakness")).size(10.0).color(DIM));
        } else {
            for (kind, multiplier) in weaknesses {
                resource_chip(ui, &format!(
                    "{} {:.0}×", atlas_type_name(kind), multiplier,
                ));
            }
        }
    });
}

fn draw_boss_details(
    ui: &mut egui::Ui,
    entry: &BossEntry,
    view: &BossView,
    hunt: &HuntView,
    live: bool,
    reference: Option<&AtlasReference>,
    restock_enabled: bool,
    pending_start: Option<(&str, bool)>,
    action: &mut Option<GameCommand>,
) {
    let tokens = hunt.items.get(&entry.token_id).copied().unwrap_or(0);
    let attempts = tokens / entry.tokens_per_entry.max(1);
    let level = hunt.trainer_level;
    let team_level = strongest_eligible_team_level(hunt);
    let has_level = level.is_some_and(|current| current >= entry.min_level);
    let has_tokens = tokens >= entry.tokens_per_entry;

    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(9.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(entry.name.chars().take(2)
                        .collect::<String>().to_uppercase())
                        .size(16.0).strong().color(ACCENT),
                );
                ui.vertical(|ui| {
                    ui.label(RichText::new(&entry.name).size(14.0).strong().color(TEXT));
                    ui.horizontal_wrapped(|ui| {
                        for kind in &entry.types {
                            ui.label(RichText::new(atlas_type_name(kind)).size(10.0).color(ACCENT));
                        }
                        if entry.types.is_empty() {
                            ui.label(RichText::new(tr("Type data unavailable")).size(10.0).color(DIM));
                        }
                    });
                });
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                let current = level.map(|value| value.to_string()).unwrap_or_else(|| "—".into());
                resource_chip(ui, &format!("{}: {}", tr("Trainer level"), current));
                resource_chip(ui, &format!("{}: {}", tr("Required level"), entry.min_level));
                let strongest = team_level.filter(|value| *value > 0)
                    .map(|value| value.to_string()).unwrap_or_else(|| "—".into());
                resource_chip(ui, &format!("{}: {}", tr("Strongest eligible team Pokémon"), strongest));
                if let Some(fight_level) = entry.fight_level(hunt) {
                    resource_chip(ui, &format!("{}: {}", tr("Boss combat level"), fight_level));
                }
                resource_chip(ui, &format!("{}: {}", tr("Possible entries"), attempts));
            });
            ui.add_space(5.0);
            detail_stat_line(
                ui, tr("Entry"),
                &format!("{} × {}", entry.tokens_per_entry, entry.token_name),
            );
            detail_stat_line(
                ui, tr("Inventory"),
                &format!("{} × {}", tokens, entry.token_name),
            );
            draw_boss_weaknesses(ui, reference, &entry.types);

            ui.add_space(8.0);
            let lead_types = reference.map(|guide| atlas_types_for_active(hunt, guide))
                .unwrap_or_default();
            draw_atlas_matchup(
                ui,
                reference,
                &lead_types,
                &entry.types,
                hunt.active_pokemon.as_ref().map(|pokemon| pokemon.name.as_str())
                    .unwrap_or(tr("Your Pokémon")),
                &entry.name,
                tr("Against your active Pokémon"),
            );

            ui.add_space(7.0);
            ui.label(RichText::new(tr(
                "Bosses fight at the level of your strongest eligible team Pokémon, never below their own unlock level. Trainer level still controls which bosses are unlocked; entry tokens and drop tiers are unchanged."
            )).size(10.0).color(MUTED));
            ui.label(
                RichText::new(if restock_enabled {
                    tr("Restock stays active during boss battles.")
                } else {
                    tr("Restock is off for this account.")
                })
                .size(10.0)
                .color(if restock_enabled { ACCENT } else { MUTED }),
            );

            let ready = live
                && view.state_known
                && !view.busy
                && !view.auto
                && view.arena.is_none()
                && has_level
                && has_tokens;
            ui.add_space(9.0);
            let compact_actions = ui.available_width() < 340.0;
            let pending_mode = pending_start
                .filter(|(key, _)| *key == entry.key.as_str())
                .map(|(_, repeat)| repeat);
            ui.horizontal_wrapped(|ui| {
                let farm_label = if compact_actions {
                    tr("Auto farm")
                } else {
                    tr("Farm until tokens run out")
                };
                if pending_mode == Some(true) {
                    ui.add_enabled(false,
                        egui::Button::new(RichText::new(tr("Starting farm…")).strong())
                            .fill(WARN.linear_multiply(0.14))
                            .stroke(Stroke::new(1.0, WARN.linear_multiply(0.7)))
                            .corner_radius(8.0));
                    ui.label(RichText::new(tr(if hunt.in_center {
                        "Entering boss from Pokémon Center…"
                    } else {
                        "Waiting for a safe opening to reach Pokémon Center…"
                    })).size(10.0).color(WARN));
                } else if ui.add_enabled(
                    ready && pending_mode.is_none(),
                    egui::Button::new(RichText::new(farm_label).strong())
                        .fill(if ready && pending_mode.is_none() {
                            ACCENT.linear_multiply(0.32)
                        } else { PANEL_ALT })
                        .stroke(Stroke::new(
                            1.0,
                            if ready && pending_mode.is_none() {
                                ACCENT.linear_multiply(0.65)
                            } else { BORDER },
                        ))
                        .corner_radius(8.0),
                ).on_hover_text(tr("Farm until tokens run out")).clicked() {
                    *action = Some(GameCommand::BossStart {
                        key: entry.key.clone(),
                        repeat: true,
                    });
                }
                if pending_mode == Some(false) {
                    ui.add_enabled(false,
                        egui::Button::new(tr("Starting fight…"))
                            .fill(WARN.linear_multiply(0.14))
                            .stroke(Stroke::new(1.0, WARN.linear_multiply(0.7))));
                    ui.label(RichText::new(tr(if hunt.in_center {
                        "Entering boss from Pokémon Center…"
                    } else {
                        "Waiting for a safe opening to reach Pokémon Center…"
                    })).size(10.0).color(WARN));
                } else if ui.add_enabled(
                    ready && pending_mode.is_none(),
                    egui::Button::new(tr("Fight once")),
                ).clicked() {
                    *action = Some(GameCommand::BossStart {
                        key: entry.key.clone(),
                        repeat: false,
                    });
                }
            });
            if !ready && pending_mode.is_none() {
                let reason = if !live {
                    tr("Game disconnected. Boss controls will become available after reconnection.").to_owned()
                } else if !view.state_known {
                    tr("Waiting for authoritative server boss state.").to_owned()
                } else if view.busy && !view.auto && view.arena.is_none() {
                    tr("Returning to Pokémon Center before boss entry").to_owned()
                } else if view.busy || view.auto || view.arena.is_some() {
                    tr("Finish the current boss run before starting another.").to_owned()
                } else if level.is_none() {
                    tr("Trainer level is not available yet.").to_owned()
                } else if !has_level {
                    tr("Trainer level is below the boss requirement.").to_owned()
                } else {
                    tr("Not enough boss entry tokens.").to_owned()
                };
                ui.label(RichText::new(reason).size(10.0).color(MUTED));
            }
        });
}


fn draw_boss_status_card(
    app: &mut ControllerApp,
    ui: &mut egui::Ui,
    index: usize,
    view: &BossView,
    hunt: &HuntView,
    live: bool,
    action: &mut Option<GameCommand>,
) {
    egui::Frame::new()
        .fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(tr("Boss status"))
                        .size(13.0)
                        .strong()
                        .color(TEXT),
                );
                let phase = if !live {
                    tr("Offline")
                } else if !view.state_known {
                    tr("Waiting for authoritative server boss state.")
                } else if view.phase.is_empty() {
                    tr("Idle")
                } else {
                    tr(&view.phase)
                };
                automation_badge(ui, "Phase", phase, live && (view.auto || view.arena.is_some()));
            });
            if let Some(arena) = view.arena.as_deref() {
                let name = view.catalog.iter()
                    .find(|entry| entry.key == arena)
                    .map(|entry| entry.name.as_str())
                    .unwrap_or(arena);
                detail_stat_line(ui, tr("Current arena"), name);
            }
            if let Some(original) = view.original_hunt.as_deref() {
                detail_stat_line(ui, tr("Return destination"), original);
            }
            if view.auto {
                let boss_name = view.auto_key.as_deref().and_then(|key| {
                    view.catalog.iter().find(|entry| entry.key == key)
                        .map(|entry| entry.name.as_str())
                }).or(view.auto_key.as_deref()).unwrap_or("—");
                detail_stat_line(ui, tr("Server repeating"), boss_name);
            }
            if let Some(warning) = view.warning.as_deref() {
                ui.label(RichText::new(tr(warning)).size(10.0).color(WARN));
            }
            if let Some(warning) = hunt.navigation_warning.as_deref() {
                ui.label(RichText::new(tr(warning)).size(10.0).color(WARN));
            }
    
            // Never let a confirmation from a previous arena or
            // disconnected session authorize leaving a new encounter.
            if app.workspace.boss_leave_confirm[index]
                && !leave_confirmation_valid(
                    live, view.arena.as_deref(),
                    app.workspace.boss_leave_target[index].as_deref(),
                )
            {
                app.workspace.boss_leave_confirm[index] = false;
                app.workspace.boss_leave_target[index] = None;
            }
    
            ui.add_space(8.0);
            let compact_actions = ui.available_width() < 340.0;
            let can_leave = live && view.arena.is_some();
            ui.horizontal_wrapped(|ui| {
                let stop_label = if compact_actions {
                    tr("Stop & return")
                } else {
                    tr("Stop repetition and return")
                };
                if ui.add_enabled(
                    live && view.can_stop,
                    egui::Button::new(stop_label),
                ).on_hover_text(tr("Stop repetition and return")).clicked() {
                    *action = Some(GameCommand::BossStop);
                    app.workspace.boss_leave_confirm[index] = false;
                    app.workspace.boss_leave_target[index] = None;
                }
                let leave_label = if compact_actions {
                    tr("Leave arena")
                } else {
                    tr("Leave boss encounter")
                };
                if ui.add_enabled(
                    can_leave,
                    egui::Button::new(
                        RichText::new(leave_label).color(if can_leave { WARN } else { DIM }),
                    )
                    .stroke(Stroke::new(1.0, if can_leave {
                        WARN.linear_multiply(0.5)
                    } else {
                        BORDER
                    })),
                ).on_hover_text(tr("Leaving an active boss encounter may forfeit its entry. Confirm?")).clicked() {
                    app.workspace.boss_leave_confirm[index] = true;
                    app.workspace.boss_leave_target[index] = view.arena.clone();
                }
            });
            if app.workspace.boss_leave_confirm[index] {
                ui.add_space(7.0);
                egui::Frame::new()
                    .fill(WARN.linear_multiply(0.08))
                    .stroke(Stroke::new(1.0, WARN.linear_multiply(0.5)))
                    .corner_radius(7.0)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(tr("Leaving an active boss encounter may forfeit its entry. Confirm?"))
                                .size(11.0)
                                .color(WARN),
                        );
                        ui.horizontal_wrapped(|ui| {
                            if ui.add_enabled(
                                leave_confirmation_valid(
                                    live, view.arena.as_deref(),
                                    app.workspace.boss_leave_target[index].as_deref(),
                                ),
                                egui::Button::new(tr("Confirm leave")),
                            ).clicked() {
                                *action = Some(GameCommand::BossLeave);
                                app.workspace.boss_leave_confirm[index] = false;
                                app.workspace.boss_leave_target[index] = None;
                            }
                            if ui.button(tr("Cancel")).clicked() {
                                app.workspace.boss_leave_confirm[index] = false;
                                app.workspace.boss_leave_target[index] = None;
                            }
                        });
                    });
            }
        });
}

fn draw_boss_history_card(
    app: &mut ControllerApp,
    ui: &mut egui::Ui,
    index: usize,
    view: &BossView,
    now: u64,
) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.label(
                RichText::new(tr("Boss victories & loot"))
                    .size(13.0)
                    .strong()
                    .color(TEXT),
            );
            ui.horizontal_wrapped(|ui| {
                resource_chip(
                    ui,
                    &format!("{}: {}", tr("Victories"), view.log.len()),
                );
            });
            if let Some(error) = view.storage_warning.as_deref() {
                ui.label(RichText::new(tr(error)).size(10.0).color(WARN));
            }
            if let Ok(status) = view.write_status.lock() {
                if let Some(error) = status.warning.as_deref() {
                    ui.label(
                        RichText::new(format!(
                            "{}: {}",
                            tr("Boss loot could not be saved; retrying"),
                            error,
                        ))
                        .size(10.0)
                        .color(WARN),
                    );
                }
            }
    
            let mut totals = BTreeMap::<u64, u64>::new();
            for battle in view.log.iter() {
                for drop in &battle.drops {
                    let total = totals.entry(drop.item_id).or_default();
                    *total = total.saturating_add(drop.quantity);
                }
            }
            if totals.is_empty() {
                ui.label(
                    RichText::new(tr("No confirmed boss item drops recorded yet."))
                        .size(10.0)
                        .color(DIM),
                );
            } else {
                ui.add_space(7.0);
                // Keep the history controls reachable even after
                // hundreds of distinct recorded item drops.
                let totals_label = if ui.available_width() < 320.0 {
                    tr("Loot totals")
                } else {
                    tr("Items acquired (retained history)")
                };
                egui::CollapsingHeader::new(format!(
                    "{} ({})", totals_label, totals.len(),
                ))
                .id_salt(("boss-loot-totals", index))
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (id, quantity) in &totals {
                            let name = app.inventory_catalog.as_ref()
                                .map(|catalog| catalog.name(*id))
                                .unwrap_or_else(|| format!("Item #{id}"));
                            resource_chip(ui, &format!("×{quantity} {name}"));
                        }
                    });
                });
            }
    
            if !view.log.is_empty() {
                ui.add_space(10.0);
                ui.separator();
    
                // A per-account filter and bounded page keep the
                // retained 500-entry journal navigable without
                // creating hundreds of widgets on every repaint.
                let mut boss_names = BTreeMap::<String, String>::new();
                for result in view.log.iter().rev() {
                    if !result.key.is_empty() {
                        boss_names.entry(result.key.clone())
                            .or_insert_with(|| result.boss.clone());
                    }
                }
                let filter = &mut app.workspace.boss_history_filter[index];
                if !filter.is_empty() && !boss_names.contains_key(filter.as_str()) {
                    filter.clear();
                }
                let previous_filter = filter.clone();
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(tr("Filter history"))
                            .size(10.0)
                            .color(MUTED),
                    );
                    let selected_name = boss_names
                        .get(filter.as_str()).map(String::as_str)
                        .unwrap_or(tr("All bosses"));
                    let max_chars = boss_name_limit(ui.available_width());
                    egui::ComboBox::from_id_salt(("boss-history-filter", index))
                        .selected_text(compact_text(selected_name, max_chars))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut *filter,
                                String::new(),
                                tr("All bosses"),
                            );
                            for (key, name) in &boss_names {
                                ui.selectable_value(
                                    &mut *filter,
                                    key.clone(),
                                    compact_text(name, max_chars),
                                ).on_hover_text(name);
                            }
                        })
                        .response
                        .on_hover_text(selected_name);
                });
                if *filter != previous_filter {
                    app.workspace.boss_history_page[index] = 0;
                }
                let filtered: Vec<_> = view.log.iter().rev()
                    .filter(|result| filter.is_empty() || result.key == *filter)
                    .collect();
                let count = filtered.len();
                let page = &mut app.workspace.boss_history_page[index];
                let page_count = count.div_ceil(BOSS_HISTORY_PAGE_SIZE).max(1);
                *page = (*page).min(page_count - 1);
                if page_count > 1 {
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(
                            *page > 0,
                            egui::Button::new(tr("← Previous")),
                        ).clicked() {
                            *page -= 1;
                        }
                        if ui.add_enabled(
                            *page + 1 < page_count,
                            egui::Button::new(tr("Next →")),
                        ).clicked() {
                            *page += 1;
                        }
                    });
                }
                // Compute the range after clicks: the count and rows
                // now describe the same page in this exact frame.
                let range = history_range(page, count);
                let first = if count == 0 { 0 } else { range.start + 1 };
                ui.label(
                    RichText::new(format!(
                        "{first}–{} / {count} · {} / {page_count}",
                        range.end, *page + 1,
                    ))
                        .size(10.0)
                        .color(DIM),
                );
                if filtered.is_empty() {
                    ui.label(
                        RichText::new(tr("No victories match this boss."))
                            .size(10.0)
                            .color(DIM),
                    );
                }
                // Both navigation and row selection use this
                // frame's final, clamped page index.
                for result in &filtered[range] {
                    let drops = if result.drops.is_empty() {
                        tr("No items").to_owned()
                    } else {
                        result.drops.iter().map(|drop| {
                            let name = app.inventory_catalog.as_ref()
                                .map(|catalog| catalog.name(drop.item_id))
                                .unwrap_or_else(|| format!("Item #{}", drop.item_id));
                            format!("{} ×{}", name, drop.quantity)
                        }).collect::<Vec<_>>().join(" · ")
                    };
                    let name_limit = boss_name_limit(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(compact_text(&result.boss, name_limit))
                                .size(11.0)
                                .strong()
                                .color(TEXT),
                        ).on_hover_text(&result.boss);
                        ui.label(
                            RichText::new(loot_age(now, result.at))
                                .size(10.0)
                                .color(DIM),
                        );
                    });
                    ui.add(
                        egui::Label::new(RichText::new(drops).size(10.0).color(MUTED))
                            .wrap(),
                    );
                    ui.add_space(6.0);
                    ui.separator();
                }
            }
            if view.log.len() >= 500 {
                ui.label(
                    RichText::new(tr("Only the latest 500 victories are retained per account."))
                        .size(10.0)
                        .color(DIM),
                );
            }
        });
}

pub(super) fn draw_boss_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let index = app.workspace.automation_account;
    if app.runtimes[index].game.native.is_none() {
        draw_native_missing_panel(
            app, ui, Some(index),
            tr("Start this account to manage server-native boss farming"),
        );
        return;
    }

    request_item_catalog(app);
    ensure_boss_combat_reference(app);
    let (view, hunt, live, restock_enabled) = {
        let native = app.runtimes[index].game.native.as_ref().expect("checked above");
        let snapshot = native.snapshot();
        (
            native.boss_snapshot(),
            native.hunt_snapshot(),
            snapshot.phase == "Online",
            snapshot.native_restock.enabled,
        )
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Same immediate-feedback bridge as Hunt Atlas: acknowledge the click in
    // the UI now, then let the worker's authoritative BossView take over.
    if let (Some(key), Some(repeat)) =
        (view.request_key.as_deref(), view.request_repeat) {
        let same = app.workspace.boss_start_pending[index].as_ref()
            .is_some_and(|(pending, mode, _)| pending == key && *mode == repeat);
        if !same {
            app.workspace.boss_start_pending[index] =
                Some((key.to_owned(), repeat, Instant::now()));
        }
    } else if app.workspace.boss_start_pending[index].as_ref().is_some_and(
        |(key, repeat, at)| {
            !live ||
            view.arena.as_deref() == Some(key.as_str()) ||
            (*repeat && view.auto && view.auto_key.as_deref() == Some(key.as_str())) ||
            (!view.busy && at.elapsed() > Duration::from_millis(500)) ||
            at.elapsed() > Duration::from_secs(5)
        },
    ) {
        app.workspace.boss_start_pending[index] = None;
    }
    if app.workspace.boss_stop_pending[index].is_some_and(|at|
        !live || (!view.phase.starts_with("Stopping") &&
            at.elapsed() > Duration::from_millis(500))) {
        app.workspace.boss_stop_pending[index] = None;
    }
    if app.workspace.boss_leave_pending[index].is_some_and(|at|
        !live || (!view.phase.starts_with("Leaving") &&
            at.elapsed() > Duration::from_millis(500))) {
        app.workspace.boss_leave_pending[index] = None;
    }
    let pending_start = app.workspace.boss_start_pending[index].as_ref()
        .map(|(key, repeat, _)| (key.clone(), *repeat));
    let mut action = None;

    egui::ScrollArea::vertical()
        .id_salt(("boss-farm", index))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new(tr("Server-native Boss Farm"))
                    .size(15.0)
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new(tr("Boss entry uses Hunt Atlas navigation to return to the Pokémon Center first, then starts the selected boss. If travel stays blocked for one minute, the pending request authorizes one Give Up. Moth restores your original hunt when the run ends."))
                    .size(10.0)
                    .color(MUTED),
            );
            if !live {
                ui.label(
                    RichText::new(tr("Game disconnected. Boss controls will become available after reconnection."))
                        .size(10.0)
                        .color(WARN),
                );
            }
            ui.add_space(10.0);
            // Keep the two panels beside each other even in a narrow window:
            // pan horizontally instead of crushing buttons and history rows.
            egui::ScrollArea::horizontal()
                .id_salt(("boss-summary-horizontal", index))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width().max(700.0));
                    ui.columns(2, |cols| {
                        egui::ScrollArea::vertical()
                            .id_salt(("boss-status-top", index))
                            .max_height(300.0)
                            .auto_shrink([false, false])
                            .show(&mut cols[0], |ui| {
                                draw_boss_status_card(app, ui, index, &view, &hunt, live, &mut action);
                            });
                        egui::ScrollArea::vertical()
                            .id_salt(("boss-history-top", index))
                            .max_height(300.0)
                            .auto_shrink([false, false])
                            .show(&mut cols[1], |ui| {
                                draw_boss_history_card(app, ui, index, &view, now);
                            });
                    });
                });
            ui.add_space(10.0);

            // Hunt Atlas-style master/detail selection: boss cards on the
            // left, selected combat and entry information on the right.
            if !view.catalog_ready {
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(tr("Waiting for the server's playable boss catalog."))
                                .size(10.0).color(MUTED),
                        );
                    });
            } else if view.catalog.is_empty() {
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(tr("No playable boss arenas are currently advertised by the server."))
                                .size(10.0).color(MUTED),
                        );
                    });
            } else {
                let reference = app.hunt_reference.clone();
                let selected = &mut app.workspace.boss_selected[index];
                if !view.catalog.iter().any(|entry| entry.key == *selected) {
                    *selected = view.catalog[0].key.clone();
                }
                let desktop = ui.available_width() >= 880.0;
                if desktop {
                    let gap = ui.spacing().item_spacing.x;
                    let total_width = ui.available_width();
                    let list_width = (total_width * 0.36).clamp(290.0, 430.0);
                    let details_width = (total_width - list_width - gap).max(0.0);
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(list_width);
                            ui.label(
                                RichText::new(tr("BOSSES"))
                                    .size(10.0).strong().color(DIM),
                            );
                            ui.add_space(5.0);
                            for entry in &view.catalog {
                                if draw_boss_selector(
                                    ui, entry, *selected == entry.key, &hunt,
                                ) {
                                    *selected = entry.key.clone();
                                }
                            }
                        });
                        ui.vertical(|ui| {
                            ui.set_width(details_width);
                            ui.label(
                                RichText::new(tr("BOSS DETAILS"))
                                    .size(10.0).strong().color(DIM),
                            );
                            ui.add_space(5.0);
                            if let Some(entry) = view.catalog.iter()
                                .find(|entry| entry.key == *selected) {
                                draw_boss_details(
                                    ui, entry, &view, &hunt, live,
                                    reference.as_deref(), restock_enabled,
                                    pending_start.as_ref().map(|(key, repeat)| (key.as_str(), *repeat)),
                                    &mut action,
                                );
                            }
                        });
                    });
                } else {
                    ui.label(
                        RichText::new(tr("BOSSES"))
                            .size(10.0).strong().color(DIM),
                    );
                    ui.add_space(5.0);
                    for entry in &view.catalog {
                        if draw_boss_selector(ui, entry, *selected == entry.key, &hunt) {
                            *selected = entry.key.clone();
                        }
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(tr("BOSS DETAILS"))
                            .size(10.0).strong().color(DIM),
                    );
                    ui.add_space(5.0);
                    if let Some(entry) = view.catalog.iter()
                        .find(|entry| entry.key == *selected) {
                        draw_boss_details(
                            ui, entry, &view, &hunt, live,
                            reference.as_deref(), restock_enabled,
                                    pending_start.as_ref().map(|(key, repeat)| (key.as_str(), *repeat)),
                                    &mut action,
                        );
                    }
                }
            }

            if let Some(command) = action.take() {
                let notice = match &command {
                    GameCommand::BossStart { repeat: true, .. } => tr("Boss farming requested"),
                    GameCommand::BossStart { repeat: false, .. } => tr("Boss fight requested"),
                    GameCommand::BossStop => tr("Boss stop queued"),
                    GameCommand::BossLeave => tr("Boss leave queued"),
                    _ => tr("Boss command queued"),
                };
                let result = app.runtimes[index].game.native.as_ref()
                    .expect("native still present")
                    .send_command(command);
                match result {
                    Ok(()) => app.set_status(notice, false),
                    Err(error) => app.set_status(error, true),
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::{boss_name_limit, history_range, leave_confirmation_valid, loot_age, BOSS_HISTORY_PAGE_SIZE};

    #[test]
    fn boss_names_use_compact_labels_in_narrow_workspaces() {
        assert_eq!(boss_name_limit(180.0), 16);
        assert_eq!(boss_name_limit(280.0), 24);
        assert_eq!(boss_name_limit(480.0), 42);
    }

    #[test]
    fn history_pages_are_bounded_and_clamped() {
        let mut page = 0;
        assert_eq!(history_range(&mut page, 0), 0..0);
        assert_eq!(history_range(&mut page, BOSS_HISTORY_PAGE_SIZE + 2), 0..15);
        page = 1;
        assert_eq!(history_range(&mut page, BOSS_HISTORY_PAGE_SIZE + 2), 15..17);
        page = 99;
        assert_eq!(history_range(&mut page, 5), 0..5);
        assert_eq!(page, 0);
    }

    #[test]
    fn leave_confirmation_requires_the_same_live_arena() {
        assert!(leave_confirmation_valid(true, Some("arena-a"), Some("arena-a")));
        assert!(!leave_confirmation_valid(true, Some("arena-b"), Some("arena-a")));
        assert!(!leave_confirmation_valid(false, Some("arena-a"), Some("arena-a")));
        assert!(!leave_confirmation_valid(true, None, Some("arena-a")));
        assert!(!leave_confirmation_valid(true, Some("arena-a"), None));
    }

    #[test]
    fn history_relative_age_never_underflows() {
        assert_eq!(loot_age(100, 200), "0m");
        assert_eq!(loot_age(3600, 0), "1h");
        assert_eq!(loot_age(86400, 0), "1d");
    }
}
