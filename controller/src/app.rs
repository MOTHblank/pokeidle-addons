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
mod hunt_panel;
mod market_panel;
mod kick_panel;
mod translations;
mod localization;
mod workspace_nav;
mod overview_panel;
mod automation_panel;
mod runtime_panel;
mod accounts_panel;
mod status_widgets;
use localization::*;
use workspace_nav::*;
use overview_panel::*;
use automation_panel::*;
use runtime_panel::*;
use accounts_panel::*;
use status_widgets::*;
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

struct CaptureSavedEvent {
    index:usize,
    config:CatchSettings,
    // A switch-only save must not discard unsaved editor changes.
    toggle_only:bool,
    result:Result<(),String>
}
pub struct ControllerApp {
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
    panel_tab: PanelTab,
    accounts_section: AccountsSection,
    kick_selected_account: usize,
    dashboard_show_history: bool,
    dashboard_session_view: bool,
    ui_options: UiOptions,
    hunt_account: usize,
    automation_account: usize,
    automation_section: AutomationSection,
    automation_drafts: [AutomationDraft;4],
    market_drafts: [Option<crate::native_market::MarketRules>;4],
    market_snatch_drafts: [crate::native_market::SnatchTarget;4],
    market_imports: [String;4],
    market_notices: [String;4],
    market_copy_targets: [bool;4],
    market_copy_activation: bool,
    history_rows_limit: usize,
    offline_histories: [Option<crate::native_history::HistorySnapshot>;4],
    hunt_query: String,
    hunt_region: String,
    hunt_type: String,
    hunt_ranking: HuntRanking,
    hunt_subtab: HuntSubtab,
    hunt_page: usize,
    hunt_action_error: [Option<String>;4],
    hunt_estimate_cache: [hunt_panel::EstimateCache;4],
    atlas_results: [hunt_panel::ExplorerCache;4],
    atlas_filters: [hunt_panel::AtlasFilters;4],
    atlas_scenarios: [hunt_panel::AtlasScenario;4],
    atlas_page: [usize;4],
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
    restock_open:[bool;4],
    capture_save_busy:[bool;4],
    capture_save_error:[Option<String>;4],
    capture_save_tx:Sender<CaptureSavedEvent>,
    capture_save_rx:Receiver<CaptureSavedEvent>,
    session_reset_confirm:[bool;4],
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
    account_names_dirty: [bool; 4],
    status: String,
    status_error: bool,
    events: VecDeque<UiEvent>,
    account_setup: [AccountSetup; 4],
    account_setup_checked_at: Option<Instant>,
    tab_url_input: [String; 4],
    tab_url_open: [bool; 4],
    idle_progress: [IdleProgressTracker; 4],
    xp_rates: [XpRateTracker; 4],
    selected_details: Option<usize>,
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
            panel_tab: PanelTab::Overview,
            accounts_section: AccountsSection::Profiles,
            kick_selected_account: 0,
            dashboard_show_history: false,
            dashboard_session_view: false,
            ui_options: Config::load_ui_options().unwrap_or_else(|error| {
                logging::warn(&format!("controller options load failed: {error}"));
                UiOptions::default()
            }),
            hunt_account: 0,
            automation_account: 0,
            automation_section: AutomationSection::Capture,
            automation_drafts: std::array::from_fn(|_|AutomationDraft::default()),
            market_drafts: std::array::from_fn(|_|None),
            market_snatch_drafts: std::array::from_fn(|_|crate::native_market::SnatchTarget{
                kind:"item".into(),currency:"gold".into(),..Default::default()
            }),
            market_imports: std::array::from_fn(|_|String::new()),
            market_notices: std::array::from_fn(|_|String::new()),
            market_copy_targets: [false;4],
            market_copy_activation: false,
            history_rows_limit: 100,
            offline_histories: std::array::from_fn(|_|None),
            hunt_query: String::new(),
            hunt_region: String::new(),
            hunt_type: String::new(),
            hunt_ranking: HuntRanking::TrainerXp,
            hunt_subtab: HuntSubtab::Explorer,
            hunt_page: 0,
            hunt_action_error: std::array::from_fn(|_|None),
            hunt_estimate_cache: std::array::from_fn(|_|hunt_panel::EstimateCache::default()),
            atlas_results: std::array::from_fn(|_|hunt_panel::ExplorerCache::default()),
            atlas_filters: std::array::from_fn(|_|hunt_panel::AtlasFilters::default()),
            atlas_scenarios: std::array::from_fn(|_|hunt_panel::AtlasScenario::default()),
            atlas_page: [0;4],
            hunt_reference: None,
            hunt_reference_error: None,
            hunt_reference_busy: false,
            hunt_reference_tx,
            hunt_reference_rx,
            capture_drafts:std::array::from_fn(|_|None),
            restock_drafts:std::array::from_fn(|_|None),
            restock_dirty:[false;4],
            restock_open:[false;4],
            capture_save_busy:[false;4],
            capture_save_error:std::array::from_fn(|_|None),
            capture_save_tx,
            capture_save_rx,
            session_reset_confirm:[false;4],
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
            account_names_dirty: [false; 4],
            status: "Ready · launch only the profiles you need".to_string(),
            status_error: false,
            events: VecDeque::new(),
            account_setup: std::array::from_fn(|_| AccountSetup::default()),
            account_setup_checked_at: None,
            tab_url_input: std::array::from_fn(|_| String::new()),
            tab_url_open: [false; 4],
            idle_progress: std::array::from_fn(|_| IdleProgressTracker::default()),
            xp_rates: std::array::from_fn(|_| XpRateTracker::default()),
            selected_details: None,
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

    fn game_index(profile: GameProfile) -> usize {
        profile.index()
    }

    fn hidden_in_tray(&self) -> bool {
        #[cfg(windows)]
        { self.tray_hidden.load(Ordering::Relaxed) }
        #[cfg(not(windows))]
        { false }
    }

    fn controller_in_background(&self, ctx: &egui::Context) -> bool {
        // Keep native background transport untouched; only UI repaint and
        // Firefox/BiDi monitoring pace respond to these preferences.
        let focused = ctx.input(|input| input.viewport().focused);
        reduce_updates_for_window(self.hidden_in_tray(), focused, self.ui_options)
    }

    fn save_ui_options(&mut self, updated: UiOptions) {
        match Config::save_ui_options(&updated) {
            Ok(()) => {
                self.ui_options = updated;
                self.set_status(tr("Controller options saved").to_owned(), false);
            }
            Err(error) => self.set_status(error, true),
        }
    }

    fn sync_monitor_paces(&self, background: bool) {
        // Exactly one account may be expanded when focused. Background mode
        // overrides that selection without clearing the open details panel.
        for (index, slot) in self.games.iter().enumerate() {
            let pace = monitor_pace_for_slot(
                self.selected_details, index, self.accounts[index].enabled, background
            );
            if let Some(monitor) = slot.monitor.as_ref() {
                monitor.set_pace(pace);
            }
        }
    }

    /// Switching accounts changes the workspace selection, not a live session.
    fn select_workspace_account(&mut self, index: usize) {
        if index >= self.games.len() || index == self.hunt_account { return; }
        self.save_dirty_account_names();
        self.history_rows_limit = 100;
        self.dashboard_show_history = false;
        self.hunt_account = index;
        if self.panel_tab == PanelTab::Hunts && self.hunt_subtab == HuntSubtab::Compare {
            self.hunt_subtab = HuntSubtab::Explorer;
        }
        self.automation_account = index;
        self.hunt_page = 0;
    }

    /// Account edits are buffered until focus is lost, the selected account
    /// changes, or the user navigates away from Accounts.
    fn save_dirty_account_names(&mut self) {
        for profile in GameProfile::ALL {
            let index = profile.index();
            if self.account_names_dirty[index] {
                let name = self.accounts[index].name.trim();
                self.accounts[index].name = if name.is_empty() {
                    profile.label().to_owned()
                } else {
                    name.to_owned()
                };
                if self.save_account_config() {
                    self.account_names_dirty[index] = false;
                }
            }
        }
    }

    fn account_name(&self, profile: GameProfile) -> String {
        self.accounts[profile.index()].name.clone()
    }

    fn account_enabled(&self, profile: GameProfile) -> bool {
        self.accounts[profile.index()].enabled
    }

    fn save_account_config(&mut self) -> bool {
        match Config::save_accounts(&self.accounts) {
            Ok(()) => true,
            Err(error) => {
                self.set_status(error, true);
                false
            }
        }
    }

    fn refresh_account_setup(&mut self) {
        for profile in GameProfile::ALL {
            let index = profile.index();
            let config = match Config::for_profile(profile) {
                Ok(config) => config,
                Err(error) => {
                    self.account_setup[index] = AccountSetup {
                        check_error: Some(error),
                        ..AccountSetup::default()
                    };
                    continue;
                }
            };

            // The setup marker is earned only after both PokéIdle and Twitch
            // report authenticated connections.
            let mut setup_complete = accounts::setup_complete(profile).unwrap_or(false);
            let verified = self.games[index].native.as_ref().is_some_and(|session| {
                session.snapshot().phase == "Online"
                    && session.twitch_snapshot().phase == "Online"
            });
            if verified && !setup_complete {
                match accounts::complete_setup(profile) {
                    Ok(()) => setup_complete = true,
                    Err(error) => logging::warn(&error),
                }
            }
            self.account_setup[index] = AccountSetup {
                profile_ready: config.profile_dir.exists(),
                setup_complete,
                check_error: None,
            };
        }

        self.account_setup_checked_at = Some(Instant::now());
    }

    fn refresh_processes(&mut self) {
        while let Ok(event)=self.capture_save_rx.try_recv() {
            self.capture_save_busy[event.index]=false;
            match event.result {
                Ok(()) => {
                    if let Some(native)=self.games[event.index].native.as_ref() {
                        if let Err(error)=native.set_capture_settings(event.config.clone()) {
                            self.capture_save_error[event.index]=Some(error);
                            continue;
                        }
                    }
                    if event.toggle_only {
                        if let Some(draft)=self.capture_drafts[event.index].as_mut() {
                            draft.enabled=event.config.enabled;
                        }
                    } else {
                        self.capture_drafts[event.index]=None;
                    }
                    if self.restock_drafts[event.index].as_ref()
                        .is_some_and(|draft| restock_fields_equal(draft, &event.config))
                    {
                        // The save has caught up with what the user sees.
                        // A different staged selection stays queued instead.
                        self.restock_drafts[event.index]=Some(event.config);
                        self.restock_dirty[event.index]=false;
                    }
                    self.capture_save_error[event.index]=None;
                    self.set_status(tr("Auto Catch settings saved"),false);
                }
                Err(error) => {
                    self.capture_save_error[event.index]=Some(error.clone());
                    self.set_status(format!("Could not save capture settings: {error}"),true);
                }
            }
        }
        while let Ok(result)=self.inventory_catalog_rx.try_recv() {
            self.inventory_catalog_busy=false;
            match result {
                Ok(catalog)=>{
                    logging::info(&format!("Native inventory catalog ready: {} items",catalog.items.len()));
                    self.inventory_catalog=Some(Arc::new(catalog));
                    self.inventory_catalog_error=None;
                }
                Err(error)=>{
                    logging::warn(&format!("Native inventory catalog unavailable: {error}"));
                    self.inventory_catalog_error=Some(error);
                }
            }
        }
        while let Ok(loaded) = self.hunt_reference_rx.try_recv() {
            self.hunt_reference_busy = false;
            match loaded {
                Ok(reference) => {
                    logging::info(&format!(
                        "Native Hunt Atlas reference ready: {} species",
                        reference.species.len()
                    ));
                    self.hunt_reference = Some(Arc::new(reference));
                    self.hunt_reference_error = None;
                }
                Err(error) => {
                    logging::warn(&format!("Native Hunt Atlas reference unavailable: {error}"));
                    self.hunt_reference_error = Some(error);
                }
            }
        }
        // Only the game TAB is closed before proof. The authentication browser
        // stays alive until both native sockets have received real welcome.
        while let Ok(event) = self.native_verified_rx.try_recv() {
            let index = event.index;
            if !self.native_verifying[index] { continue; }
            self.native_verifying[index] = false;
            let profile = GameProfile::from_index(index).expect("known slot");
            let native = match event.result {
                Ok(native) => native,
                Err(error) => {
                    self.native_error[index] = Some(error.clone());
                    self.set_status(format!(
                        "{}: native verification failed: {error}; Firefox game restored or recovery reported",
                        self.account_name(profile)), true);
                    continue;
                }
            };
            if self.games[index].handoff_closing || self.games[index].native.is_some() {
                native.stop();
                continue;
            }
            let mut child = self.games[index].child.take();
            let orphaned_pid = self.games[index].orphaned_pid.take();
            if child.is_none() && orphaned_pid.is_none() {
                native.stop();
                self.set_status("Could not find the Firefox window used to sign in", true);
                continue;
            }
            let monitor = self.games[index].monitor.take();
            self.games[index].handoff_closing = true;
            self.native_close_cancelled[index] = false;
            let tx = self.native_closed_tx.clone();
            logging::info(&format!(
                "{} native game welcome + Twitch IRC 001 confirmed; closing Firefox",
                self.account_name(profile)));
            std::thread::spawn(move || {
                let mut result = if let Some(child) = child.as_mut() {
                    firefox::stop_child(child, monitor.as_ref())
                } else {
                    Ok(())
                };
                if result.is_ok() {
                    result = crate::config::Config::for_profile(profile)
                        .and_then(|config| firefox::stop_profile_processes(&config));
                }
                if result.is_ok() {
                    if let Some(monitor) = monitor.as_ref() { monitor.stop(); }
                }
                let _ = tx.send(NativeBrowserClosedEvent {
                    index, native, cache_warning: event.cache_warning,
                    child, orphaned_pid, monitor, result
                });
            });
        }
        // Browser shutdown is asynchronous: the UI never blocks for BiDi
        // browser.close, process reaping or a force-kill fallback.
        while let Ok(event) = self.native_closed_rx.try_recv() {
            let index = event.index;
            let profile = GameProfile::from_index(index).expect("known slot");
            self.games[index].handoff_closing = false;
            let cancelled = std::mem::replace(&mut self.native_close_cancelled[index], false);
            match event.result {
                Ok(()) => {
                    self.native_error[index] = None;
                    // Successful Firefox close must be logged before any
                    // attempt to start native sockets, so a crash is localizable.
                    logging::info(&format!(
                        "{} Firefox exited after authenticated handoff",
                        self.account_name(profile)));
                    if cancelled {
                        self.set_status(format!(
                            "{}: sign-in canceled", self.account_name(profile)), false);
                        continue;
                    }
                    logging::info(&format!(
                        "{}: connecting to PokéIdle and Twitch", self.account_name(profile)));
                    self.games[index].native = Some(event.native);
                    self.capture_drafts[index]=None;
                    self.restock_drafts[index]=None;
                    self.restock_dirty[index]=false;
                    self.automation_drafts[index]=AutomationDraft::default();
                    self.require_browser_login[index] = false;
                    let cache_note = event.cache_warning
                        .map(|warning| format!(" · You may need to sign in next time: {warning}"))
                        .unwrap_or_default();
                    self.set_status(format!(
                        "{}: PokéIdle and Twitch connected. Firefox closed{cache_note}",
                        self.account_name(profile)), false);
                }
                Err(error) => {
                    // Native game is already authenticated; Firefox's game
                    // tab was closed. Retain native ownership and the remaining
                    // Firefox handle for a later explicit cleanup attempt.
                    self.games[index].native = Some(event.native);
                    self.capture_drafts[index]=None;
                    self.restock_drafts[index]=None;
                    self.restock_dirty[index]=false;
                    self.automation_drafts[index]=AutomationDraft::default();
                    self.games[index].child = event.child;
                    self.games[index].orphaned_pid = event.orphaned_pid;
                    self.games[index].monitor = event.monitor;
                    self.set_status(format!(
                        "{}: connected, but Firefox could not close automatically: {error}",
                        self.account_name(profile)), true);
                }
            }
        }
        // Try the protected native credential vault BEFORE constructing any
        // Firefox process. OS keyring requests run off the UI thread.
        while let Ok(event) = self.cached_native_rx.try_recv() {
            if !self.cached_native_busy[event.index] { continue; }
            self.cached_native_busy[event.index] = false;
            let profile = GameProfile::from_index(event.index).expect("known slot");
            if self.games[event.index].is_running() { continue; }
            if event.existing_firefox {
                // A stale managed Firefox process still owns this profile.
                // Do not create a competing native game socket from cache.
                // Adopt its BiDi session and let the two-phase handoff close it.
                self.require_browser_login[event.index] = true;
                self.launch_firefox_bootstrap(profile, false);
                continue;
            }
            if self.require_browser_login[event.index] {
                self.launch_firefox_bootstrap(profile, false);
                continue;
            }
            match event.result {
                Ok(Some(session)) => {
                    self.games[event.index].native = Some(session);
                    self.capture_drafts[event.index]=None;
                    self.automation_drafts[event.index]=AutomationDraft::default();
                    self.set_status(format!(
                        "{}: PokéIdle and Twitch connected with your saved login",
                        self.account_name(profile)), false);
                },
                Ok(None) => {
                    self.require_browser_login[event.index] = true;
                    self.set_status(format!(
                        "{}: please sign in to continue",
                        self.account_name(profile)), false);
                    self.launch_firefox_bootstrap(profile, false);
                }
                Err(error) => {
                    self.require_browser_login[event.index] = true;
                    self.set_status(format!(
                        "{}: saved login unavailable: {error}",
                        self.account_name(profile)), true);
                    self.launch_firefox_bootstrap(profile, false);
                }
            }
        }
        // Finish BiDi capture on the UI thread. The authenticated bootstrap
        // is never logged or written to a file.
        while let Ok(event) = self.native_handoff_rx.try_recv() {
            if !self.native_handoff_busy[event.index] { continue; }
            self.native_handoff_busy[event.index] = false;
            let profile = GameProfile::from_index(event.index).expect("known slot");
            match event.result {
                Err(error) => {
                    let pending = error.to_ascii_lowercase();
                    let is_bootstrap = pending.contains("reloaded")
                        || pending.contains("waiting") || pending.contains("pending")
                        || pending.contains("temporary twitch")
                        || pending.contains("hook installed")
                        || pending.contains("login has not produced");
                    if is_bootstrap {
                        self.native_error[event.index] = None;
                    } else {
                        self.native_error[event.index] = Some(error.clone());
                    }
                    self.set_status(format!(
                        "{}: {}: {error}",
                        self.account_name(profile),
                        if is_bootstrap { "waiting for login" } else { "login check" }
                    ), !is_bootstrap);
                },
                Ok(bootstrap) => {
                    // Never close Firefox without a usable authenticated hello
                    // and cookies, or if the slot changed while capture ran.
                    if self.games[event.index].native.is_some()
                        || !self.games[event.index].is_running()
                        || self.games[event.index].monitor.is_none()
                    {
                        self.set_status("Connection interrupted because the Firefox session changed", true);
                        continue;
                    }
                    let validated = crate::native_background::validate_bootstrap(&bootstrap);
                    if let Err(error) = validated {
                        self.native_error[event.index] = Some(error.clone());
                        self.set_status(format!("Could not connect to the game: {error}"), true);
                        continue;
                    }
                    // First prove Rust is authenticated while Firefox's live game
                    // WebSocket still exists, matching pokeidle-manager-moth.
                    // Only then close the game tab and finally Firefox.
                    if self.games[event.index].handoff_closing ||
                        self.native_verifying[event.index] { continue; }
                    let Some(monitor) = self.games[event.index].monitor.clone() else {
                        continue;
                    };
                    self.native_verifying[event.index] = true;
                    self.native_error[event.index] = None;
                    let tx = self.native_verified_tx.clone();
                    let index = event.index;
                    let cache_warning = event.cache_warning;
                    logging::info(&format!(
                        "{}: checking PokéIdle and Twitch connections before closing Firefox",
                        self.account_name(profile)));
                    let spawned = std::thread::Builder::new()
                        .name(format!("moth-native-handoff-{}", profile.name()))
                        .spawn(move || {
                            let mut attempted_game_close = false;
                            // A successful native welcome must precede the game
                            // tab close. Closing Firefox's original socket first
                            // could invalidate the just-captured hello/token.
                            let result = std::panic::catch_unwind(
                                std::panic::AssertUnwindSafe(|| {
                                    crate::native_tls::ensure()?;
                                    let native = NativeSession::start_verified(profile, bootstrap)?;
                                    attempted_game_close = true;
                                    monitor.close_game_for_handoff(Duration::from_secs(18))?;
                                    Ok(native)
                                })
                            ).unwrap_or_else(|_| {
                                Err("Login check failed unexpectedly. See the controller log.".to_owned())
                            });
                            let result = match (result, attempted_game_close) {
                                (Err(error), true) => {
                                    match monitor.restore_game_after_failed_handoff(
                                        Duration::from_secs(20)
                                    ) {
                                        Ok(()) => Err(format!("{error}; game tab reopened")),
                                        Err(recovery) => {
                                            crate::logging::error(&format!(
                                                "Could not recover the connection: {recovery}"
                                            ));
                                            Err(format!("{error}; game tab recovery failed: {recovery}"))
                                        }
                                    }
                                }
                                (other, _) => other,
                            };
                            let _ = tx.send(NativeVerifiedEvent {
                                index, result, cache_warning
                            });
                        });
                    if let Err(error) = spawned {
                        self.native_verifying[index] = false;
                        self.native_error[index] = Some(format!(
                            "Could not start the login check: {error}"
                        ));
                        self.set_status(format!(
                            "{}: could not verify login: {error}",
                            self.account_name(profile)), true);
                    }
                }
            }
        }
        let mut recover = Vec::new();
        for slot in &mut self.games {
            if slot.native.as_ref().is_some_and(|session|
                session.snapshot().phase == "NeedsLogin" ||
                session.twitch_snapshot().phase == "NeedsLogin"
            ) {
                if let Some(session) = slot.native.take() { session.stop(); }
                recover.push(slot.profile);
            }
        }
        for profile in recover {
            self.require_browser_login[profile.index()] = true;
            self.set_status(format!(
                "{}: login expired, opening Firefox to sign in again",
                self.account_name(profile)), true);
            self.launch_firefox_bootstrap(profile, false);
        }
        let mut command_feedback = Vec::new();
        for slot in &mut self.games {
            if let Some(monitor) = slot.monitor.as_ref() {
                while let Some(result) = monitor.take_command_feedback() {
                    command_feedback.push((
                        self.accounts[slot.profile.index()].name.clone(),
                        result,
                    ));
                }
            }
            let exited = match slot.child.as_mut() {
                Some(child) => match child.try_wait() {
                    Ok(Some(_)) => true,
                    Ok(None) => false,
                    Err(error) => {
                        logging::warn(&format!(
                            "{} Firefox process check failed: {error}", slot.profile.label()
                        ));
                        false
                    }
                },
                None => false,
            };
            if exited {
                let account_name = self.accounts[slot.profile.index()].name.clone();
                logging::info(&format!("{} Firefox child process exited", account_name));
                slot.child = None;

                // Firefox can outlive the original launcher handle. Keep the
                // existing BiDi monitor for that same profile rather than
                // permanently discarding telemetry from a still-open game.
                match Config::for_profile(slot.profile)
                    .and_then(|config| firefox::find_profile_process(&config))
                {
                    Ok(Some(pid)) => {
                        slot.orphaned_pid = Some(pid);
                        logging::warn(&format!(
                            "{} Firefox launcher exited; profile PID {} is still running, preserving monitor",
                            account_name, pid
                        ));
                    }
                    Ok(None) => {
                        slot.orphaned_pid = None;
                        if let Some(monitor) = slot.monitor.take() {
                            monitor.stop();
                        }
                    }
                    Err(error) => logging::warn(&format!(
                        "{} profile recovery failed (keeping monitor until ownership is known): {}",
                        account_name, error
                    )),
                }
            }
            if let Some(pid) = slot.orphaned_pid {
                if !firefox::process_alive(pid) {
                    slot.orphaned_pid = None;
                    if let Some(monitor) = slot.monitor.take() {
                        monitor.stop();
                    }
                }
            }
        }
        for (account_name, result) in command_feedback {
            self.set_status(
                format!("{} · {} #{}: {}", account_name, tr("Tab"), result.id, result.message),
                !result.ok,
            );
        }
        // Temporary Firefox authentication is auto-handed off to native.
        // An explicitly opened interactive Firefox must stay open instead.
        for profile in GameProfile::ALL {
            let index = profile.index();
            if !self.accounts[index].enabled || self.games[index].browser_only ||
               self.native_handoff_busy[index] || self.games[index].native.is_some() ||
               self.native_verifying[index] ||
               self.games[index].handoff_closing || self.games[index].monitor.is_none() {
                continue;
            }
            let health = self.games[index].health();
            // BiDi's captured game hello, not a DOM telemetry sample, is the
            // authentication signal. The reference manager also observes the
            // hello while the login page is still transitioning.
            if health.state != "Running" {
                continue;
            }
            if self.native_handoff_last_attempt[index]
                .is_some_and(|at| at.elapsed() < Duration::from_secs(8)) { continue; }
            self.start_native_handoff(profile);
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

    fn start_native_handoff(&mut self, profile: GameProfile) {
        let index = profile.index();
        if self.native_handoff_busy[index] { return; }
        let Some(monitor) = self.games[index].monitor.clone() else {
            self.set_status("Open Firefox and sign in to PokéIdle first", true);
            return;
        };
        self.native_handoff_busy[index] = true;
        self.native_handoff_last_attempt[index] = Some(Instant::now());
        logging::info(&format!(
            "{} capturing native game + Twitch IRC session via Firefox BiDi",
            self.account_name(profile)));
        self.set_status(format!("{}: signing in to PokéIdle and Twitch",
            self.account_name(profile)), false);
        let tx = self.native_handoff_tx.clone();
        let result = std::thread::Builder::new()
            .name(format!("moth-handoff-{}", profile.name()))
            .spawn(move || {
                let mut result = monitor.native_bootstrap(Duration::from_secs(18));
                // Firefox provides fresh authentication, not authoritative
                // native preferences. Preserve this account's saved Auto Catch
                // configuration, even after the browser login is renewed.
                // Cache and worker must receive the SAME merged bootstrap.
                let mut cache_warning = None;
                if let Ok(value) = &mut result {
                    if crate::native_background::validate_bootstrap(value).is_ok() {
                        let (merged,warning) =
                            crate::native_auth::cache_browser_handoff(profile,value);
                        *value=merged;
                        cache_warning=warning;
                    }
                }
                let _ = tx.send(NativeHandoffEvent { index, result, cache_warning });
            });
        if let Err(error) = result {
            self.native_handoff_busy[index] = false;
            self.set_status(format!("Could not finish sign-in: {error}"), true);
        }
    }

    fn launch_one(&mut self, profile: GameProfile) {
        self.launch_one_mode(profile, true);
    }

    fn launch_one_mode(&mut self, profile: GameProfile, headless: bool) {
        let index = Self::game_index(profile);
        if self.games[index].is_running() || self.cached_native_busy[index] ||
            self.native_handoff_busy[index] {
            return;
        }
        self.games[index].headless = headless;
        self.games[index].browser_only = false;
        if self.require_browser_login[index] {
            self.launch_firefox_bootstrap(profile, false);
            return;
        }
        self.cached_native_busy[index] = true;
        let tx = self.cached_native_tx.clone();
        let started = std::thread::Builder::new()
            .name(format!("moth-cache-{}", profile.name()))
            .spawn(move || {
                // A managed Firefox profile from a previous crash may still
                // have an active game socket. Determine ownership BEFORE using
                // saved credentials, or the same account may connect twice.
                let detected = Config::for_profile(profile)
                    .and_then(|config| firefox::find_profile_process(&config));
                let existing_firefox = matches!(detected.as_ref(), Ok(Some(_)));
                // A cache preflight panic must never strand the UI in
                // "starting" state. Report it, then authenticate via Firefox.
                let result = std::panic::catch_unwind(
                    std::panic::AssertUnwindSafe(|| {
                        crate::native_tls::ensure()?;
                        detected.and_then(|_| {
                            if existing_firefox { return Ok(None); }
                            crate::native_auth::load(profile)
                                .and_then(|saved| saved.map(|bootstrap| {
                                    // Never project a cached, unverified connection as
                                    // started. A failed welcome needs a fresh Firefox
                                    // authentication, not infinite background retries.
                                    NativeSession::start_verified(profile, bootstrap)
                                }).transpose())
                        })
                    })
                ).unwrap_or_else(|_| {
                    Err("Native cache verification panicked; see sanitized controller log".into())
                });
                let _ = tx.send(CachedNativeEvent {
                    index, existing_firefox, result
                });
            });
        if let Err(error) = started {
            self.cached_native_busy[index] = false;
            self.require_browser_login[index] = true;
            self.set_status(format!("Saved login could not be used: {error}"), true);
            self.launch_firefox_bootstrap(profile, false);
        }
    }

    /// Authentication-only browser fallback. Never call as the ordinary
    /// Launch path while reusable native game + Twitch credentials exist.
    fn launch_firefox_bootstrap(&mut self, profile: GameProfile, headless: bool) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);
        if self.games[index].is_running() { return; }

        let config = match crate::config::Config::for_profile(profile) {
            Ok(config) => config,
            Err(error) => {
                self.set_status(format!("{}: {}", self.account_name(profile), error), true);
                return;
            }
        };

        // An existing Firefox may own this profile even if the controller was
        // restarted or its Child handle was lost. Never spawn over that session.
        match firefox::find_profile_process(&config) {
            Ok(Some(pid)) => {
                // A previous crash may have left an authenticated, controlled
                // Moth Firefox profile alive. Attach to its existing BiDi
                // endpoint and complete native handoff; never spawn another
                // Firefox on the same profile or kill it before Rust is ready.
                self.games[index].orphaned_pid = Some(pid);
                if self.games[index].monitor.is_none() {
                    self.games[index].monitor = Some(MonitorHandle::start(
                        config.remote_debug_port, headless));
                }
                logging::info(&format!(
                    "{} adopting existing Moth Firefox profile PID {} for native handoff",
                    account_name, pid
                ));
                self.set_status(format!(
                    "{}: using your existing Firefox login to connect (process {})",
                    account_name, pid
                ), false);
                return;
            }
            Ok(None) => {}
            Err(error) => {
                self.set_status(format!("{}: {}", account_name, error), true);
                return;
            }
        }

        self.games[index].child = None;
        self.games[index].monitor = None;
        self.games[index].orphaned_pid = None;
        self.games[index].headless = headless;

        logging::info(&format!(
            "launching {} with Firefox profile {} on BiDi port {}",
            self.account_name(profile),
            config.profile_dir.display(),
            config.remote_debug_port
        ));

        match firefox::launch(&config, headless) {
            Ok((child, monitor)) => {
                let pid = child.id();
                self.games[index].child = Some(child);
                self.games[index].monitor = Some(monitor);
                self.set_status(
                    format!(
                        "{}: Firefox started in {} for sign-in",
                        self.account_name(profile),
                        if headless { "the background" } else { "a window" }
                    ),
                    false,
                );
                logging::info(&format!(
                    "{} Firefox spawned with PID {}",
                    self.account_name(profile),
                    pid
                ));
            }
            Err(error) => {
                logging::error(&format!("{} launch failed: {}", self.account_name(profile), error));
                self.set_status(format!("{}: {}", self.account_name(profile), error), true);
            }
        }
    }

    /// Explicit interactive browser mode, separate from the temporary native
    /// authentication bootstrap. Do not open two game sockets for one account.
    fn open_in_firefox(&mut self, profile: GameProfile) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);

        // A handoff worker might already be closing the Firefox game tab or
        // shutting down its process. Never race it with a new browser session.
        if self.native_verifying[index] || self.games[index].handoff_closing {
            self.set_status(format!(
                "{account_name}: Sign-in is still finishing. Open Firefox after it completes."
            ), true);
            return;
        }

        if self.games[index].browser_only && !self.games[index].headless &&
            self.games[index].is_running() {
            self.set_status(format!("{account_name}: Firefox is already open."), false);
            return;
        }

        // Stop native game/IRC or an authentication-only Firefox first. In
        // particular, don't run native and Firefox on the same game account.
        if !self.stop_one(profile) { return; }
        self.games[index].browser_only = true;
        self.launch_firefox_bootstrap(profile, false);
    }

    fn repair_monitor(&mut self, profile: GameProfile) {
        // An orphaned Firefox BiDi session cannot be reclaimed by a new
        // session.new. Only an explicit user action restarts this profile;
        // never interrupt a running game automatically on telemetry failure.
        let index = Self::game_index(profile);
        let headless = self.games[index].headless;
        let browser_only = self.games[index].browser_only;
        logging::warn(&format!(
            "{} repairing an exhausted BiDi session by user-requested Firefox restart",
            self.account_name(profile)
        ));
        if self.stop_one(profile) {
            if browser_only {
                // A manual Firefox launch must remain a manual Firefox launch
                // even after an explicit monitor repair/restart.
                self.games[index].browser_only = true;
                self.launch_firefox_bootstrap(profile, false);
            } else {
                self.launch_one_mode(profile, headless);
            }
        }
    }

    fn stop_one(&mut self, profile: GameProfile) -> bool {
        let index = Self::game_index(profile);
        // A stopped account must not keep an independent Kick worker alive.
        self.kick_monitors[index].take();
        let account_name = self.account_name(profile);
        if self.games[index].handoff_closing {
            // The dedicated shutdown worker already owns and is closing the
            // Firefox process. Do not start native mode after explicit Stop.
            self.native_close_cancelled[index] = true;
            self.set_status(format!(
                "{}: stopping sign-in", account_name), false);
            return true;
        }
        // Cancel in-flight cache lookups and browser handoffs. Results from
        // canceled workers are ignored even if a Stop is pressed mid-login.
        self.cached_native_busy[index] = false;
        self.native_handoff_busy[index] = false;
        self.native_verifying[index] = false;
        let slot = &mut self.games[index];
        if let Some(native) = slot.native.take() {
            native.stop();
            // A failed browser cleanup can leave an authenticated native game
            // plus an owned Firefox launcher. Stop must close BOTH owners.
            if slot.child.is_none() && slot.orphaned_pid.is_none() {
                if let Some(monitor) = slot.monitor.take() { monitor.stop(); }
                self.set_status(format!("{}: PokéIdle and Twitch stopped.", account_name), false);
                return true;
            }
        }

        // Stop also works after restarting the controller, before any attempt
        // to launch that profile has made the orphan process visible.
        if slot.child.is_none() && slot.orphaned_pid.is_none() {
            if let Ok(config) = Config::for_profile(profile) {
                match firefox::find_profile_process(&config) {
                    Ok(Some(pid)) => slot.orphaned_pid = Some(pid),
                    Ok(None) => {}
                    Err(error) => {
                        self.set_status(format!("{}: {}", account_name, error), true);
                        return false;
                    }
                }
            }
        }

        if let Some(mut child) = slot.child.take() {
            logging::info(&format!(
                "stopping {} Firefox PID {}", account_name, child.id()
            ));
            let closed = firefox::stop_child(&mut child, slot.monitor.as_ref())
                .and_then(|_| Config::for_profile(profile))
                .and_then(|config| firefox::stop_profile_processes(&config));
            match closed {
                Ok(()) => {
                    if let Some(monitor) = slot.monitor.take() {
                        monitor.stop();
                    }
                    slot.orphaned_pid = None;
                    self.set_status(format!("{} Firefox profile completely closed.", account_name), false);
                    true
                }
                Err(error) => {
                    logging::error(&format!("{} Firefox shutdown failed: {}", account_name, error));
                    slot.child = Some(child);
                    self.set_status(format!("{}: could not close Firefox: {}", account_name, error), true);
                    false
                }
            }
        } else if slot.orphaned_pid.is_some() {
            // Stop was explicitly clicked; never terminate a recovered process
            // automatically when the controller itself exits.
            let result = Config::for_profile(profile)
                .and_then(|config| firefox::stop_profile_processes(&config));
            match result {
                Ok(()) => {
                    slot.orphaned_pid = None;
                    if let Some(monitor) = slot.monitor.take() {
                        monitor.stop();
                    }
                    self.set_status(format!("{} Firefox closed.", account_name), false);
                    true
                }
                Err(error) => {
                    self.set_status(format!("{}: could not close Firefox: {}", account_name, error), true);
                    false
                }
            }
        } else {
            if let Some(monitor) = slot.monitor.take() {
                monitor.stop();
            }
            self.set_status(format!("{} is already stopped.", account_name), false);
            true
        }
    }

    fn launch_enabled(&mut self) {
        let enabled: Vec<GameProfile> = GameProfile::ALL.into_iter()
            .filter(|profile| self.account_enabled(*profile)).collect();
        let disabled: Vec<&str> = GameProfile::ALL.into_iter()
            .filter(|profile| !self.account_enabled(*profile))
            .map(GameProfile::label).collect();
        logging::info(&format!(
            "Launch enabled requested for {} accounts; disabled profiles skipped: {}",
            enabled.len(),
            if disabled.is_empty() { "none".to_string() } else { disabled.join(", ") }
        ));
        if enabled.is_empty() {
            self.set_status(
                "No accounts enabled. Enable a profile in Account Manager before launching.",
                true,
            );
            return;
        }
        for profile in enabled {
            self.launch_one(profile);
        }
        // Keep any launch error visible rather than overwriting it with a
        // success-shaped summary when a profile is already in use.
        if !self.status_error {
            self.set_status("Starting enabled accounts. Firefox will open only if sign-in is needed.", false);
        }
    }

    fn stop_all(&mut self) {
        let mut all_stopped = true;
        for profile in GameProfile::ALL {
            if !self.stop_one(profile) {
                all_stopped = false;
            }
        }
        if all_stopped {
            self.set_status("All accounts stopped. Firefox login windows were closed.", false);
        } else {
            self.set_status("Some Firefox instances could not be stopped; check the log.", true);
        }
    }

    fn open_logs(&mut self) {
        let path = match crate::config::data_root() {
            Ok(root) => root.join("moth-controller.log"),
            Err(error) => {
                self.set_status(error, true);
                return;
            }
        };
        if let Some(parent) = path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                self.set_status(format!("Could not create log folder: {error}"), true);
                return;
            }
        }
        if !path.exists() {
            if let Err(error) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                self.set_status(format!("Could not create log: {error}"), true);
                return;
            }
        }
        let opener = if cfg!(windows) { "notepad.exe" } else { "xdg-open" };
        match std::process::Command::new(opener).arg(&path).spawn() {
            Ok(_) => self.set_status(format!("Opened controller log · {}", path.display()), false),
            Err(error) => self.set_status(format!("Could not open log: {error}"), true),
        }
    }

    /// Use the same non-automated, visible Firefox for BOTH provider logins.
    /// Never send the Twitch login URL to a monitored BiDi profile.
    fn open_unmonitored_sign_in(&mut self, profile: GameProfile, action: ProfileAction) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);
        self.refresh_processes();

        // A native handoff still owns the profile until its cleanup finishes.
        // stop_one intentionally cancels it asynchronously, so do not launch
        // another Firefox against the same locked profile during that window.
        if self.native_verifying[index] || self.games[index].handoff_closing {
            self.set_status(format!(
                "{account_name}: Sign-in is still finishing. Try again after it completes."
            ), true);
            return;
        }

        // Stop the native game/Twitch sockets and any monitored Firefox so
        // authentication runs solely in an ordinary user-controlled browser.
        if !self.stop_one(profile) {
            return;
        }

        let config = match Config::for_profile(profile) {
            Ok(config) => config,
            Err(error) => {
                self.set_status(format!("{}: {}", account_name, error), true);
                return;
            }
        };

        // Preserve the existing per-account cookies, but do not share an
        // active Firefox process or debugger with either sign-in provider.
        match firefox::find_profile_process(&config) {
            Ok(Some(pid)) => {
                self.games[index].orphaned_pid = Some(pid);
                self.set_status(
                    format!("{}: Firefox already uses this profile (PID {}). Use Stop before launching.", account_name, pid),
                    true,
                );
                return;
            }
            Ok(None) => {}
            Err(error) => {
                self.set_status(format!("{}: {}", account_name, error), true);
                return;
            }
        }

        let (service, url, status_suffix) = match action {
            ProfileAction::GoogleSignIn => (
                "Google",
                config.url.as_str(),
                " opened Firefox without remote debugging for Google sign-in. After signing in, close that window and launch this account normally from the dashboard.",
            ),
            ProfileAction::Twitch => (
                "Twitch",
                accounts::TWITCH_LOGIN,
                " opened Firefox without remote debugging for Twitch sign-in. After signing in, close that window and launch this account normally from the dashboard.",
            ),
            ProfileAction::Folder => return,
        };

        match firefox::launch_unmonitored(&config, url) {
            Ok(child) => {
                let pid = child.id();
                self.games[index].child = Some(child);
                self.games[index].monitor = None;
                self.games[index].orphaned_pid = None;
                self.games[index].browser_only = true;
                self.games[index].headless = false;
                // After the user closes this window, refresh browser auth
                // before returning to native mode instead of trusting stale
                // cached Twitch or game login credentials.
                self.require_browser_login[index] = true;
                logging::info(&format!(
                    "{} Firefox spawned for {} sign-in without remote debugging (PID {})",
                    account_name, service, pid
                ));
                self.set_status(format!("{account_name}{status_suffix}"), false);
            }
            Err(error) => {
                logging::error(&format!(
                    "{} {} sign-in launch failed: {}",
                    account_name, service, error
                ));
                self.set_status(format!("{}: {}", account_name, error), true);
            }
        }
    }

    fn profile_action(&mut self, profile: GameProfile, action: ProfileAction) {
        let result = match action {
            ProfileAction::GoogleSignIn | ProfileAction::Twitch => {
                self.open_unmonitored_sign_in(profile, action);
                return;
            }
            ProfileAction::Folder => accounts::open_profile_folder(profile).map(|_| "Opened profile folder".to_string()),
        };

        match result {
            Ok(message) => self.set_status(format!("{} · {}", self.account_name(profile), message), false),
            Err(error) => self.set_status(error, true),
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
            && (matches!(self.panel_tab, PanelTab::Hunts | PanelTab::Automation)
                || self.panel_tab == PanelTab::Overview);
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
                        RichText::new(tr(match self.panel_tab {
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
                let active = match self.panel_tab {
                    PanelTab::Overview => Some(self.selected_details),
                    PanelTab::Hunts if self.hunt_subtab == HuntSubtab::Compare => None,
                    PanelTab::Hunts => Some(Some(self.hunt_account)),
                    PanelTab::Automation => Some(Some(self.automation_account)),
                    _ => None,
                };
                if let Some(selected) = active {
                    let next = draw_account_switcher(
                        self, ui, selected, self.panel_tab == PanelTab::Overview
                    );
                    if next != selected {
                        if let Some(index) = next {
                            self.select_workspace_account(index);
                            if self.panel_tab == PanelTab::Overview {
                                self.selected_details = Some(index);
                            }
                        } else if self.panel_tab == PanelTab::Overview {
                            self.selected_details = None;
                        }
                    }
                    ui.add_space(6.0);
                }
                match self.panel_tab {
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
        let active_explorer=if self.panel_tab==PanelTab::Hunts &&
            self.hunt_subtab==HuntSubtab::Explorer {
            Some(self.hunt_account)
        } else {None};
        for (index,results) in self.atlas_results.iter_mut().enumerate() {
            if active_explorer!=Some(index) {results.release();}
        }

        // If the user leaves Accounts while editing the name, persist it
        // without needing to close a dialog that no longer exists.
        if self.panel_tab != PanelTab::Accounts {
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
