//! Native, account-scoped hunt intelligence. No DOM, Violentmonkey or polling.
//! Derived independently from server welcome, estado patches and batalha events.
//! Other-hunt projections are explicitly scenario comparisons, NOT combat forecasts.
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
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
    pub looktype: Option<u64>,
    pub species: Vec<HuntSpecies>,
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
    pub potency: Option<u64>,
    pub shiny: bool,
    pub ivs: [Option<u64>; 6],
    pub types: Vec<String>,
}
#[derive(Clone, Debug, Default)]
pub struct WildInfo {
    pub slot: u64,
    pub name: String,
    pub level: u32,
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
}
#[derive(Clone, Debug, Default)]
pub struct HuntMeasurements {
    pub seconds: f64,
    pub kills: u64,
    pub kills_h: f64,
    pub trainer_xp_h: f64,
    pub pokemon_xp_h: f64,
    pub gold_net_h: f64,
    pub valid_trainer: bool,
    pub valid_pokemon: bool,
    pub valid_gold: bool,
    pub window_samples: usize,
}
#[derive(Clone, Debug, Default)]
pub struct HuntView {
    pub hunts: Vec<HuntEntry>,
    pub hunt_slug: Option<String>,
    pub pending_hunt_slug: Option<String>,
    pub in_center: bool,
    pub active_pokemon: Option<PokemonInfo>,
    pub pokemon_count: usize,
    pub pokemon_roster: Arc<Vec<PokemonInfo>>,
    pub wild: Option<WildInfo>,
    pub balls: Arc<BTreeMap<u64, u64>>,
    pub items: Arc<BTreeMap<u64, u64>>,
    pub automation: Option<Value>,
    pub vip_until: Option<u64>,
    pub server_now: Option<u64>,
    pub center_free_at: Option<u64>,
    pub diamonds: Option<u64>,
    pub orbs: Option<u64>,
    pub trainer_level: Option<u32>,
    pub trainer_xp: Option<u64>,
    pub gold: Option<u64>,
    pub bonuses: HuntBonuses,
    pub measurements: HuntMeasurements,
    pub sample_quality: String,
    pub battle_kills: u64,
    pub captures: u64,
    pub catalog_ready: bool,
    pub last_error: Option<String>,
}
#[derive(Clone, Debug)]
struct Sample { at: Instant, trainer: Option<u64>, pokemon: Option<u64>, gold: Option<u64>, kills: u64 }
const WINDOW: Duration = Duration::from_secs(600);
const MIN_MEASUREMENT: f64 = 20.0;

pub struct HuntIntel {
    pub view: HuntView,
    pokemon: BTreeMap<u64, PokemonInfo>,
    active_id: Option<u64>,
    samples: VecDeque<Sample>,
    session_kills: u64,
    session_start: Instant,
    pending_at: Option<Instant>,
}
impl HuntIntel {
    pub fn new() -> Self {
        Self {
            view: HuntView::default(), pokemon: BTreeMap::new(), active_id: None,
            samples: VecDeque::new(), session_kills: 0,
            session_start: Instant::now(), pending_at: None,
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
    pub fn command_failed(&mut self, error: &str) {
        self.view.pending_hunt_slug = None;
        self.pending_at = None;
        self.view.last_error = Some(error.to_owned());
    }
    pub fn on_disconnect(&mut self) {
        self.view.pending_hunt_slug = None;
        self.pending_at = None;
        self.samples.clear();
        self.view.measurements = HuntMeasurements::default();
        self.view.sample_quality = "Reconnecting; collecting fresh measurements".into();
    }
    fn adopt_hunt(&mut self, slug: &str, now: Instant) {
        if self.view.hunt_slug.as_deref() != Some(slug) {
            self.view.hunt_slug = Some(slug.to_owned());
            self.samples.clear();
            self.session_kills = 0;
            self.session_start = now;
            self.view.battle_kills = 0;
            self.view.captures = 0;
            self.view.measurements = HuntMeasurements::default();
            self.view.sample_quality = "Warming up after hunt switch".into();
        }
        if self.view.pending_hunt_slug.as_deref() == Some(slug) {
            self.view.pending_hunt_slug = None;
            self.pending_at = None;
        }
    }
    fn num(record: &Value, key: &str) -> Option<u64> { record.get(key).and_then(Value::as_u64) }
    fn set_pokemon(&mut self, value: &Value) {
        let Some(id) = Self::num(value, "id") else { return };
        let existing = self.pokemon.entry(id).or_insert_with(|| PokemonInfo {
            id, ..PokemonInfo::default()
        });
        if let Some(name) = value.get("nome").or_else(|| value.get("name")).and_then(Value::as_str) {
            existing.name = name.to_owned();
        }
        if let Some(n) = Self::num(value, "speciesId") { existing.species_id = Some(n); }
        if let Some(n) = Self::num(value, "level") { existing.level = n.min(u32::MAX as u64) as u32; }
        if let Some(n) = Self::num(value, "xp") { existing.xp = Some(n); }
        if let Some(n) = Self::num(value, "xpNivel") { existing.xp_level = Some(n); }
        if let Some(n) = Self::num(value, "xpProximo") { existing.xp_next = Some(n); }
        if let Some(n) = Self::num(value, "hp") { existing.hp = Some(n); }
        if let Some(n) = Self::num(value, "maxHp") { existing.max_hp = Some(n); }
        if let Some(n) = Self::num(value, "heldItemId") { existing.held_item_id = Some(n); }
        if let Some(n) = Self::num(value, "poder") { existing.power = Some(n); }
        if let Some(n) = value.get("quality").and_then(Value::as_f64) { existing.quality = Some(n); }
        if let Some(n) = Self::num(value, "potencia") { existing.potency = Some(n); }
        if let Some(b) = value.get("shiny").and_then(Value::as_bool) { existing.shiny = b; }
        if let Some(types) = value.get("tipos").and_then(Value::as_array) {
            existing.types = types.iter().filter_map(Value::as_str).map(str::to_owned).collect();
        }
        if let Some(ivs) = value.get("ivs") {
            for (i, field) in ["hp", "atk", "def", "spAtk", "spDef", "speed"].iter().enumerate() {
                if let Some(n) = ivs.get(field).and_then(Value::as_u64) {
                    existing.ivs[i] = Some(n);
                }
            }
        }
    }
    fn merge_state(&mut self, state: &Value, now: Instant) {
        if let Some(value) = Self::num(state, "diamonds") { self.view.diamonds = Some(value); }
        if let Some(value) = Self::num(state, "orbs") { self.view.orbs = Some(value); }
        if let Some(value) = Self::num(state, "servidorAgora") { self.view.server_now = Some(value); }
        if let Some(value) = Self::num(state, "centroLivreEm") { self.view.center_free_at = Some(value); }
        if let Some(value) = Self::num(state, "vipAte") { self.view.vip_until = Some(value); }
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
            self.view.wild = if wild.is_null() { None } else {
                wild.get("slot").and_then(Value::as_u64).map(|slot| WildInfo {
                    slot,
                    name: wild.get("nome").and_then(Value::as_str).unwrap_or("").into(),
                    level: Self::num(wild,"level").unwrap_or(0).min(u32::MAX as u64) as u32,
                    species_id: Self::num(wild,"speciesId"),
                    hp: Self::num(wild,"hp").unwrap_or(0),
                    max_hp: Self::num(wild,"maxHp").unwrap_or(0)
                })
            };
        }
        if let Some(level) = Self::num(state, "level") {
            self.view.trainer_level = Some(level.min(u32::MAX as u64) as u32);
        }
        if let Some(xp) = Self::num(state, "xp") { self.view.trainer_xp = Some(xp); }
        if let Some(gold) = Self::num(state, "gold") { self.view.gold = Some(gold); }
        if let Some(id) = Self::num(state, "activeId") {
            if self.active_id != Some(id) {
                // Two Pokémon can have unrelated total XP values; do not
                // compare across a switch and inflate measured Pokémon XP/h.
                self.samples.clear();
                self.view.measurements = HuntMeasurements::default();
            }
            self.active_id = Some(id);
        }
        if let Some(center) = state.get("noCentro").and_then(Value::as_bool) {
            self.view.in_center = center;
        }
        if let Some(slug) = state.get("huntSlug").and_then(Value::as_str) {
            // The authenticated welcome is authoritative on a new session.
            self.adopt_hunt(slug, now);
        }
        if let Some(pokemons) = state.get("pokemons").and_then(Value::as_array) {
            self.pokemon.clear();
            for p in pokemons { self.set_pokemon(p); }
        }
        if let Some(patches) = state.get("pkMud").and_then(Value::as_array) {
            for p in patches { self.set_pokemon(p); }
        }
        self.view.pokemon_count = self.pokemon.len();
        if state.get("pokemons").is_some() || state.get("pkMud").is_some() {
            self.view.pokemon_roster = Arc::new(self.pokemon.values().cloned().collect());
        }
        self.view.active_pokemon = self.active_id.and_then(|id| self.pokemon.get(&id).cloned());
        if let Some(event) = state.get("evento") {
            if let Some(n) = event.get("xpTreinadorPct").and_then(Value::as_f64) {
                self.view.bonuses.trainer_event_pct = n;
            }
            if let Some(n) = event.get("xpPokemonPct").and_then(Value::as_f64) {
                self.view.bonuses.pokemon_event_pct = n;
            }
        }
        if let Some(n) = state.get("guildBonusPct").and_then(Value::as_f64) {
            self.view.bonuses.guild_pct = n;
        }
        if let Some(twitch) = state.get("twitch") {
            if let Some(n) = twitch.get("pctAtual").or_else(|| twitch.get("pct"))
                .and_then(Value::as_f64) {
                self.view.bonuses.twitch_pct = n;
            }
        }
        if let Some(boost) = state.get("guild").and_then(|g| Self::num(g, "boostAte")) {
            let now_server = self.view.server_now.unwrap_or(0);
            self.view.bonuses.guild_boost_active = boost > now_server && now_server > 0;
        }
        if let Some(vip_until) = Self::num(state, "vipAte") {
            let now_server = self.view.server_now.unwrap_or(0);
            self.view.bonuses.vip_active = vip_until > now_server && now_server > 0;
        }
        self.sample(now);
    }
    fn catalog(&mut self, list: &[Value]) {
        let mut hunts = Vec::new();
        for item in list {
            let Some(slug) = item.get("slug").and_then(Value::as_str)
                .filter(|s| !s.is_empty() && s.len() <= 128) else { continue };
            let species = item.get("especies").and_then(Value::as_array)
                .map(|entries| entries.iter().filter_map(|entry| {
                    let id = entry.get("pokeId").or_else(|| entry.get("speciesId"))
                        .or_else(|| entry.get("id")).and_then(Value::as_u64)?;
                    let weight = entry.get("pontos").or_else(|| entry.get("weight"))
                        .or_else(|| entry.get("peso")).and_then(Value::as_f64).unwrap_or(1.0);
                    Some(HuntSpecies { id, weight: weight.max(0.0) })
                }).collect()).unwrap_or_default();
            hunts.push(HuntEntry {
                slug: slug.to_owned(),
                name: item.get("nome").and_then(Value::as_str).unwrap_or(slug).to_owned(),
                area: item.get("area").and_then(Value::as_str).unwrap_or("").to_owned(),
                region: item.get("regiao").and_then(Value::as_str).unwrap_or("").to_owned(),
                level: Self::num(item, "nivel").unwrap_or(0).min(u32::MAX as u64) as u32,
                total_spawns: Self::num(item, "totalSpawns").unwrap_or(0).min(u32::MAX as u64) as u32,
                looktype: Self::num(item, "looktype"),
                species,
            });
        }
        self.view.catalog_ready = true;
        self.view.hunts = hunts;
    }
    pub fn ingest(&mut self, raw: &str, now: Instant) {
        let Ok(frame) = serde_json::from_str::<Value>(raw) else { return };
        match frame.get("t").and_then(Value::as_str) {
            Some("welcome") => {
                if let Some(hunts) = frame.get("hunts").and_then(Value::as_array) {
                    self.catalog(hunts);
                }
                if let Some(state) = frame.get("estado") { self.merge_state(state, now); }
            }
            Some("estado") => {
                if let Some(state) = frame.get("estado") { self.merge_state(state, now); }
            }
            Some("batalha") => {
                if let Some(events) = frame.get("ev").and_then(Value::as_array) {
                    for e in events {
                        match e.get("k").and_then(Value::as_str) {
                            Some("hunt") => if let Some(slug) = e.get("slug").and_then(Value::as_str) {
                                self.adopt_hunt(slug, now);
                                self.view.in_center = false;
                            },
                            Some("centro") => { self.view.in_center = true; },
                            Some("morte") if e.get("quem").and_then(Value::as_str) == Some("selvagem") => {
                                self.session_kills = self.session_kills.saturating_add(1);
                                self.view.battle_kills = self.session_kills;
                            }
                            Some("bola") if e.get("sucesso").and_then(Value::as_bool) == Some(true) => {
                                self.view.captures = self.view.captures.saturating_add(1);
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
        if self.pending_at.is_some_and(|at| now.saturating_duration_since(at) > Duration::from_secs(20)) {
            self.command_failed("Hunt switch not confirmed by game server");
        }
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
            valid_trainer: rate(first.trainer, last.trainer).is_some(),
            valid_pokemon: rate(first.pokemon, last.pokemon).is_some(),
            valid_gold: net_gold.is_some(),
            window_samples: self.samples.len(),
        };
        self.view.sample_quality = if kills < 5 {
            "Low confidence · fewer than five observed kills".into()
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
pub fn base_gold(level: u32) -> f64 {
    (36.0 * (level.min(25_000) as f64).powf(0.6007)).round()
}
/// A comparable *scenario* at the presently observed kill pace. Not an
/// estimate of a different hunt's actual DPS, travel or XP bonuses.
pub fn atlas_pace_scenario(hunt: &HuntEntry, measured_kills_h: f64) -> Option<(f64, f64)> {
    if measured_kills_h <= 0.0 || hunt.level == 0 { return None }
    Some((base_xp(hunt.level) * measured_kills_h,
        base_gold(hunt.level) * measured_kills_h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn catalog_pokemon_and_partial_patch_follow_manager_server_fields() {
        let mut m = HuntIntel::new();
        let at = Instant::now();
        m.ingest(&json!({"t":"welcome","hunts":[{"slug":"route-1","nome":"Route One",
            "area":"Forest","regiao":"Kanto","nivel":120,
            "especies":[{"pokeId":123,"pontos":90},{"pokeId":16,"pontos":10}]}],
            "estado":{"huntSlug":"route-1","activeId":99,"level":850,"xp":1000,
                "pokemons":[{"id":99,"nome":"Scizor","level":851,"xp":2000,"quality":1.3,
                    "ivs":{"atk":31,"speed":28}}]}}).to_string(),at);
        assert_eq!(m.view.hunts.len(),1);
        assert_eq!(m.view.hunts[0].species[0].weight,90.0);
        assert_eq!(m.view.active_pokemon.as_ref().unwrap().ivs[1],Some(31));
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
        assert_eq!(intel.view.pokemon_roster.len(),1);
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
    fn unobserved_hunts_never_claim_actual_xp_per_hour() {
        let h=HuntEntry { level: 100, ..Default::default() };
        assert_eq!(base_xp(100),6008.0);
        assert_eq!(atlas_pace_scenario(&h,0.0),None);
        assert_eq!(atlas_pace_scenario(&h,100.0).unwrap().0,600800.0);
    }
}
