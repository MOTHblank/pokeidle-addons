//! Account profile management cards and profile/Kick workspace routing.
use super::{egui, kick_panel, localize_status, pt_br, status_chip, tr, tr_dynamic,
    AccountsSection, Align, Color32, ControllerApp, Duration, FontId, GameProfile,
    Layout, ProfileAction, RichText, Stroke, BAD, BORDER, DIM, GOOD, MUTED,
    PANEL, PANEL_ALT, TEXT, WARN};

/// Show all four account slots at once; a card never switches or stops another
/// account. All indicators come from the current game and Twitch connection.
pub(super) fn draw_accounts_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        if ui.selectable_label(app.workspace.accounts_section == AccountsSection::Profiles,
            if pt_br() { "Perfis PokéIdle" } else { "PokéIdle profiles" }).clicked() {
            app.workspace.accounts_section = AccountsSection::Profiles;
        }
        if ui.selectable_label(app.workspace.accounts_section == AccountsSection::Kick,
            "Kick").clicked() {
            app.workspace.accounts_section = AccountsSection::Kick;
        }
    });
    ui.add_space(12.0);
    if app.workspace.accounts_section == AccountsSection::Kick {
        kick_panel::draw_kick_panel(app, ui);
        return;
    }

    if app.account_setup_checked_at
        .is_none_or(|checked| checked.elapsed() >= Duration::from_secs(15))
    {
        app.refresh_account_setup();
    }

    ui.label(RichText::new(tr("Account Manager"))
        .font(FontId::proportional(22.0)).strong().color(TEXT));
    ui.label(RichText::new(tr("Manage up to four accounts. Firefox is only needed to sign in."))
        .size(11.0).color(MUTED));
    ui.add_space(12.0);

    let gap = 12.0;
    egui::ScrollArea::vertical().id_salt("account_manager_four_cards")
        .auto_shrink([false, true]).show(ui, |ui| {
        let columns = if ui.available_width() >= 820.0 { 2 } else { 1 };
        let width = if columns == 2 {
            (ui.available_width() - gap) / 2.0
        } else { ui.available_width() };
        for row in GameProfile::ALL.chunks(columns) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &profile in row {
                    ui.allocate_ui_with_layout(egui::vec2(width, 0.0),
                        Layout::top_down(Align::Min), |ui| {
                        draw_account_manager_card(app, ui, profile, width);
                    });
                }
            });
            ui.add_space(12.0);
        }
    });
}

fn draw_account_manager_card(app: &mut ControllerApp, ui: &mut egui::Ui,
    profile: GameProfile, width: f32)
{
    let index = profile.index();
    let enabled = app.accounts[index].enabled;
    let setup = app.account_setup[index].clone();
    let health = app.games[index].health();
    let running = app.games[index].is_running();
    let game_phase = app.games[index].native.as_ref().map(|session| session.snapshot().phase);
    let twitch_phase = app.games[index].native.as_ref().map(|session| session.twitch_snapshot());
    let game_online = game_phase.as_deref() == Some("Online")
        || (running && health.telemetry_is_fresh() && health.logged_in);
    let twitch_online = twitch_phase.as_ref().is_some_and(|state| state.phase == "Online");
    let connecting = app.native_verifying[index] || app.native_handoff_busy[index]
        || app.cached_native_busy[index] || app.games[index].handoff_closing;

    ui.set_width(width);
    egui::Frame::new().fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, if game_online && twitch_online { GOOD } else { BORDER }))
        .corner_radius(10.0).inner_margin(12.0).show(ui, |ui| {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(profile.label()).size(15.0).strong().color(TEXT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // A regular checkbox blends into the dark account cards.
                // Make the account's enabled state obvious and clickable.
                let (label, fill, border, hint) = if enabled {
                    (
                        if pt_br() { "Habilitada" } else { "Enabled" },
                        Color32::from_rgb(24, 108, 65),
                        GOOD,
                        if pt_br() { "Clique para desabilitar esta conta" }
                        else { "Click to disable this account" },
                    )
                } else {
                    (
                        if pt_br() { "Habilitar" } else { "Enable" },
                        Color32::from_rgb(144, 43, 52),
                        BAD,
                        if pt_br() { "Clique para habilitar esta conta" }
                        else { "Click to enable this account" },
                    )
                };
                if ui.add_sized(
                    [118.0, 30.0],
                    egui::Button::new(
                        RichText::new(label).strong().size(12.0).color(Color32::WHITE)
                    )
                    .fill(fill)
                    .stroke(Stroke::new(1.5, border))
                    .corner_radius(7.0),
                ).on_hover_text(hint).clicked() {
                    app.accounts[index].enabled = !enabled;
                    changed = true;
                }
            });
        });
        ui.add_space(5.0);
        let response = ui.add(
            egui::TextEdit::singleline(&mut app.accounts[index].name)
                .hint_text(profile.label()).desired_width(ui.available_width())
        );
        if response.changed() { app.account_names_dirty[index] = true; }
        if response.lost_focus() && app.account_names_dirty[index] { changed = true; }
        ui.add_space(6.0);

        let game_label = if game_online { tr("Connected") }
            else if game_phase.as_deref() == Some("NeedsLogin") { tr("Sign-in required") }
            else if game_phase.as_deref() == Some("Reconnecting") { tr("Reconnecting") }
            else if connecting { tr("Connecting") }
            else if running { tr("Waiting for login") }
            else { tr("Not signed in") };
        let twitch_label = if twitch_online { tr("Connected") }
            else if let Some(state) = twitch_phase.as_ref() {
                if state.phase == "NeedsLogin" { tr("Sign-in required") }
                else if state.phase == "Reconnecting" { tr("Reconnecting") }
                else if connecting { tr("Connecting") }
                else { tr(&state.phase) }
            } else if running { tr("Waiting for Twitch sign-in") }
            else { tr("Not verified") };
        ui.horizontal_wrapped(|ui| {
            status_chip(ui, "Twitch", twitch_label, twitch_online);
            status_chip(ui, "PokéIdle", game_label, game_online);
        });
        if let Some(state) = twitch_phase.as_ref() {
            if twitch_online {
                ui.label(RichText::new(tr_dynamic(format!("{} / {} Twitch channels",
                    state.joined, state.desired))).size(10.0).color(DIM));
            }
            if let Some(error) = &state.error {
                ui.label(RichText::new(tr_dynamic(localize_status(error.to_owned()))).size(10.0).color(WARN));
            }
        }
        if let Some(error) = app.native_error[index].as_deref() {
            ui.label(RichText::new(tr_dynamic(localize_status(error.to_owned()))).size(10.0).color(WARN));
        }
        if let Some(error) = setup.check_error.as_deref() {
            ui.label(RichText::new(tr_dynamic(localize_status(error.to_owned()))).size(10.0).color(WARN));
        }

        ui.add_space(7.0);
        ui.horizontal_wrapped(|ui| {
            if running {
                if ui.small_button(tr("Stop")).clicked() { app.stop_one(profile); }
            } else if enabled {
                if ui.small_button(tr("Start account")).clicked() {
                    app.launch_one(profile);
                }
            } else if ui.small_button(if pt_br() { "Habilitar e iniciar" }
                                      else { "Enable and start" }).clicked() {
                // Game 3/4 are opt-in by default. An explicit Start action
                // should not silently do nothing while the slot is disabled.
                app.accounts[index].enabled = true;
                if app.save_account_config() {
                    app.launch_one(profile);
                }
            }
        });

        // Keep game/Twitch profile cards compact; Kick has its own workspace.
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            let kick_running = app.kick_monitors[index].is_some();
            let kick_channels = kick_panel::desired_kick_channels(app, index).len();
            ui.label(RichText::new(format!("Kick · {} {}",
                kick_channels, if pt_br() { "canais" } else { "channels" }))
                .size(10.0).color(if kick_running { GOOD } else { MUTED }));
            if ui.small_button(if pt_br() { "Gerenciar Kick →" }
                               else { "Manage Kick →" }).clicked() {
                app.workspace.kick_selected_account = index;
                app.workspace.accounts_section = AccountsSection::Kick;
            }
        });

        // Sign-in is a primary account setup action, not a collapsible guide:
        // keep the checklist visible even after setup completes or a slot
        // is disabled. This makes the ordinary, unmonitored login path clear.
        ui.add_space(7.0);
        if ui.small_button(tr("Profile folder")).clicked() {
            app.profile_action(profile, ProfileAction::Folder);
        }
        ui.add_space(8.0);
        egui::Frame::new().fill(PANEL).corner_radius(6.0)
            .inner_margin(10.0).show(ui, |ui| {
            ui.label(RichText::new(tr("Sign-in checklist")).strong().color(TEXT));
            ui.add_space(5.0);
            status_chip(ui, "Firefox profile",
                if setup.profile_ready { tr("Ready") } else { tr("Created on first launch") },
                setup.profile_ready);
            status_chip(ui, "Twitch login", twitch_label, twitch_online);
            status_chip(ui, "PokéIdle login", game_label, game_online);
            ui.add_space(7.0);
            if game_online && twitch_online {
                ui.label(RichText::new(tr(
                    "Both logins are connected. Future starts can reuse your saved login."
                )).size(10.0).color(GOOD));
            } else {
                ui.label(RichText::new(tr(
                    "Use the buttons below to open Firefox without monitoring. After signing in, close Firefox and start the account to verify both logins."
                )).size(10.0).color(MUTED));
            }
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr("Twitch sign-in (unmonitored)")).clicked() {
                    app.profile_action(profile, ProfileAction::Twitch);
                }
                if ui.button(tr("PokéIdle / Google sign-in (unmonitored)")).clicked() {
                    app.profile_action(profile, ProfileAction::GoogleSignIn);
                }
            });
        });
        if changed {
            let trimmed = app.accounts[index].name.trim();
            app.accounts[index].name = if trimmed.is_empty() {
                profile.label().to_owned()
            } else { trimmed.to_owned() };
            if app.save_account_config() {
                app.account_names_dirty[index] = false;
            }
        }
    });
}

