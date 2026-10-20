//! Dedicated Kick workspace inside Accounts. Overview cards remain small;
//! setup and detailed monitoring live here, instead of nested pop-out dialogs.
use super::*;
use crate::native_auth;

fn kick_label(pt: bool, en: &'static str, br: &'static str) -> &'static str {
    if pt { br } else { en }
}
fn stored_channel_list(app: &ControllerApp, index: usize) -> Vec<String> {
    crate::native_kick::normalize_channels(&app.kick_channels_input[index])
        .unwrap_or_default()
}

/// The official live roster takes priority; saved manual channels fill
/// remaining slots. Never permanently persist volatile game-server results.
pub(super) fn desired_kick_channels(app: &ControllerApp, index: usize) -> Vec<String> {
    let mut names = if app.kick_discovery_available[index] {
        app.kick_discovered_live[index].clone()
    } else { Vec::new() };
    for manual in stored_channel_list(app, index) {
        if !names.iter().any(|name| name == &manual) {
            names.push(manual);
        }
    }
    names.truncate(12);
    names
}

impl ControllerApp {
    /// Discover Kick's official live channels from the exact same server
    /// state used by the in-browser scanner: state.kick.oficiais.
    /// Browser-bridge telemetry is a fallback during sign-in only.
    pub(super) fn sync_pokeidle_kick_streamers(&mut self) {
        for index in 0..4 {
            let native_roster = self.games[index].native.as_ref()
                .and_then(|session|session.official_kick_roster());
            let official_roster_changed = self.kick_discovered_roster[index] != native_roster;
            if official_roster_changed {
                self.kick_discovered_roster[index] = native_roster.clone();
            }
            let native_live = native_roster.as_ref().map(|roster|
                roster.iter().filter(|creator|creator.live)
                    .map(|creator|creator.slug.clone()).collect::<Vec<_>>());
            let found = if native_live.is_some() {
                native_live
            } else {
                let health = self.games[index].health();
                if health.kick_state_available && health.telemetry_is_fresh() {
                    let urls = health.kick_streams.iter()
                        .map(|item| item.url.as_str()).collect::<Vec<_>>().join(",");
                    Some(if urls.is_empty() { Vec::new() } else {
                        crate::native_kick::normalize_channels(&urls).unwrap_or_default()
                    })
                } else {
                    None
                }
            };
            let available = found.is_some();
            let channels = found.unwrap_or_default();
            let roster_changed = self.kick_discovery_available[index] != available ||
                self.kick_discovered_live[index] != channels;
            if roster_changed {
                self.kick_discovery_available[index] = available;
                self.kick_discovered_live[index] = channels;
            }
            // A stored Kick login owns its supervisor automatically, even
            // before PokéIdle has supplied the first live roster. An empty
            // roster starts no media workers; later updates open/close them.
            let Some(profile) = GameProfile::from_index(index) else { continue; };
            if matches!(crate::native_kick_login::status(profile),
                crate::native_kick_login::LoginStatus::Done)
            {
                self.kick_saved_login[index] = true;
            }
            let monitor_was_missing = self.kick_monitors[index].is_none();
            if self.kick_saved_login[index] && monitor_was_missing {
                self.start_kick_account(index);
            }
            let desired = if roster_changed {
                Some(desired_kick_channels(self, index))
            } else { None };
            // Query balances for any official creators *not* currently
            // supervised, including online creators beyond the HLS limit.
            // A single read-only poller handles them without video downloads.
            let sync_offline = official_roster_changed || monitor_was_missing;
            let remaining: Vec<String> = if sync_offline && self.kick_monitors[index].is_some() {
                native_roster.unwrap_or_default().into_iter()
                    .map(|creator| creator.slug).collect()
            } else { Vec::new() };
            if let Some(monitor) = self.kick_monitors[index].as_mut() {
                let outcome = desired.as_ref()
                    .map(|names| monitor.sync_channels(names))
                    .transpose().and_then(|_| {
                        if sync_offline {monitor.sync_offline_officials(&remaining)}
                        else {Ok(())}
                    });
                if let Err(error) = outcome {
                    logging::warn(&format!("Kick channel synchronization failed: {error}"));
                    self.set_status(error, true);
                }
            }
        }
    }

    fn persist_kick_channels(&mut self, index: usize) {
        match Config::save_kick_channels(&self.kick_channels_input) {
            Ok(()) => self.set_status(
                kick_label(pt_br(), "Kick channels saved", "Canais Kick salvos"), false),
            Err(error) => self.set_status(error, true),
        }
        // Channel preferences remain editable during automatic monitoring.
        // Apply changes immediately without restarting unaffected streams.
        let desired = desired_kick_channels(self, index);
        // Changing manual channels changes which official creators are
        // supervised. Reconcile the offline point poller immediately rather
        // than waiting for another server roster event.
        let official: Vec<String> = self.kick_discovered_roster[index].as_ref()
            .map(|roster|roster.iter().map(|creator|creator.slug.clone()).collect())
            .unwrap_or_default();
        if let Some(monitor) = self.kick_monitors[index].as_mut() {
            if let Err(error) = monitor.sync_channels(&desired)
                .and_then(|_|monitor.sync_offline_officials(&official)) {
                logging::warn(&format!("Kick channel synchronization failed: {error}"));
                self.set_status(error, true);
            }
        }
    }
    fn add_kick_channel(&mut self, index: usize) {
        let proposed = match crate::native_kick::normalize_channels(&self.kick_channel_new[index]) {
            Ok(found) if found.len() == 1 => found[0].clone(),
            _ => {
                self.set_status(kick_label(pt_br(), "Enter one valid Kick channel",
                    "Informe um canal Kick válido por vez"), true);
                return;
            },
        };
        let mut found = stored_channel_list(self, index);
        if found.iter().any(|name| name == &proposed) {
            self.kick_channel_new[index].clear();
            return;
        }
        found.push(proposed);
        if found.len() > 12 {
            self.set_status(kick_label(pt_br(), "Limit: 12 channels per account",
                "Limite de 12 canais por conta"), true);
            return;
        }
        found.sort();
        self.kick_channels_input[index] = found.join(", ");
        self.kick_channel_new[index].clear();
        self.persist_kick_channels(index);
    }
    fn remove_kick_channel(&mut self, index: usize, channel: &str) {
        let mut found = stored_channel_list(self, index);
        found.retain(|slug| slug != channel);
        self.kick_channels_input[index] = found.join(", ");
        self.persist_kick_channels(index);
    }
    fn start_kick_account(&mut self, index: usize) {
        if self.kick_monitors[index].is_some() { return; }
        let Some(profile) = GameProfile::from_index(index) else { return; };
        let target = desired_kick_channels(self, index);
        // KickMonitor accepts an empty list. It will pick up official live
        // channels when PokéIdle becomes available without a manual click.
        // Prefer a freshly typed token, otherwise use the OS vault. Failed
        // starts retain the masked field so the user can retry without copying.
        let entered = self.kick_api_token_input[index].trim().to_owned();
        let token = if !entered.is_empty() {
            entered.clone()
        } else {
            match native_auth::load_kick_token(profile) {
                Ok(Some(saved)) => saved,
                Ok(None) => {
                    self.kick_saved_login[index] = false;
                    self.set_status(kick_label(pt_br(), "Enter a Kick account token",
                        "Informe o token da sua conta Kick"), true);
                    return;
                },
                Err(error) => { self.set_status(error, true); return; },
            }
        };
        let monitor = match KickMonitor::start(token.clone(), &target.join(",")) {
            Ok(worker) => worker,
            Err(error) => { self.set_status(error, true); return; },
        };
        self.kick_monitors[index] = Some(monitor);
        self.kick_api_token_input[index].clear();
        let mut message = kick_label(pt_br(),
            "Checking the Kick API. Your saved login has not been verified yet.",
            "Consultando a API Kick. A credencial salva ainda não foi verificada.").to_owned();
        let mut error = false;
        if !entered.is_empty() && self.kick_remember_login[index] {
            match native_auth::save_kick_token(profile, &token) {
                Ok(()) => self.kick_saved_login[index] = true,
                Err(reason) => {
                    error = true;
                    message = format!("{} · {}", message,
                        kick_label(pt_br(), "Login was not saved:", "Login não foi salvo:"));
                    message.push(' ');
                    message.push_str(&reason);
                }
            }
        }
        if let Err(reason) = Config::save_kick_channels(&self.kick_channels_input) {
            error = true;
            message.push_str(" · ");
            message.push_str(&reason);
        }
        self.set_status(message, error);
    }
    fn forget_kick_account(&mut self, index: usize) {
        let Some(profile) = GameProfile::from_index(index) else { return; };
        match native_auth::clear_kick_token(profile) {
            Ok(()) => {
                // Explicit sign-out stops existing authenticated connections.
                self.kick_monitors[index].take();
                self.kick_api_token_input[index].clear();
                self.kick_saved_login[index] = false;
                crate::native_kick_login::reset(profile);
                self.set_status(kick_label(pt_br(), "Saved Kick login removed",
                    "Login Kick salvo removido"), false);
            },
            Err(error) => self.set_status(error, true),
        }
    }
}
fn kick_metric(ui: &mut egui::Ui, title: &str, value: String, tone: Color32) {
    ui.vertical(|ui| {
        ui.label(RichText::new(title).size(10.0).color(MUTED));
        ui.label(RichText::new(value).size(19.0).strong().color(tone));
    });
}
/// The list includes official creators even if they are not broadcasting.
/// A missing balance remains unknown (never displayed as zero).
struct KickChannelRow {
    slug: String,
    live: Option<bool>,
    points: Option<u64>,
    checked_at: Option<Instant>,
    gained: Option<u64>,
    official: bool,
    details: Option<crate::native_kick::ChannelStatus>,
    error: Option<String>,
}
fn channel_rows(
    selected: &[String],
    official: &[crate::native_kick::OfficialKickChannel],
    active: Option<&crate::native_kick::KickSnapshot>,
    offline_points: &std::collections::BTreeMap<String, crate::native_kick::OfflinePoints>,
) -> Vec<KickChannelRow> {
    let mut merged = std::collections::BTreeMap::<String, KickChannelRow>::new();
    for creator in official {
        merged.insert(creator.slug.clone(), KickChannelRow {
            slug: creator.slug.clone(), live: Some(creator.live), points: None,
            checked_at: None, gained: None, official: true,
            details: None, error: None,
        });
    }
    for slug in selected {
        merged.entry(slug.clone()).or_insert_with(|| KickChannelRow {
            slug: slug.clone(), live: None, points: None, checked_at: None,
            gained: None, official: false, details: None, error: None,
        });
    }
    if let Some(snapshot) = active {
        for channel in &snapshot.channels {
            let row = merged.entry(channel.slug.clone()).or_insert_with(|| KickChannelRow {
                slug: channel.slug.clone(), live: None, points: None,
                checked_at: None, gained: None, official: false,
                details: None, error: None,
            });
            row.live = channel.live.or(row.live);
            row.points = channel.watch.points;
            row.checked_at = channel.watch.last_points;
            row.gained = channel.watch.points.map(|_| channel.watch.points_gained);
            row.error = channel.error.clone().or_else(|| channel.watch.error.clone());
            row.details = Some(channel.clone());
        }
    }
    for (slug, observed) in offline_points {
        if let Some(row) = merged.get_mut(slug) {
            // Regular viewer status is fresher where available; do not
            // overwrite its confirmed balance with an older offline poll.
            if row.points.is_none() {
                row.points = observed.balance;
                row.checked_at = observed.updated_at;
            }
            if row.error.is_none() { row.error = observed.error.clone(); }
        }
    }
    let mut rows: Vec<_> = merged.into_values().collect();
    rows.sort_by(|a, b| {
        let rank = |item: &KickChannelRow| match item.live {
            Some(true) => 0, Some(false) => 1, None => 2
        };
        rank(a).cmp(&rank(b)).then_with(|| a.slug.cmp(&b.slug))
    });
    rows
}
fn kick_channel_line(ui: &mut egui::Ui, channel: &KickChannelRow) -> bool {
    let br = pt_br();
    let (state, state_color) = match channel.live {
        Some(true) => (kick_label(br, "● LIVE", "● AO VIVO"), GOOD),
        Some(false) => (kick_label(br, "○ Offline", "○ Offline"), MUTED),
        None => (kick_label(br, "◌ Checking", "◌ Verificando"), WARN),
    };
    let points = channel.points.map(|value| value.to_string())
        .unwrap_or_else(|| "—".to_owned());
    let stale = channel.checked_at
        .is_some_and(|checked| checked.elapsed() > Duration::from_secs(300));
    let mut open = false;
    egui::Frame::new().fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(7.0).inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&channel.slug).size(12.0).strong().color(TEXT));
                ui.label(RichText::new(state).size(10.0).color(state_color));
                ui.label(RichText::new(format!("{}: {points}",
                    kick_label(br, "Points", "Pontos")))
                    .size(11.0).color(if channel.points.is_some() { TEXT } else { MUTED }));
                if let Some(gained) = channel.gained.filter(|value| *value > 0) {
                    ui.label(RichText::new(format!("+{gained}"))
                        .size(11.0).color(GOOD));
                }
                if stale {
                    ui.label(RichText::new(kick_label(br, "Outdated", "Desatualizado"))
                        .size(10.0).color(WARN));
                }
                open = ui.small_button("↗")
                    .on_hover_text(kick_label(br, "Open channel", "Abrir canal"))
                    .clicked();
            });
            if let Some(error) = &channel.error {
                // Keep errors visible once, without duplicating them inside
                // expanded diagnostics or the account summary.
                ui.label(RichText::new(error).size(10.0).color(WARN));
            }
            if let Some(detail) = &channel.details {
                ui.collapsing(kick_label(br, "Technical details", "Detalhes técnicos"), |ui| {
                    if channel.official {
                        ui.label(RichText::new(kick_label(br,
                            "Official PokéIdle channel", "Canal oficial do PokéIdle"))
                            .size(10.0).color(MUTED));
                    }
                    let watch = &detail.watch;
                    ui.label(format!("WebSocket: {} · handshake: {}",
                        watch.ws_connected, watch.handshake_received));
                    ui.label(format!("HLS: {} segments · {:.1} MiB",
                        watch.media.segments, watch.media.bytes as f64 / 1_048_576.0));
                    ui.label(format!("Tracking: {} events", watch.events_sent));
                    if let Some(viewers) = detail.viewers {
                        ui.label(format!("Viewers: {viewers}"));
                    }
                    if let Some(error) = &watch.media.error {
                        ui.label(RichText::new(error).size(10.0).color(WARN));
                    }
                    if let Some(checked) = channel.checked_at {
                        ui.label(format!("{} {} s",
                            kick_label(br, "Balance checked", "Saldo consultado há"),
                            checked.elapsed().as_secs()));
                    }
                });
            }
        });
    open
}

pub(super) fn draw_kick_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let br = pt_br();
    egui::ScrollArea::vertical().id_salt("kick_workspace")
        .auto_shrink([false, true]).show(ui, |ui| {
    ui.label(RichText::new("Kick").font(FontId::proportional(22.0))
        .strong().color(TEXT));
    ui.add_space(8.0);

    ui.horizontal_wrapped(|ui| {
        for index in 0..4 {
            let account = &app.accounts[index];
            let selected = app.kick_selected_account == index;
            let active = app.kick_monitors[index].is_some();
            let title = format!("{}{}", account.name, if active { " ●" } else { "" });
            if ui.selectable_label(selected, title).clicked() {
                app.kick_selected_account = index;
            }
        }
    });
    ui.add_space(10.0);

    let index = app.kick_selected_account.min(3);
    let profile = GameProfile::from_index(index).expect("known Kick profile");
    let login_status = crate::native_kick_login::status(profile);
    if matches!(login_status, crate::native_kick_login::LoginStatus::Done) {
        app.kick_saved_login[index] = true;
    }
    let monitor = app.kick_monitors[index].as_ref().map(|monitor| monitor.snapshot());
    let running = monitor.is_some();
    let auth_denied = monitor.as_ref().is_some_and(|snapshot| snapshot.auth_denied);
    let configured = stored_channel_list(app, index);
    let desired = desired_kick_channels(app, index);
    let discovered = app.kick_discovered_live[index].clone();
    let auto_available = app.kick_discovery_available[index];
    let official_roster = app.games[index].native.as_ref()
        .and_then(|game| game.official_kick_roster())
        .unwrap_or_else(|| discovered.iter().map(|slug|
            crate::native_kick::OfficialKickChannel {
                slug: slug.clone(), live: true,
            }).collect());
    let offline_balances = app.kick_monitors[index].as_ref()
        .map(|session| session.offline_point_balances()).unwrap_or_default();
    let channel_list = channel_rows(&desired, &official_roster,
        monitor.as_ref(), &offline_balances);

    // PokéIdle is authoritative for its official live roster, while Kick's
    // public API is the fallback when there is no fresh game state.
    let online = if auto_available {
        desired.iter().filter(|slug| discovered.contains(slug)).count()
    } else {
        monitor.as_ref().map(|snapshot| snapshot.channels.iter()
            .filter(|channel| channel.live == Some(true)).count()).unwrap_or(0)
    };
    let receiving = monitor.as_ref().map(|snapshot|
        snapshot.channels.iter().filter(|channel|
            channel.watch.media.last_segment.is_some_and(|when|
                when.elapsed() < Duration::from_secs(25))).count()).unwrap_or(0);
    let earned: u64 = monitor.as_ref().map(|snapshot|
        snapshot.channels.iter().map(|channel| channel.watch.points_gained).sum())
        .unwrap_or(0);

    egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER))
        .corner_radius(9.0).inner_margin(12.0).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&app.accounts[index].name)
                .size(15.0).strong().color(TEXT));
            let (status, color) = if auth_denied {
                (kick_label(br, "● Login rejected", "● Login recusado"), WARN)
            } else if running {
                (kick_label(br, "● Automatic monitoring", "● Monitoramento automático"), GOOD)
            } else if app.kick_saved_login[index] {
                (kick_label(br, "○ Starting", "○ Iniciando"), MUTED)
            } else {
                (kick_label(br, "○ Sign in required", "○ Login necessário"), MUTED)
            };
            ui.label(RichText::new(status).size(10.0).color(color));
        });
        ui.add_space(7.0);
        ui.columns(3, |cols| {
            kick_metric(&mut cols[0], kick_label(br, "Live", "Ao vivo"),
                format!("{online}/{}", desired.len()), if online > 0 { GOOD } else { TEXT });
            kick_metric(&mut cols[1], kick_label(br, "Watching", "Assistindo"),
                receiving.to_string(), if receiving > 0 { GOOD } else { MUTED });
            kick_metric(&mut cols[2], kick_label(br, "Points earned", "Pontos ganhos"),
                format!("+{earned}"), if earned > 0 { GOOD } else { TEXT });
        });
    });

    if auth_denied {
        ui.add_space(5.0);
        ui.label(RichText::new(kick_label(br,
            "Kick rejected the saved login. Remove it below and sign in again.",
            "A Kick recusou o login salvo. Remova-o abaixo e entre novamente."))
            .size(11.0).color(WARN));
    } else if let Some(error) = monitor.as_ref().and_then(|snapshot|
        snapshot.error.as_ref())
    {
        ui.add_space(5.0);
        ui.label(RichText::new(error).size(10.0).color(WARN));
    }

    ui.add_space(10.0);
    if !app.kick_saved_login[index] && !running {
        egui::Frame::new().fill(PANEL_ALT).stroke(Stroke::new(1.0, BORDER))
            .corner_radius(8.0).inner_margin(12.0).show(ui, |ui| {
            ui.label(RichText::new(kick_label(br,
                "Connect your Kick account", "Conecte sua conta Kick"))
                .size(13.0).strong().color(TEXT));
            ui.label(RichText::new(kick_label(br,
                "Sign in once. Streams will then be managed automatically.",
                "Entre uma vez. As transmissões serão gerenciadas automaticamente."))
                .size(11.0).color(MUTED));
            ui.horizontal_wrapped(|ui| {
                let busy = matches!(login_status,
                    crate::native_kick_login::LoginStatus::Running);
                if busy {
                    ui.label(RichText::new(kick_label(br,
                        "Importing login…", "Importando login…"))
                        .size(11.0).color(WARN));
                    if ui.small_button(kick_label(br, "Cancel", "Cancelar")).clicked() {
                        crate::native_kick_login::cancel(profile);
                    }
                } else if ui.button(kick_label(br,
                    "Sign in with Firefox", "Entrar com Firefox")).clicked() {
                    if let Err(error) = crate::native_kick_login::start(profile) {
                        app.set_status(error, true);
                    }
                }
            });
            if let crate::native_kick_login::LoginStatus::Error(error) = &login_status {
                ui.label(RichText::new(error).size(10.0).color(WARN));
            }
            ui.collapsing(kick_label(br, "Use token instead", "Usar token manual"), |ui| {
                ui.add(egui::TextEdit::singleline(&mut app.kick_api_token_input[index])
                    .password(true)
                    .hint_text(kick_label(br, "Kick Bearer token", "Token Bearer Kick"))
                    .desired_width(ui.available_width().min(320.0)));
                ui.checkbox(&mut app.kick_remember_login[index],
                    kick_label(br, "Remember login", "Salvar login"));
                if ui.add_enabled(!app.kick_api_token_input[index].trim().is_empty(),
                    egui::Button::new(kick_label(br, "Connect", "Conectar"))).clicked() {
                    app.start_kick_account(index);
                }
            });
        });
        ui.add_space(10.0);
    } else if app.kick_saved_login[index] {
        ui.collapsing(kick_label(br, "Manage Kick login", "Gerenciar login Kick"), |ui| {
            ui.label(RichText::new(kick_label(br,
                "Credentials are stored locally. Removing the login stops monitoring.",
                "As credenciais ficam salvas localmente. Remover o login encerra o monitoramento."))
                .size(10.0).color(MUTED));
            if ui.small_button(kick_label(br, "Forget login", "Esquecer login")).clicked() {
                app.forget_kick_account(index);
            }
        });
        ui.add_space(8.0);
    }

    ui.label(RichText::new(kick_label(br, "Channels", "Canais"))
        .size(15.0).strong().color(TEXT));
    ui.add_space(6.0);

    let active_count = channel_list.iter().filter(|channel|
        channel.live != Some(false)).count();
    let offline_count = channel_list.len().saturating_sub(active_count);
    if active_count == 0 {
        ui.label(RichText::new(if !auto_available && configured.is_empty() {
            kick_label(br,
                "Start PokéIdle to discover official Kick channels.",
                "Inicie o PokéIdle para descobrir os canais oficiais da Kick.")
        } else {
            kick_label(br,
                "No live channels right now. Monitoring continues automatically.",
                "Nenhum canal ao vivo no momento. O monitoramento continua automaticamente.")
        }).size(11.0).color(MUTED));
    }
    for channel in channel_list.iter().filter(|channel| channel.live != Some(false)) {
        if kick_channel_line(ui, channel) {
            if let Err(error) = open_kick_channels_in_regular_browser(
                &[channel.slug.clone()]) {
                app.set_status(error, true);
            }
        }
        ui.add_space(4.0);
    }
    if offline_count > 0 {
        ui.collapsing(format!("{} ({offline_count})",
            kick_label(br, "Offline channels", "Canais offline")), |ui| {
            for channel in channel_list.iter().filter(|channel| channel.live == Some(false)) {
                if kick_channel_line(ui, channel) {
                    if let Err(error) = open_kick_channels_in_regular_browser(
                        &[channel.slug.clone()]) {
                        app.set_status(error, true);
                    }
                }
                ui.add_space(4.0);
            }
        });
    }

    ui.add_space(10.0);
    ui.collapsing(format!("{} ({})",
        kick_label(br, "Additional channels", "Canais adicionais"), configured.len()), |ui| {
        ui.label(RichText::new(kick_label(br,
            "Official live channels are added automatically. Add optional channels here.",
            "Os canais oficiais ao vivo entram automaticamente. Adicione canais extras aqui."))
            .size(11.0).color(MUTED));
        if !configured.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for slug in &configured {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(slug).size(11.0).color(TEXT));
                        if ui.small_button("×")
                            .on_hover_text(kick_label(br, "Remove channel", "Remover canal"))
                            .clicked() {
                            app.remove_kick_channel(index, slug);
                        }
                    });
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::TextEdit::singleline(&mut app.kick_channel_new[index])
                .hint_text(kick_label(br, "Channel or kick.com URL", "Canal ou URL kick.com"))
                .desired_width(ui.available_width().min(260.0)));
            if ui.small_button(kick_label(br, "+ Add", "+ Adicionar")).clicked() {
                app.add_kick_channel(index);
            }
        });
        if discovered.len() + configured.iter()
            .filter(|name| !discovered.contains(name)).count() > 12 {
            ui.label(RichText::new(kick_label(br,
                "Only 12 channels can be watched at once; official live channels take priority.",
                "No máximo 12 canais são assistidos por vez; os oficiais ao vivo têm prioridade."))
                .size(10.0).color(WARN));
        }
    });
    });
}

#[cfg(test)]
mod channel_row_tests {
    use super::*;
    use crate::native_kick::{OfficialKickChannel, OfflinePoints};

    #[test]
    fn one_row_per_official_and_manual_creator_even_when_offline() {
        let official = vec![
            OfficialKickChannel { slug: "offline_one".into(), live: false },
            OfficialKickChannel { slug: "live_two".into(), live: true },
        ];
        let chosen = vec!["live_two".into(), "manual_three".into()];
        let rows = channel_rows(&chosen, &official, None, &Default::default());
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].slug, "live_two");
        assert_eq!(rows[0].live, Some(true));
        assert_eq!(rows[1].slug, "offline_one");
        assert_eq!(rows[1].live, Some(false));
        assert_eq!(rows[2].slug, "manual_three");
        assert_eq!(rows[2].live, None);
        assert!(rows.iter().all(|row| row.points.is_none()));
    }

    #[test]
    fn offline_verified_balance_is_per_channel_not_a_total() {
        let official = vec![
            OfficialKickChannel { slug:"one".into(), live:false },
            OfficialKickChannel { slug:"two".into(), live:false },
        ];
        let mut balances = std::collections::BTreeMap::new();
        balances.insert("one".into(), OfflinePoints {
            balance: Some(125), ..OfflinePoints::default()
        });
        balances.insert("two".into(), OfflinePoints {
            balance: Some(800), ..OfflinePoints::default()
        });
        let rows = channel_rows(&[], &official, None, &balances);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].points, Some(125));
        assert_eq!(rows[1].points, Some(800));
    }
}
