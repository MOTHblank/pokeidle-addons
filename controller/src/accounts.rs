use crate::config::{Config, GameProfile};
use crate::firefox;
use native_windows_gui as nwg;

const TWITCH_LOGIN: &str = "https://www.twitch.tv/login";
const KICK_LOGIN: &str = "https://kick.com/";
const VIOLENTMONKEY: &str = "https://addons.mozilla.org/firefox/addon/violentmonkey/";

enum Action {
    Game(GameProfile),
    Twitch(GameProfile),
    Kick(GameProfile),
    Folder(GameProfile),
    Userscripts(GameProfile),
}

pub fn show() {
    if let Err(error) = build() {
        nwg::simple_message("Moth", &error);
    }
}

fn build() -> Result<(), String> {
    let mut window = nwg::Window::default();
    let mut title = nwg::Label::default();

    let mut g1: [nwg::Button; 5] = std::array::from_fn(|_| nwg::Button::default());
    let mut g2: [nwg::Button; 5] = std::array::from_fn(|_| nwg::Button::default());

    build_window(&mut window, &mut title)?;
    build_game_row(&window, &mut g1, GameProfile::Game1, "Game 1", 30)?;
    build_game_row(&window, &mut g2, GameProfile::Game2, "Game 2", 170)?;

    let mut close = nwg::Button::default();
    nwg::Button::builder()
        .text("Close")
        .position((470, 328))
        .size((120, 34))
        .parent(&window)
        .build(&mut close)
        .map_err(|e| format!("could not create Close button: {e}"))?;

    let actions = [
        (g1[0].handle, Action::Game(GameProfile::Game1)),
        (g1[1].handle, Action::Twitch(GameProfile::Game1)),
        (g1[2].handle, Action::Kick(GameProfile::Game1)),
        (g1[3].handle, Action::Folder(GameProfile::Game1)),
        (g1[4].handle, Action::Userscripts(GameProfile::Game1)),
        (g2[0].handle, Action::Game(GameProfile::Game2)),
        (g2[1].handle, Action::Twitch(GameProfile::Game2)),
        (g2[2].handle, Action::Kick(GameProfile::Game2)),
        (g2[3].handle, Action::Folder(GameProfile::Game2)),
        (g2[4].handle, Action::Userscripts(GameProfile::Game2)),
    ];

    let window_handle = window.handle;
    let close_handle = close.handle;

    let event_handler = nwg::full_bind_event_handler(
        &window.handle,
        move |event, _, handle| {
            use nwg::Event;

            match event {
                Event::OnWindowClose if handle == window_handle => {
                    nwg::stop_thread_dispatch();
                }
                Event::OnButtonClick if handle == close_handle => {
                    nwg::stop_thread_dispatch();
                }
                Event::OnButtonClick => {
                    for (button, action) in &actions {
                        if handle != *button {
                            continue;
                        }

                        let result = match action {
                            Action::Game(profile) => open_game(*profile),
                            Action::Twitch(profile) => open_login(*profile, TWITCH_LOGIN, "Twitch"),
                            Action::Kick(profile) => open_login(*profile, KICK_LOGIN, "KICK"),
                            Action::Folder(profile) => open_profile_folder(*profile),
                            Action::Userscripts(profile) => open_login(*profile, VIOLENTMONKEY, "official Violentmonkey"),
                        };

                        if let Err(error) = result {
                            nwg::simple_message("Moth", &error);
                        }
                        break;
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

fn build_window(window: &mut nwg::Window, title: &mut nwg::Label) -> Result<(), String> {
    nwg::Window::builder()
        .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
        .size((620, 390))
        .position((560, 260))
        .title("Moth · Accounts")
        .build(window)
        .map_err(|e| format!("could not create account manager: {e}"))?;

    nwg::Label::builder()
        .text("Account Manager")
        .position((24, 18))
        .size((560, 32))
        .parent(window)
        .build(title)
        .map_err(|e| format!("could not create account manager title: {e}"))
}

fn build_game_row(
    window: &nwg::Window,
    buttons: &mut [nwg::Button; 5],
    _profile: GameProfile,
    title: &str,
    y: i32,
) -> Result<(), String> {
    let mut frame = nwg::Frame::default();

    nwg::Frame::builder()
        .flags(nwg::FrameFlags::BORDER)
        .position((24, y + 20))
        .size((572, 132))
        .parent(window)
        .build(&mut frame)
        .map_err(|e| format!("could not create {title} box: {e}"))?;

    let mut heading = nwg::Label::default();
    nwg::Label::builder()
        .text(title)
        .position((18, 4))
        .size((180, 24))
        .parent(&frame)
        .build(&mut heading)
        .map_err(|e| format!("could not create {title} heading: {e}"))?;

    let mut description = nwg::Label::default();
    nwg::Label::builder()
        .text("PokéIdle + one Twitch login + one KICK login share this profile.")
        .position((18, 30))
        .size((530, 26))
        .parent(&frame)
        .build(&mut description)
        .map_err(|e| format!("could not create {title} description: {e}"))?;

    let labels = ["Open Game", "Twitch Login", "KICK Login", "Profile Folder", "Violentmonkey"];
    for (index, label) in labels.into_iter().enumerate() {
        nwg::Button::builder()
            .text(label)
            .position((18 + (index as i32) * 108, 72))
            .size((100, 32))
            .parent(&frame)
            .build(&mut buttons[index])
            .map_err(|e| format!("could not create {title} {label} button: {e}"))?;
    }

    Ok(())
}

fn open_game(profile: GameProfile) -> Result<(), String> {
    let config = Config::for_profile(profile)?;
    let _ = firefox::launch(&config)?;
    Ok(())
}

fn open_login(profile: GameProfile, url: &str, service: &str) -> Result<(), String> {
    let config = Config::for_profile(profile)?;
    firefox::open_url(&config, url)?;

    nwg::simple_message(
        "Moth",
        &format!(
            "{service} opened in {}. Log in normally; its session stays in this game profile.",
            profile.label()
        ),
    );

    Ok(())
}

fn open_profile_folder(profile: GameProfile) -> Result<(), String> {
    let config = Config::for_profile(profile)?;
    std::fs::create_dir_all(&config.profile_dir)
        .map_err(|e| format!("could not create profile directory: {e}"))?;

    std::process::Command::new("explorer.exe")
        .arg(&config.profile_dir)
        .spawn()
        .map_err(|e| format!("could not open profile folder: {e}"))?;

    Ok(())
}
