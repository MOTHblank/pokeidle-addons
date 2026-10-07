use crate::config::{Config, GameProfile};
use crate::firefox;
use native_windows_gui as nwg;
use std::rc::Rc;

const TWITCH_LOGIN: &str = "https://www.twitch.tv/login";
const KICK_LOGIN: &str = "https://kick.com/";

const ADDONS: &[(&str, &str)] = &[
    (
        "Auto Catch+",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/autocatch.user.js",
    ),
    (
        "Hunt Atlas",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/hunt-atlas.user.js",
    ),
    (
        "Moth Watch",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/market-bot.user.js",
    ),
    (
        "Performance+",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/performance.user.js",
    ),
    (
        "Upstream Scraper",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/scraper-exporter.user.js",
    ),
    (
        "Live Stream Scanner",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/stream-auto-open.user.js?v=5.3.1",
    ),
    (
        "Twitch Low Resource",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/twitch-low-resource.user.js",
    ),
];

enum Action {
    Game(GameProfile),
    Twitch(GameProfile),
    Kick(GameProfile),
    Addons(GameProfile),
    Folder(GameProfile),
}

pub struct AccountsWindow {
    window: Rc<nwg::Window>,
    title: nwg::Label,
    headings: [nwg::Label; 2],
    descriptions: [nwg::Label; 2],
    details: [nwg::Label; 2],
    game1_buttons: [nwg::Button; 5],
    game2_buttons: [nwg::Button; 5],
    close: nwg::Button,
    handler: nwg::EventHandler,
}

impl AccountsWindow {
    pub fn build() -> Result<Self, String> {
        let mut window = nwg::Window::default();
        nwg::Window::builder()
            .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
            .size((700, 500))
            .position((560, 260))
            .title("Moth · Profiles")
            .build(&mut window)
            .map_err(|e| format!("could not create account manager: {e}"))?;

        let window = Rc::new(window);

        let mut title = nwg::Label::default();
        nwg::Label::builder()
            .text("Profile Dashboard")
            .flags(nwg::LabelFlags::VISIBLE)
            .position((24, 18))
            .size((640, 32))
            .parent(&*window)
            .build(&mut title)
            .map_err(|e| format!("could not create account manager title: {e}"))?;

        let mut headings: [nwg::Label; 2] =
            std::array::from_fn(|_| nwg::Label::default());
        let mut descriptions: [nwg::Label; 2] =
            std::array::from_fn(|_| nwg::Label::default());
        let mut details: [nwg::Label; 2] =
            std::array::from_fn(|_| nwg::Label::default());

        let mut game1_buttons: [nwg::Button; 5] =
            std::array::from_fn(|_| nwg::Button::default());
        let mut game2_buttons: [nwg::Button; 5] =
            std::array::from_fn(|_| nwg::Button::default());

        build_game_row(
            &window,
            &mut headings[0],
            &mut descriptions[0],
            &mut details[0],
            &mut game1_buttons,
            "Game 1",
            62,
        )?;

        build_game_row(
            &window,
            &mut headings[1],
            &mut descriptions[1],
            &mut details[1],
            &mut game2_buttons,
            "Game 2",
            228,
        )?;

        let mut close = nwg::Button::default();
        nwg::Button::builder()
            .text("Close")
            .flags(nwg::ButtonFlags::VISIBLE)
            .position((550, 445))
            .size((120, 34))
            .parent(&*window)
            .build(&mut close)
            .map_err(|e| format!("could not create Close button: {e}"))?;

        let actions = [
            (game1_buttons[0].handle, Action::Game(GameProfile::Game1)),
            (game1_buttons[1].handle, Action::Twitch(GameProfile::Game1)),
            (game1_buttons[2].handle, Action::Kick(GameProfile::Game1)),
            (game1_buttons[3].handle, Action::Addons(GameProfile::Game1)),
            (game1_buttons[4].handle, Action::Folder(GameProfile::Game1)),
            (game2_buttons[0].handle, Action::Game(GameProfile::Game2)),
            (game2_buttons[1].handle, Action::Twitch(GameProfile::Game2)),
            (game2_buttons[2].handle, Action::Kick(GameProfile::Game2)),
            (game2_buttons[3].handle, Action::Addons(GameProfile::Game2)),
            (game2_buttons[4].handle, Action::Folder(GameProfile::Game2)),
        ];

        let events_window = Rc::clone(&window);
        let close_handle = close.handle;

        let handler = nwg::full_bind_event_handler(
            &window.handle,
            move |event, _, handle| {
                use nwg::Event;

                match event {
                    Event::OnWindowClose if handle == events_window.handle => {
                        events_window.set_visible(false);
                    }
                    Event::OnButtonClick if handle == close_handle => {
                        events_window.set_visible(false);
                    }
                    Event::OnButtonClick => {
                        for (button, action) in &actions {
                            if handle != *button {
                                continue;
                            }

                            let result = match action {
                                Action::Game(profile) => open_game(*profile),
                                Action::Twitch(profile) => {
                                    open_login(*profile, TWITCH_LOGIN, "Twitch")
                                }
                                Action::Kick(profile) => {
                                    open_login(*profile, KICK_LOGIN, "KICK")
                                }
                                Action::Addons(profile) => open_addons(*profile),
                                Action::Folder(profile) => open_profile_folder(*profile),
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

        Ok(Self {
            window,
            title,
            headings,
            descriptions,
            details,
            game1_buttons,
            game2_buttons,
            close,
            handler,
        })
    }

    pub fn show(&self) {
        self.window.set_visible(true);
        self.window.set_focus();
    }
}

impl Drop for AccountsWindow {
    fn drop(&mut self) {
        nwg::unbind_event_handler(&self.handler);
    }
}

fn build_game_row(
    window: &Rc<nwg::Window>,
    heading: &mut nwg::Label,
    description: &mut nwg::Label,
    details: &mut nwg::Label,
    buttons: &mut [nwg::Button; 5],
    title: &str,
    y: i32,
) -> Result<(), String> {
    nwg::Label::builder()
        .text(title)
        .flags(nwg::LabelFlags::VISIBLE)
        .position((24, y))
        .size((180, 24))
        .parent(&**window)
        .build(heading)
        .map_err(|e| format!("could not create {title} heading: {e}"))?;

    nwg::Label::builder()
        .text("PokéIdle + Twitch + KICK + Violentmonkey use this same profile.")
        .flags(nwg::LabelFlags::VISIBLE)
        .position((24, y + 28))
        .size((650, 22))
        .parent(&**window)
        .build(description)
        .map_err(|e| format!("could not create {title} description: {e}"))?;

    let profile_text = match Config::for_profile(match title {
        "Game 1" => GameProfile::Game1,
        _ => GameProfile::Game2,
    }) {
        Ok(config) => {
            let state = if config.profile_dir.exists() {
                "Profile ready"
            } else {
                "Profile will be created on first launch"
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

            format!(
                "{} · {} · {}",
                state,
                browser,
                config.profile_dir.display()
            )
        }
        Err(error) => format!("Unavailable · {error}"),
    };

    nwg::Label::builder()
        .text(&profile_text)
        .flags(nwg::LabelFlags::VISIBLE)
        .position((24, y + 50))
        .size((650, 22))
        .parent(&**window)
        .build(details)
        .map_err(|e| format!("could not create {title} details: {e}"))?;

    let labels = [
        "Open Game",
        "Twitch",
        "KICK",
        "Addons",
        "Profile",
    ];

    for (index, label) in labels.into_iter().enumerate() {
        let row = if index < 3 { 0 } else { 1 };
        let column = if index < 3 { index } else { index - 3 };

        nwg::Button::builder()
            .text(label)
            .flags(nwg::ButtonFlags::VISIBLE)
            .position((24 + (column as i32) * 170, y + 78 + row * 38))
            .size((158, 32))
            .parent(&**window)
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

fn open_addons(profile: GameProfile) -> Result<(), String> {
    let config = Config::for_profile(profile)?;

    let mut opened = 0usize;
    let mut failures = Vec::new();

    for &(name, url) in ADDONS {
        match firefox::open_url(&config, url) {
            Ok(()) => opened += 1,
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }

    if failures.is_empty() {
        nwg::simple_message(
            "Moth",
            &format!(
                "Opened {opened} addon installers in {}. Violentmonkey will show the normal install/update page for each script.",
                profile.label()
            ),
        );
        return Ok(());
    }

    Err(format!(
        "Opened {opened} addon installers, but some failed:\n{}",
        failures.join("\n")
    ))
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
