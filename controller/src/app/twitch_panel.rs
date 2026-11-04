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
        Online => (label(br, "● Connected", "● Conectado"), GOOD),
        Connecting => (label(br, "◌ Connecting", "◌ Conectando"), WARN),
        Reconnecting => (label(br, "◌ Reconnecting", "◌ Reconectando"), WARN),
        NeedsLogin => (label(br, "○ Sign-in required", "○ Login necessário"), WARN),
        Failed => (label(br, "● Connection error", "● Erro de conexão"), BAD),
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
                        "▶ WATCHING", "▶ ASSISTINDO"))
                        .size(10.0).strong().color(GOOD));
                }
                ui.label(RichText::new(if recent {
                    label(br, "● Active", "● Ativo")
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
                    ui.label(RichText::new(format!("+{}", channel.points_gained))
                        .size(11.0).color(GOOD));
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
                        "Streak-priority window ended without Twitch confirmation. Watching continues normally and Twitch can still confirm it later.",
                        "A janela de prioridade da sequência terminou sem confirmação da Twitch. A visualização continua normalmente e a Twitch ainda pode confirmar depois."))
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
        ui.label(RichText::new(label(br, "Twitch", "Twitch"))
            .font(FontId::proportional(22.0)).strong().color(TEXT));
        ui.label(RichText::new(label(br,
            "Watch official streams and collect Twitch channel points.",
            "Acompanhe as lives oficiais e acumule pontos do canal na Twitch."))
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
                metric(&mut cols[1], label(br, "Watching", "Assistindo"),
                    format!("{}/2",mining_names.len()),
                    if mining_names.len()==2 {GOOD}else if mining_names.is_empty(){MUTED}else{WARN});
                metric(&mut cols[2], label(br, "Streaks confirmed", "Sequências confirmadas"),
                    format!("{streaks_collected}/{streaks_live}"),
                    if streaks_collected > 0 {GOOD}else{MUTED});
                metric(&mut cols[3], label(br, "Points earned", "Pontos ganhos"),
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
                ui.label(RichText::new(label(br,"▶ WATCHING NOW","▶ ASSISTINDO AGORA"))
                    .size(10.0).strong().color(if mining_names.is_empty(){MUTED}else{GOOD}));
                ui.label(RichText::new(if mining_names.is_empty() {
                    label(br,"No stream selected yet","Nenhuma live selecionada ainda").to_owned()
                } else {
                    mining_names.join(" · ")
                }).size(10.0).strong().color(if mining_names.is_empty(){MUTED}else{TEXT}));
            });
            ui.label(RichText::new(format!(
                "{}: {available}/{desired} · {}: {claimed}",
                label(br, "Channels checked", "Canais consultados"),
                label(br, "Bonus chests claimed", "Baús de bônus coletados")
            )).size(10.0).color(MUTED));
            if !live_names.is_empty() && mining_names.is_empty() {
                ui.label(RichText::new(label(br,
                    "Live streams are available, but Moth is not watching one yet. Check the channels below if this persists.",
                    "Há lives disponíveis, mas o Moth ainda não está assistindo a nenhuma. Confira os canais abaixo se isso persistir."))
                    .size(10.0).color(WARN));
            }
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
                label(br, "Twitch connected. Moth can watch streams and collect channel points.",
                    "Twitch conectada. O Moth pode assistir às lives e coletar pontos do canal.")
            } else if has_game {
                label(br, "Sign in to Twitch to collect channel points. Use the code method first, or import your Firefox login.",
                    "Entre na Twitch para coletar pontos do canal. Use primeiro o código ou importe seu login do Firefox.")
            } else {
                label(br, "Sign in to Twitch, then start PokéIdle to load the official channels.",
                    "Entre na Twitch e depois inicie o PokéIdle para carregar os canais oficiais.")
            }).size(11.0).color(MUTED));
            if let Some(state)=snapshot.as_ref() {
                if connected {
                    ui.label(RichText::new(if state.reference_client {
                        label(br,"✓ Signed in with Twitch code",
                            "✓ Login por código Twitch")
                    } else {
                        label(br,"✓ Signed in through Firefox",
                            "✓ Login via Firefox")
                    }).size(10.0).color(GOOD));
                }
                let pubsub_text=if state.pubsub_connected {
                    label(br,"● Point updates connected",
                        "● Atualização de pontos conectada")
                } else {
                    label(br,"○ Reconnecting point updates",
                        "○ Reconectando atualização de pontos")
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
                        "Recommended. Sign in with a code at twitch.tv/activate.",
                        "Recomendado. Entre com um código em twitch.tv/activate."
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
                            "Opens Firefox so you can sign in to Twitch again for this account."
                        } else {
                            "Reuses the Twitch login already available in this account's Firefox profile, or opens Firefox if needed."
                        },
                        if has_login {
                            "Abre o Firefox para você entrar novamente na Twitch nesta conta."
                        } else {
                            "Reutiliza o login da Twitch já disponível no perfil Firefox desta conta ou abre o Firefox se necessário."
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
                        "Alternative Firefox import",
                        "Importação alternativa do Firefox"
                    )).on_hover_text(label(br,
                        "Try this if the regular Firefox import fails.",
                        "Tente esta opção se a importação normal do Firefox falhar."
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
                "Use the code method first. If it fails, try one of the Firefox options.",
                "Use primeiro o método por código. Se falhar, tente uma das opções do Firefox."
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
            "All official Twitch streamers are listed, including offline channels. Favorites are preferred after pending streaks. Moth watches up to two streams at once.",
            "Todos os streamers oficiais são listados, inclusive offline. Favoritos têm prioridade após sequências pendentes. O Moth assiste a até duas lives ao mesmo tempo."))
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
