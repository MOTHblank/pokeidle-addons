use crate::config::{Config, GameProfile};
use crate::accounts::show as show_accounts;
use crate::firefox;
use native_windows_gui as nwg;
use std::cell::RefCell;
use std::process::Child;
use std::rc::Rc;

struct State {
    game1: Option<Child>,
    game2: Option<Child>,
}

pub fn run() -> Result<(), String> {
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
    let mut profiles_button = nwg::Button::default();
    let mut close_button = nwg::Button::default();
    let mut status = nwg::Label::default();

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
        .text("2 game profiles · shared Twitch/KICK logins · low overhead")
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
    }));

    {
        let mut state = state.borrow_mut();

        launch_one(
            GameProfile::Game1,
            &mut state.game1,
            &game1_status,
            &status,
        );
        launch_one(
            GameProfile::Game2,
            &mut state.game2,
            &game2_status,
            &status,
        );
    }

    let game1_button_handle = game1_button.handle;
    let game2_button_handle = game2_button.handle;
    let launch_both_button_handle = launch_both_button.handle;
    let accounts_button_handle = accounts_button.handle;
    let profiles_button_handle = profiles_button.handle;
    let close_button_handle = close_button.handle;
    let window_handle = window.handle;

    let state_for_events = state.clone();

    let event_handler = nwg::full_bind_event_handler(
        &window.handle,
        move |event, _data, handle| {
            use nwg::Event;

            match event {
                Event::OnWindowClose => {
                    if handle == window_handle {
                        nwg::stop_thread_dispatch();
                    }
                }
                Event::OnButtonClick => {
                    if handle == game1_button_handle {
                        let mut state = state_for_events.borrow_mut();
                        launch_one(
                            GameProfile::Game1,
                            &mut state.game1,
                            &game1_status,
                            &status,
                        );
                    } else if handle == game2_button_handle {
                        let mut state = state_for_events.borrow_mut();
                        launch_one(
                            GameProfile::Game2,
                            &mut state.game2,
                            &game2_status,
                            &status,
                        );
                    } else if handle == launch_both_button_handle {
                        let mut state = state_for_events.borrow_mut();

                        launch_one(
                            GameProfile::Game1,
                            &mut state.game1,
                            &game1_status,
                            &status,
                        );
                        launch_one(
                            GameProfile::Game2,
                            &mut state.game2,
                            &game2_status,
                            &status,
                        );
                    } else if handle == accounts_button_handle {
                        show_accounts();
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

fn launch_one(
    profile: GameProfile,
    child_slot: &mut Option<Child>,
    profile_status: &nwg::Label,
    global_status: &nwg::Label,
) {
    if let Some(child) = child_slot.as_mut() {
        match child.try_wait() {
            Ok(None) => {
                profile_status.set_text("Running");
                global_status.set_text(&format!("{} is already running.", profile.label()));
                return;
            }
            Ok(Some(_)) | Err(_) => {
                *child_slot = None;
                profile_status.set_text("Stopped");
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

    match firefox::launch(&config) {
        Ok(child) => {
            *child_slot = Some(child);
            profile_status.set_text("Running");
            global_status.set_text(&format!(
                "Started {} · one Firefox process/profile",
                profile.label()
            ));
        }
        Err(error) => {
            profile_status.set_text("Launch failed");
            global_status.set_text(&format!("{}: {error}", profile.label()));
        }
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
