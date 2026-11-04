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

fn open_channel(login: &str) -> Result<(), String> {
    // Never allow arbitrary URLs or shell interpolation from a server field.
    let login = moth_twitch::normalize_login(login)
        .ok_or_else(|| "Invalid Twitch channel".to_owned())?;
    let mut command = std::process::Command::new(if cfg!(windows) {
        "explorer.exe"
    } else {
        "xdg-open"
    });
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.arg(format!("https://www.twitch.tv/{login}"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open Twitch channel: {error}"))
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
                ui.label(RichText::new(if recent {
                    label(br, "● Watch signal sent", "● Sinal enviado")
                } else if checked {
                    label(br, "◌ Checking", "◌ Verificando")
                } else {
                    label(br, "○ Awaiting data", "○ Aguardando dados")
                }).size(10.0).color(if recent { GOOD } else if checked { WARN } else { MUTED }));
                if channel.live {
                    let (streak_text,streak_color)=match channel.streak_confirmed {
                        Some(true)=>(label(br,"✓ Streak collected","✓ Sequência coletada"),GOOD),
                        Some(false)=>(label(br,"◌ Streak pending","◌ Sequência pendente"),WARN),
                        None=>(label(br,"◌ Streak unverified","◌ Sequência não verificada"),MUTED),
                    };
                    ui.label(RichText::new(streak_text).size(10.0).color(streak_color));
                }
                let points = channel.points
                    .map(|p| p.to_string()).unwrap_or_else(|| "—".into());
                ui.label(RichText::new(format!("{}: {points}",
                    label(br, "Balance", "Saldo")))
                    .size(11.0).color(if channel.points.is_some() { TEXT } else { MUTED }));
                open = ui.small_button("↗").on_hover_text(label(br,
                    "Open Twitch channel in your default browser",
                    "Abrir canal Twitch no navegador padrão")).clicked();
            });
            if let Some(error) = channel.error.as_ref() {
                ui.label(RichText::new(error).size(10.0).color(WARN));
            }
            ui.collapsing(label(br, "Technical details", "Detalhes técnicos"), |ui| {
                if channel.live && channel.streak_confirmed!=Some(true)
                    && channel.streak_watch_attempts>=20 {
                    ui.label(RichText::new(label(br,
                        "Streak not confirmed after watch window; still retrying in rotation.",
                        "Sequência não confirmada após janela de visualização; tentativas continuam em rodízio."))
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

        egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER))
            .corner_radius(9.0).inner_margin(12.0).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&app.accounts[index].name)
                    .size(15.0).strong().color(TEXT));
                ui.label(RichText::new(status).size(10.0).color(tone));
            });
            ui.add_space(8.0);
            ui.columns(3, |cols| {
                metric(&mut cols[0], label(br, "Channels checked", "Canais consultados"),
                    format!("{available}/{desired}"), if available > 0 { GOOD } else { TEXT });
                metric(&mut cols[1], label(br, "Streaks confirmed", "Sequências confirmadas"),
                    format!("{streaks_collected}/{streaks_live}"),
                    if streaks_collected > 0 {GOOD}else{MUTED});
                metric(&mut cols[2], label(br, "Points earned", "Pontos ganhos"),
                    format!("+{gained}"), if gained > 0 { GOOD } else { TEXT });
            });
            ui.label(RichText::new(format!(
                "{}: {claimed} · {}: {watch_signals}",
                label(br, "Bonus chests claimed", "Baús de bônus coletados"),
                label(br, "Watch signals acknowledged", "Sinais de visualização aceitos")
            )).size(10.0).color(MUTED));
            ui.label(RichText::new(label(br,
                "Earned points count observed balance increases only. Watch signals do not prove point credit or active PokéIdle bonuses.",
                "Pontos ganhos contam apenas aumentos observados de saldo. Sinais de visualização não comprovam crédito nem bônus ativos no PokéIdle."))
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
                label(br, "Authenticated in this session. The miner runs without a Twitch browser tab.",
                    "Autenticado nesta sessão. O minerador funciona sem aba da Twitch.")
            } else if has_game {
                label(br, "Twitch is optional. Sign in using this account's isolated Firefox profile, then restart the account to import the session.",
                    "A Twitch é opcional. Entre pelo perfil isolado do Firefox desta conta e reinicie a conta para importar a sessão.")
            } else {
                label(br, "Start PokéIdle to use its official Twitch channel list. Sign in to Twitch separately if you want to mine points.",
                    "Inicie o PokéIdle para usar a lista de canais oficiais da Twitch. Entre na Twitch separadamente para minerar pontos.")
            }).size(11.0).color(MUTED));
            ui.add_space(5.0);
            if ui.button(label(br, "Sign in to Twitch with Firefox",
                "Entrar na Twitch pelo Firefox")).clicked() {
                app.profile_action(profile, ProfileAction::TwitchMiner);
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
            if let Some(state)=game_state.as_ref() {
                if let Some(live)=state.twitch_official_live.as_ref() {
                    for channel in channels.values_mut() {
                        channel.live=live.binary_search(&channel.login).is_ok();
                    }
                }
            }
            if channels.is_empty() {
                ui.label(RichText::new(label(br,
                    "No official streamer catalog yet. Connect PokéIdle to discover channels.",
                    "Ainda não há catálogo oficial. Conecte o PokéIdle para descobrir canais."))
                    .size(11.0).color(MUTED));
            }
            let mut rows=channels.values().collect::<Vec<_>>();
            rows.sort_by(|a,b| {
                b.live.cmp(&a.live)
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
                    if let Err(error) = open_channel(&channel.login) {
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
