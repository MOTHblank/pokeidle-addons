//! Native capture, restock and game automation controls; worker ownership stays in ControllerApp.
use super::{draw_game_automation_contents, draw_native_automation, draw_native_missing_panel,
    egui, market_panel, tr, workspace_tab_button, AutomationSection, ControllerApp};

pub(super) fn draw_automation_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    // Keep section navigation above the only vertical scroll region. Market
    // rules can be very long; they should never push Catch controls offscreen.
    ui.horizontal_wrapped(|ui| {
        for (section, label, hint) in [
            (AutomationSection::Capture, "Auto Catch", "Capture preferences and live statistics"),
            (AutomationSection::Game, "Game & restock", "Server settings and restock status"),
            (AutomationSection::Market, "Market", "Automatic buying rules and Snatch"),
        ] {
            if workspace_tab_button(ui, label,
                app.automation_section == section, hint).clicked() {
                app.automation_section = section;
            }
        }
    });
    ui.add_space(7.0);
    ui.separator();
    let index = app.automation_account;
    egui::ScrollArea::vertical()
        .id_salt(("automation-page", index, app.automation_section as u8))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            draw_automation_contents(app, ui);
        });
}

fn draw_automation_contents(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let index = app.automation_account;
    match app.automation_section {
        AutomationSection::Capture => {
            if app.games[index].native.is_some() {
                draw_native_automation(app, ui, index);
            } else {
                draw_native_missing_panel(app, ui, Some(index),
                    tr("Start this account to configure Auto Catch"));
            }
        }
        AutomationSection::Game => draw_game_automation_contents(app, ui),
        AutomationSection::Market => market_panel::draw(app, ui, index),
    }
}

