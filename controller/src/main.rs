#![cfg_attr(windows, windows_subsystem = "windows")]

mod accounts;
mod app;
mod config;
mod firefox;
mod monitor;
mod logging;

fn main() {
    if let Err(error) = app::run() {
        eprintln!("moth-controller: {error}");
    }
}
