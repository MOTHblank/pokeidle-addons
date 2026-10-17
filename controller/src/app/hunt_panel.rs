//! Native Hunt Atlas workspace. All game state is read from the authenticated
//! native socket. The public guide provides optional, untrusted reference data.
use super::*;
use crate::native_atlas_models::{self as models, BallKind, Modifiers};
mod models_panel;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AtlasAvailability { All, Unlocked, Locked }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AtlasCollection { All, InDepot, NotInDepot }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AtlasSort {
    Xp, Gold, Capture, ShinyCaught, OutlandRare, SaleValue, Shiny, Level, Name,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct AtlasFilters {
    search: String,
    region: String,
    kind: String,
    min_level: u32,
    max_level: u32,
    availability: AtlasAvailability,
    collection: AtlasCollection,
    weak_to: std::collections::BTreeSet<String>,
    weakness_match_all: bool,
    sort: AtlasSort,
    ascending: bool,
    /// Per-account Explorer selection; stable across repaint and socket snapshots.
    selected_species: Option<u64>,
}
impl Default for AtlasFilters {
    fn default()->Self {
        Self{
            search:String::new(),region:String::new(),kind:String::new(),
            min_level:0,max_level:0,availability:AtlasAvailability::All,
            collection:AtlasCollection::All,
            weak_to:std::collections::BTreeSet::new(),weakness_match_all:false,
            sort:AtlasSort::Xp,ascending:false,selected_species:None,
        }
    }
}
impl AtlasFilters {
    fn clear(&mut self) {
        let sort=self.sort;
        let ascending=self.ascending;
        *self=Self::default();
        self.sort=sort;self.ascending=ascending;
    }
    fn active_count(&self)->usize {
        usize::from(!self.search.trim().is_empty())+
        usize::from(!self.region.is_empty())+
        usize::from(!self.kind.is_empty())+
        usize::from(self.min_level>0)+usize::from(self.max_level>0)+
        usize::from(self.availability!=AtlasAvailability::All)+
        usize::from(self.collection!=AtlasCollection::All)+
        usize::from(!self.weak_to.is_empty())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RateSource { Measured, Calibrated, Experimental, Scenario, Unavailable }
impl RateSource {
    fn label(self) -> &'static str {
        match self {
            Self::Measured => tr("Measured"),
            Self::Calibrated => tr("Calibrated estimate"),
            Self::Experimental => tr("Early estimate"),
            Self::Scenario => tr("Same-pace scenario"),
            Self::Unavailable => tr("Unavailable"),
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

/// Account-keyed combat forecasts are calculated away from egui's paint
/// path. Immutable snapshots and Arc guide data are cheap to hand to the
/// worker; no blocking game/network I/O or combat loop executes in render.
#[derive(Default)]
pub(super) struct EstimateCache {
    account: Option<usize>,
    catalog_ptr: usize,
    reference_ptr: usize,
    active_signature: Option<(u64,u32,Option<u64>,Option<u64>,[Option<u64>;6],bool)>,
    hunt_slug: Option<String>,
    live: bool,
    refreshed: Option<Instant>,
    last_archived:Option<(String,Instant)>,
    pending: Option<std::sync::mpsc::Receiver<
        std::collections::BTreeMap<String,Option<native_atlas::Forecast>>>>,
    values: std::collections::BTreeMap<String,Option<native_atlas::Forecast>>,
}
impl EstimateCache {
    fn refresh(&mut self, account:usize,view:&HuntView,
        reference:Option<&Arc<AtlasReference>>,live:bool,
    ){
        use std::sync::mpsc::TryRecvError;
        // Publish only for this cache generation. Account/lead/catalog changes
        // drop the receiver for the previous generation before accepting it.
        let complete=self.pending.as_ref().map(|rx|rx.try_recv());
        match complete {
            Some(Ok(values))=>{
                self.values=values;
                self.refreshed=Some(Instant::now());
                self.pending=None;
            }
            Some(Err(TryRecvError::Disconnected))=>{
                self.pending=None;
                self.refreshed=Some(Instant::now()); // Backoff on worker panic.
            }
            _=>{}
        }
        let catalog_ptr=Arc::as_ptr(&view.hunts) as usize;
        let reference_ptr=reference.map_or(0,|r|Arc::as_ptr(r) as usize);
        let active_signature=view.active_pokemon.as_ref().map(|p|
            (p.id,p.level,p.quality.map(f64::to_bits),p.potency,p.ivs,p.shiny));
        let changed=self.account!=Some(account) || self.catalog_ptr!=catalog_ptr
            || self.reference_ptr!=reference_ptr || self.live!=live
            || self.active_signature!=active_signature || self.hunt_slug!=view.hunt_slug;
        if changed {
            self.account=Some(account);
            self.catalog_ptr=catalog_ptr;
            self.reference_ptr=reference_ptr;
            self.active_signature=active_signature;
            self.live=live;
            self.hunt_slug=view.hunt_slug.clone();
            self.refreshed=None;
            self.last_archived=None;
            self.pending=None; // Discard outdated asynchronous results.
            self.values.clear();
        }
        if !live || self.pending.is_some() ||
            self.refreshed.is_some_and(|at|at.elapsed()<Duration::from_secs(20)) {
            return;
        }
        let Some(reference)=reference else {return};
        let guide=Arc::clone(reference);
        let snapshot=view.clone();
        let (tx,rx)=std::sync::mpsc::sync_channel(1);
        self.pending=Some(rx);
        std::thread::spawn(move ||{
            let mut values=std::collections::BTreeMap::new();
            for hunt in snapshot.hunts.iter(){
                values.insert(hunt.slug.clone(),
                    native_atlas::estimate(&snapshot,hunt,&guide));
            }
            for hunt in guide.hunts.iter(){
                if !values.contains_key(&hunt.slug) {
                    values.insert(hunt.slug.clone(),
                        native_atlas::estimate(&snapshot,hunt,&guide));
                }
            }
            let _=tx.send(values);
        });
    }
    fn get(&self,slug:&str)->Option<native_atlas::Forecast>{
        self.values.get(slug).cloned().flatten()
    }
    fn is_pending(&self)->bool{self.pending.is_some()}
    fn archive_due(&self,slug:&str)->bool{
        self.last_archived.as_ref().is_none_or(|(previous,at)|
            previous!=slug || at.elapsed()>=Duration::from_secs(900))
    }
    fn archived(&mut self,slug:&str){
        self.last_archived=Some((slug.to_owned(),Instant::now()));
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
                // Current-wallet *net* gold includes purchases/restocks,
                // whereas the guide projects *gross* kill/drop rewards.
                // Never mix these different quantities in one sortable column.
                HuntRanking::Gold => None,
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
        HuntRanking::TrainerXp => tr("Trainer XP/h"),
        HuntRanking::PokemonXp => tr("Pokémon XP/h"),
        HuntRanking::Gold => tr("Gold/h"),
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

/// Shared results pager: compact ranges make larger atlases manageable.
fn atlas_pager(ui:&mut egui::Ui,page:&mut usize,count:usize,per_page:usize)
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
fn atlas_combat_status(view:&HuntView,live:bool)->(&'static str,Color32){
    match overview_combat_state(Some(view),live) {
        OverviewCombatState::Offline => (tr("Offline / stale"),WARN),
        OverviewCombatState::Center => (tr("At Pokémon Center"),WARN),
        OverviewCombatState::Waiting => (tr("No confirmed hunt"),MUTED),
        OverviewCombatState::Active => (tr("Hunt active"),GOOD),
        OverviewCombatState::Hunting => (tr("Hunting"),GOOD),
        OverviewCombatState::Stalled => (tr("In hunt · no recent activity"),WARN),
    }
}

fn draw_atlas_encounter(
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

fn draw_hunt_row(
    ui:&mut egui::Ui,result:&RankedHunt<'_>,rank:usize,
    ranking:HuntRanking,view:&HuntView,reference:Option<&AtlasReference>,
    live:bool,native:&NativeSession,error:&mut Option<String>,
){
    let hunt=result.hunt;
    let current=view.hunt_slug.as_deref()==Some(hunt.slug.as_str());
    let pending=view.pending_hunt_slug.as_deref()==Some(hunt.slug.as_str());
    egui::Frame::new()
        .fill(if current{ACCENT.linear_multiply(0.12)}else{PANEL})
        .stroke(Stroke::new(1.0,if current{ACCENT}else{BORDER}))
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
                    if current{
                        ui.colored_label(GOOD,tr("● Current"));
                    }else if pending{
                        ui.colored_label(WARN,tr("Switching…"));
                    }else if ui.add_enabled(live,
                        egui::Button::new(tr("Hunt →"))).clicked(){
                        match native.send_command(GameCommand::Hunt(hunt.slug.clone())){
                            Ok(())=>*error=None,
                            Err(e)=>*error=Some(e),
                        }
                    }
                });
            });
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
                            "{:.1}s/kill · {:.0}% coverage",
                            f.seconds_per_kill,f.coverage*100.0
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
                                    "Guide model: {:.0}% modeled species coverage · {:.1}s/kill · {:.0} kills/h.",
                                    f.coverage*100.0,f.seconds_per_kill,f.kills_h
                                ));
                            }
                            ui.label(tr("Approximated DPS and spawn weights. Real move timing, captures, time outside combat, and item costs may differ."));
                        }
                        RateSource::Scenario => {
                            ui.label(tr("Uses the currently measured kills/h at another hunt's base level. No bonus, DPS, or travel projection."));
                        }
                        RateSource::Unavailable => {
                            ui.label(tr("No valid measured pace or sufficiently complete guide model is available."));
                        }
                    }
                });
            });
        });
}

// Explorer needs only these three fields to display/select a destination.
// Keeping an entire HuntEntry (including its species allocation) alive in
// each account's cached results multiplied RAM after the ranking-cache change.
#[derive(Clone)]
struct AtlasHuntLabel {
    slug:String,
    name:String,
    level:u32,
}
impl AtlasHuntLabel {
    fn from_hunt(hunt:&native_hunts::HuntEntry)->Self {
        Self {slug:hunt.slug.clone(),name:hunt.name.clone(),level:hunt.level}
    }
}
#[derive(Clone)]
struct AtlasDestination {
    hunt:Arc<AtlasHuntLabel>,
    unlocked:bool,
    share:f64,
    forecast:Option<native_atlas::Forecast>,
    objective:Option<f64>,
}
struct AtlasSpeciesRow {
    id:u64,
    name:String,
    types:Vec<String>,
    owned:bool,
    destinations:Vec<AtlasDestination>,
    best:Option<f64>,
}

fn atlas_destinations<'a>(
    view:&'a HuntView, reference:Option<&'a AtlasReference>
) -> Vec<(&'a native_hunts::HuntEntry,bool)> {
    // Authentication, not trainer level, is authoritative for actual unlocks.
    let mut seen=std::collections::BTreeSet::new();
    let mut result=Vec::new();
    for hunt in view.hunts.iter() {
        if seen.insert(hunt.slug.as_str()) {
            result.push((hunt,true));
        }
    }
    if let Some(reference)=reference {
        for hunt in &reference.hunts {
            if seen.insert(hunt.slug.as_str()) {
                result.push((hunt,false)); // public-guide preview only
            }
        }
    }
    result
}

/// Return the combined game type-chart multiplier for an attack type against
/// a single- or dual-type defender. The guide omits some neutral 1× entries;
/// absent *entries* are neutral, but absent type definitions are unknown.
fn atlas_type_multiplier(
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

fn atlas_type_name(kind:&str)->&str {
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
fn atlas_multiplier_text(multiplier:f64)->String{
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
fn atlas_multiplier_color(multiplier:f64,incoming:bool)->Color32{
    if (multiplier-1.0).abs()<0.001{MUTED}
    else if (multiplier<1.0)==incoming{GOOD}
    else{BAD}
}
fn atlas_type_chip(ui:&mut egui::Ui,kind:&str,multiplier:f64,incoming:bool){
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
fn draw_atlas_selected_weaknesses(
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
fn draw_atlas_type_direction(
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

fn atlas_types_for_active(
    view:&HuntView,reference:&AtlasReference
)->Vec<String>{
    let Some(pokemon)=view.active_pokemon.as_ref()else{return Vec::new()};
    if !pokemon.types.is_empty(){return pokemon.types.clone()}
    pokemon.species_id.and_then(|id|reference.species.get(&id))
        .map(|species|species.types.clone()).unwrap_or_default()
}

fn draw_atlas_matchup(
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
                ui.label(RichText::new(tr("Waiting for the game guide's type chart."))
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

fn atlas_objective(
    sort:AtlasSort, forecast:Option<&native_atlas::Forecast>,
    hunt:&native_hunts::HuntEntry, share:f64, sale_value:Option<f64>
)->Option<f64>{
    match sort {
        AtlasSort::Name=>None,
        AtlasSort::Level=>Some(hunt.level as f64),
        // Exactly one XP stream: never add trainer and Pokémon experience.
        AtlasSort::Xp=>forecast.map(|f|f.trainer_xp_h),
        AtlasSort::Gold=>forecast.map(|f|f.combat_gold_h),
        AtlasSort::SaleValue=>sale_value,
        AtlasSort::Capture|AtlasSort::ShinyCaught|AtlasSort::OutlandRare=>None,
        AtlasSort::Shiny=>forecast.and_then(|f|{
            if hunt.area.eq_ignore_ascii_case("outland"){None}
            else{Some(f.kills_h*share/24000.0)}
        }),
    }.filter(|v|v.is_finite() && *v>=0.0)
}
fn atlas_sort_label(sort:AtlasSort)->&'static str {
    match sort {
        AtlasSort::Xp=>tr("XP/h"),
        AtlasSort::Gold=>tr("Gross gold/h"),
        AtlasSort::Capture=>tr("Capture chance"),
        AtlasSort::ShinyCaught=>tr("Conditional shiny caught/h"),
        AtlasSort::OutlandRare=>tr("Outland rare units/h"),
        AtlasSort::SaleValue=>tr("Catalog sale value"),
        AtlasSort::Shiny=>tr("Shiny encounter/h (conditional)"),
        AtlasSort::Level=>tr("Hunt level"),
        AtlasSort::Name=>tr("Species name"),
    }
}
fn atlas_value_label(sort:AtlasSort,value:Option<f64>)->String {
    match (sort,value) {
        (_,None)=>"—".into(),
        (AtlasSort::Capture,Some(n))=>format!("{:.2}%",n*100.0),
        (AtlasSort::Shiny|AtlasSort::ShinyCaught|AtlasSort::OutlandRare,Some(n))=>format!("{n:.4}"),
        (_,Some(n))=>format_hunt_amount(n),
    }
}


/// Species are grouped across destinations, matching the game guide rather
/// than repeating one card per hunt. A guide-only hunt is NEVER executable.
/// Cache completed Explorer rankings across redraws. Live forecasts and
/// reference/catalog changes invalidate it; UI selection and disclosure do not.
#[derive(Default)]
pub(super) struct ExplorerCache {
    rows:Vec<AtlasSpeciesRow>,
    filters:Option<AtlasFilters>,
    scenario:Option<AtlasScenario>,
    catalog:Option<Arc<Vec<native_hunts::HuntEntry>>>,
    reference_ptr:usize,
    forecast_at:Option<Instant>,
    model_signature:Option<(u64,u32,Option<u64>,Option<u64>,[Option<u64>;6],bool)>,
    hunt_slug:Option<String>,
    trainer_level:Option<u32>,
    roster:Option<Arc<Vec<native_hunts::PokemonInfo>>>,
    roster_ready:bool,
    ball_quotes:Option<Arc<std::collections::BTreeMap<u64,native_hunts::BallQuote>>>,
    initialized:bool,
    #[cfg(test)]
    rebuilds:usize,
}
impl ExplorerCache {
    pub(super) fn release(&mut self) {
        // Release cached forecasts, destination labels and Arc source handles
        // when Explorer is not the active workspace. Only one account needs
        // rankings resident at a time.
        if self.initialized { *self=Self::default(); }
    }
    fn update(&mut self,view:&HuntView,reference:Option<&AtlasReference>,
        estimates:&EstimateCache,filters:&AtlasFilters,scenario:&AtlasScenario,
    )->&[AtlasSpeciesRow] {
        let mut key_filters=filters.clone();
        key_filters.selected_species=None;
        // Retaining the previous source Arc prevents pointer reuse from
        // hiding a genuine server roster/catalog update.
        let reference_ptr=reference.map_or(0,|r|r as *const _ as usize);
        let changed=!self.initialized ||
            self.filters.as_ref()!=Some(&key_filters) ||
            self.scenario.as_ref()!=Some(scenario) ||
            self.catalog.as_ref().is_none_or(|old|!Arc::ptr_eq(old,&view.hunts)) ||
            self.reference_ptr!=reference_ptr ||
            self.forecast_at!=estimates.refreshed ||
            self.model_signature!=estimates.active_signature ||
            self.hunt_slug!=view.hunt_slug ||
            self.trainer_level!=view.trainer_level ||
            self.roster.as_ref().is_none_or(|old|!Arc::ptr_eq(old,&view.pokemon_roster)) ||
            self.roster_ready!=view.roster_ready ||
            self.ball_quotes.as_ref().is_none_or(|old|!Arc::ptr_eq(old,&view.ball_quotes));
        if changed {
            #[cfg(test)]
            {self.rebuilds+=1;}
            self.rows=atlas_species_rows(view,reference,estimates,filters,scenario);
            self.filters=Some(key_filters);
            self.scenario=Some(scenario.clone());
            self.catalog=Some(Arc::clone(&view.hunts));
            self.reference_ptr=reference_ptr;
            self.forecast_at=estimates.refreshed;
            self.model_signature=estimates.active_signature;
            self.hunt_slug=view.hunt_slug.clone();
            self.trainer_level=view.trainer_level;
            self.roster=Some(Arc::clone(&view.pokemon_roster));
            self.roster_ready=view.roster_ready;
            self.ball_quotes=Some(Arc::clone(&view.ball_quotes));
            self.initialized=true;
        }
        &self.rows
    }
}

fn atlas_species_rows(
    view:&HuntView, reference:Option<&AtlasReference>, cache:&EstimateCache,
    filters:&AtlasFilters,scenario:&AtlasScenario,
)->Vec<AtlasSpeciesRow> {
    // The absence of a complete roster is not proof that a species is missing.
    if filters.collection!=AtlasCollection::All && !view.roster_ready {
        return Vec::new();
    }
    let query=filters.search.trim().to_lowercase();
    let known_roster:std::collections::BTreeSet<_>=view.pokemon_roster.iter()
        .filter_map(|mon|mon.species_id).collect();
    let mut grouped=std::collections::BTreeMap::<u64,AtlasSpeciesRow>::new();
    for (hunt,unlocked) in atlas_destinations(view,reference) {
        if filters.availability==AtlasAvailability::Unlocked && !unlocked
            || filters.availability==AtlasAvailability::Locked && unlocked
            || filters.min_level>0 && hunt.level<filters.min_level
            || filters.max_level>0 && hunt.level>filters.max_level
            || !filters.region.is_empty() &&
               hunt.area!=filters.region && hunt.region!=filters.region
        {continue}
        let sum=hunt.species.iter().map(|s|s.weight.max(0.0)).sum::<f64>();
        if sum<=0.0 {continue}
        // Only allocate a compact destination label if at least one species
        // actually passes the current filters.
        let mut destination_label:Option<Arc<AtlasHuntLabel>>=None;
        for spawn in &hunt.species {
            if spawn.weight<=0.0 {continue}
            let meta=reference.and_then(|r|r.species.get(&spawn.id));
            let name=meta.map(|m|m.name.as_str()).unwrap_or(tr("Unknown species"));
            let types=meta.map(|m|m.types.clone()).unwrap_or_default();
            if !filters.kind.is_empty() && !types.iter().any(|t|t==&filters.kind){continue}
            if !query.is_empty()
                && !name.to_lowercase().contains(&query)
                && !spawn.id.to_string().contains(&query)
                && !hunt.name.to_lowercase().contains(&query)
                && !hunt.area.to_lowercase().contains(&query)
                && !hunt.slug.to_lowercase().contains(&query)
            {continue}
            if !filters.weak_to.is_empty() {
                let Some(reference)=reference else{continue};
                let is_weak=|attack:&String|
                    atlas_type_multiplier(reference,attack,&types).is_some_and(|n|n>1.0);
                let matches=if filters.weakness_match_all {
                    filters.weak_to.iter().all(is_weak)
                }else{
                    filters.weak_to.iter().any(is_weak)
                };
                if !matches {continue}
            }
            let owned=known_roster.contains(&spawn.id);
            if filters.collection==AtlasCollection::InDepot && !owned ||
               filters.collection==AtlasCollection::NotInDepot && owned {continue}
            let share=spawn.weight.max(0.0)/sum;
            let forecast=cache.get(&hunt.slug);
            let base=atlas_objective(filters.sort,forecast.as_ref(),hunt,
                share,reference.and_then(|r|r.catalog_sell_values.get(&spawn.id)).copied());

            let objective=match filters.sort {
                AtlasSort::Capture=>reference.and_then(|r|
                    r.capture_rarities.get(&spawn.id).copied())
                    .and_then(|rarity|{
                        let efficiency=view.ball_quotes.get(&scenario.ball.id())
                            .and_then(|q|q.efficiency)
                            .unwrap_or_else(||scenario.ball.wiki_efficiency());
                        models::wiki_capture_chance(rarity,efficiency,0.0)
                    })
                    .map(|chance|(chance*scenario.modifiers.catch_factor()).min(1.0)),
                AtlasSort::ShinyCaught=>forecast.as_ref()
                    .and_then(|f|reference.and_then(|r|
                        r.capture_rarities.get(&spawn.id).copied().and_then(|rarity|{
                            let efficiency=view.ball_quotes.get(&scenario.ball.id())
                                .and_then(|q|q.efficiency)
                                .unwrap_or_else(||scenario.ball.wiki_efficiency());
                            models::capture_scenario(Some(rarity),efficiency,f.kills_h,share,
                                scenario.accepted_fraction_override,
                                hunt.area.eq_ignore_ascii_case("outland"),
                                &scenario.modifiers)
                        })))
                    .and_then(|estimate|estimate.shiny_caught_h),
                AtlasSort::OutlandRare=>forecast.as_ref().and_then(|f|
                    models::rare_scenarios(
                        hunt.area.eq_ignore_ascii_case("outland"),hunt.level,
                        scenario.tier,view.trainer_level,f.kills_h,&scenario.modifiers)
                        .into_iter().find(|r|r.id==scenario.rare_target)
                        .map(|r|r.units_h)),
                _=>base
            };


            let label=destination_label.get_or_insert_with(||
                Arc::new(AtlasHuntLabel::from_hunt(hunt)));
            grouped.entry(spawn.id).or_insert_with(||AtlasSpeciesRow{
                id:spawn.id,name:name.to_owned(),types,owned,
                destinations:Vec::new(),best:None,
            }).destinations.push(AtlasDestination{
                hunt:Arc::clone(label),unlocked,share,forecast,objective,
            });
        }
    }
    let mut rows:Vec<_>=grouped.into_values().collect();
    for row in &mut rows {
        row.destinations.sort_by(|a,b|{
            match (a.objective,b.objective) {
                (Some(x),Some(y))=>y.total_cmp(&x),
                (Some(_),None)=>std::cmp::Ordering::Less,
                (None,Some(_))=>std::cmp::Ordering::Greater,
                _=>std::cmp::Ordering::Equal,
            }.then_with(||b.unlocked.cmp(&a.unlocked))
             .then_with(||a.hunt.level.cmp(&b.hunt.level))
             .then_with(||a.hunt.slug.cmp(&b.hunt.slug))
        });
        row.best=row.destinations.iter().filter_map(|d|d.objective)
            .max_by(|a,b|a.total_cmp(b));
    }
    rows.sort_by(|a,b|{
        let direction=if filters.ascending {1}else{-1};
        let ordered=if filters.sort==AtlasSort::Name {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else {
            match (a.best,b.best) {
                (Some(x),Some(y))=>x.total_cmp(&y),
                (Some(_),None)=>return std::cmp::Ordering::Less,
                (None,Some(_))=>return std::cmp::Ordering::Greater,
                _=>std::cmp::Ordering::Equal,
            }
        };
        (if direction==1 {ordered}else{ordered.reverse()})
            .then_with(||a.name.cmp(&b.name)).then_with(||a.id.cmp(&b.id))
    });
    rows
}


fn account_is_live(session:&NativeSession)->bool {
    let status=session.snapshot();
    status.phase=="Online" && status.last_received
        .is_some_and(|at|at.elapsed()<=Duration::from_secs(90))
}

/// Shared workspace navigation chooses the account. This control only
/// toggles Hunt Atlas's account-comparison mode; no duplicate account strip.
/// One navigation strip, including comparison for offline sessions.
fn draw_atlas_view_controls(app:&mut ControllerApp,ui:&mut egui::Ui){
    ui.horizontal_wrapped(|ui|{
        for (tab,label,hint) in [
            (HuntSubtab::Explorer,tr("Explorer"),tr("Find Pokémon and their destinations")),
            (HuntSubtab::Recommendations,tr("Rankings"),tr("Compare unlocked hunt estimates")),
            (HuntSubtab::Current,tr("Current hunt"),tr("Selected hunt, activity and measured gains")),
            (HuntSubtab::Models,tr("Models"),tr("Capture, Outland and economy scenarios")),
            (HuntSubtab::Session,"Session",tr("Recorded battle rewards and history")),
            (HuntSubtab::Compare,tr("Compare"),tr("Compare all four accounts")),
        ]{
            if workspace_tab_button(ui,label,app.hunt_subtab==tab,hint).clicked(){
                app.hunt_subtab=tab;
            }
        }
    });
    ui.add_space(7.0);
}

/// Keep the current hunt visible while looking at other Atlas workspaces.
fn draw_atlas_context(ui:&mut egui::Ui,view:&HuntView,live:bool)->bool{
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
fn draw_atlas_outland_tier(
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
                            let threshold=
                                crate::native_atlas_models::TIER_LEVELS[(tier-1) as usize];
                            let multiplier=
                                crate::native_atlas_models::TIER_MULTIPLIERS[(tier-1) as usize];
                            let eligible=
                                crate::native_atlas_models::unlocked_tier(tier,level).is_some();
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

fn draw_atlas_compare(app:&mut ControllerApp,ui:&mut egui::Ui){
    ui.label(RichText::new(tr("Account comparison"))
        .size(16.0).strong().color(TEXT));
    ui.label(RichText::new(
        tr("Current measured results and best modeled unlocked hunts, side by side. Unknown and stale values remain unavailable.")
    ).size(11.0).color(MUTED));
    let mut views=Vec::new();
    for index in 0..app.games.len(){
        let profile=app.games[index].profile;
        let name=app.account_name(profile);
        if let Some(native)=app.games[index].native.as_ref(){
            let view=native.hunt_snapshot();
            let live=account_is_live(native);
            app.hunt_estimate_cache[index].refresh(index,&view,app.hunt_reference.as_ref(),live);
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
                    .stroke(Stroke::new(1.0,if *index==app.hunt_account{
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
                                app.hunt_subtab=HuntSubtab::Explorer;
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
                            app.hunt_estimate_cache[*index].get(&h.slug)
                                .map(|f|(h,f.trainer_xp_h,f.calibrated))
                        }).collect::<Vec<_>>();
                        estimates.sort_by(|a,b|b.1.total_cmp(&a.1));
                        for (rank,(hunt,score,calibrated)) in estimates.iter().take(3).enumerate(){
                            ui.horizontal_wrapped(|ui|{
                                ui.label(RichText::new(format!(
                                    "{}. {} · {} {}",
                                    rank+1,hunt.name,format_hunt_amount(*score),tr("Trainer XP/h")
                                )).size(11.0).color(TEXT));
                                ui.label(RichText::new(if *calibrated{tr("calibrated")}
                                    else{tr("experimental")}).size(9.0).color(WARN));
                            });
                        }
                        if estimates.is_empty(){
                            ui.label(RichText::new(if *live &&
                                app.hunt_estimate_cache[*index].is_pending(){
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



fn atlas_filters_panel(
    ui:&mut egui::Ui, f:&mut AtlasFilters, page:&mut usize,
    view:&HuntView, reference:Option<&AtlasReference>, account:usize,
){
    egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0,BORDER))
        .corner_radius(9.0).inner_margin(egui::Margin::same(10))
        .show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new(tr("Filters & sorting"))
                    .size(12.0).strong().color(TEXT));
                if f.active_count()>0 && ui.small_button(tr("Reset filters")).clicked(){
                    f.clear();*page=0;
                }
            });
            // All filters remain available without an extra disclosure click.
            egui::ScrollArea::vertical()
                .id_salt(("atlas-filter-card",account))
                .max_height(295.0)
                .auto_shrink([false,true]).show(ui,|ui|{
                ui.set_min_width(ui.available_width());
                ui.horizontal_wrapped(|ui|{
                    let width=(ui.available_width()*0.37).clamp(150.0,320.0);
                    if ui.add(egui::TextEdit::singleline(&mut f.search)
                        .hint_text(tr("Search Pokémon, hunt, area or #ID"))
                        .desired_width(width)).changed(){*page=0;}
                    ui.label(tr("Rank"));
                    egui::ComboBox::from_id_salt(("atlas-quick-sort",account))
                        .selected_text(atlas_sort_label(f.sort)).show_ui(ui,|ui|{
                            for item in [
                                AtlasSort::Xp,AtlasSort::Gold,AtlasSort::Capture,
                                AtlasSort::SaleValue,AtlasSort::ShinyCaught,
                                AtlasSort::OutlandRare,AtlasSort::Shiny,
                                AtlasSort::Level,AtlasSort::Name
                            ]{
                                if ui.selectable_value(&mut f.sort,item,atlas_sort_label(item)).changed(){
                                    f.ascending=matches!(item,AtlasSort::Name|AtlasSort::Level);
                                    *page=0;
                                }
                            }
                        });
                    if ui.small_button(if f.ascending{"↑"}else{"↓"})
                        .on_hover_text(if f.ascending{tr("Ascending sort")}
                            else{tr("Descending sort")}).clicked(){
                        f.ascending=!f.ascending;*page=0;
                    }
                });
                ui.horizontal_wrapped(|ui|{
                    ui.label(tr("Level"));
                    if ui.add(egui::DragValue::new(&mut f.min_level)
                        .range(0..=100_000).speed(1.0)
                        .prefix(tr("From "))).changed(){*page=0;}
                    if ui.add(egui::DragValue::new(&mut f.max_level)
                        .range(0..=100_000).speed(1.0)
                        .prefix(tr("To "))).changed(){*page=0;}
                    if f.min_level>0 && f.max_level>0 && f.min_level>f.max_level {
                        ui.colored_label(WARN,tr("Minimum exceeds maximum"));
                    }
                    ui.label(tr("Location"));
                    // Include both names. Some server entries have area and region
                    // simultaneously; choosing one previously hid entire areas.
                    let mut places=std::collections::BTreeSet::new();
                    for (hunt,_) in atlas_destinations(view,reference){
                        for label in [&hunt.area,&hunt.region] {
                            if !label.trim().is_empty(){places.insert(label.as_str());}
                        }
                    }
                    egui::ComboBox::from_id_salt(("atlas-place",account))
                        .selected_text(if f.region.is_empty(){tr("All areas").to_owned()}
                            else{f.region.clone()})
                        .show_ui(ui,|ui|{
                            egui::ScrollArea::vertical()
                                .id_salt(("atlas-place-options",account))
                                .max_height(345.0)
                                .show(ui,|ui|{
                                    if ui.selectable_value(&mut f.region,String::new(),
                                        tr("All areas")).changed(){*page=0;}
                                    for place in places {
                                        if ui.selectable_value(&mut f.region,
                                            place.to_owned(),place).changed(){*page=0;}
                                    }
                                });
                        });
                });
                ui.horizontal_wrapped(|ui|{
                    ui.label(tr("Availability"));
                    egui::ComboBox::from_id_salt(("atlas-access",account))
                        .selected_text(match f.availability{
                            AtlasAvailability::All=>tr("All (guide + server)"),
                            AtlasAvailability::Unlocked=>tr("Unlocked"),
                            AtlasAvailability::Locked=>tr("Guide-only / locked"),
                        }).show_ui(ui,|ui|{
                            for (item,label) in [
                                (AtlasAvailability::All,tr("All (guide + server)")),
                                (AtlasAvailability::Unlocked,tr("Unlocked")),
                                (AtlasAvailability::Locked,tr("Guide-only / locked")),
                            ]{
                                if ui.selectable_value(&mut f.availability,item,label).changed(){
                                    *page=0;
                                }
                            }
                        });
                    ui.label(tr("Type"));
                    let mut types=reference.map(|r|r.chart.keys()
                        .map(String::as_str).collect::<Vec<_>>()).unwrap_or_default();
                    types.sort_unstable();
                    egui::ComboBox::from_id_salt(("atlas-type",account))
                        .selected_text(if f.kind.is_empty(){tr("All types").to_owned()}
                            else{atlas_type_name(&f.kind).to_owned()})
                        .show_ui(ui,|ui|{
                            if ui.selectable_value(&mut f.kind,String::new(),
                                tr("All types")).changed(){*page=0;}
                            for kind in types{
                                if ui.selectable_value(&mut f.kind,kind.to_owned(),
                                    atlas_type_name(kind)).changed(){*page=0;}
                            }
                        });
                    ui.label(tr("Captured"));
                    egui::ComboBox::from_id_salt(("atlas-owned",account))
                        .selected_text(match f.collection{
                            AtlasCollection::All=>tr("All"),
                            AtlasCollection::InDepot=>tr("Captured"),
                            AtlasCollection::NotInDepot=>tr("Not captured"),
                        }).show_ui(ui,|ui|{
                            for (item,label) in [
                                (AtlasCollection::All,tr("All")),
                                (AtlasCollection::InDepot,tr("Captured")),
                                (AtlasCollection::NotInDepot,tr("Not captured")),
                            ]{
                                if ui.selectable_value(&mut f.collection,item,label).changed(){
                                    *page=0;
                                }
                            }
                        }).response.on_hover_text(tr("Capture filter uses currently owned Pokémon, not past captures."));
                });
                ui.horizontal_wrapped(|ui|{
                    ui.label(tr("Weak to"));
                    if let Some(reference)=reference {
                        let mut kinds=reference.chart.keys().map(String::as_str)
                            .collect::<Vec<_>>();
                        kinds.sort_unstable();
                        for kind in kinds {
                            let selected=f.weak_to.contains(kind);
                            if ui.add(egui::Button::new(
                                RichText::new(atlas_type_name(kind)).size(9.0))
                                .fill(if selected{ACCENT.linear_multiply(0.25)}else{PANEL_ALT})
                                .stroke(Stroke::new(1.0,if selected{ACCENT}else{BORDER}))
                            ).clicked(){
                                if selected{f.weak_to.remove(kind);}
                                else{f.weak_to.insert(kind.to_owned());}
                                *page=0;
                            }
                        }
                    }else{
                        ui.label(RichText::new(tr("Guide type chart unavailable"))
                            .size(10.0).color(MUTED));
                    }
                });
                ui.horizontal_wrapped(|ui|{
                    if ui.checkbox(&mut f.weakness_match_all,
                        tr("Match all selected weaknesses (otherwise any)")).changed(){
                        *page=0;
                    }
                    if !f.weak_to.is_empty() && ui.small_button(tr("Clear weaknesses")).clicked(){
                        f.weak_to.clear();*page=0;
                    }
                });
            });
        });
}


fn atlas_first_unlocked(row:&AtlasSpeciesRow)->Option<&AtlasDestination>{
    row.destinations.iter().find(|destination|destination.unlocked)
}

/// Drilldown rows are compact by default; shared guide drops do not repeat
/// once per destination, and the Hunt action never targets a guide-only entry.
fn draw_atlas_destination(
    ui:&mut egui::Ui,dest:&AtlasDestination,view:&HuntView,
    native:&NativeSession,live:bool,error:&mut Option<String>
){
    let current=view.hunt_slug.as_deref()==Some(dest.hunt.slug.as_str());
    let pending=view.pending_hunt_slug.as_deref()==Some(dest.hunt.slug.as_str());
    egui::Frame::new().fill(PANEL_ALT).corner_radius(7.0)
        .inner_margin(egui::Margin::symmetric(10,8)).show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new(format!(
                    "{} · Lv {}",
                    dest.hunt.name,dest.hunt.level
                )).strong().size(11.0).color(TEXT));
                if current{
                    ui.colored_label(GOOD,tr("● Current"));
                }else if pending{
                    ui.colored_label(WARN,tr("Switching…"));
                }else if !dest.unlocked{
                    ui.colored_label(WARN,tr("Guide-only · locked"));
                }else if ui.add_enabled(live,
                    egui::Button::new(tr("Hunt →"))).clicked(){
                    match native.send_command(GameCommand::Hunt(
                        dest.hunt.slug.clone())){
                        Ok(())=>*error=None,
                        Err(e)=>*error=Some(e),
                    }
                }
            });
            if let Some(f)=dest.forecast.as_ref(){
                ui.horizontal_wrapped(|ui|{
                    for (label,value) in [
                        (tr("Trainer XP/h"),f.trainer_xp_h),
                        (tr("Pokémon XP/h"),f.pokemon_xp_h),
                        (tr("Gross gold/h"),f.gold_h),
                        (tr("Kills/h"),f.kills_h),
                    ]{
                        ui.vertical(|ui|{
                            ui.label(RichText::new(label).size(9.0).color(DIM));
                            ui.label(RichText::new(format_hunt_amount(value))
                                .size(11.0).strong().color(TEXT));
                        });
                        ui.add_space(12.0);
                    }
                });
                ui.label(RichText::new(format!(
                    "{} · {:.0}% {} · {} {} / h",
                    if f.calibrated{tr("Calibrated estimate")}else{tr("Early estimate")},
                    f.coverage*100.0,tr("species modeled"),tr("combat gold"),
                    format_hunt_amount(f.combat_gold_h)
                )).size(10.0).color(MUTED));
            }else{
                ui.label(RichText::new(tr("Combat forecast unavailable for this location"))
                    .size(10.0).color(MUTED));
            }
        });
}

fn draw_atlas_species(
    ui:&mut egui::Ui,row:&AtlasSpeciesRow,sort:AtlasSort,
    view:&HuntView,reference:Option<&AtlasReference>,
    filters:&AtlasFilters,native:&NativeSession,live:bool,
    error:&mut Option<String>,
){
    ui.push_id(("atlas-species",row.id),|ui|{
        egui::Frame::new().fill(PANEL).corner_radius(9.0)
            .stroke(Stroke::new(1.0,BORDER))
            .inner_margin(egui::Margin::symmetric(12,10)).show(ui,|ui|{
                ui.set_min_width(ui.available_width());
                let compact=ui.available_width()<490.0;
                ui.horizontal(|ui|{
                    ui.label(RichText::new(row.name.chars().take(2)
                        .collect::<String>().to_uppercase())
                        .size(16.0).strong().color(ACCENT));
                    ui.vertical(|ui|{
                        ui.horizontal_wrapped(|ui|{
                            ui.label(RichText::new(&row.name)
                                .size(14.0).strong().color(TEXT));
                            ui.label(RichText::new(format!("#{} · Lv {}",row.id,
                                row.destinations.first().map(|d|d.hunt.level).unwrap_or(0)))
                                .size(10.0).color(DIM));
                            if row.owned{
                                ui.colored_label(GOOD,tr("● In depot"));
                            }
                        });
                        ui.horizontal_wrapped(|ui|{
                            for kind in &row.types{
                                ui.label(RichText::new(atlas_type_name(kind)).size(10.0).color(ACCENT));
                            }
                            ui.label(RichText::new(format!(
                                "{} {}",row.destinations.len(),tr("destinations")
                            )).size(10.0).color(MUTED));
                            ui.label(RichText::new(format!("{}: {}",tr("Catalog sale value"),
                                reference.and_then(|r|r.catalog_sell_values.get(&row.id))
                                    .map(|v|format_hunt_amount(*v))
                                    .unwrap_or_else(||"—".into())))
                                .size(10.0).color(MUTED));
                        });
                    });
                    if !compact{
                        ui.with_layout(Layout::right_to_left(Align::Center),|ui|{
                            ui.vertical(|ui|{
                                ui.label(RichText::new(atlas_value_label(sort,row.best))
                                    .size(16.0).strong().color(TEXT));
                                ui.label(RichText::new(atlas_sort_label(sort))
                                    .size(10.0).color(DIM));
                            });
                        });
                    }
                });
                if compact{
                    ui.label(RichText::new(format!(
                        "{}  ·  {}",atlas_sort_label(sort),atlas_value_label(sort,row.best)
                    )).size(12.0).strong().color(TEXT));
                }
                draw_atlas_selected_weaknesses(
                    ui,reference,&row.types,&filters.weak_to
                );
                if let Some(best)=row.destinations.first(){
                    // The highest projected destination may be guide-only. The
                    // action instead uses the highest ranked unlocked location.
                    let available=atlas_first_unlocked(row);
                    ui.horizontal_wrapped(|ui|{
                        ui.label(RichText::new(tr_dynamic(format!(
                            "Top: {} · Lv {}",
                            best.hunt.name,best.hunt.level
                        ))).size(10.0).color(MUTED));
                        if !best.unlocked{
                            ui.colored_label(WARN,tr("Top result locked"));
                        }
                        if let Some(target)=available{
                            let current=view.hunt_slug.as_deref()
                                ==Some(target.hunt.slug.as_str());
                            let pending=view.pending_hunt_slug.as_deref()
                                ==Some(target.hunt.slug.as_str());
                            if current{
                                ui.colored_label(GOOD,tr("● Current hunt"));
                            }else if pending{
                                ui.colored_label(WARN,tr("Switching…"));
                            }else if ui.add_enabled(live,egui::Button::new(
                                if best.unlocked{tr("Hunt best →")}else{tr("Hunt unlocked →")}
                            )).on_hover_text(format!(
                                "Switch to {} (server-unlocked)",target.hunt.name
                            )).clicked(){
                                match native.send_command(GameCommand::Hunt(
                                    target.hunt.slug.clone())){
                                    Ok(())=>*error=None,
                                    Err(e)=>*error=Some(e),
                                }
                            }
                        }else{
                            ui.colored_label(WARN,tr("No unlocked destination"));
                        }
                    });
                }
                ui.add_space(8.0);
                let lead_types=reference
                    .map(|guide|atlas_types_for_active(view,guide))
                    .unwrap_or_default();
                draw_atlas_matchup(ui,reference,&lead_types,&row.types,
                    view.active_pokemon.as_ref().map(|p|p.name.as_str())
                        .unwrap_or(tr("Your Pokémon")),
                    &row.name,
                    tr("Against your active Pokémon"));
                ui.add_space(7.0);
                ui.push_id("destinations",|ui|{
                    egui::CollapsingHeader::new(tr_dynamic(format!(
                        "Destinations & forecasts ({})",row.destinations.len()
                    ))).default_open(false).show(ui,|ui|{
                        for dest in row.destinations.iter().take(4){
                            ui.push_id(&dest.hunt.slug,|ui|{
                                draw_atlas_destination(
                                    ui,dest,view,native,live,error);
                            });
                            ui.add_space(4.0);
                        }
                        if row.destinations.len()>4{
                            ui.collapsing(format!(
                                "Show {} more destinations",
                                row.destinations.len().saturating_sub(4).min(31)
                            ),|ui|{
                                for dest in row.destinations.iter().skip(4).take(31){
                                    ui.push_id(&dest.hunt.slug,|ui|{
                                        draw_atlas_destination(
                                            ui,dest,view,native,live,error);
                                    });
                                    ui.add_space(4.0);
                                }
                                if row.destinations.len()>35{
                                    ui.label(RichText::new(
                                        tr("Showing the first 35 ranked destinations.")
                                    ).size(10.0).color(MUTED));
                                }
                            });
                        }
                    });
                });

            });
        ui.add_space(6.0);
    });
}

/// Compact species selector used by the desktop master/detail Explorer.
/// Visual cues express selection and model confidence without duplicating the
/// expanded destination/drop sections in every row.
/// Entire rows are clickable. Lightweight text initials replace sprites so
/// Atlas never acquires Pokémon textures just for browsing.
fn draw_atlas_species_selector(
    ui:&mut egui::Ui,row:&AtlasSpeciesRow,
    sort:AtlasSort,selected:bool,
    reference:Option<&AtlasReference>,filters:&AtlasFilters,
)->bool{
    let row_frame=egui::Frame::new()
        .fill(if selected{ACCENT.linear_multiply(0.13)}else{PANEL})
        .stroke(Stroke::new(1.0,if selected{ACCENT}else{BORDER}))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(9,7))
        .show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui|{
                ui.label(RichText::new(row.name.chars().take(2)
                    .collect::<String>().to_uppercase())
                    .size(11.0).strong().color(ACCENT));
                ui.vertical(|ui|{
                    ui.label(RichText::new(&row.name)
                        .strong().size(12.0).color(TEXT));
                    ui.label(RichText::new(format!("{}: {}",tr("Catalog sale value"),
                        reference.and_then(|r|r.catalog_sell_values.get(&row.id))
                            .map(|v|format_hunt_amount(*v))
                            .unwrap_or_else(||"—".into())))
                        .size(9.0).color(MUTED));
                    ui.label(RichText::new(format!(
                        "#{} · {} {} · Lv {}{}",
                        row.id,row.destinations.len(),tr("locations"),
                        row.destinations.first().map(|d|d.hunt.level).unwrap_or(0),
                        if row.owned{tr(" · In depot")}else{""}
                    )).size(10.0).color(MUTED));
                });
                ui.with_layout(Layout::right_to_left(Align::Center),|ui|{
                    ui.vertical(|ui|{
                        ui.label(RichText::new(atlas_value_label(sort,row.best))
                            .size(12.0).strong()
                            .color(if row.best.is_some(){TEXT}else{DIM}));
                        ui.label(RichText::new(atlas_sort_label(sort))
                            .size(9.0).color(MUTED));
                    });
                });
            });
            draw_atlas_selected_weaknesses(
                ui,reference,&row.types,&filters.weak_to
            );
        });
    let selected=ui.interact(
        row_frame.response.rect,
        ui.id().with(("atlas-selector",row.id)),
        egui::Sense::click()
    ).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
    ui.add_space(5.0);
    selected
}

fn draw_atlas_explorer(
    ui:&mut egui::Ui,view:&HuntView,
    native:&NativeSession,live:bool,index:usize,
    reference:Option<&AtlasReference>,filters:&mut AtlasFilters,
    page:&mut usize,cache:&EstimateCache,scenario:&AtlasScenario,
    results:&mut ExplorerCache,
    command_error:&mut Option<String>,
){
    atlas_filters_panel(ui,filters,page,view,reference,index);
    ui.add_space(9.0);
    let rows=results.update(view,reference,cache,filters,scenario);
    if !filters.weak_to.is_empty(){
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new(tr_dynamic(format!(
                "Weak to: {} · {}",
                filters.weak_to.iter().map(|kind|atlas_type_name(kind).to_owned()).collect::<Vec<_>>().join(" + "),
                if filters.weakness_match_all{tr("ALL selected")}else{tr("ANY selected")}
            ))).size(11.0).strong().color(ACCENT));
        });
    }
    if filters.collection!=AtlasCollection::All && !view.roster_ready {
        ui.label(RichText::new(
            tr("Depot filtering uses the current account's roster; an empty or not-yet-loaded depot cannot establish historical captures.")
        ).size(10.0).color(WARN));
    }
    let unlocked_count=view.hunts.len();
    let guide_count=reference.map_or(0,|r|r.hunts.len());
    ui.horizontal_wrapped(|ui|{
        ui.label(RichText::new(format!(
            "{} {} · {} {} · {} {}",
            rows.len(),tr("species"),unlocked_count,tr("server-unlocked destinations"),
            guide_count,tr("guide destinations")
        )).strong().size(11.0).color(TEXT));
        if cache.is_pending(){
            ui.label(RichText::new(tr("Calculating combat models..."))
                .size(10.0).color(WARN));
        }
    });
    if matches!(filters.sort,AtlasSort::Shiny) {
        ui.label(RichText::new(
            tr("Shiny estimates use a base rate of 1 in 24,000 wild encounters outside Outland. Capture chances and extra bonuses are not included.")
        ).size(10.0).color(WARN));
    }
    if filters.sort==AtlasSort::SaleValue {
        ui.label(RichText::new(tr("Catalog reference, not an observed market sale average."))
            .size(10.0).color(MUTED));
    } else if matches!(filters.sort,AtlasSort::Capture|AtlasSort::ShinyCaught|AtlasSort::OutlandRare){
        ui.label(RichText::new(tr("Conditional model; capture odds use the selected ball and modifiers."))
            .size(10.0).color(MUTED));
    }
    if rows.is_empty(){
        egui::Frame::new().fill(PANEL).corner_radius(9.0)
            .stroke(Stroke::new(1.0,BORDER))
            .inner_margin(egui::Margin::same(14)).show(ui,|ui|{
                ui.set_min_width(ui.available_width());
                ui.label(RichText::new(tr("No matching Pokémon"))
                    .size(13.0).strong().color(TEXT));
                let message=if filters.collection!=AtlasCollection::All
                    && !view.roster_ready{
                    tr("Waiting for the full Pokémon depot before ownership can be filtered.")
                }else if cache.is_pending(){
                    tr("Combat estimates are updating; rankings will refresh as inputs arrive.")
                }else if reference.is_none(){
                    tr("The guide reference is not available yet. Species names and model coverage may be incomplete.")
                }else{
                    tr("Try a wider level range, different availability or fewer filters. Unknown model values are never treated as zero.")
                };
                ui.label(RichText::new(message).size(11.0).color(MUTED));
                if filters.active_count()>0 &&
                    ui.small_button(tr("Reset explorer filters")).clicked(){
                    filters.clear();*page=0;
                }
            });
        return;
    }
    // At desktop widths the list remains short and scannable while a stable
    // details pane exposes destinations, drop assumptions and hunt controls.
    // On narrow windows a single column avoids cramped dual-scroll panels.
    let desktop=ui.available_width()>=880.0;
    let visible=atlas_pager(ui,page,rows.len(),if desktop{20}else{8});
    ui.add_space(7.0);
    let mut error=None;
    if desktop{
        let slice=&rows[visible.clone()];
        // A changed filter/page must not leave the detail pane showing an
        // item outside the visible results or a stale account selection.
        let selected=filters.selected_species
            .filter(|id|slice.iter().any(|row|row.id==*id))
            .unwrap_or(slice[0].id);
        filters.selected_species=Some(selected);
        let pane_height=ui.available_height().max(160.0);
        let gap=ui.spacing().item_spacing.x;
        let total_width=ui.available_width();
        let list_width=(total_width*0.36).clamp(290.0,430.0);
        let details_width=(total_width-list_width-gap).max(0.0);

        // Reserve an actual rectangle for each pane. max_height() on its own
        // does NOT reserve viewport height: within a parent vertical
        // ScrollArea its child can receive zero/indefinite available height,
        // which previously clipped the twenty rows to a single row.
        ui.horizontal(|ui|{
            ui.allocate_ui_with_layout(
                egui::vec2(list_width,pane_height),
                Layout::top_down(Align::Min),
                |ui|{
                    ui.label(RichText::new("POKÉMON")
                        .size(10.0).strong().color(DIM));
                    ui.add_space(5.0);
                    let list_height=ui.available_height().max(120.0);
                    egui::ScrollArea::vertical()
                        .id_salt(("atlas-species-list",index))
                        .max_height(list_height)
                        .auto_shrink([false,false])
                        .show(ui,|ui|{
                            ui.set_min_width(ui.available_width());
                            for row in slice{
                                if draw_atlas_species_selector(
                                    ui,row,filters.sort,
                                    filters.selected_species==Some(row.id),
                                    reference,filters
                                ){
                                    filters.selected_species=Some(row.id);
                                }
                            }
                        });
                }
            );
            ui.allocate_ui_with_layout(
                egui::vec2(details_width,pane_height),
                Layout::top_down(Align::Min),
                |ui|{
                    ui.label(RichText::new(tr("SPECIES DETAILS"))
                        .size(10.0).strong().color(DIM));
                    ui.add_space(5.0);
                    let details_height=ui.available_height().max(120.0);
                    egui::ScrollArea::vertical()
                        .id_salt(("atlas-species-details",index,filters.selected_species))
                        .max_height(details_height)
                        .auto_shrink([false,false])
                        .show(ui,|ui|{
                            ui.set_min_width(ui.available_width());
                            if let Some(row)=slice.iter().find(|row|
                                filters.selected_species==Some(row.id)){
                                draw_atlas_species(ui,row,filters.sort,view,reference,
                                    filters,native,live,&mut error);
                            }
                        });
                }
            );
        });
    }else{
        let pane_height=ui.available_height().max(160.0);
        let pane_width=ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(pane_width,pane_height),
            Layout::top_down(Align::Min),
            |ui|{
                egui::ScrollArea::vertical()
                    .id_salt(("atlas-species-mobile",index))
                    .max_height(ui.available_height().max(120.0))
                    .auto_shrink([false,false])
                    .show(ui,|ui|{
                        ui.set_min_width(ui.available_width());
                        for row in rows.iter().skip(visible.start)
                            .take(visible.end-visible.start){
                            draw_atlas_species(ui,row,filters.sort,view,reference,
                                filters,native,live,&mut error);
                        }
                    });
            }
        );
    }
    if let Some(error)=error {*command_error=Some(error);}
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
                .unwrap_or_else(|_| Err(tr("Guide reference loader panicked").into()));
            let _ = tx.send(result);
        });
    }
    draw_atlas_view_controls(app,ui);
    if app.hunt_subtab==HuntSubtab::Compare {
        egui::ScrollArea::vertical()
            .id_salt("hunt-atlas-all-accounts")
            .auto_shrink([false,true]).show(ui,|ui|{
                ui.set_min_width(ui.available_width());
                draw_atlas_compare(app,ui);
            });
        return;
    }
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
            "● Guide loaded · {} species",reference.species.len()
        )).size(10.0).color(GOOD)).on_hover_text(
            tr("The guide is read-only; the game server alone unlocks hunts.")
        );
    }
    ui.add_space(6.0);

    let index=app.hunt_account;
    let Some(native)=app.games[index].native.as_ref() else {
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
                    let starting=app.games[index].child.is_some()
                        || app.games[index].orphaned_pid.is_some()
                        || app.cached_native_busy[index]
                        || app.native_verifying[index]
                        || app.native_handoff_busy[index];
                    if starting {
                        ui.label(RichText::new(tr("Account starting or waiting for sign-in"))
                            .size(11.0).color(WARN));
                    } else if app.accounts[index].enabled {
                        if ui.button(tr("Start account")).clicked(){
                            app.launch_one(app.games[index].profile);
                        }
                    } else if ui.button(tr("Enable account")).clicked(){
                        app.panel_tab=PanelTab::Accounts;
                    }
                    if ui.small_button(tr("Account settings")).clicked(){
                        app.panel_tab=PanelTab::Accounts;
                    }
                });
            });
        return;
    };
    let status=native.snapshot();
    let live=status.phase=="Online" && status.last_received
        .is_some_and(|received|received.elapsed()<=Duration::from_secs(90));
    let view=native.hunt_snapshot();
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
    app.hunt_estimate_cache[index].refresh(index,&view,app.hunt_reference.as_ref(),live);
    if live {
        if let Some(slug)=view.hunt_slug.as_deref() {
            let cache=&mut app.hunt_estimate_cache[index];
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
    if let Some(error)=app.hunt_action_error[index].clone(){
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new(tr_dynamic(format!("Command: {error}"))).color(BAD));
            if ui.small_button(tr("Dismiss")).clicked(){app.hunt_action_error[index]=None;}
        });
    }
    ui.add_space(6.0);
    if app.hunt_subtab!=HuntSubtab::Current &&
        draw_atlas_context(ui,&view,live){
        app.hunt_subtab=HuntSubtab::Current;
    }
    ui.add_space(6.0);
    draw_atlas_outland_tier(ui,&view,native,index,live,
        &mut app.hunt_action_error[index]);
    ui.add_space(6.0);
    // Explorer owns the remaining CentralPanel height. Do not wrap it in
    // a full-page vertical ScrollArea: that makes available_height() inside
    // its nested list/detail panes meaningless and leaves the view blank.
    if app.hunt_subtab==HuntSubtab::Explorer{
        draw_atlas_explorer(ui,&view,native,live,index,
            app.hunt_reference.as_deref(),&mut app.atlas_filters[index],
            &mut app.atlas_page[index],&app.hunt_estimate_cache[index],
            &app.atlas_scenarios[index],&mut app.atlas_results[index],
            &mut app.hunt_action_error[index]);
        return;
    }
    egui::ScrollArea::vertical().id_salt(("hunt-atlas-workspace",index,app.hunt_subtab as u8))
        .auto_shrink([false,true]).show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            match app.hunt_subtab {
                HuntSubtab::Explorer | HuntSubtab::Compare => {}
                HuntSubtab::Models => {
                    let history=native.history_snapshot();
                    models_panel::draw_atlas_models(ui,&view,
                        app.hunt_reference.as_deref(),
                        &app.hunt_estimate_cache[index],
                        &mut app.atlas_scenarios[index],&history,live);
                }
                HuntSubtab::Current => {
                    let hunts=&view;
                    let (activity,activity_color)=atlas_combat_status(hunts,live);
                    ui.horizontal_wrapped(|ui|{
                        ui.label(RichText::new(tr("Current hunt"))
                            .size(15.0).strong().color(TEXT));
                        ui.colored_label(activity_color,format!("● {activity}"));
                    });
                    let current=hunts.hunts.iter().find(|h|
                        hunts.hunt_slug.as_deref()==Some(h.slug.as_str()));
                    let title=if hunts.in_center{tr("At Pokémon Center")}else{
                        current.map(|h|h.name.as_str())
                            .or(hunts.hunt_slug.as_deref()).unwrap_or(tr("No confirmed hunt"))
                    };
                    ui.label(RichText::new(format!(
                        "{title} · {} {} · {} {}",
                        hunts.hunts.len(),tr("unlocked hunts"),hunts.battle_kills,tr("recorded kills")
                    )).size(11.0).color(MUTED));
                    ui.add_space(8.0);
                    let two_columns=ui.available_width()>=720.0;
                    ui.columns(if two_columns{2}else{1},|cols|{
                        egui::Frame::new().fill(PANEL).corner_radius(9.0)
                            .stroke(Stroke::new(1.0,BORDER))
                            .inner_margin(egui::Margin::same(12))
                            .show(&mut cols[0],|ui|{
                                ui.label(RichText::new(tr("LEAD & HUNT")).size(10.0)
                                    .strong().color(DIM));
                                ui.columns(2,|sides|{
                                    sides[0].vertical_centered(|ui|{
                                        ui.label(RichText::new(tr("YOUR POKÉMON"))
                                            .size(9.0).strong().color(DIM));
                                        if let Some(p)=hunts.active_pokemon.as_ref(){
                                            ui.label(RichText::new(&p.name)
                                                .size(13.0).strong().color(TEXT));
                                            ui.label(RichText::new(format!(
                                                "Lv {} · {}",p.level,
                                                if p.shiny{"Shiny"}else{tr("Active")}
                                            )).size(10.0).color(MUTED));
                                        }else{
                                            ui.label(RichText::new(tr("Lead not loaded"))
                                                .size(11.0).color(MUTED));
                                        }
                                    });
                                    sides[1].vertical_centered(|ui|{
                                        ui.label(RichText::new(tr("HUNT SPECIES"))
                                            .size(9.0).strong().color(DIM));
                                        let hunt_name=if hunts.in_center {
                                            tr("At Pokémon Center")
                                        } else {
                                            current.map(|h|h.name.as_str())
                                                .or(hunts.hunt_slug.as_deref())
                                                .unwrap_or(tr("No confirmed hunt"))
                                        };
                                        ui.label(RichText::new(hunt_name)
                                            .size(13.0).strong().color(TEXT));
                                        if let Some(h)=current.filter(|_|!hunts.in_center){
                                            ui.label(RichText::new(format!(
                                                "Lv {} · {}",h.level,tr("hunt species")
                                            )).size(10.0).color(MUTED));
                                        }else if hunts.in_center{
                                            ui.label(RichText::new(tr("Not hunting"))
                                                .size(10.0).color(MUTED));
                                        }
                                    });
                                });
                                if let Some(p)=hunts.active_pokemon.as_ref(){
                                    ui.separator();
                                    ui.label(RichText::new(format!(
                                        "P{} · {} {} · {} {}",
                                        p.potency.map(|n|n.to_string())
                                            .unwrap_or_else(||"—".into()),
                                        tr("Quality"),p.quality.map(|n|format!("{n:.3}"))
                                            .unwrap_or_else(||"—".into()),
                                        hunts.pokemon_count,tr("Pokémon in party")
                                    )).size(10.0).color(MUTED));
                                    let names=["HP","ATK","DEF",tr("SpA"),tr("SpD"),"SPD"];
                                    ui.label(RichText::new(names.iter().enumerate()
                                        .map(|(i,k)|format!("{} {}",k,
                                            p.ivs[i].map(|v|v.to_string())
                                                .unwrap_or_else(||"—".into())))
                                        .collect::<Vec<_>>().join(" · "))
                                        .size(10.0).color(DIM));
                                }
                            });
                        if !two_columns{cols[0].add_space(8.0);}
                        egui::Frame::new().fill(PANEL).corner_radius(9.0)
                            .stroke(Stroke::new(1.0,BORDER))
                            .inner_margin(egui::Margin::same(12))
                            .show(&mut cols[if two_columns{1}else{0}],|ui|{
                                let b=&hunts.bonuses;
                                ui.label(RichText::new(tr("BONUSES & LOCATION"))
                                    .size(10.0).strong().color(DIM));
                                ui.label(format!(
                                    "Twitch {} · {} {}",
                                    known_percent(b.twitch_pct,b.twitch_known),tr("Guild"),
                                    known_percent(b.guild_pct,b.guild_known)
                                ));
                                ui.label(format!(
                                    "{} {} · {} {}",
                                    tr("Trainer event"),known_percent(b.trainer_event_pct,b.trainer_event_known),
                                    tr("Pokémon event"),known_percent(b.pokemon_event_pct,b.pokemon_event_known)
                                ));
                                ui.label(RichText::new(format!(
                                    "VIP {} · {} {}",
                                    known_boost(b.vip_active,b.vip_known,
                                        hunts.server_now.is_some()),tr("Guild boost"),
                                    known_boost(b.guild_boost_active,b.guild_boost_known,
                                        hunts.server_now.is_some())
                                )).size(10.0).color(MUTED));
                                ui.separator();
                                if let Some(h)=current{
                                    ui.label(RichText::new(format!(
                                        "{} · {} · Lv {} · {} {}",
                                        h.area,h.region,h.level,h.total_spawns,tr("spawn slots")
                                    )).size(10.0).color(MUTED));
                                }
                                if let Some(slug)=hunts.pending_hunt_slug.as_deref(){
                                    ui.colored_label(WARN,tr_dynamic(format!("Switch pending: {slug}")));
                                }
                                if hunts.in_center{
                                    ui.colored_label(WARN,tr("At Pokémon Center"));
                                }
                                if let Some(map)=hunts.field_map.as_deref(){
                                    ui.label(RichText::new(tr_dynamic(format!("Field: {map}")))
                                        .size(10.0).color(DIM));
                                }
                            });
                    });
                    ui.add_space(8.0);
                    let guide=app.hunt_reference.as_deref();
                    let lead_types=guide.map(|guide|
                        atlas_types_for_active(hunts,guide))
                        .unwrap_or_default();
                    let wild_target=guide.and_then(|guide|
                        hunts.wild.as_ref().and_then(|wild|
                            wild.species_id.and_then(|id|
                                guide.species.get(&id)
                                    .map(|species|(species.name.as_str(),
                                        species.types.as_slice(),
                                        tr("Current wild species"))))));
                    let representative=guide.and_then(|guide|
                        current.and_then(|hunt|hunt.species.iter()
                            .filter(|species|species.weight>0.0)
                            .max_by(|left,right|
                                left.weight.total_cmp(&right.weight))
                            .and_then(|spawn|guide.species.get(&spawn.id)
                                .map(|species|(species.name.as_str(),
                                    species.types.as_slice(),
                                    tr("Most common hunt species"))))));
                    let target=representative.or(wild_target);
                    let (target_name,target_types,note)=target.unwrap_or((
                        tr("Unidentified opponent"),&[][..],
                        tr("Opponent typing is unavailable; no advantages or weaknesses are assumed.")
                    ));
                    draw_atlas_matchup(ui,guide,&lead_types,target_types,
                        hunts.active_pokemon.as_ref().map(|p|p.name.as_str())
                            .unwrap_or(tr("Your Pokémon")),
                        target_name,note);
                    ui.add_space(8.0);
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
    ui.add_space(10.0);
    egui::Frame::new().fill(PANEL).corner_radius(9.0)
        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
            ui.label(RichText::new(tr("Current hunt · estimated results"))
                .size(12.0).strong().color(TEXT));
            if let Some(forecast)=hunts.hunt_slug.as_deref()
                .and_then(|slug|app.hunt_estimate_cache[index].get(slug)) {
                ui.horizontal_wrapped(|ui|{
                    for (label,value) in [
                        (tr("Trainer XP/h"),forecast.trainer_xp_h),
                        (tr("Pokémon XP/h"),forecast.pokemon_xp_h),
                        (tr("Gross gold/h"),forecast.gold_h),
                        (tr("Kills/h"),forecast.kills_h),
                    ]{
                        draw_rate(ui,label,Some(value),
                            if forecast.calibrated {RateSource::Calibrated}
                            else {RateSource::Experimental});
                        ui.add_space(12.0);
                    }
                });
                ui.label(RichText::new(format!(
                    "{:.2}s / {} · {:.0}% {}",
                    forecast.seconds_per_kill,tr("kill"),forecast.coverage*100.0,
                    tr("guide species coverage")
                )).size(10.0).color(MUTED));
                ui.horizontal_wrapped(|ui|{
                    ui.label(format!(
                        "{} {} · {} {}",
                        tr("Trainer XP/kill"),format_hunt_amount(forecast.trainer_xp_per_kill),
                        tr("Pokémon XP/kill"),format_hunt_amount(forecast.pokemon_xp_per_kill)
                    ));
                    ui.label(format!("{} {}",
                        tr("Combat gold/h"),format_hunt_amount(forecast.combat_gold_h)));
                });
                for (channel,observed,valid,estimate) in [
                    (tr("Trainer XP"),m.trainer_xp_h,m.valid_trainer,forecast.trainer_xp_h),
                    (tr("Pokémon XP"),m.pokemon_xp_h,m.valid_pokemon,forecast.pokemon_xp_h),
                ]{
                    if live && valid && observed>0.0{
                        let ratio=estimate/observed;
                        ui.label(RichText::new(format!(
                            "{channel}: measured {} / h vs model {} / h ({ratio:.2}×)",
                            format_hunt_amount(observed),format_hunt_amount(estimate)
                        )).size(10.0).color(
                            if (ratio-1.0).abs()>=0.25 {WARN}else{MUTED}
                        ));
                    }
                }
                ui.label(RichText::new(
                    tr("These trainer/Pokémon streams must not be added together. Modeled gross gold excludes spending.")
                ).size(10.0).color(MUTED));
            } else {
                ui.label(RichText::new(if app.hunt_estimate_cache[index].is_pending(){
                    tr("Calculating your forecast…")
                }else{
                    tr("Unavailable: reference species or active Pokémon combat data is incomplete.")
                }).size(10.0).color(MUTED));
            }
        });

                }
                HuntSubtab::Session => {
                    let session=&view.session;
                    ui.label(RichText::new(tr("Battle results recorded by the game"))
                        .size(15.0).strong().color(TEXT));
                    ui.label(tr_dynamic(format!("Duration {:02}:{:02}:{:02}",
                        session.duration_secs/3600,
                        session.duration_secs%3600/60,
                        session.duration_secs%60)));
                    ui.add_space(8.0);
                    egui::Frame::new().fill(PANEL).corner_radius(8.0)
                        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
                            let stats=[
                                ("Kills",format_number(session.kills)),
                                ("Captures",format_number(session.captures)),
                                (tr("Trainer XP"),format_number(session.trainer_xp)),
                                (tr("Pokémon XP"),format_number(session.pokemon_xp)),
                                ("Combat gold",format_number(session.combat_gold)),
                                ("Auto-sale gold",format_number(session.auto_sale_gold)),
                            ];
                            let count=if ui.available_width()>=600.0{3}else{2};
                            ui.columns(count,|cols|{
                                for (i,(label,value)) in stats.iter().enumerate(){
                                    detail_stat_line(&mut cols[i%count],label,value);
                                }
                            });
                            ui.label(tr_dynamic(format!("Shiny encounter events: {} · shiny captures: {}",
                                session.shiny_seen,session.shiny_captured)));
                            let throws=session.balls_used.values().sum::<u64>();
                            ui.label(tr_dynamic(format!("Identified ball throws: {} · confirmed successes: {}",
                                throws,session.ball_captures)));
                            if throws>0{
                                ui.label(tr_dynamic(format!("Observed success per ball: {:.1}% ({} throws)",
                                    session.ball_captures as f64*100.0/throws as f64,throws)))
                                    .on_hover_text(tr("Observed ball outcomes in this session; not a species capture probability."));
                            }else {
                                ui.label(RichText::new(tr("Capture success rate will appear after your first recorded throw."))
                                    .color(MUTED));
                            }
                        });
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui|{
                        if !app.session_reset_confirm[index] {
                            if ui.small_button(tr("Reset session counters")).clicked() {
                                app.session_reset_confirm[index]=true;
                            }
                        } else {
                            ui.label(RichText::new(tr("Clear session-only totals?")).color(WARN));
                            if ui.small_button(tr("Confirm")).clicked(){
                                native.reset_hunt_session();
                                app.session_reset_confirm[index]=false;
                            }
                            if ui.small_button(tr("Cancel")).clicked(){
                                app.session_reset_confirm[index]=false;
                            }
                        }
                        if ui.small_button(tr("Open persistent history")).clicked(){
                            app.selected_details=Some(index);
                            app.dashboard_show_history=true;
                            app.history_rows_limit=100;
                            app.panel_tab=PanelTab::Overview;
                        }
                    });
                    ui.add_space(10.0);
                    ui.label(RichText::new(tr("Balls used")).size(13.0).strong().color(TEXT));
                    if session.balls_used.is_empty(){ui.label(tr("No ball throws recorded yet."));}
                    for (id,count) in &session.balls_used {
                        let name=view.ball_names.get(id).map(String::as_str).unwrap_or(tr("Unknown ball"));
                        ui.label(format!("{name} (#{id}) · {count}"));
                    }
                    ui.separator();
                    ui.horizontal_wrapped(|ui|{
                        ui.label(RichText::new(tr("Items acquired from battles"))
                            .size(13.0).strong().color(TEXT));
                        ui.label(RichText::new(format!(
                            "{} unique drops",session.acquired.len()
                        )).size(10.0).color(MUTED));
                    });
                    let mut drops=session.acquired.iter().collect::<Vec<_>>();
                    drops.sort_by(|a,b|b.1.cmp(a.1).then_with(||a.0.cmp(b.0)));
                    if drops.is_empty(){
                        ui.label(RichText::new(tr("No confirmed battle drops yet."))
                            .size(11.0).color(MUTED));
                    }else{
                        for (name,qty) in drops.iter().take(8){
                            ui.horizontal_wrapped(|ui|{
                                ui.label(RichText::new(*name)
                                    .size(11.0).color(TEXT));
                                ui.label(RichText::new(format!("×{qty}"))
                                    .size(11.0).strong().color(ACCENT));
                            });
                        }
                        if drops.len()>8{
                            ui.collapsing(format!(
                                "All acquired items ({} unique)",drops.len()
                            ),|ui|{
                                for (name,qty) in drops.iter().take(200){
                                    ui.label(format!("{name} ×{qty}"));
                                }
                                if drops.len()>200{
                                    ui.label(tr("Showing the first 200 items."));
                                }
                            });
                        }
                    }
                    if !session.recent_drops.is_empty(){
                        ui.collapsing(tr("Latest drop events"),|ui|{
                            for (name,count) in &session.recent_drops{
                                ui.label(format!("{name} ×{count}"));
                            }
                        });
                    }
                    ui.separator();
                    ui.collapsing(tr("Persisted history for current hunt"),|ui|{
                        let history=native.history_snapshot();
                        if let Some(error)=history.load_error.as_deref()
                            .or(history.save_error.as_deref()) {
                            ui.label(RichText::new(error).color(WARN));
                        }
                        if let Some(slug)=view.hunt_slug.as_deref(){
                            let records:Vec<_>=history.days.iter()
                                .filter(|row|row.hunt==slug).collect();
                            if records.is_empty(){
                                ui.label(tr("No historical observations saved for this hunt."));
                            }else{
                                let sum=|getter:fn(&crate::native_history::HistoryDay)->u64|
                                    records.iter().fold(0u64,|total,row|
                                        total.saturating_add(getter(row)));
                                ui.label(format!(
                                    "{} {} · {} {} · {} {} · {} {}",
                                    records.len(),tr("retained days"),sum(|d|d.kills),tr("kills"),
                                    sum(|d|d.captures),tr("captures"),sum(|d|d.shiny_seen),tr("shiny events")
                                ));
                                ui.label(format!(
                                    "{} {} · {} {} · {} {}",
                                    tr("Trainer XP"),format_hunt_amount(sum(|d|d.trainer_xp) as f64),
                                    tr("Pokémon XP"),format_hunt_amount(sum(|d|d.pokemon_xp) as f64),
                                    tr("Combat gold"),format_hunt_amount(sum(|d|d.combat_gold) as f64),
                                ));
                                ui.label(RichText::new(
                                    tr("These are persistent totals, not historical XP/hour; per-hunt elapsed time is not currently retained.")
                                ).size(10.0).color(MUTED));
                            }
                        }else{
                            ui.label(tr("No current hunt ID is available for historical grouping."));
                        }
                    });
                }
                HuntSubtab::Recommendations => {
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
                        for (ranking,label) in [
                            (HuntRanking::TrainerXp,tr("Trainer XP")),
                            (HuntRanking::PokemonXp,tr("Pokémon XP")),
                            (HuntRanking::Gold,tr("Gold")),
                        ]{
                            if ui.selectable_value(&mut app.hunt_ranking,ranking,label)
                                .changed(){
                                app.hunt_page=0;
                            }
                        }
                    });
                    ui.horizontal_wrapped(|ui|{
                        if ui.add(egui::TextEdit::singleline(&mut app.hunt_query)
                            .hint_text(tr("Hunt, area, region or species")).desired_width(220.0)).changed(){
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
                                    tr("All types").to_owned()
                                }else{app.hunt_type.clone()})
                                .show_ui(ui,|ui|{
                                    if ui.selectable_value(&mut app.hunt_type,
                                        String::new(),tr("All types")).changed(){app.hunt_page=0;}
                                    for kind in types{
                                        if ui.selectable_value(&mut app.hunt_type,
                                            kind.to_owned(),atlas_type_name(kind)).changed(){app.hunt_page=0;}
                                    }
                                });
                        }
                        if ui.small_button(tr("Clear filters")).clicked(){
                            app.hunt_query.clear();app.hunt_region.clear();
                            app.hunt_type.clear();app.hunt_page=0;
                        }
                    });
                    let reference=app.hunt_reference.as_deref();
                    let ranked=ranked_hunts(hunts,reference,app.hunt_ranking,live,
                        &app.hunt_query,&app.hunt_region,&app.hunt_type,
                        &app.hunt_estimate_cache[index]);
                    ui.label(RichText::new(format!(
                        "{} {} · {}",
                        ranked.len(),tr("matching hunts"),tr("only current hunt has measured rates")
                    )).size(11.0).color(MUTED));
                    if !live {ui.label(tr("Rankings disabled while account data is stale."));}
                    let visible=atlas_pager(ui,&mut app.hunt_page,ranked.len(),16);
                    ui.add_space(7.0);
                    for (offset,result) in ranked.iter().skip(visible.start)
                        .take(visible.end-visible.start).enumerate(){
                        draw_hunt_row(ui,result,visible.start+offset+1,app.hunt_ranking,
                            hunts,reference,live,native,&mut app.hunt_action_error[index]);
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
                                    app.hunt_query.clear();
                                    app.hunt_region.clear();
                                    app.hunt_type.clear();
                                    app.hunt_page=0;
                                }
                            });
                    }
                    ui.add_space(7.0);
                    ui.collapsing(tr("XP bonus assumptions"),|ui|{
                        let b=&hunts.bonuses;
                        ui.label(format!(
                            "{}: {} · {}: {} · {}: {} · Twitch: {}",
                            tr("Trainer event"),known_percent(b.trainer_event_pct,b.trainer_event_known),
                            tr("Pokémon event"),known_percent(b.pokemon_event_pct,b.pokemon_event_known),
                            tr("Guild"),known_percent(b.guild_pct,b.guild_known),
                            known_percent(b.twitch_pct,b.twitch_known)
                        ));
                        ui.label(format!(
                            "VIP: {} · {}: {}",
                            known_boost(b.vip_active,b.vip_known,hunts.server_now.is_some()),
                            tr("Guild boost"),known_boost(b.guild_boost_active,b.guild_boost_known,hunts.server_now.is_some())
                        ));
                        ui.label(tr("Guide estimates include the bonuses shown above. Same-speed comparisons do not add these bonuses again. Any boost the game has not reported is excluded."));
                    });
                    ui.label(RichText::new(tr("Gold forecasts are gross expected rewards, not measured net gold. Guide model uses approximate DPS and may significantly overestimate real combat throughput."))
                        .size(10.0).color(MUTED));
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn guide_fixture()->AtlasReference{
        let mut guide=AtlasReference::default();
        guide.chart.insert("FIGHTING".into(),
            [("NORMAL".into(),2.0)].into_iter().collect());
        guide.chart.insert("GHOST".into(),
            [("NORMAL".into(),0.0)].into_iter().collect());
        guide.chart.insert("NORMAL".into(),Default::default());
        guide.species.insert(1,native_atlas::ReferenceSpecies{
            name:"Testmon".into(),types:vec!["NORMAL".into()],
            base:vec![100.0;6],moves:vec![],
            drops:vec![native_atlas::ReferenceDrop{
                item_id:None,item_name:Some("Moon Stone".into()),
                chance_pct:10.0,minimum:1.0,maximum:3.0,guide_price:5.0,
            }],
        });
        guide.hunts.push(native_hunts::HuntEntry{
            slug:"guide-locked".into(),name:"Guide Preview".into(),level:200,
            area:"Region".into(),
            species:vec![native_hunts::HuntSpecies{id:1,weight:100.0}],
            ..Default::default()
        });
        guide
    }
    #[test]
    fn type_matchups_respect_immunity_stacked_dual_types_and_neutral(){
        let mut guide=AtlasReference::default();
        for kind in ["FIRE","GRASS","ICE","WATER","ROCK","NORMAL","GHOST"]{
            guide.chart.insert(kind.into(),Default::default());
        }
        guide.chart.get_mut("FIRE").unwrap().extend([
            ("GRASS".into(),2.0),("ICE".into(),2.0),
            ("WATER".into(),0.5),("ROCK".into(),0.5),
        ]);
        guide.chart.get_mut("NORMAL").unwrap()
            .insert("GHOST".into(),0.0);
        let hit=|attack:&str,types:&[&str]|{
            atlas_type_multiplier(&guide,attack,
                &types.iter().map(|v|v.to_string()).collect::<Vec<_>>())
        };
        assert_eq!(hit("NORMAL",&["GHOST"]),Some(0.0));
        assert_eq!(hit("FIRE",&["WATER"]),Some(0.5));
        assert_eq!(hit("FIRE",&["WATER","ROCK"]),Some(0.25));
        assert_eq!(hit("FIRE",&["GRASS"]),Some(2.0));
        assert_eq!(hit("FIRE",&["GRASS","ICE"]),Some(4.0));
        assert_eq!(hit("FIRE",&["NORMAL"]),Some(1.0));
        assert_eq!(hit("fire",&["Grass","ICE"]),Some(4.0));
        assert_eq!(hit("FIRE",&[]),None);
        assert_eq!(hit("FIRE",&["NOT_IN_CHART"]),None);
        assert_eq!(hit("UNKNOWN",&["ICE"]),None);
    }
    #[test]
    fn matchup_colors_change_with_damage_direction(){
        assert_eq!(atlas_multiplier_color(4.0,false),GOOD);
        assert_eq!(atlas_multiplier_color(4.0,true),BAD);
        assert_eq!(atlas_multiplier_color(0.0,false),BAD);
        assert_eq!(atlas_multiplier_color(0.0,true),GOOD);
        assert_eq!(atlas_multiplier_color(0.5,false),BAD);
        assert_eq!(atlas_multiplier_color(0.5,true),GOOD);
        assert_eq!(atlas_multiplier_text(0.0),"0×");
        assert_eq!(atlas_multiplier_text(0.25),"0,25×");
        assert_eq!(atlas_multiplier_text(0.5),"0,5×");
        assert_eq!(atlas_multiplier_text(2.0),"2×");
        assert_eq!(atlas_multiplier_text(4.0),"4×");
    }
    #[test]
    fn live_party_types_take_precedence_over_guide_fallback(){
        let mut guide=AtlasReference::default();
        guide.species.insert(25,native_atlas::ReferenceSpecies{
            name:"Pikachu".into(),types:vec!["ELECTRIC".into()],
            base:vec![10.0;6],moves:vec![],drops:vec![]
        });
        let mut view=HuntView::default();
        assert!(atlas_types_for_active(&view,&guide).is_empty());
        view.active_pokemon=Some(native_hunts::PokemonInfo{
            species_id:Some(25),..Default::default()
        });
        assert_eq!(atlas_types_for_active(&view,&guide),vec!["ELECTRIC"]);
        view.active_pokemon.as_mut().unwrap().types=vec!["STEEL".into()];
        assert_eq!(atlas_types_for_active(&view,&guide),vec!["STEEL"]);
    }
    #[test]
    fn explorer_release_drops_previous_source_handles() {
        let mut cache=ExplorerCache::default();
        let view=HuntView::default();
        let original=Arc::clone(&view.hunts);
        cache.update(&view,None,&EstimateCache::default(),
            &AtlasFilters::default(),&AtlasScenario::default());
        assert!(Arc::strong_count(&original)>=3);
        cache.release();
        assert_eq!(Arc::strong_count(&original),2);
        assert!(!cache.initialized);
    }
    #[test]
    fn cached_destination_does_not_clone_hunt_species() {
        let mut hunt=native_hunts::HuntEntry::default();
        hunt.slug="test".into();
        hunt.name="Test".into();
        hunt.level=100;
        hunt.species.push(native_hunts::HuntSpecies::default());
        let label=AtlasHuntLabel::from_hunt(&hunt);
        assert_eq!(label.slug,"test");
        assert_eq!(label.name,"Test");
        assert_eq!(label.level,100);
    }
    #[test]
    fn species_action_uses_an_unlocked_destination_not_a_locked_best(){
        let locked=native_hunts::HuntEntry{
            slug:"guide-only".into(),..Default::default()
        };
        let accessible=native_hunts::HuntEntry{
            slug:"server-unlocked".into(),..Default::default()
        };
        let mut row=AtlasSpeciesRow{
            id:1,name:"Testmon".into(),types:vec![],owned:false,
            best:Some(200.0),
            destinations:vec![
                AtlasDestination{
                    hunt:Arc::new(AtlasHuntLabel::from_hunt(&locked)),unlocked:false,share:0.7,
                    forecast:None,objective:Some(200.0)
                },
                AtlasDestination{
                    hunt:Arc::new(AtlasHuntLabel::from_hunt(&accessible)),unlocked:true,share:0.3,
                    forecast:None,objective:Some(100.0)
                }
            ],
        };
        assert_eq!(
            atlas_first_unlocked(&row).map(|destination|destination.hunt.slug.as_str()),
            Some("server-unlocked")
        );
        row.destinations[1].unlocked=false;
        assert!(atlas_first_unlocked(&row).is_none());
    }
    #[test]
    fn hunt_status_uses_confirmed_location_not_wild_slot(){
        let mut view=HuntView::default();
        assert_eq!(atlas_combat_status(&view,false).0,"Offline / stale");
        assert_eq!(atlas_combat_status(&view,true).0,"No confirmed hunt");
        view.hunt_slug=Some("scyther-field".into());
        assert_eq!(atlas_combat_status(&view,true).0,"Hunting");
        view.measurements.combat_event_age_secs=Some(10);
        assert_eq!(atlas_combat_status(&view,true).0,"Hunt active");
        view.wild=Some(native_hunts::WildInfo::default());
        assert_eq!(atlas_combat_status(&view,true).0,"Hunt active");
        view.wild=None; // Encounter cleared; location and activity persist.
        assert_eq!(atlas_combat_status(&view,true).0,"Hunt active");
        view.measurements.combat_event_age_secs=Some(181);
        assert_eq!(atlas_combat_status(&view,true).0,"In hunt · no recent activity");
        view.in_center=true;
        assert_eq!(atlas_combat_status(&view,true).0,"At Pokémon Center");
    }
    #[test]
    fn explorer_preserves_authoritative_unlocked_and_locked_separation(){
        let guide=guide_fixture();
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"server-unlocked".into(),name:"Current".into(),level:50,
            species:vec![native_hunts::HuntSpecies{id:1,weight:80.0}],
            ..Default::default()
        }]);
        let locations=atlas_destinations(&view,Some(&guide));
        assert_eq!(locations.len(),2);
        assert!(locations.iter().any(|(hunt,valid)|
            *valid && hunt.slug=="server-unlocked"));
        assert!(locations.iter().any(|(hunt,valid)|
            !*valid && hunt.slug=="guide-locked"));
        let cache=EstimateCache::default();
        let mut filters=AtlasFilters::default();
        filters.availability=AtlasAvailability::Unlocked;
        assert_eq!(atlas_species_rows(&view,Some(&guide),&cache,&filters,&AtlasScenario::default())[0]
            .destinations.len(),1);
        filters.availability=AtlasAvailability::All;
        assert_eq!(atlas_species_rows(&view,Some(&guide),&cache,&filters,&AtlasScenario::default())[0]
            .destinations.len(),2);
        filters.availability=AtlasAvailability::Locked;
        assert_eq!(atlas_species_rows(&view,Some(&guide),&cache,&filters,&AtlasScenario::default())[0]
            .destinations[0].hunt.slug,"guide-locked");
        filters.min_level=250;
        assert!(atlas_species_rows(&view,Some(&guide),&cache,&filters,&AtlasScenario::default()).is_empty());
    }
    #[test]
    fn multi_weakness_filter_uses_each_multiplier_for_any_and_all_modes(){
        let mut guide=AtlasReference::default();
        for kind in ["FIRE","FIGHTING","GRASS","ICE","NORMAL","ROCK"]{
            guide.chart.insert(kind.into(),Default::default());
        }
        guide.chart.get_mut("FIRE").unwrap().extend([
            ("GRASS".into(),2.0),("ICE".into(),2.0),
            ("NORMAL".into(),1.0),("ROCK".into(),2.0),
        ]);
        guide.chart.get_mut("FIGHTING").unwrap().extend([
            ("GRASS".into(),0.5),("ICE".into(),1.0),
            ("NORMAL".into(),2.0),("ROCK".into(),2.0),
        ]);
        for (id,name,types) in [
            (1,"Four times Fire",vec!["GRASS","ICE"]),
            (2,"Double Fire Quadruple Fighting",vec!["NORMAL","ROCK"]),
        ]{
            guide.species.insert(id,native_atlas::ReferenceSpecies{
                name:name.into(),types:types.into_iter().map(str::to_owned).collect(),
                base:vec![100.0;6],moves:vec![],drops:vec![],
            });
        }
        let types_a=&guide.species[&1].types;
        let types_b=&guide.species[&2].types;
        assert_eq!(atlas_type_multiplier(&guide,"FIRE",types_a),Some(4.0));
        assert_eq!(atlas_type_multiplier(&guide,"FIGHTING",types_a),Some(0.5));
        assert_eq!(atlas_type_multiplier(&guide,"FIRE",types_b),Some(2.0));
        assert_eq!(atlas_type_multiplier(&guide,"FIGHTING",types_b),Some(4.0));
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"multi".into(),name:"Multi".into(),level:100,
            species:vec![
                native_hunts::HuntSpecies{id:1,weight:50.0},
                native_hunts::HuntSpecies{id:2,weight:50.0},
            ],
            ..Default::default()
        }]);
        let cache=EstimateCache::default();
        let mut filters=AtlasFilters::default();
        filters.weak_to.extend(["FIRE".into(),"FIGHTING".into()]);
        let scenario=AtlasScenario::default();
        let rows=atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario);
        assert_eq!(rows.len(),2,"any-mode shows species weak to either");
        filters.weakness_match_all=true;
        let rows=atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario);
        assert_eq!(rows.len(),1,"all-mode requires both selected types >1×");
        assert_eq!(rows[0].id,2);
    }
    #[test]
    fn explorer_weakness_and_depot_filters_follow_selected_account(){
        let guide=guide_fixture();
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"current".into(),name:"Current".into(),level:100,
            species:vec![native_hunts::HuntSpecies{id:1,weight:100.0}],
            ..Default::default()
        }]);
        view.pokemon_roster=Arc::new(vec![native_hunts::PokemonInfo{
            species_id:Some(1),id:15,..Default::default()
        }]);
        view.roster_ready=true;
        let cache=EstimateCache::default();
        let mut filters=AtlasFilters::default();
        filters.collection=AtlasCollection::InDepot;
        filters.weak_to.insert("FIGHTING".into());
        assert_eq!(atlas_species_rows(&view,Some(&guide),&cache,&filters,&AtlasScenario::default()).len(),1);
        filters.weak_to.insert("GHOST".into());
        filters.weakness_match_all=true;
        assert!(atlas_species_rows(&view,Some(&guide),&cache,&filters,&AtlasScenario::default()).is_empty());
        filters.weakness_match_all=false;
        filters.collection=AtlasCollection::NotInDepot;
        assert!(atlas_species_rows(&view,Some(&guide),&cache,&filters,&AtlasScenario::default()).is_empty());
    }

    #[test]
    fn gross_gold_and_sale_value_do_not_include_obsolete_npc_drops(){
        let hunt=native_hunts::HuntEntry{
            slug:"a".into(),name:"A".into(),level:120,area:"Kanto".into(),
            ..Default::default()
        };
        let forecast=native_atlas::Forecast {
            kills_h:100.0,trainer_xp_h:1000.0,pokemon_xp_h:900.0,
            gold_h:80.0,combat_gold_h:80.0,npc_loot_gold_h:0.0,
            trainer_xp_per_kill:10.0,pokemon_xp_per_kill:9.0,
            seconds_per_kill:36.0,coverage:1.0,calibrated:false,
        };
        assert_eq!(atlas_objective(AtlasSort::Gold,Some(&forecast),&hunt,1.0,None),Some(80.0));
        assert_eq!(atlas_objective(AtlasSort::Xp,Some(&forecast),&hunt,1.0,None),Some(1000.0));
        assert_eq!(atlas_objective(AtlasSort::SaleValue,None,&hunt,1.0,Some(500.0)),Some(500.0));
        assert_eq!(atlas_objective(AtlasSort::SaleValue,None,&hunt,1.0,None),None);
        assert_eq!(atlas_objective(AtlasSort::Shiny,None,&hunt,0.5,None),None);
        assert_eq!(atlas_objective(AtlasSort::Shiny,Some(&forecast),&hunt,0.5,None),Some(100.0*0.5/24000.0));
        let outland=native_hunts::HuntEntry{area:"Outland".into(),..hunt};
        assert_eq!(atlas_objective(AtlasSort::Shiny,Some(&forecast),&outland,0.5,None),None);
    }
    #[test]
    fn capture_chance_and_sale_value_do_not_depend_on_kills_per_hour(){
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"route".into(),name:"Route".into(),level:120,
            species:vec![native_hunts::HuntSpecies{id:1,weight:100.0}],
            ..Default::default()
        }]);
        let mut guide=guide_fixture();
        let cache=EstimateCache::default();
        let mut filters=AtlasFilters::default();
        filters.sort=AtlasSort::Capture;
        let scenario=AtlasScenario::default();
        let rows=atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario);
        assert_eq!(rows[0].best,None);
        guide.capture_rarities.insert(1,12000.0);
        let rows=atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario);
        assert!(rows[0].best.is_some_and(|n|n>0.0&&n<=0.10));
        filters.sort=AtlasSort::SaleValue;
        assert_eq!(atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario)[0].best,None);
        guide.catalog_sell_values.insert(1,400.0);
        let after=atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario);
        assert_eq!(after[0].best,Some(400.0));
    }
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

    #[test]
    fn explorer_cache_skips_selection_only_redraws_but_not_new_filters() {
        let view=HuntView::default();
        let estimates=EstimateCache::default();
        let scenario=AtlasScenario::default();
        let mut filters=AtlasFilters::default();
        let mut results=ExplorerCache::default();
        assert!(results.update(&view,None,&estimates,&filters,&scenario).is_empty());
        assert_eq!(results.rebuilds,1);
        filters.selected_species=Some(25);
        results.update(&view,None,&estimates,&filters,&scenario);
        assert_eq!(results.rebuilds,1);
        filters.min_level=80;
        results.update(&view,None,&estimates,&filters,&scenario);
        assert_eq!(results.rebuilds,2);
    }
    #[test]
    fn gold_ranking_never_compares_net_wallet_spend_to_gross_rewards(){
        let mut view=HuntView::default();
        view.hunt_slug=Some("a".into());
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"a".into(),name:"A".into(),level:100,..Default::default()
        }]);
        view.measurements.valid_gold=true;
        view.measurements.gold_net_h=-50_000.0;
        let mut cache=EstimateCache::default();
        cache.refresh(0,&view,None,true);
        let rows=ranked_hunts(&view,None,HuntRanking::Gold,true,"","","",&cache);
        assert_eq!(rows.len(),1);
        assert!(rows[0].value.is_none());
        assert_eq!(rows[0].source,RateSource::Unavailable);
    }

}
