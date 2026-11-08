use super::*;
/// Account-keyed combat forecasts are calculated away from egui's paint
/// path. Immutable snapshots and Arc guide data are cheap to hand to the
/// worker; no blocking game/network I/O or combat loop executes in render.
#[derive(Default)]
pub(in crate::app) struct EstimateCache {
    pub(super) account: Option<usize>,
    pub(super) catalog_ptr: usize,
    pub(super) reference_ptr: usize,
    pub(super) active_signature: Option<(u64,u32,Option<u64>,Option<u64>,[Option<u64>;6],bool,bool,Option<[u64;6]>,Option<String>,Vec<String>)>,
    pub(super) active_species_id: Option<u64>,
    pub(super) active_max_hp: Option<u64>,
    pub(super) bonus_signature: [u64;19],
    pub(super) seasonal_type:Option<String>,
    pub(super) gym_signature:Vec<(String,u8)>,
    pub(super) learned_pacing_signature:Vec<(String,String)>,
    pub(super) full_required:bool,
    pub(super) live: bool,
    pub(super) refreshed: Option<Instant>,
    // Changes on invalidation as well as publication. A timestamp alone
    // cannot distinguish multiple invalidations while forecasts are pending.
    pub(super) revision:u64,
    pub(super) failed_at:Option<Instant>,
    pub(super) last_archived:Option<(String,Instant)>,
    // Retain the receiver until an obsolete calculation exits; generation
    // cancellation prevents stale results from being published.
    pub(super) pending: Option<(u64,std::sync::mpsc::Receiver<
        std::collections::BTreeMap<String,Option<native_atlas::Forecast>>>)>,
    pub(super) generation:u64,
    pub(super) cancellation:Arc<std::sync::atomic::AtomicU64>,
    pub(super) values: std::collections::BTreeMap<String,Option<native_atlas::Forecast>>,
}
impl EstimateCache {
    pub(super) fn refresh(&mut self, account:usize,view:&HuntView,
        reference:Option<&Arc<AtlasReference>>,live:bool,aoe_enabled:bool,
    ){
        use std::sync::mpsc::TryRecvError;
        let catalog_ptr=Arc::as_ptr(&view.hunts) as usize;
        let reference_ptr=reference.map_or(0,|r|Arc::as_ptr(r) as usize);
        let active_signature=view.active_pokemon.as_ref().map(|p|
            (p.id,p.level,p.quality.map(f64::to_bits),p.potency,p.ivs,p.shiny,
                aoe_enabled,p.battle_stats.map(|stats|stats.map(f64::to_bits)),
                p.tm_elemental.clone(),p.types.clone()));
        let b=&view.bonuses;
        let bonus_signature=[
            b.vip_active as u64,b.guild_boost_active as u64,
            b.guild_pct.to_bits(),b.twitch_pct.to_bits(),b.kick_xp_pct.to_bits(),
            b.trainer_event_pct.to_bits(),b.pokemon_event_pct.to_bits(),
            b.shop_trainer_xp_multiplier.to_bits(),
            b.shop_pokemon_xp_multiplier.to_bits(),
            b.delayed_xp_multiplier.to_bits(),b.extra_loot_pct.to_bits(),
            b.guild_known as u64,b.sacrifice_xp_active as u64,
            b.authoritative_trainer_xp_multiplier.map(f64::to_bits).unwrap_or(0),
            b.authoritative_pokemon_xp_multiplier.map(f64::to_bits).unwrap_or(0),
            b.live_capture_multiplier.to_bits(),b.live_shiny_multiplier.to_bits(),
            b.raid_farm_pct.to_bits(),
            view.bicycle_factor.unwrap_or(1.0).to_bits(),
        ];
        let seasonal_type=view.seasonal_type.clone();
        let gym_signature=view.gym_podium.iter()
            .map(|(kind,place)|(kind.clone(),*place)).collect::<Vec<_>>();
        let learned_pacing_signature=view.learned_hunt_maps.iter()
            .map(|(slug,map)|(slug.clone(),map.clone())).collect::<Vec<_>>();
        let active_species_id=view.active_pokemon.as_ref().and_then(|p|p.species_id);
        let active_max_hp=view.active_pokemon.as_ref().and_then(|p|p.max_hp);
        let full_changed=self.account!=Some(account) ||
            self.catalog_ptr!=catalog_ptr ||
            self.reference_ptr!=reference_ptr || self.live!=live ||
            self.active_signature!=active_signature ||
            self.active_species_id!=active_species_id ||
            self.active_max_hp!=active_max_hp ||
            self.bonus_signature!=bonus_signature ||
            self.seasonal_type!=seasonal_type ||
            self.gym_signature!=gym_signature ||
            self.learned_pacing_signature!=learned_pacing_signature;
        if full_changed {
            self.account=Some(account);
            self.catalog_ptr=catalog_ptr;
            self.reference_ptr=reference_ptr;
            self.active_signature=active_signature;
            self.active_species_id=active_species_id;
            self.active_max_hp=active_max_hp;
            self.bonus_signature=bonus_signature;
            self.seasonal_type=seasonal_type;
            self.gym_signature=gym_signature;
            self.learned_pacing_signature=learned_pacing_signature;
            self.live=live;
            self.refreshed=None;
            self.revision=self.revision.wrapping_add(1);
            self.failed_at=None;
            self.last_archived=None;
            self.generation=self.generation.wrapping_add(1);
            self.cancellation.store(self.generation,std::sync::atomic::Ordering::Release);
            self.values.clear();
            self.full_required=true;
        }
        // A canceled worker is never published; do not spawn its replacement
        // until the old receiver has completed or disconnected.
        let complete=self.pending.as_ref().map(|(generation,rx)|
            (*generation,rx.try_recv()));
        match complete {
            Some((generation,Ok(values)))=>{
                self.pending=None;
                if generation==self.generation {
                    self.values=values;
                    self.full_required=false;
                    self.refreshed=Some(Instant::now());
                    self.revision=self.revision.wrapping_add(1);
                    self.failed_at=None;
                }
            }
            Some((generation,Err(TryRecvError::Disconnected)))=>{
                self.pending=None;
                if generation==self.generation {
                    self.failed_at=Some(Instant::now());
                }
            }
            _=>{}
        }
        if !live || self.pending.is_some() || self.refreshed.is_some() ||
            self.failed_at.is_some_and(|at|at.elapsed()<Duration::from_secs(20)) {
            return;
        }
        if !self.full_required {
            self.refreshed=Some(Instant::now());
            return;
        }
        let Some(reference)=reference else{return};
        self.failed_at=None;
        let guide=Arc::clone(reference);
        let snapshot=view.clone();
        let generation=self.generation;
        let cancellation=Arc::clone(&self.cancellation);
        let (tx,rx)=std::sync::mpsc::sync_channel(1);
        self.pending=Some((generation,rx));
        std::thread::Builder::new()
            .name("moth-hunt-forecast-cache".into())
            .spawn(move ||{
                let mut values=std::collections::BTreeMap::new();
                for hunt in snapshot.hunts.iter() {
                    if cancellation.load(std::sync::atomic::Ordering::Acquire)!=generation {
                        return;
                    }
                    values.insert(hunt.slug.clone(),
                        native_atlas::estimate_with_aoe(&snapshot,hunt,&guide,aoe_enabled));
                }
                if cancellation.load(std::sync::atomic::Ordering::Acquire)==generation {
                    let _=tx.send(values);
                }
            })
            .expect("could not start Hunt Atlas forecast worker");
    }
    pub(super) fn get(&self,slug:&str)->Option<native_atlas::Forecast>{
        self.values.get(slug).cloned().flatten()
    }
    pub(super) fn is_pending(&self)->bool{self.pending.is_some()}
    pub(super) fn archive_due(&self,slug:&str)->bool{
        self.last_archived.as_ref().is_none_or(|(previous,at)|
            previous!=slug || at.elapsed()>=Duration::from_secs(900))
    }
    pub(super) fn archived(&mut self,slug:&str){
        self.last_archived=Some((slug.to_owned(),Instant::now()));
    }
}
impl Drop for EstimateCache {
    fn drop(&mut self) {
        self.cancellation.store(u64::MAX,std::sync::atomic::Ordering::Release);
    }
}
