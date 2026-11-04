use super::*;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum RateSource { Measured, Calibrated, Experimental, Scenario, Unavailable }
impl RateSource {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Measured => tr("Measured"),
            Self::Calibrated => tr("Calibrated estimate"),
            Self::Experimental => tr("Simulated combat"),
            Self::Scenario => tr("Same-pace scenario"),
            Self::Unavailable => tr("Unavailable"),
        }
    }
    pub(super) fn color(self) -> Color32 {
        match self {
            Self::Measured => GOOD,
            Self::Calibrated => WARN,
            Self::Experimental | Self::Scenario => MUTED,
            Self::Unavailable => DIM,
        }
    }
}

pub(super) fn draw_hunt_switch_action(
    ui:&mut egui::Ui,current:bool,pending:bool,live:bool,
)->bool {
    if current {
        ui.add_enabled(false, egui::Button::new(tr("Current"))
            .fill(GOOD.linear_multiply(0.12))
            .stroke(Stroke::new(1.0, GOOD.linear_multiply(0.6))));
        return false;
    }
    if pending {
        ui.add_enabled(false, egui::Button::new(tr("Switching…"))
            .fill(WARN.linear_multiply(0.14))
            .stroke(Stroke::new(1.0, WARN.linear_multiply(0.7))));
        ui.label(RichText::new(tr("Waiting for a safe opening…"))
            .size(10.0).color(WARN));
        return false;
    }
    ui.add_enabled(live, egui::Button::new(tr("Hunt →"))).clicked()
}

pub(super) fn request_hunt_with_feedback(
    native:&NativeSession,slug:&str,error:&mut Option<String>,
    pending:&mut Option<String>,
) {
    match native.send_command(GameCommand::Hunt(slug.to_owned())) {
        Ok(()) => {
            *error=None;
            *pending=Some(slug.to_owned());
        }
        Err(message) => {
            *pending=None;
            *error=Some(message);
        }
    }
}

pub(super) fn rate_label(ranking: HuntRanking) -> &'static str {
    match ranking {
        HuntRanking::TrainerXp | HuntRanking::PokemonXp => tr("XP/h"),
        HuntRanking::Gold => tr("Gross gold/h"),
    }
}

pub(super) fn known_percent(value:f64,known:bool)->String {
    if known {format!("+{value:.1}%")} else {"— (not reported)".into()}
}
pub(super) fn known_boost(active:bool,known:bool,multiplier:f64)->String {
    if !known {tr("Unknown").into()}
    else if active {format!("{} ×{multiplier:.1}",tr("Active"))}
    else {tr("Off").into()}
}
/// The guild-ranking percentage is distinct from the guild's timed XP boost.
/// Do not present a legitimate +0% ranking as though it were the boost's state.
pub(super) fn guild_ranking_bonus(b:&native_hunts::HuntBonuses)->Option<String> {
    (b.guild_known && b.guild_pct>0.0).then(||
        format!("{} {}",tr("Guild rank"),known_percent(b.guild_pct,true)))
}
/// Report recognized status just like Twitch; measured XP is already boosted.
pub(super) fn xp_bonus_summary(b:&native_hunts::HuntBonuses)->String {
    let altar=if b.sacrifice_xp_active {
        format!(" · {} ×1.2",tr("Sacrifice boost"))
    } else {String::new()};
    format!("Twitch {} · {} {} · VIP {}{}",
        known_percent(b.twitch_pct,b.twitch_known),
        tr("Guild boost"),known_boost(b.guild_boost_active,b.guild_boost_known,1.1),
        known_boost(b.vip_active,b.vip_known,1.5),altar)
}
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub(super) enum AtlasAoeCue { NoLead, SingleTarget, Enabled }

pub(super) fn atlas_aoe_cue(
    lead:Option<&native_hunts::PokemonInfo>,aoe_enabled:bool,
)->AtlasAoeCue {
    if lead.is_none(){AtlasAoeCue::NoLead}
    else if aoe_enabled {AtlasAoeCue::Enabled}
    else {AtlasAoeCue::SingleTarget}
}
pub(super) fn atlas_aoe_label(cue:AtlasAoeCue)->(&'static str,Color32){
    match cue {
        AtlasAoeCue::NoLead => (tr("Lead unavailable"),MUTED),
        AtlasAoeCue::SingleTarget => (tr("AoE disabled · single-target"),MUTED),
        AtlasAoeCue::Enabled => (tr("AoE enabled · multiple targets"),GOOD),
    }
}
/// One consistent, visually prominent combat signal across the recommendations,
/// Explorer list/details, current hunt and account comparison panels.
pub(super) fn draw_atlas_combat_cues(
    ui:&mut egui::Ui,
    lead:Option<&native_hunts::PokemonInfo>,
    forecast:Option<&native_atlas::Forecast>,
    aoe_enabled:bool,
){
    let (aoe_label,aoe_color)=atlas_aoe_label(atlas_aoe_cue(lead,aoe_enabled));
    ui.horizontal_wrapped(|ui|{
        ui.label(RichText::new(aoe_label).size(11.0).strong().color(aoe_color));
        if let Some(f)=forecast {
            ui.label(RichText::new(format!(
                "{} {}",format_hunt_amount(f.kills_h),tr("kills/h")
            )).size(11.0).strong().color(TEXT));
            if f.aoe {
                ui.label(RichText::new(format!(
                    "≈{:.1} {}",f.engaged,tr("engaged targets")
                )).size(10.0).color(ACCENT));
            }
            let (risk,risk_color)=if f.one_hit {
                (tr("Lethal burst").to_owned(),BAD)
            }else if f.worst_hit_fraction>=0.5 {
                (format!("{} {:.0}% HP",tr("Hit risk"),f.worst_hit_fraction*100.0),WARN)
            }else{
                (format!("{} {:.0}% HP",tr("Worst hit"),f.worst_hit_fraction*100.0),MUTED)
            };
            ui.label(RichText::new(risk).size(10.0).strong().color(risk_color))
                .on_hover_text(tr("Worst modeled simultaneous hit as a share of your Pokémon's full HP; assumes automatic healing."));
            if f.deaths_h>0.0 {
                ui.label(RichText::new(format!(
                    "{:.1} {}",f.deaths_h,tr("deaths/h")
                )).size(10.0).color(WARN));
            }
        }
    });
}
pub(super) fn draw_atlas_combat_profile(
    ui:&mut egui::Ui,view:&HuntView,cache:&EstimateCache,aoe_enabled:bool,
){
    let current=view.hunt_slug.as_deref().and_then(|slug|cache.get(slug));
    egui::Frame::new().fill(PANEL_ALT)
        .stroke(Stroke::new(1.0,BORDER))
        .corner_radius(8.0).inner_margin(egui::Margin::symmetric(11,7))
        .show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new(tr("COMBAT PROFILE"))
                    .size(10.0).strong().color(DIM));
                if let Some(lead)=view.active_pokemon.as_ref() {
                    ui.label(RichText::new(format!("{} · Lv {}",lead.name,lead.level))
                        .size(11.0).strong().color(TEXT));
                }
                draw_atlas_combat_cues(ui,view.active_pokemon.as_ref(),current.as_ref(),aoe_enabled);
                if let Some(lead)=view.active_pokemon.as_ref() {
                    if let Some(element)=lead.tm_elemental.as_deref() {
                        ui.label(RichText::new(format!("Z-Move {element}"))
                            .size(10.0).color(ACCENT));
                    }
                }
                if let Some(slug)=view.hunt_slug.as_deref() {
                    let measured=view.geo_distances.get(slug)
                        .is_some_and(|samples|!samples.is_empty());
                    ui.label(RichText::new(if measured {
                        tr("● Measured spawn geometry")
                    }else{tr("≈ Default spawn geometry")})
                        .size(10.0).color(if measured {GOOD}else{MUTED}));
                }
                if view.hunt_slug.is_none() {
                    ui.label(RichText::new(tr("Select a hunt for combat projections"))
                        .size(10.0).color(MUTED));
                }else if current.is_none() && cache.is_pending(){
                    ui.label(RichText::new(tr("Computing combat forecasts"))
                        .size(10.0).color(WARN));
                }
            });
        });
}
pub(super) fn draw_rate(ui:&mut egui::Ui,label:&str,value:Option<f64>,source:RateSource){
    ui.vertical(|ui|{
        ui.label(RichText::new(label).size(10.0).color(DIM));
        ui.label(RichText::new(value.map(format_hunt_amount)
            .unwrap_or_else(||"—".into())).size(15.0).strong()
            .color(if value.is_some() {source.color()} else {MUTED}));
        ui.label(RichText::new(source.label()).size(9.0).color(source.color()));
    });
}

/// Shared results pager: compact ranges make larger atlases manageable.
pub(super) fn atlas_pager(ui:&mut egui::Ui,page:&mut usize,count:usize,per_page:usize)
    ->std::ops::Range<usize>{
    let total_pages=count.div_ceil(per_page).max(1);
    *page=(*page).min(total_pages-1);
    ui.horizontal_wrapped(|ui|{
        let first=(*page*per_page).min(count);
        let last=(first+per_page).min(count);
        ui.label(RichText::new(if count==0 {
            tr("No results").to_owned()
        }else{
            tr_dynamic(format!("Showing {}–{} of {}",first+1,last,count))
        }).size(11.0).strong().color(TEXT));
        ui.add_space(8.0);
        if ui.add_enabled(*page>0,egui::Button::new(tr("← Previous")))
            .clicked(){*page-=1;}
        ui.label(RichText::new(format!("{} / {}",*page+1,total_pages))
            .size(10.0).color(MUTED));
        if ui.add_enabled(*page+1<total_pages,
            egui::Button::new(tr("Next →"))).clicked(){*page+=1;}
    });
    let first=*page*per_page;
    first..(first+per_page).min(count)
}

/// The hunt comes from authoritative location state, not the transient
/// wild slot. Combat receipts only refine activity; a missing encounter
/// never means the player left the hunt.
/// Restore the last confirmed destination, not a guide-only or pending hunt.
/// The server-provided catalog is the only authority for executable hunts.
pub(super) fn atlas_return_hunt_slug(view:&HuntView)->Option<&str>{
    let slug=view.hunt_slug.as_deref()?;
    view.hunts.iter().any(|hunt|hunt.slug==slug).then_some(slug)
}
pub(super) fn draw_atlas_navigation_actions(
    ui:&mut egui::Ui,view:&HuntView,native:&NativeSession,
    live:bool,error:&mut Option<String>,
){
    ui.horizontal_wrapped(|ui|{
        if ui.add_enabled(live && !view.in_center && !view.pending_center,
            egui::Button::new(tr("Pokémon Center"))
        ).on_hover_text(tr("Wait for a safe combat opening. If travel stays blocked for one minute, give up once and continue to the Pokémon Center."))
            .clicked(){
            match native.send_command(GameCommand::Center){
                Ok(())=>*error=None,
                Err(message)=>*error=Some(message),
            }
        }
        ui.add_enabled_ui(live && !view.in_center,|ui|{
            ui.menu_button(tr("Pokémon Center (Give Up)"),|ui|{
                ui.label(RichText::new(tr(
                    "Giving up KOs your Pokémon and may cause XP loss. This action cannot be undone."
                )).color(WARN));
                if ui.button(tr("Confirm Give Up and go to Pokémon Center")).clicked(){
                    match native.send_command(GameCommand::GiveUp){
                        Ok(())=>*error=None,
                        Err(message)=>*error=Some(message),
                    }
                    ui.close();
                }
            }).response.on_hover_text(tr(
                "Manual emergency exit only. KOs your Pokémon and may cause XP loss."
            ));
        });
        let return_slug=atlas_return_hunt_slug(view);
        if ui.add_enabled(live && return_slug.is_some(),
            egui::Button::new(tr("Return to hunt"))
        ).on_hover_text(return_slug.unwrap_or(tr("No confirmed hunt to return to")))
            .clicked(){
            if let Some(slug)=return_slug {
                match native.send_command(GameCommand::Hunt(slug.to_owned())){
                    Ok(())=>*error=None,
                    Err(message)=>*error=Some(message),
                }
            }
        }
    });
}
/// Explorer-only pagination toolbar: the Center and Return controls occupy
/// the right half of the pager row, above Pokémon and species details.
pub(super) fn atlas_explorer_pager(
    ui:&mut egui::Ui,page:&mut usize,count:usize,per_page:usize,
    view:&HuntView,native:&NativeSession,live:bool,error:&mut Option<String>,
)->std::ops::Range<usize>{
    if ui.available_width()>=880.0 {
        let mut visible=0..0;
        ui.columns(2,|cols|{
            visible=atlas_pager(&mut cols[0],page,count,per_page);
            cols[1].with_layout(Layout::right_to_left(Align::Center),|ui|{
                draw_atlas_navigation_actions(ui,view,native,live,error);
            });
        });
        visible
    }else{
        let visible=atlas_pager(ui,page,count,per_page);
        ui.with_layout(Layout::right_to_left(Align::Center),|ui|{
            draw_atlas_navigation_actions(ui,view,native,live,error);
        });
        visible
    }
}

pub(super) fn atlas_combat_status(view:&HuntView,live:bool)->(&'static str,Color32){
    match overview_combat_state(Some(view),live) {
        OverviewCombatState::Offline => (tr("Offline / stale"),WARN),
        OverviewCombatState::Center => (tr("At Pokémon Center"),WARN),
        OverviewCombatState::Waiting => (tr("No confirmed hunt"),MUTED),
        OverviewCombatState::Active => (tr("Hunt active"),GOOD),
        OverviewCombatState::Hunting => (tr("Hunting"),GOOD),
        OverviewCombatState::Stalled => (tr("In hunt · no recent activity"),WARN),
    }
}

pub(super) fn draw_atlas_encounter(
    ui:&mut egui::Ui,spawn:&native_hunts::HuntSpecies,
    _sum:f64,reference:Option<&AtlasReference>,
){
    let species=reference.and_then(|r|r.species.get(&spawn.id));
    let name=species.map(|s|s.name.as_str()).unwrap_or(tr("Unknown species"));
    ui.push_id(spawn.id,|ui|{
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new(name).size(11.0).color(TEXT));
            ui.label(RichText::new(format!("#{}",spawn.id))
                .size(10.0).color(MUTED));
        });
    });
}

pub(in crate::app) fn atlas_type_multiplier(
    reference:&AtlasReference,attack:&str,defenders:&[String]
)->Option<f64>{
    if defenders.is_empty(){return None}
    let row=reference.chart.iter()
        .find(|(kind,_)|kind.eq_ignore_ascii_case(attack))?.1;
    let mut multiplier=1.0;
    for defender in defenders {
        if !reference.chart.keys().any(|kind|kind.eq_ignore_ascii_case(defender)){
            return None;
        }
        let step=row.iter().find(|(kind,_)|
            kind.eq_ignore_ascii_case(defender))
            .map(|(_,value)|*value).unwrap_or(1.0);
        if !step.is_finite() || step<0.0{return None}
        multiplier*=step;
    }
    Some(multiplier)
}

/// Neutral does not mean missing: hide 1× results by default so advantages,
/// immunities and weaknesses remain readable above the fold.

pub(in crate::app) fn atlas_type_name(kind:&str)->&str {
    if !pt_br(){return kind}
    match kind.to_ascii_uppercase().as_str(){
        "NORMAL"=>"Normal","FIRE"=>"Fogo","WATER"=>"Água",
        "ELECTRIC"=>"Elétrico","GRASS"=>"Planta","ICE"=>"Gelo",
        "FIGHTING"=>"Lutador","POISON"=>"Venenoso","GROUND"=>"Terra",
        "FLYING"=>"Voador","PSYCHIC"=>"Psíquico","BUG"=>"Inseto",
        "ROCK"=>"Pedra","GHOST"=>"Fantasma","DRAGON"=>"Dragão",
        "DARK"=>"Sombrio","STEEL"=>"Aço","FAIRY"=>"Fada",
        _=>kind,
    }
}
pub(super) fn atlas_multiplier_text(multiplier:f64)->String{
    let decimal=|pt:&'static str,en:&'static str| -> &'static str {
        if pt_br(){pt}else{en}
    };
    if multiplier==0.0{"0×".into()}
    else if (multiplier-0.25).abs()<0.001{decimal("0,25×","0.25×").into()}
    else if (multiplier-0.5).abs()<0.001{decimal("0,5×","0.5×").into()}
    else if (multiplier-1.0).abs()<0.001{"1×".into()}
    else if (multiplier-2.0).abs()<0.001{"2×".into()}
    else if (multiplier-4.0).abs()<0.001{"4×".into()}
    else{format!("{multiplier:.2}×")}
}
pub(super) fn atlas_multiplier_color(multiplier:f64,incoming:bool)->Color32{
    if (multiplier-1.0).abs()<0.001{MUTED}
    else if (multiplier<1.0)==incoming{GOOD}
    else{BAD}
}
pub(super) fn atlas_type_chip(ui:&mut egui::Ui,kind:&str,multiplier:f64,incoming:bool){
    let color=atlas_multiplier_color(multiplier,incoming);
    egui::Frame::new()
        .fill(color.linear_multiply(0.10))
        .stroke(Stroke::new(1.0,color.linear_multiply(0.65)))
        .corner_radius(6.0).inner_margin(egui::Margin::symmetric(7,4))
        .show(ui,|ui|{
            let response=ui.label(RichText::new(format!(
                "{}  {}",atlas_type_name(kind),atlas_multiplier_text(multiplier)
            )).size(10.0).strong().color(color));
            if multiplier==0.0{
                response.on_hover_text(tr("Immune: no damage from this attack type."));
            }
        });
}

/// Filters represent attack types against this species. Each selected type
/// still shows its own multiplier, including neutral and immune results.
pub(super) fn draw_atlas_selected_weaknesses(
    ui:&mut egui::Ui,reference:Option<&AtlasReference>,
    defender_types:&[String],selected:&std::collections::BTreeSet<String>
){
    if selected.is_empty(){return}
    ui.horizontal_wrapped(|ui|{
        ui.label(RichText::new(tr("Damage from selected types"))
            .size(9.0).strong().color(DIM));
        for attack in selected{
            if let Some(multiplier)=reference.and_then(|guide|
                atlas_type_multiplier(guide,attack,defender_types)){
                atlas_type_chip(ui,attack,multiplier,false);
            }else{
                ui.label(RichText::new(format!(
                    "{} · {}",atlas_type_name(attack),tr("Unknown")
                )).size(10.0).color(MUTED));
            }
        }
    });
}

/// Display only non-neutral multipliers for each combatant's own attack
/// types. Do not imply arbitrary moves are available or learned.
pub(super) fn draw_atlas_type_direction(
    ui:&mut egui::Ui,reference:&AtlasReference,
    attacker_types:&[String],defender_types:&[String],incoming:bool,
){
    ui.label(RichText::new(tr(if incoming{"Damage received"}else{"Damage dealt"}))
        .size(11.0).strong().color(TEXT));
    if attacker_types.is_empty() || defender_types.is_empty(){
        ui.label(RichText::new(tr("Type data unavailable")).size(10.0).color(MUTED));
        return;
    }
    let mut known=false;
    let mut shown=false;
    ui.horizontal_wrapped(|ui|{
        for kind in attacker_types{
            if let Some(multiplier)=atlas_type_multiplier(reference,kind,defender_types){
                known=true;
                if (multiplier-1.0).abs()>=0.001{
                    shown=true;
                    atlas_type_chip(ui,kind,multiplier,incoming);
                }
            }
        }
        if !known{
            ui.label(RichText::new(tr("Type data unavailable")).size(10.0).color(MUTED));
        }else if !shown{
            ui.label(RichText::new(tr("No type advantage (1×)")).size(10.0).color(MUTED));
        }
    });
}

pub(in crate::app) fn atlas_types_for_active(
    view:&HuntView,reference:&AtlasReference
)->Vec<String>{
    let Some(pokemon)=view.active_pokemon.as_ref()else{return Vec::new()};
    if !pokemon.types.is_empty(){return pokemon.types.clone()}
    pokemon.species_id.and_then(|id|reference.species.get(&id))
        .map(|species|species.types.clone()).unwrap_or_default()
}

pub(in crate::app) fn draw_atlas_matchup(
    ui:&mut egui::Ui,reference:Option<&AtlasReference>,
    lead_types:&[String],target_types:&[String],
    lead_name:&str,target_name:&str,target_context:&str,
){
    egui::Frame::new().fill(PANEL)
        .stroke(Stroke::new(1.0,BORDER))
        .corner_radius(9.0).inner_margin(egui::Margin::symmetric(12,10))
        .show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new(tr("TYPE MATCHUP"))
                    .size(12.0).strong().color(TEXT))
                    .on_hover_text(tr("Type matchup assumes same-type moves; actual moves are not verified."));
                ui.label(RichText::new(format!(
                    "{lead_name}  ↔  {target_name}"
                )).size(11.0).color(ACCENT));
            });
            ui.label(RichText::new(target_context).size(9.0).color(MUTED));
            ui.add_space(4.0);
            let Some(reference)=reference else{
                ui.label(RichText::new(tr("Type matchup data unavailable"))
                    .size(10.0).color(MUTED));
                return;
            };
            if lead_types.is_empty() || target_types.is_empty(){
                ui.label(RichText::new(tr("Type data unavailable"))
                    .size(10.0).color(MUTED));
                return;
            }
            if ui.available_width()>=540.0{
                ui.columns(2,|cols|{
                    draw_atlas_type_direction(&mut cols[0],reference,
                        lead_types,target_types,false);
                    draw_atlas_type_direction(&mut cols[1],reference,
                        target_types,lead_types,true);
                });
            }else{
                draw_atlas_type_direction(ui,reference,
                    lead_types,target_types,false);
                ui.add_space(5.0);
                draw_atlas_type_direction(ui,reference,
                    target_types,lead_types,true);
            }
        });
}

