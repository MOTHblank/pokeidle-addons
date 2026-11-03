//! UI-only workspace selection and presentation preferences.
//! No native sessions, worker channels, background jobs or timers live here.
//! Keep defaults equivalent to the previous ControllerApp initialization.
use super::{AccountsSection, AutomationSection, HuntSubtab, PanelTab};

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
        }
    }
}
