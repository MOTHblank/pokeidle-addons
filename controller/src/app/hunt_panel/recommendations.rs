use super::*;
pub(super) struct RankedHunt<'a> {
    pub(super) hunt: &'a native_hunts::HuntEntry,
    pub(super) forecast: Option<native_atlas::Forecast>,
    pub(super) value: Option<f64>,
    pub(super) source: RateSource,
}

/// Preserve uncertainty: an absent forecast, invalid observation and unknown
/// kill rate must not become a plausible-looking zero in the recommendations.
pub(super) fn ranked_hunts<'a>(
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
        (region.is_empty() || h.region == region || h.area == region) &&
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
                // Current-wallet *net* gold includes purchases/restocks,
                // whereas the guide projects *gross* kill/drop rewards.
                // Never mix these different quantities in one sortable column.
                HuntRanking::Gold if m.valid_gross_gold && m.kills>=5 =>
                    Some(m.gold_gross_h),
                HuntRanking::Gold => None,
                _ => None,
            }
        } else { None }.filter(|n| n.is_finite());

        // Do not run an optimistic guide simulation against a disconnected or
        // stale account. That account's combat and bonus state is not current.
        let forecast = if live {cache.get(&hunt.slug)} else {None};
        // Without observed calibration the naive DPS model can imply
        // impossible gold/hour values. Prefer a measured/same-pace rate.
        let forecast_value = forecast.as_ref().and_then(|f|match ranking {
            HuntRanking::TrainerXp => Some(f.trainer_xp_h),
            HuntRanking::PokemonXp => Some(f.pokemon_xp_h),
            HuntRanking::Gold => Some(f.gold_h),
        }).filter(|n|n.is_finite());

        // These are *unboosted* counterfactual rates, not predictions.
        // Applying guessed or additive bonuses would make them look precise.
        let scenario = observed_kills
            .and_then(|kills|native_hunts::atlas_pace_scenario(hunt,kills))
            .map(|(xp,gold)|match ranking {
                HuntRanking::TrainerXp | HuntRanking::PokemonXp => xp,
                HuntRanking::Gold => gold,
            }).filter(|n| n.is_finite());

        // Compare every destination under the same simulated conditions.
        // Current-hunt receipts remain a fallback if catalog details are missing.
        let (value,source) = if let Some(value) = forecast_value {
            (Some(value),RateSource::Experimental)
        } else if let Some(value) = actual {
            (Some(value),RateSource::Measured)
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


#[derive(Clone, Copy)]
pub(super) struct CachedRankingRow {
    pub(super) hunt_index:usize,
    pub(super) value:Option<f64>,
    pub(super) source:RateSource,
}

/// Keep only indices and sortable rates, not copies of the hunt catalog or
/// forecasts. Completed rankings survive egui repaints and live socket polls.
#[derive(Default)]
pub(in crate::app) struct RankingsCache {
    pub(super) rows:Vec<CachedRankingRow>,
    pub(super) catalog:Option<Arc<Vec<native_hunts::HuntEntry>>>,
    pub(super) reference:Option<Arc<AtlasReference>>,
    pub(super) forecast_revision:u64,
    pub(super) ranking:Option<HuntRanking>,
    pub(super) query:String,
    pub(super) region:String,
    pub(super) species_type:String,
    pub(super) live:bool,
    pub(super) needs_fallback:bool,
    pub(super) fallback:Option<(Option<String>,Option<u64>,Option<u64>)>,
    pub(super) initialized:bool,
    #[cfg(test)]
    pub(super) rebuilds:usize,
}
impl RankingsCache {
    pub(in crate::app) fn release(&mut self){
        if self.initialized {*self=Self::default();}
    }
    pub(super) fn fallback_signature(view:&HuntView,ranking:HuntRanking,live:bool)
        ->(Option<String>,Option<u64>,Option<u64>){
        let m=&view.measurements;
        let recent=live && m.last_battle_age_secs.is_none_or(|age|age<=180);
        let pace=(recent && m.seconds>=20.0 && m.kills>=5
            && m.kills_h.is_finite() && m.kills_h>0.0)
            .then_some(m.kills_h.to_bits());
        let actual=(if recent {
            match ranking {
                HuntRanking::TrainerXp if m.valid_trainer=>Some(m.trainer_xp_h),
                HuntRanking::PokemonXp if m.valid_pokemon=>Some(m.pokemon_xp_h),
                HuntRanking::Gold if m.valid_gross_gold && m.kills>=5=>
                    Some(m.gold_gross_h),
                _=>None,
            }
        }else{None}).filter(|n|n.is_finite()).map(f64::to_bits);
        (view.hunt_slug.clone(),pace,actual)
    }
    pub(super) fn update(&mut self,view:&HuntView,reference:Option<&Arc<AtlasReference>>,
        estimates:&EstimateCache,ranking:HuntRanking,live:bool,
        query:&str,region:&str,species_type:&str,
    )->&[CachedRankingRow] {
        // Measurements are irrelevant when every hunt already has a combat
        // forecast. Otherwise, missing forecasts may use measured rates or
        // same-pace scenarios and those inputs must remain live.
        let fallback=self.needs_fallback.then(||
            Self::fallback_signature(view,ranking,live));
        let changed=!self.initialized ||
            self.catalog.as_ref().is_none_or(|old|!Arc::ptr_eq(old,&view.hunts)) ||
            self.reference.as_ref().map(Arc::as_ptr)!=reference.map(Arc::as_ptr) ||
            self.forecast_revision!=estimates.revision ||
            self.ranking!=Some(ranking) || self.live!=live ||
            self.query!=query || self.region!=region ||
            self.species_type!=species_type ||
            self.fallback!=fallback;
        if changed {
            #[cfg(test)]
            {self.rebuilds+=1;}
            let indices=view.hunts.iter().enumerate()
                .map(|(index,hunt)|(hunt.slug.as_str(),index))
                .collect::<std::collections::BTreeMap<_,_>>();
            let ranked=ranked_hunts(view,reference.map(Arc::as_ref),ranking,live,
                query,region,species_type,estimates);
            // Only measured or same-pace rows in the *visible filter set*
            // can be reordered by incoming battle measurements.
            self.needs_fallback=live && ranked.iter().any(|row|
                row.source!=RateSource::Experimental);
            self.rows=ranked.into_iter()
                .filter_map(|row|indices.get(row.hunt.slug.as_str())
                    .map(|&hunt_index|CachedRankingRow{
                        hunt_index,value:row.value,source:row.source,
                    })).collect();
            self.fallback=self.needs_fallback.then(||
                Self::fallback_signature(view,ranking,live));
            self.catalog=Some(Arc::clone(&view.hunts));
            self.reference=reference.cloned();
            self.forecast_revision=estimates.revision;
            self.ranking=Some(ranking);
            self.live=live;
            self.query=query.to_owned();
            self.region=region.to_owned();
            self.species_type=species_type.to_owned();
            self.initialized=true;
        }
        &self.rows
    }
}

pub(super) fn draw_hunt_row(
    ui:&mut egui::Ui,result:&RankedHunt<'_>,rank:usize,
    ranking:HuntRanking,view:&HuntView,reference:Option<&AtlasReference>,
    live:bool,native:&NativeSession,error:&mut Option<String>,
    pending_ui:&mut Option<(String, Instant)>,aoe_enabled:bool,
){
    let hunt=result.hunt;
    let current=view.hunt_slug.as_deref()==Some(hunt.slug.as_str());
    let pending=view.pending_hunt_slug.as_deref()==Some(hunt.slug.as_str())
        || pending_ui.as_ref().is_some_and(|(slug,_)| slug == &hunt.slug);
    egui::Frame::new()
        .fill(if current{ACCENT.linear_multiply(0.12)}
            else if pending{WARN.linear_multiply(0.07)}else{PANEL})
        .stroke(Stroke::new(1.0,if current{ACCENT}
            else if pending{WARN.linear_multiply(0.65)}else{BORDER}))
        .corner_radius(8.0).inner_margin(egui::Margin::symmetric(12,9))
        .show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui|{
                ui.label(RichText::new(format!("{rank:02}"))
                    .size(12.0).strong().color(DIM));
                ui.vertical(|ui|{
                    ui.add(egui::Label::new(RichText::new(&hunt.name)
                        .strong().size(13.0).color(TEXT)).truncate())
                        .on_hover_text(hunt.name.as_str());
                    ui.label(RichText::new(format!(
                        "{} · {} · Lv {}",hunt.area,hunt.region,hunt.level
                    )).size(10.0).color(MUTED));
                });
                ui.with_layout(Layout::right_to_left(Align::Center),|ui|{
                    if draw_hunt_switch_action(ui,current,pending,live) {
                        request_hunt_with_feedback(
                            native,&hunt.slug,error,pending_ui);
                    }
                });
            });
            ui.add_space(6.0);
            draw_atlas_combat_cues(ui,view.active_pokemon.as_ref(),result.forecast.as_ref(),aoe_enabled);
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui|{
                draw_rate(ui,rate_label(ranking),result.value,result.source);
                if let Some(f)=result.forecast.as_ref(){
                    ui.add_space(15.0);
                    ui.vertical(|ui|{
                        ui.label(RichText::new(tr("Combat pace")).size(10.0).color(DIM));
                        ui.label(RichText::new(format!("{:.0} {}",f.kills_h,tr("kills/h")))
                            .size(13.0).strong().color(TEXT));
                        ui.label(RichText::new(format!(
                            "{:.1}s/kill · {:.1}s dead + {:.1}s combat · {:.0}% coverage",
                            f.seconds_per_kill,f.dead_time_seconds_per_kill,
                            f.combat_seconds_per_kill,f.coverage*100.0
                        )).size(9.0).color(MUTED));
                        ui.label(RichText::new(format!(
                            "{}: {:.0}",tr("Incoming damage/kill"),f.incoming_damage_per_kill
                        )).size(9.0).color(MUTED));
                    });
                }
            });
            ui.push_id(&hunt.slug,|ui|{
                ui.collapsing(tr("Pokémon encounters · estimate assumptions"),|ui|{
                    ui.label(RichText::new(format!(
                        "{} spawn slots · {} species",
                        hunt.total_spawns,hunt.species.len()
                    )).size(10.0).color(MUTED));
                    let sum=hunt.species.iter().map(|s|s.weight.max(0.0)).sum::<f64>();
                    for spawn in hunt.species.iter().take(8){
                        draw_atlas_encounter(ui,spawn,sum,reference);
                    }
                    if hunt.species.len()>8{
                        ui.collapsing(format!(
                            "Show {} more Pokémon",
                            hunt.species.len().saturating_sub(8).min(52)
                        ),|ui|{
                            for spawn in hunt.species.iter().skip(8).take(52){
                                draw_atlas_encounter(ui,spawn,sum,reference);
                            }
                        });
                    }
                    if hunt.species.len()>60 {ui.label(tr("Only the first 60 species shown."));}
                    match result.source {
                        RateSource::Measured => {
                            ui.label(tr("Measured from the currently selected hunt, not a prediction."));
                        }
                        RateSource::Calibrated | RateSource::Experimental => {
                            if let Some(f)=result.forecast.as_ref(){
                                ui.label(format!(
                                    "Combat simulation: {:.0}% species coverage · {:.1}s/kill ({:.1}s dead + {:.1}s combat) · {:.0} kills/h · {:.0} incoming damage/kill.",
                                    f.coverage*100.0,f.seconds_per_kill,
                                    f.dead_time_seconds_per_kill,f.combat_seconds_per_kill,
                                    f.kills_h,f.incoming_damage_per_kill
                                ));
                            }
                            ui.label(tr("Combat simulation includes attack cooldowns, spawn timing, AoE and lethal hits. Measured rewards are displayed separately."));
                        }
                        RateSource::Scenario => {
                            ui.label(tr("Uses the currently measured kills/h at another hunt's base level. No bonus, DPS, or travel projection."));
                        }
                        RateSource::Unavailable => {
                            ui.label(tr("Not enough data for a reliable estimate."));
                        }
                    }
                });
            });
        });
}


pub(super) fn draw_atlas_recommendations(
    ui: &mut egui::Ui, view: &HuntView, native: &NativeSession, live: bool,
    workspace: &mut WorkspaceState, reference_handle: Option<&Arc<AtlasReference>>,
    rankings: &mut RankingsCache, estimates: &EstimateCache,
    action_error: &mut Option<String>, action_pending: &mut Option<(String, Instant)>,
    aoe_enabled: bool,
) {
                    let hunts=&view;
                    // The always-visible Hunt Atlas context already presents
                    // the measured current-hunt rates. Rankings start with the
                    // user's decision, not an identical second status card.
                    ui.horizontal_wrapped(|ui|{
                        ui.label(RichText::new(tr("Where should I hunt?"))
                            .size(15.0).strong().color(TEXT));
                        ui.label(RichText::new(format!(
                            "{} {}",hunts.hunts.len(),tr("unlocked hunts")
                        )).size(11.0).color(MUTED));
                    });
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui|{
                        ui.label(RichText::new(tr("Sort recommendations by")).size(12.0).strong());
                        if workspace.hunt_filters.ranking==HuntRanking::PokemonXp {
                            workspace.hunt_filters.ranking=HuntRanking::TrainerXp;
                        }
                        for (ranking,label) in [
                            (HuntRanking::TrainerXp,tr("XP/h")),
                            (HuntRanking::Gold,tr("Gross gold/h")),
                        ]{
                            if ui.selectable_value(&mut workspace.hunt_filters.ranking,ranking,label)
                                .changed(){
                                workspace.hunt_page=0;
                            }
                        }
                    });
                    ui.horizontal_wrapped(|ui|{
                        if ui.add(egui::TextEdit::singleline(&mut workspace.hunt_filters.query)
                            .hint_text(tr("Hunt, area, region or species")).desired_width(220.0)).changed(){
                            workspace.hunt_page=0;
                        }
                        let regions=atlas_locations(hunts,None);
                        egui::ComboBox::from_id_salt("hunt-region")
                            .selected_text(if workspace.hunt_filters.region.is_empty(){
                                tr("All areas").to_owned()
                            }else{workspace.hunt_filters.region.clone()})
                            .show_ui(ui,|ui|{
                                if ui.selectable_value(&mut workspace.hunt_filters.region,
                                    String::new(),tr("All areas")).changed(){workspace.hunt_page=0;}
                                for region in regions{
                                    if ui.selectable_value(&mut workspace.hunt_filters.region,
                                        region.clone(),region.as_str()).changed(){workspace.hunt_page=0;}
                                }
                            });
                        if let Some(reference)=reference_handle.map(Arc::as_ref){
                            let mut types=reference.species.values()
                                .flat_map(|p|p.types.iter().map(String::as_str))
                                .collect::<Vec<_>>();
                            types.sort_unstable();types.dedup();
                            egui::ComboBox::from_id_salt("hunt-species-type")
                                .selected_text(if workspace.hunt_filters.pokemon_type.is_empty(){
                                    tr("All types").to_owned()
                                }else{workspace.hunt_filters.pokemon_type.clone()})
                                .show_ui(ui,|ui|{
                                    if ui.selectable_value(&mut workspace.hunt_filters.pokemon_type,
                                        String::new(),tr("All types")).changed(){workspace.hunt_page=0;}
                                    for kind in types{
                                        if ui.selectable_value(&mut workspace.hunt_filters.pokemon_type,
                                            kind.to_owned(),atlas_type_name(kind)).changed(){workspace.hunt_page=0;}
                                    }
                                });
                        }
                        if ui.small_button(tr("Clear filters")).clicked(){
                            workspace.hunt_filters.query.clear();workspace.hunt_filters.region.clear();
                            workspace.hunt_filters.pokemon_type.clear();workspace.hunt_page=0;
                        }
                    });
                    let reference=reference_handle.map(Arc::as_ref);
                    let ranked=rankings.update(hunts,reference_handle,
                        estimates,workspace.hunt_filters.ranking,live,
                        &workspace.hunt_filters.query,&workspace.hunt_filters.region,
                        &workspace.hunt_filters.pokemon_type);
                    ui.label(RichText::new(format!(
                        "{} {} · {}",
                        ranked.len(),tr("matching hunts"),tr("only current hunt has measured rates")
                    )).size(11.0).color(MUTED));
                    if !live {ui.label(tr("Rankings disabled while account data is stale."));}
                    let visible=atlas_pager(ui,&mut workspace.hunt_page,ranked.len(),16);
                    ui.add_space(7.0);
                    for (offset,result) in ranked.iter().skip(visible.start)
                        .take(visible.end-visible.start).enumerate(){
                        let hunt=&hunts.hunts[result.hunt_index];
                        let row=RankedHunt{
                            hunt,forecast:if live {estimates.get(&hunt.slug)} else {None},
                            value:result.value,source:result.source,
                        };
                        draw_hunt_row(ui,&row,visible.start+offset+1,workspace.hunt_filters.ranking,
                            hunts,reference,live,native,action_error,
                            action_pending,aoe_enabled);
                        ui.add_space(5.0);
                    }
                    if ranked.is_empty(){
                        egui::Frame::new().fill(PANEL).corner_radius(9.0)
                            .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
                                ui.set_min_width(ui.available_width());
                                ui.label(RichText::new(tr("No matching hunts"))
                                    .size(13.0).strong().color(TEXT));
                                ui.label(RichText::new(
                                    tr("Widen the search or clear the region and type filters.")
                                ).size(11.0).color(MUTED));
                                if ui.small_button(tr("Clear ranking filters")).clicked(){
                                    workspace.hunt_filters.query.clear();
                                    workspace.hunt_filters.region.clear();
                                    workspace.hunt_filters.pokemon_type.clear();
                                    workspace.hunt_page=0;
                                }
                            });
                    }
                    ui.add_space(7.0);
                    ui.collapsing(tr("XP bonuses reported by game"),|ui|{
                        let b=&hunts.bonuses;
                        ui.label(xp_bonus_summary(b));
                        if let Some(ranking)=guild_ranking_bonus(b) {
                            ui.label(ranking);
                        }
                        ui.label(format!(
                            "{}: {} · {}: {}",
                            tr("Trainer event"),known_percent(b.trainer_event_pct,b.trainer_event_known),
                            tr("Pokémon event"),known_percent(b.pokemon_event_pct,b.pokemon_event_known)
                        ));
                        ui.label(tr("Measured XP/h already includes server-awarded bonuses. Simulated forecasts apply known bonuses once; same-speed comparisons do not add bonuses."));
                    });
                    ui.label(RichText::new(tr("Combat gold/h from confirmed receipts is separate from net wallet changes. Other-hunt gold/h uses comparable full-wave combat simulations; unknown loot boosts are excluded.")));
                    ui.label(RichText::new(tr("Experimental combat forecasts cannot apply unreported gym-podium or daily seasonal-type damage bonuses. Current-hunt measured rates already include their real effects."))
                        .size(10.0).color(MUTED));
}
