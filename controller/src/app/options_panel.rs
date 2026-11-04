//! Application-wide Options page. Never starts a monitor, game socket, or
//! diagnostics sampler just to display a setting.
use super::*;

fn card(ui: &mut egui::Ui, title: &str, description: &str,
    content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(egui::Margin::same(15)).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(tr(title)).size(13.0).strong().color(TEXT));
            if !description.is_empty() {
                ui.label(RichText::new(tr(description)).size(11.0).color(MUTED));
            }
            ui.add_space(9.0);
            content(ui);
        });
}

fn description(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(tr(text)).size(10.0).color(DIM));
}

pub(super) fn draw_options_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        for (section, label) in [
            (OptionsSection::General, "General"),
            (OptionsSection::Appearance, "Appearance"),
            (OptionsSection::Performance, "Performance"),
            (OptionsSection::Notifications, "Notifications"),
            (OptionsSection::Updates, "Updates"),
            (OptionsSection::Data, "Data & logs"),
        ] {
            if workspace_tab_button(ui, label, app.workspace.options_section == section, label).clicked() {
                app.workspace.options_section = section;
            }
        }
    });
    ui.add_space(8.0);
    ui.separator();

    egui::ScrollArea::vertical().id_salt("options-page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let width = ui.available_width().min(800.0);
            ui.allocate_ui_with_layout(egui::vec2(width, 0.0),
                Layout::top_down(Align::Min), |ui| {
                    match app.workspace.options_section {
                        OptionsSection::General => general(app, ui),
                        OptionsSection::Appearance => appearance(app, ui),
                        OptionsSection::Performance => performance(app, ui),
                        OptionsSection::Notifications => notifications(app, ui),
                        OptionsSection::Updates => updates(app, ui),
                        OptionsSection::Data => data_and_logs(app, ui),
                    }
                    show_save_errors(app, ui);
                    ui.add_space(8.0);
                    description(ui, "Settings are applied and saved when changed.");
                });
        });
}

fn general(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let mut next = app.preferences;
    card(ui, "Language", "Choose the interface language.", |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut next.language_pt_br, false, "English");
            ui.selectable_value(&mut next.language_pt_br, true, "Português (Brasil)");
        });
    });
    ui.add_space(10.0);
    card(ui, "Startup", "All startup actions are optional and off by default.", |ui| {
        ui.checkbox(&mut next.launch_enabled_on_start, tr("Launch enabled accounts when Moth starts"));
        description(ui, "Only enabled accounts will launch; sign-in requirements still apply.");
        #[cfg(windows)]
        {
            ui.add_space(9.0);
            ui.checkbox(&mut next.start_with_windows, tr("Start Moth with Windows"));
            description(ui, "Registers Moth for this Windows user only. Disable to remove the registration.");
            ui.add_space(9.0);
            ui.add_enabled(app.tray.is_some(), egui::Checkbox::new(
                &mut next.start_minimized, tr("Start hidden in the system tray")));
        }
    });
    ui.add_space(10.0);
    card(ui, "Window and exit behavior", "", |ui| {
        description(ui, "Opening Moth again restores this window instead of running another controller.");
        ui.add_space(8.0);
        ui.checkbox(&mut next.confirm_exit_with_accounts, tr("Confirm exit while accounts are active"));
        description(ui, "Exiting stops Moth's active sessions; the confirmation is shown only when needed.");
        #[cfg(windows)]
        {
            ui.add_space(9.0);
            ui.add_enabled(app.tray.is_some(), egui::Checkbox::new(
                &mut next.close_to_tray, tr("Close button hides Moth in the system tray")));
            description(ui, "Use the tray Exit command to quit when close-to-tray is enabled.");
        }
    });
    if next != app.preferences { app.save_app_preferences(next); }
}

fn appearance(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let mut next = app.preferences;
    card(ui, "Display", "Moth uses its existing dark theme.", |ui| {
        egui::ComboBox::from_label(tr("Interface scale"))
            .selected_text(format!("{}%", next.ui_scale_percent))
            .show_ui(ui, |ui| {
                for scale in [85, 100, 110, 125, 150] {
                    ui.selectable_value(&mut next.ui_scale_percent, scale, format!("{scale}%"));
                }
            });
        description(ui, "Increase scaling for readability, or decrease it to fit more controls.");
        ui.add_space(12.0);
        ui.checkbox(&mut next.compact_spacing, tr("Compact spacing"));
        description(ui, "Reduces padding around controls across the interface.");
        ui.add_space(12.0);
        ui.checkbox(&mut next.reduce_motion, tr("Reduce interface animation"));
        description(ui, "Disables standard egui transition animations.");
    });
    if next != app.preferences { app.save_app_preferences(next); }
}

fn default_performance_options(current: UiOptions) -> UiOptions {
    UiOptions {
        reduce_updates_unfocused: UiOptions::default().reduce_updates_unfocused,
        reduce_updates_in_tray: UiOptions::default().reduce_updates_in_tray,
        ..current
    }
}

fn performance(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let mut next = app.ui_options;
    card(ui, "Background performance",
        "Only window redraws and Firefox monitoring are reduced; automation keeps running.", |ui| {
            ui.checkbox(&mut next.reduce_updates_unfocused,
                tr("Reduce updates when the window is not focused"));
            #[cfg(windows)]
            {
                ui.add_space(9.0);
                ui.checkbox(&mut next.reduce_updates_in_tray,
                    tr("Reduce updates when minimized to the system tray"));
            }
            ui.add_space(10.0);
            description(ui, "PokéIdle and automations run independently of display refresh.");
        });
    if next != app.ui_options {
        app.save_ui_options(next);
        ui.ctx().request_repaint();
    }
    ui.add_space(12.0);
    ui.label(format!("{}: {}", tr("Current refresh mode"),
        tr(if app.controller_in_background(ui.ctx()) {
            "Reduced background updates"
        } else { "Normal updates" })));
    ui.horizontal_wrapped(|ui| {
        if ui.small_button(tr("Restore default performance settings")).clicked() {
            app.save_ui_options(default_performance_options(app.ui_options));
        }
        if ui.small_button(tr("Open performance diagnostics")).clicked() {
            app.workspace.panel_tab = PanelTab::Diagnostics;
        }
    });
    description(ui, "Opening Diagnostics does not start sampling. Enable live sampling there; leaving the page turns it off. Explicit recordings continue until stopped.");
}

fn notifications(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let mut next = app.preferences;
    card(ui, "Desktop notifications",
        "Optional, rate-limited system alerts. No account names or server messages are included.", |ui| {
            ui.checkbox(&mut next.notifications_enabled, tr("Enable desktop notifications"));
            ui.add_enabled_ui(next.notifications_enabled, |ui| {
                ui.add_space(10.0);
                ui.checkbox(&mut next.notify_auth, tr("Sign-in required"));
                ui.checkbox(&mut next.notify_disconnect, tr("Prolonged disconnections (90 seconds)"));
                ui.checkbox(&mut next.notify_automation, tr("Automation failures"));
                ui.checkbox(&mut next.notify_boss, tr("Boss farming completed"));
                ui.separator();
                ui.checkbox(&mut next.quiet_notifications, tr("Quiet mode (suppress alerts)"));
            });
        });
    if next != app.preferences { app.save_app_preferences(next); }
    ui.add_space(10.0);
    if ui.add_enabled(app.preferences.notifications_enabled &&
        !app.preferences.quiet_notifications,
        egui::Button::new(tr("Send test notification"))).clicked() {
        alerts::test_notification(app.preferences);
    }
    description(ui, "Desktop notifications require OS notification support; failures are logged.");
}

fn updates(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let mut next = app.preferences;
    card(ui, "Application updates",
        "Moth can check GitHub and download verified updates without opening a browser.", |ui| {
            ui.checkbox(&mut next.check_updates_on_start, tr("Check for new versions on startup"));
        });
    if next != app.preferences { app.save_app_preferences(next); }

    ui.add_space(11.0);
    let (latest, notes_url, checking, downloading, staged_version, failure) =
        match app.update_status.lock() {
            Ok(status) => (
                status.latest_version.clone(),
                status.release_notes_url.clone(),
                status.checking,
                status.downloading,
                status.staged_version.clone(),
                status.last_error.clone(),
            ),
            Err(_) => (None, None, false, false, None, Some("Update status unavailable".into())),
        };

    ui.label(format!("{}: v{}", tr("Installed version"), env!("CARGO_PKG_VERSION")));
    if let Some(latest) = latest.as_ref() {
        ui.label(format!("{}: {}", tr("Latest release"), latest));
    }
    if let Some(error) = failure {
        ui.label(RichText::new(error).size(10.0).color(WARN));
    }

    let newer = latest.as_deref()
        .is_some_and(|version| update::is_newer_version(version, env!("CARGO_PKG_VERSION")));
    if staged_version.is_none() && latest.is_some() && !newer {
        ui.label(RichText::new(tr("Moth Controller is up to date.")).size(10.0).color(GOOD));
    }
    if let Some(version) = staged_version.as_ref() {
        ui.label(RichText::new(format!("{}: {version}", tr("Ready to install")))
            .size(11.0).strong().color(GOOD));
    }

    ui.horizontal_wrapped(|ui| {
        if ui.add_enabled(!checking && !downloading, egui::Button::new(
            tr(if checking { "Checking for updates..." } else { "Check now" }))).clicked() {
            match update::check_now(&app.update_status) {
                Ok(_) => app.set_status(tr("Update check started"), false),
                Err(error) => app.set_status(error, true),
            }
            ui.ctx().request_repaint();
        }
        let notes = notes_url.as_deref().unwrap_or(update::RELEASES_URL);
        ui.hyperlink_to(tr("Release notes"), notes);

        #[cfg(windows)]
        if newer && staged_version.is_none() {
            if ui.add_enabled(!checking && !downloading,
                egui::Button::new(tr(if downloading {
                    "Downloading update..."
                } else {
                    "Download update"
                }))).clicked() {
                match update::download_update(&app.update_status) {
                    Ok(_) => app.set_status(tr("Update download started"), false),
                    Err(error) => app.set_status(error, true),
                }
                ui.ctx().request_repaint();
            }
        }
    });

    #[cfg(windows)]
    if staged_version.is_some() {
        ui.add_space(10.0);
        if ui.button(tr("Restart and install update")).clicked() {
            match update::request_restart_and_install(&app.update_status) {
                Ok(_) => {
                    app.set_status(tr("Restarting to install update"), false);
                    // Update-triggered shutdown is explicit: bypass close-to-tray
                    // and the active-account confirmation so installation can proceed.
                    app.workspace.exit_confirmation = false;
                    app.workspace.exit_approved = true;
                    app.tray_exit_requested.store(true, Ordering::Release);
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Err(error) => app.set_status(error, true),
            }
        }
        description(ui,
            "The verified update closes Moth, stops active accounts, replaces the application executable, then reopens Moth.");
    } else if newer {
        description(ui,
            "Downloading never interrupts active accounts. Installation only starts after you choose to restart.");
    } else {
        description(ui,
            "Automatic checks do not download or install anything unless you choose to.");
    }

    #[cfg(not(windows))]
    if newer {
        description(ui,
            "In-app installation is currently available on Windows; use Release notes to download the Linux package.");
    }
}

fn open_data_folder() -> Result<(), String> {
    let path = crate::config::data_root()?;
    std::fs::create_dir_all(&path)
        .map_err(|error| format!("could not create data folder: {error}"))?;
    let opener = if cfg!(windows) { "explorer.exe" } else { "xdg-open" };
    std::process::Command::new(opener).arg(&path).spawn()
        .map_err(|error| format!("could not open data folder: {error}"))?;
    Ok(())
}

fn export_support_report(app: &ControllerApp) -> Result<std::path::PathBuf, String> {
    // On-demand metadata only: no raw logs, player identifiers, errors, URLs,
    // browser secrets, session payloads, or diagnostics sampling.
    let path = crate::config::data_root()?.join("moth-support-report.json");
    let running_sessions = app.runtimes.iter().filter(|runtime|
        runtime.game.native.is_some()).count();
    let firefox_processes = app.runtimes.iter().filter(|runtime|
        runtime.game.child.is_some() || runtime.game.orphaned_pid.is_some()).count();
    let kick_accounts = app.runtimes.iter().filter(|runtime|
        runtime.kick.kick_monitor.is_some()).count();
    let report = serde_json::json!({
        "format": "MothControllerSafeSupportReport",
        "version": 1,
        "controllerVersion": env!("CARGO_PKG_VERSION"),
        "operatingSystem": std::env::consts::OS,
        "activeGameSessions": running_sessions,
        "firefoxProcesses": firefox_processes,
        "kickAccountMonitors": kick_accounts,
        "diagnosticsCurrentlyCollecting": app.diagnostics_enabled &&
            app.diagnostics.as_ref().is_some_and(|sampler| !sampler.paused),
        "developerRecordingActive": app.recording.as_ref()
            .is_some_and(RecordingSession::is_running),
        "backgroundRefreshUnfocused": app.ui_options.reduce_updates_unfocused,
        "backgroundRefreshTray": app.ui_options.reduce_updates_in_tray,
    });
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("could not create report folder: {error}"))?;
    }
    let raw = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("could not serialize support report: {error}"))?;
    let staging = path.with_extension("json.tmp");
    std::fs::write(&staging, raw)
        .map_err(|error| format!("could not write support report: {error}"))?;
    std::fs::rename(&staging, &path)
        .map_err(|error| format!("could not finish support report: {error}"))?;
    Ok(path)
}

fn data_and_logs(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let mut next = app.preferences;
    card(ui, "Log management",
        "Rotate controller logs at a size limit and expire the rotated archive.", |ui| {
            egui::ComboBox::from_label(tr("Log file limit"))
                .selected_text(format!("{} MiB", next.log_max_megabytes))
                .show_ui(ui, |ui| {
                    for value in [4, 8, 16, 32, 64, 128] {
                        ui.selectable_value(&mut next.log_max_megabytes, value,
                            format!("{value} MiB"));
                    }
                });
            egui::ComboBox::from_label(tr("Rotated log retention"))
                .selected_text(format!("{} {}", next.log_retention_days, tr("days")))
                .show_ui(ui, |ui| {
                    for days in [1, 7, 14, 30, 60, 90] {
                        ui.selectable_value(&mut next.log_retention_days, days,
                            format!("{days} {}", tr("days")));
                    }
                });
            description(ui, "Active logs are preserved; only the rotated archive is eligible for expiry.");
        });
    if next != app.preferences { app.save_app_preferences(next); }
    ui.horizontal_wrapped(|ui| {
        if ui.small_button(tr("Open log")).clicked() { app.open_logs(); }
        if ui.small_button(tr("Open data folder")).clicked() {
            match open_data_folder() {
                Ok(()) => app.set_status(tr("Opened data folder"), false),
                Err(error) => app.set_status(error, true),
            }
        }
        if ui.small_button(tr("Export support report")).clicked() {
            match export_support_report(app) {
                Ok(path) => app.set_status(format!("{}: {}",
                    tr("Support report saved"), path.display()), false),
                Err(error) => app.set_status(error, true),
            }
        }
    });
    description(ui, "Support reports contain aggregate runtime counts only, never private session details.");
    ui.add_space(12.0);
    card(ui, "Settings backup",
        "Exports local app, account, and Kick stream preferences. Game server automation and credentials remain excluded.", |ui| {
            if ui.button(tr("Export safe settings backup")).clicked() {
                let backup = SettingsBackup::from_active(
                    app.preferences, app.ui_options, &app.accounts).with_saved_kick();
                match backup.and_then(|settings| settings.export()) {
                    Ok(path) => app.set_status(format!("{}: {}",
                        tr("Settings backup saved"), path.display()), false),
                    Err(error) => app.set_status(error, true),
                }
            }
            ui.add_space(9.0);
            if ui.button(tr("Preview restore from backup")).clicked() {
                match SettingsBackup::load_preview() {
                    Ok(preview) => {
                        app.workspace.backup_preview = Some(preview);
                        app.set_status(tr("Settings backup preview loaded"), false);
                    }
                    Err(error) => {
                        app.workspace.backup_preview = None;
                        app.set_status(error, true);
                    }
                }
            }
            description(ui, "Uses moth-settings-backup.json in Moth's data folder. Export replaces that file.");
        });
    if let Some(backup) = app.workspace.backup_preview.clone() {
        ui.add_space(12.0);
        card(ui, "Restore preview", "Review before replacing local settings:", |ui| {
            for (index, account) in backup.accounts.iter().enumerate() {
                ui.label(format!("{} {}: {} ({})", tr("Account"), index + 1,
                    account.name, tr(if account.enabled { "Enabled" } else { "Disabled" })));
            }
            ui.separator();
            if let Some(kick) = backup.kick.as_ref() {
                ui.label(RichText::new(tr("Kick stream preferences")).strong());
                for (index, (channels, preferences)) in kick.channels.iter()
                    .zip(kick.preferences.iter()).enumerate() {
                    ui.label(format!("{} {} · {}: {} · {}: {} · {}: {}",
                        tr("Account"), index + 1,
                        tr("Channels"), channels,
                        tr("Maximum streams"), preferences.max_streams,
                        tr("Favorites"), preferences.favorites.join(", ")));
                }
            }
            ui.label(format!("{}: {}%", tr("Interface scale"), backup.app.ui_scale_percent));
            let aoe_slots = backup.ui.atlas_aoe_enabled.iter().enumerate()
                .filter(|(_, enabled)| **enabled)
                .map(|(index, _)| (index + 1).to_string())
                .collect::<Vec<_>>();
            ui.label(format!("{}: {}", tr("Hunt Atlas AoE enabled for slots"),
                if aoe_slots.is_empty() { "—".into() } else { aoe_slots.join(", ") }));
            ui.label(format!("{}: {}", tr("Language"),
                if backup.app.language_pt_br { "Português (Brasil)" } else { "English" }));
            ui.label(format!("{}: {}", tr("Launch enabled accounts when Moth starts"),
                tr(if backup.app.launch_enabled_on_start { "Enabled" } else { "Disabled" })));
            #[cfg(windows)]
            {
                ui.label(format!("{}: {}", tr("Start Moth with Windows"),
                    tr(if backup.app.start_with_windows { "Enabled" } else { "Disabled" })));
                ui.label(format!("{}: {}", tr("Close button hides Moth in the system tray"),
                    tr(if backup.app.close_to_tray { "Enabled" } else { "Disabled" })));
            }
            ui.label(format!("{}: {}", tr("Enable desktop notifications"),
                tr(if backup.app.notifications_enabled { "Enabled" } else { "Disabled" })));
            ui.label(format!("{}: {}", tr("Check for new versions on startup"),
                tr(if backup.app.check_updates_on_start { "Enabled" } else { "Disabled" })));
            description(ui, "A restart may be required for startup behavior to take effect.");
            if app.accounts_have_live_work() {
                ui.label(RichText::new(tr("Stop all accounts before restoring settings"))
                    .size(10.0).color(WARN));
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(!app.accounts_have_live_work(),
                    egui::Button::new(tr("Confirm restore"))).clicked() {
                    match app.restore_settings_backup(backup.clone()) {
                        Ok(()) => app.set_status(tr("Settings restored"), false),
                        Err(error) => app.set_status(error, true),
                    }
                }
                if ui.small_button(tr("Cancel preview")).clicked() {
                    app.workspace.backup_preview = None;
                }
            });
        });
    }
}

fn show_save_errors(app: &mut ControllerApp, ui: &mut egui::Ui) {
    if let Some((pending, error)) = app.workspace.preferences_failed_save.clone() {
        ui.add_space(10.0);
        card(ui, "Could not save application options",
            "Previous settings remain active. Retry when the storage problem is resolved.", |ui| {
                ui.label(RichText::new(error).size(10.0).color(BAD));
                if ui.small_button(tr("Retry saving options")).clicked() {
                    app.save_app_preferences(pending);
                }
            });
    }
    if let Some((pending, error)) = app.workspace.options_failed_save.clone() {
        ui.add_space(10.0);
        card(ui, "Could not save options",
            "Previous settings remain active. Retry when the storage problem is resolved.", |ui| {
                ui.label(RichText::new(error).size(10.0).color(BAD));
                if ui.small_button(tr("Retry saving options")).clicked() {
                    app.save_ui_options(pending);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resetting_performance_does_not_change_per_account_atlas_or_app_preferences() {
        let current = UiOptions {
            reduce_updates_unfocused: false,
            reduce_updates_in_tray: false,
            atlas_aoe_enabled: [true, false, false, true],
        };
        let reset = default_performance_options(current);
        assert!(reset.reduce_updates_unfocused);
        assert!(reset.reduce_updates_in_tray);
        assert_eq!(reset.atlas_aoe_enabled, current.atlas_aoe_enabled);
    }
}
