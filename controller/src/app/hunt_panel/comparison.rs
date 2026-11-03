use super::*;
pub(super) fn account_is_live(session:&NativeSession)->bool {
    let status=session.snapshot();
    status.phase=="Online" && status.last_received
        .is_some_and(|at|at.elapsed()<=Duration::from_secs(90))
}

/// Shared workspace navigation chooses the account. This control only
/// toggles Hunt Atlas's account-comparison mode; no duplicate account strip.
/// One navigation strip, including comparison for offline sessions.
pub(super) fn draw_atlas_view_controls(app:&mut ControllerApp,ui:&mut egui::Ui){
    ui.horizontal_wrapped(|ui|{
        for (tab,label,hint) in [
            (HuntSubtab::Explorer,tr("Explorer"),tr("Find Pokémon and their destinations")),
            (HuntSubtab::Recommendations,tr("Rankings"),tr("Compare unlocked hunt estimates")),
            (HuntSubtab::Current,tr("Current hunt"),tr("Selected hunt, activity and measured gains")),
            (HuntSubtab::Models,tr("Models"),tr("Capture, Outland and economy scenarios")),
            (HuntSubtab::Session,"Session",tr("Recorded battle rewards and history")),
            (HuntSubtab::Compare,tr("Compare"),tr("Compare all four accounts")),
        ]{
            if workspace_tab_button(ui,label,app.workspace.hunt_subtab==tab,hint).clicked(){
                app.workspace.hunt_subtab=tab;
            }
        }
    });
    ui.add_space(7.0);
}

/// Keep the current hunt visible while looking at other Atlas workspaces.
pub(super) fn draw_atlas_context(ui:&mut egui::Ui,view:&HuntView,live:bool)->bool{
    let current=view.hunts.iter().find(|h|
        view.hunt_slug.as_deref()==Some(h.slug.as_str()));
    let name=if view.in_center{tr("At Pokémon Center")}else{
        current.map(|h|h.name.as_str())
            .or(view.hunt_slug.as_deref()).unwrap_or(tr("No confirmed hunt"))
    };
    let (activity,color)=atlas_combat_status(view,live);
    let mut open=false;
    egui::Frame::new().fill(PANEL).corner_radius(9.0)
        .stroke(Stroke::new(1.0,BORDER))
        .inner_margin(egui::Margin::symmetric(12,9)).show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui|{
                ui.vertical(|ui|{
                    ui.label(RichText::new(tr(if !live {"LAST CONFIRMED HUNT"}
                        else if view.in_center {"CURRENT LOCATION"}
                        else {"CURRENT HUNT"}))
                        .size(9.0).strong().color(DIM));
                    ui.add(egui::Label::new(RichText::new(name)
                        .size(13.0).strong().color(TEXT)).truncate())
                        .on_hover_text(name);
                });
                ui.colored_label(color,format!("● {activity}"));
                let m=&view.measurements;
                if live && m.valid_trainer{
                    ui.label(RichText::new(format!(
                        "{}: {}",tr("Trainer XP/h"),format_hunt_amount(m.trainer_xp_h)
                    )).size(11.0).color(GOOD));
                }
                if live && m.valid_pokemon{
                    ui.label(RichText::new(format!(
                        "{}: {}",tr("Pokémon XP/h"),format_hunt_amount(m.pokemon_xp_h)
                    )).size(11.0).color(GOOD));
                }
                if ui.small_button(tr("View hunt →")).clicked(){open=true;}
            });
        });
    open
}

/// Outland tiers are a server-backed account setting, not eight hunt slugs.
/// This control deliberately stays outside Explorer and Models so an account
/// can choose its real active tier from any Hunt Atlas workspace.
pub(super) fn draw_atlas_outland_tier(
    ui:&mut egui::Ui,view:&HuntView,native:&NativeSession,
    account:usize,live:bool,error:&mut Option<String>,
){
    let active=view.outland_tier;
    let pending=view.pending_outland_tier;
    let level=view.trainer_level;
    let mut request=None;
    egui::Frame::new().fill(PANEL_ALT)
        .corner_radius(8.0).inner_margin(egui::Margin::symmetric(11,7))
        .show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new(tr("OUTLAND TIER"))
                    .size(10.0).strong().color(DIM));
                ui.label(RichText::new(match active{
                    Some(tier)=>tr_dynamic(format!("● Outland {tier} active")),
                    None=>tr("Awaiting game state").to_owned()
                }).size(11.0).strong().color(if active.is_some(){GOOD}else{WARN}));
                let label=pending.map(|tier|tr_dynamic(format!("Requesting Outland {tier}…")))
                    .unwrap_or_else(||tr("Choose tier").into());
                egui::ComboBox::from_id_salt(("atlas-outland-tier",account))
                    .selected_text(label)
                    .width(170.0)
                    .show_ui(ui,|ui|{
                        for tier in 1u8..=8{
                            let Some((threshold,multiplier))=
                                view.outland_requirement(tier) else{continue};
                            let eligible=view.unlocked_outland_tier(tier).is_some();
                            let enabled=live && active.is_some() &&
                                pending.is_none() && eligible && active!=Some(tier);
                            ui.add_enabled_ui(enabled,|ui|{
                                if ui.selectable_label(active==Some(tier),format!(
                                    "Outland {tier} · Lv {threshold} · ×{multiplier:.2} rare drops"
                                )).clicked(){
                                    request=Some(tier);
                                }
                            });
                        }
                    });
                if let Some(tier)=pending{
                    ui.colored_label(WARN,tr_dynamic(format!("Waiting for Outland {tier} confirmation")));
                }else if level.is_none(){
                    ui.label(RichText::new(tr("Trainer level unavailable"))
                        .size(10.0).color(MUTED));
                }else if level.is_some_and(|level|level<150){
                    ui.label(RichText::new(tr("Unlocks at trainer Lv 150"))
                        .size(10.0).color(MUTED));
                }else if !live{
                    ui.label(RichText::new(tr("Offline · read only"))
                        .size(10.0).color(WARN));
                }
            });
            if let Some(message)=view.outland_tier_error.as_deref(){
                ui.label(RichText::new(message).size(10.0).color(WARN));
            }
        });
    if let Some(tier)=request{
        match native.send_command(GameCommand::OutlandTier(tier)){
            Ok(())=>*error=None,
            Err(message)=>*error=Some(message),
        }
    }
}

pub(super) fn draw_atlas_compare(app:&mut ControllerApp,ui:&mut egui::Ui){
    ui.label(RichText::new(tr("Account comparison"))
        .size(16.0).strong().color(TEXT));
    ui.label(RichText::new(
        tr("Current measured results and best modeled unlocked hunts, side by side. Unknown and stale values remain unavailable.")
    ).size(11.0).color(MUTED));
    let mut views=Vec::new();
    for index in 0..app.runtimes.len(){
        let profile=app.runtimes[index].game.profile;
        let name=app.account_name(profile);
        let runtime = &mut app.runtimes[index];
        if let Some(native)=runtime.game.native.as_ref(){
            let view=native.hunt_snapshot();
            let live=account_is_live(native);
            runtime.hunt.hunt_estimate_cache.refresh(index,&view,app.hunt_reference.as_ref(),
                live,app.ui_options.atlas_aoe_enabled[index]);
            views.push((index,name,view,live));
        }else{
            views.push((index,name,HuntView::default(),false));
        }
    }
    let max_xp=views.iter().filter_map(|(_,_,view,live)|
        if *live && view.measurements.valid_trainer{
            Some(view.measurements.trainer_xp_h.max(0.0))
        }else{None}).fold(0.0_f64,f64::max);
    ui.add_space(8.0);
    ui.columns(if ui.available_width()>860.0 {2}else{1},|columns|{
        let count=columns.len();
        for (index,name,view,live) in &views {
            let ui=&mut columns[*index%count];
            ui.push_id(("atlas-compare",index),|ui|{
                egui::Frame::new().fill(PANEL)
                    .stroke(Stroke::new(1.0,if *index==app.workspace.hunt_account{
                        ACCENT
                    }else{BORDER}))
                    .corner_radius(10.0).inner_margin(egui::Margin::same(13))
                    .show(ui,|ui|{
                        ui.set_min_width(ui.available_width());
                        ui.horizontal_wrapped(|ui|{
                            ui.label(RichText::new(name).strong().size(14.0).color(TEXT));
                            ui.colored_label(if *live{GOOD}else{WARN},
                                if *live{tr("● Live")}else{tr("○ Offline / stale")});
                            let (activity,color)=atlas_combat_status(view,*live);
                            ui.colored_label(color,format!("● {activity}"));
                            if ui.small_button(tr("Open Atlas →")).clicked(){
                                app.select_workspace_account(*index);
                                app.workspace.hunt_subtab=HuntSubtab::Explorer;
                            }
                        });
                        let current=view.hunts.iter().find(|h|
                            view.hunt_slug.as_deref()==Some(h.slug.as_str()));
                        let location=if view.in_center{tr("At Pokémon Center")}else{
                            current.map(|h|h.name.as_str())
                                .or(view.hunt_slug.as_deref()).unwrap_or(tr("No confirmed hunt"))
                        };
                        ui.label(RichText::new(format!(
                            "Location: {location}"
                        )).size(11.0).color(TEXT));
                        ui.label(RichText::new(format!(
                            "Lead: {}",view.active_pokemon.as_ref()
                                .map(|p|p.name.as_str()).unwrap_or("—")
                        )).size(10.0).color(MUTED));
                        let current_model=view.hunt_slug.as_deref()
                            .and_then(|slug|app.runtimes[*index].hunt.hunt_estimate_cache.get(slug));
                        draw_atlas_combat_cues(ui,view.active_pokemon.as_ref(),current_model.as_ref(),
                            app.ui_options.atlas_aoe_enabled[*index]);
                        ui.add_space(5.0);
                        ui.horizontal_wrapped(|ui|{
                            let m=&view.measurements;
                            draw_rate(ui,tr("Trainer XP/h"),
                                if *live&&m.valid_trainer{Some(m.trainer_xp_h)}else{None},
                                RateSource::Measured);
                            ui.add_space(9.0);
                            draw_rate(ui,tr("Pokémon XP/h"),
                                if *live&&m.valid_pokemon{Some(m.pokemon_xp_h)}else{None},
                                RateSource::Measured);
                            ui.add_space(9.0);
                            draw_rate(ui,tr("Gross gold/h"),
                                if *live&&m.valid_gross_gold{Some(m.gold_gross_h)}else{None},
                                RateSource::Measured);
                            ui.add_space(9.0);
                            draw_rate(ui,tr("Net gold/h"),
                                if *live&&m.valid_gold{Some(m.gold_net_h)}else{None},
                                RateSource::Measured);
                        });
                        if max_xp>0.0 && *live && view.measurements.valid_trainer {
                            ui.add(egui::ProgressBar::new(
                                (view.measurements.trainer_xp_h/max_xp).clamp(0.0,1.0) as f32
                            ).text(tr("Relative measured trainer XP/h")));
                        }
                        ui.add_space(7.0);
                        ui.label(RichText::new(tr("TOP MODELED UNLOCKED HUNTS"))
                            .size(10.0).strong().color(DIM));
                        let mut estimates=view.hunts.iter().filter_map(|h|{
                            app.runtimes[*index].hunt.hunt_estimate_cache.get(&h.slug)
                                .map(|f|(h,f))
                        }).collect::<Vec<_>>();
                        estimates.sort_by(|a,b|b.1.trainer_xp_h.total_cmp(&a.1.trainer_xp_h));
                        for (rank,(hunt,forecast)) in estimates.iter().take(3).enumerate(){
                            ui.label(RichText::new(format!(
                                "{}. {} · {} {}",
                                rank+1,hunt.name,format_hunt_amount(forecast.trainer_xp_h),
                                tr("Trainer XP/h")
                            )).size(11.0).strong().color(TEXT));
                            draw_atlas_combat_cues(ui,view.active_pokemon.as_ref(),Some(forecast),
                                app.ui_options.atlas_aoe_enabled[*index]);
                        }
                        if estimates.is_empty(){
                            ui.label(RichText::new(if *live &&
                                app.runtimes[*index].hunt.hunt_estimate_cache.is_pending(){
                                tr("Calculating modeled rankings...")
                            }else{tr("Not enough battle data for a reliable estimate yet.")})
                                .size(10.0).color(MUTED));
                        }
                    });
                ui.add_space(10.0);
            });
        }
    });
}



