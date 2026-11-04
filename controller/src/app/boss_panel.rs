//! Boss management UI; the game worker owns navigation and all game commands.
//! Reuse the controller's themed cards, badges, resource chips and detail rows.
use super::{
    automation_badge, compact_text, detail_stat_line, draw_native_missing_panel, egui,
    request_item_catalog, resource_chip, tr, ControllerApp, GameCommand,
    RichText, Stroke, ACCENT, BORDER, DIM, MUTED, PANEL, PANEL_ALT, TEXT, WARN,
};
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
    let (view, hunt, live) = {
        let native = app.runtimes[index].game.native.as_ref().expect("checked above");
        (
            native.boss_snapshot(),
            native.hunt_snapshot(),
            native.snapshot().phase == "Online",
        )
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut action = None;

    egui::ScrollArea::vertical()
        .id_salt(("boss-manager", index))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new(tr("Server-native Boss Manager"))
                    .size(15.0)
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new(tr("The game server repeats the chosen boss, heals and spends its entry tokens. Moth travels to the Pokémon Center and restores your original hunt when the run ends. It never uses Give Up automatically."))
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

            // Keep selection and its requirements together. Missing catalog
            // data must not hide status or persisted loot farther down.
            egui::Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(tr("Choose boss"))
                            .size(13.0)
                            .strong()
                            .color(TEXT),
                    );
                    if !view.catalog_ready {
                        ui.label(
                            RichText::new(tr("Waiting for the server's playable boss catalog."))
                                .size(10.0)
                                .color(MUTED),
                        );
                    } else if view.catalog.is_empty() {
                        ui.label(
                            RichText::new(tr("No playable boss arenas are currently advertised by the server."))
                                .size(10.0)
                                .color(MUTED),
                        );
                    } else {
                        let selected = &mut app.workspace.boss_selected[index];
                        if !view.catalog.iter().any(|entry| entry.key == *selected) {
                            *selected = view.catalog[0].key.clone();
                        }
                        let selected_name = view.catalog.iter()
                            .find(|entry| entry.key == *selected)
                            .map(|entry| entry.name.as_str())
                            .unwrap_or("—");
                        let max_chars = boss_name_limit(ui.available_width());
                        egui::ComboBox::from_id_salt(("boss-choice", index))
                            .selected_text(compact_text(selected_name, max_chars))
                            .show_ui(ui, |ui| {
                                for entry in &view.catalog {
                                    ui.selectable_value(
                                        &mut *selected,
                                        entry.key.clone(),
                                        compact_text(&entry.name, max_chars),
                                    ).on_hover_text(&entry.name);
                                }
                            })
                            .response
                            .on_hover_text(selected_name);

                        if let Some(entry) = view.catalog.iter().find(|entry| entry.key == *selected) {
                            let tokens = hunt.items.get(&entry.token_id).copied().unwrap_or(0);
                            let attempts = tokens / entry.tokens_per_entry.max(1);
                            let level = hunt.trainer_level;
                            let has_level = level.is_some_and(|current| current >= entry.min_level);
                            let has_tokens = tokens >= entry.tokens_per_entry;

                            ui.add_space(7.0);
                            ui.horizontal_wrapped(|ui| {
                                let current = level
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "—".into());
                                resource_chip(ui, &format!("{}: {}", tr("Trainer level"), current));
                                resource_chip(ui, &format!("{}: {}", tr("Required level"), entry.min_level));
                                if let Some(fight_level) = entry.fight_level(level) {
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
                            ui.label(RichText::new(tr(
                                "Bosses now fight at your trainer level (minimum: their unlock level). A higher trainer level increases boss stats and battle duration; entry tokens and drop tiers still follow the original boss."
                            )).size(10.0).color(MUTED));

                            let ready = live
                                && view.state_known
                                && !view.busy
                                && !view.auto
                                && view.arena.is_none()
                                && has_level
                                && has_tokens;
                            ui.add_space(9.0);
                            let compact_actions = ui.available_width() < 340.0;
                            ui.horizontal_wrapped(|ui| {
                                let farm_label = if compact_actions {
                                    tr("Auto farm")
                                } else {
                                    tr("Farm until tokens run out")
                                };
                                if ui.add_enabled(
                                    ready,
                                    egui::Button::new(
                                        RichText::new(farm_label).strong(),
                                    )
                                    .fill(if ready {
                                        ACCENT.linear_multiply(0.32)
                                    } else {
                                        PANEL_ALT
                                    })
                                    .stroke(Stroke::new(1.0, if ready {
                                        ACCENT.linear_multiply(0.65)
                                    } else {
                                        BORDER
                                    }))
                                    .corner_radius(8.0),
                                ).on_hover_text(tr("Farm until tokens run out")).clicked() {
                                    action = Some(GameCommand::BossStart {
                                        key: entry.key.clone(),
                                        repeat: true,
                                    });
                                }
                                if ui.add_enabled(
                                    ready,
                                    egui::Button::new(tr("Fight once")),
                                ).clicked() {
                                    action = Some(GameCommand::BossStart {
                                        key: entry.key.clone(),
                                        repeat: false,
                                    });
                                }
                            });
                            if !ready {
                                let reason = if !live {
                                    tr("Game disconnected. Boss controls will become available after reconnection.").to_owned()
                                } else if !view.state_known {
                                    tr("Waiting for authoritative server boss state.").to_owned()
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
                        }
                    }
                });

            ui.add_space(10.0);
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
                            action = Some(GameCommand::BossStop);
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
                                        action = Some(GameCommand::BossLeave);
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

            ui.add_space(10.0);
            egui::Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(tr("Boss loot history"))
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

            if let Some(command) = action.take() {
                let notice = match &command {
                    GameCommand::BossStart { repeat: true, .. } => tr("Boss farming queued"),
                    GameCommand::BossStart { repeat: false, .. } => tr("Boss fight queued"),
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
