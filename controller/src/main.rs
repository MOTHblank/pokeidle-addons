#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod config;
mod firefox;

fn main() {
    if let Err(error) = app::run() {
        eprintln!("moth-controller: {error}");
    }
}
