//! Native capture, restock and game automation controls; worker ownership stays in ControllerApp.
use super::*;

pub(super) fn draw_automation_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    // Keep section navigation above the only vertical scroll region. Market
    // rules can be very long; they should never push Catch controls offscreen.
    ui.horizontal_wrapped(|ui| {
        for (section, label, hint) in [
            (AutomationSection::Capture, "Auto Catch", "Capture preferences and live statistics"),
            (AutomationSection::Game, "Game & restock", "Server settings and restock status"),
            (AutomationSection::Market, "Market", "Automatic buying rules and Snatch"),
        ] {
            if workspace_tab_button(ui, label,
                app.automation_section == section, hint).clicked() {
                app.automation_section = section;
            }
        }
    });
    ui.add_space(7.0);
    ui.separator();
    let index = app.automation_account;
    egui::ScrollArea::vertical()
        .id_salt(("automation-page", index, app.automation_section as u8))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            draw_automation_contents(app, ui);
        });
}

pub(super) fn draw_automation_contents(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let index = app.automation_account;
    match app.automation_section {
        AutomationSection::Capture => {
            if app.games[index].native.is_some() {
                draw_native_automation(app, ui, index);
            } else {
                draw_native_missing_panel(app, ui, Some(index),
                    tr("Start this account to configure Auto Catch"));
            }
        }
        AutomationSection::Game => draw_game_automation_contents(app, ui),
        AutomationSection::Market => market_panel::draw(app, ui, index),
    }
}

pub(super) fn draw_game_automation_contents(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let index = app.automation_account;
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
            app.selected_details = None;
            app.panel_tab = PanelTab::Overview;
        }
    });
}

// The Overview never infers this setting from temporary capture readiness.
pub(super) fn auto_catch_setting_label(enabled:bool)->&'static str {
    if enabled {"Auto Catch on"} else {"Auto Catch off"}
}

// A separate, concrete operational signal for Automation only.
// Ready means the next valid target can be thrown at, not proof of a capture.
pub(super) fn auto_catch_operational_label(connected:bool,
    capture:&crate::native_gameplay::GameplayStatus)->&'static str {
    if !connected {
        "Auto Catch awaiting game connection"
    } else if !capture.capture_ready {
        "Auto Catch waiting for conditions"
    } else if capture.pending_results>0 {
        "Auto Catch active · capture receipts pending"
    } else {
        "Auto Catch ready for targets"
    }
}

#[cfg(test)]
mod auto_catch_ui_state_tests {
    use super::{auto_catch_setting_label,auto_catch_operational_label};
    use crate::native_gameplay::GameplayStatus;

    #[test]
    fn setting_is_independent_of_temporary_capture_readiness() {
        let mut capture=GameplayStatus::default();
        assert_eq!(auto_catch_setting_label(true),"Auto Catch on");
        assert_eq!(auto_catch_setting_label(false),"Auto Catch off");
        assert_eq!(auto_catch_operational_label(false,&capture),
            "Auto Catch awaiting game connection");
        capture.capture_ready=true;
        assert_eq!(auto_catch_setting_label(true),"Auto Catch on");
        assert_eq!(auto_catch_operational_label(true,&capture),
            "Auto Catch ready for targets");
        capture.pending_results=2;
        assert_eq!(auto_catch_setting_label(true),"Auto Catch on");
        assert_eq!(auto_catch_operational_label(true,&capture),
            "Auto Catch active · capture receipts pending");
        capture.capture_ready=false;
        capture.pending_results=2;
        assert_eq!(auto_catch_operational_label(true,&capture),
            "Auto Catch waiting for conditions");
    }
}

pub(super) fn draw_native_automation(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize) {
    let profile = app.games[index].profile;
    let Some(native) = app.games[index].native.as_ref() else { return; };
    let game = native.snapshot();
    let twitch = native.twitch_snapshot();

    egui::Frame::new()
        .fill(PANEL_ALT)
        .corner_radius(10.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                automation_badge(ui, "PokéIdle", &game.phase, game.phase == "Online");
                let twitch_status = format!("{} · {}/{} channels", tr(&twitch.phase), twitch.joined, twitch.desired);
                automation_badge(ui, "Twitch", &twitch_status, twitch.phase == "Online");
            });
            ui.add_space(7.0);
            ui.separator();
            ui.add_space(6.0);

            // The switch is a saved preference. Operational readiness is
            // independent and shown only on this Automation page.
            let capture_enabled=native.capture_settings().enabled;
            let capture=&game.native_capture_status;
            let capture_connected=game.phase=="Online";
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Auto Catch")).strong().size(11.0).color(TEXT));
                ui.label(RichText::new(tr(auto_catch_setting_label(capture_enabled)))
                    .size(11.0).strong()
                    .color(if capture_enabled {GOOD} else {MUTED}));
                ui.add_space(8.0);
                if ui.add_enabled(!app.capture_save_busy[index],
                    egui::Button::new(tr(if capture_enabled {"Disable"} else {"Enable"})))
                    .on_hover_text(tr("Turn Auto Catch on or off for this account"))
                    .clicked()
                {
                    // Use the same confirmed save/event flow as the editor.
                    // A failed vault write must not silently change the switch.
                    // Toggle ONLY the current saved switch. Do not silently
                    // commit unrelated unsaved changes in the editor.
                    let mut next=native.capture_settings();
                    next.enabled=!capture_enabled;
                    if let Err(error)=next.validate() {
                        app.capture_save_error[index]=Some(error);
                    } else {
                        app.capture_save_error[index]=None;
                        app.capture_save_busy[index]=true;
                        let tx=app.capture_save_tx.clone();
                        std::thread::spawn(move || {
                            let result=std::panic::catch_unwind(
                                std::panic::AssertUnwindSafe(||
                                    crate::native_auth::set_capture_settings(profile,&next)))
                                .unwrap_or_else(|_|Err("Encrypted native settings save panicked".into()));
                            let _=tx.send(CaptureSavedEvent{
                                index,config:next,toggle_only:true,result
                            });
                        });
                    }
                }
                if app.capture_save_busy[index] {
                    ui.label(RichText::new(tr("Saving Auto Catch settings"))
                        .size(10.0).color(WARN));
                }
            });
            if capture_enabled {
                let operational=auto_catch_operational_label(capture_connected,capture);
                ui.label(RichText::new(format!("{}: {}",tr("Capture operation"),tr(operational)))
                    .size(11.0).color(if capture_connected && capture.capture_ready {GOOD} else {WARN}));
                if capture_connected && !capture.capture_ready {
                    if let Some(reason)=capture.blocked_reason.as_deref() {
                        ui.label(RichText::new(tr(reason)).size(10.0).color(MUTED));
                    }
                }
            }
            // Only native-matched results are attributed to Auto Catch.
            // The server's broader ball-event totals may include VIP/manual.
            ui.horizontal_wrapped(|ui| {
                for (label,value) in [
                    ("Throws sent",capture.throws),
                    ("Auto Catch matched captures",capture.catches),
                    ("Auto Catch matched failures",capture.failures),
                    ("Pending capture receipts",capture.pending_results)
                ] {
                    ui.label(RichText::new(format!("{}: {}",tr(label),value))
                        .size(10.0).color(MUTED));
                }
            });

            ui.collapsing(tr("Capture diagnostics"),|ui| {
            if capture_enabled {
                let restock=&game.native_restock;
                let selected_ball=restock.rules.iter().find(|r|r.kind=="ball");
                let ball_detail=selected_ball.map(|ball| {
                    format!(" · {} #{}: {}/{}",
                        ball.name,ball.id,
                        ball.current.map(|n|n.to_string())
                            .unwrap_or_else(||"—".into()),ball.target)
                }).unwrap_or_default();
                ui.label(RichText::new(format!(
                    "{}: {}{} · {}: {}",
                    tr("Restock"),tr(&restock.phase),ball_detail,
                    tr("Stock awaiting confirmation"),
                    game.native_capture_status.unreflected_throws
                )).size(10.0).color(
                    if restock.last_error.is_some() {WARN} else {DIM}
                ));
                if let Some(error)=restock.last_error.as_deref() {
                    ui.label(RichText::new(tr(error)).size(10.0).color(WARN));
                }
                if game.native_capture_status.capture_resyncs>0 {
                    ui.label(RichText::new(format!(
                        "{}: {}",tr("Capture timeouts"),
                        game.native_capture_status.capture_resyncs
                    )).size(10.0).color(WARN));
                }
                if let Some(notice)=game.native_capture_status.last_server_notice.as_deref() {
                    ui.label(RichText::new(format!(
                        "{}: {}",tr("Capture server notice"),notice
                    )).size(10.0).color(WARN));
                }
                if let Some(warning)=game.native_capture_status.warning.as_deref() {
                    ui.label(RichText::new(tr(warning)).size(10.0).color(WARN));
                }
            }
            if game.native_capture_status.deaths_without_slot>0 {
                ui.label(RichText::new(format!("{} {}",
                    game.native_capture_status.deaths_without_slot,
                    tr("wild death events without capture slots")))
                    .size(10.0).color(WARN));
            }
            if !game.native_capture_status.last_action.is_empty() {
                ui.label(RichText::new(tr(&game.native_capture_status.last_action))
                    .size(10.0).color(MUTED));
            }
            if let Some(error)=game.native_capture_status.error.as_deref() {
                ui.label(RichText::new(tr(error)).size(10.0).color(WARN));
            }
            }); // Capture diagnostics: keep transient messages off account cards.
            ui.collapsing(tr("Auto Catch · ball statistics"),|ui|{
                let capture=&game.native_capture_status;
                if capture.used_by_ball.is_empty() &&
                    capture.server_caught_by_ball.is_empty() {
                    ui.label(RichText::new(tr("No Auto Catch attempts recorded yet"))
                        .size(10.0).color(DIM));
                }
                let ids=capture.used_by_ball.keys()
                    .chain(capture.server_caught_by_ball.keys())
                    .copied().collect::<std::collections::BTreeSet<_>>();
                for id in ids.iter() {
                    let sent=capture.used_by_ball.get(id).copied().unwrap_or(0);
                    let confirmed=capture.server_caught_by_ball.get(id).copied().unwrap_or(0);
                    let matched=capture.caught_by_ball.get(id).copied().unwrap_or(0);
                    ui.label(RichText::new(format!(
                        "Ball #{id}: {sent} sent · {confirmed} {} · {matched} {}",
                        tr("Ball-event captures"),tr("Native-matched")
                    )).size(10.0).color(MUTED));
                }
                ui.label(RichText::new(format!(
                    "{}: {} · {}: {} · {}: {}",
                    tr("Ball-event captures"),capture.server_catches,
                    tr("Unmatched receipts"),capture.unmatched_successes,
                    tr("Stock awaiting confirmation"),capture.unreflected_throws
                )).size(10.0).color(DIM));
                ui.label(RichText::new(
                    tr("Only observed ball battle events are counted; built-in VIP captures may not generate them.")
                ).size(10.0).color(DIM));
            });
        });
    ui.add_space(8.0);
    egui::CollapsingHeader::new(tr("Configure Auto Catch"))
        .id_salt(("native-capture-editor",index))
        .default_open(true)
        .show(ui,|ui|{
            draw_native_capture_config(app,ui,index);
        });
}

pub(super) fn capture_ball_name(k:&str)->&'static str {
    match k {"beast"=>"Beast Ball","ultra"=>"Ultra Ball","super"=>"Super Ball",
        "great"=>"Great Ball","poke"=>"Poké Ball",_=>"None"}
}
// Auto Catch+ restock picker: compact selectable items instead of raw IDs.
// IDs are the game protocol IDs. Public metadata
// only enriches captions; absent metadata never switches a saved selection.
// Compact, icon-only Auto Catch+ choice. Names and stock remain in tooltips.
// Every variant must occupy the same fixed-sized cell: a centered/justified
// fallback label used to stretch the "Off" cell across an entire column,
// hiding the real ball/potion icons on the next wrapped row.
pub(super) const RESTOCK_ICON_SIZE: f32 = 23.0;
pub(super) const RESTOCK_ICON_PADDING: i8 = 4;

pub(super) fn restock_selector_columns(available_width: f32) -> usize {
    // Each full picker needs roughly 240 px for its six or seven icon cells.
    // Overview account cards are often only 350–500 px wide, so use stacked
    // full-width groups there instead of three unusably narrow columns.
    if available_width >= 900.0 { 3 } else { 1 }
}

#[cfg(test)]
mod restock_picker_layout_tests {
    use super::restock_selector_columns;
    #[test]
    fn full_width_groups_on_account_cards_and_three_columns_on_wide_panels() {
        for width in [230.0, 350.0, 480.0, 779.0, 899.0] {
            assert_eq!(restock_selector_columns(width), 1);
        }
        for width in [900.0, 1100.0, 1400.0] {
            assert_eq!(restock_selector_columns(width), 3);
        }
    }
}

// Rebase only the restock portion onto the latest worker preferences. This
// prevents an old restock draft from resetting Auto Catch's ball priorities,
// reserves or enabled state when the two settings pages are edited in sequence.
pub(super) fn copy_restock_fields(to: &mut CatchSettings, from: &CatchSettings) {
    to.restock_enabled = from.restock_enabled;
    to.gold_reserve = from.gold_reserve;
    to.ball_id = from.ball_id;
    to.potion_id = from.potion_id;
    to.revive_id = from.revive_id;
    to.ball_at = from.ball_at;
    to.ball_to = from.ball_to;
    to.potion_at = from.potion_at;
    to.potion_to = from.potion_to;
    to.revive_at = from.revive_at;
    to.revive_to = from.revive_to;
}

pub(super) fn restock_fields_equal(a: &CatchSettings, b: &CatchSettings) -> bool {
    let mut updated = b.clone();
    copy_restock_fields(&mut updated, a);
    updated == *b
}

#[cfg(test)]
mod restock_selection_tests {
    use super::*;
    #[test]
    fn golden_potion_and_max_revive_preserve_independent_selections() {
        let mut staged = CatchSettings::default();
        staged.potion_id = 70070;
        staged.revive_id = 206;
        staged.restock_enabled = true;
        staged.validate().unwrap();

        let mut latest = CatchSettings::default();
        latest.enabled = true;
        latest.interval_ms = 800;
        copy_restock_fields(&mut latest, &staged);
        assert_eq!(latest.potion_id, 70070);
        assert_eq!(latest.revive_id, 206);
        assert!(latest.enabled);
        assert_eq!(latest.interval_ms, 800);
        assert!(restock_fields_equal(&staged, &latest));
        assert_eq!(CatchSettings::from_value(Some(&latest.as_value())), latest);

        staged.revive_id = 205;
        assert!(!restock_fields_equal(&staged, &latest));
        copy_restock_fields(&mut latest, &staged);
        assert_eq!(latest.potion_id, 70070);
        assert_eq!(latest.revive_id, 205);
    }
}

pub(super) fn capture_restock_chip(
    ui: &mut egui::Ui, visuals: &mut VisualCache, kind: SpriteKind,
    group: &'static str, selected: &mut u64, id: u64, name: &str, icon: &str,
    icon_hint: Option<&str>, price: Option<u64>, stock: Option<u64>,
) {
    let active = *selected == id;
    // A real Button owns both its artwork and pointer/keyboard interaction.
    // Interacting with a Frame's response after placing an Image inside it
    // let adjacent/wrapped controls compete for clicks, particularly at the
    // Golden Potion → Revives boundary. The ID is stable across icon loads.
    ui.push_id(("restock-item", group, id), |ui| {
        let button = if let Some(texture) = visuals.texture_id(ui, kind, id, icon_hint) {
            // Button::image clamps artwork to the text height; use the
            // regular Button atom to preserve the full 23px game icon.
            egui::Button::new(egui::Image::new((
                texture, egui::vec2(RESTOCK_ICON_SIZE, RESTOCK_ICON_SIZE)
            )))
        } else {
            egui::Button::new(RichText::new(icon).size(15.0))
        }
        .fill(if active { ACCENT.linear_multiply(0.25) } else { PANEL_ALT })
        .stroke(Stroke::new(if active { 1.5 } else { 1.0 },
            if active { ACCENT } else { BORDER }))
        .corner_radius(5.0)
        .selected(active);
        let price_hint = price.map(|p| format!(" · {p} gold")).unwrap_or_default();
        let stock_hint = stock.map(|n| format!(" · {n} in bag")).unwrap_or_default();
        if ui.add_sized(
            [RESTOCK_ICON_SIZE + 2.0 * RESTOCK_ICON_PADDING as f32;
             2], button
        ).on_hover_text(format!("{name}{price_hint}{stock_hint}")).clicked() {
            *selected = id;
        }
    });
}
pub(super) fn capture_restock_threshold(
    ui:&mut egui::Ui,label:&str,at:&mut u64,to:&mut u64,
    current:Option<u64>
) {
    ui.horizontal_wrapped(|ui|{
        ui.add_sized([77.0,18.0],egui::Label::new(
            RichText::new(label).strong().size(10.0).color(TEXT)));
        ui.label(RichText::new(tr("at ≤")).size(10.0).color(DIM));
        ui.add_sized([75.0,20.0],egui::DragValue::new(at).range(0..=1_000_000));
        ui.label(RichText::new(tr("to")).size(10.0).color(DIM));
        ui.add_sized([75.0,20.0],egui::DragValue::new(to).range(0..=1_000_000));
        if let Some(count)=current {
            ui.label(RichText::new(format!("{count} {}",tr("in bag"))).size(10.0)
                .color(if count <= *at {WARN} else {DIM}));
        }
    });
}
/// Live restock picker ported from Auto Catch+. Each account card edits only its
/// own authenticated native settings, with no item-ID input or explicit save step.
pub(super) fn draw_overview_restock(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize) {
    if !app.restock_open[index] { return; }
    request_item_catalog(app);
    app.visuals.poll(ui.ctx());
    let Some(native) = app.games[index].native.as_ref() else {
        ui.label(RichText::new(tr("Start this account to configure restock"))
            .size(10.0).color(WARN));
        return;
    };
    let profile = app.games[index].profile;
    let current = native.capture_settings();
    if app.restock_drafts[index].is_none() {
        app.restock_drafts[index] = Some(current.clone());
    }
    let cfg = app.restock_drafts[index].as_mut().expect("restock draft initialized");
    let previous = cfg.clone();
    {
    let restock_status=native.snapshot().native_restock;
    egui::Frame::new().fill(PANEL).corner_radius(8.0)
        .inner_margin(egui::Margin::same(10)).show(ui,|ui|{
        ui.horizontal_wrapped(|ui|{
            ui.checkbox(&mut cfg.restock_enabled,tr("Automatic restock"));
            ui.label(RichText::new(tr(&restock_status.phase)).size(10.0)
                .color(if restock_status.pending {WARN} else {DIM}));
            ui.label(RichText::new(format!(
                "{} ordered · {} confirmed",
                restock_status.ordered,restock_status.confirmed
            )).size(10.0).color(DIM));
        });
        let game_catalog=native.hunt_snapshot();
        let catalog=app.inventory_catalog.as_deref();
        let selected_stock=|kind:&str,id:u64| -> Option<u64> {
            // The authoritative HuntView inventory contains every owned item.
            // RestockStatus only contains already-saved rules, so it cannot
            // give stock for a newly selected potion/revive until saving ends.
            let inventory = if kind == "ball" {
                game_catalog.balls.as_ref()
            } else {
                game_catalog.items.as_ref()
            };
            selected_restock_stock(inventory, id).or_else(||
                restock_status.rules.iter()
                    .find(|rule|rule.kind==kind && rule.id==id)
                    .and_then(|rule|rule.current))
        };
        let selected_price=|kind:&str,id:u64| -> Option<u64> {
            restock_status.rules.iter()
                .find(|rule|rule.kind==kind && rule.id==id)
                .and_then(|rule|rule.price)
        };
        ui.add_space(5.0);
        let columns = restock_selector_columns(ui.available_width());
        ui.columns(columns, |cols| {
        let potion_col = if columns == 3 { 1 } else { 0 };
        let revive_col = if columns == 3 { 2 } else { 0 };
        cols[0].label(RichText::new(tr("Balls")).strong().size(11.0).color(TEXT));
        cols[0].horizontal_wrapped(|ui|{
            capture_restock_chip(ui,&mut app.visuals,SpriteKind::Ball,
                "ball", &mut cfg.ball_id,0,"Off","—",None,None,None);
            for (id, fallback) in [(1,"Poké Ball"),(2,"Great Ball"),
                (3,"Super Ball"),(4,"Ultra Ball"),(5,"Beast Ball")] {
                let name = game_catalog.ball_names.get(&id)
                    .map(String::as_str).unwrap_or(fallback);
                capture_restock_chip(ui,&mut app.visuals,SpriteKind::Ball,
                "ball", &mut cfg.ball_id,id,name,"●",None,
                    selected_price("ball",id),selected_stock("ball",id));
            }
            if cfg.ball_id>5 && !game_catalog.ball_names.contains_key(&cfg.ball_id) {
                let id=cfg.ball_id;
                capture_restock_chip(ui,&mut app.visuals,SpriteKind::Ball,
                "ball", &mut cfg.ball_id,id,&format!("Saved ball #{id}"),"●",None,None,None);
            }
        });
        if columns == 1 { cols[0].add_space(8.0); }
        cols[potion_col].label(RichText::new(tr("Potions")).strong().size(11.0).color(TEXT));
        // Match Auto Catch+: Small, Ultimate and Golden Potion. Preserve
        // any existing selection even if the downloaded catalog is old.
        let mut potion_choices=vec![
            (200_u64,String::from("Small Potion")),
            (201_u64,String::from("Great Potion")),
            (202_u64,String::from("Ultra Potion")),
            (203_u64,String::from("Hyper Potion")),
            (204_u64,String::from("Ultimate Potion")),
            (70070_u64,String::from("Golden Potion"))
        ];
        if let Some(c) = catalog {
            for (id, item) in &c.items {
                let name = item.name.to_lowercase();
                if (name.contains("potion") || name.contains("poção") ||
                    name.contains("pocao")) && item.price_gold.unwrap_or(0)>0 &&
                    !potion_choices.iter().any(|(existing,_)|existing==id) {
                    potion_choices.push((*id,item.name.clone()));
                }
            }
        }
        if cfg.potion_id>0 && !potion_choices.iter().any(|(id,_)|*id==cfg.potion_id) {
            let name=catalog.map(|c|c.name(cfg.potion_id))
                .unwrap_or_else(||format!("Saved potion #{}",cfg.potion_id));
            potion_choices.push((cfg.potion_id,name));
        }
        cols[potion_col].horizontal_wrapped(|ui|{
            capture_restock_chip(ui,&mut app.visuals,SpriteKind::Item,
                "potion", &mut cfg.potion_id,0,"Off","—",None,None,None);
            for (id,name) in potion_choices {
                let price=selected_price("item",id)
                    .or_else(||catalog.and_then(|c|c.items.get(&id))
                        .and_then(|item|item.price_gold));
                capture_restock_chip(ui,&mut app.visuals,SpriteKind::Item,
                "potion", &mut cfg.potion_id,id,&name,"💊",
                    catalog.and_then(|c|c.items.get(&id)).and_then(|m|m.icon.as_deref()),
                    price,selected_stock("item",id));
            }
        });
        if columns == 1 { cols[0].add_space(8.0); }
        cols[revive_col].label(RichText::new(tr("Revives")).strong().size(11.0).color(TEXT));
        let mut revive_choices=vec![
            (205_u64,String::from("Revive")),
            (206_u64,String::from("Max Revive"))
        ];
        if let Some(c)=catalog {
            for (id,item) in &c.items {
                if item.name.to_lowercase().contains("revive") &&
                    !revive_choices.iter().any(|(existing,_)|existing==id) {
                    revive_choices.push((*id,item.name.clone()));
                }
            }
        }
        if cfg.revive_id>0 && !revive_choices.iter().any(|(id,_)|*id==cfg.revive_id) {
            revive_choices.push((cfg.revive_id,format!("Saved revive #{}",cfg.revive_id)));
        }
        cols[revive_col].horizontal_wrapped(|ui|{
            capture_restock_chip(ui,&mut app.visuals,SpriteKind::Item,
                "revive", &mut cfg.revive_id,0,"Off","—",None,None,None);
            for (id,name) in revive_choices {
                let price=selected_price("item",id)
                    .or_else(||catalog.and_then(|c|c.items.get(&id))
                        .and_then(|item|item.price_gold));
                capture_restock_chip(ui,&mut app.visuals,SpriteKind::Item,
                "revive", &mut cfg.revive_id,id,&name,"✚",
                    catalog.and_then(|c|c.items.get(&id)).and_then(|m|m.icon.as_deref()),
                    price,selected_stock("item",id));
            }
        });
        });
        if cfg.restock_enabled && cfg.potion_id>0 {
            ui.label(RichText::new(tr(
                "Game Auto-Potion uses the potion selected here while restock is on. Its on/off switch and HP threshold stay unchanged."
            )).size(10.0).color(DIM));
        }
        if cfg.restock_enabled && cfg.revive_id>0 {
            ui.label(RichText::new(tr(
                "Game Auto-Revive uses the revive selected here while restock is on. Its on/off switch stays unchanged."
            )).size(10.0).color(DIM));
        }
        ui.add_space(10.0);
        ui.label(RichText::new(tr("Restock at ≤ / Fill to")).size(10.0).color(DIM));
        capture_restock_threshold(ui,tr("Balls"),&mut cfg.ball_at,&mut cfg.ball_to,
            selected_stock("ball",cfg.ball_id));
        capture_restock_threshold(ui,tr("Potions"),&mut cfg.potion_at,&mut cfg.potion_to,
            selected_stock("item",cfg.potion_id));
        capture_restock_threshold(ui,tr("Revives"),&mut cfg.revive_at,&mut cfg.revive_to,
            selected_stock("item",cfg.revive_id));
        ui.horizontal(|ui|{
            ui.label(RichText::new(tr("Gold reserve")).size(10.0).color(MUTED));
            ui.add(egui::DragValue::new(&mut cfg.gold_reserve).range(0..=u64::MAX));
        });
        if let Some(error)=restock_status.last_error.as_deref() {
            ui.label(RichText::new(tr(error)).size(10.0).color(WARN));
        }
    });
    }
    if *cfg != previous {
        app.restock_dirty[index] = true;
    }
    if let Some(error) = app.capture_save_error[index].as_deref() {
        ui.label(RichText::new(error).size(10.0).color(WARN));
    }
    if app.capture_save_busy[index] {
        ui.label(RichText::new(if app.restock_dirty[index] {
            tr("Restock changes queued; saving previous selection...")
        } else {
            tr("Saving encrypted preferences...")
        }).size(10.0).color(if app.restock_dirty[index] { WARN } else { DIM }));
    }
    if app.restock_dirty[index] && !app.capture_save_busy[index] {
        // A selection made during an earlier save stays visible and is sent
        // immediately after the previous acknowledgement. The latest current
        // worker configuration is used to avoid overwriting Auto Catch.
        let mut next = current;
        copy_restock_fields(&mut next,
            app.restock_drafts[index].as_ref().expect("restock draft initialized"));
        if let Err(error) = next.validate() {
            app.capture_save_error[index] = Some(error);
            app.restock_dirty[index] = false; // Retry after the next edit.
        } else {
            app.capture_save_error[index] = None;
            app.restock_dirty[index] = false;
            app.capture_save_busy[index] = true;
            let tx = app.capture_save_tx.clone();
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(
                    std::panic::AssertUnwindSafe(||
                        crate::native_auth::set_capture_settings(profile, &next)))
                    .unwrap_or_else(|_| Err("Encrypted restock save panicked".into()));
                let _ = tx.send(CaptureSavedEvent { index, config: next, toggle_only:false, result });
            });
        }
    }
}

pub(super) fn draw_native_capture_config(app:&mut ControllerApp,ui:&mut egui::Ui,index:usize) {
    let profile=app.games[index].profile;
    let Some(native)=app.games[index].native.as_ref() else {return};
    if app.capture_drafts[index].is_none() {
        app.capture_drafts[index]=Some(native.capture_settings());
    }
    let Some(cfg)=app.capture_drafts[index].as_mut() else{return};
    ui.add_enabled(!app.capture_save_busy[index],
        egui::Checkbox::new(&mut cfg.enabled,tr("Enable Auto Catch")));
    ui.horizontal_wrapped(|ui|{
        ui.label(RichText::new(tr("Throw interval")).size(10.0).color(MUTED));
        ui.add(egui::DragValue::new(&mut cfg.interval_ms)
            .range(100..=3000).speed(25.0).suffix(" ms"));
        ui.label(RichText::new(tr("Native throws can run every 0.1 s without waiting for capture receipts"))
            .size(10.0).color(DIM));
    });
    ui.label(RichText::new(tr("Preferred balls (primary and fallback)"))
        .size(11.0).color(TEXT));
    let keys=["beast","ultra","super","great","poke"];
    let mut chosen=[
        cfg.priorities.first().cloned().unwrap_or_default(),
        cfg.priorities.get(1).cloned().unwrap_or_default()
    ];
    ui.horizontal_wrapped(|ui|{
        for slot in 0..2 {
            ui.label(if slot==0{tr("Primary")}else{tr("Fallback")});
            egui::ComboBox::from_id_salt(("cap-choice",index,slot))
                .selected_text(capture_ball_name(&chosen[slot])).show_ui(ui,|ui|{
                    ui.selectable_value(&mut chosen[slot],String::new(),tr("None"));
                    for key in keys {
                        ui.selectable_value(&mut chosen[slot],key.into(),capture_ball_name(key));
                    }
                });
        }
    });
    cfg.priorities=chosen.iter().filter(|v|!v.is_empty()).cloned().collect();
    ui.label(RichText::new(tr("Uses the first ball with stock above reserve, then the fallback"))
        .size(10.0).color(DIM));
    ui.label(RichText::new(tr("Ball inventory reserves")).size(11.0).color(DIM));
    ui.horizontal_wrapped(|ui|{
        for key in keys {
            ui.vertical(|ui|{
                ui.label(RichText::new(capture_ball_name(key)).size(10.0).color(DIM));
                ui.add(egui::DragValue::new(cfg.reserves.entry(key.into()).or_default())
                    .range(0..=1_000_000));
            });
        }
    });
    ui.separator();
    ui.label(RichText::new(tr("Restock is configured on each Overview account card"))
        .size(10.0).color(DIM));
    if let Some(error)=app.capture_save_error[index].as_deref(){
        ui.colored_label(BAD,error);
    }
    let mut discard_edits=false;
    ui.horizontal(|ui|{
        if ui.add_enabled(!app.capture_save_busy[index],
            egui::Button::new(tr("Save settings"))).clicked() {
            let to_save=cfg.clone();
            if let Err(error)=to_save.validate() {
                app.capture_save_error[index]=Some(error);
            } else {
                app.capture_save_error[index]=None;
                app.capture_save_busy[index]=true;
                let tx=app.capture_save_tx.clone();
                std::thread::spawn(move || {
                    let result=std::panic::catch_unwind(
                        std::panic::AssertUnwindSafe(||
                            crate::native_auth::set_capture_settings(profile,&to_save))
                    ).unwrap_or_else(|_|Err("Encrypted native settings save panicked".into()));
                    let _=tx.send(CaptureSavedEvent{index,config:to_save,toggle_only:false,result});
                });
            }
        }
        if ui.small_button(tr("Discard")).clicked() {
            discard_edits=true;
        }
    });
    if discard_edits {
        app.capture_drafts[index]=Some(native.capture_settings());
        app.capture_save_error[index]=None;
    }
    if app.capture_save_busy[index] {
        ui.label(RichText::new(tr("Saving encrypted preferences..."))
            .size(10.0).color(MUTED));
    }
}

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
                    app.hunt_account = index;
                    app.panel_tab = PanelTab::Hunts;
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
        app.automation_account = index;
        app.panel_tab = PanelTab::Automation;
    }
}

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

// Detailed view uses close label/value pairs instead of pushing numbers to the
// far edge of large columns. Preserve full values in the hover tooltip.
pub(super) fn detail_stat_line(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_sized(
            [108.0, 18.0],
            egui::Label::new(RichText::new(tr(label)).size(10.0).color(DIM)),
        );
        ui.add(
            egui::Label::new(
                RichText::new(compact_text(value, 32)).size(11.0).strong().color(TEXT),
            ).truncate(),
        ).on_hover_text(value);
    });
}

pub(super) fn resource_chip(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(7.0)
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.label(RichText::new(compact_text(text, 24)).size(10.0).color(TEXT));
        });
}

pub(super) fn automation_badge(ui: &mut egui::Ui, label: &str, value: &str, good: bool) -> egui::Response {
    egui::Frame::new()
        .fill(if good { GOOD.linear_multiply(0.08) } else { PANEL_ALT })
        .stroke(Stroke::new(1.0, if good { GOOD.linear_multiply(0.28) } else { BORDER }))
        .corner_radius(7.0)
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("● {}", tr(label)))
                        .size(9.0)
                        .strong()
                        .color(if good { GOOD } else { MUTED }),
                );
                ui.add_space(4.0);
                ui.label(RichText::new(compact_text(tr(value), 54)).size(9.0).color(TEXT));
            });
        }).response
}

pub(super) fn bonus_chip(ui: &mut egui::Ui, text: &str) {
    let display = compact_text(text, 38);
    let measured = ui
        .painter()
        .layout_no_wrap(display.clone(), FontId::proportional(10.0), TEXT)
        .size();

    // Reserve the whole chip width before laying it out in a wrapping row.
    // Otherwise egui can give a chip only the remaining few pixels in the row,
    // causing each character to wrap vertically.
    let max_width = (ui.max_rect().width() - 20.0).max(44.0);
    let chip_width = (measured.x + 28.0).min(max_width);
    let text_width = (chip_width - 20.0).max(24.0);
    let chip_height = measured.y + 14.0;

    ui.allocate_ui_with_layout(
        egui::vec2(chip_width, chip_height),
        Layout::left_to_right(Align::Center),
        |chip_ui| {
            egui::Frame::new()
                .fill(ACCENT.linear_multiply(0.10))
                .stroke(Stroke::new(1.0, ACCENT.linear_multiply(0.28)))
                .corner_radius(7.0)
                .inner_margin(Margin::symmetric(9, 6))
                .show(chip_ui, |content_ui| {
                    content_ui.add_sized(
                        egui::vec2(text_width, measured.y),
                        egui::Label::new(
                            RichText::new(display)
                                .size(10.0)
                                .strong()
                                .color(TEXT),
                        )
                        .truncate(),
                    );
                });
        },
    );
}

