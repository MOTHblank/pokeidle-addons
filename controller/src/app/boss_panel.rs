//! Boss management UI; the game worker owns navigation and all game commands.
//! Reuse the controller's themed cards, badges, resource chips and detail rows.
use super::{
    automation_badge, detail_stat_line, draw_native_missing_panel, egui,
    request_item_catalog, resource_chip, tr, ControllerApp, GameCommand,
    RichText, Stroke, ACCENT, BORDER, DIM, MUTED, PANEL, PANEL_ALT, TEXT, WARN,
};
use std::collections::BTreeMap;
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
                        egui::ComboBox::from_id_salt(("boss-choice", index))
                            .selected_text(
                                view.catalog
                                    .iter()
                                    .find(|entry| entry.key == *selected)
                                    .map(|entry| entry.name.as_str())
                                    .unwrap_or("—"),
                            )
                            .show_ui(ui, |ui| {
                                for entry in &view.catalog {
                                    ui.selectable_value(
                                        selected,
                                        entry.key.clone(),
                                        &entry.name,
                                    );
                                }
                            });

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

                            let ready = live
                                && view.state_known
                                && !view.busy
                                && !view.auto
                                && view.arena.is_none()
                                && has_level
                                && has_tokens;
                            ui.add_space(9.0);
                            ui.horizontal_wrapped(|ui| {
                                if ui.add_enabled(
                                    ready,
                                    egui::Button::new(
                                        RichText::new(tr("Farm until tokens run out"))
                                            .strong()
                                            .color(TEXT),
                                    )
                                    .fill(ACCENT.linear_multiply(0.32))
                                    .stroke(Stroke::new(1.0, ACCENT.linear_multiply(0.65)))
                                    .corner_radius(8.0),
                                ).clicked() {
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
                        automation_badge(ui, "Boss status", phase, live && (view.auto || view.arena.is_some()));
                    });
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

                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(
                            live && view.can_stop,
                            egui::Button::new(tr("Stop repetition and return")),
                        ).clicked() {
                            action = Some(GameCommand::BossStop);
                        }
                        if ui.add_enabled(
                            live && view.arena.is_some(),
                            egui::Button::new(
                                RichText::new(tr("Leave boss encounter")).color(WARN),
                            )
                            .stroke(Stroke::new(1.0, WARN.linear_multiply(0.5))),
                        ).clicked() {
                            app.workspace.boss_leave_confirm[index] = true;
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
                                        live && view.arena.is_some(),
                                        egui::Button::new(tr("Confirm leave")),
                                    ).clicked() {
                                        action = Some(GameCommand::BossLeave);
                                        app.workspace.boss_leave_confirm[index] = false;
                                    }
                                    if ui.button(tr("Cancel")).clicked() {
                                        app.workspace.boss_leave_confirm[index] = false;
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
                        ui.label(
                            RichText::new(tr("Items acquired (retained history)"))
                                .size(10.0)
                                .strong()
                                .color(MUTED),
                        );
                        ui.horizontal_wrapped(|ui| {
                            for (id, quantity) in totals {
                                let name = app.inventory_catalog.as_ref()
                                    .map(|catalog| catalog.name(id))
                                    .unwrap_or_else(|| format!("Item #{id}"));
                                resource_chip(ui, &format!("{name} ×{quantity}"));
                            }
                        });
                    }

                    if !view.log.is_empty() {
                        ui.add_space(10.0);
                        ui.separator();
                        for result in view.log.iter().rev().take(60) {
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
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(&result.boss)
                                        .size(11.0)
                                        .strong()
                                        .color(TEXT),
                                );
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
                    if view.log.len() > 60 {
                        ui.label(
                            RichText::new(tr("Showing the latest 60 victories (up to 500 are retained per account)."))
                                .size(10.0)
                                .color(DIM),
                        );
                    }
                });

            if let Some(command) = action.take() {
                let result = app.runtimes[index].game.native.as_ref()
                    .expect("native still present")
                    .send_command(command);
                match result {
                    Ok(()) => app.set_status(tr("Boss command queued"), false),
                    Err(error) => app.set_status(error, true),
                }
            }
        });
}
