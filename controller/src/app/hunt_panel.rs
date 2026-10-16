//! Native Hunt Atlas workspace. All game state is read from the authenticated
//! native socket. The public guide provides optional, untrusted reference data.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum HuntSubtab { Recommendations, Current, Session }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RateSource { Measured, Calibrated, Experimental, Scenario, Unavailable }
impl RateSource {
    fn label(self) -> &'static str {
        match self {
            Self::Measured => "Measured",
            Self::Calibrated => "Calibrated estimate",
            Self::Experimental => "Experimental estimate",
            Self::Scenario => "Same-pace scenario",
            Self::Unavailable => "Unavailable",
        }
    }
    fn color(self) -> Color32 {
        match self {
            Self::Measured => GOOD,
            Self::Calibrated => WARN,
            Self::Experimental | Self::Scenario => MUTED,
            Self::Unavailable => DIM,
        }
    }
}

struct RankedHunt<'a> {
    hunt: &'a native_hunts::HuntEntry,
    forecast: Option<native_atlas::Forecast>,
    value: Option<f64>,
    source: RateSource,
}

/// At most one guide simulation for a stable account and hunt catalog per
/// eight seconds, regardless of egui's paint frequency or table filter changes.
/// Actual measured rates remain live and bypass this cache entirely.
#[derive(Default)]
pub(super) struct EstimateCache {
    account: Option<usize>,
    catalog_ptr: usize,
    reference_ptr: usize,
    active_signature: Option<(u64,u32,Option<u64>,Option<u64>,[Option<u64>;6],bool)>,
    hunt_slug: Option<String>,
    live: bool,
    refreshed: Option<Instant>,
    values: std::collections::BTreeMap<String,Option<native_atlas::Forecast>>,
}
impl EstimateCache {
    fn refresh(&mut self, account:usize,view:&HuntView,
        reference:Option<&AtlasReference>,live:bool,
    ){
        let catalog_ptr=Arc::as_ptr(&view.hunts) as usize;
        let reference_ptr=reference.map_or(0,|r|r as *const _ as usize);
        let active_signature=view.active_pokemon.as_ref().map(|p|
            (p.id,p.level,p.quality.map(f64::to_bits),p.potency,p.ivs,p.shiny));
        if self.account==Some(account) && self.catalog_ptr==catalog_ptr
            && self.reference_ptr==reference_ptr && self.live==live
            && self.active_signature==active_signature && self.hunt_slug==view.hunt_slug
            && self.refreshed.is_some_and(|at|at.elapsed()<Duration::from_secs(8))
        {return;}
        self.account=Some(account);
        self.catalog_ptr=catalog_ptr;
        self.reference_ptr=reference_ptr;
        self.active_signature=active_signature;
        self.live=live;
        self.hunt_slug=view.hunt_slug.clone();
        self.refreshed=Some(Instant::now());
        self.values.clear();
        if !live {return;}
        if let Some(reference)=reference {
            for hunt in view.hunts.iter() {
                self.values.insert(hunt.slug.clone(),
                    native_atlas::estimate(view,hunt,reference));
            }
        }
    }
    fn get(&self,slug:&str)->Option<native_atlas::Forecast>{
        self.values.get(slug).cloned().flatten()
    }
}

/// Preserve uncertainty: an absent forecast, invalid observation and unknown
/// kill rate must not become a plausible-looking zero in the recommendations.
fn ranked_hunts<'a>(
    view: &'a HuntView, reference: Option<&AtlasReference>, ranking: HuntRanking,
    live: bool, query: &str, region: &str, species_type: &str,
    cache: &EstimateCache,
) -> Vec<RankedHunt<'a>> {
    let needle = query.trim().to_lowercase();
    let m = &view.measurements;
    let observed_kills = if live && m.last_battle_age_secs.is_none_or(|age|age<=180)
        && m.seconds >= 20.0 && m.kills >= 5
        && m.kills_h.is_finite() && m.kills_h > 0.0 { Some(m.kills_h) } else { None };
    let mut ranked = view.hunts.iter().filter(|h| {
        (region.is_empty() || h.region == region) &&
        (species_type.is_empty() || h.species.iter().any(|sp|
            reference.and_then(|r|r.species.get(&sp.id)).is_some_and(|meta|
                meta.types.iter().any(|kind| kind == species_type)))) &&
        (needle.is_empty() || h.name.to_lowercase().contains(&needle) ||
            h.region.to_lowercase().contains(&needle) ||
            h.area.to_lowercase().contains(&needle) ||
            h.slug.to_lowercase().contains(&needle) ||
            h.species.iter().any(|sp| sp.id.to_string().contains(&needle) ||
                reference.and_then(|r|r.species.get(&sp.id))
                    .is_some_and(|meta|meta.name.to_lowercase().contains(&needle))))
    }).map(|hunt| {
        let actual = if live && m.last_battle_age_secs.is_none_or(|age|age<=180)
            && view.hunt_slug.as_deref() == Some(hunt.slug.as_str()) {
            match ranking {
                HuntRanking::TrainerXp if m.valid_trainer => Some(m.trainer_xp_h),
                HuntRanking::PokemonXp if m.valid_pokemon => Some(m.pokemon_xp_h),
                HuntRanking::Gold if m.valid_gold => Some(m.gold_net_h),
                _ => None,
            }
        } else { None }.filter(|n| n.is_finite());

        // Do not run an optimistic guide simulation against a disconnected or
        // stale account. That account's combat and bonus state is not current.
        let forecast = if live {cache.get(&hunt.slug)} else {None};
        let forecast_value = forecast.as_ref().map(|f|match ranking {
            HuntRanking::TrainerXp => f.trainer_xp_h,
            HuntRanking::PokemonXp => f.pokemon_xp_h,
            HuntRanking::Gold => f.gold_h,
        }).filter(|n|n.is_finite());

        // These are *unboosted* counterfactual rates, not predictions.
        // Applying guessed or additive bonuses would make them look precise.
        let scenario = observed_kills
            .and_then(|kills|native_hunts::atlas_pace_scenario(hunt,kills))
            .map(|(xp,gold)|match ranking {
                HuntRanking::TrainerXp | HuntRanking::PokemonXp => xp,
                HuntRanking::Gold => gold,
            }).filter(|n| n.is_finite());

        let (value,source) = if let Some(value) = actual {
            (Some(value),RateSource::Measured)
        } else if let Some(value) = forecast_value {
            (Some(value),if forecast.as_ref().is_some_and(|f|f.calibrated)
                && bonuses_complete(view,ranking) {
                RateSource::Calibrated
            } else { RateSource::Experimental })
        } else if let Some(value) = scenario {
            (Some(value),RateSource::Scenario)
        } else {
            (None,RateSource::Unavailable)
        };
        RankedHunt { hunt, forecast, value, source }
    }).collect::<Vec<_>>();
    ranked.sort_by(|a,b|match (a.value,b.value) {
        (Some(x),Some(y)) => y.total_cmp(&x),
        (Some(_),None) => std::cmp::Ordering::Less,
        (None,Some(_)) => std::cmp::Ordering::Greater,
        (None,None) => std::cmp::Ordering::Equal,
    }.then_with(||a.hunt.level.cmp(&b.hunt.level))
        .then_with(||a.hunt.name.cmp(&b.hunt.name)));
    ranked
}

fn rate_label(ranking: HuntRanking) -> &'static str {
    match ranking {
        HuntRanking::TrainerXp => "Trainer XP/h",
        HuntRanking::PokemonXp => "Pokémon XP/h",
        HuntRanking::Gold => "Gold/h",
    }
}

fn known_percent(value:f64,known:bool)->String {
    if known {format!("+{value:.1}%")} else {"— (not reported)".into()}
}
fn known_boost(active:bool,known:bool,clock_known:bool)->&'static str {
    if !known || !clock_known {"unknown"}
    else if active {"on"} else {"off"}
}
fn bonuses_complete(view:&HuntView,ranking:HuntRanking)->bool {
    if ranking==HuntRanking::Gold {return true;}
    let b=&view.bonuses;
    b.vip_known && b.guild_known && b.guild_boost_known && b.twitch_known
        && view.server_now.is_some() &&
        match ranking {
            HuntRanking::TrainerXp=>b.trainer_event_known,
            HuntRanking::PokemonXp=>b.pokemon_event_known,
            HuntRanking::Gold=>true,
        }
}
fn draw_rate(ui:&mut egui::Ui,label:&str,value:Option<f64>,source:RateSource){
    ui.vertical(|ui|{
        ui.label(RichText::new(label).size(10.0).color(DIM));
        ui.label(RichText::new(value.map(format_hunt_amount)
            .unwrap_or_else(||"—".into())).size(15.0).strong()
            .color(if value.is_some() {source.color()} else {MUTED}));
        ui.label(RichText::new(source.label()).size(9.0).color(source.color()));
    });
}

fn draw_hunt_row(
    ui:&mut egui::Ui, result:&RankedHunt<'_>, rank:usize,
    ranking:HuntRanking, view:&HuntView, reference:Option<&AtlasReference>,
    live:bool, native:&NativeSession, error:&mut Option<String>,
){
    let hunt=result.hunt;
    let current=view.hunt_slug.as_deref()==Some(hunt.slug.as_str());
    let pending=view.pending_hunt_slug.as_deref()==Some(hunt.slug.as_str());
    egui::Frame::new()
        .fill(if current {ACCENT.linear_multiply(0.11)} else {PANEL})
        .stroke(Stroke::new(1.0,if current {ACCENT}else{BORDER}))
        .corner_radius(8.0).inner_margin(egui::Margin::symmetric(12,9))
        .show(ui,|ui|{
            let wide=ui.available_width()>=650.0;
            if wide {
                ui.columns(3,|cols|{
                    cols[0].horizontal(|ui|{
                        ui.label(RichText::new(format!("{:02}",rank)).size(10.0).color(DIM));
                        ui.vertical(|ui|{
                            ui.add(egui::Label::new(RichText::new(&hunt.name)
                                .strong().size(12.0).color(TEXT)).truncate())
                                .on_hover_text(hunt.name.as_str());
                            ui.label(RichText::new(format!("{} · {} · Lv {}",hunt.area,
                                hunt.region,hunt.level)).size(10.0).color(MUTED));
                        });
                    });
                    draw_rate(&mut cols[1],rate_label(ranking),result.value,result.source);
                    cols[2].horizontal(|ui|{
                        if current { ui.label(RichText::new("CURRENT").color(GOOD)); }
                        else if pending { ui.label(RichText::new("Pending").color(WARN)); }
                        else if ui.add_enabled(live,egui::Button::new("Hunt")).clicked() {
                            if let Err(e)=native.send_command(GameCommand::Hunt(hunt.slug.clone())){
                                *error=Some(e);
                            } else {*error=None;}
                        }
                        if let Some(f)=result.forecast.as_ref(){
                            ui.vertical(|ui|{
                                ui.label(RichText::new(format!("{:.0} kills/h",f.kills_h))
                                    .size(11.0).color(TEXT));
                                ui.label(RichText::new(format!("{:.1}s / kill",f.seconds_per_kill))
                                    .size(10.0).color(DIM));
                            });
                        }
                    });
                });
            } else {
                ui.horizontal_wrapped(|ui|{
                    ui.label(RichText::new(format!("{:02} · {} · Lv {}",
                        rank,hunt.name,hunt.level)).strong().color(TEXT));
                    if current {ui.label(RichText::new("CURRENT").color(GOOD));}
                    else if pending {ui.label(RichText::new("Pending").color(WARN));}
                    else if ui.add_enabled(live,egui::Button::new("Hunt")).clicked(){
                        if let Err(e)=native.send_command(GameCommand::Hunt(hunt.slug.clone())){
                            *error=Some(e);
                        } else {*error=None;}
                    }
                });
                ui.label(RichText::new(format!("{} · {}",hunt.area,hunt.region))
                    .size(10.0).color(DIM));
                draw_rate(ui,rate_label(ranking),result.value,result.source);
            }
            ui.push_id(&hunt.slug,|ui|{
                ui.collapsing("Spawn composition · forecast assumptions",|ui|{
                    ui.label(RichText::new(format!(
                        "{} spawn slots · {} species",
                        hunt.total_spawns,hunt.species.len()
                    )).size(10.0).color(MUTED));
                    let sum=hunt.species.iter().map(|s|s.weight.max(0.0)).sum::<f64>();
                    for spawn in hunt.species.iter().take(60) {
                        let species=reference.and_then(|r|r.species.get(&spawn.id));
                        let name=species.map(|s|s.name.as_str()).unwrap_or("Unknown species");
                        let pct=if sum>0.0 {Some(100.0*spawn.weight.max(0.0)/sum)}
                            else {None};
                        ui.label(RichText::new(format!(
                            "{} · #{} · {}",
                            name,spawn.id,pct.map(|v|format!("{v:.1}%"))
                                .unwrap_or_else(||"weight unavailable".into())
                        )).size(10.0).color(TEXT));
                    }
                    if hunt.species.len()>60 {ui.label("Only the first 60 species shown.");}
                    match result.source {
                        RateSource::Measured => {
                            ui.label("Measured from the currently selected hunt, not a prediction.");
                        }
                        RateSource::Calibrated | RateSource::Experimental => {
                            if let Some(f)=result.forecast.as_ref(){
                                ui.label(format!(
                                    "Guide model: {:.0}% modeled species coverage · {:.1}s/kill · {:.0} kills/h.",
                                    f.coverage*100.0,f.seconds_per_kill,f.kills_h
                                ));
                            }
                            ui.label("Approximated DPS and spawn weights. Real move timing, captures, time outside combat, and item costs may differ.");
                        }
                        RateSource::Scenario => {
                            ui.label("Uses the currently measured kills/h at another hunt's base level. No bonus, DPS, or travel projection.");
                        }
                        RateSource::Unavailable => {
                            ui.label("No valid measured pace or sufficiently complete guide model is available.");
                        }
                    }
                });
            });
        });
}

pub(super) fn draw_hunt_atlas_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    // Only load the independent public reference when this tab is actually
    // opened; game socket, Twitch and normal dashboard stay extremely light.
    if app.hunt_reference.is_none() && app.hunt_reference_error.is_none() &&
        !app.hunt_reference_busy {
        app.hunt_reference_busy = true;
        let tx = app.hunt_reference_tx.clone();
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(native_atlas::load)
                .unwrap_or_else(|_| Err("Guide reference loader panicked".into()));
            let _ = tx.send(result);
        });
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tr("Account")).size(11.0).color(DIM));
        egui::ComboBox::from_id_salt("hunt-account")
            .selected_text(app.account_name(app.games[app.hunt_account].profile))
            .show_ui(ui, |ui| {
                for index in 0..app.games.len() {
                    let name = app.account_name(app.games[index].profile);
                    ui.selectable_value(&mut app.hunt_account, index, name);
                }
            });
        ui.add_space(8.0);
        ui.label(RichText::new(tr("Native server catalog · no userscripts"))
            .size(10.0).color(MUTED));
    });
    ui.add_space(4.0);
    if app.hunt_reference_busy {
        ui.label(RichText::new(tr("Loading species and move reference..."))
            .size(10.0).color(MUTED));
    } else if let Some(error) = app.hunt_reference_error.clone() {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("{}: {}",
                tr("Guide reference unavailable"), error)).size(10.0).color(WARN));
            if ui.small_button(tr("Retry")).clicked() {
                app.hunt_reference_error = None;
            }
        });
    } else if let Some(reference) = app.hunt_reference.as_deref() {
        ui.label(RichText::new(format!(
            "{} {} · {}",
            reference.species.len(), tr("species in guide"),
            tr("Server determines accessible hunts")
        )).size(10.0).color(MUTED));
    }
    ui.add_space(10.0);

    let index=app.hunt_account;
    let Some(native)=app.games[index].native.as_ref() else {
        ui.label(RichText::new("Account offline. Start its native session to see hunts.")
            .color(WARN));
        return;
    };
    let status=native.snapshot();
    let live=status.phase=="Online" && status.last_received
        .is_some_and(|received|received.elapsed()<=Duration::from_secs(90));
    let view=native.hunt_snapshot();
    ui.horizontal_wrapped(|ui|{
        ui.label(RichText::new(format!("Game: {}",status.phase))
            .strong().color(if live {GOOD}else{WARN}));
        if let Some(received)=status.last_received {
            ui.label(RichText::new(format!("Last game packet: {}s ago",
                received.elapsed().as_secs())).size(10.0).color(MUTED));
        }
        if let Some(error)=status.error.as_deref() {
            ui.add(egui::Label::new(RichText::new(error).size(10.0).color(WARN))
                .truncate()).on_hover_text(error);
        }
    });
    if !live {
        ui.label(RichText::new("Disconnected or stale. Previous account data is read-only; rates and switching are unavailable.")
            .color(WARN).size(11.0));
    }
    if !view.catalog_ready {
        ui.label(RichText::new("Waiting for server-authenticated hunt catalog…")
            .color(WARN));
        return;
    }
    if let Some(error)=view.last_error.as_deref(){
        ui.label(RichText::new(format!("Hunt switch: {error}")).color(WARN));
    }
    if let Some(error)=app.hunt_action_error[index].clone(){
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new(format!("Command: {error}")).color(BAD));
            if ui.small_button("Dismiss").clicked(){app.hunt_action_error[index]=None;}
        });
    }
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui|{
        ui.selectable_value(&mut app.hunt_subtab,HuntSubtab::Recommendations,
            "Recommendations");
        ui.selectable_value(&mut app.hunt_subtab,HuntSubtab::Current,"Current hunt");
        ui.selectable_value(&mut app.hunt_subtab,HuntSubtab::Session,"Session & rewards");
    });
    ui.separator();
    egui::ScrollArea::vertical().id_salt(("hunt-atlas-workspace",index,app.hunt_subtab as u8))
        .auto_shrink([false,false]).show(ui,|ui|{
            match app.hunt_subtab {
                HuntSubtab::Current => {
                    ui.label(RichText::new("Current Pokémon, combat measurements and bonuses")
                        .size(12.0).color(MUTED));
                    let hunts=&view;
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .inner_margin(egui::Margin::same(14)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr("Active Pokémon")).size(10.0).color(DIM));
                    if let Some(p) = hunts.active_pokemon.as_ref() {
                        ui.label(RichText::new(format!("{} · Lv. {}", p.name, p.level))
                            .size(16.0).strong().color(TEXT));
                        ui.label(RichText::new(format!(
                            "P{} · Q {} · {} · {} Pokémon",
                            p.potency.map(|n|n.to_string()).unwrap_or_else(||"—".into()),
                            p.quality.map(|n|format!("{n:.3}")).unwrap_or_else(||"—".into()),
                            if p.shiny { "Shiny" } else { "Normal" },
                            hunts.pokemon_count
                        )).size(10.0).color(DIM));
                        let labels = ["HP", "ATK", "DEF", "SpA", "SpD", "SPD"];
                        let stats = labels.iter().enumerate()
                            .map(|(i,k)| format!("{} {}", k,
                                p.ivs[i].map(|v|v.to_string()).unwrap_or_else(|| "—".into())))
                            .collect::<Vec<_>>().join(" · ");
                        ui.label(RichText::new(stats).size(10.0).color(MUTED));
                    } else {
                        ui.label(tr("Waiting for active Pokémon state"));
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr("Bonuses considered")).size(10.0).color(DIM));
                    let b = &hunts.bonuses;
                    ui.label(format!("Twitch {} · Guild {}",
                        known_percent(b.twitch_pct,b.twitch_known),
                        known_percent(b.guild_pct,b.guild_known)));
                    ui.label(format!("Event: Trainer {} · Pokémon {}",
                        known_percent(b.trainer_event_pct,b.trainer_event_known),
                        known_percent(b.pokemon_event_pct,b.pokemon_event_known)));
                    ui.label(RichText::new(format!(
                        "VIP: {} · Guild boost: {}",
                        known_boost(b.vip_active,b.vip_known,hunts.server_now.is_some()),
                        known_boost(b.guild_boost_active,b.guild_boost_known,hunts.server_now.is_some()),
                    )).size(10.0).color(DIM));
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr("Current hunt")).size(10.0).color(DIM));
                    let title = hunts.hunts.iter().find(|h|Some(h.slug.as_str()) == hunts.hunt_slug.as_deref())
                        .map(|h|h.name.as_str()).or(hunts.hunt_slug.as_deref()).unwrap_or("—");
                    ui.label(RichText::new(title).size(14.0).strong().color(TEXT));
                    ui.label(RichText::new(format!(
                        "{} {} · {} {}",
                        hunts.hunts.len(), tr("available hunts"),
                        hunts.battle_kills, tr("observed kills")
                    )).size(10.0).color(DIM));
                });
            });
        });
    ui.add_space(10.0);
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .inner_margin(egui::Margin::same(13)).show(ui,|ui|{
            let current=hunts.hunts.iter().find(|h|
                hunts.hunt_slug.as_deref()==Some(h.slug.as_str()));
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new(tr("Current hunt details"))
                    .size(13.0).strong().color(TEXT));
                if let Some(h)=current {
                    ui.label(RichText::new(format!(
                        "{} · {} · Lv {} · {} {}",
                        h.area,h.region,h.level,h.total_spawns,tr("spawn slots")
                    )).size(11.0).color(MUTED));
                }
                if hunts.in_center {ui.label(RichText::new(tr("At Pokémon Center")).color(WARN));}
                if let Some(slug)=hunts.pending_hunt_slug.as_deref() {
                    ui.label(RichText::new(format!("{} {slug}",tr("Switch pending")))
                        .size(10.0).color(WARN));
                }
            });
            if let Some(wild)=hunts.wild.as_ref(){
                ui.label(RichText::new(format!(
                    "{}: {} · Lv {} · HP {}/{}",tr("Wild encounter"),
                    wild.name,wild.level,wild.hp,wild.max_hp
                )).size(10.0).color(DIM));
            }
            if let Some(map)=hunts.field_map.as_deref() {
                ui.label(RichText::new(format!("Field: {map}")).size(10.0).color(DIM));
            }
        });
    ui.add_space(10.0);
    let m = &hunts.measurements;
    egui::Frame::new().fill(PANEL_ALT).corner_radius(10.0)
        .inner_margin(egui::Margin::same(13)).show(ui, |ui| {
            ui.label(RichText::new(tr("Measured during current hunt"))
                .size(12.0).strong().color(TEXT));
            ui.horizontal_wrapped(|ui| {
                for (label, amount, valid) in [
                    (tr("Trainer XP/h"), m.trainer_xp_h, live && m.valid_trainer),
                    (tr("Pokémon XP/h"), m.pokemon_xp_h, live && m.valid_pokemon),
                    (tr("Net gold/h"), m.gold_net_h, live && m.valid_gold),
                    (tr("Kills/h"), m.kills_h, live && m.seconds >= 20.0 &&
                        m.last_battle_age_secs.is_none_or(|age|age<=180)),
                ] {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(label).size(10.0).color(DIM));
                        ui.label(RichText::new(if valid {
                            format_hunt_amount(amount)
                        } else { "—".into() }).size(15.0).strong()
                            .color(if valid { GOOD } else { MUTED }));
                    });
                    ui.add_space(13.0);
                }
            });
            ui.label(RichText::new(format!(
                "{} · {:.0}s · {} {}",
                hunts.sample_quality, m.seconds, m.kills, tr("kills in window")
            )).size(10.0).color(MUTED));
        });

                }
                HuntSubtab::Session => {
                    let session=&view.session;
                    ui.label(RichText::new("Native session · authoritative battle receipts")
                        .size(15.0).strong().color(TEXT));
                    ui.label(format!("Duration {:02}:{:02}:{:02}",
                        session.duration_secs/3600,
                        session.duration_secs%3600/60,
                        session.duration_secs%60));
                    ui.add_space(8.0);
                    egui::Frame::new().fill(PANEL).corner_radius(8.0)
                        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
                            ui.columns(3,|cols|{
                                detail_stat_line(&mut cols[0],"Kills",&format_number(session.kills));
                                detail_stat_line(&mut cols[0],"Captures",&format_number(session.captures));
                                detail_stat_line(&mut cols[1],"Trainer XP",&format_number(session.trainer_xp));
                                detail_stat_line(&mut cols[1],"Pokémon XP",&format_number(session.pokemon_xp));
                                detail_stat_line(&mut cols[2],"Combat gold",&format_number(session.combat_gold));
                                detail_stat_line(&mut cols[2],"Auto-sale gold",&format_number(session.auto_sale_gold));
                            });
                            ui.label(format!("Shiny encounter events: {} · shiny captures: {}",
                                session.shiny_seen,session.shiny_captured));
                            let throws=session.balls_used.values().sum::<u64>();
                            ui.label(format!("Identified ball throws: {} · confirmed successes: {}",
                                throws,session.ball_captures));
                            if throws>0{
                                ui.label(format!("Observed success per ball: {:.1}% ({} throws)",
                                    session.ball_captures as f64*100.0/throws as f64,throws))
                                    .on_hover_text("Observed ball outcomes in this session; not a species capture probability.");
                            }else {
                                ui.label(RichText::new("Capture success rate unavailable (no ball receipts)")
                                    .color(MUTED));
                            }
                        });
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui|{
                        if !app.session_reset_confirm[index] {
                            if ui.small_button("Reset session counters").clicked() {
                                app.session_reset_confirm[index]=true;
                            }
                        } else {
                            ui.label(RichText::new("Clear session-only totals?").color(WARN));
                            if ui.small_button("Confirm").clicked(){
                                native.reset_hunt_session();
                                app.session_reset_confirm[index]=false;
                            }
                            if ui.small_button("Cancel").clicked(){
                                app.session_reset_confirm[index]=false;
                            }
                        }
                        if ui.small_button("Open persistent history").clicked(){
                            app.history_account=index;
                            app.panel_tab=PanelTab::History;
                        }
                    });
                    ui.add_space(10.0);
                    ui.label(RichText::new("Balls used").size(13.0).strong().color(TEXT));
                    if session.balls_used.is_empty(){ui.label("No ball throws recorded yet.");}
                    for (id,count) in &session.balls_used {
                        let name=view.ball_names.get(id).map(String::as_str).unwrap_or("Unknown ball");
                        ui.label(format!("{name} (#{id}) · {count}"));
                    }
                    ui.separator();
                    ui.label(RichText::new("Items acquired from battles").size(13.0).strong().color(TEXT));
                    if session.acquired.is_empty(){ui.label("No confirmed drops yet.");}
                    let mut drops=session.acquired.iter().collect::<Vec<_>>();
                    drops.sort_by(|a,b|b.1.cmp(a.1).then_with(||a.0.cmp(b.0)));
                    for (name,qty) in drops.iter().take(200) {
                        ui.label(format!("{name} ×{qty}"));
                    }
                    if drops.len()>200 {ui.label("Showing the first 200 items.");}
                    ui.collapsing("Most recent drops",|ui|{
                        for (name,count) in &session.recent_drops{
                            ui.label(format!("{name} ×{count}"));
                        }
                    });
                }
                HuntSubtab::Recommendations => {
                    let hunts=&view;
                    let m=&hunts.measurements;
                    let current=hunts.hunts.iter().find(|h|
                        hunts.hunt_slug.as_deref()==Some(h.slug.as_str()));
                    egui::Frame::new().fill(PANEL_ALT).corner_radius(8.0)
                        .inner_margin(egui::Margin::same(10)).show(ui,|ui|{
                            ui.horizontal_wrapped(|ui|{
                                ui.vertical(|ui|{
                                    ui.label(RichText::new("CURRENT HUNT").size(10.0).color(DIM));
                                    ui.label(RichText::new(current.map(|h|h.name.as_str())
                                        .or(hunts.hunt_slug.as_deref()).unwrap_or("Unavailable"))
                                        .strong().size(14.0).color(TEXT));
                                });
                                ui.add_space(12.0);
                                draw_rate(ui,"Trainer XP/h",
                                    if live && m.valid_trainer {Some(m.trainer_xp_h)} else {None},
                                    RateSource::Measured);
                                ui.add_space(12.0);
                                draw_rate(ui,"Pokémon XP/h",
                                    if live && m.valid_pokemon {Some(m.pokemon_xp_h)} else {None},
                                    RateSource::Measured);
                                ui.add_space(12.0);
                                draw_rate(ui,"Net gold/h",
                                    if live && m.valid_gold {Some(m.gold_net_h)} else {None},
                                    RateSource::Measured);
                            });
                            ui.label(RichText::new(format!("{} · {:.0}s · {} kills",
                                hunts.sample_quality,m.seconds,m.kills))
                                .size(10.0).color(MUTED));
                        });
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui|{
                        ui.label(RichText::new("Sort recommendations by").size(12.0).strong());
                        ui.selectable_value(&mut app.hunt_ranking,HuntRanking::TrainerXp,"Trainer XP");
                        ui.selectable_value(&mut app.hunt_ranking,HuntRanking::PokemonXp,"Pokémon XP");
                        ui.selectable_value(&mut app.hunt_ranking,HuntRanking::Gold,"Gold");
                    });
                    ui.horizontal_wrapped(|ui|{
                        if ui.add(egui::TextEdit::singleline(&mut app.hunt_query)
                            .hint_text("Hunt, area, region or species").desired_width(220.0)).changed(){
                            app.hunt_page=0;
                        }
                        let mut regions=hunts.hunts.iter().map(|h|h.region.as_str())
                            .filter(|r|!r.is_empty()).collect::<Vec<_>>();
                        regions.sort_unstable();regions.dedup();
                        egui::ComboBox::from_id_salt("hunt-region")
                            .selected_text(if app.hunt_region.is_empty(){
                                "All regions".to_owned()
                            }else{app.hunt_region.clone()})
                            .show_ui(ui,|ui|{
                                if ui.selectable_value(&mut app.hunt_region,
                                    String::new(),"All regions").changed(){app.hunt_page=0;}
                                for region in regions{
                                    if ui.selectable_value(&mut app.hunt_region,
                                        region.to_owned(),region).changed(){app.hunt_page=0;}
                                }
                            });
                        if let Some(reference)=app.hunt_reference.as_deref(){
                            let mut types=reference.species.values()
                                .flat_map(|p|p.types.iter().map(String::as_str))
                                .collect::<Vec<_>>();
                            types.sort_unstable();types.dedup();
                            egui::ComboBox::from_id_salt("hunt-species-type")
                                .selected_text(if app.hunt_type.is_empty(){
                                    "All types".to_owned()
                                }else{app.hunt_type.clone()})
                                .show_ui(ui,|ui|{
                                    if ui.selectable_value(&mut app.hunt_type,
                                        String::new(),"All types").changed(){app.hunt_page=0;}
                                    for kind in types{
                                        if ui.selectable_value(&mut app.hunt_type,
                                            kind.to_owned(),kind).changed(){app.hunt_page=0;}
                                    }
                                });
                        }
                        if ui.small_button("Clear filters").clicked(){
                            app.hunt_query.clear();app.hunt_region.clear();
                            app.hunt_type.clear();app.hunt_page=0;
                        }
                    });
                    let reference=app.hunt_reference.as_deref();
                    app.hunt_estimate_cache.refresh(index,hunts,reference,live);
                    let ranked=ranked_hunts(hunts,reference,app.hunt_ranking,live,
                        &app.hunt_query,&app.hunt_region,&app.hunt_type,
                        &app.hunt_estimate_cache);
                    ui.label(RichText::new(format!(
                        "{} matching hunts · measured = this hunt only · guide estimates are uncertain · scenarios are not combat predictions",
                        ranked.len()
                    )).size(10.0).color(WARN));
                    if !live {ui.label("Rankings disabled while account data is stale.");}
                    const PAGE_SIZE:usize=35;
                    let total_pages=ranked.len().div_ceil(PAGE_SIZE).max(1);
                    app.hunt_page=app.hunt_page.min(total_pages-1);
                    ui.horizontal(|ui|{
                        if ui.add_enabled(app.hunt_page>0,egui::Button::new("Previous")).clicked(){
                            app.hunt_page-=1;
                        }
                        ui.label(format!("Page {} of {}",app.hunt_page+1,total_pages));
                        if ui.add_enabled(app.hunt_page+1<total_pages,
                            egui::Button::new("Next")).clicked(){app.hunt_page+=1;}
                    });
                    let first=app.hunt_page*PAGE_SIZE;
                    for (offset,result) in ranked.iter().skip(first).take(PAGE_SIZE).enumerate(){
                        draw_hunt_row(ui,result,first+offset+1,app.hunt_ranking,
                            hunts,reference,live,native,&mut app.hunt_action_error[index]);
                        ui.add_space(5.0);
                    }
                    if ranked.is_empty(){ui.label("No hunts match the current filters.");}
                    ui.add_space(7.0);
                    ui.collapsing("XP bonus assumptions",|ui|{
                        let b=&hunts.bonuses;
                        ui.label(format!(
                            "Trainer event: {} · Pokémon event: {} · Guild: {} · Twitch: {}",
                            known_percent(b.trainer_event_pct,b.trainer_event_known),
                            known_percent(b.pokemon_event_pct,b.pokemon_event_known),
                            known_percent(b.guild_pct,b.guild_known),
                            known_percent(b.twitch_pct,b.twitch_known)
                        ));
                        ui.label(format!(
                            "VIP: {} · Guild boost: {}",
                            known_boost(b.vip_active,b.vip_known,hunts.server_now.is_some()),
                            known_boost(b.guild_boost_active,b.guild_boost_known,hunts.server_now.is_some())
                        ));
                        ui.label("Guide projections multiply the reported bonus factors. Same-pace scenarios do NOT assume these bonuses. Store boosts not present in native telemetry are not added.");
                    });
                    ui.label(RichText::new("Gold forecasts are gross expected rewards, not measured net gold. Guide model uses approximate DPS and may significantly overestimate real combat throughput.")
                        .size(10.0).color(MUTED));
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_rate_is_never_turned_into_zero(){
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"a".into(),name:"A".into(),level:100,..Default::default()
        }]);
        let mut cache=EstimateCache::default();
        cache.refresh(0,&view,None,true);
        let rows=ranked_hunts(&view,None,HuntRanking::TrainerXp,true,"","","",&cache);
        assert_eq!(rows.len(),1);
        assert!(rows[0].value.is_none());
        assert_eq!(rows[0].source,RateSource::Unavailable);
    }
    #[test]
    fn current_measured_and_other_scenario_are_separately_labeled(){
        let mut view=HuntView::default();
        view.hunt_slug=Some("a".into());
        view.hunts=Arc::new(vec![
            native_hunts::HuntEntry{slug:"a".into(),name:"A".into(),level:100,..Default::default()},
            native_hunts::HuntEntry{slug:"b".into(),name:"B".into(),level:120,..Default::default()},
        ]);
        view.measurements.seconds=60.0;
        view.measurements.kills=10;
        view.measurements.kills_h=600.0;
        view.measurements.trainer_xp_h=1200.0;
        view.measurements.valid_trainer=true;
        let mut cache=EstimateCache::default();
        cache.refresh(0,&view,None,true);
        let rows=ranked_hunts(&view,None,HuntRanking::TrainerXp,true,"","","",&cache);
        assert_eq!(rows.iter().find(|r|r.hunt.slug=="a").unwrap().source,RateSource::Measured);
        assert_eq!(rows.iter().find(|r|r.hunt.slug=="b").unwrap().source,RateSource::Scenario);
        let stale=ranked_hunts(&view,None,HuntRanking::TrainerXp,false,"","","",&cache);
        assert!(stale.iter().all(|r|r.value.is_none()));
    }
    #[test]
    fn partial_bonus_data_downgrades_xp_forecast_confidence(){
        let mut view=HuntView::default();
        view.bonuses.trainer_event_pct=20.0;
        assert!(!bonuses_complete(&view,HuntRanking::TrainerXp));
        assert!(bonuses_complete(&view,HuntRanking::Gold));
        view.server_now=Some(100);
        view.bonuses.trainer_event_known=true;
        view.bonuses.guild_known=true;
        view.bonuses.guild_boost_known=true;
        view.bonuses.twitch_known=true;
        view.bonuses.vip_known=true;
        assert!(bonuses_complete(&view,HuntRanking::TrainerXp));
        assert!(!bonuses_complete(&view,HuntRanking::PokemonXp));
    }

}
