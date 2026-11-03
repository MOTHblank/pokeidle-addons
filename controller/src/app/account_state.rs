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
                self.set_status(tr("Controller options saved").to_owned(), false);
            }
            Err(error) => self.set_status(error, true),
        }
    }

    pub(super) fn sync_monitor_paces(&self, background: bool) {
        // Exactly one account may be expanded when focused. Background mode
        // overrides that selection without clearing the open details panel.
        for (index, slot) in self.games.iter().enumerate() {
            let pace = monitor_pace_for_slot(
                self.workspace.selected_details, index, self.accounts[index].enabled, background
            );
            if let Some(monitor) = slot.monitor.as_ref() {
                monitor.set_pace(pace);
            }
        }
    }

    /// Switching accounts changes the workspace selection, not a live session.
    pub(super) fn select_workspace_account(&mut self, index: usize) {
        if index >= self.games.len() || self.workspace.account_selection_matches(index) { return; }
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
                    self.account_setup[index] = AccountSetup {
                        check_error: Some(error),
                        ..AccountSetup::default()
                    };
                    continue;
                }
            };

            // The setup marker is earned only after both PokéIdle and Twitch
            // report authenticated connections.
            let mut setup_complete = accounts::setup_complete(profile).unwrap_or(false);
            let verified = self.games[index].native.as_ref().is_some_and(|session| {
                session.snapshot().phase == "Online"
                    && session.twitch_snapshot().phase == "Online"
            });
            if verified && !setup_complete {
                match accounts::complete_setup(profile) {
                    Ok(()) => setup_complete = true,
                    Err(error) => logging::warn(&error),
                }
            }
            self.account_setup[index] = AccountSetup {
                profile_ready: config.profile_dir.exists(),
                setup_complete,
                check_error: None,
            };
        }

        self.account_setup_checked_at = Some(Instant::now());
    }

}
