use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PacingCacheProfile {
    pub(super) slug:String,
    pub(super) source:crate::native_atlas_pacing::PacingSource,
    pub(super) map_id:Option<String>,
    pub(super) base_s:u64,
    pub(super) aoe_s:u64,
    pub(super) p10_s:u64,
    pub(super) p90_s:u64,
    pub(super) aoe_p10_s:u64,
    pub(super) aoe_p90_s:u64,
}
fn resolved_pacing_profiles(view:&HuntView)->Vec<PacingCacheProfile> {
    view.hunts.iter().map(|hunt|{
        let learned=view.learned_hunt_maps.get(&hunt.slug).map(String::as_str);
        let pace=crate::native_atlas_pacing::resolve(&hunt.slug,learned);
        PacingCacheProfile {
            slug:hunt.slug.clone(),
            source:pace.source,
            map_id:pace.map_id,
            base_s:pace.base_seconds_per_kill.to_bits(),
            aoe_s:pace.aoe_seconds_per_kill.to_bits(),
            p10_s:pace.uncertainty.p10.to_bits(),
            p90_s:pace.uncertainty.p90.to_bits(),
            aoe_p10_s:pace.aoe_uncertainty.p10.to_bits(),
            aoe_p90_s:pace.aoe_uncertainty.p90.to_bits(),
        }
    }).collect()
}
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
    /// Forecast-relevant XP/gold modifiers only; movement and AoE are explicit keys.
    pub(super) bonus_signature: [u64;16],
    pub(super) bicycle_factor_bits:u64,
    pub(super) aoe_enabled:bool,
    pub(super) seasonal_type:Option<String>,
    pub(super) gym_signature:Vec<(String,u8)>,
    /// Resolved per-hunt pacing profiles are part of the forecast key. The
    /// lightweight revision only tells us when this vector must be rebuilt.
    pub(super) pacing_revision:u64,
    pub(super) pacing_profiles:Vec<PacingCacheProfile>,
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
                p.tm_aoe,p.battle_stats.map(|stats|stats.map(f64::to_bits)),
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
            b.raid_farm_pct.to_bits(),
        ];
        let bicycle_factor_bits=view.bicycle_factor.unwrap_or(1.0).to_bits();
        let seasonal_type=view.seasonal_type.clone();
        let gym_signature=view.gym_podium.iter()
            .map(|(kind,place)|(kind.clone(),*place)).collect::<Vec<_>>();
        let rebuild_pacing=self.account!=Some(account) ||
            self.catalog_ptr!=catalog_ptr ||
            self.pacing_revision!=view.pacing_revision;
        let next_pacing_profiles=rebuild_pacing.then(||resolved_pacing_profiles(view));
        let pacing_changed=next_pacing_profiles.as_ref()
            .is_some_and(|profiles|profiles!=&self.pacing_profiles);
        let active_species_id=view.active_pokemon.as_ref().and_then(|p|p.species_id);
        let active_max_hp=view.active_pokemon.as_ref().and_then(|p|p.max_hp);
        let full_changed=self.account!=Some(account) ||
            self.catalog_ptr!=catalog_ptr ||
            self.reference_ptr!=reference_ptr || self.live!=live ||
            self.active_signature!=active_signature ||
            self.active_species_id!=active_species_id ||
            self.active_max_hp!=active_max_hp ||
            self.bonus_signature!=bonus_signature ||
            self.bicycle_factor_bits!=bicycle_factor_bits ||
            self.aoe_enabled!=aoe_enabled ||
            self.seasonal_type!=seasonal_type ||
            self.gym_signature!=gym_signature ||
            pacing_changed;
        if full_changed {
            self.account=Some(account);
            self.catalog_ptr=catalog_ptr;
            self.reference_ptr=reference_ptr;
            self.active_signature=active_signature;
            self.active_species_id=active_species_id;
            self.active_max_hp=active_max_hp;
            self.bonus_signature=bonus_signature;
            self.bicycle_factor_bits=bicycle_factor_bits;
            self.aoe_enabled=aoe_enabled;
            self.seasonal_type=seasonal_type;
            self.gym_signature=gym_signature;
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
        if let Some(profiles)=next_pacing_profiles {
            self.pacing_profiles=profiles;
        }
        if rebuild_pacing {
            self.pacing_revision=view.pacing_revision;
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
