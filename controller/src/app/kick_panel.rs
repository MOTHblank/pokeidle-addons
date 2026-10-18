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

impl ControllerApp {
    fn persist_kick_channels(&mut self) {
        match Config::save_kick_channels(&self.kick_channels_input) {
            Ok(()) => self.set_status(
                kick_label(pt_br(), "Kick channels saved", "Canais Kick salvos").into(), false),
            Err(error) => self.set_status(error, true),
        }
    }
    fn add_kick_channel(&mut self, index: usize) {
        let proposed = match crate::native_kick::normalize_channels(&self.kick_channel_new[index]) {
            Ok(found) if found.len() == 1 => found[0].clone(),
            _ => {
                self.set_status(kick_label(pt_br(), "Enter one valid Kick channel",
                    "Informe um canal Kick válido por vez").into(), true);
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
                "Limite de 12 canais por conta").into(), true);
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
        if stored_channel_list(self, index).is_empty() {
            self.set_status(kick_label(pt_br(), "Add a Kick channel first",
                "Adicione um canal Kick primeiro").into(), true);
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
                        "Informe o token da sua conta Kick").into(), true);
                    return;
                },
                Err(error) => { self.set_status(error, true); return; },
            }
        };
        let monitor = match KickMonitor::start(token.clone(), &self.kick_channels_input[index]) {
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
            "Transmissões Kick interrompidas").into(), false);
    }
    fn forget_kick_account(&mut self, index: usize) {
        let Some(profile) = GameProfile::from_index(index) else { return; };
        match native_auth::clear_kick_token(profile) {
            Ok(()) => {
                // Explicit sign-out stops existing authenticated connections.
                self.kick_monitors[index].take();
                self.kick_api_token_input[index].clear();
                self.kick_saved_login[index] = false;
                self.set_status(kick_label(pt_br(), "Saved Kick login removed",
                    "Login Kick salvo removido").into(), false);
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
fn kick_stream_card(ui: &mut egui::Ui, channel: &crate::native_kick::ChannelStatus,
                    width: f32) {
    let br = pt_br();
    let watch = &channel.watch;
    let recent = watch.media.last_segment
        .is_some_and(|last| last.elapsed() < Duration::from_secs(25));
    let connected = watch.ws_connected && watch.handshake_received;
    let points_updated = watch.last_points
        .is_some_and(|last| last.elapsed() < Duration::from_secs(300));
    let live = channel.live == Some(true);
    let state = if channel.live.is_none() {
        kick_label(br, "Checking", "Verificando")
    } else if live {
        kick_label(br, "LIVE", "AO VIVO")
    } else {
        "OFFLINE"
    };
    let tone = if live { GOOD } else { MUTED };
    egui::Frame::new().fill(PANEL_ALT).stroke(Stroke::new(1.0,
        if live && recent { GOOD } else { BORDER }))
        .corner_radius(10.0).inner_margin(12.0).show(ui, |ui| {
        ui.set_min_width(width - 24.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("● {}", state)).size(10.0).strong().color(tone));
            ui.label(RichText::new(format!("kick.com/{}", channel.slug))
                .size(12.0).strong().color(TEXT));
        });
        if !channel.title.is_empty() {
            ui.label(RichText::new(&channel.title).size(10.0).color(MUTED));
        }
        ui.add_space(6.0);
        ui.columns(2, |cols| {
            cols[0].label(RichText::new(kick_label(br, "Current points", "Saldo de pontos"))
                .size(10.0).color(MUTED));
            cols[0].label(RichText::new(watch.points
                .map(|points| points.to_string()).unwrap_or_else(|| "—".into()))
                .size(20.0).strong().color(TEXT));
            cols[1].label(RichText::new(kick_label(br, "Observed gain", "Ganho observado"))
                .size(10.0).color(MUTED));
            cols[1].label(RichText::new(if watch.points.is_some() {
                    format!("+{}", watch.points_gained)
                } else { "—".into() })
                .size(20.0).strong()
                .color(if watch.points_gained > 0 { GOOD } else { MUTED }));
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(
                if connected {
                    kick_label(br, "● WebSocket connected", "● WebSocket conectado")
                } else {
                    kick_label(br, "○ WebSocket pending", "○ WebSocket pendente")
                }).size(10.0).color(if connected { GOOD } else { MUTED }));
            ui.label(RichText::new(
                if recent {
                    kick_label(br, "● Media flowing", "● Recebendo mídia")
                } else {
                    kick_label(br, "○ Media inactive", "○ Mídia inativa")
                }).size(10.0).color(if recent { GOOD } else { MUTED }));
        });
        if watch.points.is_none() {
            ui.label(RichText::new(kick_label(br,
                "Point balance not verified yet.",
                "Saldo de pontos ainda não verificado.")).size(10.0).color(WARN));
        } else if !points_updated {
            ui.label(RichText::new(kick_label(br,
                "Point data is outdated.",
                "Dados de pontos desatualizados.")).size(10.0).color(WARN));
        } else if watch.points_gained == 0 && live {
            ui.label(RichText::new(kick_label(br,
                "Awaiting a confirmed point increase.",
                "Aguardando aumento confirmado de pontos.")).size(10.0).color(MUTED));
        }
        if let Some(error) = channel.error.as_ref().or(watch.error.as_ref()) {
            ui.label(RichText::new(error).size(10.0).color(WARN));
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.small_button(kick_label(br, "Open channel ↗", "Abrir canal ↗")).clicked() {
                let _ = open_kick_channels_in_regular_browser(&[channel.slug.clone()]);
            }
        });
        ui.collapsing(kick_label(br, "Connection details", "Detalhes da conexão"), |ui| {
            ui.label(format!("WebSocket: {} · handshake: {}",
                watch.ws_connected, watch.handshake_received));
            ui.label(format!("HLS: {} segments · {:.1} MiB",
                watch.media.segments, watch.media.bytes as f64 / 1_048_576.0));
            if let Some(rate) = watch.media.bitrate {
                ui.label(format!("HLS bitrate: {} kbps", rate / 1000));
            }
            ui.label(format!("Tracking events: {}", watch.events_sent));
            if let Some(viewers) = channel.viewers { ui.label(format!("Viewers: {viewers}")); }
            if let Some(last) = watch.last_points {
                ui.label(format!("Balance checked {} s ago", last.elapsed().as_secs()));
            }
            if let Some(error) = &watch.media.error {
                ui.label(RichText::new(error).size(10.0).color(WARN));
            }
        });
    });
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
    let monitor = app.kick_monitors[i].as_ref().map(|monitor| monitor.snapshot());
    let running = monitor.is_some();
    let configured = stored_channel_list(app, i);
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
                format!("{online}/{}", configured.len()), if online > 0 { GOOD } else { TEXT });
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
                "● Token protected by the operating system (may expire).",
                "● Token protegido pelo sistema (pode expirar)."))
                .size(11.0).color(GOOD));
        } else {
            ui.label(RichText::new(kick_label(br,
                "Login is not configured. Use your own Kick account Bearer token.",
                "Login não configurado. Use o token Bearer da sua própria conta Kick."))
                .size(11.0).color(MUTED));
        }
        if !running {
            ui.horizontal(|ui| {
                ui.label(RichText::new(kick_label(br, "Token", "Token")).size(10.0).color(MUTED));
                ui.add(egui::TextEdit::singleline(&mut app.kick_api_token_input[i])
                    .password(true).hint_text(kick_label(br,
                        "Paste to sign in or replace saved login",
                        "Cole para entrar ou substituir o login salvo"))
                    .desired_width(ui.available_width().min(550.0)));
            });
            ui.checkbox(&mut app.kick_remember_login[i], kick_label(br,
                "Remember login securely (OS vault)", "Salvar login com proteção do sistema"));
            ui.horizontal_wrapped(|ui| {
                ui.hyperlink_to(kick_label(br, "Open Kick login ↗", "Abrir login da Kick ↗"),
                    "https://kick.com/");
                if app.kick_saved_login[i] &&
                    ui.small_button(kick_label(br, "Forget saved login", "Esquecer login salvo")).clicked()
                {
                    app.forget_kick_account(i);
                }
            });
            ui.label(RichText::new(kick_label(br,
                "Automatic Kick sign-in is not available. The Bearer token is a secret; never share it.",
                "Login automático da Kick ainda não existe. O token Bearer é secreto; não o compartilhe."))
                .size(10.0).color(DIM));
        } else {
            ui.label(RichText::new(kick_label(br,
                "The token entry stays hidden while streaming.",
                "O campo do token fica oculto enquanto as transmissões estão ativas."))
                .size(10.0).color(MUTED));
        }
        ui.add_space(10.0);
        ui.label(RichText::new(kick_label(br, "2 · Channels", "2 · Canais"))
            .strong().size(14.0).color(TEXT));
        if configured.is_empty() {
            ui.label(RichText::new(kick_label(br,
                "No channels configured. Add a Kick channel below.",
                "Nenhum canal configurado. Adicione um canal abaixo."))
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
                                egui::Button::new("×").small()).clicked() {
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
                "Up to 12 channels per account. Changes save automatically.",
                "Até 12 canais por conta. Alterações salvas automaticamente."))
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
            } else if ui.add_enabled(!configured.is_empty(),
                egui::Button::new(kick_label(br, "▶ Start streams", "▶ Iniciar transmissões")))
                .clicked()
            {
                app.start_kick_account(i);
            }
        });
    });

    ui.add_space(12.0);
    ui.label(RichText::new(kick_label(br, "Channel activity", "Atividade dos canais"))
        .size(15.0).strong().color(TEXT));
    ui.label(RichText::new(kick_label(br,
        "* Gains require a real point-balance increase. A connection alone is not proof of earning.",
        "* Ganhos exigem aumento real do saldo. Uma conexão ativa não confirma pontos."))
        .size(10.0).color(MUTED));
    ui.add_space(8.0);
    ui.vertical(|ui| {
        if let Some(snapshot) = &monitor {
            let cols = if ui.available_width() >= 850.0 { 2 } else { 1 };
            let gap = 12.0;
            let width = if cols == 2 {
                (ui.available_width() - gap) / 2.0
            } else { ui.available_width() };
            for row in snapshot.channels.chunks(cols) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for channel in row {
                        ui.allocate_ui_with_layout(egui::vec2(width, 0.0),
                            Layout::top_down(Align::Min), |ui| {
                            kick_stream_card(ui, channel, width);
                        });
                    }
                });
                ui.add_space(10.0);
            }
        } else if !configured.is_empty() {
            ui.label(RichText::new(kick_label(br,
                "Start streams to display connection health and point balances.",
                "Inicie as transmissões para ver conexões e saldos de pontos."))
                .size(11.0).color(MUTED));
        } else {
            ui.label(RichText::new(kick_label(br,
                "Add your first channel to begin.", "Adicione seu primeiro canal para começar."))
                .size(11.0).color(MUTED));
        }
    });
    });
}
