use crate::accounts::AccountsWindow;
use crate::config::{Config, GameProfile};
use crate::firefox;
use crate::monitor;
use crate::logging;
use native_windows_gui as nwg;
use std::cell::RefCell;
use std::process::Child;
use std::rc::Rc;

struct State {
    game1: Option<Child>,
    game2: Option<Child>,
    game1_monitor: Option<monitor::MonitorHandle>,
    game2_monitor: Option<monitor::MonitorHandle>,
    accounts: Option<AccountsWindow>,
}

pub fn run() -> Result<(), String> {
    logging::init();
    logging::info("controller starting");
    nwg::init().map_err(|error| format!("could not initialize Windows GUI: {error}"))?;
    nwg::Font::set_global_family("Segoe UI")
        .map_err(|error| format!("could not set default Windows font: {error}"))?;

    let mut window = nwg::Window::default();
    let mut title = nwg::Label::default();
    let mut subtitle = nwg::Label::default();
    let mut game1_label = nwg::Label::default();
    let mut game1_status = nwg::Label::default();
    let mut game1_button = nwg::Button::default();
    let mut game2_label = nwg::Label::default();
    let mut game2_status = nwg::Label::default();
    let mut game2_button = nwg::Button::default();
    let mut launch_both_button = nwg::Button::default();
    let mut accounts_button = nwg::Button::default();
    let mut logs_button = nwg::Button::default();
    let mut profiles_button = nwg::Button::default();
    let mut close_button = nwg::Button::default();
    let mut status = nwg::Label::default();
    let mut health_notice = nwg::Notice::default();

    nwg::Window::builder()
        .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
        .size((520, 390))
        .position((500, 300))
        .title("Moth · PokéIdle")
        .build(&mut window)
        .map_err(|error| format!("could not create controller window: {error}"))?;

    nwg::Label::builder()
        .text("Moth · PokéIdle")
        .flags(nwg::LabelFlags::VISIBLE)
        .position((24, 20))
        .size((470, 34))
        .parent(&window)
        .build(&mut title)
        .map_err(|error| format!("could not create title: {error}"))?;

    nwg::Label::builder()
        .text("2 isolated profiles · shared Twitch/KICK logins · low overhead")
        .flags(nwg::LabelFlags::VISIBLE)
        .position((24, 56))
        .size((470, 28))
        .parent(&window)
        .build(&mut subtitle)
        .map_err(|error| format!("could not create subtitle: {error}"))?;

    nwg::Label::builder()
        .text("GAME 1")
        .flags(nwg::LabelFlags::VISIBLE)
        .position((24, 104))
        .size((90, 28))
        .parent(&window)
        .build(&mut game1_label)
        .map_err(|error| format!("could not create Game 1 label: {error}"))?;

    nwg::Label::builder()
        .text("Not running")
        .flags(nwg::LabelFlags::VISIBLE)
        .position((116, 104))
        .size((180, 28))
        .parent(&window)
        .build(&mut game1_status)
        .map_err(|error| format!("could not create Game 1 status: {error}"))?;

    nwg::Button::builder()
        .text("Launch Game 1")
        .position((315, 98))
        .size((160, 36))
        .parent(&window)
        .build(&mut game1_button)
        .map_err(|error| format!("could not create Game 1 button: {error}"))?;

    nwg::Label::builder()
        .text("GAME 2")
        .flags(nwg::LabelFlags::VISIBLE)
        .position((24, 160))
        .size((90, 28))
        .parent(&window)
        .build(&mut game2_label)
        .map_err(|error| format!("could not create Game 2 label: {error}"))?;

    nwg::Label::builder()
        .text("Not running")
        .flags(nwg::LabelFlags::VISIBLE)
        .position((116, 160))
        .size((180, 28))
        .parent(&window)
        .build(&mut game2_status)
        .map_err(|error| format!("could not create Game 2 status: {error}"))?;

    nwg::Button::builder()
        .text("Launch Game 2")
        .position((315, 154))
        .size((160, 36))
        .parent(&window)
        .build(&mut game2_button)
        .map_err(|error| format!("could not create Game 2 button: {error}"))?;

    nwg::Button::builder()
        .text("Launch Both")
        .position((24, 220))
        .size((145, 38))
        .parent(&window)
        .build(&mut launch_both_button)
        .map_err(|error| format!("could not create Launch Both button: {error}"))?;

    nwg::Button::builder()
        .text("Open Logs")
        .position((180, 220))
        .size((145, 38))
        .parent(&window)
        .build(&mut logs_button)
        .map_err(|error| format!("could not create Logs button: {error}"))?;

    nwg::Button::builder()
        .text("Accounts")
        .position((24, 268))
        .size((145, 38))
        .parent(&window)
        .build(&mut accounts_button)
        .map_err(|error| format!("could not create Accounts button: {error}"))?;

    nwg::Button::builder()
        .text("Open Profiles")
        .position((180, 268))
        .size((145, 38))
        .parent(&window)
        .build(&mut profiles_button)
        .map_err(|error| format!("could not create Profiles button: {error}"))?;

    nwg::Button::builder()
        .text("Close")
        .position((336, 268))
        .size((139, 38))
        .parent(&window)
        .build(&mut close_button)
        .map_err(|error| format!("could not create Close button: {error}"))?;

    nwg::Notice::builder()
        .parent(&window)
        .build(&mut health_notice)
        .map_err(|error| format!("could not create health notice: {error}"))?;

    nwg::Label::builder()
        .text("Starting…")
        .flags(nwg::LabelFlags::VISIBLE)
        .position((24, 328))
        .size((450, 40))
        .parent(&window)
        .build(&mut status)
        .map_err(|error| format!("could not create status: {error}"))?;

    let state = Rc::new(RefCell::new(State {
        game1: None,
        game2: None,
        game1_monitor: None,
        game2_monitor: None,
        accounts: None,
    }));

    game1_status.set_text("Stopped");
    game2_status.set_text("Stopped");
    status.set_text("Ready · launch only the profiles you need");

    let game1_button_handle = game1_button.handle;
    let game2_button_handle = game2_button.handle;
    let launch_both_button_handle = launch_both_button.handle;
    let accounts_button_handle = accounts_button.handle;
    let logs_button_handle = logs_button.handle;
    let profiles_button_handle = profiles_button.handle;
    let close_button_handle = close_button.handle;
    let health_notice_handle = health_notice.handle;
    let window_handle = window.handle;

    let state_for_events = state.clone();
    let state_for_health = state.clone();

    let notice_sender = health_notice.sender();

    let event_handler = nwg::full_bind_event_handler(
        &window.handle,
        move |event, _data, handle| {
            use nwg::Event;

            match event {
                Event::OnNotice if handle == health_notice_handle => {
                    let state = state_for_health.borrow();
                    update_health_label(state.game1_monitor.as_ref(), &game1_status);
                    update_health_label(state.game2_monitor.as_ref(), &game2_status);
                }
                Event::OnWindowClose => {
                    if handle == window_handle {
                        nwg::stop_thread_dispatch();
                    }
                }
                Event::OnButtonClick => {
                    if handle == game1_button_handle {
                        let mut state = state_for_events.borrow_mut();
                        toggle_one(
                            GameProfile::Game1,
                            &mut state,
                            &game1_button,
                            &game1_status,
                            &status,
                            notice_sender,
                        );
                    } else if handle == game2_button_handle {
                        let mut state = state_for_events.borrow_mut();
                        toggle_one(
                            GameProfile::Game2,
                            &mut state,
                            &game2_button,
                            &game2_status,
                            &status,
                            notice_sender,
                        );
                    } else if handle == launch_both_button_handle {
                        let mut state = state_for_events.borrow_mut();

                        launch_one(
                            GameProfile::Game1,
                            &mut state,
                            &game1_button,
                            &game1_status,
                            &status,
                            notice_sender,
                        );
                        launch_one(
                            GameProfile::Game2,
                            &mut state,
                            &game2_button,
                            &game2_status,
                            &status,
                            notice_sender,
                        );
                    } else if handle == accounts_button_handle {
                        let mut state = state_for_events.borrow_mut();

                        if state.accounts.is_none() {
                            match AccountsWindow::build() {
                                Ok(accounts) => {
                                    state.accounts = Some(accounts);
                                }
                                Err(error) => {
                                    nwg::simple_message("Moth", &error);
                                }
                            }
                        }

                        if let Some(accounts) = state.accounts.as_ref() {
                            accounts.show();
                        }
                    } else if handle == logs_button_handle {
                        open_logs(&status);
                    } else if handle == profiles_button_handle {
                        open_profiles_folder(&status);
                    } else if handle == close_button_handle {
                        nwg::stop_thread_dispatch();
                    }
                }
                _ => {}
            }
        },
    );

    nwg::dispatch_thread_events();
    nwg::unbind_event_handler(&event_handler);
    Ok(())
}

fn toggle_one(
    profile: GameProfile,
    state: &mut State,
    button: &nwg::Button,
    notice_sender: nwg::NoticeSender,
    profile_status: &nwg::Label,
    global_status: &nwg::Label,
) {
    let running = match profile {
        GameProfile::Game1 => state.game1.as_mut().and_then(|child| child.try_wait().ok()).flatten().is_none() && state.game1.is_some(),
        GameProfile::Game2 => state.game2.as_mut().and_then(|child| child.try_wait().ok()).flatten().is_none() && state.game2.is_some(),
    };

    if running {
        stop_one(profile, state, button, profile_status, global_status);
    } else {
        launch_one(profile, state, button, profile_status, global_status, notice_sender);
    }
}

fn launch_one(
    profile: GameProfile,
    state: &mut State,
    button: &nwg::Button,
    profile_status: &nwg::Label,
    global_status: &nwg::Label,
    notice_sender: nwg::NoticeSender,
) {
    let (child_slot, monitor_slot): (&mut Option<Child>, &mut Option<monitor::MonitorHandle>) =
        match profile {
            GameProfile::Game1 => (&mut state.game1, &mut state.game1_monitor),
            GameProfile::Game2 => (&mut state.game2, &mut state.game2_monitor),
        };
    if let Some(child) = child_slot.as_mut() {
        match child.try_wait() {
            Ok(None) => {
                button.set_text(&format!("Stop {}", profile.label()));
                profile_status.set_text("Running");
                global_status.set_text(&format!("{} is already running.", profile.label()));
                return;
            }
            Ok(Some(_)) | Err(_) => {
                *child_slot = None;
                *monitor_slot = None;
                profile_status.set_text("Stopped");
                button.set_text(&format!("Launch {}", profile.label()));
            }
        }
    }

    let config = match Config::for_profile(profile) {
        Ok(config) => config,
        Err(error) => {
            profile_status.set_text("Unavailable");
            global_status.set_text(&format!("{}: {error}", profile.label()));
            return;
        }
    };

    logging::info(&format!("launching {} with Firefox profile {} on BiDi port {}", profile.label(), config.profile_dir.display(), config.remote_debug_port));

    match firefox::launch(&config, notice_sender) {
        Ok((child, monitor)) => {
            logging::info(&format!("{} Firefox spawned with PID {}", profile.label(), child.id()));
            *child_slot = Some(child);
            *monitor_slot = Some(monitor);
            button.set_text(&format!("Stop {}", profile.label()));
            profile_status.set_text("Headless · connecting");
            global_status.set_text(&format!(
                "Started {} · headless Firefox + Rust BiDi monitor",
                profile.label()
            ));
        }
        Err(error) => {
            logging::error(&format!("{} launch failed: {}", profile.label(), error));
            profile_status.set_text("Launch failed");
            global_status.set_text(&format!("{}: {error}", profile.label()));
        }
    }
}

fn stop_one(
    profile: GameProfile,
    state: &mut State,
    button: &nwg::Button,
    profile_status: &nwg::Label,
    global_status: &nwg::Label,
) {
    let (child_slot, monitor_slot): (&mut Option<Child>, &mut Option<monitor::MonitorHandle>) =
        match profile {
            GameProfile::Game1 => (&mut state.game1, &mut state.game1_monitor),
            GameProfile::Game2 => (&mut state.game2, &mut state.game2_monitor),
        };

    if let Some(monitor) = monitor_slot.as_ref() {
        monitor.stop();
    }

    if let Some(mut child) = child_slot.take() {
        logging::info(&format!("stopping {} Firefox PID {}", profile.label(), child.id()));
        match child.kill() {
            Ok(()) => {
                let _ = child.wait();
                profile_status.set_text("Stopped");
                global_status.set_text(&format!("{} Firefox closed.", profile.label()));
            }
            Err(error) => {
                logging::error(&format!("failed to stop {} Firefox PID {}: {}", profile.label(), child.id(), error));
                profile_status.set_text("Stop failed");
                global_status.set_text(&format!("{}: could not close Firefox: {}", profile.label(), error));
                *child_slot = Some(child);
                return;
            }
        }
    } else {
        profile_status.set_text("Stopped");
        global_status.set_text(&format!("{} is already stopped.", profile.label()));
    }

    *monitor_slot = None;
    button.set_text(&format!("Launch {}", profile.label()));
}

fn update_health_label(
    monitor: Option<&monitor::MonitorHandle>,
    label: &nwg::Label,
) {
    let Some(monitor) = monitor else {
        return;
    };

    let health = monitor.health();
    label.set_text(&format!(
        "{} · {}",
        health.state,
        health.summary()
    ));
}

fn open_logs(status: &nwg::Label) {
    let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
        status.set_text("Could not determine LOCALAPPDATA for logs");
        return;
    };

    let path = std::path::PathBuf::from(local_app_data)
        .join("Moth")
        .join("PokeIdle")
        .join("moth-controller.log");

    if let Some(parent) = path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            status.set_text(&format!("Could not create log folder: {error}"));
            return;
        }
    }

    if !path.exists() {
        let _ = std::fs::OpenOptions::new().create(true).append(true).open(&path);
    }

    match std::process::Command::new("notepad.exe").arg(&path).spawn() {
        Ok(_) => status.set_text(&format!("Opened log: {}", path.display())),
        Err(error) => status.set_text(&format!("Could not open log: {error}")),
    }
}

fn open_profiles_folder(status: &nwg::Label) {
    let profiles = match Config::profiles_dir() {
        Ok(path) => path,
        Err(error) => {
            status.set_text(&error);
            return;
        }
    };

    if let Err(error) = std::fs::create_dir_all(&profiles) {
        status.set_text(&format!("Could not create profiles folder: {error}"));
        return;
    }

    match std::process::Command::new("explorer.exe")
        .arg(&profiles)
        .spawn()
    {
        Ok(_) => status.set_text(&format!("Opened {}", profiles.display())),
        Err(error) => status.set_text(&format!("Could not open folder: {error}")),
    }
}
