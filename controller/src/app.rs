use crate::accounts;
use crate::config::GameProfile;
use crate::firefox;
use crate::logging;
use crate::monitor::{Health, MonitorHandle};
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
}

impl GameSlot {
    fn new(profile: GameProfile) -> Self {
        Self {
            profile,
            child: None,
            monitor: None,
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
        let index = Self::game_index(profile);
        self.refresh_processes();

        if self.games[index].is_running() {
            self.set_status(format!("{} is already running.", profile.label()), false);
            return;
        }

        self.games[index].child = None;
        self.games[index].monitor = None;

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

        match firefox::launch(&config) {
            Ok((child, monitor)) => {
                let pid = child.id();
                self.games[index].child = Some(child);
                self.games[index].monitor = Some(monitor);
                self.set_status(
                    format!(
                        "{} started · headless Firefox · Rust BiDi health monitor",
                        profile.label()
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

    fn stop_one(&mut self, profile: GameProfile) {
        let index = Self::game_index(profile);
        let slot = &mut self.games[index];

        if let Some(monitor) = slot.monitor.as_ref() {
            monitor.stop();
        }

        let Some(mut child) = slot.child.take() else {
            slot.monitor = None;
            self.set_status(format!("{} is already stopped.", profile.label()), false);
            return;
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
                ui.add_space(16.0);
                draw_runtime_section(self, ui);
            });

        if self.show_profiles {
            draw_profiles_window(self, ui.ctx());
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
                    runtime_row(ui, "Firefox", "HEADLESS", GOOD);
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
            RichText::new("Two isolated browser profiles")
                .size(11.0)
                .color(DIM),
        );
    });

    ui.add_space(12.0);

    let available = ui.available_width();
    let gap = 14.0;
    let card_width = ((available - gap) / 2.0).max(320.0);

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;

        for index in 0..2 {
            draw_game_card(app, ui, index, card_width);
        }
    });
}

fn draw_game_card(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize, width: f32) {
    let profile = app.games[index].profile;
    let health = app.games[index].health();
    let running = app.games[index].is_running();

    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(12.0)
        .inner_margin(15.0)
        .show(ui, |ui| {
            ui.set_width(width - 30.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(profile.label())
                        .font(FontId::proportional(13.0))
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(7.0);
                status_badge(ui, &health, running);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_sized(
                            [78.0, 28.0],
                            egui::Button::new(
                                RichText::new(if running { "Stop" } else { "Launch" })
                                    .size(11.0)
                                    .strong(),
                            )
                            .fill(if running {
                                BAD.linear_multiply(0.18)
                            } else {
                                ACCENT.linear_multiply(0.82)
                            })
                            .stroke(Stroke::new(
                                1.0,
                                if running {
                                    BAD.linear_multiply(0.55)
                                } else {
                                    ACCENT
                                },
                            ))
                            .corner_radius(7.0),
                        )
                        .clicked()
                    {
                        if running {
                            app.stop_one(profile);
                        } else {
                            app.launch_one(profile);
                        }
                    }
                });
            });

            ui.add_space(12.0);

            ui.label(
                RichText::new(compact_text(&health.summary(), 60))
                    .font(FontId::proportional(21.0))
                    .strong()
                    .color(TEXT),
            );

            ui.add_space(3.0);

            ui.label(
                RichText::new(compact_text(&health.details(), 76))
                    .size(11.0)
                    .color(MUTED),
            );

            ui.add_space(15.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(9.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        metric(ui, "HUNT", if health.hunt.is_empty() { "—" } else { &health.hunt });
                        metric(ui, "FALLEN", &health.fallen_count.to_string());
                        metric(ui, "FPS", if health.performance_fps.is_empty() { "—" } else { &health.performance_fps });
                        metric(ui, "SCENE", if health.performance_scene_runs.is_empty() { "—" } else { &health.performance_scene_runs });
                    });
                });

            ui.add_space(12.0);

            let addon_fraction = if health.addon_total == 0 {
                0.0
            } else {
                f32::from(health.addon_ok) / f32::from(health.addon_total)
            };

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Addons {}/{}", health.addon_ok, health.addon_total))
                        .size(11.0)
                        .strong()
                        .color(if health.addon_ok == health.addon_total {
                            GOOD
                        } else {
                            WARN
                        }),
                );

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

            ui.add(
                egui::ProgressBar::new(addon_fraction)
                    .desired_height(5.0)
                    .fill(if health.addon_ok == health.addon_total {
                        GOOD
                    } else {
                        WARN
                    })
                    .show_percentage(),
            );

            if !health.addon_missing.is_empty() {
                ui.add_space(5.0);
                ui.label(
                    RichText::new(format!(
                        "Missing: {}",
                        health.addon_missing.join(" · ")
                    ))
                    .size(10.0)
                    .color(WARN),
                );
            }

            if !health.active_pokemon.is_empty() {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(compact_text(&health.active_pokemon, 78))
                        .size(10.0)
                        .color(DIM),
                );
            }
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
                runtime_chip(ui, "Stream chat", "Auto-open hourly", MUTED);
                runtime_chip(ui, "UI repaint", "1 sec", MUTED);
            });
        });
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
