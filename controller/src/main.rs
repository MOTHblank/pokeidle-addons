#![cfg_attr(windows, windows_subsystem = "windows")]

mod accounts;
mod app;
#[path = "../icon.rs"]
mod icon;
mod config;
mod kick;
mod native_kick;
mod native_kick_pool;
mod native_kick_login;
mod native_kick_hls;
mod native_kick_viewer;
mod native_kick_http;
mod firefox;
mod monitor;
mod native_auth;
mod native_background;
mod native_gameplay;
mod native_capture_config;
mod native_items;
mod native_visuals;
mod native_hunts;
mod native_hunt_navigation;
mod native_boss;
mod native_history;
mod native_automation;
mod native_pvp;
mod native_market;
mod native_atlas;
mod native_atlas_models;
mod native_restock;
mod native_tls;
mod logging;
mod diagnostics;
mod diagnostics_threads;
mod diagnostics_recording;
mod update;
mod preferences;
mod settings_backup;
mod startup;
mod alerts;
mod single_instance;

fn main() {
    // The downloaded replacement binary doubles as a tiny updater helper.
    // Handle that mode before touching single-instance state, TLS or user data.
    if update::maybe_run_update_helper() {
        return;
    }

    // A second launch only signals the original window. Do this before
    // initializing TLS, opening accounts or scheduling any background work.
    let instance = match single_instance::acquire() {
        Ok(single_instance::Launch::Primary(instance)) => instance,
        Ok(single_instance::Launch::Existing) => return,
        Err(error) => {
            single_instance::report_acquire_error(&error);
            return;
        }
    };

    // rustls 0.23 cannot infer a default when transitive dependencies enable
    // both ring and aws-lc-rs. Initialize ONE process-wide provider before
    // spawning cache or game TLS workers. No authentication is sent
    // if provider installation fails.
    if let Err(error) = native_tls::ensure() {
        eprintln!("Native TLS initialization failed: {error}");
        return;
    }
    // Windows release builds do not have an attached console. Without a
    // persistent panic hook, one unexpected panic during authentication
    // handoff looks like a silent whole-controller crash. Never include the
    // panic payload here: authentication data must not leak into logs.
    std::panic::set_hook(Box::new(|info| {
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("unnamed");
        let location = info.location()
            .map(|at| format!("{}:{}:{}", at.file(), at.line(), at.column()))
            .unwrap_or_else(|| "unknown location".to_owned());
        crate::logging::error(&format!(
            "Rust panic in thread {name} at {location} (payload redacted)"
        ));
    }));
    if let Err(error) = app::run(instance) {
        crate::logging::error(&format!("Controller UI exited with error: {error}"));
        eprintln!("moth-controller: {error}");
    } else {
        crate::logging::info("Controller UI exited normally");
    }
}
