//! Each profile owns its connection generation, editor drafts, feature caches,
//! and asynchronous request state. ControllerApp coordinates these four
//! runtimes and the application-wide channels rather than parallel slot arrays.
use super::*;

pub(super) struct AccountRuntime {
    pub(super) game: GameSlot,
    pub(super) session: SessionState,
    pub(super) capture: CaptureState,
    pub(super) automation: AutomationState,
    pub(super) hunt: HuntAccountState,
    pub(super) kick: KickAccountState,
    pub(super) twitch: TwitchAccountState,
    pub(super) telemetry: TelemetryState,
}

pub(super) struct SessionState {
    pub(super) native_handoff_busy: bool,
    pub(super) native_handoff_last_attempt: Option<Instant>,
    pub(super) native_verifying: bool,
    pub(super) native_error: Option<String>,
    pub(super) native_close_cancelled: bool,
    /// Pending relaunch intent remains attached to this exact account.
    pub(super) after_stop: Option<AfterStop>,
    pub(super) cached_native_busy: bool,
    /// Every asynchronous sign-in stage must match this account's generation.
    pub(super) auth_generation: Arc<AtomicU64>,
    pub(super) require_browser_login: bool,
}

pub(super) struct CaptureState {
    pub(super) capture_draft: Option<CatchSettings>,
    pub(super) restock_draft: Option<CatchSettings>,
    pub(super) restock_dirty: bool,
    pub(super) restock_retries: RestockSaveRetry,
    pub(super) restock_save_error: Option<String>,
    pub(super) capture_save_busy: bool,
    pub(super) capture_save_id: u64,
    /// Vault writes carry the captured generation across the worker boundary.
    pub(super) capture_generation: Arc<AtomicU64>,
    pub(super) capture_save_error: Option<String>,
}

pub(super) struct AutomationState {
    pub(super) automation_draft: AutomationDraft,
    pub(super) market_draft: Option<crate::native_market::MarketRules>,
    pub(super) market_snatch_draft: crate::native_market::SnatchTarget,
    pub(super) market_import: String,
    pub(super) market_notice: String,
    /// Editor-only search text. Rule IDs keep simultaneous expanded pickers independent.
    pub(super) market_item_searches: std::collections::HashMap<String,String>,
    pub(super) market_snatch_item_search: String,
}

pub(super) struct HuntAccountState {
    pub(super) offline_history: Option<crate::native_history::HistorySnapshot>,
    pub(super) hunt_action_error: Option<String>,
    pub(super) hunt_estimate_cache: hunt_panel::EstimateCache,
    pub(super) hunt_rankings: hunt_panel::RankingsCache,
    pub(super) atlas_results: hunt_panel::ExplorerCache,
    pub(super) atlas_filters: hunt_panel::AtlasFilters,
    pub(super) atlas_scenarios: hunt_panel::AtlasScenario,
}

pub(super) struct KickAccountState {
    pub(super) kick_monitor: Option<KickSupervisor>,
    pub(super) kick_api_token_input: String,
    pub(super) kick_channels_input: String,
    pub(super) kick_stream_preferences: KickStreamPreferences,
    pub(super) kick_discovered_live: Vec<String>,
    pub(super) kick_discovered_roster: Option<Vec<crate::native_kick::OfficialKickChannel>>,
    pub(super) kick_native_stamp: Option<(Instant,u64)>,
    pub(super) kick_sync_checked: Option<Instant>,
    pub(super) kick_display_cache: kick_panel::KickDisplayCache,
    pub(super) kick_discovery_available: bool,
    pub(super) kick_channel_new: String,
    pub(super) kick_saved_login: bool,
    /// Loaded in a background worker; never saved to an unprotected file.
    pub(super) kick_token_cached: Option<String>,
    pub(super) kick_token_loading: bool,
    pub(super) kick_token_checked: bool,
    pub(super) kick_token_revision: u64,
    pub(super) kick_login_done_seen: bool,
    pub(super) kick_remember_login: bool,
}

pub(super) struct TwitchAccountState {
    pub(super) login_done_seen: bool,
}

pub(super) struct TelemetryState {
    pub(super) account_setup: AccountSetup,
    pub(super) idle_progress: IdleProgressTracker,
    pub(super) xp_rates: XpRateTracker,
}

impl AccountRuntime {
    pub(super) fn new(
        index: usize,
        kick_channels_input: String,
        kick_stream_preferences: KickStreamPreferences,
    ) -> Self {
        let profile = GameProfile::from_index(index).expect("valid profile slot");
        Self {
            game: GameSlot::new(profile),
            session: SessionState {
                native_handoff_busy: false,
                native_handoff_last_attempt: None,
                native_verifying: false,
                native_error: None,
                native_close_cancelled: false,
                after_stop: None,
                cached_native_busy: false,
                auth_generation: Arc::new(AtomicU64::new(0)),
                require_browser_login: false,
            },
            capture: CaptureState {
                capture_draft: None,
                restock_draft: None,
                restock_dirty: false,
                restock_retries: RestockSaveRetry::default(),
                restock_save_error: None,
                capture_save_busy: false,
                capture_save_id: 0,
                capture_generation: Arc::new(AtomicU64::new(0)),
                capture_save_error: None,
            },
            automation: AutomationState {
                automation_draft: AutomationDraft::default(),
                market_draft: None,
                market_snatch_draft: crate::native_market::SnatchTarget {
                    kind: "item".into(),
                    currency: "gold".into(),
                    ..Default::default()
                },
                market_import: String::new(),
                market_notice: String::new(),
                market_item_searches: std::collections::HashMap::new(),
                market_snatch_item_search: String::new(),
            },
            hunt: HuntAccountState {
                offline_history: None,
                hunt_action_error: None,
                hunt_estimate_cache: hunt_panel::EstimateCache::default(),
                hunt_rankings: hunt_panel::RankingsCache::default(),
                atlas_results: hunt_panel::ExplorerCache::default(),
                atlas_filters: hunt_panel::AtlasFilters::default(),
                atlas_scenarios: hunt_panel::AtlasScenario::default(),
            },
            kick: KickAccountState {
                kick_monitor: None,
                kick_api_token_input: String::new(),
                kick_channels_input,
                kick_stream_preferences,
                kick_discovered_live: Vec::new(),
                kick_discovered_roster: None,
                kick_native_stamp: None,
                kick_sync_checked: None,
                kick_display_cache: kick_panel::KickDisplayCache::default(),
                kick_discovery_available: false,
                kick_channel_new: String::new(),
                kick_saved_login: false,
                kick_token_cached: None,
                kick_token_loading: false,
                kick_token_checked: false,
                kick_token_revision: 0,
                kick_login_done_seen: false,
                kick_remember_login: true,
            },
            twitch: TwitchAccountState {
                login_done_seen: false,
            },
            telemetry: TelemetryState {
                account_setup: AccountSetup::default(),
                idle_progress: IdleProgressTracker::default(),
                xp_rates: XpRateTracker::default(),
            },
        }
    }

    /// The same account boundary invalidates both authentication workers and
    /// encrypted capture writes. Sibling account runtimes are never touched.
    pub(super) fn invalidate_session(&mut self) {
        self.session.auth_generation.fetch_add(1, Ordering::AcqRel);
        crate::native_auth::invalidate_capture_generation(&self.capture.capture_generation);
        self.capture.capture_save_busy = false;
        self.capture.capture_save_id = self.capture.capture_save_id.wrapping_add(1);
        self.capture.capture_draft = None;
        self.capture.restock_draft = None;
        self.capture.restock_dirty = false;
        self.capture.restock_retries = RestockSaveRetry::default();
        self.capture.restock_save_error = None;
        self.capture.capture_save_error = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_runtimes_own_their_matching_profile() {
        let runtimes: [AccountRuntime; 4] = std::array::from_fn(|index|
            AccountRuntime::new(index, String::new(), KickStreamPreferences::default())
        );
        for (index, runtime) in runtimes.iter().enumerate() {
            assert_eq!(runtime.game.profile.index(), index);
        }
    }

    #[test]
    fn invalidating_one_runtime_preserves_other_accounts() {
        let mut runtimes: [AccountRuntime; 4] = std::array::from_fn(|index|
            AccountRuntime::new(index, String::new(), KickStreamPreferences::default())
        );
        runtimes[0].capture.restock_dirty = true;
        runtimes[0].capture.capture_save_busy = true;
        runtimes[1].capture.restock_dirty = true;
        runtimes[1].capture.capture_save_busy = true;
        let old_id = runtimes[0].capture.capture_save_id;
        runtimes[0].invalidate_session();
        assert!(!runtimes[0].capture.restock_dirty);
        assert!(!runtimes[0].capture.capture_save_busy);
        assert_eq!(runtimes[0].capture.capture_save_id, old_id.wrapping_add(1));
        assert_eq!(runtimes[0].session.auth_generation.load(Ordering::Acquire), 1);
        assert!(runtimes[1].capture.restock_dirty);
        assert!(runtimes[1].capture.capture_save_busy);
        assert_eq!(runtimes[1].session.auth_generation.load(Ordering::Acquire), 0);
        assert_eq!(runtimes[1].capture.capture_generation.load(Ordering::Acquire), 0);
    }
}
