use crate::accounts;
use crate::config::GameProfile;
use crate::firefox;
use crate::logging;
use crate::monitor::{Health, MonitorHandle};
use serde_json::json;
use eframe::egui::{self, Align, Color32, FontId, Layout, Margin, RichText, Stroke, TextStyle};
use std::process::Child;

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
    child: Option<Child>,
    monitor: Option<MonitorHandle>,
    headless: bool,
}

impl GameSlot {
    fn new(profile: GameProfile) -> Self {
        Self {
            profile,
            child: None,
            monitor: None,
            headless: true,
        }
    }

    fn is_running(&mut self) -> bool {
        match self.child.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(None)),
            None => false,
        }
    }

    fn health(&self) -> Health {
        self.monitor
            .as_ref()
            .map(MonitorHandle::health)
            .unwrap_or_default()
    }
}

pub struct ControllerApp {
    games: [GameSlot; 2],
    show_profiles: bool,
    show_atlas: bool,
    show_market: bool,
    atlas_profile: GameProfile,
    market_profile: GameProfile,
    market_search: String,
    atlas_search: String,
    market_currency: String,
    status: String,
    status_error: bool,
}

impl ControllerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);

        logging::init();
        logging::info("controller UI initialized with egui/eframe");

        Self {
            games: [
                GameSlot::new(GameProfile::Game1),
                GameSlot::new(GameProfile::Game2),
            ],
            show_profiles: false,
            show_atlas: false,
            show_market: false,
            atlas_profile: GameProfile::Game1,
            market_profile: GameProfile::Game1,
            market_search: String::new(),
            atlas_search: String::new(),
            market_currency: "gold".to_string(),
            status: "Ready · launch only the profiles you need".to_string(),
            status_error: false,
        }
    }

    fn game_index(profile: GameProfile) -> usize {
        match profile {
            GameProfile::Game1 => 0,
            GameProfile::Game2 => 1,
        }
    }

    fn refresh_processes(&mut self) {
        for slot in &mut self.games {
            let exited = match slot.child.as_mut() {
                Some(child) => !matches!(child.try_wait(), Ok(None)),
                None => false,
            };

            if exited {
                logging::info(&format!("{} Firefox process exited", slot.profile.label()));
                if let Some(monitor) = slot.monitor.as_ref() {
                    monitor.stop();
                }
                slot.child = None;
                slot.monitor = None;
            }
        }
    }

    fn set_status(&mut self, message: impl Into<String>, error: bool) {
        self.status = message.into();
        self.status_error = error;
    }

    fn launch_one(&mut self, profile: GameProfile) {
        self.launch_one_mode(profile, true);
    }

    fn launch_one_mode(&mut self, profile: GameProfile, headless: bool) {
        let index = Self::game_index(profile);
        self.refresh_processes();

        if self.games[index].is_running() {
            self.set_status(format!("{} is already running.", profile.label()), false);
            return;
        }

        self.games[index].child = None;
        self.games[index].monitor = None;
        self.games[index].headless = headless;

        let config = match crate::config::Config::for_profile(profile) {
            Ok(config) => config,
            Err(error) => {
                self.set_status(format!("{}: {}", profile.label(), error), true);
                return;
            }
        };

        logging::info(&format!(
            "launching {} with Firefox profile {} on BiDi port {}",
            profile.label(),
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
                        "{} started · {} Firefox · Rust BiDi health monitor",
                        profile.label(),
                        if headless { "headless" } else { "visible" }
                    ),
                    false,
                );
                logging::info(&format!(
                    "{} Firefox spawned with PID {}",
                    profile.label(),
                    pid
                ));
            }
            Err(error) => {
                logging::error(&format!("{} launch failed: {}", profile.label(), error));
                self.set_status(format!("{}: {}", profile.label(), error), true);
            }
        }
    }

    fn set_browser_mode(&mut self, profile: GameProfile, headless: bool) {
        let index = Self::game_index(profile);
        self.refresh_processes();

        if !self.games[index].is_running() {
            self.games[index].headless = headless;
            self.launch_one_mode(profile, headless);
            return;
        }

        if self.games[index].headless == headless {
            self.set_status(
                format!(
                    "{} is already running in {} mode.",
                    profile.label(),
                    if headless { "headless" } else { "visible" }
                ),
                false,
            );
            return;
        }

        self.set_status(
            format!(
                "{}: switching Firefox to {} mode...",
                profile.label(),
                if headless { "headless" } else { "visible" }
            ),
            false,
        );
        if self.stop_one(profile) {
            self.launch_one_mode(profile, headless);
        }
    }

    fn stop_one(&mut self, profile: GameProfile) -> bool {
        let index = Self::game_index(profile);
        let slot = &mut self.games[index];

        if let Some(monitor) = slot.monitor.as_ref() {
            monitor.stop();
        }

        let Some(mut child) = slot.child.take() else {
            slot.monitor = None;
            self.set_status(format!("{} is already stopped.", profile.label()), false);
            return true;
        };

        logging::info(&format!(
            "stopping {} Firefox PID {}",
            profile.label(),
            child.id()
        ));

        match child.kill() {
            Ok(()) => {
                let _ = child.wait();
                slot.monitor = None;
                self.set_status(format!("{} Firefox closed.", profile.label()), false);
                true
            }
            Err(error) => {
                logging::error(&format!(
                    "failed to stop {} Firefox PID {}: {}",
                    profile.label(),
                    child.id(),
                    error
                ));
                slot.child = Some(child);
                self.set_status(
                    format!("{}: could not close Firefox: {}", profile.label(), error),
                    true,
                );
                false
            }
        }
    }

    fn launch_both(&mut self) {
        self.launch_one(GameProfile::Game1);
        self.launch_one(GameProfile::Game2);
        self.set_status("Launch Both requested · monitoring will update as Firefox becomes ready", false);
    }

    fn stop_all(&mut self) {
        self.stop_one(GameProfile::Game1);
        self.stop_one(GameProfile::Game2);
        self.set_status("All Firefox instances closed.", false);
    }

    fn open_logs(&mut self) {
        let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
            self.set_status("Could not determine LOCALAPPDATA for logs.", true);
            return;
        };

        let path = std::path::PathBuf::from(local_app_data)
            .join("Moth")
            .join("PokeIdle")
            .join("moth-controller.log");

        if let Some(parent) = path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                self.set_status(format!("Could not create log folder: {error}"), true);
                return;
            }
        }

        if !path.exists() {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path);
        }

        match std::process::Command::new("notepad.exe")
            .arg(&path)
            .spawn()
        {
            Ok(_) => self.set_status(format!("Opened controller log · {}", path.display()), false),
            Err(error) => self.set_status(format!("Could not open log: {error}"), true),
        }
    }

    fn open_profiles_folder(&mut self) {
        let profiles = match crate::config::Config::profiles_dir() {
            Ok(path) => path,
            Err(error) => {
                self.set_status(error, true);
                return;
            }
        };

        if let Err(error) = std::fs::create_dir_all(&profiles) {
            self.set_status(format!("Could not create profiles folder: {error}"), true);
            return;
        }

        match std::process::Command::new("explorer.exe")
            .arg(&profiles)
            .spawn()
        {
            Ok(_) => self.set_status(format!("Opened {}", profiles.display()), false),
            Err(error) => self.set_status(format!("Could not open folder: {error}"), true),
        }
    }

    fn profile_action(&mut self, profile: GameProfile, action: ProfileAction) {
        let result = match action {
            ProfileAction::Game => accounts::open_game(profile).map(|_| "Opened game".to_string()),
            ProfileAction::Twitch => accounts::open_login(profile, accounts::TWITCH_LOGIN, "Twitch"),
            ProfileAction::Kick => accounts::open_login(profile, accounts::KICK_LOGIN, "KICK"),
            ProfileAction::Addons => accounts::open_addons(profile)
                .map(|count| format!("Opened {count} addon installers")),
            ProfileAction::Folder => accounts::open_profile_folder(profile).map(|_| "Opened profile folder".to_string()),
        };

        match result {
            Ok(message) => self.set_status(format!("{} · {}", profile.label(), message), false),
            Err(error) => self.set_status(error, true),
        }
    }
}

impl Drop for ControllerApp {
    fn drop(&mut self) {
        for slot in &mut self.games {
            if let Some(monitor) = slot.monitor.as_ref() {
                monitor.stop();
            }

            if let Some(mut child) = slot.child.take() {
                logging::info(&format!(
                    "controller shutting down; closing {} Firefox PID {}",
                    slot.profile.label(),
                    child.id()
                ));
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl eframe::App for ControllerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.refresh_processes();

        // The browser monitor itself polls every 5s. A 1Hz UI repaint keeps the
        // dashboard fresh without turning the controller into another busy loop.
        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));

        draw_sidebar(self, ui);

        egui::Panel::top("header")
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(24, 18)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Overview")
                            .font(FontId::proportional(26.0))
                            .strong()
                            .color(TEXT),
                    );
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("PokéIdle controller")
                            .font(FontId::proportional(14.0))
                            .color(MUTED),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("STOP ALL")
                                        .size(12.0)
                                        .strong()
                                        .color(TEXT),
                                )
                                .fill(BAD.linear_multiply(0.18))
                                .stroke(Stroke::new(1.0, BAD.linear_multiply(0.55)))
                                .corner_radius(8.0),
                            )
                            .clicked()
                        {
                            self.stop_all();
                        }

                        ui.add_space(8.0);

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("LAUNCH BOTH")
                                        .size(12.0)
                                        .strong()
                                        .color(TEXT),
                                )
                                .fill(ACCENT.linear_multiply(0.8))
                                .stroke(Stroke::new(1.0, ACCENT))
                                .corner_radius(8.0),
                            )
                            .clicked()
                        {
                            self.launch_both();
                        }
                    });
                });
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
                    ui.label(
                        RichText::new(&self.status)
                            .size(12.0)
                            .color(if self.status_error { TEXT } else { MUTED }),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(Margin::symmetric(24, 16)))
            .show(ui, |ui| {
                draw_instance_section(self, ui);
            });

        if self.show_profiles {
            draw_profiles_window(self, ui.ctx());
        }
        if self.show_atlas {
            draw_atlas_window(self, ui.ctx());
        }
        if self.show_market {
            draw_market_window(self, ui.ctx());
        }
    }
}

#[derive(Clone, Copy)]
enum ProfileAction {
    Game,
    Twitch,
    Kick,
    Addons,
    Folder,
}

fn draw_sidebar(app: &mut ControllerApp, ui: &mut egui::Ui) {
    egui::Panel::left("sidebar")
        .resizable(false)
        .default_size(206.0)
        .frame(
            egui::Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER))
                .inner_margin(16.0),
        )
        .show(ui, |ui| {
            ui.label(
                RichText::new("MOTH")
                    .font(FontId::proportional(22.0))
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new("POKEIDLE")
                    .font(FontId::proportional(10.0))
                    .strong()
                    .color(ACCENT),
            );
            ui.add_space(22.0);

            section_label(ui, "WORKSPACE");

            if sidebar_button(ui, "▦  Dashboard", true).clicked() {
                app.set_status("Dashboard · live health polling enabled", false);
            }

            if sidebar_button(ui, "◫  Profiles", false).clicked() {
                app.show_profiles = true;
            }

            if sidebar_button(ui, "⌁  Hunt Atlas", false).clicked() {
                app.show_atlas = true;
            }

            if sidebar_button(ui, "◇  Moth Watch", false).clicked() {
                app.show_market = true;
            }

            if sidebar_button(ui, "≡  Logs", false).clicked() {
                app.open_logs();
            }

            ui.add_space(18.0);
            section_label(ui, "OPERATIONS");

            if sidebar_button(ui, "▶  Launch both", false).clicked() {
                app.launch_both();
            }

            if sidebar_button(ui, "■  Stop all", false).clicked() {
                app.stop_all();
            }

            ui.add_space(20.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(10.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("RUNTIME")
                            .size(10.0)
                            .strong()
                            .color(DIM),
                    );
                    ui.add_space(7.0);
                    runtime_row(ui, "Firefox", if app.games.iter().all(|game| game.headless) { "HEADLESS" } else if app.games.iter().all(|game| !game.headless) { "VISIBLE" } else { "MIXED" }, GOOD);
                    runtime_row(ui, "Monitor", "BiDi", GOOD);
                    runtime_row(ui, "Poll", "5 sec", MUTED);
                });

            ui.add_space(12.0);
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.label(
                    RichText::new("Rust rewrite · Windows")
                        .size(10.0)
                        .color(DIM),
                );
            });
        });
}

fn draw_instance_section(app: &mut ControllerApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Instances")
                .font(FontId::proportional(17.0))
                .strong()
                .color(TEXT),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new("Live state for both isolated Firefox profiles")
                .size(11.0)
                .color(DIM),
        );
    });

    ui.add_space(12.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let gap = ui.spacing().item_spacing.x;
            let card_width = ((ui.available_width() - gap) / 2.0).max(320.0);

            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;

                ui.allocate_ui_with_layout(
                    egui::vec2(card_width, 0.0),
                    Layout::top_down(Align::Min),
                    |ui| {
                        draw_game_card(app, ui, 0, card_width);
                    },
                );

                ui.allocate_ui_with_layout(
                    egui::vec2(card_width, 0.0),
                    Layout::top_down(Align::Min),
                    |ui| {
                        draw_game_card(app, ui, 1, card_width);
                    },
                );
            });

            ui.add_space(16.0);
            draw_runtime_section(app, ui);
        });
}

fn draw_game_card(
    app: &mut ControllerApp,
    ui: &mut egui::Ui,
    index: usize,
    width: f32,
) {
    ui.set_width(width);

    let profile = app.games[index].profile;
    let health = app.games[index].health();
    let running = app.games[index].is_running();

    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(12.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(profile.label())
                        .size(14.0)
                        .strong()
                        .color(TEXT),
                );

                ui.add_space(8.0);
                status_badge(ui, &health, running);

                if running {
                    let mode_label = if app.games[index].headless { "Show Firefox" } else { "Hide Firefox" };
                    let mode_headless = !app.games[index].headless;
                    let clicked = ui
                        .add_sized(
                            [108.0, 30.0],
                            egui::Button::new(
                                RichText::new(mode_label).size(10.0).strong(),
                            )
                            .fill(PANEL_ALT)
                            .stroke(Stroke::new(1.0, BORDER))
                            .corner_radius(7.0),
                        )
                        .clicked();
                    if clicked {
                        app.set_browser_mode(profile, mode_headless);
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let button_text = if running { "Stop" } else { "Launch" };
                    let clicked = ui
                        .add_sized(
                            [84.0, 30.0],
                            egui::Button::new(
                                RichText::new(button_text).size(11.0).strong(),
                            )
                            .fill(if running {
                                BAD.linear_multiply(0.18)
                            } else {
                                ACCENT.linear_multiply(0.82)
                            })
                            .stroke(Stroke::new(
                                1.0,
                                if running { BAD.linear_multiply(0.55) } else { ACCENT },
                            ))
                            .corner_radius(7.0),
                        )
                        .clicked();

                    if clicked {
                        if running {
                            app.stop_one(profile);
                        } else {
                            app.launch_one(profile);
                        }
                    }
                });
            });

            ui.add_space(14.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(
                            if health.activity == "Hunting" {
                                "CURRENT ACTIVITY"
                            } else {
                                "STATE"
                            },
                        )
                        .size(9.0)
                        .strong()
                        .color(DIM),
                    );

                    ui.add_space(3.0);

                    let headline = match health.activity.as_str() {
                        "Hunting" if !health.hunt.is_empty() => {
                            format!("Hunting · {}", compact_text(&health.hunt, 34))
                        }
                        "Hunting" => "Hunting".to_string(),
                        "Center" => "Online · Center".to_string(),
                        "Login" => "Waiting for login".to_string(),
                        _ => health.summary(),
                    };

                    ui.add(
                        egui::Label::new(
                            RichText::new(compact_text(&headline, 52))
                                .size(18.0)
                                .strong()
                                .color(TEXT),
                        )
                        .truncate(),
                    );

                    if !health.active_pokemon.is_empty() {
                        ui.add_space(3.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(compact_text(&health.active_pokemon, 54))
                                    .size(11.0)
                                    .color(MUTED),
                            )
                            .truncate(),
                        );
                    }

                    ui.add_space(9.0);

                    let trainer_level = if health.player_level == 0 {
                        "—".to_string()
                    } else {
                        health.player_level.to_string()
                    };
                    let fallen = health.fallen_count.to_string();

                    ui.horizontal_wrapped(|ui| {
                        mini_metric(ui, "TRAINER LV", &trainer_level);
                        mini_metric(ui, "TRAINER XP", if health.player_xp.is_empty() { "—" } else { &health.player_xp });
                        mini_metric(ui, "POKÉMON LV", if health.pokemon_level.is_empty() { "—" } else { &health.pokemon_level });
                        mini_metric(ui, "POKÉMON XP", if health.pokemon_xp.is_empty() { "—" } else { &health.pokemon_xp });
                        mini_metric(ui, "FALLEN", &fallen);
                    });
                });

            ui.add_space(12.0);

            ui.label(
                RichText::new("RESOURCES")
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    if health.ball_stock.is_empty() {
                        ui.label(RichText::new("Pokéballs: unavailable").size(11.0).color(MUTED));
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for item in &health.ball_stock {
                                resource_chip(ui, item);
                            }
                        });
                    }

                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        let gold = format_number(health.gold);
                        let orbs = format_number(health.orbs);
                        resource_value(ui, "GOLD", &gold, WARN);
                        resource_value(ui, "GEMS", &orbs, ACCENT);
                        resource_value(
                            ui,
                            "AUTO CATCH",

                            if health.autocatch_on { "ON" } else { "OFF" },
                            if health.autocatch_on { GOOD } else { MUTED },
                        );
                        resource_value(
                            ui,
                            "USED",
                            &health.autocatch_balls_used.to_string(),
                            TEXT,
                        );
                        resource_value(
                            ui,
                            "CAPTURES",
                            &health.autocatch_captures.to_string(),
                            TEXT,
                        );
                        resource_value(
                            ui,
                            "SUCCESS",
                            if health.autocatch_rate.is_empty() { "—" } else { &health.autocatch_rate },
                            TEXT,
                        );
                    });
                });

            ui.add_space(12.0);

            ui.label(
                RichText::new("AUTOMATION")
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            let performance_value = if health.performance_fps.is_empty() {
                "Loaded".to_string()
            } else {
                format!("{} FPS", health.performance_fps)
            };

            ui.horizontal_wrapped(|ui| {
                automation_badge(
                    ui,
                    "Auto Catch",
                    if health.autocatch_on { "Active" } else { "Off" },
                    health.autocatch_on,
                );
                automation_badge(
                    ui,
                    "Restock",
                    if health.autocatch_restock.is_empty() { "Off" } else { &health.autocatch_restock },
                    health.autocatch_on && !health.autocatch_restock.eq_ignore_ascii_case("off"),
                );
                automation_badge(
                    ui,
                    "Stream scanner",
                    if health.stream_scan_status.is_empty() {
                        "Waiting"
                    } else {
                        &health.stream_scan_status
                    },
                    health.stream_scan_status.eq_ignore_ascii_case("ok"),
                );
                if health.stream_scan_live > 0 || health.stream_scan_opened > 0 {
                    let streams_value =
                        format!("{} live · {} opened", health.stream_scan_live, health.stream_scan_opened);

                    automation_badge(
                        ui,
                        "Streams",
                        &streams_value,
                        true,
                    );
                }
                automation_badge(ui, "Performance+", &performance_value, true);
            });

            ui.add_space(12.0);
            ui.label(
                RichText::new("OPEN TABS")
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    if health.tabs.is_empty() {
                        ui.label(
                            RichText::new("No top-level tabs reported")
                                .size(10.0)
                                .color(DIM),
                        );
                    } else {
                        for tab in &health.tabs {
                            ui.horizontal(|ui| {
                                let marker = if tab.kind == "Twitch" {
                                    if tab.low_resource { "●" } else { "○" }
                                } else {
                                    "●"
                                };

                                ui.label(
                                    RichText::new(marker)
                                        .size(9.0)
                                        .color(if tab.kind == "Twitch" && tab.low_resource { GOOD } else { MUTED }),
                                );

                                ui.label(
                                    RichText::new(&tab.kind)
                                        .size(9.0)
                                        .strong()
                                        .color(TEXT),
                                );

                                ui.add_space(5.0);

                                let title = if tab.title.is_empty() {
                                    &tab.url
                                } else {
                                    &tab.title
                                };

                                ui.add(
                                    egui::Label::new(
                                        RichText::new(compact_text(title, 54))
                                            .size(9.0)
                                            .color(MUTED),
                                    )
                                    .truncate(),
                                );

                                if tab.kind == "Twitch" {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(if tab.low_resource { "LOW RESOURCE" } else { "FULL" })
                                                .size(8.0)
                                                .strong()
                                                .color(if tab.low_resource { GOOD } else { WARN }),
                                        );
                                    });
                                }
                            });
                        }
                    }
                });

            ui.add_space(12.0);
            ui.label(
                RichText::new("STREAMS & BONUS")
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    if !health.stream_watching.is_empty() {
                        ui.label(
                            RichText::new(format!(
                                "ACTIVE · +{}% XP",
                                format_pct(health.stream_bonus_pct)
                            ))
                            .size(11.0)
                            .strong()
                            .color(GOOD),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Watching: {}",
                                health.stream_watching.join(", ")
                            ))
                            .size(10.0)
                            .color(MUTED),
                        );
                    } else if !health.stream_missing.is_empty() {
                        ui.label(
                            RichText::new("LIVE BONUS AVAILABLE")
                                .size(10.0)
                                .strong()
                                .color(WARN),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Open chat: {}{}",
                                health.stream_missing.join(", "),
                                if health.stream_bonus_pct > 0.0 {
                                    format!(" · +{}% XP", format_pct(health.stream_bonus_pct))
                                } else {
                                    String::new()
                                }
                            ))
                            .size(10.0)
                            .color(TEXT),
                        );
                    } else if !health.stream_bonus_last.is_empty() {
                        let last_seen = if health.stream_bonus_last > 0 {
                            format!(" · {}", format_time(health.stream_bonus_last))
                        } else {
                            String::new()
                        };

                        ui.label(
                            RichText::new(format!(
                                "No active bonus · last seen{}: {}",
                                last_seen,
                                health.stream_bonus_last
                            ))
                            .size(10.0)
                            .color(WARN),
                        );
                    } else {
                        ui.label(
                            RichText::new("No Twitch bonus detected")
                                .size(10.0)
                                .color(DIM),
                        );
                    }

                    ui.add_space(6.0);

                    ui.horizontal_wrapped(|ui| {
                        for tab in health.tabs.iter().filter(|tab| {
                            tab.kind == "Twitch" || tab.kind == "KICK"
                        }) {
                            let state = if tab.kind == "Twitch" {
                                if tab.low_resource { "LOW" } else { "FULL" }
                            } else {
                                "OPEN"
                            };

                            ui.label(
                                RichText::new(format!(
                                    "{} · {} · {}",
                                    tab.kind,
                                    compact_text(
                                        tab.title.rsplit('/').next().unwrap_or(&tab.title),
                                        24
                                    ),
                                    state
                                ))
                                .size(9.0)
                                .color(if tab.kind == "Twitch" && tab.low_resource {
                                    GOOD
                                } else {
                                    MUTED
                                }),
                            );
                        }
                    });
                });

            ui.add_space(12.0);
            ui.label(
                RichText::new("XP BONUS SOURCES")
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    let mut sources = health.xp_sources.clone();
                    for bonus in &health.xp_bonuses {
                        if !sources.iter().any(|source| source == bonus) {
                            sources.push(bonus.clone());
                        }
                    }

                    if sources.is_empty() {
                        ui.label(
                            RichText::new("No active XP bonus detected")
                                .size(10.0)
                                .color(DIM),
                        );
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for source in sources {
                                bonus_chip(ui, &source);
                            }
                        });
                    }
                });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if health.last_game_message_ms > 0 {
                    let age = (chrono_like_now_ms().saturating_sub(health.last_game_message_ms)) / 1000;
                    ui.label(
                        RichText::new(format!("DATA {}s ago", age))
                            .size(8.0)
                            .color(if age <= 10 { GOOD } else { WARN }),
                    );
                    ui.add_space(8.0);
                }

                let addon_color = if health.addon_ok == health.addon_total {
                    GOOD
                } else {
                    WARN
                };

                ui.label(
                    RichText::new(format!("ADDONS {}/{}", health.addon_ok, health.addon_total))
                        .size(10.0)
                        .strong()
                        .color(addon_color),
                );

                if !health.addon_missing.is_empty() {
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("Missing: {}", health.addon_missing.join(" · ")))
                                .size(10.0)
                                .color(WARN),
                        )
                        .truncate(),
                    );
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if health.twitch_tabs > 0 {
                        ui.label(
                            RichText::new(format!(
                                "Twitch LR {}/{}",
                                health.twitch_low_resource_ok, health.twitch_tabs
                            ))
                            .size(10.0)
                            .color(MUTED),
                        );
                    }
                });
            });
        });
}

fn mini_metric(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(8.0).strong().color(DIM));
        ui.add(
            egui::Label::new(
                RichText::new(compact_text(value, 18))
                    .size(11.0)
                    .strong()
                    .color(TEXT),
            )
            .truncate(),
        );
    });
    ui.add_space(16.0);
}

fn resource_chip(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(7.0)
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.label(RichText::new(compact_text(text, 24)).size(10.0).color(TEXT));
        });
}

fn resource_value(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(8.0).strong().color(DIM));
        ui.add(
            egui::Label::new(
                RichText::new(compact_text(value, 18))
                    .size(10.0)
                    .strong()
                    .color(color),
            )
            .truncate(),
        );
    });
    ui.add_space(14.0);
}

fn automation_badge(ui: &mut egui::Ui, label: &str, value: &str, good: bool) {
    egui::Frame::new()
        .fill(if good { GOOD.linear_multiply(0.08) } else { PANEL_ALT })
        .stroke(Stroke::new(1.0, if good { GOOD.linear_multiply(0.28) } else { BORDER }))
        .corner_radius(7.0)
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("● {}", label))
                        .size(9.0)
                        .strong()
                        .color(if good { GOOD } else { MUTED }),
                );
                ui.add_space(4.0);
                ui.label(RichText::new(compact_text(value, 24)).size(9.0).color(TEXT));
            });
        });
}

fn bonus_chip(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(ACCENT.linear_multiply(0.10))
        .stroke(Stroke::new(1.0, ACCENT.linear_multiply(0.28)))
        .corner_radius(7.0)
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.label(RichText::new(compact_text(text, 38)).size(10.0).strong().color(TEXT));
        });
}


fn draw_runtime_section(app: &mut ControllerApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Runtime")
                .font(FontId::proportional(17.0))
                .strong()
                .color(TEXT),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new("Controller-side diagnostics")
                .size(11.0)
                .color(DIM),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button("Open Profiles").clicked() {
                app.show_profiles = true;
            }
            if ui.button("Open Logs").clicked() {
                app.open_logs();
            }
        });
    });

    ui.add_space(10.0);

    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                runtime_chip(ui, "Firefox", "Headless", GOOD);
                runtime_chip(ui, "BiDi", "Active", GOOD);
                runtime_chip(ui, "Profiles", "2 isolated", MUTED);
                runtime_chip(ui, "Stream chat", "First scan 30s · hourly", MUTED);
                runtime_chip(ui, "UI repaint", "1 sec", MUTED);
            });
        });
}


fn game_selector(
    ui: &mut egui::Ui,
    selected: &mut GameProfile,
) {
    ui.horizontal(|ui| {
        for profile in GameProfile::ALL {
            let active = *selected == profile;
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(profile.label())
                            .size(10.0)
                            .strong()
                            .color(if active { TEXT } else { MUTED }),
                    )
                    .fill(if active {
                        ACCENT.linear_multiply(0.22)
                    } else {
                        PANEL_ALT
                    })
                    .stroke(Stroke::new(
                        1.0,
                        if active { ACCENT } else { BORDER },
                    ))
                    .corner_radius(7.0),
                )
                .clicked()
            {
                *selected = profile;
            }
        }
    });
}

fn draw_atlas_window(app: &mut ControllerApp, ctx: &egui::Context) {
    egui::Window::new("Hunt Atlas")
        .resizable(true)
        .default_width(900.0)
        .default_height(620.0)
        .min_width(700.0)
        .min_height(480.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(12.0)
                .inner_margin(18.0),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Hunt Atlas")
                        .font(FontId::proportional(22.0))
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Map intelligence + observed XP/hour")
                        .size(12.0)
                        .color(MUTED),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        app.show_atlas = false;
                    }
                });
            });

            ui.add_space(14.0);
            game_selector(ui, &mut app.atlas_profile);

            let index = ControllerApp::game_index(app.atlas_profile);
            let health = app.games[index].health();

            ui.add_space(12.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(10.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Current hunt").size(9.0).strong().color(DIM));
                        ui.add_space(8.0);
                        ui.label(RichText::new(if health.hunt.is_empty() { "—" } else { &health.hunt })
                            .size(14.0).strong().color(TEXT));
                        ui.add_space(18.0);
                        ui.label(RichText::new(format!("Trainer Lv {}", health.player_level))
                            .size(10.0).color(MUTED));
                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("{} hunts available", health.hunts.len()))
                            .size(10.0).color(MUTED));
                    });
                });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Filter").size(10.0).strong().color(DIM));
                ui.add_sized(
                    [260.0, 28.0],
                    egui::TextEdit::singleline(&mut app.atlas_search)
                        .hint_text("hunt or Pokémon"),
                );
            });

            ui.add_space(10.0);

            let search = app.atlas_search.to_lowercase();
            let mut hunts: Vec<_> = health
                .hunts
                .iter()
                .filter(|hunt| {
                    search.is_empty()
                        || hunt.name.to_lowercase().contains(&search)
                        || hunt.slug.to_lowercase().contains(&search)
                        || hunt.species.iter().any(|name| name.to_lowercase().contains(&search))
                })
                .cloned()
                .collect();

            hunts.sort_by(|a, b| {
                if a.slug == health.hunt {
                    std::cmp::Ordering::Less
                } else if b.slug == health.hunt {
                    std::cmp::Ordering::Greater
                } else {
                    b.xp_per_hour.cmp(&a.xp_per_hour).then(a.level.cmp(&b.level))
                }
            });

            egui::ScrollArea::vertical()
                .id_salt("atlas_list")
                .max_height(410.0)
                .show(ui, |ui| {
                    for hunt in hunts {
                        let current = !health.hunt_slug.is_empty()
                            && hunt.slug.eq_ignore_ascii_case(&health.hunt_slug);

                        egui::Frame::new()
                            .fill(if current { ACCENT.linear_multiply(0.08) } else { PANEL_ALT })
                            .stroke(Stroke::new(1.0, if current { ACCENT.linear_multiply(0.35) } else { BORDER }))
                            .corner_radius(9.0)
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(&hunt.name).size(12.0).strong().color(TEXT));
                                        ui.label(
                                            RichText::new(format!(
                                                "Lv {} · {} species",
                                                hunt.level,
                                                hunt.species.len()
                                            ))
                                            .size(9.0)
                                            .color(MUTED),
                                        );
                                    });

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        let xp = format_rate(hunt.xp_per_hour);
                                        let pxp = format_rate(hunt.pokemon_xp_per_hour);
                                        let kills = if hunt.kills_per_hour > 0 {
                                            format!("{} kills/h", hunt.kills_per_hour)
                                        } else {
                                            "warming up".to_string()
                                        };

                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(format!("{} trainer XP/h", xp)).size(11.0).strong().color(if hunt.xp_per_hour > 0 { GOOD } else { MUTED }));
                                            ui.label(RichText::new(format!("{} Pokémon XP/h · {}", pxp, kills)).size(9.0).color(MUTED));
                                        });

                                        ui.add_space(18.0);

                                        let button = if current { "Current" } else { "Go" };
                                        let clicked = ui.add_sized([70.0, 28.0], egui::Button::new(button)).clicked();
                                        if clicked && !current {
                                            let profile_label = app.atlas_profile.label();
                                            let monitor = app.games[index].monitor.clone();

                                            if let Some(monitor) = monitor {
                                                let hunt_slug = hunt.slug.clone();
                                                let hunt_name = hunt.name.clone();
                                                monitor.send(json!({
                                                    "t": "hunt.select",
                                                    "slug": hunt_slug
                                                }));
                                                app.set_status(
                                                    format!("{} · changing hunt to {}", profile_label, hunt_name),
                                                    false,
                                                );
                                            } else {
                                                app.set_status(
                                                    format!("{} is not running.", profile_label),
                                                    true,
                                                );
                                            }
                                        }
                                    });
                                });
                            });

                        ui.add_space(7.0);
                    }

                    if health.hunts.is_empty() {
                        ui.label(RichText::new("No hunt data yet. The controller bridge must receive the game's welcome message first.")
                            .size(11.0).color(DIM));
                    }
                });
        });
}

fn draw_market_window(app: &mut ControllerApp, ctx: &egui::Context) {
    egui::Window::new("Moth Watch")
        .resizable(true)
        .default_width(1000.0)
        .default_height(650.0)
        .min_width(780.0)
        .min_height(520.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(12.0)
                .inner_margin(18.0),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Moth Watch")
                        .font(FontId::proportional(22.0))
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new("RMT market browser")
                        .size(12.0)
                        .color(MUTED),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        app.show_market = false;
                    }
                });
            });

            ui.add_space(14.0);
            game_selector(ui, &mut app.market_profile);

            let index = ControllerApp::game_index(app.market_profile);
            let health = app.games[index].health();

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                let gold = format_number(health.gold);
                let orbs = format_number(health.orbs);
                ui.label(RichText::new(format!("Gold {}", gold)).size(11.0).strong().color(WARN));
                ui.add_space(14.0);
                ui.label(RichText::new(format!("Gems {}", orbs)).size(11.0).strong().color(ACCENT));
                ui.add_space(20.0);

                if ui.button("Refresh market").clicked() {
                    if let Some(monitor) = app.games[index].monitor.as_ref() {
                        monitor.send(json!({ "t": "market.itens" }));
                        app.set_status(format!("{} · market refresh requested", app.market_profile.label()), false);
                    } else {
                        app.set_status(format!("{} is not running.", app.market_profile.label()), true);
                    }
                }
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Find item").size(10.0).strong().color(DIM));
                ui.add_sized(
                    [300.0, 28.0],
                    egui::TextEdit::singleline(&mut app.market_search)
                        .hint_text("item name"),
                );

                ui.add_space(10.0);

                for currency in ["gold", "orb"] {
                    let active = app.market_currency == currency;
                    if ui
                        .add(
                            egui::Button::new(RichText::new(if currency == "gold" { "Gold" } else { "Gems" }).size(10.0))
                                .fill(if active { ACCENT.linear_multiply(0.20) } else { PANEL_ALT })
                                .stroke(Stroke::new(1.0, if active { ACCENT } else { BORDER }))
                                .corner_radius(7.0),
                        )
                        .clicked()
                    {
                        app.market_currency = currency.to_string();
                    }
                }
            });

            ui.add_space(10.0);

            let search = app.market_search.to_lowercase();

            egui::ScrollArea::vertical()
                .id_salt("market_catalog")
                .max_height(250.0)
                .show(ui, |ui| {
                    if health.market_summary.is_empty() {
                        ui.label(
                            RichText::new("No market summary loaded. Press Refresh market.")
                                .size(11.0)
                                .color(DIM),
                        );

                        ui.add_space(6.0);

                        let catalog: Vec<_> = health
                            .market_catalog
                            .iter()
                            .filter(|item| {
                                search.is_empty()
                                    || item.name.to_lowercase().contains(&search)
                            })
                            .take(35)
                            .cloned()
                            .collect();

                        ui.horizontal_wrapped(|ui| {
                            for item in catalog {
                                let label = compact_text(&item.name, 22);
                                if ui.button(label).clicked() {
                                    let profile_label = app.market_profile.label();
                                    let monitor = app.games[index].monitor.clone();
                                    if let Some(monitor) = monitor {
                                        let item_id = item.id;
                                        let item_name = item.name.clone();
                                        let currency = app.market_currency.clone();

                                        monitor.send(json!({
                                            "t": "market.item",
                                            "itemId": item_id,
                                            "moeda": currency
                                        }));
                                        app.set_status(
                                            format!("{} · inspecting {}", profile_label, item_name),
                                            false,
                                        );
                                    } else {
                                        app.set_status(
                                            format!("{} is not running.", profile_label),
                                            true,
                                        );
                                    }
                                }
                            }
                        });
                    } else {
                        for item in health.market_summary.iter().filter(|item| {
                            search.is_empty()
                                || item.name.to_lowercase().contains(&search)
                        }).take(50) {
                            egui::Frame::new()
                                .fill(PANEL_ALT)
                                .stroke(Stroke::new(1.0, BORDER))
                                .corner_radius(8.0)
                                .inner_margin(9.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(compact_text(&item.name, 32))
                                                    .size(11.0)
                                                    .strong()
                                                    .color(TEXT),
                                            );

                                            let gold = if item.gold_min > 0 {
                                                format!("{} gold", format_number(item.gold_min))
                                            } else {
                                                "— gold".to_string()
                                            };
                                            let gems = if item.orb_min > 0 {
                                                format!("{} gems", format_number(item.orb_min))
                                            } else {
                                                "— gems".to_string()
                                            };

                                            ui.label(
                                                RichText::new(format!(
                                                    "{} · {} · {} listings",
                                                    gold,
                                                    gems,
                                                    item.listings
                                                ))
                                                .size(9.0)
                                                .color(MUTED),
                                            );
                                        });

                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            let profile_label = app.market_profile.label();
                                            let monitor = app.games[index].monitor.clone();
                                            if ui.button("Inspect").clicked() {
                                                if let Some(monitor) = monitor {
                                                    let item_id = item.item_id;
                                                    let item_name = item.name.clone();
                                                    let currency = app.market_currency.clone();
                                                    monitor.send(json!({
                                                        "t": "market.item",
                                                        "itemId": item_id,
                                                        "moeda": currency
                                                    }));
                                                    app.set_status(
                                                        format!("{} · inspecting {}", profile_label, item_name),
                                                        false,
                                                    );
                                                } else {
                                                    app.set_status(
                                                        format!("{} is not running.", profile_label),
                                                        true,
                                                    );
                                                }
                                            }
                                        });
                                    });
                                });

                            ui.add_space(5.0);
                        }
                    }
                });

            ui.add_space(12.0);

            ui.label(RichText::new("LISTINGS").size(9.0).strong().color(DIM));
            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .id_salt("market_listings")
                .max_height(300.0)
                .show(ui, |ui| {
                    if health.market_listings.is_empty() {
                        ui.label(
                            RichText::new("No item listings loaded. Pick an item above to inspect its RMT listings.")
                                .size(11.0)
                                .color(DIM),
                        );
                    } else {
                        for listing in &health.market_listings {
                            if listing.currency != app.market_currency {
                                continue;
                            }

                            egui::Frame::new()
                                .fill(PANEL_ALT)
                                .stroke(Stroke::new(1.0, BORDER))
                                .corner_radius(8.0)
                                .inner_margin(9.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(compact_text(&listing.name, 30)).size(11.0).strong().color(TEXT));
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} × {} · seller {}",
                                                    format_number(listing.price),
                                                    listing.quantity,
                                                    compact_text(&listing.seller, 20)
                                                ))
                                                .size(9.0)
                                                .color(MUTED),
                                            );
                                        });

                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if ui.button("Buy 1").clicked() {
                                                let profile_label = app.market_profile.label();
                                                let monitor = app.games[index].monitor.clone();

                                                if let Some(monitor) = monitor {
                                                    let listing_id = listing.id;
                                                    let listing_name = listing.name.clone();
                                                    let currency = listing.currency.clone();
                                                    monitor.send(json!({
                                                        "t": "market.comprar",
                                                        "id": listing_id,
                                                        "qtd": 1,
                                                        "preco": listing.price,
                                                        "moeda": currency
                                                    }));
                                                    app.set_status(
                                                        format!("{} · buy command sent for {}", profile_label, listing_name),
                                                        false,
                                                    );
                                                } else {
                                                    app.set_status(
                                                        format!("{} is not running.", profile_label),
                                                        true,
                                                    );
                                                }
                                            }
                                        });
                                    });
                                });
                            ui.add_space(6.0);
                        }
                    }
                });
        });
}

fn chrono_like_now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn format_pct(value: f32) -> String {
    if value.fract().abs() < 0.01 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}

fn format_time(timestamp_ms: u64) -> String {
    let seconds = timestamp_ms / 1000;
    let minute = (seconds / 60) % 60;
    let hour = (seconds / 3600) % 24;
    format!("{:02}:{:02}", hour, minute)
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

fn format_rate(value: u64) -> String {
    format_number(value)
}

fn draw_profiles_window(app: &mut ControllerApp, ctx: &egui::Context) {
    egui::Window::new("Profiles")
        .title_bar(true)
        .resizable(true)
        .default_width(700.0)
        .default_height(520.0)
        .min_width(620.0)
        .min_height(420.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(12.0)
                .inner_margin(18.0),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Profiles")
                        .font(FontId::proportional(22.0))
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Account, browser and addon entry points")
                        .size(12.0)
                        .color(MUTED),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        app.show_profiles = false;
                    }
                });
            });

            ui.add_space(18.0);

            for profile in GameProfile::ALL {
                draw_profile_card(app, ui, profile);
                ui.add_space(12.0);
            }

            ui.add_space(2.0);
            ui.label(
                RichText::new(
                    "Login sessions stay inside each Firefox profile. Addon installers open through Violentmonkey.",
                )
                .size(10.0)
                .color(DIM),
            );
        });
}

fn draw_profile_card(app: &mut ControllerApp, ui: &mut egui::Ui, profile: GameProfile) {
    let config = crate::config::Config::for_profile(profile);
    let (profile_state, browser) = match config {
        Ok(ref config) => {
            let state = if config.profile_dir.exists() {
                "Profile ready"
            } else {
                "Created on first launch"
            };

            let browser = if config
                .firefox_executable
                .to_string_lossy()
                .to_lowercase()
                .contains("developer")
            {
                "Firefox Developer Edition"
            } else {
                "Firefox"
            };

            (state.to_string(), browser.to_string())
        }
        Err(error) => ("Unavailable".to_string(), error),
    };

    egui::Frame::new()
        .fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(profile.label())
                        .font(FontId::proportional(15.0))
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(8.0);
                ui.label(RichText::new(profile_state).size(11.0).color(GOOD));
            });

            ui.add_space(4.0);
            ui.label(
                RichText::new(browser)
                    .size(10.0)
                    .color(MUTED),
            );

            ui.add_space(12.0);

            ui.horizontal_wrapped(|ui| {
                profile_button(ui, "Open Game", || app.profile_action(profile, ProfileAction::Game));
                profile_button(ui, "Twitch", || app.profile_action(profile, ProfileAction::Twitch));
                profile_button(ui, "KICK", || app.profile_action(profile, ProfileAction::Kick));
                profile_button(ui, "Addons", || app.profile_action(profile, ProfileAction::Addons));
                profile_button(ui, "Profile folder", || app.profile_action(profile, ProfileAction::Folder));
            });
        });
}

fn profile_button(ui: &mut egui::Ui, label: &str, mut action: impl FnMut()) {
    if ui
        .add_sized(
            [124.0, 32.0],
            egui::Button::new(RichText::new(label).size(11.0)),
        )
        .clicked()
    {
        action();
    }
}

fn status_badge(ui: &mut egui::Ui, health: &Health, running: bool) {
    let (label, color) = if !running {
        ("STOPPED", DIM)
    } else if health.state == "Running" && health.activity == "Hunting" {
        ("HUNTING", GOOD)
    } else if health.state == "Running" && health.logged_in {
        ("ONLINE", GOOD)
    } else if health.state == "Connecting" || health.state == "Reconnecting" {
        ("CONNECTING", WARN)
    } else if health.state == "Running" && !health.logged_in {
        ("LOGIN", WARN)
    } else {
        ("ERROR", BAD)
    };

    ui.label(
        RichText::new(format!("● {}", label))
            .size(10.0)
            .strong()
            .color(color),
    );
}

fn metric(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(label)
                .size(9.0)
                .strong()
                .color(DIM),
        );
        ui.label(
            RichText::new(compact_text(value, 22))
                .size(11.0)
                .color(TEXT),
        );
    });
    ui.add_space(18.0);
}

fn runtime_chip(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    egui::Frame::new()
        .fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8.0)
        .inner_margin(Margin::symmetric(11, 8))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(label).size(9.0).strong().color(DIM));
                ui.label(RichText::new(value).size(11.0).color(color));
            });
        });
}

fn runtime_row(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(10.0).color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).size(10.0).strong().color(color));
        });
    });
    ui.add_space(5.0);
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(9.0)
            .strong()
            .color(DIM),
    );
    ui.add_space(5.0);
}

fn sidebar_button<'a>(ui: &mut egui::Ui, text: &'a str, selected: bool) -> egui::Response {
    let fill = if selected {
        ACCENT.linear_multiply(0.18)
    } else {
        Color32::TRANSPARENT
    };

    ui.add_sized(
        [174.0, 34.0],
        egui::Button::new(
            RichText::new(text)
                .size(11.0)
                .strong()
                .color(if selected { TEXT } else { MUTED }),
        )
        .fill(fill)
        .stroke(if selected {
            Stroke::new(1.0, ACCENT.linear_multiply(0.6))
        } else {
            Stroke::NONE
        })
        .corner_radius(7.0),
    )
}

fn compact_text(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let mut out: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        out.push('…');
    }

    out
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
            (TextStyle::Small, FontId::proportional(10.0)),
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
    });
}

pub fn run() -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Moth · PokéIdle")
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Moth · PokéIdle",
        options,
        Box::new(|cc| Ok(Box::new(ControllerApp::new(cc)))),
    )
    .map_err(|error| format!("could not start controller UI: {error}"))
}
