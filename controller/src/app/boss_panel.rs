//! Boss management UI; the game worker owns navigation and all game commands.
use super::{draw_native_missing_panel, egui, request_item_catalog, tr, ControllerApp, GameCommand};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn draw_boss_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let index = app.workspace.automation_account;
    if app.runtimes[index].game.native.is_none() {
        draw_native_missing_panel(app, ui, Some(index),
            tr("Start this account to manage server-native boss farming"));
        return;
    }
    request_item_catalog(app);
    let (view, hunt, live) = {
        let native = app.runtimes[index].game.native.as_ref().expect("checked above");
        (native.boss_snapshot(), native.hunt_snapshot(),
            native.snapshot().phase == "Online")
    };
    let now = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_secs();
    egui::ScrollArea::vertical()
        .id_salt(("boss-manager",index))
        .auto_shrink([false,false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.heading(tr("Server-native Boss Manager"));
            ui.label(tr("The game server repeats the chosen boss, heals and spends its entry tokens. Moth travels to the Pokémon Center and restores your original hunt when the run ends. It never uses Give Up automatically."));
            if !live {
                ui.colored_label(super::WARN,tr(
                    "Game disconnected. Boss controls will become available after reconnection."));
            }
            ui.add_space(10.0);
            if !view.catalog_ready {
                ui.label(tr("Waiting for the server's playable boss catalog."));
                return;
            }
            if view.catalog.is_empty() {
                ui.label(tr("No playable boss arenas are currently advertised by the server."));
                return;
            }
            if !view.state_known {
                ui.colored_label(super::WARN,tr("Waiting for authoritative server boss state."));
            }

            let selected = &mut app.workspace.boss_selected[index];
            if !view.catalog.iter().any(|entry| entry.key == *selected) {
                *selected = view.catalog[0].key.clone();
            }
            egui::ComboBox::from_id_salt(("boss-choice", index))
                .selected_text(view.catalog.iter().find(|entry| entry.key == *selected)
                    .map(|entry| entry.name.as_str()).unwrap_or("—"))
                .show_ui(ui, |ui| {
                    for entry in &view.catalog {
                        ui.selectable_value(selected, entry.key.clone(), &entry.name);
                    }
                });
            let chosen = view.catalog.iter().find(|entry| entry.key == *selected);
            let mut action = None;
            if let Some(entry) = chosen {
                let tokens = hunt.items.get(&entry.token_id).copied().unwrap_or(0);
                let attempts = tokens / entry.tokens_per_entry;
                ui.label(format!("{}: {} · {}: {} × {} · {}: {}",
                    tr("Trainer level"), hunt.trainer_level.unwrap_or_default(),
                    tr("Entry"), entry.tokens_per_entry, entry.token_name,
                    tr("Inventory"), tokens));
                ui.label(format!("{}: {} · {}: {}",
                    tr("Required level"), entry.min_level,
                    tr("Possible entries"), attempts));
                let ready = live && view.state_known && !view.busy && !view.auto && view.arena.is_none()
                    && hunt.trainer_level.unwrap_or(0) >= entry.min_level
                    && tokens >= entry.tokens_per_entry;
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(ready,
                        egui::Button::new(tr("Farm until tokens run out"))).clicked() {
                        action = Some(GameCommand::BossStart {
                            key: entry.key.clone(), repeat: true,
                        });
                    }
                    if ui.add_enabled(ready,
                        egui::Button::new(tr("Fight once"))).clicked() {
                        action = Some(GameCommand::BossStart {
                            key: entry.key.clone(), repeat: false,
                        });
                    }
                });
                if !ready && !view.busy && !view.auto && view.arena.is_none() {
                    ui.small(tr("Boss entry requires sufficient tokens and trainer level."));
                }
            }
            ui.add_space(7.0);
            ui.label(format!("{}: {}",tr("Boss status"),
                if view.phase.is_empty() { tr("Idle") } else { tr(&view.phase) }));
            if let Some(original) = view.original_hunt.as_deref() {
                ui.label(format!("{}: {}",tr("Return destination"),original));
            }
            if view.auto {
                ui.label(format!("{}: {}",tr("Server repeating"),
                    view.auto_key.as_deref().unwrap_or("—")));
            }
            if let Some(error) = view.warning.as_deref() {
                ui.colored_label(super::WARN,tr(error));
            }
            if let Some(error) = hunt.navigation_warning.as_deref() {
                ui.colored_label(super::WARN,tr(error));
            }
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(live && view.can_stop,
                    egui::Button::new(tr("Stop repetition and return"))).clicked() {
                    action = Some(GameCommand::BossStop);
                }
                if ui.add_enabled(live && view.arena.is_some(),
                    egui::Button::new(tr("Leave boss encounter"))).clicked() {
                    app.workspace.boss_leave_confirm[index] = true;
                }
                if app.workspace.boss_leave_confirm[index] {
                    ui.label(tr("Leaving an active boss encounter may forfeit its entry. Confirm?"));
                    if ui.button(tr("Confirm leave")).clicked() {
                        action = Some(GameCommand::BossLeave);
                        app.workspace.boss_leave_confirm[index] = false;
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        app.workspace.boss_leave_confirm[index] = false;
                    }
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.heading(tr("Boss loot history"));
            if let Some(error) = view.storage_warning.as_deref() {
                ui.colored_label(super::WARN, tr(error));
            }
            if let Ok(status) = view.write_status.lock() {
                if let Some(error) = status.warning.as_deref() {
                    ui.colored_label(super::WARN, error);
                }
            }
            ui.label(format!("{}: {}",tr("Confirmed boss victories recorded"),view.log.len()));
            let mut totals = BTreeMap::<u64,u64>::new();
            for battle in view.log.iter() {
                for drop in &battle.drops {
                    let total = totals.entry(drop.item_id).or_default();
                    *total = total.saturating_add(drop.quantity);
                }
            }
            if totals.is_empty() {
                ui.label(tr("No confirmed boss item drops recorded yet."));
            } else {
                ui.label(tr("Items acquired (retained history)"));
                ui.horizontal_wrapped(|ui| {
                    for (id, quantity) in totals {
                        let name = app.inventory_catalog.as_ref()
                            .map(|catalog| catalog.name(id))
                            .unwrap_or_else(|| format!("Item #{id}"));
                        ui.label(format!("{name} ×{quantity}"));
                    }
                });
            }
            ui.add_space(6.0);
            for result in view.log.iter().rev().take(60) {
                let elapsed = now.saturating_sub(result.at);
                let ago = if elapsed < 3600 {
                    format!("{}m",elapsed / 60)
                } else if elapsed < 86_400 {
                    format!("{}h",elapsed / 3600)
                } else {
                    format!("{}d",elapsed / 86_400)
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("{} · {}",result.boss, ago));
                    if result.drops.is_empty() {
                        ui.small(tr("No items"));
                    } else {
                        for drop in &result.drops {
                            let name = app.inventory_catalog.as_ref()
                                .map(|catalog| catalog.name(drop.item_id))
                                .unwrap_or_else(|| format!("Item #{}",drop.item_id));
                            ui.label(format!("{} ×{}",name,drop.quantity));
                        }
                    }
                });
                ui.separator();
            }
            if view.log.len() > 60 {
                ui.small(tr("Showing the latest 60 victories (up to 500 are retained per account)."));
            }

            if let Some(command) = action {
                let result = app.runtimes[index].game.native.as_ref()
                    .expect("native still present").send_command(command);
                match result {
                    Ok(()) => app.set_status(tr("Boss command queued"), false),
                    Err(error) => app.set_status(error, true),
                }
            }
        });
}
