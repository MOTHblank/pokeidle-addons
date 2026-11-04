//! Native Hunt Atlas workspace. Account state comes from the authenticated
//! game socket; species metadata comes from public game-hosted catalogs.
use super::*;
use crate::native_atlas_models::{self as models, BallKind, Modifiers};
mod models_panel;


// Each module owns its calculation or view state; rendering only coordinates them.
mod forecast_cache;
pub(super) mod components;
mod explorer;
mod recommendations;
mod comparison;
mod current;
mod session;
#[cfg(test)]
mod tests;

use components::*;
use explorer::*;
use recommendations::*;
use comparison::*;
use current::*;
use session::*;

// Account runtimes only need these cache/filter handles.
pub(super) use forecast_cache::EstimateCache;
pub(super) use explorer::{AtlasFilters, ExplorerCache};
pub(super) use recommendations::RankingsCache;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum HuntSubtab { Explorer, Recommendations, Current, Models, Session, Compare }

#[derive(Clone,Debug,PartialEq)]
pub(super) struct AtlasScenario {
    hunt_slug:String,
    ball:BallKind,
    tier:u8,
    modifiers:Modifiers,
    rare_target:u64,
    accepted_fraction_override:Option<f64>,
    modifiers_open:bool,
}
impl Default for AtlasScenario {
    fn default()->Self{
        Self{hunt_slug:String::new(),ball:BallKind::Ultra,tier:1,
            modifiers:Modifiers::default(),rare_target:70000,
            accepted_fraction_override:None,
            modifiers_open:false}
    }
}

/// Per-account modeling preference. The controller never sends this flag to
/// the game; users tick it only when their Pokémon actually has AoE.
fn draw_atlas_aoe_checkbox(app:&mut ControllerApp,ui:&mut egui::Ui){
    let index=app.workspace.hunt_account;
    let account_name=app.account_name(app.runtimes[index].game.profile);
    let mut aoe_enabled=app.ui_options.atlas_aoe_enabled[index];
    egui::Frame::new().fill(PANEL_ALT)
        .stroke(Stroke::new(1.0,BORDER))
        .corner_radius(8.0).inner_margin(egui::Margin::symmetric(12,7))
        .show(ui,|ui|{
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new(account_name.as_str()).size(11.0).strong().color(TEXT));
                ui.checkbox(&mut aoe_enabled,tr("My Pokémon has AoE"))
                    .on_hover_text(tr("Check only if this account's active Pokémon has AoE. The checkbox changes Hunt Atlas simulations, not game settings."));
                ui.label(RichText::new(tr(
                    "Off by default · per-account · applies to every Hunt Atlas estimate"
                )).size(10.0).color(MUTED));
            });
        });
    if aoe_enabled!=app.ui_options.atlas_aoe_enabled[index] {
        let mut next=app.ui_options;
        next.atlas_aoe_enabled[index]=aoe_enabled;
        app.save_ui_options(next);
        // Keep the background history model consistent with the account's
        // explicit AoE preference; no additional simulation is started here.
        if app.ui_options.atlas_aoe_enabled[index]==aoe_enabled {
            if let (Some(native),Some(reference))=(
                app.runtimes[index].game.native.as_ref(),app.hunt_reference.as_ref()) {
                native.enable_background_forecasts(Arc::clone(reference),aoe_enabled);
            }
        }
        ui.ctx().request_repaint();
    }
}

pub(super) fn draw_hunt_atlas_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    if app.workspace.hunt_subtab==HuntSubtab::Explorer {
        egui::ScrollArea::vertical()
            .id_salt(("hunt-atlas-explorer-page",app.workspace.hunt_account))
            .auto_shrink([false,false])
            .show(ui,|ui|{
                ui.set_min_width(ui.available_width());
                draw_hunt_atlas_panel_contents(app,ui);
            });
    }else{
        draw_hunt_atlas_panel_contents(app,ui);
    }
}

fn draw_hunt_atlas_panel_contents(app: &mut ControllerApp, ui: &mut egui::Ui) {
    // Only load the independent public reference when this tab is actually
    // opened; game socket and normal dashboard stay extremely light.
    if app.hunt_reference.is_none() && app.hunt_reference_error.is_none() &&
        !app.hunt_reference_busy {
        app.hunt_reference_busy = true;
        let tx = app.hunt_reference_tx.clone();
        std::thread::Builder::new()
            .name("moth-hunt-reference-load".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(native_atlas::load)
                    .unwrap_or_else(|_| Err(tr("Pokémon data unavailable").into()));
                let _ = tx.send(result);
            })
            .expect("could not start Hunt Atlas reference loader");
    }
    draw_atlas_view_controls(app,ui);
    draw_atlas_aoe_checkbox(app,ui);
    ui.add_space(6.0);
    if app.workspace.hunt_subtab==HuntSubtab::Compare {
        egui::ScrollArea::vertical()
            .id_salt("hunt-atlas-all-accounts")
            .auto_shrink([false,true]).show(ui,|ui|{
                ui.set_min_width(ui.available_width());
                draw_atlas_compare(app,ui);
            });
        return;
    }
    if app.hunt_reference_busy {
        ui.label(RichText::new(tr("Loading Pokémon data..."))
            .size(10.0).color(MUTED));
    } else if let Some(error) = app.hunt_reference_error.clone() {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("{}: {}",
                tr("Pokémon data unavailable"), error)).size(10.0).color(WARN));
            if ui.small_button(tr("Retry")).clicked() {
                app.hunt_reference_error = None;
            }
        });
    }
    ui.add_space(6.0);

    let index=app.workspace.hunt_account;
    if app.runtimes[index].game.native.is_none() {
        egui::Frame::new().fill(PANEL)
            .stroke(Stroke::new(1.0,BORDER))
            .corner_radius(9.0).inner_margin(egui::Margin::same(14))
            .show(ui,|ui|{
                ui.label(RichText::new(tr("Hunt data unavailable"))
                    .size(15.0).strong().color(TEXT));
                ui.label(RichText::new(
                    tr("Start this account to load unlocked hunts, battle measurements and Pokémon.")
                ).size(11.0).color(MUTED));
                ui.horizontal_wrapped(|ui|{
                    let starting=app.runtimes[index].game.child.is_some()
                        || app.runtimes[index].game.orphaned_pid.is_some()
                        || app.runtimes[index].session.cached_native_busy
                        || app.runtimes[index].session.native_verifying
                        || app.runtimes[index].session.native_handoff_busy;
                    if starting {
                        ui.label(RichText::new(tr("Account starting or waiting for sign-in"))
                            .size(11.0).color(WARN));
                    } else if app.accounts[index].enabled {
                        if ui.button(tr("Start account")).clicked(){
                            app.launch_one(app.runtimes[index].game.profile);
                        }
                    } else if ui.button(tr("Enable account")).clicked(){
                        app.workspace.panel_tab=PanelTab::Accounts;
                    }
                    if ui.small_button(tr("Account settings")).clicked(){
                        app.workspace.panel_tab=PanelTab::Accounts;
                    }
                });
            });
        return;
    }
    let runtime = &mut app.runtimes[index];
    let native = runtime.game.native.as_ref().expect("native presence checked");
    let status=native.snapshot();
    let live=status.phase=="Online" && status.last_received
        .is_some_and(|received|received.elapsed()<=Duration::from_secs(90));
    let view=native.hunt_snapshot();
    // Preserve instant click feedback until the worker publishes the
    // authoritative pending/current state. Never leave optimistic UI stuck.
    if let Some(server_pending)=view.pending_hunt_slug.as_deref() {
        let replace=runtime.hunt.hunt_action_pending.as_ref()
            .is_none_or(|(slug,_)| slug != server_pending);
        if replace {
            runtime.hunt.hunt_action_pending =
                Some((server_pending.to_owned(), Instant::now()));
        }
    } else if runtime.hunt.hunt_action_pending.as_ref().is_some_and(|(slug,at)|
        !live || view.hunt_slug.as_deref()==Some(slug.as_str()) ||
        at.elapsed() > Duration::from_secs(3)) {
        runtime.hunt.hunt_action_pending=None;
    }
    ui.horizontal_wrapped(|ui|{
        ui.label(RichText::new(if live {tr("● Game connected")}else{tr("● Game stale")})
            .size(11.0).strong().color(if live {GOOD}else{WARN}));
        if let Some(received)=status.last_received {
            ui.label(RichText::new(tr_dynamic(format!("Last game packet: {}s ago",
                received.elapsed().as_secs()))).size(10.0).color(MUTED));
        }
        if let Some(error)=status.error.as_deref() {
            ui.add(egui::Label::new(RichText::new(error).size(10.0).color(WARN))
                .truncate()).on_hover_text(error);
        }
    });
    if !live {
        ui.label(RichText::new(tr("Disconnected or stale. Previous account data is read-only; rates and switching are unavailable."))
            .color(WARN).size(11.0));
    }
    if !view.catalog_ready {
        ui.label(RichText::new(tr("Loading available hunts from the game…"))
            .color(WARN));
        return;
    }
    runtime.hunt.hunt_estimate_cache.refresh(index,&view,app.hunt_reference.as_ref(),
        live,app.ui_options.atlas_aoe_enabled[index]);
    if live {
        if let Some(slug)=view.hunt_slug.as_deref() {
            let cache=&mut runtime.hunt.hunt_estimate_cache;
            if cache.archive_due(slug) {
                if let Some(model)=cache.get(slug){
                    if native.archive_hunt_forecast(slug,model.trainer_xp_h,
                        model.pokemon_xp_h,model.kills_h,model.calibrated) {
                        cache.archived(slug);
                    }
                }
            }
        }
    }
    if let Some(error)=view.last_error.as_deref(){
        ui.label(RichText::new(tr_dynamic(format!("Hunt switch: {error}"))).color(WARN));
    }
    if let Some(warning)=view.navigation_warning.as_deref(){
        ui.label(RichText::new(tr(warning)).color(WARN));
    }
    if view.pending_center {
        ui.label(RichText::new(tr("Waiting for a safe opening to reach the Pokémon Center…"))
            .color(WARN).size(11.0));
    }
    if let Some(error)=runtime.hunt.hunt_action_error.clone(){
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new(tr_dynamic(format!("Command: {error}"))).color(BAD));
            if ui.small_button(tr("Dismiss")).clicked(){runtime.hunt.hunt_action_error=None;}
        });
    }
    ui.add_space(6.0);
    if app.workspace.hunt_subtab!=HuntSubtab::Current &&
        draw_atlas_context(ui,&view,live){
        app.workspace.hunt_subtab=HuntSubtab::Current;
    }
    ui.add_space(6.0);
    draw_atlas_combat_profile(ui,&view,&runtime.hunt.hunt_estimate_cache,
        app.ui_options.atlas_aoe_enabled[index]);
    ui.add_space(6.0);
    draw_atlas_outland_tier(ui,&view,native,index,live,
        &mut runtime.hunt.hunt_action_error);
    ui.add_space(6.0);
    // Explorer uses one full-page ScrollArea; its result panes now size to
    // their contents rather than relying on available_height().
    if app.workspace.hunt_subtab==HuntSubtab::Explorer{
        draw_atlas_explorer(ui,&view,native,live,index,
            app.ui_options.atlas_aoe_enabled[index],
            app.hunt_reference.as_deref(),&mut runtime.hunt.atlas_filters,
            &mut app.workspace.hunt_filters.atlas_page[index],&runtime.hunt.hunt_estimate_cache,
            &runtime.hunt.atlas_scenarios,&mut runtime.hunt.atlas_results,
            &mut runtime.hunt.hunt_action_error,
            &mut runtime.hunt.hunt_action_pending);
        return;
    }
    egui::ScrollArea::vertical().id_salt(("hunt-atlas-workspace",index,app.workspace.hunt_subtab as u8))
        .auto_shrink([false,true]).show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            match app.workspace.hunt_subtab {
                HuntSubtab::Explorer | HuntSubtab::Compare => {}
                HuntSubtab::Models => {
                    let history=native.history_snapshot();
                    models_panel::draw_atlas_models(ui,&view,
                        app.hunt_reference.as_deref(),
                        &runtime.hunt.hunt_estimate_cache,
                        &mut runtime.hunt.atlas_scenarios,&history,live);
                }
                HuntSubtab::Current => {
                    draw_atlas_current(ui,&view,app.hunt_reference.as_deref(),
                        &runtime.hunt.hunt_estimate_cache,app.ui_options.atlas_aoe_enabled[index],live);
                }
                HuntSubtab::Session => {
                    draw_atlas_session(ui,&view,native,index,&mut app.workspace);
                }
                HuntSubtab::Recommendations => {
                    draw_atlas_recommendations(ui,&view,native,live,
                        &mut app.workspace,app.hunt_reference.as_ref(),
                        &mut runtime.hunt.hunt_rankings,&runtime.hunt.hunt_estimate_cache,
                        &mut runtime.hunt.hunt_action_error,
                        &mut runtime.hunt.hunt_action_pending,
                        app.ui_options.atlas_aoe_enabled[index]);
                }
            }
        });
}
