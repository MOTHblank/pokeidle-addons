//! Account profile management cards and profile/Kick workspace routing.
use super::{egui, kick_panel, localize_status, pt_br, status_chip, tr, tr_dynamic,
    AccountsSection, Align, Color32, ControllerApp, Duration, FontId, GameProfile,
    Layout, ProfileAction, RichText, Stroke, BAD, BORDER, GOOD, MUTED,
    PANEL, PANEL_ALT, TEXT, WARN};

/// Show all four account slots at once; a card never switches or stops another
/// account. Connection indicators come from the authenticated game session.
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
    let setup = app.runtimes[index].telemetry.account_setup.clone();
    let health = app.runtimes[index].game.health();
    let running = app.runtimes[index].game.is_running();
    let game_phase = app.runtimes[index].game.native.as_ref().map(|session| session.snapshot().phase);
    let twitch_miner = app.runtimes[index].game.native.as_ref()
        .and_then(|session| session.twitch_miner_snapshot());
    let twitch_miner_online = twitch_miner.as_ref()
        .is_some_and(|state| state.phase == moth_twitch::MinerPhase::Online);
    let game_online = game_phase.as_deref() == Some("Online")
        || (running && health.telemetry_is_fresh() && health.logged_in);
    let connecting = app.runtimes[index].session.native_verifying || app.runtimes[index].session.native_handoff_busy
        || app.runtimes[index].session.cached_native_busy || app.runtimes[index].game.handoff_closing;

    ui.set_width(width);
    egui::Frame::new().fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, if game_online { GOOD } else { BORDER }))
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
        if response.changed() { app.workspace.account_names_dirty[index] = true; }
        if response.lost_focus() && app.workspace.account_names_dirty[index] { changed = true; }
        ui.add_space(6.0);

        let game_label = if game_online { tr("Connected") }
            else if game_phase.as_deref() == Some("NeedsLogin") { tr("Sign-in required") }
            else if game_phase.as_deref() == Some("Reconnecting") { tr("Reconnecting") }
            else if connecting { tr("Connecting") }
            else if running { tr("Waiting for login") }
            else { tr("Not signed in") };
        let twitch_miner_label = match twitch_miner.as_ref().map(|state| state.phase) {
            Some(moth_twitch::MinerPhase::Online) => tr("Active"),
            Some(moth_twitch::MinerPhase::Connecting) |
            Some(moth_twitch::MinerPhase::Reconnecting) => tr("Reconnecting"),
            Some(moth_twitch::MinerPhase::NeedsLogin) => tr("Optional sign-in"),
            Some(moth_twitch::MinerPhase::Failed) => tr("Error"),
            Some(moth_twitch::MinerPhase::Stopped) => tr("Stopped"),
            None => tr("Optional sign-in"),
        };
        ui.horizontal_wrapped(|ui| {
            status_chip(ui, "PokéIdle", game_label, game_online);
            status_chip(ui, tr("Twitch miner"), twitch_miner_label, twitch_miner_online);
        });
        if let Some(state)=twitch_miner.as_ref() {
            if twitch_miner_online {
                ui.label(RichText::new(if pt_br() {
                    format!("{} / {} canais · +{} pontos · {} bônus",
                        state.active,state.desired,state.points_gained,state.bonuses_claimed)
                } else {
                    format!("{} / {} channels · +{} points · {} bonuses",
                        state.active,state.desired,state.points_gained,state.bonuses_claimed)
                }).size(10.0).color(DIM));
            }
            if let Some(error)=state.error.as_deref() {
                ui.label(RichText::new(error).size(10.0).color(WARN));
            }
        }
        if let Some(error) = app.runtimes[index].session.native_error.as_deref() {
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

        // Keep game profile cards compact; Kick has its own workspace.
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            let kick_running = app.runtimes[index].kick.kick_monitor.is_some();
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
            status_chip(ui, "PokéIdle login", game_label, game_online);
            status_chip(ui, tr("Twitch miner"), twitch_miner_label, twitch_miner_online);
            ui.add_space(7.0);
            if game_online {
                ui.label(RichText::new(tr(
                    "PokéIdle is connected. Future starts can reuse your saved login."
                )).size(10.0).color(GOOD));
            } else {
                ui.label(RichText::new(tr(
                    "Open Firefox without monitoring, sign in to PokéIdle, then close Firefox and start the account."
                )).size(10.0).color(MUTED));
            }
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr("PokéIdle / Google sign-in (unmonitored)")).clicked() {
                    app.profile_action(profile, ProfileAction::GoogleSignIn);
                }
                if ui.button(tr("Twitch miner sign-in (optional)")).clicked() {
                    app.profile_action(profile, ProfileAction::TwitchMiner);
                }
            });
            ui.label(RichText::new(tr(
                "Twitch mining is optional and never blocks PokéIdle startup."
            )).size(10.0).color(MUTED));
        });
        if changed {
            let trimmed = app.accounts[index].name.trim();
            app.accounts[index].name = if trimmed.is_empty() {
                profile.label().to_owned()
            } else { trimmed.to_owned() };
            if app.save_account_config() {
                app.workspace.account_names_dirty[index] = false;
            }
        }
    });
}

