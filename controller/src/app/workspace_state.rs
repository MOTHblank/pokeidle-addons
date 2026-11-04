//! UI-only workspace selection and presentation preferences.
//! No native sessions, worker channels, background jobs or timers live here.
//! Keep defaults equivalent to the previous ControllerApp initialization.
use super::{AccountsSection, AutomationSection, HuntRanking, HuntSubtab, PanelTab, UiOptions,
    AppPreferences, SettingsBackup, OptionsSection};

#[derive(Default)]
pub(super) struct BrowserTabEditor {
    pub(super) input: String,
    pub(super) open: bool,
}

/// Hunt Atlas search and ranking belong to the currently visible workspace.
pub(super) struct HuntUiFilters {
    pub(super) query: String,
    pub(super) region: String,
    pub(super) pokemon_type: String,
    pub(super) ranking: HuntRanking,
    pub(super) atlas_page: [usize;4],
}

impl Default for HuntUiFilters {
    fn default() -> Self {
        Self {
            query: String::new(),
            region: String::new(),
            pokemon_type: String::new(),
            ranking: HuntRanking::TrainerXp,
            atlas_page: [0;4],
        }
    }
}

pub(super) struct WorkspaceState {
    pub(super) panel_tab: PanelTab,
    pub(super) options_section: OptionsSection,
    pub(super) preferences_failed_save: Option<(AppPreferences, String)>,
    pub(super) backup_preview: Option<SettingsBackup>,
    pub(super) exit_confirmation: bool,
    pub(super) exit_approved: bool,
    /// An unsuccessful Options write retains the exact requested values for retry.
    pub(super) options_failed_save: Option<(UiOptions, String)>,
    pub(super) accounts_section: AccountsSection,
    pub(super) kick_selected_account: usize,
    pub(super) twitch_selected_account: usize,
    pub(super) twitch_favorites: [Vec<String>;4],
    pub(super) dashboard_show_history: bool,
    pub(super) dashboard_session_view: bool,
    pub(super) hunt_account: usize,
    pub(super) automation_account: usize,
    pub(super) automation_section: AutomationSection,
    pub(super) history_rows_limit: usize,
    pub(super) hunt_subtab: HuntSubtab,
    pub(super) hunt_page: usize,
    pub(super) boss_selected: [String;4],
    pub(super) boss_leave_confirm: [bool;4],
    pub(super) boss_leave_target: [Option<String>;4],
    pub(super) boss_history_page: [usize;4],
    pub(super) boss_history_filter: [String;4],
    pub(super) selected_details: Option<usize>,
    pub(super) restock_open: [bool;4],
    pub(super) session_reset_confirm: [bool;4],
    pub(super) account_names_dirty: [bool; 4],
    pub(super) market_copy_targets: [bool;4],
    pub(super) market_copy_activation: bool,
    pub(super) browser_tabs: [BrowserTabEditor; 4],
    pub(super) hunt_filters: HuntUiFilters,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            panel_tab: PanelTab::Overview,
            options_section: OptionsSection::General,
            preferences_failed_save: None,
            backup_preview: None,
            exit_confirmation: false,
            exit_approved: false,
            options_failed_save: None,
            accounts_section: AccountsSection::Profiles,
            kick_selected_account: 0,
            twitch_selected_account: 0,
            twitch_favorites: std::array::from_fn(|_| Vec::new()),
            dashboard_show_history: false,
            dashboard_session_view: false,
            hunt_account: 0,
            automation_account: 0,
            automation_section: AutomationSection::Capture,
            history_rows_limit: 100,
            hunt_subtab: HuntSubtab::Explorer,
            hunt_page: 0,
            boss_selected: std::array::from_fn(|_| String::new()),
            boss_leave_confirm: [false;4],
            boss_leave_target: std::array::from_fn(|_| None),
            boss_history_page: [0;4],
            boss_history_filter: std::array::from_fn(|_| String::new()),
            selected_details: None,
            restock_open: [false;4],
            session_reset_confirm: [false;4],
            account_names_dirty: [false; 4],
            market_copy_targets: [false;4],
            market_copy_activation: false,
            browser_tabs: std::array::from_fn(|_| BrowserTabEditor::default()),
            hunt_filters: HuntUiFilters::default(),
        }
    }
}

// The caller persists any edited account name before changing the selection.
// This transition is purely UI state and never touches a game session.
impl WorkspaceState {
    pub(super) fn account_selection_matches(&self, index: usize) -> bool {
        self.hunt_account == index && self.automation_account == index
    }

    pub(super) fn select_account(&mut self, index: usize) {
        self.history_rows_limit = 100;
        self.dashboard_show_history = false;
        self.hunt_account = index;
        if self.panel_tab == PanelTab::Hunts && self.hunt_subtab == HuntSubtab::Compare {
            self.hunt_subtab = HuntSubtab::Explorer;
        }
        self.automation_account = index;
        self.hunt_page = 0;
        // A leave confirmation must never survive a switch between accounts.
        self.boss_leave_confirm = [false;4];
        self.boss_leave_target = std::array::from_fn(|_| None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_options_save_survives_account_switch_for_exact_retry() {
        let mut workspace = WorkspaceState::default();
        assert!(workspace.options_failed_save.is_none());
        let mut requested = UiOptions::default();
        requested.reduce_updates_unfocused = false;
        requested.atlas_aoe_enabled[2] = true;
        workspace.options_failed_save = Some((requested, "disk unavailable".into()));
        workspace.select_account(3);
        let (retry, error) = workspace.options_failed_save.as_ref().unwrap();
        assert_eq!(*retry, requested);
        assert_eq!(error, "disk unavailable");
    }

    #[test]
    fn switching_accounts_discards_leave_confirmations_but_keeps_history_views() {
        let mut workspace = WorkspaceState::default();
        workspace.boss_leave_confirm = [true;4];
        workspace.boss_leave_target[1] = Some("old-arena".into());
        workspace.boss_history_page[1] = 2;
        workspace.boss_history_filter[1] = "example".into();
        workspace.select_account(2);
        assert_eq!(workspace.boss_leave_confirm, [false;4]);
        assert!(workspace.boss_leave_target.iter().all(Option::is_none));
        assert_eq!(workspace.boss_history_page[1], 2);
        assert_eq!(workspace.boss_history_filter[1], "example");
    }

    #[test]
    fn selecting_account_synchronizes_automation_when_hunt_already_matches() {
        let mut workspace=WorkspaceState::default();
        workspace.hunt_account=2;
        workspace.automation_account=1;
        assert!(!workspace.account_selection_matches(2));
        workspace.select_account(2);
        assert_eq!(workspace.hunt_account,2);
        assert_eq!(workspace.automation_account,2);
        assert!(workspace.account_selection_matches(2));
    }

    #[test]
    fn selecting_account_synchronizes_hunt_when_automation_already_matches() {
        let mut workspace=WorkspaceState::default();
        workspace.hunt_account=0;
        workspace.automation_account=3;
        assert!(!workspace.account_selection_matches(3));
        workspace.select_account(3);
        assert_eq!(workspace.hunt_account,3);
        assert_eq!(workspace.automation_account,3);
        assert!(workspace.account_selection_matches(3));
    }
}
