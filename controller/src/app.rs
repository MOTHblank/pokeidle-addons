use crate::accounts;
use crate::icon;
use crate::config::{AccountConfig, Config, GameProfile, KickStreamPreferences, UiOptions};
use crate::firefox;
use crate::kick::KickStream;
use crate::native_kick::KickMonitor;
use crate::logging;
use crate::native_background::NativeSession;
use crate::native_gameplay::GameCommand;
use crate::native_hunts::{self, HuntView};
use crate::native_capture_config::CatchSettings;
use crate::native_items::{self, ItemCatalog};
use crate::native_visuals::{Kind as SpriteKind, VisualCache};
use crate::native_atlas::{self, Reference as AtlasReference};
// Views borrow cached state; only ControllerApp owns sessions, monitors,
// worker channels and their scheduling. Rendering must not add polling loops.
mod hunt_panel;
mod market_panel;
mod kick_panel;
mod translations;
mod localization;
mod workspace_nav;
mod workspace_state;
mod overview_panel;
mod automation_panel;
mod automation_game;
mod automation_manager;
mod automation_details;
mod automation_browser;
mod automation_streams;
mod automation_capture;
mod automation_restock;
mod automation_widgets;
mod runtime_panel;
mod accounts_panel;
mod status_widgets;
mod account_state;
mod connection_events;
mod session_actions;

// Each shared symbol is intentional: no wildcard panel imports.
use localization::{pt_br, localize_status, tr_dynamic, tr};
use workspace_nav::{draw_header_actions, draw_account_switcher, draw_options_panel, draw_sidebar, draw_instance_section};
use workspace_state::WorkspaceState;
use overview_panel::{OverviewTotals, collect_overview_totals, draw_overview_totals, request_item_catalog, draw_native_missing_panel, format_hunt_amount, draw_overview_card, selected_restock_stock, OverviewCombatState, overview_combat_state, draw_history_panel, workspace_tab_button};
use automation_panel::draw_automation_panel;
use automation_game::draw_game_automation_contents;
use automation_details::draw_game_details;
use automation_browser::draw_browser_controls;
use automation_streams::draw_stream_bonus_details;
use automation_capture::{auto_catch_setting_label, draw_native_automation};
use automation_restock::{restock_fields_equal, draw_overview_restock};
use automation_widgets::{detail_stat_line, resource_chip, automation_badge, bonus_chip};
use runtime_panel::{draw_runtime_section, draw_credits};
use accounts_panel::draw_accounts_panel;
use status_widgets::{status_chip, status_badge, runtime_chip, runtime_row, section_label, sidebar_button, compact_text};
use kick_panel::{multikick_link, open_kick_channels_in_regular_browser};
use hunt_panel::HuntSubtab;
use crate::monitor::{
    Health, MonitorHandle, MonitorPace, MONITOR_HEARTBEAT_INTERVAL,
    MONITOR_POLL_INTERVAL, MONITOR_TAB_INTERVAL,
};
use crate::update::{self, SharedUpdateStatus};
use serde_json::json;
use eframe::egui::{self, Align, Color32, FontId, Layout, Margin, RichText, Stroke, TextStyle};
use std::collections::VecDeque;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use std::sync::atomic::{AtomicBool, Ordering};

use std::sync::Arc;
#[cfg(windows)]
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

static PT_BR: AtomicBool = AtomicBool::new(true);

// Redraw cached information more often without triggering additional BiDi reads.
const UI_REPAINT_INTERVAL: Duration = Duration::from_secs(3);
const UI_BACKGROUND_REPAINT_INTERVAL: Duration = Duration::from_secs(30);

// eframe/egui reports native window focus via ViewportInfo. Treat missing
// focus information as foreground instead of disabling live monitoring on
// platforms where the OS cannot report it.
fn controller_is_background(hidden_in_tray: bool, focused: Option<bool>) -> bool {
    hidden_in_tray || focused == Some(false)
}

/// Tray state has precedence over focus. The two toggles are independent:
/// disabling blur throttling need not disable the tray resource-saving mode.
fn reduce_updates_for_window(
    hidden_in_tray: bool,
    focused: Option<bool>,
    options: UiOptions,
) -> bool {
    if hidden_in_tray {
        options.reduce_updates_in_tray
    } else {
        options.reduce_updates_unfocused && controller_is_background(false, focused)
    }
}

fn monitor_pace_for_slot(
    expanded: Option<usize>,
    index: usize,
    enabled: bool,
    background: bool,
) -> MonitorPace {
    if background {
        MonitorPace::Background
    } else if enabled && expanded == Some(index) {
        MonitorPace::Detailed
    } else {
        MonitorPace::Compact
    }
}

// Normal throttling, an old heartbeat, and transient BiDi probe failures
// must not encourage a Firefox restart. Only a confirmed session-slot
// failure or an existing Firefox profile with no monitor offers repair.
fn monitor_repair_available(
    running: bool,
    last_error: Option<&str>,
    orphaned_without_monitor: bool,
) -> bool {
    running
        && (orphaned_without_monitor
            || last_error.is_some_and(|error| {
                error.contains("Maximum number of active sessions")
            }))
}

#[cfg(test)]
mod adaptive_monitor_ui_tests {
    use super::{
        controller_is_background, monitor_pace_for_slot, monitor_repair_available,
        reduce_updates_for_window, MonitorPace, UiOptions,
    };

    #[test]
    fn inactive_window_and_system_tray_share_background_mode() {
        assert!(!controller_is_background(false, Some(true)));
        assert!(!controller_is_background(false, None)); // unknown: don't throttle
        assert!(controller_is_background(false, Some(false))); // alt-tab
        assert!(controller_is_background(true, Some(true))); // tray override
        assert!(controller_is_background(true, Some(false)));
        assert!(controller_is_background(true, None));
    }

    #[test]
    fn background_update_switches_are_independent_and_default_to_old_behavior() {
        let default = UiOptions::default();
        assert!(reduce_updates_for_window(false, Some(false), default));
        assert!(reduce_updates_for_window(true, Some(true), default));
        assert!(!reduce_updates_for_window(false, Some(true), default));

        let normal_unfocused = UiOptions {
            reduce_updates_unfocused: false,
            reduce_updates_in_tray: true,
        atlas_aoe_enabled: [false;4],
        };
        assert!(!reduce_updates_for_window(false, Some(false), normal_unfocused));
        assert!(reduce_updates_for_window(true, Some(false), normal_unfocused));

        let full_speed_tray = UiOptions {
            reduce_updates_unfocused: true,
            reduce_updates_in_tray: false,
        atlas_aoe_enabled: [false;4],
        };
        assert!(reduce_updates_for_window(false, Some(false), full_speed_tray));
        assert!(!reduce_updates_for_window(true, Some(false), full_speed_tray));

        let full_speed_everywhere = UiOptions {
            reduce_updates_unfocused: false,
            reduce_updates_in_tray: false,
        atlas_aoe_enabled: [false;4],
        };
        for hidden in [true, false] {
            for focused in [Some(true), Some(false), None] {
                assert!(!reduce_updates_for_window(hidden, focused, full_speed_everywhere));
            }
        }
    }

    #[test]
    fn only_the_visible_expanded_account_gets_fast_polling() {
        assert_eq!(monitor_pace_for_slot(Some(2), 2, true, false), MonitorPace::Detailed);
        assert_eq!(monitor_pace_for_slot(Some(2), 1, true, false), MonitorPace::Compact);
        assert_eq!(monitor_pace_for_slot(Some(2), 2, false, false), MonitorPace::Compact);
        assert_eq!(monitor_pace_for_slot(None, 2, true, false), MonitorPace::Compact);
        for index in 0..4 {
            assert_eq!(
                monitor_pace_for_slot(Some(2), index, true, true),
                MonitorPace::Background
            );
        }
        // The expanded selection remains set while focus is lost, but no
        // account may keep five-second polling.
        let background = controller_is_background(false, Some(false));
        for index in 0..4 {
            assert_eq!(
                monitor_pace_for_slot(Some(2), index, true, background),
                MonitorPace::Background
            );
        }
        assert_eq!(
            monitor_pace_for_slot(Some(2), 2, true, controller_is_background(false, Some(true))),
            MonitorPace::Detailed
        );
    }

    #[test]
    fn transient_monitor_failures_do_not_prompt_firefox_restart() {
        // Normal compact, expanded, and background monitoring never requires
        // a restart merely because its most recent reading is stale.
        assert!(!monitor_repair_available(true, None, false));
        for error in [
            "IO error: A connection attempt failed (os error 10060)",
            "Firefox has no PokéIdle app context",
            "BiDi command 7 timed out while waiting for its response",
            "IO error: An established connection was aborted (os error 10053)",
        ] {
            assert!(!monitor_repair_available(true, Some(error), false), "{error}");
        }

        // The repair button remains for actual session exhaustion or an
        // existing Firefox process whose monitor no longer exists.
        assert!(monitor_repair_available(
            true, Some("Maximum number of active sessions"), false
        ));
        assert!(monitor_repair_available(true, None, true));
        assert!(!monitor_repair_available(
            false, Some("Maximum number of active sessions"), true
        ));
    }
}

const BG: Color32 = Color32::from_rgb(11, 13, 18);
const PANEL: Color32 = Color32::from_rgb(17, 20, 27);
const PANEL_ALT: Color32 = Color32::from_rgb(22, 26, 34);
const BORDER: Color32 = Color32::from_rgb(43, 49, 61);
const TEXT: Color32 = Color32::from_rgb(235, 238, 245);
const MUTED: Color32 = Color32::from_rgb(151, 159, 174);
const DIM: Color32 = Color32::from_rgb(103, 112, 129);
const ACCENT: Color32 = Color32::from_rgb(123, 97, 255);
const GOOD: Color32 = Color32::from_rgb(72, 201, 142);
const WARN: Color32 = Color32::from_rgb(245, 180, 72);
const BAD: Color32 = Color32::from_rgb(235, 91, 91);

struct GameSlot {
    profile: GameProfile,
    native: Option<NativeSession>,
    handoff_closing: bool,
    child: Option<Child>,
    monitor: Option<MonitorHandle>,
    headless: bool,
    orphaned_pid: Option<u32>,
    // User explicitly chose visible Firefox; never auto-handoff this browser.
    browser_only: bool,
}

impl GameSlot {
    fn new(profile: GameProfile) -> Self {
        Self {
            profile,
            native: None,
            handoff_closing: false,
            child: None,
            monitor: None,
            headless: true,
            orphaned_pid: None,
            browser_only: false,
        }
    }

    fn is_running(&mut self) -> bool {
        if self.native.is_some() || self.handoff_closing { return true; }
        if let Some(child) = self.child.as_mut() {
            // On an OS inspection error, conservatively keep ownership rather
            // than offer a duplicate launch into a potentially locked profile.
            match child.try_wait() {
                Ok(None) | Err(_) => return true,
                Ok(Some(_)) => {}
            }
        }
        if let Some(pid) = self.orphaned_pid {
            if firefox::process_alive(pid) {
                return true;
            }
            self.orphaned_pid = None;
        }
        false
    }

    fn health(&self) -> Health {
        if self.handoff_closing {
            let mut health = Health::default();
            health.state = "Connecting".into();
            return health;
        }
        if let Some(native) = self.native.as_ref() {
            let state = native.snapshot();
            let mut health = Health::default();
            health.state = if state.phase == "Online" { "Running".into() }
                else { state.phase.clone() };
            // Project the same authoritative native account domains used by
            // Hunts / Inventory. A Firefox Bridge snapshot is not required.
            let view = native.hunt_snapshot();
            health.activity = if view.in_center { "Center".into() }
                else if view.hunt_slug.is_some() { "Hunting".into() }
                else { "Online".into() };
            health.active_pokemon = view.active_pokemon.as_ref()
                .map(|pokemon| pokemon.name.clone()).unwrap_or_default();
            health.pokemon_level = view.active_pokemon.as_ref()
                .map(|pokemon| pokemon.level.to_string()).unwrap_or_default();
            health.pokemon_xp = view.active_pokemon.as_ref()
                .and_then(|pokemon| pokemon.xp).map(|xp| xp.to_string())
                .unwrap_or_default();
            health.fallen_count = view.pokemon_roster.iter()
                .filter(|pokemon| pokemon.hp == Some(0)).count()
                .min(u32::MAX as usize) as u32;
            health.ball_stock = view.balls.iter().map(|(id, qty)| {
                format!("{}: {}", view.ball_names.get(id)
                    .map(String::as_str).unwrap_or("Ball"), qty)
            }).collect();
            health.autocatch_rate = format!("{} throws · {} captures",
                state.native_throws, state.captures);
            health.autocatch_restock = state.native_restock.phase.clone();
            health.xp_bonuses = vec![
                format!("Twitch +{:.0}%", view.bonuses.twitch_pct),
                format!("Guild +{:.0}%", view.bonuses.guild_pct),
                format!("Trainer event +{:.0}%", view.bonuses.trainer_event_pct),
                format!("Pokémon event +{:.0}%", view.bonuses.pokemon_event_pct),
            ];
            // Existing widgets consume hunt_atlas, but never claim a
            // speculative rate is a measured one.
            health.hunt_atlas = json!({
                "huntSlug": view.hunt_slug.clone(),
                "xpH": if view.measurements.valid_trainer {
                    Some(view.measurements.trainer_xp_h)
                } else { None },
                "coinsH": if view.measurements.valid_gold {
                    Some(view.measurements.gold_net_h)
                } else { None },
                "xpMeasured": view.measurements.valid_trainer,
            });
            health.bridge_connected = state.phase == "Online";
            health.bridge_snapshot_ok = state.phase == "Online";
            health.autocatch_on = state.native_auto_catch;
            health.autocatch_balls_used = state.native_throws.min(u32::MAX as u64) as u32;
            health.autocatch_captures = state.captures.min(u32::MAX as u64) as u32;
            health.stream_watching = state.twitch_watching.clone();
            health.stream_bonus_pct = state.twitch_bonus_pct;
            health.stream_scan_twitch_channels = Some(state.twitch_live.len() as u32);
            health.stream_bonus = if state.twitch_bonus_pct > 0.0 {
                format!("+{}% XP", state.twitch_bonus_pct)
            } else { String::new() };
            health.game_ready = state.phase == "Online";
            health.logged_in = state.phase == "Online";
            health.hunt_slug = view.hunt_slug.clone().or(state.hunt_slug.clone()).unwrap_or_default();
            health.hunt = view.hunts.iter()
                .find(|hunt| hunt.slug == health.hunt_slug)
                .map(|hunt| hunt.name.clone())
                .unwrap_or_else(|| health.hunt_slug.clone());
            health.player_level = view.trainer_level.or(state.trainer_level).unwrap_or_default();
            health.player_total_xp = view.trainer_xp.or(state.trainer_xp);
            health.player_xp = health.player_total_xp.map(|xp| xp.to_string()).unwrap_or_default();
            health.gold = view.gold.or(state.gold).unwrap_or_default();
            health.orbs = view.orbs.or(state.orbs).unwrap_or_default();
            health.last_successful_probe_at = state.last_received;
            health.last_heartbeat_at = state.last_received;
            health.last_error = state.error;
            return health;
        }
        // The launcher process can exit while the exact-profile Firefox process
        // and its BiDi session remain alive. Preserve live monitoring in that case.
        if let Some(monitor) = self.monitor.as_ref() {
            return monitor.health();
        }
        if self.orphaned_pid.is_some() {
            let mut health = Health::default();
            health.state = "Existing Firefox".to_string();
            return health;
        }
        Health::default()
    }
}

#[derive(Clone, Debug, Default)]
struct IdleProgressTracker {
    last_xp: Option<String>,
    last_level: Option<u32>,
    last_balls_used: Option<u32>,
    last_ball_stock: Vec<String>,
    unchanged_since: Option<Instant>,
    last_sample_at: Option<Instant>,
    warning: bool,
}

impl IdleProgressTracker {
    /// Progress is evaluated only when a new successful BiDi sample arrives.
    /// Re-rendering the same sample must never produce a false inactivity alert.
    fn observe(&mut self, health: &Health) -> bool {
        let Some(sample_at) = health.last_successful_probe_at else {
            return false;
        };
        if self.last_sample_at == Some(sample_at) {
            return self.warning;
        }

        let xp = health.player_xp.clone();
        let level = health.player_level;
        let balls_used = health.autocatch_balls_used;
        let changed = self.last_xp.as_ref() != Some(&xp)
            || self.last_level != Some(level)
            || self.last_balls_used != Some(balls_used)
            || ball_stock_decreased(&self.last_ball_stock, &health.ball_stock);

        if changed || self.unchanged_since.is_none() {
            self.last_xp = Some(xp);
            self.last_level = Some(level);
            self.last_balls_used = Some(balls_used);
            self.unchanged_since = Some(sample_at);
            self.warning = false;
        } else {
            self.warning = self.unchanged_since.is_some_and(|since| {
                sample_at.duration_since(since) >= Duration::from_secs(5 * 60)
            });
        }

        self.last_ball_stock = health.ball_stock.clone();
        self.last_sample_at = Some(sample_at);
        self.warning
    }
}

/// Estimate earned trainer XP per hour from a short rolling history.
/// Samples are cumulative XP and monotonic timestamps, never UI redraws.
/// A four-to-six-sample window smooths combat bursts without endless averaging.
#[derive(Debug, Default, Clone)]
struct XpRateTracker {
    samples: VecDeque<(Instant, u64)>,
    rate_per_hour: Option<f64>,
}

impl XpRateTracker {
    const MAX_SAMPLES: usize = 6;
    const MAX_WINDOW: Duration = Duration::from_secs(20 * 60);

    fn observe(&mut self, health: &Health) -> Option<f64> {
        if !health.telemetry_is_fresh() || !health.logged_in || !health.bridge_snapshot_ok {
            *self = Self::default();
            return None;
        }
        let (Some(at), Some(total)) =
            (health.last_successful_probe_at, health.player_total_xp)
        else {
            *self = Self::default();
            return None;
        };

        if let Some(&(previous_at, previous_total)) = self.samples.back() {
            // Ignore cached UI redraws and older readings.
            if at <= previous_at {
                return self.rate_per_hour;
            }
            // A reset, character switch, or unexpected counter decrease
            // must never produce an absurd negative rate.
            if total < previous_total
                || at.duration_since(previous_at) > Self::MAX_WINDOW
            {
                // A long gap would blend inactive time into the rate. Begin
                // again instead of displaying an unreliable average.
                self.samples.clear();
                self.rate_per_hour = None;
            }
        }
        self.samples.push_back((at, total));
        while self.samples.len() > Self::MAX_SAMPLES
            || self.samples.front().is_some_and(|(old_at, _)|
                at.duration_since(*old_at) > Self::MAX_WINDOW
                && self.samples.len() > 2)
        {
            self.samples.pop_front();
        }

        self.rate_per_hour = self.samples.front().zip(self.samples.back())
            .and_then(|(&(start_at, start_total), &(end_at, end_total))| {
                let seconds = end_at.duration_since(start_at).as_secs_f64();
                if seconds < 60.0 { return None; }
                let earned = end_total.checked_sub(start_total)?;
                Some((earned as f64 * 3600.0) / seconds)
            });
        self.rate_per_hour
    }
}

#[cfg(test)]
mod xp_rate_tests {
    use super::{Health, XpRateTracker};
    use std::time::{Duration, Instant};

    fn sample(at: Instant, total: Option<u64>) -> Health {
        let mut health = Health::default();
        health.state = "Running".into();
        health.logged_in = true;
        health.bridge_snapshot_ok = true;
        health.player_total_xp = total;
        health.last_successful_probe_at = Some(at);
        health
    }

    #[test]
    fn rate_normalizes_elapsed_time_and_ignores_ui_redraws() {
        let now = Instant::now();
        let mut tracker = XpRateTracker::default();
        let baseline = sample(now - Duration::from_secs(180), Some(1500));
        assert_eq!(tracker.observe(&baseline), None);
        assert_eq!(tracker.observe(&baseline), None);
        let next = sample(now, Some(1620));
        assert_eq!(tracker.observe(&next), Some(2400.0));
        assert_eq!(tracker.observe(&next), Some(2400.0));
        assert_eq!(tracker.observe(&baseline), Some(2400.0));
    }

    #[test]
    fn gains_are_smoothed_across_readings_and_level_ups() {
        let now = Instant::now();
        let mut tracker = XpRateTracker::default();
        tracker.observe(&sample(now - Duration::from_secs(240), Some(9800)));
        assert_eq!(tracker.observe(&sample(now - Duration::from_secs(120), Some(10100))), Some(9000.0));
        // No progress in the latest interval lowers the rolling rate.
        assert_eq!(tracker.observe(&sample(now, Some(10100))), Some(4500.0));
        // A genuine counter reset creates a new baseline.
        assert_eq!(tracker.observe(&sample(now + Duration::from_secs(1), Some(50))), None);
    }

    #[test]
    fn stale_or_missing_game_data_clears_rate() {
        let now = Instant::now();
        let mut tracker = XpRateTracker::default();
        tracker.observe(&sample(now - Duration::from_secs(180), Some(20)));
        tracker.observe(&sample(now, Some(40)));
        assert_eq!(tracker.observe(&sample(now, None)), None);
        assert_eq!(tracker.observe(&sample(now, Some(50))), None);
        let mut disconnected = sample(now, Some(70));
        disconnected.state = "Reconnecting".into();
        assert_eq!(tracker.observe(&disconnected), None);
        assert_eq!(tracker.observe(&sample(now, Some(70))), None);
    }

    #[test]
    fn long_gap_does_not_produce_misleading_exp_rate() {
        let now = Instant::now();
        let mut tracker = XpRateTracker::default();
        tracker.samples.push_back((now - Duration::from_secs(25 * 60), 1000));
        tracker.rate_per_hour = Some(5000.0);
        assert_eq!(tracker.observe(&sample(now, Some(4000))), None);
        assert_eq!(tracker.samples.len(), 1);
    }


}

#[cfg(test)]
mod idle_progress_tests {
    use super::{Health, IdleProgressTracker};
    use std::time::{Duration, Instant};

    #[test]
    fn unchanged_rendering_does_not_create_an_idle_warning() {
        let base = Instant::now() - Duration::from_secs(11 * 60);
        let mut sample = Health::default();
        sample.player_xp = "100".to_string();
        sample.player_level = 5;
        sample.last_successful_probe_at = Some(base);
        let mut tracker = IdleProgressTracker::default();

        assert!(!tracker.observe(&sample));
        for _ in 0..100 {
            assert!(!tracker.observe(&sample));
        }
        sample.last_successful_probe_at = Some(base + Duration::from_secs(5 * 60));
        assert!(tracker.observe(&sample));
        sample.player_xp = "101".to_string();
        sample.last_successful_probe_at = Some(base + Duration::from_secs(10 * 60));
        assert!(!tracker.observe(&sample));
    }
}

fn ball_stock_decreased(previous: &[String], current: &[String]) -> bool {
    previous.iter().any(|before| {
        let Some((before_name, before_count)) = before.rsplit_once(' ') else {
            return false;
        };
        let Ok(before_count) = before_count.parse::<u64>() else {
            return false;
        };

        let after_count = current
            .iter()
            .find_map(|item| {
                let (name, count) = item.rsplit_once(' ')?;
                if name == before_name {
                    count.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);

        after_count < before_count
    })
}

#[derive(Clone, Debug, Default)]
struct AccountSetup {
    profile_ready: bool,
    setup_complete: bool,
    check_error: Option<String>,
}

struct UiEvent {
    at: Instant,
    message: String,
    is_error: bool,
}

#[cfg(windows)]
struct TrayRuntime {
    _icon: TrayIcon,
    stop_worker: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

#[cfg(windows)]
impl TrayRuntime {
    fn new(ctx: egui::Context, hidden: Arc<AtomicBool>) -> Result<Self, String> {
        let menu = Menu::new();
        let show_item = MenuItem::new("Mostrar / Show Moth", true, None);
        let separator = PredefinedMenuItem::separator();
        let quit_item = MenuItem::new("Sair / Exit", true, None);

        menu.append(&show_item)
            .map_err(|error| format!("could not add tray Show menu item: {error}"))?;
        menu.append(&separator)
            .map_err(|error| format!("could not add tray separator: {error}"))?;
        menu.append(&quit_item)
            .map_err(|error| format!("could not add tray Exit menu item: {error}"))?;

        let icon = TrayIconBuilder::new()
            .with_id("moth-pokeidle-controller")
            .with_tooltip("Moth · PokéIdle")
            .with_icon(make_tray_icon()?)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .map_err(|error| format!("could not create system tray icon: {error}"))?;

        let tray_id = icon.id().clone();
        let show_id = show_item.id().clone();
        let quit_id = quit_item.id().clone();
        let stop_worker = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop_worker);

        let worker = std::thread::Builder::new()
            .name("moth-tray-events".to_string())
            .spawn(move || {
                while !worker_stop.load(Ordering::Acquire) {
                    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                        match event {
                            TrayIconEvent::Click {
                                id,
                                button: MouseButton::Left,
                                button_state: MouseButtonState::Up,
                                ..
                            } if id == tray_id => {
                                restore_from_tray(&ctx, &hidden);
                            }
                            TrayIconEvent::DoubleClick {
                                id,
                                button: MouseButton::Left,
                                ..
                            } if id == tray_id => {
                                restore_from_tray(&ctx, &hidden);
                            }
                            _ => {}
                        }
                    }

                    while let Ok(event) = MenuEvent::receiver().try_recv() {
                        if event.id == show_id {
                            restore_from_tray(&ctx, &hidden);
                        } else if event.id == quit_id {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            ctx.request_repaint();
                        }
                    }

                    std::thread::sleep(Duration::from_millis(100));
                }
            })
            .map_err(|error| format!("could not start tray event worker: {error}"))?;

        Ok(Self {
            _icon: icon,
            stop_worker,
            worker: Some(worker),
        })
    }

    fn shutdown(&mut self) {
        self.stop_worker.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(windows)]
impl Drop for TrayRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(windows)]
fn restore_from_tray(ctx: &egui::Context, hidden: &AtomicBool) {
    hidden.store(false, Ordering::Relaxed);
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.request_repaint();
}

#[cfg(windows)]
fn make_tray_icon() -> Result<tray_icon::Icon, String> {
    const SIZE: u32 = 32;
    tray_icon::Icon::from_rgba(icon::rgba(SIZE), SIZE, SIZE)
        .map_err(|error| format!("could not create tray icon image: {error}"))
}

struct NativeHandoffEvent {
    index: usize,
    result: Result<serde_json::Value, String>,
    cache_warning: Option<String>,
}

struct CachedNativeEvent {
    index: usize,
    existing_firefox: bool,
    result: Result<Option<NativeSession>, String>,
}

/// Authenticated Rust game WSS + Twitch IRC must both be confirmed before
/// the complete Firefox process may close.
struct NativeVerifiedEvent {
    index: usize,
    result: Result<NativeSession, String>,
    cache_warning: Option<String>,
}

/// The child process and monitor are owned exclusively by the shutdown worker.
/// Returning them to the UI on error preserves ownership for retry/recovery.
struct NativeBrowserClosedEvent {
    index: usize,
    native: NativeSession,
    cache_warning: Option<String>,
    child: Option<Child>,
    orphaned_pid: Option<u32>,
    monitor: Option<MonitorHandle>,
    result: Result<(), String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PanelTab { Overview, Hunts, Accounts, Automation, Options }

#[derive(Clone, Copy, PartialEq, Eq)]
enum AccountsSection { Profiles, Kick }

/// Profile commands are application actions, not sidebar presentation state.
#[derive(Clone, Copy)]
enum ProfileAction {
    GoogleSignIn,
    Twitch,
    Folder,
}


/// Account-local automation workspaces. Keep the selected section while
/// switching accounts; the scroll position is isolated per account/section.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AutomationSection { Capture, Game, Market }
#[derive(Clone,Default)]
struct AutomationDraft {
    initialized: bool,
    hp_limit: f64,
    lock_grade: f64,
}
impl AutomationDraft {
    fn from_server(value:&serde_json::Value) -> Self {
        Self { initialized:true,
            hp_limit:value.get("hpLimiar").and_then(serde_json::Value::as_f64).unwrap_or(0.0),
            lock_grade:value.get("autoLockNotaMin").and_then(serde_json::Value::as_f64).unwrap_or(0.0) }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum HuntRanking { TrainerXp, PokemonXp, Gold }

#[derive(Clone, Copy, PartialEq, Eq)]
enum CaptureSaveOrigin { Editor, Toggle, Restock }

struct CaptureSavedEvent {
    index:usize,
    config:CatchSettings,
    origin:CaptureSaveOrigin,
    result:Result<(),String>
}
pub struct ControllerApp {
    workspace: WorkspaceState,
    native_handoff_tx: Sender<NativeHandoffEvent>,
    native_handoff_rx: Receiver<NativeHandoffEvent>,
    native_handoff_busy: [bool; 4],
    native_handoff_last_attempt: [Option<Instant>; 4],
    native_verified_tx: Sender<NativeVerifiedEvent>,
    native_verified_rx: Receiver<NativeVerifiedEvent>,
    native_verifying: [bool; 4],
    native_error: [Option<String>; 4],
    native_closed_tx: Sender<NativeBrowserClosedEvent>,
    native_closed_rx: Receiver<NativeBrowserClosedEvent>,
    native_close_cancelled: [bool; 4],
    cached_native_tx: Sender<CachedNativeEvent>,
    cached_native_rx: Receiver<CachedNativeEvent>,
    cached_native_busy: [bool; 4],
    require_browser_login: [bool; 4],
    games: [GameSlot; 4],





    ui_options: UiOptions,



    automation_drafts: [AutomationDraft;4],
    market_drafts: [Option<crate::native_market::MarketRules>;4],
    market_snatch_drafts: [crate::native_market::SnatchTarget;4],
    market_imports: [String;4],
    market_notices: [String;4],



    offline_histories: [Option<crate::native_history::HistorySnapshot>;4],






    hunt_action_error: [Option<String>;4],
    hunt_estimate_cache: [hunt_panel::EstimateCache;4],
    atlas_results: [hunt_panel::ExplorerCache;4],
    atlas_filters: [hunt_panel::AtlasFilters;4],
    atlas_scenarios: [hunt_panel::AtlasScenario;4],

    hunt_reference: Option<Arc<AtlasReference>>,
    hunt_reference_error: Option<String>,
    hunt_reference_busy: bool,
    hunt_reference_tx: Sender<Result<AtlasReference,String>>,
    hunt_reference_rx: Receiver<Result<AtlasReference,String>>,
    capture_drafts:[Option<CatchSettings>;4],
    // Restock has optimistic per-account editing state. Never rebuild the
    // picker from the last confirmed worker snapshot while a save is in flight.
    restock_drafts:[Option<CatchSettings>;4],
    restock_dirty:[bool;4],

    capture_save_busy:[bool;4],
    capture_save_error:[Option<String>;4],
    capture_save_tx:Sender<CaptureSavedEvent>,
    capture_save_rx:Receiver<CaptureSavedEvent>,

    inventory_catalog:Option<Arc<ItemCatalog>>,
    visuals:VisualCache,
    inventory_catalog_error:Option<String>,
    inventory_catalog_busy:bool,
    inventory_catalog_tx:Sender<Result<ItemCatalog,String>>,
    inventory_catalog_rx:Receiver<Result<ItemCatalog,String>>,
    accounts: [AccountConfig; 4],
    /// Kick website sessions run independently per account. Bearer tokens
    /// are entered in masked fields and optionally stored in the OS vault.
    kick_monitors: [Option<KickMonitor>; 4],
    kick_api_token_input: [String; 4],
    kick_channels_input: [String; 4],
    kick_stream_preferences: [KickStreamPreferences; 4],
    // Live official channels are discovered from the native PokéIdle socket,
    // not copied into the user's saved manual preferences.
    kick_discovered_live: [Vec<String>; 4],
    // Only changes in the native official roster trigger Kick worker reconciliation.
    kick_discovered_roster: [Option<Vec<crate::native_kick::OfficialKickChannel>>; 4],
    kick_native_stamp: [Option<(Instant,u64)>; 4],
    kick_sync_checked: [Option<Instant>; 4],
    kick_display_cache: [kick_panel::KickDisplayCache; 4],
    kick_discovery_available: [bool; 4],
    kick_channel_new: [String; 4],
    kick_saved_login: [bool; 4],
    kick_remember_login: [bool; 4],

    status: String,
    status_error: bool,
    events: VecDeque<UiEvent>,
    account_setup: [AccountSetup; 4],
    account_setup_checked_at: Option<Instant>,


    idle_progress: [IdleProgressTracker; 4],
    xp_rates: [XpRateTracker; 4],

    update_status: SharedUpdateStatus,
    #[cfg(windows)]
    tray: Option<TrayRuntime>,
    #[cfg(windows)]
    tray_hidden: Arc<AtomicBool>,
}

impl ControllerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);

        logging::init();
        logging::info(&format!(
            "Moth native controller v{} · verified TLS provider initialized: {}",
            env!("CARGO_PKG_VERSION"),
            rustls::crypto::CryptoProvider::get_default().is_some()
        ));
        logging::info("controller UI initialized with egui/eframe");
        let update_status = update::start_update_check();
        let (native_handoff_tx, native_handoff_rx) = mpsc::channel();
        let (cached_native_tx, cached_native_rx) = mpsc::channel();
        let (native_closed_tx, native_closed_rx) = mpsc::channel();
        let (native_verified_tx, native_verified_rx) = mpsc::channel();
        let (hunt_reference_tx, hunt_reference_rx) = mpsc::channel();
        let (capture_save_tx,capture_save_rx)=mpsc::channel();
        let (inventory_catalog_tx,inventory_catalog_rx)=mpsc::channel();

        #[cfg(windows)]
        let tray_hidden = Arc::new(AtomicBool::new(false));
        #[cfg(windows)]
        let tray = match TrayRuntime::new(cc.egui_ctx.clone(), Arc::clone(&tray_hidden)) {
            Ok(tray) => Some(tray),
            Err(error) => {
                logging::warn(&format!("system tray initialization failed: {error}"));
                None
            }
        };

        Self {
            native_handoff_tx,
            native_handoff_rx,
            native_handoff_busy: [false; 4],
            native_handoff_last_attempt: [None; 4],
            native_verified_tx,
            native_verified_rx,
            native_verifying: [false; 4],
            native_error: std::array::from_fn(|_| None),
            native_closed_tx,
            native_closed_rx,
            native_close_cancelled: [false; 4],
            cached_native_tx,
            cached_native_rx,
            cached_native_busy: [false; 4],
            require_browser_login: [false; 4],
            games: std::array::from_fn(|i| {
                GameSlot::new(GameProfile::from_index(i).expect("valid profile slot"))
            }),
            workspace: WorkspaceState::default(),





            ui_options: Config::load_ui_options().unwrap_or_else(|error| {
                logging::warn(&format!("controller options load failed: {error}"));
                UiOptions::default()
            }),



            automation_drafts: std::array::from_fn(|_|AutomationDraft::default()),
            market_drafts: std::array::from_fn(|_|None),
            market_snatch_drafts: std::array::from_fn(|_|crate::native_market::SnatchTarget{
                kind:"item".into(),currency:"gold".into(),..Default::default()
            }),
            market_imports: std::array::from_fn(|_|String::new()),
            market_notices: std::array::from_fn(|_|String::new()),



            offline_histories: std::array::from_fn(|_|None),






            hunt_action_error: std::array::from_fn(|_|None),
            hunt_estimate_cache: std::array::from_fn(|_|hunt_panel::EstimateCache::default()),
            atlas_results: std::array::from_fn(|_|hunt_panel::ExplorerCache::default()),
            atlas_filters: std::array::from_fn(|_|hunt_panel::AtlasFilters::default()),
            atlas_scenarios: std::array::from_fn(|_|hunt_panel::AtlasScenario::default()),

            hunt_reference: None,
            hunt_reference_error: None,
            hunt_reference_busy: false,
            hunt_reference_tx,
            hunt_reference_rx,
            capture_drafts:std::array::from_fn(|_|None),
            restock_drafts:std::array::from_fn(|_|None),
            restock_dirty:[false;4],

            capture_save_busy:[false;4],
            capture_save_error:std::array::from_fn(|_|None),
            capture_save_tx,
            capture_save_rx,

            inventory_catalog:None,
            visuals:VisualCache::default(),
            inventory_catalog_error:None,
            inventory_catalog_busy:false,
            inventory_catalog_tx,
            inventory_catalog_rx,
            kick_monitors: std::array::from_fn(|_| None),
            kick_api_token_input: std::array::from_fn(|_| String::new()),
            kick_channels_input: Config::load_kick_channels().unwrap_or_else(|error| {
                logging::warn(&error);
                std::array::from_fn(|_| String::new())
            }),
            kick_stream_preferences: Config::load_kick_stream_preferences()
                .unwrap_or_else(|error| {
                    logging::warn(&error);
                    std::array::from_fn(|_| KickStreamPreferences::default())
                }),
            kick_discovered_live: std::array::from_fn(|_| Vec::new()),
            kick_discovered_roster: std::array::from_fn(|_| None),
            kick_native_stamp: [None; 4],
            kick_sync_checked: [None; 4],
            kick_display_cache: std::array::from_fn(|_|kick_panel::KickDisplayCache::default()),
            kick_discovery_available: [false; 4],
            kick_channel_new: std::array::from_fn(|_| String::new()),
            kick_saved_login: std::array::from_fn(|index| {
                GameProfile::from_index(index).is_some_and(|profile|
                    crate::native_auth::load_kick_token(profile)
                        .ok().flatten().is_some())
            }),
            kick_remember_login: [true; 4],

            status: "Ready · launch only the profiles you need".to_string(),
            status_error: false,
            events: VecDeque::new(),
            account_setup: std::array::from_fn(|_| AccountSetup::default()),
            account_setup_checked_at: None,


            idle_progress: std::array::from_fn(|_| IdleProgressTracker::default()),
            xp_rates: std::array::from_fn(|_| XpRateTracker::default()),

            update_status,
            #[cfg(windows)]
            tray,
            #[cfg(windows)]
            tray_hidden,
            accounts: Config::load_accounts().unwrap_or_else(|error| {
                logging::warn(&format!("account configuration load failed: {error}"));
                std::array::from_fn(|i| AccountConfig::default_for(GameProfile::from_index(i).expect("valid account slot")))
            }),
        }
    }

    fn idle_warning_for(&mut self, index: usize, health: &Health) -> bool {
        let tracker = &mut self.idle_progress[index];
        let eligible = health.telemetry_is_fresh()
            && health.game_ready
            && health.logged_in
            && health.bridge_connected
            && health.activity == "Hunting"
            && !health.player_xp.is_empty();

        if !eligible {
            *tracker = IdleProgressTracker::default();
            return false;
        }

        tracker.observe(health)
    }

    fn set_status(&mut self, message: impl Into<String>, error: bool) {
        self.status = message.into();
        self.status_error = error;
        self.events.push_front(UiEvent {
            at: Instant::now(),
            message: self.status.clone(),
            is_error: error,
        });
        self.events.truncate(80);
        if error {
            logging::warn(&self.status);
        }
    }


}

impl Drop for ControllerApp {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let Some(tray) = self.tray.as_mut() {
            tray.shutdown();
        }

        for slot in &mut self.games {
            if let Some(mut child) = slot.child.take() {
                let account_name = self.accounts[slot.profile.index()].name.clone();
                logging::info(&format!(
                    "controller shutting down; closing {} Firefox PID {}",
                    account_name,
                    child.id()
                ));
                if let Err(error) = firefox::stop_child(&mut child, slot.monitor.as_ref()) {
                    logging::error(&format!("{} shutdown failed: {}", account_name, error));
                }
            }
            if let Some(monitor) = slot.monitor.take() {
                monitor.stop();
            }
            // Recovered processes are intentionally not owned by this controller
            // instance. Only an explicit Stop action may close those browsers.
        }
    }
}

impl eframe::App for ControllerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.refresh_processes();
        self.sync_pokeidle_kick_streamers();
        // Drain replies from requests made on the previous UI frame before
        // resetting the current frame's Pokémon sprite usage.
        self.visuals.poll(ui.ctx());
        self.visuals.begin_frame();
        if self.account_setup_checked_at
            .map(|when| when.elapsed() >= Duration::from_secs(5 * 60))
            .unwrap_or(true)
        {
            self.refresh_account_setup();
        }

        // Live native panels publish snapshots at one-second cadence.
        // Repaint only while visible; tray/background retains the cheap timer.
        let background = self.controller_in_background(ui.ctx());
        let native_detail = self.games.iter().any(|game| game.native.is_some())
            && (matches!(self.workspace.panel_tab, PanelTab::Hunts | PanelTab::Automation)
                || self.workspace.panel_tab == PanelTab::Overview);
        ui.ctx().request_repaint_after(
            if background { UI_BACKGROUND_REPAINT_INTERVAL }
            else if native_detail { Duration::from_secs(1) }
            else { UI_REPAINT_INTERVAL }
        );

        draw_sidebar(self, ui);

        egui::Panel::top("header")
            .frame(egui::Frame::new().fill(BG).inner_margin(Margin::symmetric(20, 13)))
            .show(ui, |ui| {
                // On smaller windows give actions their own row instead of
                // allowing right-aligned buttons to overlap the page title.
                let compact = ui.available_width() < 780.0;
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr(match self.workspace.panel_tab {
                            PanelTab::Overview => "Overview",
                            PanelTab::Hunts => "Hunt Atlas",
                            PanelTab::Accounts => "Accounts",
                            PanelTab::Automation => "Automation",
                            PanelTab::Options => "Options",
                        }))
                            .font(FontId::proportional(23.0))
                            .strong()
                            .color(TEXT),
                    );
                    if !compact {
                        ui.add_space(12.0);
                        ui.label(RichText::new(tr("PokéIdle controller"))
                            .size(12.0).color(MUTED));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            draw_header_actions(self, ui);
                        });
                    }
                });
                if compact {
                    ui.add_space(5.0);
                    ui.horizontal_wrapped(|ui| draw_header_actions(self, ui));
                }
            });

        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0, BORDER))
                    .inner_margin(Margin::symmetric(24, 11)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let dot_color = if self.status_error { BAD } else { GOOD };
                    ui.colored_label(dot_color, "●");
                    ui.add_space(6.0);
                    let localized = localize_status(self.status.clone());
                    ui.add(egui::Label::new(
                        RichText::new(&localized)
                            .size(12.0)
                            .color(if self.status_error { TEXT } else { MUTED }),
                    ).truncate()).on_hover_text(localized);
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(Margin::symmetric(16, 10)))
            .show(ui, |ui| {
                // Accounts has its own full page with an account selector.
                // All other native workspaces share the same compact strip.
                let active = match self.workspace.panel_tab {
                    PanelTab::Overview => Some(self.workspace.selected_details),
                    PanelTab::Hunts if self.workspace.hunt_subtab == HuntSubtab::Compare => None,
                    PanelTab::Hunts => Some(Some(self.workspace.hunt_account)),
                    PanelTab::Automation => Some(Some(self.workspace.automation_account)),
                    _ => None,
                };
                if let Some(selected) = active {
                    let next = draw_account_switcher(
                        self, ui, selected, self.workspace.panel_tab == PanelTab::Overview
                    );
                    if next != selected {
                        if let Some(index) = next {
                            self.select_workspace_account(index);
                            if self.workspace.panel_tab == PanelTab::Overview {
                                self.workspace.selected_details = Some(index);
                            }
                        } else if self.workspace.panel_tab == PanelTab::Overview {
                            self.workspace.selected_details = None;
                        }
                    }
                    ui.add_space(6.0);
                }
                match self.workspace.panel_tab {
                    PanelTab::Overview => draw_instance_section(self, ui),
                    PanelTab::Hunts => hunt_panel::draw_hunt_atlas_panel(self, ui),
                    PanelTab::Accounts => draw_accounts_panel(self, ui),
                    PanelTab::Automation => draw_automation_panel(self, ui),
                    PanelTab::Options => draw_options_panel(self, ui),
                }
            });

        // Keep exactly the Pokémon art used by Overview this frame (at most
        // two sprites per account). Hunt Atlas and other tabs keep none.
        self.visuals.end_frame();

        // Only keep Explorer results for the account actually being viewed.
        // Rankings can be rebuilt on demand; holding all four accounts'
        // destination copies throughout Overview/Automation wastes RAM.
        let active_explorer=if self.workspace.panel_tab==PanelTab::Hunts &&
            self.workspace.hunt_subtab==HuntSubtab::Explorer {
            Some(self.workspace.hunt_account)
        } else {None};
        for (index,results) in self.atlas_results.iter_mut().enumerate() {
            if active_explorer!=Some(index) {results.release();}
        }

        // If the user leaves Accounts while editing the name, persist it
        // without needing to close a dialog that no longer exists.
        if self.workspace.panel_tab != PanelTab::Accounts {
            self.save_dirty_account_names();
        }
        // Apply the persisted preference immediately to all Firefox monitors.
        // Neither native game nor Twitch connections are throttled.
        self.sync_monitor_paces(self.controller_in_background(ui.ctx()));
    }
}

fn configure_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.interact_size.y = 30.0;

        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(22.0)),
            (TextStyle::Body, FontId::proportional(13.0)),
            (TextStyle::Button, FontId::proportional(12.0)),
            (TextStyle::Small, FontId::proportional(11.0)),
            (TextStyle::Monospace, FontId::monospace(11.0)),
        ]
        .into();

        style.visuals.window_fill = PANEL;
        style.visuals.panel_fill = BG;
        style.visuals.faint_bg_color = PANEL_ALT;
        style.visuals.extreme_bg_color = BG;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.widgets.noninteractive.bg_fill = PANEL;
        style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        style.visuals.widgets.inactive.bg_fill = PANEL_ALT;
        style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(28, 33, 43);
        style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(31, 36, 46);
        style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT);
        style.visuals.widgets.hovered.bg_stroke =
            Stroke::new(1.0, ACCENT.linear_multiply(0.40));
        style.visuals.widgets.active.bg_stroke =
            Stroke::new(1.0, ACCENT.linear_multiply(0.65));
        style.visuals.selection.bg_fill = ACCENT.linear_multiply(0.30);
        style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    });
}

pub fn run() -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Moth · PokéIdle")
            .with_app_id("moth-controller")
            .with_icon(egui::IconData {
                rgba: icon::rgba(icon::DEFAULT_ICON_SIZE),
                width: icon::DEFAULT_ICON_SIZE,
                height: icon::DEFAULT_ICON_SIZE,
            })
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([800.0, 560.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Moth · PokéIdle",
        options,
        Box::new(|cc| Ok(Box::new(ControllerApp::new(cc)))),
    )
    .map_err(|error| format!("could not start controller UI: {error}"))
}

fn format_pct(value: f32) -> String {
    if value.fract().abs() < 0.01 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}

fn format_number(value: u64) -> String {
    let mut value = value.to_string();
    let mut i = value.len() as isize - 3;
    while i > 0 {
        value.insert(i as usize, ',');
        i -= 3;
    }
    value
}
