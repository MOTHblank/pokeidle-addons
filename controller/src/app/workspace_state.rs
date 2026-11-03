//! UI-only workspace selection and presentation preferences.
//! No native sessions, worker channels, background jobs or timers live here.
//! Keep defaults equivalent to the previous ControllerApp initialization.
use super::{AccountsSection, AutomationSection, HuntRanking, HuntSubtab, PanelTab};

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
    pub(super) accounts_section: AccountsSection,
    pub(super) kick_selected_account: usize,
    pub(super) dashboard_show_history: bool,
    pub(super) dashboard_session_view: bool,
    pub(super) hunt_account: usize,
    pub(super) automation_account: usize,
    pub(super) automation_section: AutomationSection,
    pub(super) history_rows_limit: usize,
    pub(super) hunt_subtab: HuntSubtab,
    pub(super) hunt_page: usize,
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
            accounts_section: AccountsSection::Profiles,
            kick_selected_account: 0,
            dashboard_show_history: false,
            dashboard_session_view: false,
            hunt_account: 0,
            automation_account: 0,
            automation_section: AutomationSection::Capture,
            history_rows_limit: 100,
            hunt_subtab: HuntSubtab::Explorer,
            hunt_page: 0,
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
    pub(super) fn select_account(&mut self, index: usize) {
        self.history_rows_limit = 100;
        self.dashboard_show_history = false;
        self.hunt_account = index;
        if self.panel_tab == PanelTab::Hunts && self.hunt_subtab == HuntSubtab::Compare {
            self.hunt_subtab = HuntSubtab::Explorer;
        }
        self.automation_account = index;
        self.hunt_page = 0;
    }
}
