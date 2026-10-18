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
            let native = self.games[index].native.as_ref()
                .and_then(|session| session.official_kick_live());
            let found = if native.is_some() {
                native
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
            let desired = if roster_changed {
                Some(desired_kick_channels(self, index))
            } else { None };
            // Only the browserless native game socket supplies the complete
            // official roster; the old browser bridge reports live names only.
            let offline: Vec<String> = self.games[index].native.as_ref()
                .and_then(|game| game.official_kick_roster())
                .unwrap_or_default().into_iter()
                .filter(|creator| !creator.live)
                .map(|creator| creator.slug).collect();
            if let Some(monitor) = self.kick_monitors[index].as_mut() {
                let outcome = desired.as_ref()
                    .map(|names| monitor.sync_channels(names))
                    .transpose().and_then(|_| monitor.sync_offline_officials(&offline));
                if let Err(error) = outcome {
                    logging::warn(&format!("Kick channel synchronization failed: {error}"));
                    self.set_status(error, true);
                }
            }
        }
    }

    fn persist_kick_channels(&mut self) {
        match Config::save_kick_channels(&self.kick_channels_input) {
            Ok(()) => self.set_status(
                kick_label(pt_br(), "Kick channels saved", "Canais Kick salvos"), false),
            Err(error) => self.set_status(error, true),
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
        self.persist_kick_channels();
    }
    fn remove_kick_channel(&mut self, index: usize, channel: &str) {
        let mut found = stored_channel_list(self, index);
        found.retain(|slug| slug != channel);
        self.kick_channels_input[index] = found.join(", ");
        self.persist_kick_channels();
    }
    fn start_kick_account(&mut self, index: usize) {
        if self.kick_monitors[index].is_some() { return; }
        let Some(profile) = GameProfile::from_index(index) else { return; };
        let target = desired_kick_channels(self, index);
        if target.is_empty() && !self.kick_discovery_available[index] {
            self.set_status(kick_label(pt_br(),
                "Start PokéIdle or add a Kick channel first",
                "Inicie o PokéIdle ou adicione um canal Kick primeiro"), true);
            return;
        }
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
            "Kick sessions started. Waiting for channel data.",
            "Sessões Kick iniciadas. Aguardando dados dos canais.").to_owned();
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
    fn stop_kick_account(&mut self, index: usize) {
        self.kick_monitors[index].take();
        self.set_status(kick_label(pt_br(), "Kick streams stopped",
            "Transmissões Kick interrompidas"), false);
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
        Some(false) => ("○ OFFLINE", MUTED),
        None => (kick_label(br, "◌ Not checked", "◌ Não verificado"), WARN),
    };
    let points = channel.points.map(|value| value.to_string())
        .unwrap_or_else(|| "—".to_owned());
    let stale = channel.checked_at
        .is_some_and(|checked| checked.elapsed() > Duration::from_secs(300));
    let compact = ui.available_width() < 720.0;
    let mut open = false;
    egui::Frame::new().fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8.0).inner_margin(egui::Margin::symmetric(11, 9))
        .show(ui, |ui| {
        ui.horizontal(|ui| {
            if compact {
                ui.vertical(|ui| {
                    ui.label(RichText::new(&channel.slug).size(13.0).strong().color(TEXT));
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(state).size(10.0).strong().color(state_color));
                        ui.label(RichText::new(format!("{}: {points}",
                            kick_label(br, "Points", "Pontos"))).size(11.0).strong()
                            .color(if channel.points.is_some() { TEXT } else { MUTED }));
                        if stale { ui.label(RichText::new(
                            kick_label(br, "outdated", "desatualizado"))
                            .size(10.0).color(WARN)); }
                    });
                });
            } else {
                ui.add_sized([170.0, 22.0],
                    egui::Label::new(RichText::new(&channel.slug)
                        .size(12.0).strong().color(TEXT)));
                ui.add_sized([130.0, 22.0],
                    egui::Label::new(RichText::new(state)
                        .size(11.0).strong().color(state_color)));
                let shown = if stale { format!("{points}*") } else { points };
                ui.add_sized([160.0, 22.0],
                    egui::Label::new(RichText::new(format!("{}: {shown}",
                        kick_label(br, "Points", "Pontos")))
                        .size(12.0).strong().color(TEXT)));
                if let Some(gained) = channel.gained {
                    ui.label(RichText::new(format!("+{gained}"))
                        .size(10.0).color(if gained > 0 { GOOD } else { MUTED }));
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                open = ui.small_button(kick_label(br, "Open ↗", "Abrir ↗")).clicked();
            });
        });
        if channel.error.is_some() || channel.details.is_some() {
            ui.collapsing(kick_label(br, "Details", "Detalhes"), |ui| {
                if channel.official {
                    ui.label(RichText::new(kick_label(br,
                        "Official PokéIdle channel", "Canal oficial do PokéIdle"))
                        .size(10.0).color(MUTED));
                }
                if let Some(error) = &channel.error {
                    ui.label(RichText::new(error).size(10.0).color(WARN));
                }
                if let Some(detail) = &channel.details {
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
    ui.label(RichText::new(kick_label(br, "Kick streams", "Transmissões Kick"))
        .font(FontId::proportional(22.0)).strong().color(TEXT));
    ui.label(RichText::new(kick_label(br,
        "One workspace for channels, connection health and real point changes.",
        "Canais, conexões e variação real dos pontos em um só lugar."))
        .size(11.0).color(MUTED));
    ui.add_space(12.0);

    ui.horizontal_wrapped(|ui| {
        for i in 0..4 {
            let account = &app.accounts[i];
            let selected = app.kick_selected_account == i;
            let active = app.kick_monitors[i].is_some();
            let title = format!("{}{}",
                account.name, if active { " · ●" } else { "" });
            if ui.selectable_label(selected, title).clicked() {
                app.kick_selected_account = i;
            }
        }
    });
    ui.add_space(10.0);
    let i = app.kick_selected_account.min(3);
    let profile = GameProfile::from_index(i).expect("known Kick profile");
    let kick_login_status = crate::native_kick_login::status(profile);
    if matches!(kick_login_status, crate::native_kick_login::LoginStatus::Done) {
        app.kick_saved_login[i] = true;
    }
    let monitor = app.kick_monitors[i].as_ref().map(|monitor| monitor.snapshot());
    let running = monitor.is_some();
    let configured = stored_channel_list(app, i);
    let desired = desired_kick_channels(app, i);
    let discovered = app.kick_discovered_live[i].clone();
    let auto_available = app.kick_discovery_available[i];
    let official_roster = app.games[i].native.as_ref()
        .and_then(|game| game.official_kick_roster())
        .unwrap_or_else(|| discovered.iter().map(|slug|
            crate::native_kick::OfficialKickChannel {
                slug: slug.clone(), live: true,
            }).collect());
    let offline_balances = app.kick_monitors[i].as_ref()
        .map(|session| session.offline_point_balances()).unwrap_or_default();
    let channel_list = channel_rows(&desired, &official_roster,
        monitor.as_ref(), &offline_balances);
    let online = monitor.as_ref().map(|snapshot|
        snapshot.channels.iter().filter(|c| c.live == Some(true)).count()).unwrap_or(0);
    let receiving = monitor.as_ref().map(|snapshot|
        snapshot.channels.iter().filter(|c|
            c.watch.media.last_segment.is_some_and(|t| t.elapsed() < Duration::from_secs(25)))
            .count()).unwrap_or(0);
    let earned: u64 = monitor.as_ref().map(|snapshot|
        snapshot.channels.iter().map(|c| c.watch.points_gained).sum()).unwrap_or(0);
    egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0).inner_margin(14.0).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&app.accounts[i].name).size(16.0).strong().color(TEXT));
            ui.label(RichText::new(if running {
                kick_label(br, "● RUNNING", "● EM EXECUÇÃO")
            } else {
                kick_label(br, "○ STOPPED", "○ PARADO")
            }).size(10.0).color(if running { GOOD } else { MUTED }));
        });
        ui.add_space(8.0);
        ui.columns(3, |cols| {
            kick_metric(&mut cols[0], kick_label(br, "Live", "Ao vivo"),
                format!("{online}/{}", desired.len()), if online > 0 { GOOD } else { TEXT });
            kick_metric(&mut cols[1], kick_label(br, "Receiving media", "Recebendo mídia"),
                receiving.to_string(), if receiving > 0 { GOOD } else { TEXT });
            kick_metric(&mut cols[2], kick_label(br, "Points gained*", "Pontos ganhos*"),
                format!("+{earned}"), if earned > 0 { GOOD } else { TEXT });
        });
    });

    ui.add_space(12.0);
    egui::Frame::new().fill(PANEL_ALT).stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0).inner_margin(12.0).show(ui, |ui| {
        ui.label(RichText::new(kick_label(br, "1 · Account login", "1 · Login da conta"))
            .strong().size(14.0).color(TEXT));
        if app.kick_saved_login[i] {
            ui.label(RichText::new(kick_label(br,
                "● Kick login saved securely in your operating system.",
                "● Login Kick salvo com proteção do sistema."))
                .size(11.0).color(GOOD));
        } else {
            ui.label(RichText::new(kick_label(br,
                "Authenticate once with this account's Firefox profile.",
                "Entre uma vez na Kick com o Firefox deste perfil."))
                .size(11.0).color(MUTED));
        }
        if !running {
            ui.horizontal_wrapped(|ui| {
                let busy = matches!(kick_login_status,
                    crate::native_kick_login::LoginStatus::Running);
                if busy {
                    ui.label(RichText::new(kick_label(br,
                        "● Waiting for Kick login in Firefox…",
                        "● Aguardando login na Kick pelo Firefox…"))
                        .color(WARN));
                    if ui.button(kick_label(br, "Cancel", "Cancelar")).clicked() {
                        crate::native_kick_login::cancel(profile);
                    }
                } else if ui.button(kick_label(br,
                    "Open Firefox and import Kick login",
                    "Entrar na Kick pelo Firefox")).clicked() {
                    if let Err(error) = crate::native_kick_login::start(profile) {
                        app.set_status(error, true);
                    }
                }
                if app.kick_saved_login[i] &&
                    ui.small_button(kick_label(br,
                        "Forget login", "Esquecer login")).clicked() {
                    app.forget_kick_account(i);
                }
            });
            if let crate::native_kick_login::LoginStatus::Error(message) = &kick_login_status {
                ui.label(RichText::new(message).size(10.0).color(WARN));
            }
            ui.label(RichText::new(kick_label(br,
                "The temporary Firefox profile closes after successful capture. Finish the PokéIdle browser handoff first.",
                "O Firefox temporário fecha ao importar o login. Conclua primeiro a conexão nativa do PokéIdle."))
                .size(10.0).color(DIM));
            ui.collapsing(kick_label(br, "Advanced · Manual token",
                "Avançado · Token manual"), |ui| {
                ui.add(egui::TextEdit::singleline(&mut app.kick_api_token_input[i])
                    .password(true)
                    .hint_text(kick_label(br, "Kick account Bearer token",
                        "Token Bearer da conta Kick"))
                    .desired_width(ui.available_width().min(480.0)));
                ui.checkbox(&mut app.kick_remember_login[i],
                    kick_label(br, "Save in OS vault", "Salvar no cofre do sistema"));
                ui.label(RichText::new(kick_label(br,
                    "Fallback only. Never share your token.",
                    "Apenas alternativa. Nunca compartilhe seu token."))
                    .size(10.0).color(DIM));
            });
        } else {
            ui.label(RichText::new(kick_label(br,
                "The token entry stays hidden while streaming.",
                "O campo do token fica oculto enquanto as transmissões estão ativas."))
                .size(10.0).color(MUTED));
        }
        ui.add_space(10.0);
        ui.label(RichText::new(kick_label(br, "2 · Channels", "2 · Canais"))
            .strong().size(14.0).color(TEXT));
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(if auto_available {
                format!("{} {}",
                    discovered.len(),
                    kick_label(br, "official live channels detected on pokeidle.io",
                        "canais oficiais ao vivo encontrados no pokeidle.io"))
            } else {
                kick_label(br,
                    "Start your PokéIdle account to discover official Kick streamers.",
                    "Inicie sua conta PokéIdle para descobrir os streamers oficiais da Kick.")
                    .to_owned()
            }).size(11.0).color(if auto_available { GOOD } else { MUTED }));
        });
        if !discovered.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for slug in &discovered {
                    ui.label(RichText::new(format!("● {slug}"))
                        .size(11.0).color(GOOD));
                }
            });
        }
        if discovered.len() + configured.iter()
            .filter(|name| !discovered.contains(name)).count() > 12 {
            ui.label(RichText::new(kick_label(br,
                "12-channel limit: live official channels take priority.",
                "Limite de 12 canais: os oficiais ao vivo têm prioridade."))
                .size(10.0).color(WARN));
        }
        ui.add_space(8.0);
        ui.label(RichText::new(kick_label(br,
            "Manual channels (optional)", "Canais manuais (opcionais)"))
            .size(11.0).color(MUTED));
        if configured.is_empty() {
            ui.label(RichText::new(kick_label(br,
                "Official PokéIdle channels are added automatically; add extras below if needed.",
                "Canais oficiais do PokéIdle entram automaticamente. Adicione extras se desejar."))
                .size(11.0).color(MUTED));
        } else {
            ui.horizontal_wrapped(|ui| {
                for channel in &configured {
                    egui::Frame::new().fill(PANEL)
                        .stroke(Stroke::new(1.0, BORDER))
                        .corner_radius(6.0).inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(channel).size(11.0).color(TEXT));
                            if ui.add_enabled(!running,
                                egui::Button::new("×")).clicked() {
                                app.remove_kick_channel(i, channel);
                            }
                        });
                    });
                }
            });
        }
        if !running {
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut app.kick_channel_new[i])
                    .hint_text(kick_label(br, "Channel name or kick.com URL",
                        "Nome do canal ou link kick.com"))
                    .desired_width(ui.available_width().min(340.0)));
                if ui.button(kick_label(br, "+ Add channel", "+ Adicionar canal")).clicked() {
                    app.add_kick_channel(i);
                }
            });
            ui.label(RichText::new(kick_label(br,
                "Up to 12 active channels. Manual choices are saved; official live channels update automatically.",
                "Até 12 canais ativos. Os manuais ficam salvos; oficiais ao vivo atualizam automaticamente."))
                .size(10.0).color(DIM));
        } else {
            ui.label(RichText::new(kick_label(br,
                "Stop streams before changing channel choices.",
                "Pare as transmissões para alterar os canais."))
                .size(10.0).color(DIM));
        }
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            if running {
                if ui.button(kick_label(br, "■ Stop streams", "■ Parar transmissões")).clicked() {
                    app.stop_kick_account(i);
                }
            } else if ui.add_enabled(!desired.is_empty() || auto_available,
                egui::Button::new(kick_label(br, "▶ Start streams", "▶ Iniciar transmissões")))
                .clicked()
            {
                app.start_kick_account(i);
            }
        });
    });

    ui.add_space(12.0);
    ui.label(RichText::new(kick_label(br, "Channel activity", "Canais Kick"))
        .size(15.0).strong().color(TEXT));
    ui.label(RichText::new(kick_label(br,
        "Each channel shows its status and your verified point balance; — means unavailable.",
        "Cada canal mostra seu status e saldo de pontos verificado; — indica saldo indisponível."))
        .size(10.0).color(MUTED));
    ui.add_space(8.0);
    ui.vertical(|ui| {
        if channel_list.is_empty() {
            ui.label(RichText::new(kick_label(br,
                "Start PokéIdle to discover official streamers, or add a manual channel.",
                "Inicie o PokéIdle para descobrir streamers oficiais ou adicione um canal manual."))
                .size(11.0).color(MUTED));
        }
        for channel in &channel_list {
            if kick_channel_line(ui, channel) {
                if let Err(error) = open_kick_channels_in_regular_browser(
                    &[channel.slug.clone()]) {
                    app.set_status(error, true);
                }
            }
            ui.add_space(5.0);
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
