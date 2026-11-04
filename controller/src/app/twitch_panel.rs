//! Twitch workspace follows the same Accounts layout language as Kick.
//! This page only reads cached miner snapshots; no HTTP or worker polling runs in egui.
use super::*;

fn label(br: bool, en: &'static str, pt: &'static str) -> &'static str {
    if br { pt } else { en }
}

fn metric(ui: &mut egui::Ui, name: &str, value: String, color: Color32) {
    ui.vertical(|ui| {
        ui.label(RichText::new(name).size(10.0).color(MUTED));
        ui.label(RichText::new(value).size(19.0).strong().color(color));
    });
}

fn phase_label(phase: moth_twitch::MinerPhase, br: bool) -> (&'static str, Color32) {
    use moth_twitch::MinerPhase::*;
    match phase {
        Online => (label(br, "● Miner connected", "● Minerador conectado"), GOOD),
        Connecting => (label(br, "◌ Connecting", "◌ Conectando"), WARN),
        Reconnecting => (label(br, "◌ Reconnecting", "◌ Reconectando"), WARN),
        NeedsLogin => (label(br, "○ Sign-in required", "○ Login necessário"), WARN),
        Failed => (label(br, "● Miner error", "● Erro no minerador"), BAD),
        Stopped => (label(br, "○ Stopped", "○ Parado"), MUTED),
    }
}

fn open_channel(profile: GameProfile, login: &str) -> Result<(), String> {
    let config = Config::for_profile(profile)?;
    firefox::open_twitch_stream(&config, login)
}

fn channel_row(ui: &mut egui::Ui, channel: &moth_twitch::StreamerState,
               favorite: bool, br: bool) -> (bool,bool) {
    let recent = channel.last_watch_signal
        .is_some_and(|at| at.elapsed() < Duration::from_secs(125));
    let checked = channel.last_update
        .is_some_and(|at| at.elapsed() < Duration::from_secs(180));
    let mut open = false;
    let mut toggle_favorite=false;
    egui::Frame::new().fill(PANEL_ALT).stroke(Stroke::new(1.0, BORDER))
        .corner_radius(7.0).inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                toggle_favorite=ui.small_button(if favorite {"★"}else{"☆"})
                    .on_hover_text(label(br,"Toggle favorite streamer",
                        "Marcar ou desmarcar streamer favorito")).clicked();
                ui.label(RichText::new(&channel.login).size(12.0).strong().color(TEXT));
                ui.label(RichText::new(if channel.live {
                    label(br,"● LIVE","● AO VIVO")
                } else {
                    label(br,"○ Offline","○ Offline")
                }).size(10.0).color(if channel.live {GOOD}else{MUTED}));
                if channel.watching {
                    ui.label(RichText::new(label(br,
                        "▶ MINING SLOT", "▶ VAGA MINERANDO"))
                        .size(10.0).strong().color(GOOD));
                }
                ui.label(RichText::new(if recent {
                    label(br, "● Watch signal sent", "● Sinal enviado")
                } else if checked {
                    label(br, "◌ Checking", "◌ Verificando")
                } else {
                    label(br, "○ Awaiting data", "○ Aguardando dados")
                }).size(10.0).color(if recent { GOOD } else if checked { WARN } else { MUTED }));
                if channel.live {
                    let (streak_text,streak_color)=if channel.streak_confirmed==Some(true) {
                        (label(br,"✓ Streak collected","✓ Sequência coletada"),GOOD)
                    } else if channel.streak_priority_exhausted {
                        (label(br,"○ Streak not confirmed","○ Sequência não confirmada"),WARN)
                    } else if channel.watching {
                        (label(br,"◌ Collecting streak","◌ Coletando sequência"),WARN)
                    } else {
                        (label(br,"◌ Waiting for streak slot","◌ Aguardando vaga de sequência"),MUTED)
                    };
                    ui.label(RichText::new(streak_text).size(10.0).color(streak_color));
                }
                let points = channel.points
                    .map(|p| p.to_string()).unwrap_or_else(|| "—".into());
                ui.label(RichText::new(format!("{}: {points}",
                    label(br, "Balance", "Saldo")))
                    .size(11.0).color(if channel.points.is_some() { TEXT } else { MUTED }));
                if channel.points_gained > 0 {
                    ui.label(RichText::new(format!(
                        "+{} {}",
                        channel.points_gained,
                        label(br, "from Moth watch", "pela visualização do Moth")
                    )).size(11.0).color(GOOD));
                }
                open = ui.small_button("↗").on_hover_text(label(br,
                    "Open Twitch channel in this account's Firefox profile",
                    "Abrir canal Twitch no perfil Firefox desta conta")).clicked();
            });
            if let Some(error) = channel.error.as_ref() {
                ui.label(RichText::new(error).size(10.0).color(WARN));
            }
            ui.collapsing(label(br, "Technical details", "Detalhes técnicos"), |ui| {
                if channel.live && channel.streak_priority_exhausted
                    && channel.streak_confirmed!=Some(true) {
                    ui.label(RichText::new(label(br,
                        "Reference streak-priority window ended without Twitch confirmation. Farming continues normally and RewardList/PubSub can still confirm it later.",
                        "A janela de prioridade de sequência terminou sem confirmação da Twitch. A mineração continua normalmente e RewardList/PubSub ainda podem confirmar depois."))
                        .size(10.0).color(WARN));
                }
                if let Some(length)=channel.streak_length {
                    ui.label(format!("{}: {length}",label(br,"Watch streak","Sequência")));
                }
                if let Some(at)=channel.streak_checked {
                    ui.label(format!("{}: {} s",
                        label(br,"Streak verified","Sequência verificada há"),
                        at.elapsed().as_secs()));
                }
                if let Some(at) = channel.last_update {
                    ui.label(format!("{}: {} s", label(br, "Last check", "Última consulta"),
                        at.elapsed().as_secs()));
                }
                if let Some(at) = channel.last_media_activity {
                    ui.label(format!("{}: {} s", label(br, "Last HLS media activity",
                        "Última atividade de mídia HLS"), at.elapsed().as_secs()));
                }
                if let Some(at) = channel.last_watch_signal {
                    ui.label(format!("{}: {} s", label(br, "Last acknowledged watch signal",
                        "Último sinal de visualização aceito"), at.elapsed().as_secs()));
                }
                if channel.channel_id.is_some() {
                    ui.label(RichText::new(label(br, "Channel ID resolved",
                        "ID do canal identificado")).size(10.0).color(MUTED));
                }
                ui.label(RichText::new(label(br,
                    "Watch signals are HTTP acknowledgements, not proof that Twitch credited points.",
                    "Sinais enviados são confirmações HTTP, não comprovam crédito de pontos pela Twitch."))
                    .size(10.0).color(MUTED));
            });
        });
    (open,toggle_favorite)
}

impl ControllerApp {
    fn toggle_twitch_favorite(&mut self, index:usize, login:&str) {
        let Some(login)=moth_twitch::normalize_login(login) else {return};
        let mut next=self.workspace.twitch_favorites.clone();
        if let Some(position)=next[index].iter().position(|item|item==&login) {
            next[index].remove(position);
        } else if next[index].len()<128 {
            next[index].push(login);
        } else {
            self.set_status(label(pt_br(),"Too many favorite channels",
                "Muitos canais favoritos"),true);
            return;
        }
        if let Err(error)=Config::save_twitch_favorites(&next) {
            self.set_status(error,true);
            return;
        }
        self.workspace.twitch_favorites=next;
        if let Some(game)=self.runtimes[index].game.native.as_ref() {
            game.set_twitch_favorites(self.workspace.twitch_favorites[index].clone());
        }
    }
    fn forget_twitch_account(&mut self,index:usize) {
        let Some(profile)=GameProfile::from_index(index) else{return};
        if let Err(error)=crate::native_twitch_login::clear_browser_session(profile) {
            self.set_status(error,true);
            return;
        }
        if let Err(error)=crate::native_auth::clear_twitch_token(profile) {
            self.set_status(error,true);
            return;
        }
        if let Some(game)=self.runtimes[index].game.native.as_ref() {
            game.forget_twitch_login();
        }
        crate::native_twitch_login::reset(profile);
        self.runtimes[index].twitch.login_done_seen=false;
        self.set_status(label(pt_br(),"Twitch login removed","Login Twitch removido"),false);
    }

}

pub(super) fn draw_twitch_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let br = pt_br();
    egui::ScrollArea::vertical().id_salt("twitch_workspace")
        .auto_shrink([false, true]).show(ui, |ui| {
        ui.label(RichText::new(label(br, "Twitch Miner", "Minerador Twitch"))
            .font(FontId::proportional(22.0)).strong().color(TEXT));
        ui.label(RichText::new(label(br,
            "Native channel points mining, separate from PokéIdle authentication.",
            "Mineração nativa de pontos do canal, independente do login do PokéIdle."))
            .size(11.0).color(MUTED));
        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            for index in 0..4 {
                let selected = app.workspace.twitch_selected_account == index;
                let active = app.runtimes[index].game.native.as_ref()
                    .and_then(|game| game.twitch_miner_snapshot())
                    .is_some_and(|state| state.phase == moth_twitch::MinerPhase::Online);
                let title = format!("{}{}", app.accounts[index].name,
                    if active { " ●" } else { "" });
                if ui.selectable_label(selected, title).clicked() {
                    app.workspace.twitch_selected_account = index;
                }
            }
        });
        ui.add_space(10.0);

        let index = app.workspace.twitch_selected_account.min(3);
        let profile = GameProfile::from_index(index).expect("valid Twitch account");
        let login_status=crate::native_twitch_login::status(profile);
        let (has_game, snapshot, game_state) = {
            let game = app.runtimes[index].game.native.as_ref();
            (game.is_some(),
                game.and_then(|session| session.twitch_miner_snapshot()),
                game.map(|session| session.snapshot()))
        };
        let connected = snapshot.as_ref()
            .is_some_and(|state| state.phase == moth_twitch::MinerPhase::Online);
        let phase = snapshot.as_ref().map(|state| state.phase);
        let (status, tone) = phase
            .map(|phase| phase_label(phase, br))
            .unwrap_or((label(br, "○ Optional sign-in", "○ Login opcional"), MUTED));
        let desired = snapshot.as_ref().map_or(0, |state| state.desired);
        let available = snapshot.as_ref().map_or(0, |state| state.active);
        let watch_signals = snapshot.as_ref().map_or(0, |state| state.watch_signals);
        let gained = snapshot.as_ref().map_or(0, |state| state.points_gained);
        let claimed = snapshot.as_ref().map_or(0, |state| state.bonuses_claimed);
        let (streaks_collected, streaks_live)=snapshot.as_ref().map(|state| {
            let eligible=state.streamers.iter().filter(|channel|channel.live);
            let live=eligible.clone().count();
            let confirmed=eligible.filter(|channel|channel.streak_confirmed==Some(true)).count();
            (confirmed,live)
        }).unwrap_or((0,0));
        let mut live_names=snapshot.as_ref().map(|state|
            state.streamers.iter().filter(|channel|channel.live)
                .map(|channel|channel.login.clone()).collect::<Vec<_>>()
        ).unwrap_or_default();
        live_names.sort_unstable();
        live_names.dedup();
        let mut mining_names=snapshot.as_ref().map(|state|
            state.streamers.iter().filter(|channel|channel.watching)
                .map(|channel|channel.login.clone()).collect::<Vec<_>>()
        ).unwrap_or_default();
        mining_names.sort_unstable();
        mining_names.dedup();

        egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER))
            .corner_radius(9.0).inner_margin(12.0).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&app.accounts[index].name)
                    .size(15.0).strong().color(TEXT));
                ui.label(RichText::new(status).size(10.0).color(tone));
            });
            ui.add_space(8.0);
            ui.columns(4, |cols| {
                metric(&mut cols[0], label(br, "Live now", "Ao vivo agora"),
                    live_names.len().to_string(), if live_names.is_empty() {MUTED}else{GOOD});
                metric(&mut cols[1], label(br, "Mining slots", "Vagas minerando"),
                    format!("{}/2",mining_names.len()),
                    if mining_names.len()==2 {GOOD}else if mining_names.is_empty(){MUTED}else{WARN});
                metric(&mut cols[2], label(br, "Streaks confirmed", "Sequências confirmadas"),
                    format!("{streaks_collected}/{streaks_live}"),
                    if streaks_collected > 0 {GOOD}else{MUTED});
                metric(&mut cols[3], label(br, "Moth watch points", "Pontos via Moth"),
                    format!("+{gained}"), if gained > 0 { GOOD } else { TEXT });
            });
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(label(br,"● LIVE NOW","● AO VIVO AGORA"))
                    .size(10.0).strong().color(if live_names.is_empty(){MUTED}else{GOOD}));
                ui.label(RichText::new(if live_names.is_empty() {
                    label(br,"None detected","Nenhuma detectada").to_owned()
                } else {
                    live_names.join(" · ")
                }).size(10.0).color(TEXT));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(label(br,"▶ MINING NOW","▶ MINERANDO AGORA"))
                    .size(10.0).strong().color(if mining_names.is_empty(){MUTED}else{GOOD}));
                ui.label(RichText::new(if mining_names.is_empty() {
                    label(br,"No watch slot selected yet","Nenhuma vaga selecionada ainda").to_owned()
                } else {
                    mining_names.join(" · ")
                }).size(10.0).strong().color(if mining_names.is_empty(){MUTED}else{TEXT}));
            });
            ui.label(RichText::new(format!(
                "{}: {available}/{desired} · {}: {claimed} · {}: {watch_signals}",
                label(br, "Channels checked", "Canais consultados"),
                label(br, "Bonus chests claimed", "Baús de bônus coletados"),
                label(br, "Watch signals acknowledged", "Sinais de visualização aceitos")
            )).size(10.0).color(MUTED));
            if !live_names.is_empty() && mining_names.is_empty() {
                ui.label(RichText::new(label(br,
                    "Live streams are detected, but no mining slot is established yet. Stream metadata/watch setup is still pending or failed; check the channel rows for errors.",
                    "Há lives detectadas, mas nenhuma vaga de mineração foi estabelecida. Os metadados/configuração de visualização ainda estão pendentes ou falharam; confira os erros nos canais."))
                    .size(10.0).color(WARN));
            }
            ui.label(RichText::new(label(br,
                "Moth watch points count only Twitch WATCH/WATCH_STREAK events after Moth has sent watch activity for that channel. Balance increases, pending bonus claims, and activity from other Twitch clients are not counted as Moth-mined points.",
                "Pontos via Moth contam apenas eventos WATCH/WATCH_STREAK da Twitch após o Moth enviar atividade de visualização para o canal. Aumentos de saldo, bônus pendentes e atividade de outros clientes Twitch não são contados como pontos minerados pelo Moth."))
                .size(9.0).color(DIM));
        });

        if let Some(state) = snapshot.as_ref() {
            if let Some(error) = state.error.as_deref() {
                ui.add_space(6.0);
                ui.label(RichText::new(error).size(11.0).color(WARN));
            }
        }

        ui.add_space(10.0);
        egui::Frame::new().fill(PANEL_ALT).stroke(Stroke::new(1.0, BORDER))
            .corner_radius(8.0).inner_margin(12.0).show(ui, |ui| {
            ui.label(RichText::new(label(br, "Twitch connection", "Conexão Twitch"))
                .size(13.0).strong().color(TEXT));
            ui.label(RichText::new(if connected {
                label(br, "Authenticated. Moth uses the same Twitch client identity and device-code flow as the reference channel-points miner; Firefox import remains available as a fallback.",
                    "Autenticado. O Moth usa a mesma identidade de cliente Twitch e fluxo por código de dispositivo do minerador de referência; a importação pelo Firefox continua disponível como alternativa.")
            } else if has_game {
                label(br, "Recommended: sign in with Twitch's device code, matching the reference miner. Firefox import is available when you want to reuse an existing browser session.",
                    "Recomendado: entre com o código de dispositivo da Twitch, igual ao minerador de referência. A importação pelo Firefox fica disponível para reutilizar uma sessão existente do navegador.")
            } else {
                label(br, "Sign in with Twitch's device code first. The token is stored in Moth's protected credential vault; start PokéIdle afterward to receive the official streamer roster.",
                    "Primeiro entre com o código de dispositivo da Twitch. O token é salvo no cofre protegido do Moth; depois inicie o PokéIdle para receber a lista oficial de streamers.")
            }).size(11.0).color(MUTED));
            if let Some(state)=snapshot.as_ref() {
                if connected {
                    ui.label(RichText::new(if state.reference_client {
                        label(br,"✓ Reference Twitch client session",
                            "✓ Sessão com cliente Twitch de referência")
                    } else {
                        label(br,"◌ Firefox compatibility session",
                            "◌ Sessão de compatibilidade via Firefox")
                    }).size(10.0).color(if state.reference_client {GOOD}else{WARN}));
                }
                let pubsub_text=if state.pubsub_connected {
                    label(br,"● Channel-points events connected",
                        "● Eventos de pontos conectados")
                } else {
                    label(br,"○ Channel-points events reconnecting; polling fallback remains active",
                        "○ Eventos de pontos reconectando; consulta periódica continua ativa")
                };
                ui.label(RichText::new(pubsub_text).size(10.0)
                    .color(if state.pubsub_connected {GOOD}else{WARN}));
                if let Some(error)=state.pubsub_error.as_deref() {
                    ui.label(RichText::new(error).size(10.0).color(WARN));
                }
            }
            ui.add_space(5.0);
            let busy=matches!(login_status,crate::native_twitch_login::LoginStatus::Running);
            if busy {
                ui.horizontal_wrapped(|ui| {
                    if let Some(code)=crate::native_twitch_login::activation_code(profile) {
                        ui.label(RichText::new(format!(
                            "{}: {code}",
                            label(br,"Enter at twitch.tv/activate","Digite em twitch.tv/activate")
                        )).size(12.0).strong().color(WARN));
                    } else {
                        ui.label(RichText::new(label(br,
                            "Complete the Twitch sign-in…","Conclua o login da Twitch…"))
                            .size(11.0).color(WARN));
                    }
                    if ui.small_button(label(br,"Cancel","Cancelar")).clicked() {
                        crate::native_twitch_login::cancel(profile);
                    }
                });
            } else {
                let has_login=snapshot.is_some() ||
                    matches!(login_status,crate::native_twitch_login::LoginStatus::Done);
                ui.horizontal_wrapped(|ui| {
                    if ui.button(label(br,
                        if has_login {"Refresh with Twitch code"} else {"Sign in with Twitch code"},
                        if has_login {"Renovar com código Twitch"} else {"Entrar com código Twitch"}
                    )).on_hover_text(label(br,
                        "Recommended. Uses Twitch's Android TV device-code flow, matching the reference channel-points miner.",
                        "Recomendado. Usa o fluxo de código de dispositivo Android TV da Twitch, igual ao minerador de referência."
                    )).clicked() {
                        app.runtimes[index].twitch.login_done_seen=false;
                        if let Err(error)=crate::native_twitch_login::start_reference(profile) {
                            app.set_status(error,true);
                        }
                    }
                    if ui.button(label(br,
                        if has_login {"Refresh via Firefox"} else {"Import Firefox login"},
                        if has_login {"Renovar via Firefox"} else {"Importar login do Firefox"}
                    )).on_hover_text(label(br,
                        if has_login {
                            "Starts a fresh Firefox Twitch login and imports the resulting auth-token."
                        } else {
                            "Reuses an existing Twitch auth-token from this account's Firefox profile, or opens Firefox if none is present."
                        },
                        if has_login {
                            "Inicia um novo login Twitch pelo Firefox e importa o auth-token resultante."
                        } else {
                            "Reutiliza um auth-token Twitch existente no perfil Firefox desta conta, ou abre o Firefox se não houver um."
                        }
                    )).clicked() {
                        app.runtimes[index].twitch.login_done_seen=false;
                        let result=if has_login {
                            crate::native_twitch_login::start_fresh(profile)
                        } else {
                            crate::native_twitch_login::start(profile)
                        };
                        if let Err(error)=result {
                            app.set_status(error,true);
                        }
                    }
                    if ui.small_button(label(br,
                        "Firefox BiDi fallback",
                        "Alternativa Firefox BiDi"
                    )).on_hover_text(label(br,
                        "Reads the same Firefox auth cookie through the browser protocol when direct cookies.sqlite access is troublesome.",
                        "Lê o mesmo cookie de autenticação pelo protocolo do navegador quando o acesso direto ao cookies.sqlite apresenta problemas."
                    )).clicked() {
                        app.runtimes[index].twitch.login_done_seen=false;
                        if let Err(error)=crate::native_twitch_login::start_alternative(profile) {
                            app.set_status(error,true);
                        }
                    }
                    if has_login && ui.small_button(label(br,"Forget login","Esquecer login")).clicked() {
                        app.forget_twitch_account(index);
                    }
                });
            }
            ui.label(RichText::new(label(br,
                "Device-code sign-in gives Moth the same Twitch Client-ID/token pairing used by the reference miner. Firefox methods are compatibility fallbacks.",
                "O login por código fornece ao Moth o mesmo par Client-ID/token usado pelo minerador de referência. Os métodos Firefox são alternativas de compatibilidade."
            )).size(10.0).color(MUTED));
                        if let crate::native_twitch_login::LoginStatus::Error(error)=&login_status {
                ui.label(RichText::new(localize_status(error.clone())).size(10.0)
                    .color(if error=="Twitch login cancelled"{MUTED}else{WARN}));
            }
            if let Some(game_state) = game_state.as_ref() {
                let linked = game_state.twitch_linked.unwrap_or(false);
                ui.label(RichText::new(format!("{}: {}{}",
                    label(br, "PokéIdle linked Twitch", "Twitch vinculada ao PokéIdle"),
                    if linked { label(br, "Yes", "Sim") } else { label(br, "Not confirmed", "Não confirmado") },
                    game_state.twitch_login.as_ref()
                        .map(|name| format!(" · {name}")).unwrap_or_default()
                )).size(10.0).color(if linked { GOOD } else { MUTED }));
            }
        });

        ui.add_space(10.0);
        ui.label(RichText::new(label(br, "Channels", "Canais"))
            .size(15.0).strong().color(TEXT));
        ui.label(RichText::new(label(br,
            "All official Twitch streamers are listed, including offline channels. Favorites are preferred after pending streaks. Two watch slots are fixed; all streamer balances are checked when authenticated.",
            "Todos os streamers oficiais são listados, inclusive offline. Favoritos têm prioridade após sequências pendentes. Duas vagas de visualização fixas; todos os saldos são consultados com autenticação."))
            .size(10.0).color(MUTED));
        ui.add_space(6.0);
        {
            let favorites=app.workspace.twitch_favorites[index].clone();
            let mut channels=std::collections::BTreeMap::<String,moth_twitch::StreamerState>::new();
            // Server-authoritative catalog stays visible even before Twitch
            // authentication has completed or the miner has polled balances.
            if let Some(state)=game_state.as_ref() {
                if let Some(official)=state.twitch_official_roster.as_ref() {
                    for login in official {
                        channels.insert(login.clone(),moth_twitch::StreamerState {
                            login:login.clone(), ..Default::default()
                        });
                    }
                }
            }
            for name in &favorites {
                channels.entry(name.clone()).or_insert_with(||
                    moth_twitch::StreamerState {
                        login:name.clone(), ..Default::default()
                    });
            }
            if let Some(snapshot)=snapshot.as_ref() {
                for streamer in &snapshot.streamers {
                    channels.insert(streamer.login.clone(),streamer.clone());
                }
            }
            // Live/offline state comes from moth-twitch's direct Twitch checks.
            // PokéIdle's legacy live roster is no longer authoritative.
            if channels.is_empty() {
                ui.label(RichText::new(label(br,
                    "No official streamer catalog yet. Connect PokéIdle to discover channels.",
                    "Ainda não há catálogo oficial. Conecte o PokéIdle para descobrir canais."))
                    .size(11.0).color(MUTED));
            }
            let mut rows=channels.values().collect::<Vec<_>>();
            rows.sort_by(|a,b| {
                b.watching.cmp(&a.watching)
                    .then_with(||b.live.cmp(&a.live))
                    .then_with(||favorites.contains(&b.login).cmp(&favorites.contains(&a.login)))
                    .then_with(||a.login.cmp(&b.login))
            });
            let mut offline=0usize;
            for channel in rows {
                if !channel.live {offline+=1;}
                let (open, favorite_clicked)=channel_row(ui, channel,
                    favorites.contains(&channel.login),br);
                if favorite_clicked {
                    app.toggle_twitch_favorite(index,&channel.login);
                }
                if open {
                    if let Err(error) = open_channel(profile, &channel.login) {
                        app.set_status(error, true);
                    }
                }
                ui.add_space(4.0);
            }
            if offline>0 {
                ui.label(RichText::new(format!("{}: {offline}",
                    label(br,"Offline streamers","Streamers offline")))
                    .size(10.0).color(MUTED));
            }
        }
    });
}
