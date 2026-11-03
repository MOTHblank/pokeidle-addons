//! Native Auto Catch configuration, capture behavior and UI labels.
use super::*;

// The Overview never infers this setting from temporary capture readiness.
pub(super) fn auto_catch_setting_label(enabled:bool)->&'static str {
    if enabled {"Auto Catch on"} else {"Auto Catch off"}
}

// A separate, concrete operational signal for Automation only.
// Ready means the next valid target can be thrown at, not proof of a capture.
fn auto_catch_operational_label(connected:bool,
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
    let profile = app.runtimes[index].game.profile;
    let Some(native) = app.runtimes[index].game.native.as_ref() else { return; };
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
                if ui.add_enabled(!app.runtimes[index].capture.capture_save_busy,
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
                        app.runtimes[index].capture.capture_save_error=Some(error);
                    } else {
                        app.runtimes[index].capture.capture_save_error=None;
                        app.runtimes[index].capture.capture_save_busy=true;
                        app.runtimes[index].capture.capture_save_id=app.runtimes[index].capture.capture_save_id.wrapping_add(1);
                        let request_id=app.runtimes[index].capture.capture_save_id;
                        let tx=app.capture_save_tx.clone();
                        let generation_state=Arc::clone(&app.runtimes[index].capture.capture_generation);
                        let generation=generation_state.load(Ordering::Acquire);
                        let updater=native.capture_settings_updater();
                        std::thread::spawn(move || {
                            let result=std::panic::catch_unwind(
                                std::panic::AssertUnwindSafe(||
                                    crate::native_auth::persist_and_apply_capture_settings(
                                        profile,&next,&generation_state,generation,&updater)))
                                .unwrap_or_else(|_|Err("Encrypted native settings save panicked".into()));
                            let _=tx.send(CaptureSavedEvent{
                                index,generation,request_id,config:next,
                                origin:CaptureSaveOrigin::Toggle,result
                            });

                        });
                    }
                }
                if app.runtimes[index].capture.capture_save_busy {
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

fn capture_ball_name(k:&str)->&'static str {
    match k {"beast"=>"Beast Ball","ultra"=>"Ultra Ball","super"=>"Super Ball",
        "great"=>"Great Ball","poke"=>"Poké Ball",_=>"None"}
}

// The Capture editor owns only throwing preferences. Restock can be saved
// separately while this editor is open, so never write its stale restock fields.
fn copy_capture_fields(to:&mut CatchSettings, from:&CatchSettings) {
    to.enabled=from.enabled;
    to.priorities=from.priorities.clone();
    to.interval_ms=from.interval_ms;
    to.reserves=from.reserves.clone();
}

fn draw_native_capture_config(app:&mut ControllerApp,ui:&mut egui::Ui,index:usize) {
    let profile=app.runtimes[index].game.profile;
    let Some(native)=app.runtimes[index].game.native.as_ref() else {return};
    if app.runtimes[index].capture.capture_draft.is_none() {
        app.runtimes[index].capture.capture_draft=Some(native.capture_settings());
    }
    let Some(cfg)=app.runtimes[index].capture.capture_draft.as_mut() else{return};
    // Freeze every editor control during the asynchronous vault write: the
    // acknowledged snapshot can then safely replace the submitted draft.
    let saving=app.runtimes[index].capture.capture_save_busy;
    let mut discard_edits=false;
    ui.add_enabled_ui(!saving,|ui|{
        ui.checkbox(&mut cfg.enabled,tr("Enable Auto Catch"));
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
        ui.horizontal(|ui|{
            if ui.button(tr("Save settings")).clicked() {
                let mut to_save=native.capture_settings();
                copy_capture_fields(&mut to_save,cfg);
                if let Err(error)=to_save.validate() {
                    app.runtimes[index].capture.capture_save_error=Some(error);
                } else {
                    app.runtimes[index].capture.capture_save_error=None;
                    app.runtimes[index].capture.capture_save_busy=true;
                    app.runtimes[index].capture.capture_save_id=app.runtimes[index].capture.capture_save_id.wrapping_add(1);
                    let request_id=app.runtimes[index].capture.capture_save_id;
                    let tx=app.capture_save_tx.clone();
                    let generation_state=Arc::clone(&app.runtimes[index].capture.capture_generation);
                    let generation=generation_state.load(Ordering::Acquire);
                    let updater=native.capture_settings_updater();
                    std::thread::spawn(move || {
                        let result=std::panic::catch_unwind(
                            std::panic::AssertUnwindSafe(||
                                crate::native_auth::persist_and_apply_capture_settings(
                                    profile,&to_save,&generation_state,generation,&updater))
                        ).unwrap_or_else(|_|Err("Encrypted native settings save panicked".into()));
                        let _=tx.send(CaptureSavedEvent{
                            index,generation,request_id,config:to_save,
                            origin:CaptureSaveOrigin::Editor,result
                        });
                    });
                }
            }
            if ui.small_button(tr("Discard")).clicked() {
                discard_edits=true;
            }
        });
    });
    if let Some(error)=app.runtimes[index].capture.capture_save_error.as_deref() {
        ui.colored_label(BAD,error);
    }
    if discard_edits {
        app.runtimes[index].capture.capture_draft=Some(native.capture_settings());
        app.runtimes[index].capture.capture_save_error=None;
    }
    if app.runtimes[index].capture.capture_save_busy {
        ui.label(RichText::new(tr("Saving encrypted preferences..."))
            .size(10.0).color(MUTED));
    }
}


#[cfg(test)]
mod capture_editor_save_tests {
    use super::*;

    #[test]
    fn editor_save_merges_only_throwing_fields_with_current_restock() {
        let mut current=CatchSettings::default();
        current.restock_enabled=true;
        current.ball_id=5;
        current.potion_id=203;
        current.revive_id=206;
        current.gold_reserve=123;
        let mut editor=CatchSettings::default();
        editor.enabled=true;
        editor.interval_ms=250;
        editor.priorities=vec!["beast".into(),"ultra".into()];
        editor.reserves.insert("beast".into(),7);
        copy_capture_fields(&mut current,&editor);
        current.validate().unwrap();
        assert!(current.enabled);
        assert_eq!(current.interval_ms,250);
        assert_eq!(current.priorities,vec!["beast","ultra"]);
        assert_eq!(current.reserves.get("beast"),Some(&7));
        assert!(current.restock_enabled);
        assert_eq!(current.ball_id,5);
        assert_eq!(current.potion_id,203);
        assert_eq!(current.revive_id,206);
        assert_eq!(current.gold_reserve,123);
    }
}
