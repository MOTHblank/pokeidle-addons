//! Game-side automation settings editor using existing native worker snapshots.
use super::{automation_badge, draw_native_missing_panel, egui, format_number,
    localize_status, tr, tr_dynamic, AutomationDraft, ControllerApp, GameCommand,
    PanelTab, RichText, DIM, MUTED, PANEL, PANEL_ALT, TEXT, WARN};

pub(super) fn draw_game_automation_contents(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let index = app.workspace.automation_account;
    let Some(native) = app.games[index].native.as_ref() else {
        draw_native_missing_panel(app,ui,Some(index),
            tr("Start this account to manage game settings"));
        return;
    };
    let snapshot = native.snapshot();
    let view = native.hunt_snapshot();
    if let Some(automation) = view.automation.as_ref() {
        if !app.automation_drafts[index].initialized {
            app.automation_drafts[index]=AutomationDraft::from_server(automation);
        }
        let mut commands:Vec<(String,serde_json::Value)>=Vec::new();
        let can_edit=snapshot.phase=="Online" &&
            snapshot.native_automation.pending_field.is_none();
        egui::Frame::new().fill(PANEL).corner_radius(10.0)
            .inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Game automation")).size(13.0).strong().color(TEXT));
                if ui.small_button(tr("Reload game settings")).clicked() {
                    app.automation_drafts[index]=AutomationDraft::from_server(automation);
                }
                if let Some(field)=snapshot.native_automation.pending_field.as_deref() {
                    ui.label(RichText::new(format!("{}: {field}",tr("Awaiting confirmation")))
                        .size(10.0).color(WARN));
                }
                ui.label(RichText::new(format!("{}: {} · {}: {}",
                    tr("Confirmed"),snapshot.native_automation.confirmed,
                    tr("Queued"),snapshot.native_automation.queued))
                    .size(10.0).color(DIM));
            });
            if let Some(error)=snapshot.native_automation.error.as_deref(){
                ui.label(RichText::new(tr_dynamic(localize_status(error.to_owned()))).size(10.0).color(WARN));
            }
            ui.label(RichText::new(tr(
                "Changes only take effect after PokéIdle confirms them."
            )).size(10.0).color(DIM));
            ui.add_space(8.0);
            let groups: [(&str, &[(&str, &str)]); 2] = [
                ("Recovery & selling", &[
                    ("autoPotion", "Auto Potion"),
                    ("autoRevive", "Auto Revive"),
                    ("autoVendaLoot", "Auto Sell"),
                    ("autoVoltarHunt", "Return to Hunt"),
                ]),
                ("Pokémon protection", &[
                    ("autoLockShiny", "Lock Shiny"),
                    ("autoLockNota9", "Lock Grade 9"),
                    ("autoLockP5", "Lock Potency 5"),
                ]),
            ];
            for (heading, fields) in groups {
                ui.label(RichText::new(tr(heading)).size(11.0).strong().color(MUTED));
                ui.horizontal_wrapped(|ui| {
                    for (key, label) in fields {
                        if let Some(current)=automation.get(*key).and_then(serde_json::Value::as_bool) {
                            let mut enabled=current;
                            if ui.add_enabled(can_edit,
                                egui::Checkbox::new(&mut enabled,tr(label))).changed() {
                                commands.push(((*key).into(),serde_json::json!(enabled)));
                            }
                        }
                    }
                });
                ui.add_space(5.0);
            }
            ui.add_space(10.0);
            let draft=&mut app.automation_drafts[index];
            ui.horizontal_wrapped(|ui| {
                if automation.get("hpLimiar").is_some() {
                    ui.label(tr("Potion HP threshold (%)"));
                    ui.add(egui::DragValue::new(&mut draft.hp_limit)
                        .range(0.0..=100.0).speed(0.5));
                    if ui.add_enabled(can_edit,egui::Button::new(tr("Apply HP"))).clicked() {
                        commands.push(("hpLimiar".into(),serde_json::json!(draft.hp_limit)));
                    }
                }
                if automation.get("autoLockNotaMin").is_some() {
                    ui.label(tr("Minimum lock grade"));
                    ui.add(egui::DragValue::new(&mut draft.lock_grade)
                        .range(0.0..=10.0).speed(0.1));
                    if ui.add_enabled(can_edit,egui::Button::new(tr("Apply grade"))).clicked() {
                        commands.push(("autoLockNotaMin".into(),
                            serde_json::json!(draft.lock_grade)));
                    }
                }
            });
        });
        for (field,value) in commands {
            if let Err(error)=native.send_command(GameCommand::SetAutomation {field,value}) {
                app.status=format!("Server automation: {error}");
                app.status_error=true;
            }
        }
        ui.add_space(10.0);
    } else {
        ui.label(RichText::new(tr("Loading game automation settings"))
            .size(11.0).color(WARN));
    }
    let restock = &snapshot.native_restock;
    egui::Frame::new().fill(PANEL_ALT).corner_radius(10.0)
        .inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Restock details"))
                    .size(13.0).strong().color(TEXT));
                automation_badge(ui, "Restock", &restock.phase,
                    restock.enabled && restock.phase == "Stock OK");
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("{} {} · {} {}", restock.ordered, tr("ordered"),
                    restock.confirmed, tr("confirmed")));
                if restock.pending { ui.label(RichText::new(tr("Pending")).color(WARN)); }
                if let Some(order) = restock.last_order.as_deref() {
                    ui.label(RichText::new(order).size(10.0).color(MUTED));
                }
            });
            if let Some(error) = restock.last_error.as_deref() {
                ui.label(RichText::new(tr(error)).size(10.0).color(WARN));
            }
            ui.add_space(8.0);
            for rule in &restock.rules {
                ui.horizontal_wrapped(|ui| {
                    let below_target=rule.current.is_some_and(|stock|stock<rule.low);
                    ui.label(RichText::new(&rule.name).size(11.0).strong()
                        .color(if below_target{WARN}else{TEXT}))
                        .on_hover_text(tr_dynamic(format!("Item #{}",rule.id)));
                    ui.label(RichText::new(format!(
                        "{} {} / {} · {} {}",
                        rule.current.map(|n|n.to_string()).unwrap_or_else(||"—".into()),
                        tr("in stock"),rule.target,
                        tr("buy at"),rule.low
                    )).size(10.0).color(DIM));
                    let price = rule.price.map(|n|format!("{} gold",format_number(n)))
                        .unwrap_or_else(||tr("Price unavailable").into());
                    ui.label(RichText::new(format!("{} · {}",price,rule.price_source))
                        .size(10.0).color(if rule.price.is_some() { MUTED } else { WARN }));
                });
            }
            if restock.enabled && restock.rules.is_empty() {
                ui.label(RichText::new(tr("No valid restock rules configured"))
                    .size(10.0).color(WARN));
            }
        });
    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tr("Restock is configured on each Overview account card"))
            .size(11.0).color(MUTED));
        if ui.small_button(tr("Open restock on Overview")).clicked() {
            app.restock_open[index] = true;
            app.workspace.selected_details = None;
            app.workspace.panel_tab = PanelTab::Overview;
        }
    });
}

