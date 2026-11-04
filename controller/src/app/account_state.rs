//! Per-profile presentation preferences and configuration; no new session ownership.
use super::*;

impl ControllerApp {
    pub(super) fn game_index(profile: GameProfile) -> usize {
        profile.index()
    }

    pub(super) fn hidden_in_tray(&self) -> bool {
        #[cfg(windows)]
        { self.tray_hidden.load(Ordering::Relaxed) }
        #[cfg(not(windows))]
        { false }
    }

    pub(super) fn controller_in_background(&self, ctx: &egui::Context) -> bool {
        // Keep native background transport untouched; only UI repaint and
        // Firefox/BiDi monitoring pace respond to these preferences.
        let focused = ctx.input(|input| input.viewport().focused);
        reduce_updates_for_window(self.hidden_in_tray(), focused, self.ui_options)
    }

    pub(super) fn save_ui_options(&mut self, updated: UiOptions) {
        match Config::save_ui_options(&updated) {
            Ok(()) => {
                self.ui_options = updated;
                self.workspace.options_failed_save = None;
                self.set_status(tr("Controller options saved").to_owned(), false);
            }
            Err(error) => {
                self.workspace.options_failed_save = Some((updated, error.clone()));
                self.set_status(error, true);
            },
        }
    }

    fn try_save_app_preferences(&mut self, next: AppPreferences) -> Result<(), String> {
        if next == self.preferences { return Ok(()); }
        let before = self.preferences;
        if next.start_with_windows != before.start_with_windows {
            startup::set_start_with_windows(next.start_with_windows)?;
        }
        if let Err(error) = next.save() {
            if next.start_with_windows != before.start_with_windows {
                let _ = startup::set_start_with_windows(before.start_with_windows);
            }
            return Err(error);
        }
        self.preferences = next;
        PT_BR.store(next.language_pt_br, Ordering::Relaxed);
        apply_visual_preferences(&self.lifecycle_ctx, next);
        logging::set_limits(next.log_max_megabytes, next.log_retention_days);
        if next.check_updates_on_start && !before.check_updates_on_start {
            let _ = update::check_now(&self.update_status);
        }
        self.lifecycle_ctx.request_repaint();
        Ok(())
    }

    pub(super) fn save_app_preferences(&mut self, next: AppPreferences) {
        match self.try_save_app_preferences(next) {
            Ok(()) => {
                self.workspace.preferences_failed_save = None;
                self.set_status(tr("Application options saved").to_owned(), false);
            }
            Err(error) => {
                self.workspace.preferences_failed_save = Some((next, error.clone()));
                self.set_status(error, true);
            }
        }
    }

    pub(super) fn restore_settings_backup(&mut self, backup: SettingsBackup) -> Result<(), String> {
        if self.accounts_have_live_work() {
            return Err("Stop all accounts before restoring settings".into());
        }
        // Exclude non-portable Windows startup/tray state on Linux.
        #[cfg(not(windows))]
        let backup = {
            let mut backup = backup;
            backup.app.start_with_windows = false;
            backup.app.start_minimized = false;
            backup.app.close_to_tray = false;
            backup
        };
        let previous = SettingsBackup::from_active(
            self.preferences, self.ui_options, &self.accounts).with_saved_kick()?;
        if backup.app.start_with_windows != previous.app.start_with_windows {
            startup::set_start_with_windows(backup.app.start_with_windows)?;
        }
        let result = (|| {
            Config::save_accounts(&backup.accounts)?;
            Config::save_ui_options(&backup.ui)?;
            if let Some(kick) = backup.kick.as_ref() {
                Config::save_kick_channels(&kick.channels, &kick.preferences)?;
            }
            backup.app.save()?;
            Ok::<(), String>(())
        })();
        if let Err(error) = result {
            // Best effort rollback of all three persisted settings files.
            let _ = Config::save_accounts(&previous.accounts);
            let _ = Config::save_ui_options(&previous.ui);
            if let Some(kick) = previous.kick.as_ref() {
                let _ = Config::save_kick_channels(&kick.channels, &kick.preferences);
            }
            let _ = previous.app.save();
            if backup.app.start_with_windows != previous.app.start_with_windows {
                let _ = startup::set_start_with_windows(previous.app.start_with_windows);
            }
            return Err(format!("Could not restore settings (rollback attempted): {error}"));
        }
        // All four slots are offline, so replace the local Kick editor
        // state without racing live monitor workers or queued automation.
        if let Some(kick) = backup.kick.as_ref() {
            for (index, runtime) in self.runtimes.iter_mut().enumerate() {
                runtime.kick.kick_channels_input = kick.channels[index].clone();
                runtime.kick.kick_stream_preferences = kick.preferences[index].clone();
            }
        }
        self.accounts = backup.accounts;
        self.ui_options = backup.ui;
        self.preferences = backup.app;
        self.workspace.options_failed_save = None;
        self.workspace.preferences_failed_save = None;
        self.workspace.backup_preview = None;
        PT_BR.store(self.preferences.language_pt_br, Ordering::Relaxed);
        apply_visual_preferences(&self.lifecycle_ctx, self.preferences);
        logging::set_limits(self.preferences.log_max_megabytes, self.preferences.log_retention_days);
        self.refresh_account_setup();
        self.lifecycle_ctx.request_repaint();
        Ok(())
    }

    pub(super) fn sync_monitor_paces(&self, background: bool) {
        // Exactly one account may be expanded when focused. Background mode
        // overrides that selection without clearing the open details panel.
        for (index, slot) in self.runtimes.iter().map(|runtime| &runtime.game).enumerate() {
            let display_pace = monitor_pace_for_slot(
                self.workspace.selected_details, index, self.accounts[index].enabled, background
            );
            // Sign-in is control-plane work, not display telemetry. Keep the
            // BiDi authentication monitor responsive even while minimized.
            let awaiting_handoff = self.accounts[index].enabled &&
                slot.native.is_none() && !slot.browser_only &&
                !slot.stopping && !slot.shutdown_failed;
            let pace = if awaiting_handoff {
                MonitorPace::Detailed
            } else {
                display_pace
            };
            if let Some(monitor) = slot.monitor.as_ref() {
                monitor.set_pace(pace);
            }
        }
    }

    /// Switching accounts changes the workspace selection, not a live session.
    pub(super) fn select_workspace_account(&mut self, index: usize) {
        if index >= self.runtimes.len() || self.workspace.account_selection_matches(index) { return; }
        self.save_dirty_account_names();
        self.workspace.select_account(index);
    }

    /// Account edits are buffered until focus is lost, the selected account
    /// changes, or the user navigates away from Accounts.
    pub(super) fn save_dirty_account_names(&mut self) {
        for profile in GameProfile::ALL {
            let index = profile.index();
            if self.workspace.account_names_dirty[index] {
                let name = self.accounts[index].name.trim();
                self.accounts[index].name = if name.is_empty() {
                    profile.label().to_owned()
                } else {
                    name.to_owned()
                };
                if self.save_account_config() {
                    self.workspace.account_names_dirty[index] = false;
                }
            }
        }
    }

    pub(super) fn account_name(&self, profile: GameProfile) -> String {
        self.accounts[profile.index()].name.clone()
    }

    pub(super) fn account_enabled(&self, profile: GameProfile) -> bool {
        self.accounts[profile.index()].enabled
    }

    pub(super) fn save_account_config(&mut self) -> bool {
        match Config::save_accounts(&self.accounts) {
            Ok(()) => true,
            Err(error) => {
                self.set_status(error, true);
                false
            }
        }
    }

    pub(super) fn refresh_account_setup(&mut self) {
        for profile in GameProfile::ALL {
            let index = profile.index();
            let config = match Config::for_profile(profile) {
                Ok(config) => config,
                Err(error) => {
                    self.runtimes[index].telemetry.account_setup = AccountSetup {
                        check_error: Some(error),
                        ..AccountSetup::default()
                    };
                    continue;
                }
            };

            // The setup marker is earned after the native PokéIdle session
            // reports an authenticated connection. Twitch is no longer a
            // controller-owned login prerequisite.
            let mut setup_complete = accounts::setup_complete(profile).unwrap_or(false);
            let verified = self.runtimes[index].game.native.as_ref().is_some_and(|session| {
                session.snapshot().phase == "Online"
            });
            if verified && !setup_complete {
                match accounts::complete_setup(profile) {
                    Ok(()) => setup_complete = true,
                    Err(error) => logging::warn(&error),
                }
            }
            self.runtimes[index].telemetry.account_setup = AccountSetup {
                profile_ready: config.profile_dir.exists(),
                setup_complete,
                check_error: None,
            };
        }

        self.account_setup_checked_at = Some(Instant::now());
    }

}
