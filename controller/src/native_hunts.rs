//! Native, account-scoped hunt intelligence. No DOM, Violentmonkey or polling.
//! Derived independently from server welcome, estado patches and batalha events.
//! Other-hunt projections are explicitly scenario comparisons, NOT combat forecasts.
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default)]
pub struct HuntSpecies {
    pub id: u64,
    pub weight: f64,
}
#[derive(Clone, Debug, Default)]
pub struct HuntEntry {
    pub slug: String,
    pub name: String,
    pub area: String,
    pub region: String,
    pub level: u32,
    pub total_spawns: u32,
    // Retained for hunt artwork even when no current widget displays it.
    #[allow(dead_code)]
    pub looktype: Option<u64>,
    pub species: Vec<HuntSpecies>,
}
// PokéIdle has its own species identifiers (including alternate forms), and
// they are not restricted to the National Pokédex range. This compact table
// is copied from the game's upstream sprite patch catalog.
#[derive(Default)]
struct GameSpeciesArt {
    looktypes: BTreeMap<u64, u64>,
    names: Vec<(String, u64)>,
}

fn species_art_catalog() -> &'static GameSpeciesArt {
    static CATALOG: OnceLock<GameSpeciesArt> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut catalog = GameSpeciesArt::default();
        for line in include_str!("../assets/visual/pokedex-names.tsv").lines() {
            if line.starts_with('#') { continue; }
            let mut fields = line.splitn(3, '\t');
            let (Some(id), Some(looktype), Some(name)) =
                (fields.next(), fields.next(), fields.next()) else { continue };
            let (Ok(id), Ok(looktype)) = (id.parse::<u64>(), looktype.parse::<u64>())
                else { continue };
            if id == 0 || looktype == 0 { continue; }
            let normalized = normalized_species_name(name);
            if normalized.is_empty() { continue; }
            catalog.looktypes.insert(id, looktype);
            catalog.names.push((format!(" {normalized} "), id));
        }
        catalog
    })
}

pub fn game_species_looktype(id: u64) -> Option<u64> {
    species_art_catalog().looktypes.get(&id).copied()
}

/// Normal and alternate PokéIdle species IDs can share the same sprite
/// looktype. Only *after* this explicit mapping do we use a Pokédex CDN.
pub fn game_species_dex_sprite(id: u64) -> Option<u64> {
    let looktype = game_species_looktype(id)?;
    (60001..=61025).contains(&looktype).then_some(looktype - 60000)
}

fn normalized_species_name(name: &str) -> String {
    name.chars().map(|ch| {
        if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { ' ' }
    }).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

// Name lookup is only a fallback if the game's confirmed hunt catalog did
// not include its species ID. Resolve full form names before base names.
fn species_id_from_hunt_name(name: &str) -> Option<u64> {
    let normalized = format!(" {} ", normalized_species_name(name));
    species_art_catalog().names.iter()
        .filter(|(needle, _)| normalized.contains(needle))
        .max_by_key(|(needle, _)| needle.len())
        .map(|(_, id)| *id)
}

/// A hunt represents one species. Use the game's ID directly, rather than
/// treating its spawn list as a weighted draw or reading the current enemy.
pub fn representative_hunt_species(hunt: &HuntEntry) -> Option<u64> {
    hunt.species.iter().find(|species| species.id > 0)
        .map(|species| species.id)
        .or_else(|| species_id_from_hunt_name(&hunt.name))
        .or_else(|| species_id_from_hunt_name(&hunt.slug))
}

#[derive(Clone, Debug, Default)]
pub struct PokemonInfo {
    pub id: u64,
    pub species_id: Option<u64>,
    pub name: String,
    pub level: u32,
    pub xp: Option<u64>,
    pub xp_level: Option<u64>,
    pub xp_next: Option<u64>,
    pub hp: Option<u64>,
    pub max_hp: Option<u64>,
    pub held_item_id: Option<u64>,
    pub power: Option<u64>,
    pub quality: Option<f64>,
    pub grade: Option<f64>,
    pub potency: Option<u64>,
    pub looktype: Option<u64>,
    pub shiny_looktype: Option<u64>,
    pub shiny: bool,
    pub ivs: [Option<u64>; 6],
    pub types: Vec<String>,
    /// Server-computed battle stats take precedence over reconstructed IV stats.
    pub battle_stats: Option<[f64; 6]>,
    /// AoE TM reaches multiple wild Pokémon in the hunt's 11×11 tile square.
    pub tm_aoe: bool,
    pub tm_elemental: Option<String>,
}
#[derive(Clone, Debug, Default)]
pub struct WildInfo {
    // These identifiers are retained for reliable target/artwork matching.
    #[allow(dead_code)]
    pub slot: u64,
    pub name: String,
    pub level: u32,
    #[allow(dead_code)]
    pub species_id: Option<u64>,
    pub hp: u64,
    pub max_hp: u64,
}
#[derive(Clone, Debug, Default)]
pub struct HuntBonuses {
    pub trainer_event_pct: f64,
    pub pokemon_event_pct: f64,
    pub guild_pct: f64,
    pub twitch_pct: f64,
    pub guild_boost_active: bool,
    pub vip_active: bool,
    pub trainer_event_known: bool,
    pub pokemon_event_known: bool,
    pub guild_known: bool,
    pub twitch_known: bool,
    pub guild_boost_known: bool,
    pub vip_known: bool,
    pub kick_xp_pct: f64,
    pub shop_trainer_xp_multiplier: f64,
    pub shop_pokemon_xp_multiplier: f64,
    pub delayed_xp_multiplier: f64,
}
#[derive(Clone, Debug, Default)]
pub struct HuntMeasurements {
    pub seconds: f64,
    pub kills: u64,
    pub kills_h: f64,
    pub trainer_xp_h: f64,
    pub pokemon_xp_h: f64,
    pub gold_net_h: f64,
    /// Gross Coins paid in server-confirmed wild-death receipts, excluding sales.
    pub gold_gross_h: f64,
    pub valid_gross_gold: bool,
    pub valid_trainer: bool,
    pub valid_pokemon: bool,
    pub valid_gold: bool,
    pub window_samples: usize,
    /// Age of the most recent server-confirmed kill, rather than a state poll.
    pub last_battle_age_secs: Option<u64>,
    /// Age of the latest server-confirmed combat action or wild HP change.
    /// Unlike last_battle_age_secs, this detects activity during long fights.
    pub combat_event_age_secs: Option<u64>,
}
#[derive(Clone, Debug, Default)]
pub struct HuntSessionStats {
    pub duration_secs: u64,
    pub kills:u64,
    pub captures:u64,
    pub trainer_xp:u64,
    pub pokemon_xp:u64,
    pub combat_gold:u64,
    pub auto_sale_gold:u64,
    pub shiny_seen:u64,
    pub shiny_captured:u64,
    pub balls_used:BTreeMap<u64,u64>,
    pub ball_captures:u64,
    pub acquired:BTreeMap<String,u64>,
    pub recent_drops:Vec<(String,u64)>,
}
#[derive(Clone,Debug,Default)]
pub struct BallQuote {
    pub efficiency:Option<f64>,
    pub gold_price:Option<f64>,
    pub diamond_price:Option<f64>,
}
#[derive(Clone, Debug, Default)]
pub struct HuntView {
    pub account_nick: Option<String>,
    pub field_map: Option<String>,
    pub field_slug: Option<String>,
    pub field_init_timestamp: Option<u64>,
    /// Real full-wave spawn-distance quantiles, learned only from native field events.
    pub geo_distances: Arc<BTreeMap<String,Vec<f64>>>,
    pub hunts: Arc<Vec<HuntEntry>>,
    pub hunt_slug: Option<String>,
    /// Stable representative sprite for the confirmed hunt. This is not
    /// cleared between kills or tied to the current wild target.
    pub hunt_art_species: Option<u64>,
    pub pending_hunt_slug: Option<String>,
    pub in_center: bool,
    pub active_pokemon: Option<PokemonInfo>,
    pub pokemon_count: usize,
    pub pokemon_roster: Arc<Vec<PokemonInfo>>,
    /// True only after a complete server-provided pokemons roster, not
    /// inferred from partial pkMud updates or a previously online session.
    pub roster_ready: bool,
    pub wild: Option<WildInfo>,
    pub balls: Arc<BTreeMap<u64, u64>>,
    pub ball_names: Arc<BTreeMap<u64, String>>,
    /// Catalog quotes are alternatives by currency, not additive costs.
    pub ball_quotes: Arc<BTreeMap<u64,BallQuote>>,
    pub outland_tier:Option<u8>,
    /// Validated eight-tier table from the game's welcome catalog.
    /// Absent catalogs use the pinned upstream v1.314.2 thresholds.
    pub outland_catalog:Option<[(u32,f64);8]>,
    /// Awaiting authoritative tierOutland from the game (not socket send).
    pub pending_outland_tier:Option<u8>,
    pub outland_tier_error:Option<String>,
    pub items: Arc<BTreeMap<u64, u64>>,
    pub automation: Option<Value>,
    pub vip_until: Option<u64>,
    pub guild_boost_until: Option<u64>,
    pub server_now: Option<u64>,
    pub center_free_at: Option<u64>,
    pub diamonds: Option<u64>,
    pub orbs: Option<u64>,
    pub trainer_level: Option<u32>,
    pub trainer_xp: Option<u64>,
    pub gold: Option<u64>,
    pub bonuses: HuntBonuses,
    pub measurements: HuntMeasurements,
    pub session: HuntSessionStats,
    pub sample_quality: String,
    pub battle_kills: u64,
    pub captures: u64,
    pub catalog_ready: bool,
    pub last_error: Option<String>,
}
impl HuntView {
    pub fn outland_requirement(&self,tier:u8)->Option<(u32,f64)> {
        let index=usize::from(tier.checked_sub(1)?);
        if index>=8{return None}
        Some(self.outland_catalog.map(|rows|rows[index]).unwrap_or((
            crate::native_atlas_models::TIER_LEVELS[index],
            crate::native_atlas_models::TIER_MULTIPLIERS[index])))
    }
    pub fn unlocked_outland_tier(&self,tier:u8)->Option<f64>{
        let (level,mult)=self.outland_requirement(tier)?;
        (self.trainer_level? >= level).then_some(mult)
    }
}
/// Refuse partial, reordered or malformed tables from a remote game message.
/// Do not mix an untrusted tier threshold with an unrelated multiplier.
fn parse_outland_catalog(raw:&Value)->Option<[(u32,f64);8]>{
    let rows=raw.as_array().filter(|rows|rows.len()==8)?;
    let mut out=[(0u32,0.0f64);8];
    for (index,row) in rows.iter().enumerate(){
        let tier=row.get("tier").and_then(Value::as_u64)?;
        let level=row.get("nivel").or_else(||row.get("level"))
            .and_then(Value::as_u64).and_then(|v|u32::try_from(v).ok())?;
        let mult=row.get("mult").or_else(||row.get("multiplier"))
            .and_then(Value::as_f64)?;
        if tier!=index as u64+1 || level==0 ||
            (index>0&&level<=out[index-1].0) ||
            !mult.is_finite() || !(1.0..=100.0).contains(&mult) ||
            (index>0&&mult<out[index-1].1) {return None}
        out[index]=(level,mult);
    }
    Some(out)
}
#[derive(Clone, Debug)]
struct Sample { at: Instant, trainer: Option<u64>, pokemon: Option<u64>, gold: Option<u64>, kills: u64 }
/// Server-confirmed reward receipts, independent of the cadence of estado.
/// Manager-style battle statistics must not depend on a full state refresh.
#[derive(Clone, Debug)]
struct BattleSample {
    at: Instant, trainer: u64, pokemon: u64, gold: u64, kills: u64,
    trainer_receipts: u64, pokemon_receipts: u64, gold_receipts: u64,
}
const WINDOW: Duration = Duration::from_secs(600);
const MIN_MEASUREMENT: f64 = 20.0;

pub struct HuntIntel {
    pub view: HuntView,
    pokemon: BTreeMap<u64, PokemonInfo>,
    roster_dirty: bool,
    active_id: Option<u64>,
    samples: VecDeque<Sample>,
    battle_samples: VecDeque<BattleSample>,
    battle_trainer: u64,
    battle_pokemon: u64,
    battle_trainer_receipts: u64,
    battle_pokemon_receipts: u64,
    battle_gold: u64,
    battle_gold_receipts: u64,
    last_battle_at: Option<Instant>,
    last_combat_at: Option<Instant>,
    session_kills: u64,
    session_start: Instant,
    recent_drops: VecDeque<(String,u64)>,
    pending_at: Option<Instant>,
    pending_outland_at: Option<Instant>,
    boost_multipliers: BTreeMap<String,f64>,
    geo_samples: BTreeMap<String,Vec<Vec<f64>>>,
    geo_wave: Option<(String,Instant,Vec<f64>)>,
    geo_alive: BTreeMap<u64,bool>,
    geo_hero: Option<(f64,f64)>,
}
impl HuntIntel {
    pub fn new() -> Self {
        Self {
            view: HuntView::default(), pokemon: BTreeMap::new(),
            roster_dirty: false, active_id: None,
            samples: VecDeque::new(), battle_samples: VecDeque::new(),
            battle_trainer: 0, battle_pokemon: 0,
            battle_trainer_receipts: 0, battle_pokemon_receipts: 0,
            battle_gold: 0, battle_gold_receipts: 0,
            last_battle_at: None, last_combat_at: None, session_kills: 0,
            session_start: Instant::now(),
            recent_drops: VecDeque::new(), pending_at: None,
            pending_outland_at: None,
            boost_multipliers:BTreeMap::new(),
            geo_samples:BTreeMap::new(),
            geo_wave:None,geo_alive:BTreeMap::new(),geo_hero:None,
        }
    }
    /// Produce a cheap UI snapshot: roster data is cloned only on the
    /// 1-second UI publish cadence, never once per high-rate game delta.
    pub fn snapshot(&mut self) -> HuntView {
        self.finish_geo_wave(Instant::now());
        self.expire_outland_request(Instant::now());
        self.view.session.duration_secs=self.session_start.elapsed().as_secs();
        self.view.measurements.last_battle_age_secs=self.last_battle_at
            .map(|at|at.elapsed().as_secs());
        self.view.measurements.combat_event_age_secs=self.last_combat_at
            .map(|at|at.elapsed().as_secs());
        if self.view.measurements.last_battle_age_secs.is_some_and(|age|age>180) {
            // Keeping the previous 10-minute average on screen when combat has
            // stopped makes a stale prediction look like live measurement.
            self.view.measurements.valid_trainer=false;
            self.view.measurements.valid_pokemon=false;
            self.view.measurements.valid_gold=false;
            self.view.measurements.valid_gross_gold=false;
            self.view.sample_quality="No confirmed kill in three minutes; measured pace expired".into();
        }
        if self.roster_dirty {
            self.view.pokemon_roster = Arc::new(self.pokemon.values().cloned().collect());
            self.roster_dirty = false;
        }
        self.view.clone()
    }
    pub fn reset_session(&mut self, now:Instant) {
        self.session_start=now;
        self.recent_drops.clear();
        self.view.session=HuntSessionStats::default();
    }
    /// Track an Outland selection only after validation, before socket write.
    /// The UI never overwrites outland_tier optimistically: a matching server
    /// state is the only confirmation that the setting took effect.
    pub fn request_outland_tier(&mut self,tier:u8,now:Instant)->Result<(),String>{
        if !(1..=8).contains(&tier){
            return Err("Unknown Outland tier".into());
        }
        if self.view.outland_tier.is_none(){
            return Err("Waiting for server-reported Outland tier".into());
        }
        let level=self.view.trainer_level
            .ok_or("Trainer level not reported by the game")?;
        let threshold=self.view.outland_requirement(tier)
            .ok_or("Unknown Outland tier")?.0;
        if level<threshold {
            return Err(format!(
                "Outland {tier} requires trainer level {threshold}"
            ));
        }
        if self.view.pending_outland_tier.is_some(){
            return Err("An Outland tier change is already awaiting confirmation".into());
        }
        if self.view.outland_tier==Some(tier){
            return Err("Outland tier is already active".into());
        }
        self.view.pending_outland_tier=Some(tier);
        self.pending_outland_at=Some(now);
        self.view.outland_tier_error=None;
        Ok(())
    }
    pub fn outland_command_failed(&mut self,message:&str){
        self.view.pending_outland_tier=None;
        self.pending_outland_at=None;
        self.view.outland_tier_error=Some(message.into());
    }
    fn expire_outland_request(&mut self,now:Instant){
        if self.pending_outland_at.is_some_and(|at|
            now.saturating_duration_since(at)>Duration::from_secs(6)){
            self.outland_command_failed(
                "Outland tier change was not confirmed by the game. Retry."
            );
        }
    }
    pub fn request_hunt(&mut self, slug: &str, now: Instant) -> Result<(), String> {
        if !self.view.catalog_ready || !self.view.hunts.iter().any(|h| h.slug == slug) {
            return Err("Hunt not present in authoritative account catalog".into());
        }
        self.view.pending_hunt_slug = Some(slug.to_owned());
        self.pending_at = Some(now);
        Ok(())
    }
    /// Only call after an explicit server sale receipt for this Pokémon.
    /// The server's next full roster is still the final source of truth.
    pub fn confirm_pokemon_sale(&mut self, id:u64) {
        if self.pokemon.remove(&id).is_some() {
            self.roster_dirty=true;
            self.view.pokemon_count=self.pokemon.len();
            if self.active_id==Some(id) {
                self.active_id=None;
                self.view.active_pokemon=None;
            }
        }
    }
    pub fn command_failed(&mut self, error: &str) {
        self.view.pending_hunt_slug = None;
        self.pending_at = None;
        self.view.last_error = Some(error.to_owned());
    }
    pub fn on_disconnect(&mut self) {
        self.view.pending_outland_tier=None;
        self.pending_outland_at=None;
        self.view.outland_tier=None;
        self.view.outland_catalog=None;
        self.view.outland_tier_error=None;
        self.view.pending_hunt_slug = None;
        self.pending_at = None;
        self.samples.clear();
        self.battle_samples.clear();
        self.battle_trainer = 0;
        self.battle_pokemon = 0;
        self.battle_trainer_receipts = 0;
        self.battle_pokemon_receipts = 0;
        self.battle_gold = 0;
        self.battle_gold_receipts = 0;
        self.session_kills = 0; // Reset the per-hunt *rate* baseline, not session totals.
        self.view.battle_kills = 0;
        self.last_battle_at = None;
        self.last_combat_at = None;
        self.view.wild = None;
        self.view.roster_ready = false;
        self.view.measurements = HuntMeasurements::default();
        self.view.sample_quality = "Reconnecting; collecting fresh measurements".into();
    }
    fn adopt_hunt(&mut self, slug: &str, now: Instant) {
        if self.view.hunt_slug.as_deref() != Some(slug) {
            self.view.hunt_slug = Some(slug.to_owned());
            self.view.hunt_art_species = self.view.hunts.iter()
                .find(|hunt| hunt.slug == slug)
                .and_then(representative_hunt_species)
                .or_else(|| species_id_from_hunt_name(slug));
            // A confirmed new location supersedes the previous Center state.
            self.view.in_center = false;
            self.samples.clear();
            self.battle_samples.clear();
            self.battle_trainer = 0;
            self.battle_pokemon = 0;
            self.battle_trainer_receipts = 0;
            self.battle_pokemon_receipts = 0;
        self.battle_gold = 0;
        self.battle_gold_receipts = 0;
            self.last_battle_at = None;
            self.last_combat_at = None;
            self.view.wild = None;
            self.battle_samples.push_back(BattleSample {
                at: now, trainer: 0, pokemon: 0, gold: 0, kills: 0,
                trainer_receipts: 0, pokemon_receipts: 0, gold_receipts: 0,
            });
            self.session_kills = 0;
            // Like Manager, keep accumulated session loot and XP across
            // confirmed hunt switches. Only this hunt's *rate* resets.
            self.view.battle_kills = 0;
            self.view.captures = 0;
            self.view.measurements = HuntMeasurements::default();
            self.view.sample_quality = "Warming up after hunt switch".into();
        }
        if self.view.pending_hunt_slug.as_deref() == Some(slug) {
            self.view.pending_hunt_slug = None;
            self.pending_at = None;
            self.view.last_error = None;
        }
    }
    fn num(record: &Value, key: &str) -> Option<u64> { record.get(key).and_then(Value::as_u64) }
    /// Only full waves count as geometry. Spawn slots, not active wild encounters,
    /// provide movement distances. No monitoring state or hunt counters are mutated.
    fn finish_geo_wave(&mut self, now:Instant) {
        let finished=self.geo_wave.as_ref().is_some_and(|(_,since,_)|
            now.saturating_duration_since(*since)>=Duration::from_secs(4));
        if !finished {return}
        let Some((slug,_,mut distances))=self.geo_wave.take() else{return};
        let Some(hunt)=self.view.hunts.iter().find(|h|h.slug==slug) else{return};
        if hunt.level>150 {return} // Per-slot replacement has no full wave.
        let expected=if hunt.total_spawns>0 {hunt.total_spawns as f64}
            else {hunt.species.iter().map(|s|s.weight.max(0.0)).sum::<f64>().min(16.0)};
        let threshold=(expected*0.8).ceil().max(6.0);
        if (distances.len() as f64)<threshold {return}
        distances.retain(|d|d.is_finite()&&*d>=0.0);
        if distances.is_empty(){return}
        distances.sort_by(f64::total_cmp);
        let histories=self.geo_samples.entry(slug.clone()).or_default();
        histories.push(distances);
        if histories.len()>6 {histories.remove(0);}
        let all=histories.iter().flat_map(|v|v.iter().copied()).collect();
        let mut next=(*self.view.geo_distances).clone();
        next.insert(slug,all);
        self.view.geo_distances=Arc::new(next);
    }
    fn field_geo_init(&mut self, frame:&Value) {
        self.geo_wave=None;
        self.geo_alive.clear();
        self.geo_hero=frame.get("heroi").and_then(|h|
            Some((h.get("c")?.as_f64()?,h.get("r")?.as_f64()?)));
        if let Some(mobs)=frame.get("mobs").and_then(Value::as_array){
            for mob in mobs{
                if Self::is_field_wild(mob){
                    if let Some(slot)=Self::num(mob,"s"){
                        self.geo_alive.insert(slot,mob.get("x").and_then(Value::as_bool)!=Some(true));
                    }
                }
            }
        }
    }
    fn is_field_wild(mob:&Value)->bool {
        !["bn","cura","tm","depot","tr"].iter()
            .any(|k|mob.get(*k).is_some_and(|v|!v.is_null()&&v!=false))
            && mob.get("ti").is_none()
    }
    fn field_geo_delta(&mut self, frame:&Value, now:Instant) {
        self.finish_geo_wave(now);
        if let Some(hero)=frame.get("heroi") {
            if let (Some(c),Some(r))=(hero.get("c").and_then(Value::as_f64),
                hero.get("r").and_then(Value::as_f64)) {
                self.geo_hero=Some((c,r));
            }
        }
        let slug=self.view.field_slug.as_deref()
            .filter(|slug|self.view.hunts.iter().any(|h|h.slug==*slug));
        for mob in frame.get("mobs").and_then(Value::as_array)
            .into_iter().flatten(){
            if !Self::is_field_wild(mob){continue}
            let Some(slot)=Self::num(mob,"s") else{continue};
            let dead=mob.get("x").and_then(Value::as_bool)==Some(true);
            let was_alive=self.geo_alive.insert(slot,!dead);
            if !dead && was_alive!=Some(true) {
                if let (Some(slug),Some((hc,hr)),Some(c),Some(r))=(
                    slug,self.geo_hero,mob.get("c").and_then(Value::as_f64),
                    mob.get("r").and_then(Value::as_f64)) {
                    let dist=(c-hc).hypot(r-hr);
                    if dist.is_finite() && dist<=300.0{
                        if self.geo_wave.as_ref().is_none_or(|(s,_,_)|s!=slug){
                            self.geo_wave=Some((slug.to_owned(),now,Vec::new()));
                        }
                        if let Some((_,_,samples))=self.geo_wave.as_mut(){
                            if samples.len()<32 {samples.push((dist*10.0).round()/10.0);}
                        }
                    }
                }
            }
        }
        for slot in frame.get("fora").and_then(Value::as_array)
            .into_iter().flatten().filter_map(Value::as_u64){
            self.geo_alive.remove(&slot);
        }
    }
    fn set_pokemon(&mut self, value: &Value) {
        let Some(id) = Self::num(value, "id") else { return };
        self.roster_dirty = true;
        let existing = self.pokemon.entry(id).or_insert_with(|| PokemonInfo {
            id, ..PokemonInfo::default()
        });
        if let Some(name) = value.get("nome").or_else(|| value.get("name")).and_then(Value::as_str) {
            existing.name = name.to_owned();
        }
        // Game roster frames may use pokeId while wild frames use speciesId.
        // Both are Pokédex IDs; the owned id is an account-local instance ID.
        if let Some(n) = Self::num(value, "speciesId")
            .or_else(|| Self::num(value, "pokeId")) {
            existing.species_id = Some(n);
        }
        if let Some(n) = Self::num(value, "level") { existing.level = n.min(u32::MAX as u64) as u32; }
        if let Some(n) = Self::num(value, "xp") { existing.xp = Some(n); }
        if let Some(n) = Self::num(value, "xpNivel") { existing.xp_level = Some(n); }
        if let Some(n) = Self::num(value, "xpProximo") { existing.xp_next = Some(n); }
        if let Some(n) = Self::num(value, "hp") { existing.hp = Some(n); }
        if let Some(n) = Self::num(value, "maxHp") { existing.max_hp = Some(n); }
        if let Some(raw) = value.get("heldItemId") {
            existing.held_item_id = raw.as_u64();
        }
        if let Some(n) = Self::num(value, "poder") { existing.power = Some(n); }
        if let Some(raw) = value.get("quality") { existing.quality = raw.as_f64(); }
        if let Some(raw) = value.get("nota") { existing.grade = raw.as_f64(); }
        if let Some(n) = Self::num(value, "potencia") { existing.potency = Some(n); }
        if let Some(raw) = value.get("looktype") { existing.looktype = raw.as_u64(); }
        if let Some(raw) = value.get("lookShiny") { existing.shiny_looktype = raw.as_u64(); }
        if let Some(b) = value.get("shiny").and_then(Value::as_bool) { existing.shiny = b; }
        if let Some(types) = value.get("tipos").and_then(Value::as_array) {
            existing.types = types.iter().filter_map(Value::as_str).map(str::to_owned).collect();
        }
        if let Some(ivs) = value.get("ivs") {
            for (i, field) in ["hp", "atk", "def", "spAtk", "spDef", "speed"].iter().enumerate() {
                if let Some(raw) = ivs.get(field) {
                    existing.ivs[i] = raw.as_u64();
                }
            }
        }
        if let Some(raw) = value.get("stats").and_then(Value::as_object) {
            let fields = ["hp", "atk", "def", "spAtk", "spDef", "speed"];
            let numbers = fields.map(|key|raw.get(key).and_then(Value::as_f64));
            if numbers.iter().all(|n|n.is_some_and(|v|v.is_finite()&&v>0.0)) {
                existing.battle_stats = Some(numbers.map(|n|n.unwrap_or(1.0)));
            }
        }
        if let Some(v) = value.get("tmAoe").or_else(||value.get("tm_aoe")) {
            existing.tm_aoe = v.as_bool()==Some(true) || v.as_u64()==Some(1)
                || v.as_str()==Some("true");
        }
        if let Some(v) = value.get("tmElemental") {
            existing.tm_elemental = v.as_str().map(str::to_owned)
                .or_else(||v.as_u64().map(|n|n.to_string()));
        }
    }
    /// Merge full or partial server-confirmed wild state. A missing slot is
    /// not equivalent to an absent encounter; a new slot must not inherit
    /// the previous opponent's name/species.
    fn parse_wild(value: &Value, previous: Option<&WildInfo>) -> Option<WildInfo> {
        if !value.is_object() { return None; }
        let given_slot = Self::num(value, "slot");
        let same = previous.filter(|old|
            given_slot.is_none_or(|slot| slot == old.slot));
        let name = value.get("nome").or_else(|| value.get("name"))
            .and_then(Value::as_str).map(str::trim)
            .filter(|name| !name.is_empty()).map(str::to_owned)
            .or_else(|| same.map(|old| old.name.clone()))
            .unwrap_or_default();
        let species_id = Self::num(value, "speciesId")
            .or_else(|| Self::num(value, "pokeId"))
            .or_else(|| same.and_then(|old| old.species_id));
        if name.is_empty() && species_id.is_none() { return None; }
        Some(WildInfo {
            slot: given_slot.or_else(|| same.map(|old| old.slot)).unwrap_or(0),
            name,
            species_id,
            level: Self::num(value, "level")
                .or_else(|| same.map(|old| old.level as u64))
                .unwrap_or(0).min(u32::MAX as u64) as u32,
            hp: Self::num(value, "hp")
                .or_else(|| same.map(|old| old.hp)).unwrap_or(0),
            max_hp: Self::num(value, "maxHp")
                .or_else(|| same.map(|old| old.max_hp)).unwrap_or(0),
        })
    }
    fn merge_state(&mut self, state: &Value, now: Instant) {
        // Switch hunt before applying its opponent. On a welcome, the same
        // state already includes the new wild encounter; do not clear it after.
        if let Some(slug) = state.get("huntSlug").and_then(Value::as_str) {
            self.adopt_hunt(slug, now);
        }
        if let Some(raw)=state.get("tierOutland")
            .or_else(||state.get("outlandTier"))
            .or_else(||state.get("outland_tier")) {
            self.view.outland_tier=raw.as_u64()
                .filter(|tier|(1..=8).contains(tier)).map(|tier|tier as u8);
            if self.view.outland_tier.is_some() &&
                self.view.pending_outland_tier==self.view.outland_tier{
                self.view.pending_outland_tier=None;
                self.pending_outland_at=None;
                self.view.outland_tier_error=None;
            }
        }
        if let Some(nick) = state.get("nick").and_then(Value::as_str)
            .map(str::trim).filter(|nick|!nick.is_empty()) {
            self.view.account_nick = Some(nick.to_owned());
        }
        if let Some(value) = Self::num(state, "diamonds") { self.view.diamonds = Some(value); }
        if let Some(value) = Self::num(state, "orbs") { self.view.orbs = Some(value); }
        if let Some(value) = Self::num(state, "servidorAgora") { self.view.server_now = Some(value); }
        if let Some(value) = Self::num(state, "centroLivreEm") { self.view.center_free_at = Some(value); }
        if let Some(value)=state.get("vipAte") {
            self.view.vip_until=value.as_u64();
            self.view.bonuses.vip_known=true;
        }
        if let Some(data) = state.get("automation") {
            if data.is_object() {
                let mut merged = self.view.automation.take()
                    .filter(Value::is_object).unwrap_or_else(||serde_json::json!({}));
                if let (Some(a), Some(b)) = (merged.as_object_mut(),data.as_object()) {
                    for (k,v) in b { a.insert(k.clone(),v.clone()); }
                }
                self.view.automation = Some(merged);
            }
        }
        for (key, target) in [
            ("balls", true), ("items", false)
        ] {
            if let Some(raw) = state.get(key).and_then(Value::as_object) {
                let inventory = raw.iter().filter_map(|(id,count)|{
                    Some((id.parse::<u64>().ok()?,count.as_u64()?))
                }).collect::<BTreeMap<_,_>>();
                if target { self.view.balls = Arc::new(inventory); }
                else { self.view.items = Arc::new(inventory); }
            }
        }
        if let Some(wild) = state.get("selvagem") {
            let previous = self.view.wild.clone();
            self.view.wild = Self::parse_wild(wild, previous.as_ref());
            // The first confirmed encounter also establishes combat activity.
            if self.view.wild.as_ref().is_some_and(|current| {
                previous.as_ref().is_none_or(|old|
                    old.slot != current.slot ||
                    old.species_id != current.species_id ||
                    old.name != current.name ||
                    old.hp != current.hp ||
                    old.max_hp != current.max_hp)
            }) {
                self.last_combat_at = Some(now);
            }
        }
        if let Some(level) = Self::num(state, "level") {
            self.view.trainer_level = Some(level.min(u32::MAX as u64) as u32);
        }
        if let Some(xp) = Self::num(state, "xp") { self.view.trainer_xp = Some(xp); }
        if let Some(gold) = Self::num(state, "gold") { self.view.gold = Some(gold); }
        if let Some(raw_id) = state.get("activeId") {
            let new_id=raw_id.as_u64().filter(|id|*id>0);
            if self.active_id != new_id {
                // Switching or clearing active Pokémon invalidates observed
                // Pokémon XP comparisons. An explicit null must clear it.
                self.samples.clear();
                self.battle_samples.clear();
                self.battle_trainer=0;
                self.battle_pokemon=0;
                self.battle_trainer_receipts=0;
                self.battle_pokemon_receipts=0;
                self.battle_gold=0;
                self.battle_gold_receipts=0;
                self.last_battle_at=None;
                self.last_combat_at=None;
                self.battle_samples.push_back(BattleSample{
                    at:now,trainer:0,pokemon:0,gold:0,kills:self.session_kills,
                    trainer_receipts:0,pokemon_receipts:0,gold_receipts:0,
                });
                self.view.measurements = HuntMeasurements::default();
            }
            self.active_id=new_id;
        }
        if let Some(center) = state.get("noCentro").and_then(Value::as_bool) {
            self.view.in_center = center;
            if center { self.view.wild = None; }
        }
        if let Some(pokemons) = state.get("pokemons").and_then(Value::as_array) {
            self.view.roster_ready = true;
            // Even an empty authoritative depot must invalidate the previously
            // published Arc roster; otherwise sold Pokémon remain visible.
            self.pokemon.clear();
            self.roster_dirty=true;
            for p in pokemons { self.set_pokemon(p); }
        }
        if let Some(patches) = state.get("pkMud").and_then(Value::as_array) {
            for p in patches { self.set_pokemon(p); }
        }
        self.view.pokemon_count = self.pokemon.len();
        self.view.active_pokemon = self.active_id.and_then(|id| self.pokemon.get(&id).cloned());
        if let Some(event) = state.get("evento") {
            if let Some(n) = event.get("xpTreinadorPct").and_then(Value::as_f64) {
                self.view.bonuses.trainer_event_pct = n;
                self.view.bonuses.trainer_event_known = true;
            }
            if let Some(n) = event.get("xpPokemonPct").and_then(Value::as_f64) {
                self.view.bonuses.pokemon_event_pct = n;
                self.view.bonuses.pokemon_event_known = true;
            }
        }
        if let Some(n) = state.get("guildBonusPct").and_then(Value::as_f64) {
            self.view.bonuses.guild_pct = n;
            self.view.bonuses.guild_known = true;
        }
        if let Some(twitch) = state.get("twitch") {
            if let Some(n) = twitch.get("pctAtual").or_else(|| twitch.get("pct"))
                .and_then(Value::as_f64) {
                self.view.bonuses.twitch_pct = n;
                self.view.bonuses.twitch_known = true;
            }
        }
        // Store XP boosts, Kick and catch-up multipliers are independent and
        // stack with the already tracked VIP, guild, Twitch and event bonuses.
        let clock=Self::num(state,"servidorAgora")
            .or(self.view.server_now).unwrap_or(0);
        let shop=state.get("loja");
        for (key,dest) in [
            ("xp",&mut self.view.bonuses.shop_trainer_xp_multiplier),
            ("pokexp",&mut self.view.bonuses.shop_pokemon_xp_multiplier)
        ]{
            let until=shop.and_then(|v|v.get("boosts"))
                .and_then(|v|v.get(key)).and_then(Value::as_u64).unwrap_or(0);
            *dest=if clock>0 && until>clock {
                self.boost_multipliers.get(key).copied().unwrap_or(1.0)
            }else{1.0};
        }
        if let Some(late)=shop.and_then(|v|v.get("atrasados")) {
            let mult=late.get("mult").and_then(Value::as_f64).unwrap_or(1.0);
            let target=late.get("alvo").and_then(Value::as_u64).unwrap_or(0);
            let level=self.view.trainer_level.unwrap_or(0);
            self.view.bonuses.delayed_xp_multiplier=if mult.is_finite()
                && mult>1.0 && level>0 && (level as u64)<target {mult}
                else {1.0};
        }
        if let Some(kick)=state.get("kick") {
            let expiry=kick.get("bonus").and_then(|v|v.get("xp"))
                .and_then(Value::as_u64).unwrap_or(0);
            let pct=kick.get("pct").and_then(Value::as_f64).unwrap_or(0.0);
            self.view.bonuses.kick_xp_pct=if clock>0&&expiry>clock
                &&pct.is_finite()&&pct>0.0{pct}else{0.0};
        }
        if let Some(raw)=state.get("guild").and_then(|g|g.get("boostAte")){
            self.view.guild_boost_until=raw.as_u64();
            self.view.bonuses.guild_boost_known=true;
        }
        let server_now=self.view.server_now.unwrap_or(0);
        self.view.bonuses.guild_boost_active=server_now>0 &&
            self.view.guild_boost_until.is_some_and(|until|until>server_now);
        self.view.bonuses.vip_active=server_now>0 &&
            self.view.vip_until.is_some_and(|until|until>server_now);
        self.sample(now);
    }
    fn catalog(&mut self, list: &[Value]) {
        let mut hunts = Vec::new();
        for item in list {
            let Some(slug) = item.get("slug").or_else(|| item.get("s"))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty() && s.len() <= 128) else { continue };
            // Support full game entries and compact [species ID, weight]
            // entries; a partial catalog must not suppress hunt artwork.
            let mut species = item.get("especies").or_else(|| item.get("e"))
                .and_then(Value::as_array)
                .map(|entries| entries.iter().filter_map(|entry| {
                    let (id, weight) = if let Some(pair) = entry.as_array() {
                        (pair.first()?.as_u64()?,
                         pair.get(1).and_then(Value::as_f64).unwrap_or(1.0))
                    } else if let Some(id) = entry.as_u64() {
                        (id, 1.0)
                    } else {
                        (entry.get("pokeId").or_else(|| entry.get("speciesId"))
                            .or_else(|| entry.get("id")).and_then(Value::as_u64)?,
                         entry.get("pontos").or_else(|| entry.get("weight"))
                            .or_else(|| entry.get("peso")).and_then(Value::as_f64)
                            .unwrap_or(1.0))
                    };
                    (id > 0 && weight.is_finite())
                        .then_some(HuntSpecies { id, weight: weight.max(0.0) })
                }).collect::<Vec<_>>()).unwrap_or_default();
            // Some single-species hunts expose the ID directly rather than
            // as an array. Retain that explicit ID instead of name guessing.
            if species.is_empty() {
                let id = item.get("pokeId").or_else(|| item.get("speciesId"))
                    .or_else(|| item.get("especieId"))
                    .or_else(|| item.get("especie")).and_then(Value::as_u64)
                    .or_else(|| item.get("especie")
                        .and_then(|entry| entry.get("pokeId"))
                        .and_then(Value::as_u64));
                if let Some(id) = id.filter(|id| *id > 0) {
                    species.push(HuntSpecies { id, weight: 1.0 });
                }
            }
            hunts.push(HuntEntry {
                slug: slug.to_owned(),
                name: item.get("nome").or_else(||item.get("name"))
                    .or_else(||item.get("n"))
                    .and_then(Value::as_str).unwrap_or(slug).to_owned(),
                area: item.get("area").or_else(||item.get("a"))
                    .and_then(Value::as_str).unwrap_or("").to_owned(),
                region: item.get("regiao").or_else(||item.get("region"))
                    .or_else(||item.get("r"))
                    .and_then(Value::as_str).unwrap_or("").to_owned(),
                level: Self::num(item, "nivel")
                    .or_else(||Self::num(item,"level"))
                    .or_else(||Self::num(item,"nv"))
                    .unwrap_or(0).min(u32::MAX as u64) as u32,
                total_spawns: Self::num(item, "totalSpawns").unwrap_or(0).min(u32::MAX as u64) as u32,
                looktype: Self::num(item, "looktype"),
                species,
            });
        }
        self.view.catalog_ready = true;
        self.view.hunts = Arc::new(hunts);
        if let Some(slug) = self.view.hunt_slug.as_deref() {
            let resolved = self.view.hunts.iter()
                .find(|hunt| hunt.slug == slug)
                .and_then(representative_hunt_species)
                .or_else(|| species_id_from_hunt_name(slug));
            self.view.hunt_art_species = resolved.or(self.view.hunt_art_species);
        }
    }
    #[cfg(test)]
    pub fn ingest(&mut self, raw: &str, now: Instant) {
        let Ok(frame) = serde_json::from_str::<Value>(raw) else { return };
        self.ingest_value(&frame, now);
    }
    pub fn ingest_value(&mut self, frame: &Value, now: Instant) {
        match frame.get("t").and_then(Value::as_str) {
            Some("welcome") => {
                if let Some(boosts)=frame.get("tiposBoost").and_then(Value::as_array){
                    self.boost_multipliers=boosts.iter().filter_map(|row|{
                        let key=row.get("chave").and_then(Value::as_str)?;
                        let mult=row.get("mult").and_then(Value::as_f64)?;
                        (mult.is_finite()&&mult>=1.0).then_some((key.to_owned(),mult))
                    }).collect();
                }
                self.view.outland_catalog=frame.get("catalogoOutlandTiers")
                    .or_else(||frame.get("outlandTiers"))
                    .and_then(parse_outland_catalog);
                if let Some(catalog) = frame.get("catalogoBolas").and_then(Value::as_array) {
                    self.view.ball_names=Arc::new(catalog.iter().filter_map(|ball| {
                        Some((ball.get("id").and_then(Value::as_u64)?,
                            ball.get("nome").or_else(||ball.get("name"))
                                .and_then(Value::as_str)?.to_owned()))
                    }).collect());
                    self.view.ball_quotes=Arc::new(catalog.iter().filter_map(|ball|{
                        let id=ball.get("id").and_then(Value::as_u64)?;
                        let num=|keys:&[&str]|keys.iter().find_map(|key|
                            ball.get(*key).and_then(Value::as_f64)
                                .filter(|v|v.is_finite()&&*v>=0.0));
                        Some((id,BallQuote{
                            efficiency:num(&["catchRate","efficiency"]).filter(|v|*v>0.0),
                            gold_price:num(&["priceGold","precoGold","goldPrice"]),
                            diamond_price:num(&["priceDiamond","precoDiamantes","diamondPrice"]),
                        }))
                    }).collect());
                }
                if let Some(hunts) = frame.get("hunts").and_then(Value::as_array) {
                    self.catalog(hunts);
                }
                if let Some(state) = frame.get("estado") { self.merge_state(state, now); }
            }
            Some("estado") => {
                if let Some(state) = frame.get("estado") { self.merge_state(state, now); }
            }
            Some("campo.init") => {
                self.field_geo_init(frame);
                if let Some(map) = frame.get("mapa").and_then(Value::as_str) {
                    self.view.field_map = Some(map.to_owned());
                }
                if let Some(slug) = frame.get("slug").and_then(Value::as_str) {
                    self.view.field_slug = Some(slug.to_owned());
                    // campo.init is a second server-confirmed location signal.
                    // Accept it as the hunt only when it matches an unlocked
                    // catalog slug; arbitrary maps are not necessarily hunts.
                    if self.view.hunts.iter().any(|hunt| hunt.slug == slug) {
                        self.adopt_hunt(slug, now);
                        self.view.in_center = false;
                    }
                }
                if let Some(timestamp) = Self::num(&frame, "ts") {
                    self.view.field_init_timestamp = Some(timestamp);
                }
            }
            Some("campo") => self.field_geo_delta(frame,now),
            Some("batalha") => {
                if let Some(events) = frame.get("ev").and_then(Value::as_array) {
                    for e in events {
                        // Navigation/status events do not establish fighting.
                        // All other confirmed battle events do, even before a kill.
                        if e.get("k").and_then(Value::as_str).is_some_and(|kind|
                            !matches!(kind, "hunt" | "centro")
                        ) {
                            self.last_combat_at = Some(now);
                        }
                        match e.get("k").and_then(Value::as_str) {
                            Some("hunt") => if let Some(slug) = e.get("slug").and_then(Value::as_str) {
                                self.adopt_hunt(slug, now);
                                self.view.in_center = false;
                            },
                            Some("centro") => {
                                self.view.in_center = true;
                                self.view.wild = None;
                            },
                            Some("morte") if e.get("quem").and_then(Value::as_str) == Some("selvagem") => {
                                self.session_kills = self.session_kills.saturating_add(1);
                                self.last_battle_at=Some(now);
                                self.last_combat_at=Some(now);
                                self.view.battle_kills = self.session_kills;
                                let session=&mut self.view.session;
                                session.kills=session.kills.saturating_add(1);
                                // Battle receipts are authoritative even when the
                                // game sends no subsequent estado/pkMud delta.
                                if let Some(xp) = Self::num(e, "xpTreinador") {
                                    self.battle_trainer=self.battle_trainer.saturating_add(xp);
                                    self.battle_trainer_receipts=
                                        self.battle_trainer_receipts.saturating_add(1);
                                }
                                if let Some(xp) = Self::num(e, "xpPokemon") {
                                    self.battle_pokemon=self.battle_pokemon.saturating_add(xp);
                                    self.battle_pokemon_receipts=
                                        self.battle_pokemon_receipts.saturating_add(1);
                                }
                                // "ouro" is the actual kill payout, including the new
                                // converted loot bonus; "ouroVendaAuto" is a sale, not raw gold.
                                // Require a receipt for every kill in the observation window.
                                if let Some(gold) = Self::num(e, "ouro") {
                                    self.battle_gold=self.battle_gold.saturating_add(gold);
                                    self.battle_gold_receipts=
                                        self.battle_gold_receipts.saturating_add(1);
                                }
                                if self.battle_samples.is_empty() {
                                    self.battle_samples.push_back(BattleSample {
                                        at: now, trainer: 0, pokemon: 0, gold: 0, kills: 0,
                                        trainer_receipts: 0, pokemon_receipts: 0, gold_receipts: 0,
                                    });
                                }
                                self.battle_samples.push_back(BattleSample {
                                    at: now, trainer: self.battle_trainer,
                                    pokemon: self.battle_pokemon, gold: self.battle_gold,
                                    kills: self.session_kills,
                                    trainer_receipts: self.battle_trainer_receipts,
                                    pokemon_receipts: self.battle_pokemon_receipts,
                                    gold_receipts: self.battle_gold_receipts,
                                });
                                for (key,dest) in [
                                    ("xpTreinador",&mut session.trainer_xp),
                                    ("xpPokemon",&mut session.pokemon_xp),
                                    ("ouro",&mut session.combat_gold),
                                    ("ouroVendaAuto",&mut session.auto_sale_gold)
                                ] {
                                    *dest=dest.saturating_add(Self::num(e,key).unwrap_or(0));
                                }
                                if let Some(drops)=e.get("drops").and_then(Value::as_array) {
                                    for drop in drops {
                                        let Some(name)=drop.get("nome").and_then(Value::as_str)
                                            .map(str::trim).filter(|n|!n.is_empty()) else {continue};
                                        let quantity=Self::num(drop,"ganho").filter(|n|*n>0)
                                            .or_else(||Self::num(drop,"qtd")).unwrap_or(0);
                                        if quantity==0 {continue}
                                        let total=session.acquired.entry(name.to_owned()).or_default();
                                        *total=total.saturating_add(quantity);
                                        self.recent_drops.push_front((name.to_owned(),quantity));
                                        self.recent_drops.truncate(30);
                                    }
                                    session.recent_drops=self.recent_drops.iter().cloned().collect();
                                }
                            }
                            Some("bola") => {
                                let session=&mut self.view.session;
                                let captured=e.get("sucesso").and_then(Value::as_bool)==Some(true);
                                if let Some(ball_id)=Self::num(e,"ballId") {
                                    let total=session.balls_used.entry(ball_id).or_default();
                                    *total=total.saturating_add(1);
                                    if captured {
                                        session.ball_captures=session.ball_captures.saturating_add(1);
                                    }
                                }
                                let shiny=e.get("shiny").and_then(Value::as_bool)==Some(true);
                                if shiny {session.shiny_seen=session.shiny_seen.saturating_add(1);}
                                if captured {
                                    self.view.captures=self.view.captures.saturating_add(1);
                                    session.captures=session.captures.saturating_add(1);
                                    if shiny {session.shiny_captured=session.shiny_captured.saturating_add(1);}
                                }
                            }
                            Some("fugiu") if e.get("shiny").and_then(Value::as_bool)==Some(true) => {
                                self.view.session.shiny_seen=self.view.session.shiny_seen.saturating_add(1);
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
        // Update battle-derived rates on every relevant server frame. Without
        // this, the old dashboard showed zero/blank XP until an estado arrived.
        self.refresh_battle_rates(now);
        self.expire_outland_request(now);
        if self.pending_at.is_some_and(|at| now.saturating_duration_since(at) > Duration::from_secs(20)) {
            self.command_failed("Hunt switch not confirmed by game server");
        }
    }
    fn refresh_battle_rates(&mut self, now: Instant) {
        while self.battle_samples.len() > 1 &&
            self.battle_samples.front().is_some_and(|sample|
                now.saturating_duration_since(sample.at) > WINDOW) {
            self.battle_samples.pop_front();
        }
        let (Some(first), Some(last)) =
            (self.battle_samples.front(), self.battle_samples.back()) else { return };
        let seconds = last.at.saturating_duration_since(first.at).as_secs_f64();
        if seconds < MIN_MEASUREMENT || last.kills <= first.kills { return; }
        let scale = 3600.0 / seconds;
        let kills = last.kills.saturating_sub(first.kills);
        let measurements = &mut self.view.measurements;
        measurements.seconds = seconds;
        measurements.kills = kills;
        measurements.kills_h = kills as f64 * scale;
        measurements.valid_trainer =
            last.trainer_receipts.saturating_sub(first.trainer_receipts)==kills;
        measurements.valid_pokemon =
            last.pokemon_receipts.saturating_sub(first.pokemon_receipts)==kills;
        if measurements.valid_trainer {
            measurements.trainer_xp_h =
                last.trainer.saturating_sub(first.trainer) as f64 * scale;
        }
        if measurements.valid_pokemon {
            measurements.pokemon_xp_h =
                last.pokemon.saturating_sub(first.pokemon) as f64 * scale;
        }
        measurements.valid_gross_gold =
            last.gold_receipts.saturating_sub(first.gold_receipts)==kills;
        if measurements.valid_gross_gold {
            measurements.gold_gross_h =
                last.gold.saturating_sub(first.gold) as f64 * scale;
        }
        measurements.window_samples = self.battle_samples.len();
        self.view.sample_quality = if kills < 5 {
            "Early estimate · fewer than five battles recorded".into()
        } else {
            "Recorded battles · last 10 minutes".into()
        };
    }
    fn sample(&mut self, now: Instant) {
        let item = Sample {
            at: now, trainer: self.view.trainer_xp,
            pokemon: self.view.active_pokemon.as_ref().and_then(|p| p.xp),
            gold: self.view.gold, kills: self.session_kills,
        };
        if self.samples.back().is_some_and(|old| now.saturating_duration_since(old.at) < Duration::from_secs(1)) {
            return;
        }
        self.samples.push_back(item);
        while self.samples.front().is_some_and(|old| now.saturating_duration_since(old.at) > WINDOW) {
            self.samples.pop_front();
        }
        let Some(first) = self.samples.front() else { return };
        let Some(last) = self.samples.back() else { return };
        let sec = last.at.saturating_duration_since(first.at).as_secs_f64();
        if sec < MIN_MEASUREMENT { self.view.sample_quality = "Warming up · minimum 20 seconds".into(); return; }
        let scale = 3600.0 / sec;
        let rate = |a: Option<u64>, b: Option<u64>| -> Option<f64> {
            let (Some(a), Some(b)) = (a, b) else { return None };
            if b < a { return None }
            Some((b - a) as f64 * scale)
        };
        let kills = last.kills.saturating_sub(first.kills);
        // Net gold may be negative after potions, restock purchases or other
        // spend; hiding decreases incorrectly shows "no data".
        let net_gold = match (first.gold, last.gold) {
            (Some(a),Some(b)) => Some((b as i128 - a as i128) as f64 * scale),
            _ => None
        };
        self.view.measurements = HuntMeasurements {
            seconds: sec, kills, kills_h: kills as f64 * scale,
            trainer_xp_h: rate(first.trainer, last.trainer).unwrap_or(0.0),
            pokemon_xp_h: rate(first.pokemon, last.pokemon).unwrap_or(0.0),
            gold_net_h: net_gold.unwrap_or(0.0),
            gold_gross_h: self.view.measurements.gold_gross_h,
            valid_gross_gold: self.view.measurements.valid_gross_gold,
            valid_trainer: rate(first.trainer, last.trainer).is_some(),
            valid_pokemon: rate(first.pokemon, last.pokemon).is_some(),
            valid_gold: net_gold.is_some(),
            window_samples: self.samples.len(),
            last_battle_age_secs: self.view.measurements.last_battle_age_secs,
            combat_event_age_secs: self.view.measurements.combat_event_age_secs,
        };
        self.view.sample_quality = if kills < 5 {
            "Early estimate · fewer than five battles recorded".into()
        } else { "Observed live · rolling 10-minute window".into() };
    }
}

/// Exact unboosted level XP formula used by PokéIdle. Trainer and Pokémon
/// rewards use separate multipliers, not the sum of the two channels.
pub fn base_xp(level: u32) -> f64 {
    let n = level as f64;
    if level <= 150 { (0.6 * n * n).floor() + 8.0 }
    else { (13_500.0 * (n / 150.0).powf(1.25)).floor() }
}
/// Actual base Coins per kill from shared/sell-value.mjs, capped at level 25k.
pub fn base_gold(level: u32) -> f64 {
    if level == 0 { return 0.0; }
    let exponent = (6000.0_f64 / 36.0).ln() / 5000.0_f64.ln();
    (36.0 * (level.min(25_000) as f64).powf(exponent)).round().max(1.0)
}
/// Since 15 Oct 2026 the ordinary loot value is credited as Coins at death,
/// not dropped as NPC-sellable items. This matches ouroDoAbate in the game's
/// shared/recompensa-hunt.mjs; boosts multiply only the converted loot part.
pub fn gross_gold_per_kill(level: u32, loot_boost_pct: f64) -> f64 {
    if level == 0 { return 0.0; }
    let base = base_gold(level);
    let cap = base_gold(25_000);
    let target_base = if level <= 25_000 {
        base
    } else {
        cap * (level as f64 / 25_000.0).powf(0.30)
    };
    let loot = (target_base * 1.15).round().max(base) - base;
    let bonus = if loot_boost_pct.is_finite() { loot_boost_pct.max(0.0) } else { 0.0 };
    base + (loot * (1.0 + bonus / 100.0)).round()
}
/// A comparable *scenario* at the presently observed kill pace. Not an
/// estimate of a different hunt's actual DPS, travel or XP bonuses.
pub fn atlas_pace_scenario(hunt: &HuntEntry, measured_kills_h: f64) -> Option<(f64, f64)> {
    if measured_kills_h <= 0.0 || hunt.level == 0 { return None }
    Some((base_xp(hunt.level) * measured_kills_h,
        gross_gold_per_kill(hunt.level, 0.0) * measured_kills_h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn representative_sprite_uses_server_species_then_hunt_name() {
        let mut hunt = HuntEntry {
            name: "Outland Scyther Level 150".into(),
            slug: "outland-scyther-150".into(),
            ..Default::default()
        };
        assert_eq!(representative_hunt_species(&hunt), Some(123));
        hunt.species = vec![
            HuntSpecies { id: 127, weight: 10.0 },
            HuntSpecies { id: 25, weight: 90.0 },
        ];
        // Even if the fixture contains multiple species, the species ID
        // supplied by the hunt is authoritative; weights do not pick artwork.
        assert_eq!(representative_hunt_species(&hunt), Some(127));
        hunt.species = vec![HuntSpecies { id: 2006, weight: 10.0 }];
        assert_eq!(representative_hunt_species(&hunt), Some(2006));
        hunt.species = vec![HuntSpecies { id: 10501, weight: 10.0 }];
        assert_eq!(representative_hunt_species(&hunt), Some(10501));
        assert_eq!(species_id_from_hunt_name("Mewtwo Cave"), Some(150));
        assert_eq!(species_id_from_hunt_name("Scyther's Nest"), Some(123));
        assert_eq!(species_id_from_hunt_name("empty clearing"), None);
    }

    #[test]
    fn upstream_game_species_ids_resolve_to_real_sprite_looktypes() {
        // These are PokéIdle IDs, not National Pokédex IDs. Previously every
        // species above #1025 was silently rejected.
        assert_eq!(game_species_looktype(123), Some(60123));
        assert_eq!(game_species_looktype(2006), Some(60123));
        assert_eq!(game_species_dex_sprite(2006), Some(123));
        assert_eq!(game_species_looktype(14009), Some(60009));
        assert_eq!(game_species_dex_sprite(14009), Some(9));
        assert_eq!(game_species_looktype(14448), Some(60448));
        assert_eq!(game_species_dex_sprite(14448), Some(448));
        // New upstream visual IDs may be added without invalidating older mappings.
        assert!(species_art_catalog().looktypes.len() >= 944);
        assert_eq!(game_species_looktype(0), None);
        assert_eq!(species_id_from_hunt_name("Furious Scyther hunt"), Some(2006));
    }

    #[test]
    fn hunt_sprite_uses_single_server_species_without_an_encounter() {
        let mut intel = HuntIntel::new();
        let now = Instant::now();
        intel.ingest(r#"{"t":"welcome","hunts":[
            {"slug":"forest","nome":"Forest Hunt","especies":[{"pokeId":2006}]},
            {"slug":"beach","nome":"Beach Hunt","pokeId":14009}
        ],"estado":{"huntSlug":"forest","selvagem":null}}"#, now);
        // No wild spawn is required to identify the hunt's Pokémon.
        assert_eq!(intel.snapshot().hunt_art_species, Some(2006));
        intel.ingest(r#"{"t":"estado","estado":{"selvagem":{
            "speciesId":123,"nome":"Scyther","hp":30}}}"#, now);
        assert_eq!(intel.snapshot().hunt_art_species, Some(2006));
        intel.ingest(r#"{"t":"estado","estado":{"selvagem":null}}"#, now);
        assert_eq!(intel.snapshot().hunt_art_species, Some(2006));
        intel.ingest(r#"{"t":"batalha","ev":[{"k":"hunt","slug":"beach"}]}"#, now);
        assert_eq!(intel.snapshot().hunt_art_species, Some(14009));
        assert!(intel.snapshot().wild.is_none());
    }

    #[test]
    fn compact_species_catalog_supplies_sprite_id() {
        let mut intel = HuntIntel::new();
        intel.ingest(r#"{"t":"welcome","hunts":[
            {"s":"forest","n":"Forest","nv":150,"e":[[2006,1]]}
        ],"estado":{"huntSlug":"forest"}}"#, Instant::now());
        assert_eq!(intel.snapshot().hunt_art_species, Some(2006));
        assert_eq!(intel.snapshot().hunts[0].species.len(), 1);
        assert_eq!(intel.snapshot().hunts[0].level, 150);
    }

    #[test]
    fn location_init_recovers_a_confirmed_hunt_without_wild_encounters() {
        let now = Instant::now();
        let mut intel = HuntIntel::new();
        intel.ingest(r#"{"t":"welcome","hunts":[
            {"slug":"forest","nome":"Forest Hunt"},
            {"slug":"beach","nome":"Beach Hunt"}
        ],"estado":{"noCentro":true}}"#, now);
        intel.ingest(r#"{"t":"campo.init","slug":"forest","mapa":"forest-map"}"#, now);
        assert_eq!(intel.snapshot().hunt_slug.as_deref(), Some("forest"));
        assert!(!intel.snapshot().in_center);
        intel.ingest(r#"{"t":"estado","estado":{"selvagem":null}}"#, now);
        assert_eq!(intel.snapshot().hunt_slug.as_deref(), Some("forest"));
        intel.ingest(r#"{"t":"campo.init","slug":"unknown-map"}"#, now);
        assert_eq!(intel.snapshot().hunt_slug.as_deref(), Some("forest"));
    }

    #[test]
    fn pokemon_instance_id_and_pokedex_id_are_distinct() {
        let mut intel = HuntIntel::new();
        intel.ingest(r#"{"t":"welcome","estado":{"activeId":42,
            "pokemons":[{"id":42,"pokeId":25,"nome":"Pikachu","looktype":60025}]}}"#,
            Instant::now());
        let active = intel.snapshot().active_pokemon.unwrap();
        assert_eq!(active.id, 42);
        assert_eq!(active.species_id, Some(25));
        assert_eq!(active.looktype, Some(60025));
    }

    #[test]
    fn opponent_health_and_battle_events_update_combat_activity_without_a_kill() {
        let mut intel = HuntIntel::new();
        let now = Instant::now();
        intel.ingest(r#"{"t":"estado","estado":{"selvagem":{"slot":1,"nome":"Scyther","level":100,"speciesId":123,"hp":90,"maxHp":100}}}"#,now);
        assert!(intel.snapshot().measurements.combat_event_age_secs.is_some());
        intel.ingest(r#"{"t":"estado","estado":{"selvagem":{"slot":1,"nome":"Scyther","level":100,"speciesId":123,"hp":60,"maxHp":100}}}"#,now+Duration::from_secs(2));
        let snap = intel.snapshot();
        assert!(snap.measurements.combat_event_age_secs.is_some());
        assert_eq!(snap.measurements.last_battle_age_secs,None);
        assert_eq!(snap.wild.as_ref().unwrap().name,"Scyther");
        intel.ingest(r#"{"t":"batalha","ev":[{"k":"ataque"}]}"#,now+Duration::from_secs(4));
        assert!(intel.snapshot().measurements.combat_event_age_secs.is_some());
        intel.on_disconnect();
        let disconnected = intel.snapshot();
        assert_eq!(disconnected.measurements.combat_event_age_secs,None);
        assert!(disconnected.wild.is_none());
    }

    #[test]
    fn wild_partial_updates_keep_enemy_visible_until_explicitly_cleared() {
        let mut intel = HuntIntel::new();
        let now = Instant::now();
        intel.ingest(r#"{"t":"welcome","estado":{"selvagem":{
            "nome":"Scyther","speciesId":123,"level":150,"hp":90,"maxHp":100}}}"#, now);
        let first = intel.snapshot().wild.unwrap();
        assert_eq!(first.name,"Scyther");
        assert_eq!(first.slot,0);
        assert_eq!(first.species_id,Some(123));
        assert!(intel.snapshot().measurements.combat_event_age_secs.is_some());
        intel.ingest(r#"{"t":"estado","estado":{"selvagem":{"hp":60}}}"#,
            now + Duration::from_secs(1));
        let patched = intel.snapshot().wild.unwrap();
        assert_eq!(patched.name,"Scyther");
        assert_eq!(patched.species_id,Some(123));
        assert_eq!(patched.level,150);
        assert_eq!(patched.hp,60);
        intel.ingest(r#"{"t":"estado","estado":{"selvagem":null}}"#,
            now + Duration::from_secs(2));
        assert!(intel.snapshot().wild.is_none());
    }

    #[test]
    fn a_new_unidentified_slot_must_not_be_called_the_old_enemy() {
        let mut intel=HuntIntel::new();
        let now=Instant::now();
        intel.ingest(r#"{"t":"estado","estado":{"selvagem":{
            "slot":1,"nome":"Scyther","speciesId":123,"hp":100}}}"#,now);
        intel.ingest(r#"{"t":"estado","estado":{"selvagem":{
            "slot":2,"hp":100}}}"#,now + Duration::from_secs(1));
        assert!(intel.snapshot().wild.is_none());
        intel.ingest(r#"{"t":"estado","estado":{"selvagem":{
            "slot":2,"nome":"Pinsir","speciesId":127,"hp":50}}}"#,
            now + Duration::from_secs(2));
        assert_eq!(intel.snapshot().wild.unwrap().name,"Pinsir");
    }

    #[test]
    fn enemy_remains_until_authoritative_replacement_or_center() {
        let mut intel=HuntIntel::new();
        let now=Instant::now();
        let spawn=r#"{"t":"estado","estado":{"selvagem":{
            "slot":1,"nome":"Scyther","hp":100}}}"#;
        intel.ingest(spawn,now);
        intel.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem"}]}"#,now);
        // Death receipts do not necessarily carry the next encounter. Hold
        // the last confirmed target until an estado replacement or null arrives.
        assert_eq!(intel.snapshot().wild.unwrap().name,"Scyther");
        intel.ingest(r#"{"t":"estado","estado":{"noCentro":true}}"#,now);
        assert!(intel.snapshot().wild.is_none());
    }

    #[test]
    fn nullable_pokemon_patches_remove_held_items_and_clear_active_id() {
        let now=Instant::now();let mut m=HuntIntel::new();
        m.ingest(r#"{"t":"welcome","estado":{"activeId":7,
            "pokemons":[{"id":7,"nome":"Pidgeot","heldItemId":212,
                "ivs":{"hp":31,"atk":20}}]}}"#,now);
        m.ingest(r#"{"t":"estado","estado":{"activeId":null,
            "pkMud":[{"id":7,"heldItemId":null,"ivs":{"hp":null}}]}}"#,now);
        let roster=m.snapshot();
        assert!(roster.active_pokemon.is_none());
        assert_eq!(roster.pokemon_roster[0].held_item_id,None);
        assert_eq!(roster.pokemon_roster[0].ivs[0],None);
        assert_eq!(roster.pokemon_roster[0].ivs[1],Some(20));
    }
    #[test]
    fn partial_roster_is_not_credited_as_a_complete_collection(){
        let now=Instant::now();let mut m=HuntIntel::new();
        m.ingest(r#"{"t":"estado","estado":{"pkMud":[{"id":7,"speciesId":25}]}}"#,now);
        assert!(!m.view.roster_ready);
        m.ingest(r#"{"t":"welcome","estado":{"pokemons":[]}}"#,now);
        assert!(m.view.roster_ready);
        m.on_disconnect();
        assert!(!m.view.roster_ready);
    }
    #[test]
    fn full_empty_roster_replaces_previous_nonempty_snapshot() {
        let now=Instant::now();let mut m=HuntIntel::new();
        m.ingest(r#"{"t":"welcome","estado":{"pokemons":[{"id":7,"nome":"Pidgey"}]}}"#,now);
        assert_eq!(m.snapshot().pokemon_roster.len(),1);
        m.ingest(r#"{"t":"estado","estado":{"pokemons":[]}}"#,now);
        assert!(m.snapshot().pokemon_roster.is_empty());
    }
    #[test]
    fn sale_receipt_updates_native_roster_without_guessing_other_ids() {
        let now=Instant::now();let mut m=HuntIntel::new();
        m.ingest(r#"{"t":"welcome","estado":{
            "pokemons":[{"id":4,"nome":"Abra"},{"id":5,"nome":"Kadabra"}],
            "activeId":5}}"#,now);
        assert_eq!(m.snapshot().pokemon_roster.len(),2);
        m.confirm_pokemon_sale(4);
        assert_eq!(m.snapshot().pokemon_roster.len(),1);
        assert_eq!(m.view.active_pokemon.as_ref().unwrap().id,5);
    }
    #[test]
    fn nick_and_field_initialization_are_retained_without_guessing_hunt() {
        let now=Instant::now();let mut m=HuntIntel::new();
        m.ingest(r#"{"t":"welcome","estado":{"nick":"Example","gold":5}}"#,now);
        m.ingest(r#"{"t":"campo.init","slug":"route-map",
            "mapa":"Grasslands","ts":12345}"#,now);
        assert_eq!(m.view.account_nick.as_deref(),Some("Example"));
        assert_eq!(m.view.field_map.as_deref(),Some("Grasslands"));
        assert_eq!(m.view.field_slug.as_deref(),Some("route-map"));
        assert_eq!(m.view.field_init_timestamp,Some(12345));
        assert!(m.view.hunt_slug.is_none());
    }

    #[test]
    fn catalog_pokemon_and_partial_patch_follow_manager_server_fields() {
        let mut m = HuntIntel::new();
        let at = Instant::now();
        m.ingest(&json!({"t":"welcome","hunts":[{"slug":"route-1","nome":"Route One",
            "area":"Forest","regiao":"Kanto","nivel":120,
            "especies":[{"pokeId":123,"pontos":90},{"pokeId":16,"pontos":10}]}],
            "estado":{"huntSlug":"route-1","activeId":99,"level":850,"xp":1000,
                "pokemons":[{"id":99,"nome":"Scizor","level":851,"xp":2000,"quality":1.3,
                    "nota":9.4,"looktype":131,"lookShiny":132,
                    "ivs":{"atk":31,"speed":28}}]}}).to_string(),at);
        assert_eq!(m.view.hunts.len(),1);
        assert_eq!(m.view.hunts[0].species[0].weight,90.0);
        assert_eq!(m.view.active_pokemon.as_ref().unwrap().ivs[1],Some(31));
        assert_eq!(m.view.active_pokemon.as_ref().unwrap().grade,Some(9.4));
        assert_eq!(m.view.active_pokemon.as_ref().unwrap().looktype,Some(131));
        assert_eq!(m.view.active_pokemon.as_ref().unwrap().shiny_looktype,Some(132));
        m.ingest(&json!({"t":"estado","estado":{"pkMud":[{"id":99,"xp":2400}],
            "evento":{"xpTreinadorPct":30.0},"guildBonusPct":5.0}}).to_string(),
            at+Duration::from_secs(2));
        assert_eq!(m.view.active_pokemon.as_ref().unwrap().xp,Some(2400));
        assert_eq!(m.view.bonuses.trainer_event_pct,30.0);
    }
    #[test]
    fn switching_requires_catalog_membership_and_resets_observation() {
        let mut m=HuntIntel::new();
        let at=Instant::now();
        m.ingest(r#"{"t":"welcome","hunts":[{"slug":"a","nome":"A"},{"slug":"b","nome":"B"}],"estado":{"huntSlug":"a","xp":100}}"#,at);
        assert!(m.request_hunt("bogus",at).is_err());
        m.request_hunt("b",at).unwrap();
        assert_eq!(m.view.hunt_slug.as_deref(),Some("a"));
        m.ingest(r#"{"t":"batalha","ev":[{"k":"hunt","slug":"b","nome":"B"}]}"#,at+Duration::from_secs(1));
        assert_eq!(m.view.hunt_slug.as_deref(),Some("b"));
        assert_eq!(m.view.pending_hunt_slug,None);
    }
    #[test]
    fn xp_channels_are_distinct_and_rates_are_observed() {
        let mut m=HuntIntel::new();
        let at=Instant::now();
        m.ingest(r#"{"t":"welcome","hunts":[{"slug":"a","nome":"A"}],"estado":{"huntSlug":"a",
            "xp":1000,"gold":100,"activeId":2,"pokemons":[{"id":2,"nome":"X","xp":2000}]}}"#,at);
        for _ in 0..10 {m.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem"}]}"#,at);}
        m.ingest(r#"{"t":"estado","estado":{"xp":2100,"gold":150,"pkMud":[{"id":2,"xp":2600}]}}"#,
            at+Duration::from_secs(60));
        assert_eq!(m.view.measurements.kills,10);
        assert_eq!(m.view.measurements.trainer_xp_h,66000.0);
        assert_eq!(m.view.measurements.pokemon_xp_h,36000.0);
        assert_eq!(m.view.measurements.gold_net_h,3000.0);
    }
    #[test]
    fn account_domains_merge_partial_state_without_erasing_missing_inventory() {
        let mut intel = HuntIntel::new();
        let now = Instant::now();
        intel.ingest(r#"{"t":"welcome","hunts":[],"estado":{"balls":{"1":15},
            "items":{"200":2},"diamonds":90,"orbs":12,
            "automation":{"autoPotion":true,"hpLimiar":50},
            "selvagem":{"slot":5,"nome":"Scyther","level":150,"hp":100,"maxHp":900}}}"#,now);
        intel.ingest(r#"{"t":"estado","estado":{"automation":{"autoRevive":true},
            "pkMud":[{"id":99,"nome":"X","level":20,"hp":99}]}}"#,
            now+Duration::from_secs(1));
        assert_eq!(intel.view.balls.get(&1),Some(&15));
        assert_eq!(intel.view.items.get(&200),Some(&2));
        assert_eq!(intel.view.automation.as_ref().unwrap()["autoPotion"],true);
        assert_eq!(intel.view.automation.as_ref().unwrap()["autoRevive"],true);
        assert_eq!(intel.view.wild.as_ref().unwrap().slot,5);
        assert_eq!(intel.snapshot().pokemon_roster.len(),1);
    }
    #[test]
    fn gold_payout_matches_upstream_loot_conversion_and_high_level_cap() {
        assert_eq!(base_gold(1),36.0);
        assert_eq!(base_gold(5_000),6_000.0);
        assert_eq!(base_gold(25_000),base_gold(160_300));
        assert_eq!(gross_gold_per_kill(1,0.0),41.0);
        assert_eq!(gross_gold_per_kill(100,0.0),658.0);
        assert_eq!(gross_gold_per_kill(100,40.0),692.0);
        assert_eq!(gross_gold_per_kill(25_000,0.0),18_142.0);
        assert_eq!(gross_gold_per_kill(160_300,0.0),31_681.0);
        assert_eq!(gross_gold_per_kill(160_300,40.0),38_043.0);
        assert_eq!(gross_gold_per_kill(0,0.0),0.0);
    }
    #[test]
    fn receipts_measure_combat_gold_separately_from_wallet_spend_and_sales() {
        let mut model=HuntIntel::new();
        let now=Instant::now();
        model.ingest(r#"{"t":"welcome","hunts":[{"slug":"a","nome":"A"}],
            "estado":{"huntSlug":"a","gold":1000}}"#,now);
        for elapsed in [20,40] {
            model.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem",
                "ouro":658,"ouroVendaAuto":1000000,"xpTreinador":100,"xpPokemon":100}]}"#,
                now+Duration::from_secs(elapsed));
        }
        model.ingest(r#"{"t":"estado","estado":{"gold":400}}"#,
            now+Duration::from_secs(60));
        assert!(model.view.measurements.valid_gross_gold);
        assert_eq!(model.view.measurements.gold_gross_h,118_440.0);
        assert_eq!(model.view.measurements.gold_net_h,-36_000.0);
        assert_eq!(model.view.session.combat_gold,1316);
        assert_eq!(model.view.session.auto_sale_gold,2_000_000);
    }
    #[test]
    fn missing_gold_receipt_does_not_fabricate_gross_income() {
        let mut model=HuntIntel::new();
        let now=Instant::now();
        model.ingest(r#"{"t":"welcome","hunts":[{"slug":"a"}],
            "estado":{"huntSlug":"a","gold":500}}"#,now);
        model.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem",
            "ouro":500}]}"#,now+Duration::from_secs(20));
        model.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem"}]}"#,
            now+Duration::from_secs(40));
        assert!(!model.view.measurements.valid_gross_gold);
    }
    #[test]
    fn measured_net_gold_includes_negative_spend() {
        let mut model=HuntIntel::new();
        let now=Instant::now();
        model.ingest(r#"{"t":"welcome","hunts":[{"slug":"a","nome":"A"}],
            "estado":{"huntSlug":"a","gold":1000}}"#,now);
        model.ingest(r#"{"t":"estado","estado":{"gold":500}}"#,now+Duration::from_secs(60));
        assert_eq!(model.view.measurements.gold_net_h,-30_000.0);
        assert!(model.view.measurements.valid_gold);
    }
    #[test]
    fn acquired_drops_are_counted_from_death_receipts_not_inventory_purchases() {
        let mut intel=HuntIntel::new();let now=Instant::now();
        intel.ingest(r#"{"t":"welcome","hunts":[{"slug":"a","nome":"A"}],
            "estado":{"huntSlug":"a","items":{"55":900}}}"#,now);
        intel.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem",
            "xpTreinador":100,"xpPokemon":50,"ouro":33,"ouroVendaAuto":5,
            "drops":[{"nome":"Moon Stone","qtd":3,"ganho":2}]}]}"#,now);
        intel.ingest(r#"{"t":"batalha","ev":[{"k":"compra","nome":"Moon Stone",
            "qtd":5,"gasto":400}]}"#,now);
        intel.ingest(r#"{"t":"estado","estado":{"items":{"55":910}}}"#,now);
        assert_eq!(intel.view.session.acquired["Moon Stone"],2);
        assert_eq!(intel.view.session.trainer_xp,100);
        assert_eq!(intel.view.session.pokemon_xp,50);
        assert_eq!(intel.view.session.combat_gold,33);
        assert_eq!(intel.view.session.auto_sale_gold,5);
        intel.reset_session(now+Duration::from_secs(1));
        assert!(intel.view.session.acquired.is_empty());
        assert_eq!(intel.view.items.get(&55),Some(&910));
    }

    #[test]
    fn battle_receipts_feed_live_rates_without_periodic_estado() {
        let now=Instant::now();
        let mut intel=HuntIntel::new();
        intel.ingest(r#"{"t":"welcome","hunts":[{"slug":"a","nome":"A"}],
            "estado":{"huntSlug":"a","activeId":7,"pokemons":[{"id":7,"nome":"X","xp":10}]}}"#,now);
        for i in 0..6 {
            intel.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem",
                "xpTreinador":100,"xpPokemon":50,"ouro":10}]}"#,
                now+Duration::from_secs(5*(i+1)));
        }
        assert!(intel.view.measurements.valid_trainer);
        assert!(intel.view.measurements.valid_pokemon);
        assert_eq!(intel.view.measurements.kills,6);
        assert_eq!(intel.view.measurements.trainer_xp_h,72_000.0);
        assert_eq!(intel.view.measurements.pokemon_xp_h,36_000.0);
        assert!(!intel.view.measurements.valid_gold); // No fabricated net-gold delta
    }

    #[test]
    fn new_hunt_cannot_inherit_previous_battle_reward_rates() {
        let now=Instant::now();
        let mut intel=HuntIntel::new();
        intel.ingest(r#"{"t":"welcome","hunts":[{"slug":"a","nome":"A"},{"slug":"b","nome":"B"}],
            "estado":{"huntSlug":"a"}}"#,now);
        for i in 0..5 {
            intel.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem",
                "xpTreinador":100,"xpPokemon":50}]}"#,
                now+Duration::from_secs(5*(i+1)));
        }
        assert!(intel.view.measurements.valid_trainer);
        intel.ingest(r#"{"t":"batalha","ev":[{"k":"hunt","slug":"b","nome":"B"}]}"#,
            now+Duration::from_secs(26));
        assert!(!intel.view.measurements.valid_trainer);
        assert_eq!(intel.view.measurements.kills,0);
    }
    #[test]
    fn missing_reward_channel_is_unknown_not_measured_zero() {
        let now=Instant::now();
        let mut intel=HuntIntel::new();
        intel.ingest(r#"{"t":"welcome","hunts":[{"slug":"a"}],
            "estado":{"huntSlug":"a"}}"#,now);
        for i in 0..6 {
            intel.ingest(r#"{"t":"batalha","ev":[{"k":"morte",
                "quem":"selvagem","xpTreinador":100}]}"#,
                now+Duration::from_secs(5*(i+1)));
        }
        assert!(intel.view.measurements.valid_trainer);
        assert!(!intel.view.measurements.valid_pokemon);
    }
    #[test]
    fn reconnect_reanchors_rate_without_erasing_session_rewards() {
        let now=Instant::now();
        let mut intel=HuntIntel::new();
        intel.ingest(r#"{"t":"welcome","hunts":[{"slug":"a"}],
            "estado":{"huntSlug":"a"}}"#,now);
        for i in 0..6 {
            intel.ingest(r#"{"t":"batalha","ev":[{"k":"morte",
                "quem":"selvagem","xpTreinador":100,"xpPokemon":40}]}"#,
                now+Duration::from_secs(5*(i+1)));
        }
        assert!(intel.view.measurements.valid_trainer);
        let before=intel.view.session.kills;
        intel.on_disconnect();
        for i in 0..6 {
            intel.ingest(r#"{"t":"batalha","ev":[{"k":"morte",
                "quem":"selvagem","xpTreinador":100,"xpPokemon":40}]}"#,
                now+Duration::from_secs(60+5*(i+1)));
        }
        assert_eq!(intel.view.measurements.kills,6);
        assert_eq!(intel.view.session.kills,before+6);
    }
    #[test]
    fn unobserved_hunts_never_claim_actual_xp_per_hour() {
        let h=HuntEntry { level: 100, ..Default::default() };
        assert_eq!(base_xp(100),6008.0);
        assert_eq!(atlas_pace_scenario(&h,0.0),None);
        assert_eq!(atlas_pace_scenario(&h,100.0).unwrap().0,600800.0);
    }
    #[test]
    fn active_pokemon_switch_reanchors_battle_receipts_without_counting_old_kills(){
        let now=Instant::now();
        let mut intel=HuntIntel::new();
        intel.ingest(r#"{"t":"welcome","hunts":[{"slug":"a","nome":"A"}],
            "estado":{"huntSlug":"a","activeId":7,"pokemons":[{"id":7,"nome":"A"},{"id":8,"nome":"B"}]}}"#,now);
        for i in 0..6 {
            intel.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem",
                "xpTreinador":100,"xpPokemon":50}]}"#,now+Duration::from_secs(5*(i+1)));
        }
        intel.ingest(r#"{"t":"estado","estado":{"activeId":8}}"#,now+Duration::from_secs(31));
        assert!(!intel.view.measurements.valid_pokemon);
        assert_eq!(intel.view.measurements.kills,0);
        for i in 0..6 {
            intel.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem",
                "xpTreinador":100,"xpPokemon":20}]}"#,now+Duration::from_secs(35+5*i));
        }
        assert_eq!(intel.view.measurements.kills,6);
        assert_eq!(intel.view.measurements.pokemon_xp_h,120.0*3600.0/29.0);
    }
    #[test]
    fn bonus_expiration_is_recomputed_when_server_clock_moves(){
        let mut intel=HuntIntel::new();
        let now=Instant::now();
        intel.ingest(r#"{"t":"welcome","estado":{
            "servidorAgora":100,"vipAte":120,"guild":{"boostAte":130}}}"#,now);
        assert!(intel.view.bonuses.vip_active);
        assert!(intel.view.bonuses.guild_boost_active);
        intel.ingest(r#"{"t":"estado","estado":{"servidorAgora":140}}"#,now);
        assert!(!intel.view.bonuses.vip_active);
        assert!(!intel.view.bonuses.guild_boost_active);
    }
    #[test]
    fn idle_battle_measurements_expire_without_discarding_session_totals(){
        let mut intel=HuntIntel::new();
        let now=Instant::now();
        intel.ingest(r#"{"t":"welcome","hunts":[{"slug":"a"}],"estado":{"huntSlug":"a"}}"#,now);
        for i in 0..6 {
            intel.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem",
                "xpTreinador":100,"xpPokemon":50}]}"#,now+Duration::from_secs(5*(i+1)));
        }
        assert!(intel.view.measurements.valid_trainer);
        // Explicitly simulate a snapshot after time has passed without new
        // receipts, independent of periodic server-state traffic.
        intel.last_battle_at=Some(Instant::now()-Duration::from_secs(181));
        let stats=intel.snapshot();
        assert!(!stats.measurements.valid_trainer);
        assert_eq!(stats.session.kills,6);
    }

    #[test]
    fn missing_bonus_fields_are_unknown_rather_than_reported_zero(){
        let mut intel=HuntIntel::new();
        let now=Instant::now();
        intel.ingest(r#"{"t":"welcome","estado":{"level":10}}"#,now);
        assert!(!intel.view.bonuses.twitch_known);
        assert!(!intel.view.bonuses.vip_known);
        intel.ingest(r#"{"t":"estado","estado":{"twitch":{"pct":0},
            "servidorAgora":100,"vipAte":null}}"#,now);
        assert!(intel.view.bonuses.twitch_known);
        assert_eq!(intel.view.bonuses.twitch_pct,0.0);
        assert!(intel.view.bonuses.vip_known);
        assert!(!intel.view.bonuses.vip_active);
    }

    #[test]
    fn outland_catalog_gate_from_server_overrides_baked_thresholds(){
        let now=Instant::now();
        let mut intel=HuntIntel::new();
        intel.ingest(r#"{"t":"welcome","catalogoOutlandTiers":[
            {"tier":1,"nivel":150,"mult":1},{"tier":2,"nivel":1100,"mult":1.25},
            {"tier":3,"nivel":2100,"mult":1.5},{"tier":4,"nivel":4100,"mult":2},
            {"tier":5,"nivel":8100,"mult":3},{"tier":6,"nivel":16100,"mult":4},
            {"tier":7,"nivel":24100,"mult":6},{"tier":8,"nivel":36100,"mult":8}
        ],"estado":{"level":1000,"tierOutland":1}}"#,now);
        assert_eq!(intel.view.outland_requirement(2),Some((1100,1.25)));
        assert_eq!(intel.view.unlocked_outland_tier(2),None);
        assert!(intel.request_outland_tier(2,now).is_err());
        intel.ingest(r#"{"t":"estado","estado":{"level":1100}}"#,now);
        assert_eq!(intel.view.unlocked_outland_tier(2),Some(1.25));
        assert!(intel.request_outland_tier(2,now).is_ok());
        intel.on_disconnect();
        assert_eq!(intel.view.outland_requirement(2),Some((1000,1.25)));
    }
    #[test]
    fn malformed_outland_catalog_cannot_unlock_a_higher_tier(){
        let mut catalog=serde_json::json!([
            {"tier":1,"nivel":150,"mult":1},{"tier":2,"nivel":1000,"mult":1.25},
            {"tier":3,"nivel":2000,"mult":1.5},{"tier":4,"nivel":4000,"mult":2},
            {"tier":5,"nivel":8000,"mult":3},{"tier":6,"nivel":16000,"mult":4},
            {"tier":7,"nivel":24000,"mult":6},{"tier":8,"nivel":36000,"mult":8}
        ]);
        assert!(parse_outland_catalog(&catalog).is_some());
        catalog[1]["nivel"]=serde_json::json!(100);
        assert!(parse_outland_catalog(&catalog).is_none());
        catalog[1]["nivel"]=serde_json::json!(1000);
        catalog[2]["mult"]=serde_json::json!(null);
        assert!(parse_outland_catalog(&catalog).is_none());
    }
    #[test]
    fn outland_tier_changes_only_on_server_confirmation(){
        let now=Instant::now();
        let mut intel=HuntIntel::new();
        intel.ingest(r#"{"t":"welcome","estado":{"level":1000,
            "tierOutland":1}}"#,now);
        assert!(intel.request_outland_tier(3,now).is_err());
        assert!(intel.request_outland_tier(2,now).is_ok());
        let pending=intel.snapshot();
        assert_eq!(pending.outland_tier,Some(1));
        assert_eq!(pending.pending_outland_tier,Some(2));
        assert!(intel.request_outland_tier(1,now).is_err());
        // A state patch carrying an unrelated old tier is not an ack.
        intel.ingest(r#"{"t":"estado","estado":{"tierOutland":1}}"#,now);
        assert_eq!(intel.snapshot().pending_outland_tier,Some(2));
        // The only confirmation is a server-reported matching tier.
        intel.ingest(r#"{"t":"estado","estado":{"tierOutland":2}}"#,now);
        assert_eq!(intel.snapshot().outland_tier,Some(2));
        assert_eq!(intel.snapshot().pending_outland_tier,None);
        assert_eq!(intel.snapshot().outland_tier_error,None);
    }
    #[test]
    fn outland_tier_timeout_and_disconnect_discard_pending_request(){
        let now=Instant::now();
        let mut intel=HuntIntel::new();
        intel.ingest(r#"{"t":"welcome","estado":{"level":8000,
            "tierOutland":2}}"#,now);
        assert!(intel.request_outland_tier(5,now).is_ok());
        intel.expire_outland_request(now+Duration::from_secs(7));
        assert_eq!(intel.view.outland_tier,Some(2));
        assert_eq!(intel.view.pending_outland_tier,None);
        assert!(intel.view.outland_tier_error.is_some());
        assert!(intel.request_outland_tier(5,now).is_ok());
        intel.on_disconnect();
        assert_eq!(intel.view.pending_outland_tier,None);
        assert_eq!(intel.view.outland_tier,None);
    }
    #[test]
    fn capture_catalog_preserves_currency_alternatives_and_outland_tier(){
        let now=Instant::now();
        let mut intel=HuntIntel::new();
        intel.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,
            "nome":"Ultra","catchRate":4,"priceGold":130},
            {"id":5,"nome":"Beast","catchRate":8,"priceDiamond":4}],
            "estado":{"tierOutland":5}}"#,now);
        let view=intel.snapshot();
        assert_eq!(view.outland_tier,Some(5));
        assert_eq!(view.ball_quotes[&4].efficiency,Some(4.0));
        assert_eq!(view.ball_quotes[&5].diamond_price,Some(4.0));
        assert_eq!(view.ball_quotes[&5].gold_price,None);
    }
    #[test]
    fn ball_success_ratio_uses_only_identified_ball_receipts(){
        let now=Instant::now();
        let mut intel=HuntIntel::new();
        intel.ingest(r#"{"t":"batalha","ev":[
            {"k":"bola","ballId":3,"sucesso":true},
            {"k":"bola","ballId":3,"sucesso":false},
            {"k":"bola","sucesso":true}]}"#,now);
        assert_eq!(intel.view.session.captures,2);
        assert_eq!(intel.view.session.balls_used.get(&3),Some(&2));
        assert_eq!(intel.view.session.ball_captures,1);
    }

}
