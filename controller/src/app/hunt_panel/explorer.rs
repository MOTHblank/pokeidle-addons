use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AtlasCollection { All, InDepot, NotInDepot }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AtlasSort {
    Kills, Xp, Gold, Capture, ShinyCaught, OutlandRare, SaleValue, Shiny, Level, Name,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::app) struct AtlasFilters {
    pub(super) search: String,
    pub(super) region: String,
    pub(super) kind: String,
    pub(super) min_level: u32,
    pub(super) max_level: u32,
    pub(super) collection: AtlasCollection,
    pub(super) weak_to: std::collections::BTreeSet<String>,
    pub(super) weakness_match_all: bool,
    pub(super) sort: AtlasSort,
    pub(super) ascending: bool,
    /// Per-account Explorer selection; stable across repaint and socket snapshots.
    pub(super) selected_species: Option<u64>,
}
impl Default for AtlasFilters {
    fn default()->Self {
        Self{
            search:String::new(),region:String::new(),kind:String::new(),
            min_level:0,max_level:0,
            collection:AtlasCollection::All,
            weak_to:std::collections::BTreeSet::new(),weakness_match_all:false,
            sort:AtlasSort::Xp,ascending:false,selected_species:None,
        }
    }
}
impl AtlasFilters {
    pub(super) fn clear(&mut self) {
        let sort=self.sort;
        let ascending=self.ascending;
        *self=Self::default();
        self.sort=sort;self.ascending=ascending;
    }
    pub(super) fn active_count(&self)->usize {
        usize::from(!self.search.trim().is_empty())+
        usize::from(!self.region.is_empty())+
        usize::from(!self.kind.is_empty())+
        usize::from(self.min_level>0)+usize::from(self.max_level>0)+
        usize::from(self.collection!=AtlasCollection::All)+
        usize::from(!self.weak_to.is_empty())
    }
}

// Explorer needs only these three fields to display/select a destination.
// Keeping an entire HuntEntry (including its species allocation) alive in
// each account's cached results multiplied RAM after the ranking-cache change.
#[derive(Clone)]
pub(super) struct AtlasHuntLabel {
    pub(super) slug:String,
    pub(super) name:String,
    pub(super) level:u32,
}
impl AtlasHuntLabel {
    pub(super) fn from_hunt(hunt:&native_hunts::HuntEntry)->Self {
        Self {slug:hunt.slug.clone(),name:hunt.name.clone(),level:hunt.level}
    }
}
#[derive(Clone)]
pub(super) struct AtlasDestination {
    pub(super) hunt:Arc<AtlasHuntLabel>,
    pub(super) unlocked:bool,
    pub(super) forecast:Option<native_atlas::Forecast>,
    pub(super) objective:Option<f64>,
    pub(super) mkt_value:Option<f64>,
}
pub(super) struct AtlasSpeciesRow {
    pub(super) id:u64,
    pub(super) name:String,
    pub(super) types:Vec<String>,
    pub(super) owned:bool,
    pub(super) destinations:Vec<AtlasDestination>,
    pub(super) best:Option<f64>,
}

pub(super) fn atlas_destinations<'a>(
    view:&'a HuntView,_reference:Option<&'a AtlasReference>
)->Vec<(&'a native_hunts::HuntEntry,bool)> {
    // The authenticated server's hunt catalog is the only destination list.
    view.hunts.iter().map(|hunt|(hunt,true)).collect()
}

/// Neutral multipliers omitted from the bundled upstream chart are 1×.
pub(super) fn atlas_locations(
    view:&HuntView,_reference:Option<&AtlasReference>
)->std::collections::BTreeSet<String>{
    let mut locations=std::collections::BTreeSet::new();
    for hunt in view.hunts.iter(){
        for name in [&hunt.area,&hunt.region]{
            if !name.trim().is_empty(){locations.insert(name.trim().to_owned());}
        }
    }
    locations
}
pub(super) fn atlas_hunt_matches_location(hunt:&native_hunts::HuntEntry,location:&str)->bool{
    location.is_empty()||
    hunt.area.trim().eq_ignore_ascii_case(location)||
    hunt.region.trim().eq_ignore_ascii_case(location)
}

pub(super) fn atlas_objective(
    sort:AtlasSort, forecast:Option<&native_atlas::Forecast>,
    hunt:&native_hunts::HuntEntry, share:f64, sale_value:Option<f64>
)->Option<f64>{
    match sort {
        AtlasSort::Name=>None,
        AtlasSort::Level=>Some(hunt.level as f64),
        // Exactly one XP stream: never add trainer and Pokémon experience.
        AtlasSort::Kills=>forecast.map(|f|f.kills_h),
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
pub(super) fn atlas_sort_label(sort:AtlasSort)->&'static str {
    match sort {
        AtlasSort::Kills=>tr("Kills/h · combat speed"),
        AtlasSort::Xp=>tr("XP/h"),
        AtlasSort::Gold=>tr("Gross gold/h"),
        AtlasSort::Capture=>tr("Capture chance"),
        AtlasSort::ShinyCaught=>tr("Conditional shiny caught/h"),
        AtlasSort::OutlandRare=>tr("Outland rare units/h"),
        AtlasSort::SaleValue=>tr("MKT sale value"),
        AtlasSort::Shiny=>tr("Shiny encounter/h (conditional)"),
        AtlasSort::Level=>tr("Hunt level"),
        AtlasSort::Name=>tr("Species name"),
    }
}
pub(super) fn atlas_value_label(sort:AtlasSort,value:Option<f64>)->String {
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
pub(in crate::app) struct ExplorerCache {
    pub(super) rows:Vec<AtlasSpeciesRow>,
    pub(super) filters:Option<AtlasFilters>,
    pub(super) scenario:Option<AtlasScenario>,
    pub(super) catalog:Option<Arc<Vec<native_hunts::HuntEntry>>>,
    pub(super) reference_ptr:usize,
    pub(super) forecast_revision:u64,
    pub(super) model_signature:Option<(u64,u32,Option<u64>,Option<u64>,[Option<u64>;6],bool,bool,Option<[u64;6]>,Option<String>,Vec<String>)>,
    pub(super) trainer_level:Option<u32>,
    pub(super) outland_catalog:Option<[(u32,f64);8]>,
    pub(super) roster:Option<Arc<Vec<native_hunts::PokemonInfo>>>,
    pub(super) owned_species:std::collections::BTreeSet<u64>,
    pub(super) roster_ready:bool,
    pub(super) ball_quotes:Option<Arc<std::collections::BTreeMap<u64,native_hunts::BallQuote>>>,
    pub(super) initialized:bool,
    #[cfg(test)]
    pub(super) rebuilds:usize,
}
impl ExplorerCache {
    pub(in crate::app) fn release(&mut self) {
        // Release cached forecasts, destination labels and Arc source handles
        // when Explorer is not the active workspace. Only one account needs
        // rankings resident at a time.
        if self.initialized { *self=Self::default(); }
    }
    pub(super) fn update(&mut self,view:&HuntView,reference:Option<&AtlasReference>,
        estimates:&EstimateCache,filters:&AtlasFilters,scenario:&AtlasScenario,
    )->&[AtlasSpeciesRow] {
        let mut key_filters=filters.clone();
        key_filters.selected_species=None;
        // Explorer uses only the modeling fields relevant to this sort;
        // disclosure state, selected destination and unrelated scenario
        // knobs must not trigger a full species/destination sort.
        let mut key_scenario=scenario.clone();
        key_scenario.hunt_slug.clear();
        key_scenario.modifiers_open=false;
        match filters.sort {
            AtlasSort::Capture=>{
                key_scenario.tier=1;
                key_scenario.rare_target=70000;
                key_scenario.accepted_fraction_override=None;
            }
            AtlasSort::ShinyCaught=>{
                key_scenario.tier=1;
                key_scenario.rare_target=70000;
            }
            AtlasSort::OutlandRare=>{
                key_scenario.ball=BallKind::Ultra;
                key_scenario.accepted_fraction_override=None;
            }
            _=>key_scenario=AtlasScenario::default(),
        }
        // Roster Arc changes on HP/XP updates. Only the owned species IDs
        // affect the Explorer labels and collection filter.
        let roster_changed=self.roster.as_ref().is_none_or(|old|
            !Arc::ptr_eq(old,&view.pokemon_roster));
        let species=roster_changed.then(||view.pokemon_roster.iter()
            .filter_map(|p|p.species_id)
            .collect::<std::collections::BTreeSet<_>>());
        let ownership_changed=species.as_ref().is_some_and(|ids|
            ids!=&self.owned_species);
        // Retaining the previous source Arc prevents pointer reuse from
        // hiding a genuine server roster/catalog update.
        let reference_ptr=reference.map_or(0,|r|r as *const _ as usize);
        // Explorer gold now comes from the same immutable combat forecasts
        // as XP and kills; unrelated live reward receipts cannot reorder it.
        let changed=!self.initialized ||
            self.filters.as_ref()!=Some(&key_filters) ||
            self.scenario.as_ref()!=Some(&key_scenario) ||
            self.catalog.as_ref().is_none_or(|old|!Arc::ptr_eq(old,&view.hunts)) ||
            self.reference_ptr!=reference_ptr ||
            self.forecast_revision!=estimates.revision ||
            self.model_signature!=estimates.active_signature ||
            self.trainer_level!=view.trainer_level ||
            self.outland_catalog!=view.outland_catalog ||
            ownership_changed ||
            self.roster_ready!=view.roster_ready ||
            self.ball_quotes.as_ref().is_none_or(|old|!Arc::ptr_eq(old,&view.ball_quotes));
        if changed {
            #[cfg(test)]
            {self.rebuilds+=1;}
            self.rows=atlas_species_rows(view,reference,estimates,filters,scenario);
            self.filters=Some(key_filters);
            self.scenario=Some(key_scenario);
            self.catalog=Some(Arc::clone(&view.hunts));
            self.reference_ptr=reference_ptr;
            self.forecast_revision=estimates.revision;
            self.model_signature=estimates.active_signature.clone();
            self.trainer_level=view.trainer_level;
            self.outland_catalog=view.outland_catalog;
            self.roster_ready=view.roster_ready;
            self.ball_quotes=Some(Arc::clone(&view.ball_quotes));
            self.initialized=true;
        }
        if let Some(species)=species {
            self.owned_species=species;
            self.roster=Some(Arc::clone(&view.pokemon_roster));
        }
        &self.rows
    }
}

pub(super) fn atlas_species_rows(
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
    let mut lowest_hunt_levels=std::collections::BTreeMap::<u64,u32>::new();
    for hunt in view.hunts.iter(){
        for spawn in &hunt.species{
            if spawn.weight>0.0&&hunt.level>0{
                lowest_hunt_levels.entry(spawn.id)
                    .and_modify(|n|*n=(*n).min(hunt.level))
                    .or_insert(hunt.level);
            }
        }
    }
    for (hunt,unlocked) in atlas_destinations(view,reference) {
        if filters.min_level>0 && hunt.level<filters.min_level
            || filters.max_level>0 && hunt.level>filters.max_level
            || !atlas_hunt_matches_location(hunt,&filters.region)
        {continue}
        let sum=hunt.species.iter().map(|s|s.weight.max(0.0)).sum::<f64>();
        if sum<=0.0 {continue}
        let hunt_forecast=cache.get(&hunt.slug);
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
            let forecast=hunt_forecast.clone();
            let mkt_value=reference.and_then(|r|native_atlas::mkt_sale_estimate(
                r,spawn.id,hunt.level,
                lowest_hunt_levels.get(&spawn.id).copied().unwrap_or(hunt.level)
            ));
            // The same full-wave simulator must feed both XP and gold.
            // Measured gold is available in Current hunt, never mixed into rankings.
            let base=atlas_objective(filters.sort,forecast.as_ref(),hunt,share,mkt_value);

            let objective=match filters.sort {
                AtlasSort::Capture=>reference.and_then(|r|{
                    let efficiency=view.ball_quotes.get(&scenario.ball.id())
                        .and_then(|q|q.efficiency)
                        .unwrap_or_else(||scenario.ball.wiki_efficiency());
                    let (stage,rarity)=native_atlas::capture_inputs(r,spawn.id);
                    models::capture_chance_for_stage(stage,rarity,efficiency,0.0)
                }).map(|chance|(chance*scenario.modifiers.catch_factor()).min(0.10)),
                AtlasSort::ShinyCaught=>forecast.as_ref()
                    .and_then(|f|reference.and_then(|r|{
                        let efficiency=view.ball_quotes.get(&scenario.ball.id())
                            .and_then(|q|q.efficiency)
                            .unwrap_or_else(||scenario.ball.wiki_efficiency());
                        let (stage,rarity)=native_atlas::capture_inputs(r,spawn.id);
                        models::capture_scenario_with_stage(
                            stage,rarity,efficiency,f.kills_h,share,
                            scenario.accepted_fraction_override,
                            hunt.area.eq_ignore_ascii_case("outland"),
                            &scenario.modifiers)
                    })).and_then(|estimate|estimate.shiny_caught_h),
                AtlasSort::OutlandRare=>forecast.as_ref().and_then(|f|
                    models::rare_scenarios_with_gate(
                        hunt.area.eq_ignore_ascii_case("outland"),hunt.level,
                        view.outland_requirement(scenario.tier),
                        view.trainer_level,f.kills_h,&scenario.modifiers)
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
                hunt:Arc::clone(label),unlocked,forecast,objective,mkt_value,
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


/// Classic Pokémon type palette, used only for the Atlas type picker.
pub(super) const ATLAS_TYPES:[&str;18]=[
    "NORMAL","FIRE","WATER","ELECTRIC","GRASS","ICE",
    "FIGHTING","POISON","GROUND","FLYING","PSYCHIC","BUG",
    "ROCK","GHOST","DRAGON","DARK","STEEL","FAIRY",
];
pub(super) fn atlas_type_color(kind:&str)->Color32{
    let (r,g,b)=match kind {
        "NORMAL"=>(0xA8,0xA7,0x7A),
        "FIRE"=>(0xEE,0x81,0x30),
        "WATER"=>(0x63,0x90,0xF0),
        "ELECTRIC"=>(0xF7,0xD0,0x2C),
        "GRASS"=>(0x7A,0xC7,0x4C),
        "ICE"=>(0x96,0xD9,0xD6),
        "FIGHTING"=>(0xC2,0x2E,0x28),
        "POISON"=>(0xA3,0x3E,0xA1),
        "GROUND"=>(0xE2,0xBF,0x65),
        "FLYING"=>(0xA9,0x8F,0xF3),
        "PSYCHIC"=>(0xF9,0x55,0x87),
        "BUG"=>(0xA6,0xB9,0x1A),
        "ROCK"=>(0xB6,0xA1,0x36),
        "GHOST"=>(0x73,0x57,0x97),
        "DRAGON"=>(0x6F,0x35,0xFC),
        "DARK"=>(0x70,0x57,0x46),
        "STEEL"=>(0xB7,0xB7,0xCE),
        "FAIRY"=>(0xD6,0x85,0xAD),
        _=>(0x78,0x80,0x8A),
    };
    Color32::from_rgb(r,g,b)
}
/// Standard relative-luminance contrast rule: white/black lettering
/// remains readable even on yellow Electric or pale Ice and Steel tiles.
pub(super) fn atlas_type_text_color(background:Color32)->Color32{
    let linear=|c:u8|{
        let x=c as f32/255.0;
        if x<=0.04045{x/12.92}else{((x+0.055)/1.055).powf(2.4)}
    };
    let luminance=0.2126*linear(background.r())+
        0.7152*linear(background.g())+0.0722*linear(background.b());
    if luminance>0.179{Color32::BLACK}else{Color32::WHITE}
}
pub(super) fn atlas_type_grid_columns(available:f32)->usize{
    let gap=6.0f32;
    (((available.max(0.0)+gap)/(112.0+gap)).floor() as usize).clamp(1,6)
}
pub(super) fn draw_atlas_weakness_tiles(
    ui:&mut egui::Ui,f:&mut AtlasFilters,page:&mut usize,
    reference:Option<&AtlasReference>,account:usize,
){
    let available=ui.available_width();
    let gap=6.0f32;
    let columns=atlas_type_grid_columns(available);
    let width=((available-gap*(columns.saturating_sub(1) as f32))
        /columns as f32).max(64.0);
    egui::Grid::new(("atlas-weak-type-grid",account))
        .num_columns(columns).spacing([gap,gap])
        .show(ui,|ui|{
            for (i,kind) in ATLAS_TYPES.iter().enumerate(){
                let selected=f.weak_to.contains(*kind);
                let base=atlas_type_color(kind);
                // Keep the exact classic hue visible in both states.
                // The checkmark and contrast-colored border, not dimming,
                // communicate which type is selected.
                let fg=atlas_type_text_color(base);
                let response=ui.add_enabled(
                    reference.is_some(),
                    egui::Button::new(RichText::new(if selected{
                        format!("✓ {}",atlas_type_name(kind))
                    }else{atlas_type_name(kind).to_owned()})
                        .size(12.0).strong().color(fg))
                        .fill(base)
                        .stroke(Stroke::new(if selected{2.5}else{1.0},
                            if selected{fg}else{base.gamma_multiply(0.84)}))
                        .corner_radius(6.0)
                        .min_size(egui::vec2(width,36.0))
                ).on_hover_text(tr("Filter Pokémon by weakness to this attack type"));
                if response.clicked(){
                    if selected{f.weak_to.remove(*kind);}
                    else{f.weak_to.insert((*kind).to_owned());}
                    *page=0;
                }
                if (i+1)%columns==0{ui.end_row();}
            }
        });
}

/// Equal-width grid tiles, with selected checkmark AND outline; selection
/// never relies on hue alone. At narrow widths columns decrease instead of
/// wrapping eighteen tiny buttons into an unpredictable line.
pub(super) fn draw_atlas_weakness_grid(
    ui:&mut egui::Ui,f:&mut AtlasFilters,page:&mut usize,
    reference:Option<&AtlasReference>,account:usize,
){
    egui::Frame::new().fill(PANEL_ALT)
        .stroke(Stroke::new(1.0,BORDER))
        .corner_radius(8.0).inner_margin(egui::Margin::same(9))
        .show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new(tr("Weak to"))
                    .size(12.0).strong().color(TEXT));
                if !f.weak_to.is_empty(){
                    ui.label(RichText::new(format!(
                        "{} {}",f.weak_to.len(),tr("selected types")
                    )).size(10.0).color(ACCENT));
                }
                ui.add_space(8.0);
                ui.label(RichText::new(tr("Match"))
                    .size(10.0).color(DIM));
                if ui.selectable_value(&mut f.weakness_match_all,false,tr("Any"))
                    .on_hover_text(tr("Match weaknesses to at least one selected attack type"))
                    .changed(){*page=0;}
                if ui.selectable_value(&mut f.weakness_match_all,true,tr("All"))
                    .on_hover_text(tr("Match weaknesses to every selected attack type"))
                    .changed(){*page=0;}
                if !f.weak_to.is_empty() &&
                    ui.small_button(tr("Clear weaknesses")).clicked(){
                    f.weak_to.clear();*page=0;
                }
            });
            ui.add_space(5.0);
            if reference.is_none(){
                ui.label(RichText::new(tr("Type matchup data unavailable"))
                    .size(10.0).color(MUTED));
            }
            // Let the page scroll instead of trapping wheel input inside the
            // weakness grid on narrow windows.
            draw_atlas_weakness_tiles(ui,f,page,reference,account);

        });
}
pub(super) fn atlas_filters_panel(
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
            // Keep filters in the same document flow as the results, so the
            // user can scroll the entire Atlas page from any section.
            ui.vertical(|ui|{
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
                                AtlasSort::Kills,AtlasSort::Xp,AtlasSort::Gold,AtlasSort::Capture,
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
                    if ui.selectable_label(f.sort==AtlasSort::Kills,
                        tr("Fastest kills")).clicked(){
                        f.sort=AtlasSort::Kills;f.ascending=false;*page=0;
                    }
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
                    let places=atlas_locations(view,reference);
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
            });
            ui.add_space(8.0);
            draw_atlas_weakness_grid(ui,f,page,reference,account);
        });
}


pub(super) fn atlas_first_unlocked(row:&AtlasSpeciesRow)->Option<&AtlasDestination>{
    row.destinations.iter().find(|destination|destination.unlocked)
}

/// Drilldown rows are compact by default; shared guide drops do not repeat
/// once per destination, and the Hunt action never targets a guide-only entry.
pub(super) fn draw_atlas_destination(
    ui:&mut egui::Ui,dest:&AtlasDestination,view:&HuntView,
    native:&NativeSession,live:bool,error:&mut Option<String>,
    pending_ui:&mut Option<String>,aoe_enabled:bool,
){
    let current=view.hunt_slug.as_deref()==Some(dest.hunt.slug.as_str());
    let pending=view.pending_hunt_slug.as_deref()==Some(dest.hunt.slug.as_str())
        || pending_ui.as_deref()==Some(dest.hunt.slug.as_str());
    egui::Frame::new()
        .fill(if pending{WARN.linear_multiply(0.07)}else{PANEL_ALT})
        .stroke(Stroke::new(1.0,if pending{WARN.linear_multiply(0.65)}else{BORDER}))
        .corner_radius(7.0)
        .inner_margin(egui::Margin::symmetric(10,8)).show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new(format!(
                    "{} · Lv {}",
                    dest.hunt.name,dest.hunt.level
                )).strong().size(11.0).color(TEXT));
                if !dest.unlocked && !current && !pending {
                    ui.colored_label(WARN,tr("Unavailable"));
                } else if dest.unlocked &&
                    draw_hunt_switch_action(ui,current,pending,live) {
                    request_hunt_with_feedback(
                        native,&dest.hunt.slug,error,pending_ui);
                }
            });
            if let Some(f)=dest.forecast.as_ref(){
                draw_atlas_combat_cues(ui,view.active_pokemon.as_ref(),Some(f),aoe_enabled);
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui|{
                    for (label,value) in [
                        (tr("Trainer XP/h"),Some(f.trainer_xp_h)),
                        (tr("Pokémon XP/h"),Some(f.pokemon_xp_h)),
                        (tr("Gross gold/h"),Some(f.gold_h)),
                        (tr("Kills/h"),Some(f.kills_h)),
                    ]{
                        ui.vertical(|ui|{
                            ui.label(RichText::new(label).size(9.0).color(DIM));
                            ui.label(RichText::new(value.map(format_hunt_amount)
                                .unwrap_or_else(||"—".into()))
                                .size(11.0).strong().color(TEXT));
                        });
                        ui.add_space(12.0);
                    }
                });
                ui.label(RichText::new(format!(
                    "{} · {:.0}% {} · {} {} / h",
                    tr("Simulated combat"),
                    f.coverage*100.0,tr("species modeled"),tr("combat gold"),
                    format_hunt_amount(f.combat_gold_h)
                )).size(10.0).color(MUTED));
            }else{
                ui.label(RichText::new(tr("Combat forecast unavailable for this location"))
                    .size(10.0).color(MUTED));
            }
        });
}

pub(super) fn draw_atlas_species(
    ui:&mut egui::Ui,row:&AtlasSpeciesRow,sort:AtlasSort,
    view:&HuntView,reference:Option<&AtlasReference>,
    filters:&AtlasFilters,native:&NativeSession,live:bool,
    error:&mut Option<String>,pending_ui:&mut Option<String>,
    aoe_enabled:bool,
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
                            ui.label(RichText::new(format!("{}: {}",tr("MKT estimate"),
                                row.destinations.first().and_then(|d|d.mkt_value)
                                    .map(format_hunt_amount)
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
                    draw_atlas_combat_cues(ui,view.active_pokemon.as_ref(),best.forecast.as_ref(),aoe_enabled);
                    ui.add_space(5.0);
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
                                ==Some(target.hunt.slug.as_str())
                                || pending_ui.as_deref()==Some(target.hunt.slug.as_str());
                            if draw_hunt_switch_action(ui,current,pending,live) {
                                request_hunt_with_feedback(
                                    native,&target.hunt.slug,error,pending_ui);
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
                                    ui,dest,view,native,live,error,pending_ui,aoe_enabled);
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
                                            ui,dest,view,native,live,error,pending_ui,aoe_enabled);
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
pub(super) fn draw_atlas_species_selector(
    ui:&mut egui::Ui,row:&AtlasSpeciesRow,
    sort:AtlasSort,selected:bool,
    reference:Option<&AtlasReference>,filters:&AtlasFilters,
    lead:Option<&native_hunts::PokemonInfo>,
    aoe_enabled:bool,
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
                    ui.label(RichText::new(format!("{}: {}",tr("MKT estimate"),
                        row.destinations.first().and_then(|d|d.mkt_value)
                            .map(format_hunt_amount)
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
            if let Some(top)=row.destinations.first(){
                draw_atlas_combat_cues(ui,lead,top.forecast.as_ref(),aoe_enabled);
            }
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

pub(super) fn draw_atlas_explorer(
    ui:&mut egui::Ui,view:&HuntView,
    native:&NativeSession,live:bool,index:usize,aoe_enabled:bool,
    reference:Option<&AtlasReference>,filters:&mut AtlasFilters,
    page:&mut usize,cache:&EstimateCache,scenario:&AtlasScenario,
    results:&mut ExplorerCache,
    command_error:&mut Option<String>,
    command_pending:&mut Option<String>,
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
    ui.horizontal_wrapped(|ui|{
        ui.label(RichText::new(format!(
            "{} {} · {} {}",
            rows.len(),tr("species"),view.hunts.len(),tr("available hunts")
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
    if filters.sort==AtlasSort::Gold {
        ui.label(RichText::new(tr("Gold rankings use simulated kills/h for every hunt; measured current-hunt gold is shown separately."))
            .size(10.0).color(MUTED));
    }
    if filters.sort==AtlasSort::SaleValue {
        ui.label(RichText::new(tr("Estimated MKT NPC buyback for a non-shiny quality-1 capture, using its caught level. Not RMT player-market prices."))
            .size(10.0).color(MUTED));
    } else if matches!(filters.sort,AtlasSort::Capture|AtlasSort::ShinyCaught|AtlasSort::OutlandRare){
        ui.label(RichText::new(tr("Conditional model; capture odds use the selected ball and modifiers."))
            .size(10.0).color(MUTED));
    }
    // Keep travel controls accessible even when the active filters return
    // zero matches; they should never depend on Explorer search results.
    let desktop=ui.available_width()>=880.0;
    let visible=atlas_explorer_pager(
        ui,page,rows.len(),if desktop{20}else{8},
        view,native,live,command_error,
    );
    ui.add_space(7.0);
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
                    tr("Pokémon data is still loading; some estimates may be unavailable.")
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
    let mut error=None;
    if desktop{
        let slice=&rows[visible.clone()];
        // A changed filter/page must not leave the detail pane showing an
        // item outside the visible results or a stale account selection.
        let selected=filters.selected_species
            .filter(|id|slice.iter().any(|row|row.id==*id))
            .unwrap_or(slice[0].id);
        filters.selected_species=Some(selected);
        let gap=ui.spacing().item_spacing.x;
        let total_width=ui.available_width();
        let list_width=(total_width*0.36).clamp(290.0,430.0);
        let details_width=(total_width-list_width-gap).max(0.0);

        // Both panes take their natural content height. A single enclosing
        // ScrollArea handles page scrolling, including the filters above.
        // Fixed-height nested scrollers previously trapped the mouse wheel
        // and made the rest of Hunt Atlas impossible to scroll.
        ui.horizontal_top(|ui|{
            ui.vertical(|ui|{
                ui.set_width(list_width);
                ui.label(RichText::new("POKÉMON")
                    .size(10.0).strong().color(DIM));
                ui.add_space(5.0);
                for row in slice{
                    if draw_atlas_species_selector(
                        ui,row,filters.sort,
                        filters.selected_species==Some(row.id),
                        reference,filters,view.active_pokemon.as_ref(),aoe_enabled
                    ){
                        filters.selected_species=Some(row.id);
                    }
                }
            });
            ui.vertical(|ui|{
                ui.set_width(details_width);
                ui.label(RichText::new(tr("SPECIES DETAILS"))
                    .size(10.0).strong().color(DIM));
                ui.add_space(5.0);
                if let Some(row)=slice.iter().find(|row|
                    filters.selected_species==Some(row.id)){
                    draw_atlas_species(ui,row,filters.sort,view,reference,
                        filters,native,live,&mut error,command_pending,aoe_enabled);
                }
            });
        });
    }else{
        // On narrow windows the same page scroll contains all result cards.
        for row in rows.iter().skip(visible.start)
            .take(visible.end-visible.start){
            draw_atlas_species(ui,row,filters.sort,view,reference,
                filters,native,live,&mut error,command_pending,aoe_enabled);
        }
    }
    if let Some(error)=error {*command_error=Some(error);}
}

