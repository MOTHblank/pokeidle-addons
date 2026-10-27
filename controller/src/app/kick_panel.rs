//! Dedicated Kick workspace inside Accounts. Overview cards remain small;
//! setup and detailed monitoring live here, instead of nested pop-out dialogs.
use super::*;
use crate::native_auth;

fn kick_label(pt: bool, en: &'static str, br: &'static str) -> &'static str {
    if pt { br } else { en }
}
fn kick_error_ui(error: &str) -> String {
    if pt_br() { translations::kick_error(error.to_owned()) }
    else { error.to_owned() }
}
fn stored_channel_list(app: &ControllerApp, index: usize) -> Vec<String> {
    crate::native_kick::normalize_channels(&app.kick_channels_input[index])
        .unwrap_or_default()
}

/// Favorite order wins the limited viewer slots, followed by other official
/// live channels, then manual channels. Offline official creators never occupy
/// slots; the complete roster remains available in the Kick workspace.
fn select_kick_streams(official_live: &[String], manual: &[String],
                       prefs: &KickStreamPreferences) -> Vec<String> {
    let mut names = Vec::<String>::new();
    for slug in official_live.iter().chain(manual) {
        if !names.contains(slug) { names.push(slug.clone()); }
    }
    // Stable ordering preserves the original official-first order among
    // non-favorites. Favorites follow the order they were starred.
    names.sort_by_key(|slug| {
        let position = prefs.favorites.iter().position(|favorite| favorite == slug);
        (position.is_none(), position.unwrap_or(usize::MAX))
    });
    names.truncate(prefs.max_streams.min(12));
    names
}

pub(super) fn desired_kick_channels(app: &ControllerApp, index: usize) -> Vec<String> {
    let official = if app.kick_discovery_available[index] {
        app.kick_discovered_live[index].as_slice()
    } else { &[] };
    select_kick_streams(official, &stored_channel_list(app, index),
        &app.kick_stream_preferences[index])
}

impl ControllerApp {
    /// Discover Kick's official live channels from the exact same server
    /// state used by the in-browser scanner: state.kick.oficiais.
    /// Browser-bridge telemetry is a fallback during sign-in only.
    pub(super) fn sync_pokeidle_kick_streamers(&mut self) {
        // Native roster changes are checked without cloning the roster.
        // A slower fallback still handles browser sign-in and login recovery.
        const FALLBACK:Duration=Duration::from_secs(5);
        let now=Instant::now();
        for index in 0..4 {
            let stamp=self.games[index].native.as_ref()
                .and_then(|game|game.official_kick_roster_stamp());
            let native_changed=self.kick_native_stamp[index]!=stamp;
            if !native_changed && self.kick_sync_checked[index].is_some_and(|at|
                now.saturating_duration_since(at)<FALLBACK) {continue}
            self.kick_sync_checked[index]=Some(now);
            if native_changed {
                self.kick_native_stamp[index]=stamp;
                self.kick_discovered_roster[index]=if stamp.is_some() {
                    self.games[index].native.as_ref()
                        .and_then(|game|game.official_kick_roster())
                } else {None};
            }
            let found=if let Some(roster)=self.kick_discovered_roster[index].as_ref() {
                Some(roster.iter().filter(|creator|creator.live)
                    .map(|creator|creator.slug.clone()).collect::<Vec<_>>())
            } else {
                // The browser telemetry path is only used before native
                // handoff; don't demand a browser for native game operation.
                let health=self.games[index].health();
                if health.kick_state_available && health.telemetry_is_fresh() {
                    let urls=health.kick_streams.iter()
                        .map(|item|item.url.as_str()).collect::<Vec<_>>().join(",");
                    Some(if urls.is_empty() {Vec::new()} else {
                        crate::native_kick::normalize_channels(&urls).unwrap_or_default()
                    })
                } else {None}
            };
            let available=found.is_some();
            let channels=found.unwrap_or_default();
            let roster_changed=self.kick_discovery_available[index]!=available ||
                self.kick_discovered_live[index]!=channels;
            if roster_changed {
                self.kick_discovery_available[index]=available;
                self.kick_discovered_live[index]=channels;
            }
            let Some(profile)=GameProfile::from_index(index) else {continue};
            if !self.kick_saved_login[index] && matches!(
                crate::native_kick_login::status(profile),
                crate::native_kick_login::LoginStatus::Done
            ) {self.kick_saved_login[index]=true;}
            let monitor_missing=self.kick_monitors[index].is_none();
            if self.kick_saved_login[index] && monitor_missing {
                self.start_kick_account(index);
            }
            let desired=roster_changed.then(||desired_kick_channels(self,index));
            let sync_offline=native_changed || roster_changed || monitor_missing;
            let remaining=if sync_offline && self.kick_monitors[index].is_some() {
                self.kick_discovered_roster[index].as_ref()
                    .map(|roster|roster.iter().map(|c|c.slug.clone()).collect::<Vec<_>>())
                    .unwrap_or_default()
            } else {Vec::new()};
            if let Some(monitor)=self.kick_monitors[index].as_mut() {
                let outcome=desired.as_ref().map(|names|monitor.sync_channels(names))
                    .transpose().and_then(|_| {
                        if sync_offline {monitor.sync_offline_officials(&remaining)}
                        else {Ok(())}
                    });
                if let Err(error)=outcome {
                    logging::warn(&format!("Kick channel synchronization failed: {error}"));
                    self.set_status(error,true);
                }
            }
        }
    }

    fn persist_kick_channels(&mut self, index: usize) {
        match Config::save_kick_channels(&self.kick_channels_input,
            &self.kick_stream_preferences) {
            Ok(()) => self.set_status(
                kick_label(pt_br(), "Kick preferences saved", "Preferências Kick salvas"), false),
            Err(error) => self.set_status(error, true),
        }
        // Channel preferences remain editable during automatic monitoring.
        // Apply changes immediately without restarting unaffected streams.
        self.kick_display_cache[index]=KickDisplayCache::default();
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
    fn toggle_kick_favorite(&mut self, index: usize, slug: &str) {
        let favorites = &mut self.kick_stream_preferences[index].favorites;
        if let Some(position) = favorites.iter().position(|name| name == slug) {
            favorites.remove(position);
        } else if favorites.len() < 128 {
            favorites.push(slug.to_owned());
        } else {
            self.set_status(kick_label(pt_br(), "Too many favorite channels",
                "Muitos canais favoritos"), true);
            return;
        }
        self.persist_kick_channels(index);
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
        // Keep a favorite if this channel is also an official creator.
        let official = self.kick_discovered_roster[index].as_ref()
            .is_some_and(|roster| roster.iter().any(|creator| creator.slug == channel));
        if !official {
            self.kick_stream_preferences[index].favorites.retain(|slug| slug != channel);
        }
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
        self.kick_display_cache[index]=KickDisplayCache::default();
        self.kick_api_token_input[index].clear();
        let mut message = kick_label(pt_br(),
            "Checking Kick access. The connection has not been verified yet.",
            "Verificando o acesso à Kick. A conexão ainda não foi confirmada.").to_owned();
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
        if let Err(reason) = Config::save_kick_channels(&self.kick_channels_input,
            &self.kick_stream_preferences) {
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
                self.kick_display_cache[index]=KickDisplayCache::default();
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
    manual: &[String],
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
    for slug in selected.iter().chain(manual) {
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
// Prepared only when data/selection changes or the five-second display
// refresh expires. A redraw never needs to rebuild the merged channel list.
struct KickDisplay {
    configured:Vec<String>,
    desired:Vec<String>,
    discovered:Vec<String>,
    auto_available:bool,
    saved_login:bool,
    monitor:Option<crate::native_kick::KickSnapshot>,
    channels:Vec<KickChannelRow>,
    online:usize,
    receiving:usize,
    earned:u64,
}
#[derive(Default)]
pub(super) struct KickDisplayCache {
    updated_at:Option<Instant>,
    native_stamp:Option<(Instant,u64)>,
    monitor_identity:Option<Instant>,
    channel_input:String,
    display:Option<Arc<KickDisplay>>,
}
impl KickDisplayCache {
    fn needs_refresh(&self,now:Instant,stamp:Option<(Instant,u64)>,
        monitor_identity:Option<Instant>,input:&str,
        discovered:&[String],auto_available:bool,saved_login:bool)->bool {
        self.updated_at.is_none_or(|at|
            now.saturating_duration_since(at)>=Duration::from_secs(5)) ||
        self.native_stamp!=stamp ||
        self.monitor_identity!=monitor_identity ||
        self.channel_input!=input ||
        self.display.as_ref().is_none_or(|view|
            view.discovered.as_slice()!=discovered ||
            view.auto_available!=auto_available ||
            view.saved_login!=saved_login)
    }
}
fn kick_channel_line(ui: &mut egui::Ui, channel: &KickChannelRow,
                     favorite: bool, selected: bool) -> (bool, bool) {
    let br = pt_br();
    let (state, state_color) = if !selected && channel.live.is_none() {
        (kick_label(br, "○ Not monitored", "○ Não monitorado"), MUTED)
    } else { match channel.live {
        Some(true) => (kick_label(br, "● LIVE", "● AO VIVO"), GOOD),
        Some(false) => (kick_label(br, "○ Offline", "○ Offline"), MUTED),
        None => (kick_label(br, "◌ Checking", "◌ Verificando"), WARN),
    }};
    let points = channel.points.map(|value| value.to_string())
        .unwrap_or_else(|| "—".to_owned());
    let stale = channel.checked_at
        .is_some_and(|checked| checked.elapsed() > Duration::from_secs(300));
    let mut open = false;
    let mut toggle_favorite = false;
    egui::Frame::new().fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(7.0).inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                toggle_favorite = ui.small_button(if favorite { "★" } else { "☆" })
                    .on_hover_text(kick_label(br,
                        "Toggle preferred streamer (favorites fill slots first)",
                        "Alternar streamer favorito (favoritos ocupam as vagas primeiro)"))
                    .clicked();
                ui.label(RichText::new(&channel.slug).size(12.0).strong().color(TEXT));
                ui.label(RichText::new(state).size(10.0).color(state_color));
                if selected {
                    ui.label(RichText::new(kick_label(br, "Selected", "Selecionado"))
                        .size(10.0).color(GOOD));
                } else if channel.live != Some(false) {
                    ui.label(RichText::new(kick_label(br, "Outside stream limit", "Fora do limite"))
                        .size(10.0).color(MUTED));
                }
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
                    .on_hover_text(kick_label(br,
                        "Open channel in your default browser",
                        "Abrir canal no navegador padrão"))
                    .clicked();
            });
            if let Some(error) = &channel.error {
                // Keep errors visible once, without duplicating them inside
                // expanded diagnostics or the account summary.
                ui.label(RichText::new(kick_error_ui(error)).size(10.0).color(WARN));
            }
            if let Some(detail) = &channel.details {
                ui.collapsing(kick_label(br, "Technical details", "Detalhes técnicos"), |ui| {
                    if channel.official {
                        ui.label(RichText::new(kick_label(br,
                            "Official PokéIdle channel", "Canal oficial do PokéIdle"))
                            .size(10.0).color(MUTED));
                    }
                    let watch = &detail.watch;
                    let yes_no = |value: bool| if value {
                        kick_label(br, "yes", "sim")
                    } else {
                        kick_label(br, "no", "não")
                    };
                    ui.label(format!("WebSocket: {} · {}: {}",
                        yes_no(watch.ws_connected),
                        kick_label(br, "handshake", "negociação"),
                        yes_no(watch.handshake_received)));
                    ui.label(format!("HLS: {} {} · {:.1} MiB",
                        watch.media.segments,
                        kick_label(br, "segments", "segmentos"),
                        watch.media.bytes as f64 / 1_048_576.0));
                    ui.label(format!("{}: {} {}",
                        kick_label(br, "Tracking", "Rastreamento"), watch.events_sent,
                        kick_label(br, "events", "eventos")));
                    if let Some(viewers) = detail.viewers {
                        ui.label(format!("{}: {viewers}",
                            kick_label(br, "Viewers", "Espectadores")));
                    }
                    if let Some(error) = &watch.media.error {
                        ui.label(RichText::new(kick_error_ui(error)).size(10.0).color(WARN));
                    }
                    if let Some(checked) = channel.checked_at {
                        ui.label(format!("{} {} s",
                            kick_label(br, "Balance checked", "Saldo consultado há"),
                            checked.elapsed().as_secs()));
                    }
                });
            }
        });
    (open, toggle_favorite)
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
    let now=Instant::now();
    let monitor_identity=app.kick_monitors[index].as_ref().map(|m|m.identity());
    let native_stamp=app.kick_native_stamp[index];
    if app.kick_display_cache[index].needs_refresh(now,native_stamp,monitor_identity,
        &app.kick_channels_input[index],&app.kick_discovered_live[index],
        app.kick_discovery_available[index],app.kick_saved_login[index]) {
        let monitor=app.kick_monitors[index].as_ref().map(|m|m.snapshot());
        let configured=stored_channel_list(app,index);
        let desired=desired_kick_channels(app,index);
        let discovered=app.kick_discovered_live[index].clone();
        let auto_available=app.kick_discovery_available[index];
        let official_roster=app.kick_discovered_roster[index].clone()
            .unwrap_or_else(||discovered.iter().map(|slug|
                crate::native_kick::OfficialKickChannel{
                    slug:slug.clone(),live:true,
                }).collect());
        let offline_balances=app.kick_monitors[index].as_ref()
            .map(|session|session.offline_point_balances()).unwrap_or_default();
        let channels=channel_rows(&desired,&configured,&official_roster,
            monitor.as_ref(),&offline_balances);
        let online=if auto_available {
            desired.iter().filter(|slug|discovered.contains(slug)).count()
        } else {
            monitor.as_ref().map(|snapshot|snapshot.channels.iter()
                .filter(|channel|channel.live==Some(true)).count()).unwrap_or(0)
        };
        let receiving=monitor.as_ref().map(|snapshot|
            snapshot.channels.iter().filter(|channel|
                channel.watch.media.last_segment.is_some_and(|when|
                    when.elapsed()<Duration::from_secs(25))).count()).unwrap_or(0);
        let earned=monitor.as_ref().map(|snapshot|
            snapshot.channels.iter().map(|channel|channel.watch.points_gained).sum())
            .unwrap_or(0);
        app.kick_display_cache[index]=KickDisplayCache{
            updated_at:Some(now),native_stamp,monitor_identity,
            channel_input:app.kick_channels_input[index].clone(),
            display:Some(Arc::new(KickDisplay{
                configured,desired,discovered,auto_available,
                saved_login:app.kick_saved_login[index],
                monitor,channels,online,receiving,earned,
            })),
        };
    }
    // A shared immutable model avoids cloning all rows for the UI and
    // remains valid if the user edits settings in the same frame.
    let display=Arc::clone(app.kick_display_cache[index].display.as_ref()
        .expect("Kick display cache initialized"));
    let monitor=display.monitor.as_ref();
    let running=monitor.is_some();
    let auth_denied=monitor.is_some_and(|snapshot|snapshot.auth_denied);
    let configured=&display.configured;
    let desired=&display.desired;
    let discovered=&display.discovered;
    let auto_available=display.auto_available;
    let channel_list=&display.channels;
    let online=display.online;
    let receiving=display.receiving;
    let earned=display.earned;

    egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER))
        .corner_radius(9.0).inner_margin(12.0).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&app.accounts[index].name)
                .size(15.0).strong().color(TEXT));
            let (status, color) = if auth_denied {
                (kick_label(br, "● Private API blocked", "● API privada bloqueada"), WARN)
            } else if running {
                (kick_label(br, "● Monitor running", "● Monitor em execução"), GOOD)
            } else if app.kick_saved_login[index] {
                (kick_label(br, "○ Waiting to start", "○ Aguardando início"), MUTED)
            } else {
                (kick_label(br, "○ Sign in required", "○ Login necessário"), MUTED)
            };
            ui.label(RichText::new(status).size(10.0).color(color));
        });
        ui.add_space(7.0);
        ui.columns(3, |cols| {
            kick_metric(&mut cols[0], kick_label(br, "Live", "Ao vivo"),
                format!("{online}/{}", desired.len()), if online > 0 { GOOD } else { TEXT });
            kick_metric(&mut cols[1], kick_label(br, "Media receiving", "Recebendo mídia"),
                receiving.to_string(), if receiving > 0 { GOOD } else { MUTED });
            kick_metric(&mut cols[2], kick_label(br, "Points earned", "Pontos ganhos"),
                format!("+{earned}"), if earned > 0 { GOOD } else { TEXT });
        });
        ui.label(RichText::new(kick_label(br,
            "Media receiving counts recent transfers, not confirmed views. Earned points count only observed balance increases.",
            "Recebendo mídia conta transferências recentes, não visualizações confirmadas. Pontos ganhos contam apenas aumentos observados do saldo."))
            .size(9.0).color(DIM));
    });

    if auth_denied {
        ui.add_space(5.0);
        ui.label(RichText::new(if app.kick_saved_login[index] {
            kick_label(br,
                "A private Kick request was blocked (401/403). This may be temporary, not necessarily an expired login. The monitor is paused; the saved login was kept.",
                "Uma solicitação privada da Kick foi bloqueada (401/403). Isso pode ser temporário, não necessariamente um login expirado. O monitor foi pausado; o login salvo foi preservado.")
        } else {
            kick_label(br,
                "A private Kick request was blocked (401/403). This may be temporary. Monitoring is paused; enter the token again to reconnect.",
                "Uma solicitação privada da Kick foi bloqueada (401/403). Isso pode ser temporário. O monitoramento foi pausado; informe o token novamente para reconectar.")
        })
            .size(11.0).color(WARN));
        if app.kick_saved_login[index] {
            if ui.small_button(kick_label(br, "Retry Kick connection", "Tentar conexão Kick novamente")).clicked() {
                app.kick_monitors[index].take();
                app.start_kick_account(index);
            }
        } else if ui.small_button(kick_label(br,
            "Stop and enter token again", "Parar e informar token novamente")).clicked() {
            app.kick_monitors[index].take();
            app.kick_display_cache[index]=KickDisplayCache::default();
        }
    } else if let Some(error) = monitor.as_ref().and_then(|snapshot|
        snapshot.error.as_ref())
    {
        ui.add_space(5.0);
        ui.label(RichText::new(kick_error_ui(error)).size(10.0).color(WARN));
    }

    ui.add_space(10.0);
    if !app.kick_saved_login[index] && !running {
        egui::Frame::new().fill(PANEL_ALT).stroke(Stroke::new(1.0, BORDER))
            .corner_radius(8.0).inner_margin(12.0).show(ui, |ui| {
            ui.label(RichText::new(kick_label(br,
                "Connect your Kick account", "Conecte sua conta Kick"))
                .size(13.0).strong().color(TEXT));
            ui.label(RichText::new(kick_label(br,
                "Enter Kick in the regular Firefox window. When detected, Moth imports the session and closes the temporary window.",
                "Entre na Kick pela janela comum do Firefox. Ao detectar a sessão, o Moth importa o login e fecha a janela temporária."))
                .size(11.0).color(MUTED));
            ui.horizontal_wrapped(|ui| {
                let busy = matches!(login_status,
                    crate::native_kick_login::LoginStatus::Running);
                if busy {
                    ui.label(RichText::new(kick_label(br,
                        "Complete sign-in in Firefox…", "Conclua o login no Firefox…"))
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
                ui.label(RichText::new(kick_error_ui(error)).size(10.0)
                    .color(if error == "Kick login cancelled" { MUTED } else { WARN }));
            }
            ui.collapsing(kick_label(br, "Advanced: enter token manually", "Avançado: informar token manualmente"), |ui| {
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
    } else if running {
        // A manually entered token may be intentionally unsaved. Give the
        // user a way to replace it without restarting the controller.
        if ui.small_button(kick_label(br, "Stop Kick monitoring", "Parar monitoramento Kick")).clicked() {
            app.kick_monitors[index].take();
            app.kick_display_cache[index]=KickDisplayCache::default();
        }
        ui.add_space(8.0);
    }

    ui.label(RichText::new(kick_label(br, "Channels", "Canais"))
        .size(15.0).strong().color(TEXT));
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(kick_label(br,
            "Maximum simultaneous streams", "Máximo de streams simultâneas"))
            .size(11.0).color(TEXT));
        let limit = &mut app.kick_stream_preferences[index].max_streams;
        if ui.add(egui::Slider::new(limit, 0..=12).show_value(true)).changed() {
            app.persist_kick_channels(index);
        }
    });
    ui.label(RichText::new(kick_label(br,
        "Recommended minimum: 2 simultaneous Kick streams. Set 0 to pause all streams.",
        "Mínimo recomendado: 2 streams da Kick simultâneas. Use 0 para pausar todas as streams."))
        .size(10.0).color(MUTED));
    ui.label(RichText::new(kick_label(br,
        "Star preferred streamers. Favorites fill slots in the order you star them; other live official channels fill the rest.",
        "Marque seus streamers favoritos. Eles ocupam vagas na ordem em que foram marcados; outros canais oficiais ao vivo ocupam o restante."))
        .size(10.0).color(MUTED));
    ui.label(RichText::new(kick_label(br,
        "The ↗ Open channel button tries to open Kick in your system's default browser. Kick often rejects connections from browsers used by the app.",
        "O botão ↗ Abrir canal tenta abrir a Kick no navegador padrão do sistema. A Kick frequentemente rejeita conexões pelos navegadores usados pelo aplicativo."))
        .size(10.0).color(MUTED));
    ui.add_space(6.0);

    let active_count = channel_list.iter().filter(|channel|
        channel.live != Some(false)).count();
    let offline_count = channel_list.len().saturating_sub(active_count);
    if active_count == 0 {
        ui.label(RichText::new(if !auto_available && configured.is_empty() {
            kick_label(br,
                "Connect PokéIdle to discover official Kick channels.",
                "Conecte o PokéIdle para descobrir os canais oficiais da Kick.")
        } else if !running {
            kick_label(br,
                "Connect Kick to start channel monitoring.",
                "Conecte a Kick para iniciar o monitoramento dos canais.")
        } else if auth_denied {
            kick_label(br,
                "Channel checks are paused until Kick access is restored.",
                "A verificação de canais está pausada até o acesso à Kick voltar.")
        } else {
            kick_label(br,
                "No live channels confirmed right now. The monitor will keep checking.",
                "Nenhum canal ao vivo confirmado no momento. O monitor continuará verificando.")
        }).size(11.0).color(MUTED));
    }
    for channel in channel_list.iter().filter(|channel| channel.live != Some(false)) {
        let (open, favorite_clicked) = kick_channel_line(ui, channel,
            app.kick_stream_preferences[index].favorites.contains(&channel.slug),
            desired.contains(&channel.slug));
        if favorite_clicked { app.toggle_kick_favorite(index, &channel.slug); }
        if open {
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
                let (open, favorite_clicked) = kick_channel_line(ui, channel,
                    app.kick_stream_preferences[index].favorites.contains(&channel.slug),
                    desired.contains(&channel.slug));
                if favorite_clicked { app.toggle_kick_favorite(index, &channel.slug); }
                if open {
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
            "Official channels come from PokéIdle when connected. Add optional Kick channels here.",
            "Os canais oficiais vêm do PokéIdle quando conectado. Adicione canais extras da Kick aqui."))
            .size(11.0).color(MUTED));
        if !configured.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for slug in configured {
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
            .filter(|name| !discovered.contains(name)).count()
            > app.kick_stream_preferences[index].max_streams {
            ui.label(RichText::new(kick_label(br,
                "More channels available than slots. Favorites take priority.",
                "Há mais canais que vagas. Favoritos têm prioridade."))
                .size(10.0).color(MUTED));
        }
    });
    });
}

#[cfg(test)]
mod channel_row_tests {
    use super::*;
    use crate::native_kick::{OfficialKickChannel, OfflinePoints};

    #[test]
    fn unchanged_kick_display_reuses_its_cached_rows() {
        let now=Instant::now();
        let discovered=vec!["official".to_owned()];
        let mut cache=KickDisplayCache::default();
        assert!(cache.needs_refresh(now,None,None,"",&discovered,true,false));
        cache.updated_at=Some(now);
        cache.display=Some(Arc::new(KickDisplay{
            configured:vec![],desired:vec![],discovered:discovered.clone(),
            auto_available:true,saved_login:false,
            monitor:None,channels:vec![],online:0,receiving:0,earned:0,
        }));
        assert!(!cache.needs_refresh(now+Duration::from_secs(1),None,None,
            "",&discovered,true,false));
        assert!(cache.needs_refresh(now+Duration::from_secs(1),None,None,
            "manual",&discovered,true,false));
        assert!(cache.needs_refresh(now+Duration::from_secs(1),None,None,
            "",&["changed".to_owned()],true,false));
        assert!(cache.needs_refresh(now+Duration::from_secs(1),
            Some((now,1)),None,"",&discovered,true,false));
        assert!(cache.needs_refresh(now+Duration::from_secs(5),None,None,
            "",&discovered,true,false));
    }
    #[test]
    fn one_row_per_official_and_manual_creator_even_when_offline() {
        let official = vec![
            OfficialKickChannel { slug: "offline_one".into(), live: false },
            OfficialKickChannel { slug: "live_two".into(), live: true },
        ];
        let chosen = vec!["live_two".into(), "manual_three".into()];
        let rows = channel_rows(&chosen, &[], &official, None, &Default::default());
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
    fn unselected_manual_channels_remain_visible_and_can_be_favorited() {
        let rows = channel_rows(&["selected".into()],
            &["selected".into(), "waiting".into()], &[], None, &Default::default());
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.slug == "waiting"));
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
        let rows = channel_rows(&[], &[], &official, None, &balances);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].points, Some(125));
        assert_eq!(rows[1].points, Some(800));
    }
}

#[cfg(test)]
mod kick_stream_selection_tests {
    use super::*;

    #[test]
    fn preferred_creators_fill_three_slots_before_others() {
        let live = vec!["alpha".into(), "beta".into(), "gamma".into()];
        let manual = vec!["extra".into(), "beta".into()];
        let prefs = KickStreamPreferences {
            max_streams: 3, favorites: vec!["extra".into(), "gamma".into()],
        };
        assert_eq!(select_kick_streams(&live, &manual, &prefs),
            vec!["extra", "gamma", "alpha"]);
    }
    #[test]
    fn offline_official_favorite_does_not_reserve_a_slot() {
        let prefs = KickStreamPreferences {
            max_streams: 1, favorites: vec!["offline".into()],
        };
        assert_eq!(select_kick_streams(&["live".into()], &[], &prefs),
            vec!["live"]);
    }
    #[test]
    fn favorite_order_is_stable_and_manual_channels_are_deduplicated() {
        let prefs = KickStreamPreferences {
            max_streams: 2, favorites: vec!["third".into(), "first".into()],
        };
        assert_eq!(select_kick_streams(
            &["first".into(), "second".into(), "third".into()],
            &["first".into(), "fourth".into()], &prefs),
            vec!["third", "first"]);
        let defaults = KickStreamPreferences::default();
        assert_eq!(select_kick_streams(&["first".into()], &["first".into(), "other".into()],
            &defaults), vec!["first", "other"]);
    }
    #[test]
    fn zero_limit_stops_all_stream_sessions() {
        let prefs = KickStreamPreferences { max_streams: 0, favorites: vec!["fav".into()] };
        assert!(select_kick_streams(&["fav".into()], &["other".into()], &prefs).is_empty());
    }
    #[test]
    fn limit_never_exceeds_twelve_even_for_invalid_in_memory_settings() {
        let prefs = KickStreamPreferences { max_streams: 999, favorites: vec![] };
        let live = (0..30).map(|n| format!("channel{n}")).collect::<Vec<_>>();
        assert_eq!(select_kick_streams(&live, &[], &prefs).len(), 12);
    }
}
