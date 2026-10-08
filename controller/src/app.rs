use crate::accounts;
use crate::config::{AccountConfig, Config, GameProfile};
use crate::firefox;
use crate::logging;
use crate::monitor::{Health, MonitorHandle};
use serde_json::json;
use eframe::egui::{self, Align, Color32, FontId, Layout, Margin, RichText, Stroke, TextStyle};
use std::process::Child;


use std::sync::atomic::{AtomicBool, Ordering};

static PT_BR: AtomicBool = AtomicBool::new(true);

fn pt_br() -> bool {
    PT_BR.load(Ordering::Relaxed)
}

fn localize_status(message: String) -> String {
    if !pt_br() {
        return message;
    }

    let mut value = message;
    for (en, pt) in [
        ("Ready · launch only the profiles you need", "Pronto · inicie apenas os perfis de que precisa"),
        (" is already running.", " já está em execução."),
        (" is already stopped.", " já está parado."),
        (" Firefox closed.", " Firefox fechado."),
        (" could not close Firefox: ", " não foi possível fechar o Firefox: "),
        ("Launch Both requested · monitoring will update as Firefox becomes ready", "Iniciar ambos solicitado · o monitor atualizará quando o Firefox estiver pronto"),
        ("All Firefox instances closed.", "Todas as instâncias do Firefox foram fechadas."),
        ("Opened controller log · ", "Log do controlador aberto · "),
        ("Opened profile folder", "Pasta do perfil aberta"),
        ("Opened game", "Jogo aberto"),
        ("Opened profile folder", "Pasta do perfil aberta"),
        ("Opened ", "Aberto: "),
        ("Could not ", "Não foi possível "),
        (" is not running.", " não está em execução."),
        ("market refresh requested", "atualização do mercado solicitada"),
        ("loading listings to buy ", "carregando ofertas para comprar "),
        ("buy command sent for ", "comando de compra enviado para "),
        ("changing hunt to ", "mudando a caça para "),
        ("switching Firefox to ", "mudando o Firefox para o modo "),
        (" mode...", " ..."),
        ("started · ", "iniciado · "),
        (" · market refresh requested", " · atualização do mercado solicitada"),
    ] {
        value = value.replace(en, pt);
    }
    value
}

fn tr<'a>(en: &'a str) -> &'a str {
    if !pt_br() {
        return en;
    }

    match en {
        "Overview" => "Visão geral",
        "PokéIdle controller" => "Controlador do PokéIdle",
        "STOP ALL" => "PARAR TUDO",
        "LAUNCH BOTH" => "INICIAR AMBOS",
        "LAUNCH ENABLED" => "INICIAR HABILITADAS",
        "Accounts" => "Contas",
        "◫  Accounts" => "◫  Contas",
        "Account Manager" => "Gerenciador de contas",
        "Enabled" => "Habilitada",
        "Enable account" => "Habilitar conta",
        "Account name" => "Nome da conta",
        "Save accounts" => "Salvar contas",
        "Firefox profile" => "Perfil do Firefox",
        "Disabled" => "Desabilitada",
        "Available slots" => "Slots disponíveis",
        "Unique profile" => "Perfil exclusivo",
        "Manage up to 4 unique Firefox profiles. Each account has its own browser storage and BiDi connection." => "Gerencie até 4 perfis exclusivos do Firefox. Cada conta possui seu próprio armazenamento do navegador e conexão BiDi.",
        "Status" => "Status",
        "Name cannot be empty" => "O nome não pode ficar vazio",
        "Each slot is a separate Firefox profile. Disabling a slot removes it from the dashboard and from Launch enabled." => "Cada slot é um perfil separado do Firefox. Desabilitar um slot o remove do painel e de Iniciar habilitadas.",
        "Add account" => "Adicionar conta",
        "No accounts enabled. Open Account Manager to add or enable one." => "Nenhuma conta habilitada. Abra o Gerenciador de contas para adicionar ou habilitar uma conta.",
        "by MOTHblank" => "por MOTHblank",
        "Google Play" => "Google Play",
        "X" => "X",
        "WhatsApp / Pix" => "WhatsApp / Pix",
        "source code" => "código-fonte",
        "Credits" => "Créditos",
        "MOTH" => "MOTH",
        "POKEIDLE" => "POKEIDLE",
        "WORKSPACE" => "ÁREA DE TRABALHO",
        "▦  Dashboard" => "▦  Painel",
        "◫  Profiles" => "◫  Perfis",
        "⌁  Hunt Atlas" => "⌁  Atlas de Caça",
        "◇  Moth Watch" => "Moth Watch",
        "≡  Logs" => "≡  Logs",
        "OPERATIONS" => "OPERAÇÕES",
        "▶  Launch both" => "▶  Iniciar ambos",
        "■  Stop all" => "■  Parar tudo",
        "RUNTIME" => "EXECUÇÃO",
        "Firefox" => "Firefox",
        "Monitor" => "Monitor",
        "Poll" => "Consulta",
        "HEADLESS" => "OCULTO",
        "VISIBLE" => "VISÍVEL",
        "MIXED" => "MISTO",
        "Rust rewrite · Windows" => "Reescrita em Rust · Windows",
        "Instances" => "Instâncias",
        "Live state for both isolated Firefox profiles" => "Estado atual dos dois perfis isolados do Firefox",
        "Show Firefox" => "Mostrar Firefox",
        "Hide Firefox" => "Ocultar Firefox",
        "Stop" => "Parar",
        "Launch" => "Iniciar",
        "CURRENT ACTIVITY" => "ATIVIDADE ATUAL",
        "STATE" => "ESTADO",
        "Hunting" => "Caçando",
        "Online · Center" => "Online · Centro",
        "Waiting for login" => "Aguardando login",
        "TRAINER LV" => "NÍVEL DO TREINADOR",
        "TRAINER XP" => "XP DO TREINADOR",
        "POKÉMON LV" => "NÍVEL DO POKÉMON",
        "POKÉMON XP" => "XP DO POKÉMON",
        "FALLEN" => "CAÍDOS",
        "RESOURCES" => "RECURSOS",
        "Pokéballs: unavailable" => "Pokébolas: indisponíveis",
        "GOLD" => "OURO",
        "GEMS" => "GEMAS",
        "AUTO CATCH" => "CAPTURA AUTOMÁTICA",
        "USED" => "USADAS",
        "CAPTURES" => "CAPTURAS",
        "SUCCESS" => "SUCESSO",
        "ON" => "ATIVADO",
        "OFF" => "DESATIVADO",
        "AUTOMATION" => "AUTOMAÇÃO",
        "Active" => "Ativo",
        "Restock" => "Reposição",
        "Stream scanner" => "Scanner de streams",
        "Waiting" => "Aguardando",
        "Streams" => "Streams",
        "OPEN TABS" => "ABAS ABERTAS",
        "No top-level tabs reported" => "Nenhuma aba principal detectada",
        "LOW RESOURCE" => "BAIXO CONSUMO",
        "FULL" => "COMPLETO",
        "STREAMS & BONUS" => "STREAMS E BÔNUS",
        "ACTIVE" => "ATIVO",
        "Watching:" => "Assistindo:",
        "LIVE BONUS AVAILABLE" => "BÔNUS DE LIVE DISPONÍVEL",
        "Open chat:" => "Abra o chat:",
        "No active bonus" => "Nenhum bônus ativo",
        "No Twitch bonus detected" => "Nenhum bônus da Twitch detectado",
        "OPEN" => "ABERTA",
        "XP BONUS SOURCES" => "FONTES DE BÔNUS DE XP",
        "No active XP bonus detected" => "Nenhum bônus de XP ativo detectado",
        "DATA" => "DADOS",
        "ADDONS" => "ADDONS",
        "Missing:" => "Ausentes:",
        "Runtime" => "Execução",
        "Controller-side diagnostics" => "Diagnóstico do controlador",
        "Open Profiles" => "Abrir perfis",
        "Open Logs" => "Abrir logs",
        "Headless" => "Oculto",
        "2 isolated" => "2 isolados",
        "Stream chat" => "Chat das streams",
        "First scan 30s · hourly" => "Primeiro scan em 30s · a cada hora",
        "UI repaint" => "Atualização da interface",
        "1 sec" => "1 s",
        "Hunt Atlas" => "Atlas de Caça",
        "Map intelligence + observed XP/hour" => "Inteligência do mapa + XP/h observado",
        "Select a hunt to travel · XP rates are measured from battle events" => "Selecione uma caça para viajar · as taxas de XP são medidas pelos combates",
        "Select a hunt to travel · XP/hour uses measured combat data or the Hunt Atlas combat model" => "Selecione uma caça para viajar · XP/hora usa dados de combate medidos ou o modelo de combate do Hunt Atlas",
        "Region" => "Região",
        "All regions" => "Todas as regiões",
        "Min" => "Mín.",
        "Max" => "Máx.",
        "Availability" => "Disponibilidade",
        "All" => "Todas",
        "Unlocked" => "Liberadas",
        "Locked" => "Bloqueadas",
        "Type" => "Tipo",
        "All types" => "Todos os tipos",
        "Weak to" => "Fraco contra",
        "Any weakness" => "Qualquer fraqueza",
        "Collection" => "Coleção",
        "Caught + uncaught" => "Capturados + não capturados",
        "Uncaught only" => "Só não capturados",
        "Caught only" => "Só capturados",
        "Sort" => "Ordenar",
        "XP/hour" => "XP/hora",
        "Name" => "Nome",
        "Pokédex number" => "Número da Pokédex",
        "Level" => "Nível",
        "MKT" => "MKT",
        "RMT" => "RMT",
        "Matchup" => "Matchup",
        "Ascending" => "Crescente",
        "Descending" => "Decrescente",
        "Clear filters" => "Limpar filtros",
        "Advantage" => "vantagem",
        "Disadvantage" => "desvantagem",
        "Neutral" => "neutro",
        "Immune" => "imune",
        "Causes" => "Causa",
        "Receives" => "Recebe",
        "caught" => "capturado",
        "not caught" => "não capturado",
        "No Pokémon match these filters." => "Nenhum Pokémon corresponde a estes filtros.",
        "Hunt Atlas is waiting for detailed Pokémon data from the userscript." => "O Hunt Atlas está aguardando os dados detalhados dos Pokémon enviados pelo userscript.",
        "Current hunt" => "Caça atual",
        "Filter" => "Filtro",
        "hunt or Pokémon" => "caça ou Pokémon",
        "hunts available" => "caças disponíveis",
        "unlocked" => "liberada",
        "locked" => "bloqueada",
        "Wait" => "Aguarde",
        "learning" => "aquecendo",
        "observed" => "observado",
        "species" => "espécies",
        "kills/h" => "abates/h",
        "warming up" => "aquecendo",
        "trainer XP/h" => "XP de treinador/h",
        "Pokémon XP/h" => "XP de Pokémon/h",
        "Current" => "Atual",
        "Go" => "Ir",
        "No hunt data yet. The controller bridge must receive the game's welcome message first." => "Ainda não há dados de caça. A ponte do controlador precisa receber primeiro a mensagem de boas-vindas do jogo.",
        "Moth Watch" => "Moth Watch",
        "RMT market browser" => "Navegador do mercado RMT",
        "Select an item, then buy from the live listings below" => "Selecione um item e compre nas ofertas abaixo",
        "Refresh market" => "Atualizar mercado",
        "Find item" => "Encontrar item",
        "item name" => "nome do item",
        "Gold" => "Ouro",
        "Gems" => "Gemas",
        "No market summary loaded. Press Refresh market." => "Resumo do mercado não carregado. Clique em Atualizar mercado.",
        "listings" => "ofertas",
        "No listings in the selected currency match your search." => "Nenhuma oferta na moeda selecionada corresponde à sua busca.",
        "LISTINGS" => "OFERTAS",
        "GOLD ONLY" => "APENAS OURO",
        "GEMS ONLY" => "APENAS GEMAS",
        "No item listings loaded. Pick an item above to load listings you can buy." => "Nenhuma oferta de item carregada. Escolha um item acima para carregar ofertas disponíveis.",
        "seller" => "vendedor",
        "Buy" => "Comprar",
        "Profiles" => "Perfis",
        "Account, browser and addon entry points" => "Pontos de acesso a contas, navegador e addons",
        "Close" => "Fechar",
        "Open Game" => "Abrir jogo",
        "Addons" => "Addons",
        "Profile folder" => "Pasta do perfil",
        "Profile ready" => "Perfil pronto",
        "Created on first launch" => "Criado na primeira execução",
        "Firefox Developer Edition" => "Firefox Developer Edition",
        "Unavailable" => "Indisponível",
        "STOPPED" => "PARADO",
        "HUNTING" => "CAÇANDO",
        "ONLINE" => "ONLINE",
        "CONNECTING" => "CONECTANDO",
        "LOGIN" => "LOGIN",
        "ERROR" => "ERRO",
        "LANGUAGE" => "IDIOMA",
        "▶  Launch enabled" => "▶  Iniciar habilitadas",
        "enabled isolated Firefox profiles" => "perfis isolados do Firefox habilitados",
        _ => en,
    }
}

const BG: Color32 = Color32::from_rgb(11, 13, 18);
const PANEL: Color32 = Color32::from_rgb(17, 20, 27);
const PANEL_ALT: Color32 = Color32::from_rgb(22, 26, 34);
const BORDER: Color32 = Color32::from_rgb(43, 49, 61);
const TEXT: Color32 = Color32::from_rgb(235, 238, 245);
const MUTED: Color32 = Color32::from_rgb(151, 159, 174);
const DIM: Color32 = Color32::from_rgb(103, 112, 129);
const ACCENT: Color32 = Color32::from_rgb(123, 97, 255);
const GOOD: Color32 = Color32::from_rgb(72, 201, 142);
const WARN: Color32 = Color32::from_rgb(245, 180, 72);
const BAD: Color32 = Color32::from_rgb(235, 91, 91);

struct GameSlot {
    profile: GameProfile,
    child: Option<Child>,
    monitor: Option<MonitorHandle>,
    headless: bool,
}

impl GameSlot {
    fn new(profile: GameProfile) -> Self {
        Self {
            profile,
            child: None,
            monitor: None,
            headless: true,
        }
    }

    fn is_running(&mut self) -> bool {
        match self.child.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(None)),
            None => false,
        }
    }

    fn health(&self) -> Health {
        self.monitor
            .as_ref()
            .map(MonitorHandle::health)
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AtlasFilters {
    region: String,
    min_level: String,
    max_level: String,
    availability: String,
    type_filter: String,
    weakness: String,
    collection: String,
    sort: String,
    sort_direction: String,
}

impl Default for AtlasFilters {
    fn default() -> Self {
        Self {
            region: "all".to_string(),
            min_level: String::new(),
            max_level: String::new(),
            availability: "unlocked".to_string(),
            type_filter: "all".to_string(),
            weakness: "all".to_string(),
            collection: "all".to_string(),
            sort: "xp".to_string(),
            sort_direction: "desc".to_string(),
        }
    }
}

pub struct ControllerApp {
    games: [GameSlot; 4],
    show_accounts: bool,
    show_atlas: bool,
    show_market: bool,
    accounts: [AccountConfig; 4],
    atlas_profile: GameProfile,
    atlas_filters: AtlasFilters,
    market_profile: GameProfile,
    market_search: String,
    atlas_search: String,
    market_currency: String,
    status: String,
    status_error: bool,
}

impl ControllerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);

        logging::init();
        logging::info("controller UI initialized with egui/eframe");

        Self {
            games: std::array::from_fn(|i| {
                GameSlot::new(GameProfile::from_index(i).expect("valid profile slot"))
            }),
            show_accounts: false,
            show_atlas: false,
            show_market: false,
            atlas_profile: GameProfile::Game1,
            atlas_filters: AtlasFilters::default(),
            market_profile: GameProfile::Game1,
            market_search: String::new(),
            atlas_search: String::new(),
            market_currency: "gold".to_string(),
            status: localize_status("Ready · launch only the profiles you need".to_string()),
            status_error: false,
            accounts: Config::load_accounts().unwrap_or_else(|error| {
                logging::warn(&format!("account configuration load failed: {error}"));
                std::array::from_fn(|i| AccountConfig::default_for(GameProfile::from_index(i).expect("valid account slot")))
            }),
        }
    }

    fn game_index(profile: GameProfile) -> usize {
        profile.index()
    }

    fn account_name(&self, profile: GameProfile) -> String {
        self.accounts[profile.index()].name.clone()
    }

    fn account_enabled(&self, profile: GameProfile) -> bool {
        self.accounts[profile.index()].enabled
    }

    fn save_account_config(&mut self) -> bool {
        match Config::save_accounts(&self.accounts) {
            Ok(()) => true,
            Err(error) => {
                self.set_status(error, true);
                false
            }
        }
    }

    fn refresh_processes(&mut self) {
        for slot in &mut self.games {
            let exited = match slot.child.as_mut() {
                Some(child) => !matches!(child.try_wait(), Ok(None)),
                None => false,
            };

            if exited {
                let account_name = self.accounts[slot.profile.index()].name.clone();
                logging::info(&format!("{} Firefox process exited", account_name));
                if let Some(monitor) = slot.monitor.as_ref() {
                    monitor.stop();
                }
                slot.child = None;
                slot.monitor = None;
            }
        }
    }

    fn set_status(&mut self, message: impl Into<String>, error: bool) {
        self.status = localize_status(message.into());
        self.status_error = error;
    }

    fn launch_one(&mut self, profile: GameProfile) {
        self.launch_one_mode(profile, true);
    }

    fn launch_one_mode(&mut self, profile: GameProfile, headless: bool) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);
        self.refresh_processes();

        if self.games[index].is_running() {
            self.set_status(format!("{} is already running.", account_name), false);
            return;
        }

        self.games[index].child = None;
        self.games[index].monitor = None;
        self.games[index].headless = headless;

        let config = match crate::config::Config::for_profile(profile) {
            Ok(config) => config,
            Err(error) => {
                self.set_status(format!("{}: {}", self.account_name(profile), error), true);
                return;
            }
        };

        logging::info(&format!(
            "launching {} with Firefox profile {} on BiDi port {}",
            self.account_name(profile),
            config.profile_dir.display(),
            config.remote_debug_port
        ));

        match firefox::launch(&config, headless) {
            Ok((child, monitor)) => {
                let pid = child.id();
                self.games[index].child = Some(child);
                self.games[index].monitor = Some(monitor);
                self.set_status(
                    format!(
                        "{} started · {} Firefox · Rust BiDi health monitor",
                        self.account_name(profile),
                        if headless { "headless" } else { "visible" }
                    ),
                    false,
                );
                logging::info(&format!(
                    "{} Firefox spawned with PID {}",
                    self.account_name(profile),
                    pid
                ));
            }
            Err(error) => {
                logging::error(&format!("{} launch failed: {}", self.account_name(profile), error));
                self.set_status(format!("{}: {}", self.account_name(profile), error), true);
            }
        }
    }

    fn set_browser_mode(&mut self, profile: GameProfile, headless: bool) {
        let index = Self::game_index(profile);
        self.refresh_processes();

        if !self.games[index].is_running() {
            self.games[index].headless = headless;
            self.launch_one_mode(profile, headless);
            return;
        }

        if self.games[index].headless == headless {
            self.set_status(
                format!(
                    "{} is already running in {} mode.",
                    self.account_name(profile),
                    if headless { "headless" } else { "visible" }
                ),
                false,
            );
            return;
        }

        self.set_status(
            format!(
                "{}: switching Firefox to {} mode...",
                self.account_name(profile),
                if headless { "headless" } else { "visible" }
            ),
            false,
        );
        if self.stop_one(profile) {
            self.launch_one_mode(profile, headless);
        }
    }

    fn stop_one(&mut self, profile: GameProfile) -> bool {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);
        let slot = &mut self.games[index];

        if let Some(monitor) = slot.monitor.as_ref() {
            monitor.stop();
        }

        let Some(mut child) = slot.child.take() else {
            slot.monitor = None;
            self.set_status(format!("{} is already stopped.", account_name), false);
            return true;
        };

        logging::info(&format!(
            "stopping {} Firefox PID {}",
            account_name,
            child.id()
        ));

        match child.kill() {
            Ok(()) => {
                let _ = child.wait();
                slot.monitor = None;
                self.set_status(format!("{} Firefox closed.", account_name), false);
                true
            }
            Err(error) => {
                logging::error(&format!(
                    "failed to stop {} Firefox PID {}: {}",
                    account_name,
                    child.id(),
                    error
                ));
                slot.child = Some(child);
                self.set_status(
                    format!("{}: could not close Firefox: {}", account_name, error),
                    true,
                );
                false
            }
        }
    }

    fn launch_enabled(&mut self) {
        for profile in GameProfile::ALL {
            if self.account_enabled(profile) {
                self.launch_one(profile);
            }
        }
        self.set_status("Launch enabled requested · monitoring will update as Firefox becomes ready", false);
    }

    fn stop_all(&mut self) {
        for profile in GameProfile::ALL {
            self.stop_one(profile);
        }
        self.set_status("All Firefox instances closed.", false);
    }

    fn open_logs(&mut self) {
        let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
            self.set_status("Could not determine LOCALAPPDATA for logs.", true);
            return;
        };

        let path = std::path::PathBuf::from(local_app_data)
            .join("Moth")
            .join("PokeIdle")
            .join("moth-controller.log");

        if let Some(parent) = path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                self.set_status(format!("Could not create log folder: {error}"), true);
                return;
            }
        }

        if !path.exists() {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path);
        }

        match std::process::Command::new("notepad.exe")
            .arg(&path)
            .spawn()
        {
            Ok(_) => self.set_status(format!("Opened controller log · {}", path.display()), false),
            Err(error) => self.set_status(format!("Could not open log: {error}"), true),
        }
    }

    fn open_profiles_folder(&mut self) {
        let profiles = match crate::config::Config::profiles_dir() {
            Ok(path) => path,
            Err(error) => {
                self.set_status(error, true);
                return;
            }
        };

        if let Err(error) = std::fs::create_dir_all(&profiles) {
            self.set_status(format!("Could not create profiles folder: {error}"), true);
            return;
        }

        match std::process::Command::new("explorer.exe")
            .arg(&profiles)
            .spawn()
        {
            Ok(_) => self.set_status(format!("Opened {}", profiles.display()), false),
            Err(error) => self.set_status(format!("Could not open folder: {error}"), true),
        }
    }

    fn profile_action(&mut self, profile: GameProfile, action: ProfileAction) {
        let result = match action {
            ProfileAction::Game => accounts::open_game(profile).map(|_| "Opened game".to_string()),
            ProfileAction::Twitch => accounts::open_login(profile, accounts::TWITCH_LOGIN, "Twitch"),
            ProfileAction::Kick => accounts::open_login(profile, accounts::KICK_LOGIN, "KICK"),
            ProfileAction::Addons => accounts::open_addons(profile)
                .map(|count| format!("Opened {count} addon installers")),
            ProfileAction::Folder => accounts::open_profile_folder(profile).map(|_| "Opened profile folder".to_string()),
        };

        match result {
            Ok(message) => self.set_status(format!("{} · {}", self.account_name(profile), message), false),
            Err(error) => self.set_status(error, true),
        }
    }
}

impl Drop for ControllerApp {
    fn drop(&mut self) {
        for slot in &mut self.games {
            if let Some(monitor) = slot.monitor.as_ref() {
                monitor.stop();
            }

            if let Some(mut child) = slot.child.take() {
                let account_name = self.accounts[slot.profile.index()].name.clone();
                logging::info(&format!(
                    "controller shutting down; closing {} Firefox PID {}",
                    account_name,
                    child.id()
                ));
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl eframe::App for ControllerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.refresh_processes();

        // Keep the normal UI at 1Hz, but repaint the Atlas smoothly while its
        // hunt-change cooldown countdown is active.
        let repaint_after = if self.show_atlas {
            let health = self.games[self.atlas_profile.index()].health();
            if hunt_change_cooldown_ms(&health) > 0 {
                std::time::Duration::from_millis(100)
            } else {
                std::time::Duration::from_secs(1)
            }
        } else {
            std::time::Duration::from_secs(1)
        };
        ui.ctx().request_repaint_after(repaint_after);

        draw_sidebar(self, ui);

        egui::Panel::top("header")
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(24, 18)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr("Overview"))
                            .font(FontId::proportional(26.0))
                            .strong()
                            .color(TEXT),
                    );
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(tr("PokéIdle controller"))
                            .font(FontId::proportional(14.0))
                            .color(MUTED),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(tr("STOP ALL"))
                                        .size(12.0)
                                        .strong()
                                        .color(TEXT),
                                )
                                .fill(BAD.linear_multiply(0.18))
                                .stroke(Stroke::new(1.0, BAD.linear_multiply(0.55)))
                                .corner_radius(8.0),
                            )
                            .clicked()
                        {
                            self.stop_all();
                        }

                        ui.add_space(8.0);

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(tr("LAUNCH ENABLED"))
                                        .size(12.0)
                                        .strong()
                                        .color(TEXT),
                                )
                                .fill(ACCENT.linear_multiply(0.8))
                                .stroke(Stroke::new(1.0, ACCENT))
                                .corner_radius(8.0),
                            )
                            .clicked()
                        {
                            self.launch_enabled();
                        }
                    });
                });
            });

        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0, BORDER))
                    .inner_margin(Margin::symmetric(24, 11)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let dot_color = if self.status_error { BAD } else { GOOD };
                    ui.colored_label(dot_color, "●");
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(&self.status)
                            .size(12.0)
                            .color(if self.status_error { TEXT } else { MUTED }),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(Margin::symmetric(24, 16)))
            .show(ui, |ui| {
                draw_instance_section(self, ui);
            });

        if self.show_accounts {
            draw_accounts_window(self, ui.ctx());
        }
        if self.show_atlas {
            draw_atlas_window(self, ui.ctx());
        }
        if self.show_market {
            draw_market_window(self, ui.ctx());
        }
    }
}

#[derive(Clone, Copy)]
enum ProfileAction {
    Game,
    Twitch,
    Kick,
    Addons,
    Folder,
}

fn draw_sidebar(app: &mut ControllerApp, ui: &mut egui::Ui) {
    egui::Panel::left("sidebar")
        .resizable(false)
        .default_size(206.0)
        .frame(
            egui::Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER))
                .inner_margin(16.0),
        )
        .show(ui, |ui| {
            ui.label(
                RichText::new("MOTH")
                    .font(FontId::proportional(22.0))
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new("POKEIDLE")
                    .font(FontId::proportional(10.0))
                    .strong()
                    .color(ACCENT),
            );
            ui.add_space(22.0);

            section_label(ui, tr("WORKSPACE"));

            if sidebar_button(ui, tr("▦  Dashboard"), true).clicked() {
                app.set_status("Dashboard · live health polling enabled", false);
            }

            if sidebar_button(ui, tr("◫  Accounts"), false).clicked() {
                app.show_accounts = true;
            }

            if sidebar_button(ui, tr("⌁  Hunt Atlas"), false).clicked() {
                app.show_atlas = true;
            }

            if sidebar_button(ui, tr("◇  Moth Watch"), false).clicked() {
                app.show_market = true;
            }

            if sidebar_button(ui, tr("≡  Logs"), false).clicked() {
                app.open_logs();
            }

            ui.add_space(18.0);
            section_label(ui, tr("OPERATIONS"));

            if sidebar_button(ui, tr("▶  Launch enabled"), false).clicked() {
                app.launch_enabled();
            }

            if sidebar_button(ui, tr("■  Stop all"), false).clicked() {
                app.stop_all();
            }

            ui.add_space(16.0);
            section_label(ui, tr("LANGUAGE"));

            ui.horizontal(|ui| {
                let en_selected = !pt_br();
                let pt_selected = pt_br();

                let en_button = egui::Button::new(
                    RichText::new("EN").size(10.0).strong(),
                )
                .fill(if en_selected {
                    ACCENT.linear_multiply(0.22)
                } else {
                    PANEL_ALT
                })
                .stroke(Stroke::new(
                    1.0,
                    if en_selected { ACCENT } else { BORDER },
                ))
                .corner_radius(7.0);

                if ui.add_sized([78.0, 30.0], en_button).clicked() {
                    PT_BR.store(false, Ordering::Relaxed);
                }

                let pt_button = egui::Button::new(
                    RichText::new("PT-BR").size(10.0).strong(),
                )
                .fill(if pt_selected {
                    ACCENT.linear_multiply(0.22)
                } else {
                    PANEL_ALT
                })
                .stroke(Stroke::new(
                    1.0,
                    if pt_selected { ACCENT } else { BORDER },
                ))
                .corner_radius(7.0);

                if ui.add_sized([78.0, 30.0], pt_button).clicked() {
                    PT_BR.store(true, Ordering::Relaxed);
                }
            });

            ui.add_space(20.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(10.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(tr("RUNTIME"))
                            .size(10.0)
                            .strong()
                            .color(DIM),
                    );
                    ui.add_space(7.0);
                    runtime_row(ui, "Firefox", if app.games.iter().all(|game| game.headless) { tr("HEADLESS") } else if app.games.iter().all(|game| !game.headless) { tr("VISIBLE") } else { tr("MIXED") }, GOOD);
                    runtime_row(ui, "Monitor", "BiDi", GOOD);
                    runtime_row(ui, "Poll", "5 sec", MUTED);
                });

            ui.add_space(12.0);
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.add_space(6.0);
                draw_credits(ui);
                ui.add_space(10.0);
                ui.label(
                    RichText::new(tr("Rust rewrite · Windows"))
                        .size(10.0)
                        .color(DIM),
                );
            });
        });
}

fn draw_instance_section(app: &mut ControllerApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(tr("Instances"))
                .font(FontId::proportional(17.0))
                .strong()
                .color(TEXT),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(tr("Live state for enabled isolated Firefox profiles"))
                .size(11.0)
                .color(DIM),
        );
    });

    ui.add_space(12.0);

    egui::ScrollArea::vertical()
        .id_salt("instances_page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let profiles: Vec<usize> = GameProfile::ALL
                .iter()
                .map(|profile| profile.index())
                .filter(|index| app.accounts[*index].enabled)
                .collect();

            if profiles.is_empty() {
                ui.label(
                    RichText::new(tr("No accounts enabled. Open Account Manager to add or enable one."))
                        .size(12.0)
                        .color(DIM),
                );
            } else {
                let gap = ui.spacing().item_spacing.x;
                let available = ui.available_width();
                let columns = if available >= 880.0 { 2 } else { 1 };
                let card_width = if columns == 2 {
                    ((available - gap) / 2.0).max(390.0)
                } else {
                    available.max(390.0)
                };

                for chunk in profiles.chunks(columns) {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;

                        for index in chunk {
                            ui.allocate_ui_with_layout(
                                egui::vec2(card_width, 0.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    draw_game_card(app, ui, *index, card_width);
                                },
                            );
                        }
                    });

                    ui.add_space(12.0);
                }
            }

            ui.add_space(16.0);
            draw_runtime_section(app, ui);
        });
}

fn draw_game_card(
    app: &mut ControllerApp,
    ui: &mut egui::Ui,
    index: usize,
    width: f32,
) {
    ui.set_width(width);

    let profile = app.games[index].profile;
    let account_name = app.account_name(profile);
    let health = app.games[index].health();
    let running = app.games[index].is_running();

    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(12.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(account_name)
                        .size(14.0)
                        .strong()
                        .color(TEXT),
                );

                ui.add_space(8.0);
                status_badge(ui, &health, running);

                if running {
                    let mode_label = if app.games[index].headless { tr("Show Firefox") } else { tr("Hide Firefox") };
                    let mode_headless = !app.games[index].headless;
                    let clicked = ui
                        .add_sized(
                            [108.0, 30.0],
                            egui::Button::new(
                                RichText::new(mode_label).size(10.0).strong(),
                            )
                            .fill(PANEL_ALT)
                            .stroke(Stroke::new(1.0, BORDER))
                            .corner_radius(7.0),
                        )
                        .clicked();
                    if clicked {
                        app.set_browser_mode(profile, mode_headless);
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let button_text = if running { tr("Stop") } else { tr("Launch") };
                    let clicked = ui
                        .add_sized(
                            [84.0, 30.0],
                            egui::Button::new(
                                RichText::new(button_text).size(11.0).strong(),
                            )
                            .fill(if running {
                                BAD.linear_multiply(0.18)
                            } else {
                                ACCENT.linear_multiply(0.82)
                            })
                            .stroke(Stroke::new(
                                1.0,
                                if running { BAD.linear_multiply(0.55) } else { ACCENT },
                            ))
                            .corner_radius(7.0),
                        )
                        .clicked();

                    if clicked {
                        if running {
                            app.stop_one(profile);
                        } else {
                            app.launch_one(profile);
                        }
                    }
                });
            });

            ui.add_space(14.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(
                            if health.activity == "Hunting" {
                                tr("CURRENT ACTIVITY")
                            } else {
                                tr("STATE")
                            },
                        )
                        .size(9.0)
                        .strong()
                        .color(DIM),
                    );

                    ui.add_space(3.0);

                    let headline = match health.activity.as_str() {
                        "Hunting" if !health.hunt.is_empty() => {
                            format!("Hunting · {}", compact_text(&health.hunt, 34))
                        }
                        "Center" => "Online · Center".to_string(),
                        "Login" => "Waiting for login".to_string(),
                        _ => health.summary(),
                    };

                    ui.add(
                        egui::Label::new(
                            RichText::new(compact_text(&headline, 52))
                                .size(18.0)
                                .strong()
                                .color(TEXT),
                        )
                        .truncate(),
                    );

                    if !health.active_pokemon.is_empty() {
                        ui.add_space(3.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(compact_text(&health.active_pokemon, 54))
                                    .size(11.0)
                                    .color(MUTED),
                            )
                            .truncate(),
                        );
                    }

                    ui.add_space(9.0);

                    let trainer_level = if health.player_level == 0 {
                        "—".to_string()
                    } else {
                        health.player_level.to_string()
                    };
                    let fallen = health.fallen_count.to_string();

                    ui.horizontal_wrapped(|ui| {
                        mini_metric(ui, "TRAINER LV", &trainer_level);
                        mini_metric(ui, "TRAINER XP", if health.player_xp.is_empty() { "—" } else { &health.player_xp });
                        mini_metric(ui, "POKÉMON LV", if health.pokemon_level.is_empty() { "—" } else { &health.pokemon_level });
                        mini_metric(ui, "POKÉMON XP", if health.pokemon_xp.is_empty() { "—" } else { &health.pokemon_xp });
                        mini_metric(ui, "FALLEN", &fallen);
                    });
                });

            ui.add_space(12.0);

            ui.label(
                RichText::new(tr("RESOURCES"))
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    if health.ball_stock.is_empty() {
                        ui.label(RichText::new(tr("Pokéballs: unavailable")).size(11.0).color(MUTED));
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for item in &health.ball_stock {
                                resource_chip(ui, item);
                            }
                        });
                    }

                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        let gold = format_number(health.gold);
                        let orbs = format_number(health.orbs);
                        resource_value(ui, tr("GOLD"), &gold, WARN);
                        resource_value(ui, tr("GEMS"), &orbs, ACCENT);
                        resource_value(
                            ui,
                            tr("AUTO CATCH"),

                            if health.autocatch_on { "ON" } else { "OFF" },
                            if health.autocatch_on { GOOD } else { MUTED },
                        );
                        resource_value(
                            ui,
                            tr("USED"),
                            &health.autocatch_balls_used.to_string(),
                            TEXT,
                        );
                        resource_value(
                            ui,
                            tr("CAPTURES"),
                            &health.autocatch_captures.to_string(),
                            TEXT,
                        );
                        resource_value(
                            ui,
                            tr("SUCCESS"),
                            if health.autocatch_rate.is_empty() { "—" } else { &health.autocatch_rate },
                            TEXT,
                        );
                    });
                });

            ui.add_space(12.0);

            ui.label(
                RichText::new(tr("AUTOMATION"))
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            let performance_value = if health.performance_fps.is_empty() {
                "Loaded".to_string()
            } else {
                format!("{} FPS", health.performance_fps)
            };

            ui.horizontal_wrapped(|ui| {
                automation_badge(
                    ui,
                    "Auto Catch",
                    if health.autocatch_on { "Active" } else { "Off" },
                    health.autocatch_on,
                );
                automation_badge(
                    ui,
                    "Restock",
                    if health.autocatch_restock.is_empty() { "Off" } else { &health.autocatch_restock },
                    health.autocatch_on && !health.autocatch_restock.eq_ignore_ascii_case("off"),
                );
                automation_badge(
                    ui,
                    "Stream scanner",
                    if health.stream_scan_status.is_empty() {
                        "Waiting"
                    } else {
                        &health.stream_scan_status
                    },
                    health.stream_scan_status.eq_ignore_ascii_case("ok"),
                );
                if health.stream_scan_live > 0 || health.stream_scan_opened > 0 {
                    let streams_value =
                        format!("{} live · {} opened", health.stream_scan_live, health.stream_scan_opened);

                    automation_badge(
                        ui,
                        "Streams",
                        &streams_value,
                        true,
                    );
                }
                automation_badge(ui, "Performance+", &performance_value, true);
            });

            ui.add_space(12.0);
            ui.label(
                RichText::new(tr("OPEN TABS"))
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    if health.tabs.is_empty() {
                        ui.label(
                            RichText::new(tr("No top-level tabs reported"))
                                .size(10.0)
                                .color(DIM),
                        );
                    } else {
                        for tab in &health.tabs {
                            ui.horizontal(|ui| {
                                let marker = if tab.kind == "Twitch" {
                                    if tab.low_resource { "●" } else { "○" }
                                } else {
                                    "●"
                                };

                                ui.label(
                                    RichText::new(marker)
                                        .size(9.0)
                                        .color(if tab.kind == "Twitch" && tab.low_resource { GOOD } else { MUTED }),
                                );

                                ui.label(
                                    RichText::new(&tab.kind)
                                        .size(9.0)
                                        .strong()
                                        .color(TEXT),
                                );

                                ui.add_space(5.0);

                                let title = if tab.title.is_empty() {
                                    &tab.url
                                } else {
                                    &tab.title
                                };

                                ui.add(
                                    egui::Label::new(
                                        RichText::new(compact_text(title, 54))
                                            .size(9.0)
                                            .color(MUTED),
                                    )
                                    .truncate(),
                                );

                                if tab.kind == "Twitch" {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(if tab.low_resource { tr("LOW RESOURCE") } else { tr("FULL") })
                                                .size(8.0)
                                                .strong()
                                                .color(if tab.low_resource { GOOD } else { WARN }),
                                        );
                                    });
                                }
                            });
                        }
                    }
                });

            ui.add_space(12.0);
            ui.label(
                RichText::new(tr("STREAMS & BONUS"))
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    if !health.stream_watching.is_empty() {
                        ui.label(
                            RichText::new(format!(
                                "ACTIVE · +{}% XP",
                                format_pct(health.stream_bonus_pct)
                            ))
                            .size(11.0)
                            .strong()
                            .color(GOOD),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Watching: {}",
                                health.stream_watching.join(", ")
                            ))
                            .size(10.0)
                            .color(MUTED),
                        );
                    } else if !health.stream_missing.is_empty() {
                        ui.label(
                            RichText::new(tr("LIVE BONUS AVAILABLE"))
                                .size(10.0)
                                .strong()
                                .color(WARN),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Open chat: {}{}",
                                health.stream_missing.join(", "),
                                if health.stream_bonus_pct > 0.0 {
                                    format!(" · +{}% XP", format_pct(health.stream_bonus_pct))
                                } else {
                                    String::new()
                                }
                            ))
                            .size(10.0)
                            .color(TEXT),
                        );
                    } else if !health.stream_bonus_last.is_empty() {
                        let last_seen = if health.last_game_message_ms > 0 {
                            format!(" · {}", format_time(health.last_game_message_ms))
                        } else {
                            String::new()
                        };

                        ui.label(
                            RichText::new(format!(
                                "No active bonus · last seen{}: {}",
                                last_seen,
                                health.stream_bonus_last
                            ))
                            .size(10.0)
                            .color(WARN),
                        );
                    } else {
                        ui.label(
                            RichText::new(tr("No Twitch bonus detected"))
                                .size(10.0)
                                .color(DIM),
                        );
                    }

                    ui.add_space(6.0);

                    ui.horizontal_wrapped(|ui| {
                        for tab in health.tabs.iter().filter(|tab| {
                            tab.kind == "Twitch" || tab.kind == "KICK"
                        }) {
                            let state = if tab.kind == "Twitch" {
                                if tab.low_resource { "LOW" } else { tr("FULL") }
                            } else {
                                "OPEN"
                            };

                            ui.label(
                                RichText::new(format!(
                                    "{} · {} · {}",
                                    tab.kind,
                                    compact_text(
                                        tab.title.rsplit('/').next().unwrap_or(&tab.title),
                                        24
                                    ),
                                    state
                                ))
                                .size(9.0)
                                .color(if tab.kind == "Twitch" && tab.low_resource {
                                    GOOD
                                } else {
                                    MUTED
                                }),
                            );
                        }
                    });
                });

            ui.add_space(12.0);
            ui.label(
                RichText::new(tr("XP BONUS SOURCES"))
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    let mut sources = health.xp_sources.clone();
                    for bonus in &health.xp_bonuses {
                        if !sources.iter().any(|source| source == bonus) {
                            sources.push(bonus.clone());
                        }
                    }

                    if sources.is_empty() {
                        ui.label(
                            RichText::new(tr("No active XP bonus detected"))
                                .size(10.0)
                                .color(DIM),
                        );
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for source in sources {
                                bonus_chip(ui, &source);
                            }
                        });
                    }
                });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if health.last_game_message_ms > 0 {
                    let age = (chrono_like_now_ms().saturating_sub(health.last_game_message_ms)) / 1000;
                    ui.label(
                        RichText::new(format!("DATA {}s ago", age))
                            .size(8.0)
                            .color(if age <= 10 { GOOD } else { WARN }),
                    );
                    ui.add_space(8.0);
                }

                let addon_color = if health.addon_ok == health.addon_total {
                    GOOD
                } else {
                    WARN
                };

                ui.label(
                    RichText::new(format!("ADDONS {}/{}", health.addon_ok, health.addon_total))
                        .size(10.0)
                        .strong()
                        .color(addon_color),
                );

                if !health.addon_missing.is_empty() {
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("Missing: {}", health.addon_missing.join(" · ")))
                                .size(10.0)
                                .color(WARN),
                        )
                        .truncate(),
                    );
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if health.twitch_tabs > 0 {
                        ui.label(
                            RichText::new(format!(
                                "Twitch LR {}/{}",
                                health.twitch_low_resource_ok, health.twitch_tabs
                            ))
                            .size(10.0)
                            .color(MUTED),
                        );
                    }
                });
            });
        });
}

fn mini_metric(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(RichText::new(tr(label)).size(8.0).strong().color(DIM));
        ui.add(
            egui::Label::new(
                RichText::new(compact_text(value, 18))
                    .size(11.0)
                    .strong()
                    .color(TEXT),
            )
            .truncate(),
        );
    });
    ui.add_space(16.0);
}

fn resource_chip(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(7.0)
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.label(RichText::new(compact_text(text, 24)).size(10.0).color(TEXT));
        });
}

fn resource_value(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    ui.vertical(|ui| {
        ui.label(RichText::new(tr(label)).size(8.0).strong().color(DIM));
        ui.add(
            egui::Label::new(
                RichText::new(compact_text(value, 18))
                    .size(10.0)
                    .strong()
                    .color(color),
            )
            .truncate(),
        );
    });
    ui.add_space(14.0);
}

fn automation_badge(ui: &mut egui::Ui, label: &str, value: &str, good: bool) {
    egui::Frame::new()
        .fill(if good { GOOD.linear_multiply(0.08) } else { PANEL_ALT })
        .stroke(Stroke::new(1.0, if good { GOOD.linear_multiply(0.28) } else { BORDER }))
        .corner_radius(7.0)
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("● {}", tr(label)))
                        .size(9.0)
                        .strong()
                        .color(if good { GOOD } else { MUTED }),
                );
                ui.add_space(4.0);
                ui.label(RichText::new(compact_text(value, 24)).size(9.0).color(TEXT));
            });
        });
}

fn bonus_chip(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(ACCENT.linear_multiply(0.10))
        .stroke(Stroke::new(1.0, ACCENT.linear_multiply(0.28)))
        .corner_radius(7.0)
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.label(RichText::new(compact_text(text, 38)).size(10.0).strong().color(TEXT));
        });
}


fn draw_runtime_section(app: &mut ControllerApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(tr("Runtime"))
                .font(FontId::proportional(17.0))
                .strong()
                .color(TEXT),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(tr("Controller-side diagnostics"))
                .size(11.0)
                .color(DIM),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button(tr("Open Profiles")).clicked() {
                app.show_accounts = true;
            }
            if ui.button(tr("Open Logs")).clicked() {
                app.open_logs();
            }
        });
    });

    ui.add_space(10.0);

    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                runtime_chip(ui, "Firefox", "Headless", GOOD);
                runtime_chip(ui, "BiDi", "Active", GOOD);
                runtime_chip(ui, tr("Profiles"), "2 isolated", MUTED);
                runtime_chip(ui, "Stream chat", "First scan 30s · hourly", MUTED);
                runtime_chip(ui, "UI repaint", "1 sec", MUTED);
            });
        });
}


fn game_selector(
    account_names: &[String; 4],
    ui: &mut egui::Ui,
    selected: &mut GameProfile,
) {
    ui.horizontal(|ui| {
        for profile in GameProfile::ALL {
            let active = *selected == profile;
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(account_names[profile.index()].as_str())
                            .size(10.0)
                            .strong()
                            .color(if active { TEXT } else { MUTED }),
                    )
                    .fill(if active {
                        ACCENT.linear_multiply(0.22)
                    } else {
                        PANEL_ALT
                    })
                    .stroke(Stroke::new(
                        1.0,
                        if active { ACCENT } else { BORDER },
                    ))
                    .corner_radius(7.0),
                )
                .clicked()
            {
                *selected = profile;
            }
        }
    });
}

fn draw_credits(ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8.0)
        .inner_margin(Margin::symmetric(9, 7))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(tr("Credits"))
                        .size(9.0)
                        .strong()
                        .color(DIM),
                );

                ui.add_space(3.0);

                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(tr("by MOTHblank"))
                            .size(9.0)
                            .color(MUTED),
                    );
                    ui.label(RichText::new("·").size(9.0).color(DIM));
                    ui.hyperlink_to(
                        tr("Google Play"),
                        "https://play.google.com/store/apps/developer?id=MOTHblank",
                    );
                    ui.label(RichText::new("·").size(9.0).color(DIM));
                    ui.hyperlink_to(
                        tr("X"),
                        "https://x.com/MOTHblank",
                    );
                    ui.label(RichText::new("·").size(9.0).color(DIM));
                    ui.hyperlink_to(
                        tr("WhatsApp / Pix"),
                        "https://wa.me/+5537999933376",
                    );
                    ui.label(RichText::new("·").size(9.0).color(DIM));
                    ui.hyperlink_to(
                        tr("source code"),
                        "https://github.com/MOTHblank/pokeidle-addons",
                    );
                });
            });
        });
}

fn hunt_change_cooldown_label(milliseconds: u64) -> String {
    if pt_br() {
        format!(
            "Troca de hunt disponível em {:.1}s",
            milliseconds as f32 / 1000.0
        )
    } else {
        format!(
            "Hunt change available in {:.1}s",
            milliseconds as f32 / 1000.0
        )
    }
}

fn hunt_change_cooldown_ms(health: &Health) -> u64 {
    if health.last_battle_at == 0 {
        return 0;
    }

    2600_u64.saturating_sub(
        chrono_like_now_ms().saturating_sub(health.last_battle_at)
    )
}

fn atlas_type_options() -> [&'static str; 18] {
    [
        "NORMAL", "FIRE", "WATER", "ELECTRIC", "GRASS", "ICE",
        "FIGHTING", "POISON", "GROUND", "FLYING", "PSYCHIC", "BUG",
        "ROCK", "GHOST", "DRAGON", "DARK", "STEEL", "FAIRY",
    ]
}

fn atlas_compare_numeric(
    a: f32,
    b: f32,
    direction: &str,
    missing_bottom: bool,
) -> std::cmp::Ordering {
    let a_missing = !a.is_finite() || (missing_bottom && a <= 0.0);
    let b_missing = !b.is_finite() || (missing_bottom && b <= 0.0);

    match (a_missing, b_missing) {
        (true, true) => std::cmp::Ordering::Equal,
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        (false, false) => {
            let base = a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);
            if direction == "asc" {
                base
            } else {
                base.reverse()
            }
        }
    }
}

fn atlas_matchup_label(multiplier: f32, direction: &str, attack_type: &str) -> (String, Color32) {
    let relation = if multiplier == 0.0 {
        ("Immune", BAD)
    } else if direction == "offense" && multiplier > 1.0 {
        ("Advantage", GOOD)
    } else if direction == "offense" && multiplier < 1.0 {
        ("Disadvantage", BAD)
    } else if direction == "defense" && multiplier < 1.0 {
        ("Advantage", GOOD)
    } else if direction == "defense" && multiplier > 1.0 {
        ("Disadvantage", BAD)
    } else {
        ("Neutral", MUTED)
    };

    (
        format!(
            "{} {}{} · {}",
            if direction == "offense" { "ATK" } else { "DEF" },
            format_multiplier(multiplier),
            if attack_type.is_empty() {
                String::new()
            } else {
                format!(" {}", atlas_type_label(attack_type))
            },
            tr(relation.0)
        ),
        relation.1,
    )
}

fn atlas_type_label(value: &str) -> String {
    if !pt_br() {
        return value.to_string();
    }

    match value.to_ascii_uppercase().as_str() {
        "NORMAL" => "Normal",
        "FIRE" => "Fogo",
        "WATER" => "Água",
        "ELECTRIC" => "Elétrico",
        "GRASS" => "Grama",
        "ICE" => "Gelo",
        "FIGHTING" => "Lutador",
        "POISON" => "Veneno",
        "GROUND" => "Terra",
        "FLYING" => "Voador",
        "PSYCHIC" => "Psíquico",
        "BUG" => "Inseto",
        "ROCK" => "Pedra",
        "GHOST" => "Fantasma",
        "DRAGON" => "Dragão",
        "DARK" => "Sombrio",
        "STEEL" => "Aço",
        "FAIRY" => "Fada",
        _ => value,
    }
    .to_string()
}

fn format_multiplier(value: f32) -> String {
    if value.is_finite() {
        if (value.fract()).abs() < 0.01 {
            format!("×{:.0}", value)
        } else {
            format!("×{:.2}", value)
        }
    } else {
        "×?".to_string()
    }
}

fn draw_atlas_window(app: &mut ControllerApp, ctx: &egui::Context) {
    let screen = ctx.content_rect();
    let width = (screen.width() - 64.0).clamp(620.0, 980.0);
    let height = (screen.height() - 96.0).clamp(460.0, 720.0);

    let mut open = app.show_atlas;
    egui::Window::new(tr("Hunt Atlas"))
        .id(egui::Id::new("hunt_atlas_window"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(width)
        .default_height(height)
        .min_width(560.0_f32.min(width))
        .min_height(440.0_f32.min(height))
        .max_width(width)
        .max_height(height)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(12.0)
                .inner_margin(18.0),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr("Hunt Atlas"))
                        .font(FontId::proportional(22.0))
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(tr("Map intelligence + observed XP/hour"))
                        .size(12.0)
                        .color(MUTED),
                );
            });

            ui.add_space(2.0);
            ui.label(
                RichText::new(tr("Select a hunt to travel · XP rates are measured from battle events"))
                    .size(10.0)
                    .color(DIM),
            );

            ui.add_space(12.0);
            let account_names = std::array::from_fn(|i| app.accounts[i].name.clone());
            game_selector(&account_names, ui, &mut app.atlas_profile);

            let index = ControllerApp::game_index(app.atlas_profile);
            let health = app.games[index].health();
            let cooldown_ms = hunt_change_cooldown_ms(&health);

            ui.add_space(10.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(10.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(tr("Current hunt"))
                                .size(9.0)
                                .strong()
                                .color(DIM),
                        );
                        ui.add_space(7.0);
                        ui.label(
                            RichText::new(if health.hunt.is_empty() { "—" } else { &health.hunt })
                                .size(14.0)
                                .strong()
                                .color(TEXT),
                        );
                        ui.add_space(18.0);
                        ui.label(
                            RichText::new(format!("Trainer Lv {}", health.player_level))
                                .size(10.0)
                                .color(MUTED),
                        );
                        ui.add_space(10.0);

                        let cooldown_text = if cooldown_ms > 0 {
                            hunt_change_cooldown_label(cooldown_ms)
                        } else {
                            tr("Current").to_string()
                        };

                        ui.label(
                            RichText::new(cooldown_text)
                                .size(10.0)
                                .strong()
                                .color(if cooldown_ms > 0 { WARN } else { GOOD }),
                        );
                    });
                });

            ui.add_space(10.0);

            // Search and reset.
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Filter")).size(10.0).strong().color(DIM));
                ui.add_sized(
                    [230.0, 28.0],
                    egui::TextEdit::singleline(&mut app.atlas_search)
                        .hint_text(tr("hunt or Pokémon")),
                );

                let active = app.atlas_filters != AtlasFilters::default();
                if ui
                    .add_enabled(active, egui::Button::new(tr("Clear filters")))
                    .clicked()
                {
                    app.atlas_filters = AtlasFilters::default();
                    app.atlas_search.clear();
                }

                let direction = if app.atlas_filters.sort_direction == "asc" { "↑" } else { "↓" };
                if ui.button(direction).clicked() {
                    app.atlas_filters.sort_direction = if app.atlas_filters.sort_direction == "asc" {
                        "desc".to_string()
                    } else {
                        "asc".to_string()
                    };
                }
            });

            ui.add_space(8.0);

            // Filter controls mirror the web Atlas: region, level, availability,
            // type, weakness and collection.
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Region")).size(9.0).strong().color(DIM));
                egui::ComboBox::from_id_salt("atlas_region")
                    .selected_text(
                        if app.atlas_filters.region == "all" {
                            tr("All regions").to_string()
                        } else {
                            app.atlas_filters.region.clone()
                        }
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.atlas_filters.region,
                            "all".to_string(),
                            tr("All regions"),
                        );

                        let mut regions = health
                            .hunts
                            .iter()
                            .map(|hunt| hunt.area.clone())
                            .filter(|area| !area.is_empty())
                            .collect::<Vec<_>>();
                        regions.sort();
                        regions.dedup();

                        for region in regions {
                            ui.selectable_value(
                                &mut app.atlas_filters.region,
                                region.clone(),
                                region,
                            );
                        }
                    });

                ui.label(RichText::new(tr("Min")).size(9.0).strong().color(DIM));
                ui.add_sized(
                    [64.0, 26.0],
                    egui::TextEdit::singleline(&mut app.atlas_filters.min_level)
                        .hint_text("1"),
                );
                ui.label(RichText::new(tr("Max")).size(9.0).strong().color(DIM));
                ui.add_sized(
                    [64.0, 26.0],
                    egui::TextEdit::singleline(&mut app.atlas_filters.max_level)
                        .hint_text("9999"),
                );

                ui.label(RichText::new(tr("Availability")).size(9.0).strong().color(DIM));
                egui::ComboBox::from_id_salt("atlas_availability")
                    .selected_text(match app.atlas_filters.availability.as_str() {
                        "locked" => tr("Locked"),
                        "all" => tr("All"),
                        _ => tr("Unlocked"),
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.atlas_filters.availability, "unlocked".to_string(), tr("Unlocked"));
                        ui.selectable_value(&mut app.atlas_filters.availability, "all".to_string(), tr("All"));
                        ui.selectable_value(&mut app.atlas_filters.availability, "locked".to_string(), tr("Locked"));
                    });
            });

            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Type")).size(9.0).strong().color(DIM));
                egui::ComboBox::from_id_salt("atlas_type")
                    .selected_text(if app.atlas_filters.type_filter == "all" {
                        tr("All types").to_string()
                    } else {
                        app.atlas_filters.type_filter.clone()
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.atlas_filters.type_filter,
                            "all".to_string(),
                            tr("All types"),
                        );
                        for value in atlas_type_options() {
                            ui.selectable_value(
                                &mut app.atlas_filters.type_filter,
                                value.to_string(),
                                value,
                            );
                        }
                    });

                ui.label(RichText::new(tr("Weak to")).size(9.0).strong().color(DIM));
                egui::ComboBox::from_id_salt("atlas_weakness")
                    .selected_text(if app.atlas_filters.weakness == "all" {
                        tr("Any weakness").to_string()
                    } else {
                        app.atlas_filters.weakness.clone()
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.atlas_filters.weakness,
                            "all".to_string(),
                            tr("Any weakness"),
                        );
                        for value in atlas_type_options() {
                            ui.selectable_value(
                                &mut app.atlas_filters.weakness,
                                value.to_string(),
                                value,
                            );
                        }
                    });

                ui.label(RichText::new(tr("Collection")).size(9.0).strong().color(DIM));
                egui::ComboBox::from_id_salt("atlas_collection")
                    .selected_text(match app.atlas_filters.collection.as_str() {
                        "captured" => tr("Caught only"),
                        "uncaught" => tr("Uncaught only"),
                        _ => tr("Caught + uncaught"),
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.atlas_filters.collection,
                            "all".to_string(),
                            tr("Caught + uncaught"),
                        );
                        ui.selectable_value(
                            &mut app.atlas_filters.collection,
                            "uncaught".to_string(),
                            tr("Uncaught only"),
                        );
                        ui.selectable_value(
                            &mut app.atlas_filters.collection,
                            "captured".to_string(),
                            tr("Caught only"),
                        );
                    });

                ui.label(RichText::new(tr("Sort")).size(9.0).strong().color(DIM));
                egui::ComboBox::from_id_salt("atlas_sort")
                    .selected_text(match app.atlas_filters.sort.as_str() {
                        "name" => tr("Name"),
                        "pokedex" => tr("Pokédex number"),
                        "level" => tr("Level"),
                        "npc" => tr("MKT"),
                        "player_market" => tr("RMT"),
                        "matchup" => tr("Matchup"),
                        _ => tr("XP/hour"),
                    })
                    .show_ui(ui, |ui| {
                        for (value, label) in [
                            ("xp", tr("XP/hour")),
                            ("name", tr("Name")),
                            ("pokedex", tr("Pokédex number")),
                            ("level", tr("Level")),
                            ("npc", tr("MKT")),
                            ("player_market", tr("RMT")),
                            ("matchup", tr("Matchup")),
                        ] {
                            ui.selectable_value(
                                &mut app.atlas_filters.sort,
                                value.to_string(),
                                label,
                            );
                        }
                    });
            });

            ui.add_space(9.0);

            let search = app.atlas_search.to_lowercase();
            let min_level = app.atlas_filters.min_level.trim().parse::<u32>().ok();
            let max_level = app.atlas_filters.max_level.trim().parse::<u32>().ok();

            let mut rows: Vec<(usize, usize)> = Vec::new();

            for (hunt_index, hunt) in health.hunts.iter().enumerate() {
                let unlocked = hunt.unlocked;
                if app.atlas_filters.region != "all"
                    && hunt.area != app.atlas_filters.region
                {
                    continue;
                }

                if let Some(min) = min_level {
                    if hunt.level < min {
                        continue;
                    }
                }

                if let Some(max) = max_level {
                    if hunt.level > max {
                        continue;
                    }
                }

                match app.atlas_filters.availability.as_str() {
                    "unlocked" if !unlocked => continue,
                    "locked" if unlocked => continue,
                    _ => {}
                }

                for (species_index, species) in hunt.species_details.iter().enumerate() {
                    let captured = species.captured;

                    match app.atlas_filters.collection.as_str() {
                        "captured" if !captured => continue,
                        "uncaught" if captured => continue,
                        _ => {}
                    }

                    if app.atlas_filters.type_filter != "all"
                        && !species.types.iter().any(|value| {
                            value.eq_ignore_ascii_case(&app.atlas_filters.type_filter)
                        })
                    {
                        continue;
                    }

                    if app.atlas_filters.weakness != "all"
                        && !species.weak_to.iter().any(|value| {
                            value.eq_ignore_ascii_case(&app.atlas_filters.weakness)
                        })
                    {
                        continue;
                    }

                    if !search.is_empty() {
                        let matches = species.name.to_lowercase().contains(&search)
                            || hunt.name.to_lowercase().contains(&search)
                            || hunt.slug.to_lowercase().contains(&search)
                            || hunt.area.to_lowercase().contains(&search)
                            || species.types.iter().any(|value| value.to_lowercase().contains(&search));
                        if !matches {
                            continue;
                        }
                    }

                    rows.push((hunt_index, species_index));
                }
            }

            let direction = app.atlas_filters.sort_direction.as_str();
            rows.sort_by(|(ha, sa), (hb, sb)| {
                let a_hunt = &health.hunts[*ha];
                let b_hunt = &health.hunts[*hb];
                let a = &a_hunt.species_details[*sa];
                let b = &b_hunt.species_details[*sb];

                let comparison = match app.atlas_filters.sort.as_str() {
                    "name" => {
                        let base = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                        if direction == "asc" { base } else { base.reverse() }
                    },
                    "pokedex" => {
                        atlas_compare_numeric(
                            a.id as f32,
                            b.id as f32,
                            direction,
                            false,
                        )
                    }
                    "level" => atlas_compare_numeric(
                        a_hunt.level as f32,
                        b_hunt.level as f32,
                        direction,
                        false,
                    ),
                    "npc" => atlas_compare_numeric(
                        a.npc_value as f32,
                        b.npc_value as f32,
                        direction,
                        true,
                    ),
                    "player_market" => atlas_compare_numeric(
                        a.market_value as f32,
                        b.market_value as f32,
                        direction,
                        true,
                    ),
                    "matchup" => atlas_compare_numeric(
                        a.matchup_score.unwrap_or(f32::NAN),
                        b.matchup_score.unwrap_or(f32::NAN),
                        direction,
                        true,
                    ),
                    _ => atlas_compare_numeric(
                        a_hunt.xp_per_hour as f32,
                        b_hunt.xp_per_hour as f32,
                        direction,
                        true,
                    ),
                };

                if comparison == std::cmp::Ordering::Equal {
                    a.name
                        .to_lowercase()
                        .cmp(&b.name.to_lowercase())
                        .then_with(|| a.id.cmp(&b.id))
                } else {
                    comparison
                }
            });

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} Pokémon",
                        rows.len()
                    ))
                    .size(9.0)
                    .strong()
                    .color(DIM),
                );

                if cooldown_ms > 0 {
                    ui.label(
                        RichText::new(format!(
                            "{} {:.1}s",
                            tr("Wait"),
                            cooldown_ms as f32 / 1000.0
                        ))
                        .size(9.0)
                        .strong()
                        .color(WARN),
                    );
                }
            });

            ui.add_space(6.0);

            if health.hunts.iter().all(|hunt| hunt.species_details.is_empty()) {
                ui.label(
                    RichText::new(tr("Hunt Atlas is waiting for detailed Pokémon data from the userscript."))
                        .size(11.0)
                        .color(DIM),
                );
            } else if rows.is_empty() {
                ui.label(
                    RichText::new(tr("No Pokémon match these filters."))
                        .size(11.0)
                        .color(DIM),
                );
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("atlas_species_list")
                    .auto_shrink([false, false])
                    .max_height(ui.available_height().max(180.0))
                    .show_rows(ui, 112.0, rows.len(), |ui, row_range| {
                        for row in row_range {
                            let (hunt_index, species_index) = rows[row];
                            let hunt = &health.hunts[hunt_index];
                            let species = &hunt.species_details[species_index];

                            let current = hunt.current
                                || (
                                    !health.hunt_slug.is_empty()
                                        && hunt.slug.eq_ignore_ascii_case(&health.hunt_slug)
                                );

                            let ready = cooldown_ms == 0;
                            let xp_prefix = match hunt.xp_source.to_ascii_lowercase().as_str() {
                                "observed" | "measured" => "observed",
                                "combat model" | "modeled" => "model",
                                _ => "",
                            };

                            let xp_label = if hunt.xp_per_hour > 0 {
                                if xp_prefix.is_empty() {
                                    format!("{} {}", format_rate(hunt.xp_per_hour), tr("trainer XP/h"))
                                } else {
                                    format!(
                                        "{} · {} {}",
                                        xp_prefix,
                                        format_rate(hunt.xp_per_hour),
                                        tr("trainer XP/h")
                                    )
                                }
                            } else {
                                tr("learning").to_string()
                            };

                            let pokemon_xp = if hunt.pokemon_xp_per_hour > 0 {
                                format!("{} {}", format_rate(hunt.pokemon_xp_per_hour), tr("Pokémon XP/h"))
                            } else {
                                "—".to_string()
                            };

                            let captured_label = if species.captured {
                                if species.capture_count > 1 {
                                    format!("✓ {} ×{}", tr("caught"), species.capture_count)
                                } else {
                                    format!("✓ {}", tr("caught"))
                                }
                            } else {
                                format!("○ {}", tr("not caught"))
                            };

                            ui.set_width(ui.available_width());

                            egui::Frame::new()
                                .fill(if current { ACCENT.linear_multiply(0.08) } else { PANEL_ALT })
                                .stroke(Stroke::new(
                                    1.0,
                                    if current { ACCENT.linear_multiply(0.35) } else { BORDER },
                                ))
                                .corner_radius(9.0)
                                .inner_margin(10.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.set_width(230.0);
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(&species.name)
                                                    .size(12.0)
                                                    .strong()
                                                    .color(TEXT),
                                            );

                                            ui.label(
                                                RichText::new(format!(
                                                    "{} · Lv {} · {}",
                                                    hunt.name,
                                                    hunt.level,
                                                    hunt.area
                                                ))
                                                .size(9.0)
                                                .color(MUTED),
                                            );

                                            let type_text = if species.types.is_empty() {
                                                "TYPE ?".to_string()
                                            } else {
                                                species.types.join(" · ")
                                            };

                                            ui.label(
                                                RichText::new(type_text)
                                                    .size(8.0)
                                                    .color(DIM),
                                            );

                                            ui.label(
                                                RichText::new(captured_label)
                                                    .size(8.0)
                                                    .color(if species.captured { GOOD } else { DIM }),
                                            );
                                        });

                                        ui.add_space(10.0);

                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(xp_label)
                                                    .size(10.0)
                                                    .strong()
                                                    .color(if hunt.xp_per_hour > 0 {
                                                        GOOD
                                                    } else {
                                                        MUTED
                                                    }),
                                            );

                                            ui.label(
                                                RichText::new(format!(
                                                    "{} · {}",
                                                    pokemon_xp,
                                                    if hunt.kills_per_hour > 0 {
                                                        format!("{} {}", hunt.kills_per_hour, tr("kills/h"))
                                                    } else {
                                                        tr("learning").to_string()
                                                    }
                                                ))
                                                .size(8.0)
                                                .color(MUTED),
                                            );

                                            if let Some(value) = species.offense_multiplier {
                                                let (label, color) = atlas_matchup_label(
                                                    value,
                                                    "offense",
                                                    &species.offense_type,
                                                );
                                                ui.label(
                                                    RichText::new(label)
                                                        .size(8.0)
                                                        .strong()
                                                        .color(color),
                                                );
                                            }

                                            if let Some(value) = species.defense_multiplier {
                                                let (label, color) = atlas_matchup_label(
                                                    value,
                                                    "defense",
                                                    &species.defense_type,
                                                );
                                                ui.label(
                                                    RichText::new(label)
                                                        .size(8.0)
                                                        .strong()
                                                        .color(color),
                                                );
                                            }
                                        });

                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            let button_text = if current {
                                                tr("Current")
                                            } else if !ready {
                                                if pt_br() {
                                                    format!("Aguarde {:.1}s", cooldown_ms as f32 / 1000.0)
                                                } else {
                                                    format!("Wait {:.1}s", cooldown_ms as f32 / 1000.0)
                                                }
                                            } else if !hunt.unlocked {
                                                tr("locked").to_string()
                                            } else {
                                                tr("Go").to_string()
                                            };

                                            let enabled = hunt.unlocked && !current && ready;
                                            if ui
                                                .add_enabled(
                                                    enabled,
                                                    egui::Button::new(button_text),
                                                )
                                                .clicked()
                                            {
                                                let profile_label = app.account_name(app.atlas_profile);
                                                let monitor = app.games[index].monitor.clone();

                                                if let Some(monitor) = monitor {
                                                    if hunt_change_cooldown_ms(&health) > 0 {
                                                        app.set_status(
                                                            format!(
                                                                "{} · {:.1}s before you can change hunts",
                                                                profile_label,
                                                                hunt_change_cooldown_ms(&health) as f32 / 1000.0
                                                            ),
                                                            false,
                                                        );
                                                    } else {
                                                        let hunt_slug = hunt.slug.clone();
                                                        let hunt_name = hunt.name.clone();
                                                        monitor.send(json!({
                                                            "t": "hunt.select",
                                                            "slug": hunt_slug
                                                        }));
                                                        app.set_status(
                                                            format!("{} · changing hunt to {}", profile_label, hunt_name),
                                                            false,
                                                        );
                                                    }
                                                } else {
                                                    app.set_status(
                                                        format!("{} is not running.", profile_label),
                                                        true,
                                                    );
                                                }
                                            }
                                        });
                                    });
                                });
                        }
                    });
            }
        });

    app.show_atlas = open;
}

fn draw_market_window(app: &mut ControllerApp, ctx: &egui::Context) {
    let mut open = app.show_market;
    let viewport = ctx.content_rect();
    let available_width = (viewport.width() - 32.0).max(420.0);
    let available_height = (viewport.height() - 32.0).max(360.0);
    let default_width = available_width.min(1000.0);
    let default_height = available_height.min(650.0);
    let max_width = available_width.max(420.0);
    let max_height = available_height.max(360.0);

    egui::Window::new(tr("Moth Watch"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(default_width)
        .default_height(default_height)
        .min_width(420.0_f32.min(max_width))
        .min_height(360.0_f32.min(max_height))
        .max_width(max_width)
        .max_height(max_height)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(12.0)
                .inner_margin(18.0),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr("Moth Watch"))
                        .font(FontId::proportional(22.0))
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(tr("RMT market browser"))
                        .size(12.0)
                        .color(MUTED),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(tr("Select an item, then buy from the live listings below"))
                            .size(10.0)
                            .color(DIM),
                    );
                });
            });

            ui.add_space(14.0);
            let account_names = std::array::from_fn(|i| app.accounts[i].name.clone());
            game_selector(&account_names, ui, &mut app.market_profile);

            let index = ControllerApp::game_index(app.market_profile);
            let health = app.games[index].health();

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                let gold = format_number(health.gold);
                let orbs = format_number(health.orbs);
                ui.label(RichText::new(format!("Gold {}", gold)).size(11.0).strong().color(WARN));
                ui.add_space(14.0);
                ui.label(RichText::new(format!("Gems {}", orbs)).size(11.0).strong().color(ACCENT));
                ui.add_space(20.0);

                if ui.button(tr("Refresh market")).clicked() {
                    if let Some(monitor) = app.games[index].monitor.as_ref() {
                        monitor.send(json!({ "t": "market.itens" }));
                        app.set_status(format!("{} · market refresh requested", app.account_name(app.market_profile)), false);
                    } else {
                        app.set_status(format!("{} is not running.", app.account_name(app.market_profile)), true);
                    }
                }
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Find item")).size(10.0).strong().color(DIM));
                ui.add_sized(
                    [300.0, 28.0],
                    egui::TextEdit::singleline(&mut app.market_search)
                        .hint_text(tr("item name")),
                );

                ui.add_space(10.0);

                for currency in ["gold", "orb"] {
                    let active = app.market_currency == currency;
                    if ui
                        .add(
                            egui::Button::new(RichText::new(if currency == "gold" { "Gold" } else { "Gems" }).size(10.0))
                                .fill(if active { ACCENT.linear_multiply(0.20) } else { PANEL_ALT })
                                .stroke(Stroke::new(1.0, if active { ACCENT } else { BORDER }))
                                .corner_radius(7.0),
                        )
                        .clicked()
                    {
                        app.market_currency = currency.to_string();
                    }
                }
            });

            ui.add_space(10.0);

            let search = app.market_search.to_lowercase();

            egui::ScrollArea::vertical()
                .id_salt("market_catalog")
                .max_height(250.0)
                .show(ui, |ui| {
                    if health.market_summary.is_empty() {
                        ui.label(
                            RichText::new(tr("No market summary loaded. Press Refresh market."))
                                .size(11.0)
                                .color(DIM),
                        );

                        ui.add_space(6.0);

                        let catalog: Vec<_> = health
                            .market_catalog
                            .iter()
                            .filter(|item| {
                                search.is_empty()
                                    || item.name.to_lowercase().contains(&search)
                            })
                            .take(35)
                            .cloned()
                            .collect();

                        ui.horizontal_wrapped(|ui| {
                            for item in catalog {
                                let label = compact_text(&item.name, 22);
                                if ui.button(label).clicked() {
                                    let profile_label = app.account_name(app.market_profile);
                                    let monitor = app.games[index].monitor.clone();
                                    if let Some(monitor) = monitor {
                                        let item_id = item.id;
                                        let item_name = item.name.clone();
                                        let currency = app.market_currency.clone();

                                        monitor.send(json!({
                                            "t": "market.item",
                                            "itemId": item_id,
                                            "moeda": currency
                                        }));
                                        app.set_status(
                                            format!("{} · loading listings to buy {}", profile_label, item_name),
                                            false,
                                        );
                                    } else {
                                        app.set_status(
                                            format!("{} is not running.", profile_label),
                                            true,
                                        );
                                    }
                                }
                            }
                        });
                    } else {
                        let selected_currency = app.market_currency.clone();
                        let filtered_items: Vec<_> = health
                            .market_summary
                            .iter()
                            .filter(|item| {
                                let currency_available = match selected_currency.as_str() {
                                    "gold" => item.gold_min > 0,
                                    "orb" => item.orb_min > 0,
                                    _ => true,
                                };

                                currency_available
                                    && (search.is_empty()
                                        || item.name.to_lowercase().contains(&search))
                            })
                            .take(50)
                            .cloned()
                            .collect();

                        for item in filtered_items {
                            egui::Frame::new()
                                .fill(PANEL_ALT)
                                .stroke(Stroke::new(1.0, BORDER))
                                .corner_radius(8.0)
                                .inner_margin(9.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(compact_text(&item.name, 32))
                                                    .size(11.0)
                                                    .strong()
                                                    .color(TEXT),
                                            );

                                            let price = if app.market_currency == "gold" {
                                                if item.gold_min > 0 {
                                                    format!("{} gold", format_number(item.gold_min))
                                                } else {
                                                    "— gold".to_string()
                                                }
                                            } else if item.orb_min > 0 {
                                                format!("{} gems", format_number(item.orb_min))
                                            } else {
                                                "— gems".to_string()
                                            };

                                            let reference = if app.market_currency == "gold" {
                                                item.gold_average
                                            } else {
                                                item.orb_average
                                            };
                                            let min_price = if app.market_currency == "gold" {
                                                item.gold_min
                                            } else {
                                                item.orb_min
                                            };
                                            let discount = if reference > min_price && min_price > 0 {
                                                format!(" · {}", market_discount_label(
                                                    ((reference - min_price) as f32 / reference as f32) * 100.0
                                                ))
                                            } else {
                                                String::new()
                                            };

                                            ui.label(
                                                RichText::new(format!(
                                                    "{} · {} listings{}",
                                                    price,
                                                    item.listings,
                                                    discount
                                                ))
                                                .size(9.0)
                                                .color(MUTED),
                                            );
                                        });

                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            let profile_label = app.account_name(app.market_profile);
                                            let monitor = app.games[index].monitor.clone();
                                            if ui.button(tr("Buy")).clicked() {
                                                if let Some(monitor) = monitor {
                                                    let item_id = item.item_id;
                                                    let item_name = item.name.clone();
                                                    let currency = selected_currency.clone();
                                                    monitor.send(json!({
                                                        "t": "market.item",
                                                        "itemId": item_id,
                                                        "moeda": currency
                                                    }));
                                                    app.set_status(
                                                        format!("{} · loading listings to buy {}", profile_label, item_name),
                                                        false,
                                                    );
                                                } else {
                                                    app.set_status(
                                                        format!("{} is not running.", profile_label),
                                                        true,
                                                    );
                                                }
                                            }
                                        });
                                    });
                                });

                            ui.add_space(5.0);
                        }

                        if !health.market_summary.iter().any(|item| {
                            let currency_available = match app.market_currency.as_str() {
                                _ => true,
                            };
                            currency_available
                                && (search.is_empty()
                                    || item.name.to_lowercase().contains(&search))
                        }) {
                            ui.label(
                                RichText::new(tr("No listings in the selected currency match your search."))
                                    .size(11.0)
                                    .color(DIM),
                            );
                        }
                    }
                });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("LISTINGS")).size(9.0).strong().color(DIM));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(if app.market_currency == "gold" { tr("GOLD ONLY") } else { tr("GEMS ONLY") })
                        .size(8.0)
                        .strong()
                        .color(if app.market_currency == "gold" { WARN } else { ACCENT }),
                );
            });
            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .id_salt("market_listings")
                .max_height(300.0)
                .show(ui, |ui| {
                    if health.market_listings.is_empty() {
                        ui.label(
                            RichText::new(tr("No item listings loaded. Pick an item above to load listings you can buy."))
                                .size(11.0)
                                .color(DIM),
                        );
                    } else {
                        for listing in &health.market_listings {
                            if listing.currency != app.market_currency {
                                continue;
                            }

                            egui::Frame::new()
                                .fill(PANEL_ALT)
                                .stroke(Stroke::new(1.0, BORDER))
                                .corner_radius(8.0)
                                .inner_margin(9.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(compact_text(&listing.name, 30)).size(11.0).strong().color(TEXT));
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} × {} · seller {}",
                                                    format_number(listing.price),
                                                    listing.quantity,
                                                    compact_text(&listing.seller, 20)
                                                ))
                                                .size(9.0)
                                                .color(MUTED),
                                            );
                                            let reference = if listing.reference_price > 0 {
                                                format!(
                                                    " · {} · ref {}",
                                                    market_discount_label(listing.discount_pct),
                                                    format_number(listing.reference_price)
                                                )
                                            } else {
                                                String::new()
                                            };
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}{}",
                                                    market_wait_label(listing.seconds_until_buy),
                                                    reference
                                                ))
                                                .size(9.0)
                                                .color(
                                                    if listing.seconds_until_buy > 0 {
                                                        WARN
                                                    } else if listing.discount_pct > 0.05 {
                                                        GOOD
                                                    } else {
                                                        DIM
                                                    }
                                                ),
                                            );
                                        });

                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            let can_buy = listing.seconds_until_buy == 0;
                                            if ui
                                                .add_enabled(
                                                    can_buy,
                                                    egui::Button::new(
                                                        if can_buy { tr("Buy") } else { tr("Wait") }
                                                    )
                                                )
                                                .clicked()
                                            {
                                                let profile_label = app.account_name(app.market_profile);
                                                let monitor = app.games[index].monitor.clone();

                                                if let Some(monitor) = monitor {
                                                    let listing_id = listing.id;
                                                    let listing_name = listing.name.clone();
                                                    let currency = listing.currency.clone();
                                                    monitor.send(json!({
                                                        "t": "market.comprar",
                                                        "id": listing_id,
                                                        "qtd": listing.quantity.max(1),
                                                        "preco": listing.price,
                                                        "moeda": currency
                                                    }));
                                                    app.set_status(
                                                        format!("{} · buy command sent for {}", profile_label, listing_name),
                                                        false,
                                                    );
                                                } else {
                                                    app.set_status(
                                                        format!("{} is not running.", profile_label),
                                                        true,
                                                    );
                                                }
                                            }
                                        });
                                    });
                                });
                            ui.add_space(6.0);
                        }
                    }
                });
        });
    app.show_market = open;
}

pub(crate) fn chrono_like_now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn format_pct(value: f32) -> String {
    if value.fract().abs() < 0.01 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}

fn format_time(timestamp_ms: u64) -> String {
    let seconds = timestamp_ms / 1000;
    let minute = (seconds / 60) % 60;
    let hour = (seconds / 3600) % 24;
    format!("{:02}:{:02}", hour, minute)
}

fn format_number(value: u64) -> String {
    let mut value = value.to_string();
    let mut i = value.len() as isize - 3;
    while i > 0 {
        value.insert(i as usize, ',');
        i -= 3;
    }
    value
}

fn format_rate(value: u64) -> String {
    format_number(value)
}

fn market_wait_label(seconds: u64) -> String {
    if seconds == 0 {
        return if pt_br() { "Disponível".to_string() } else { "Available".to_string() };
    }

    let minutes = seconds / 60;
    let secs = seconds % 60;

    if minutes > 0 {
        if pt_br() {
            format!("Libera em {}m {:02}s", minutes, secs)
        } else {
            format!("Available in {}m {:02}s", minutes, secs)
        }
    } else if pt_br() {
        format!("Libera em {}s", secs)
    } else {
        format!("Available in {}s", secs)
    }
}

fn market_discount_label(discount_pct: f32) -> String {
    if discount_pct > 0.05 {
        if pt_br() {
            format!("−{}% vs média 7d", format_pct(discount_pct))
        } else {
            format!("−{}% vs 7d avg", format_pct(discount_pct))
        }
    } else if pt_br() {
        "Sem desconto vs média 7d".to_string()
    } else {
        "No discount vs 7d avg".to_string()
    }
}

fn draw_accounts_window(app: &mut ControllerApp, ctx: &egui::Context) {
    let mut open = app.show_accounts;
    egui::Window::new(tr("Account Manager"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(820.0)
        .default_height(620.0)
        .min_width(720.0)
        .min_height(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(12.0)
                .inner_margin(18.0),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(tr("Account Manager"))
                            .font(FontId::proportional(22.0))
                            .strong()
                            .color(TEXT),
                    );
                    ui.label(
                        RichText::new(tr("Manage up to 4 unique Firefox profiles. Each account has its own browser storage and BiDi connection."))
                            .size(11.0)
                            .color(MUTED),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(tr("Close")).clicked() {
                        app.show_accounts = false;
                    }
                });
            });

            ui.add_space(12.0);

            egui::ScrollArea::vertical()
                .id_salt("account_manager_list")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for profile in GameProfile::ALL {
                        let index = profile.index();
                        let account = &mut app.accounts[index];
                        let mut changed = false;

                        egui::Frame::new()
                            .fill(PANEL_ALT)
                            .stroke(Stroke::new(1.0, BORDER))
                            .corner_radius(10.0)
                            .inner_margin(14.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(profile.label())
                                            .size(15.0)
                                            .strong()
                                            .color(TEXT),
                                    );

                                    ui.add_space(10.0);

                                    let checkbox_glyph = if account.enabled { "✓" } else { "" };
                                    let checkbox = egui::Button::new(
                                        RichText::new(checkbox_glyph)
                                            .size(13.0)
                                            .strong()
                                            .color(if account.enabled { TEXT } else { DIM }),
                                    )
                                    .fill(if account.enabled {
                                        ACCENT.linear_multiply(0.22)
                                    } else {
                                        PANEL
                                    })
                                    .stroke(Stroke::new(
                                        1.0,
                                        if account.enabled { ACCENT } else { BORDER },
                                    ))
                                    .corner_radius(5.0);

                                    if ui.add_sized([28.0, 28.0], checkbox).clicked() {
                                        account.enabled = !account.enabled;
                                        changed = true;
                                    }

                                    ui.label(
                                        RichText::new(tr("Enable account"))
                                            .size(10.0)
                                            .color(TEXT),
                                    );

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{}: {}",
                                                tr("Firefox profile"),
                                                profile.name()
                                            ))
                                            .size(9.0)
                                            .color(DIM),
                                        );
                                    });
                                });

                                ui.add_space(8.0);

                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(tr("Account name"))
                                            .size(10.0)
                                            .strong()
                                            .color(DIM),
                                    );
                                    let response = ui.add_sized(
                                        [360.0, 30.0],
                                        egui::TextEdit::singleline(&mut account.name)
                                            .hint_text(profile.label()),
                                    );
                                    changed |= response.changed();

                                    if account.name.trim().is_empty() {
                                        ui.colored_label(WARN, tr("Name cannot be empty"));
                                    }
                                });

                                ui.add_space(7.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} {}",
                                            tr("Status"),
                                            if account.enabled {
                                                tr("Enabled")
                                            } else {
                                                tr("Disabled")
                                            }
                                        ))
                                        .size(10.0)
                                        .color(if account.enabled { GOOD } else { DIM }),
                                    );
                                    ui.add_space(14.0);
                                    ui.label(
                                        RichText::new(format!(
                                            "{}: %LOCALAPPDATA%\\Moth\\PokeIdle\\Profiles\\{}",
                                            tr("Firefox profile"),
                                            profile.name()
                                        ))
                                        .size(9.0)
                                        .color(DIM),
                                    );
                                });

                                if changed {
                                    // Persistence happens after this frame so the same
                                    // mutable account reference is no longer borrowed.
                                }
                            });

                        ui.add_space(10.0);

                        // Save changes immediately, while preserving the account slot.
                        if changed {
                            let trimmed = app.accounts[index].name.trim().to_string();
                            app.accounts[index].name = if trimmed.is_empty() {
                                profile.label().to_string()
                            } else {
                                trimmed
                            };
                            app.save_account_config();
                        }
                    }

                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(tr("Each slot is a separate Firefox profile. Disabling a slot removes it from the dashboard and from Launch enabled."))
                            .size(10.0)
                            .color(DIM),
                    );
                });
        });

    app.show_accounts = open;
}

fn draw_profile_card(app: &mut ControllerApp, ui: &mut egui::Ui, profile: GameProfile) {
    let config = crate::config::Config::for_profile(profile);
    let (profile_state, browser) = match config {
        Ok(ref config) => {
            let state = if config.profile_dir.exists() {
                "Profile ready"
            } else {
                "Created on first launch"
            };

            let browser = if config
                .firefox_executable
                .to_string_lossy()
                .to_lowercase()
                .contains("developer")
            {
                "Firefox Developer Edition"
            } else {
                "Firefox"
            };

            (state.to_string(), browser.to_string())
        }
        Err(error) => ("Unavailable".to_string(), error),
    };

    egui::Frame::new()
        .fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(app.account_name(profile))
                        .font(FontId::proportional(15.0))
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(8.0);
                ui.label(RichText::new(profile_state).size(11.0).color(GOOD));
            });

            ui.add_space(4.0);
            ui.label(
                RichText::new(browser)
                    .size(10.0)
                    .color(MUTED),
            );

            ui.add_space(12.0);

            ui.horizontal_wrapped(|ui| {
                profile_button(ui, tr("Open Game"), || app.profile_action(profile, ProfileAction::Game));
                profile_button(ui, "Twitch", || app.profile_action(profile, ProfileAction::Twitch));
                profile_button(ui, "KICK", || app.profile_action(profile, ProfileAction::Kick));
                profile_button(ui, tr("Addons"), || app.profile_action(profile, ProfileAction::Addons));
                profile_button(ui, tr("Profile folder"), || app.profile_action(profile, ProfileAction::Folder));
            });
        });
}

fn profile_button(ui: &mut egui::Ui, label: &str, mut action: impl FnMut()) {
    if ui
        .add_sized(
            [124.0, 32.0],
            egui::Button::new(RichText::new(label).size(11.0)),
        )
        .clicked()
    {
        action();
    }
}

fn status_badge(ui: &mut egui::Ui, health: &Health, running: bool) {
    let (label, color) = if !running {
        ("STOPPED", DIM)
    } else if health.state == "Running" && health.activity == "Hunting" {
        ("HUNTING", GOOD)
    } else if health.state == "Running" && health.logged_in {
        ("ONLINE", GOOD)
    } else if health.state == "Connecting" || health.state == "Reconnecting" {
        ("CONNECTING", WARN)
    } else if health.state == "Running" && !health.logged_in {
        ("LOGIN", WARN)
    } else {
        ("ERROR", BAD)
    };

    ui.label(
        RichText::new(format!("● {}", label))
            .size(10.0)
            .strong()
            .color(color),
    );
}

fn metric(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(label)
                .size(9.0)
                .strong()
                .color(DIM),
        );
        ui.label(
            RichText::new(compact_text(value, 22))
                .size(11.0)
                .color(TEXT),
        );
    });
    ui.add_space(18.0);
}

fn runtime_chip(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    egui::Frame::new()
        .fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8.0)
        .inner_margin(Margin::symmetric(11, 8))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(tr(label)).size(9.0).strong().color(DIM));
                ui.label(RichText::new(tr(value)).size(11.0).color(color));
            });
        });
}

fn runtime_row(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr(label)).size(10.0).color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(tr(value)).size(10.0).strong().color(color));
        });
    });
    ui.add_space(5.0);
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(9.0)
            .strong()
            .color(DIM),
    );
    ui.add_space(5.0);
}

fn sidebar_button<'a>(ui: &mut egui::Ui, text: &'a str, selected: bool) -> egui::Response {
    let fill = if selected {
        ACCENT.linear_multiply(0.18)
    } else {
        Color32::TRANSPARENT
    };

    ui.add_sized(
        [174.0, 34.0],
        egui::Button::new(
            RichText::new(text)
                .size(11.0)
                .strong()
                .color(if selected { TEXT } else { MUTED }),
        )
        .fill(fill)
        .stroke(if selected {
            Stroke::new(1.0, ACCENT.linear_multiply(0.6))
        } else {
            Stroke::NONE
        })
        .corner_radius(7.0),
    )
}

fn compact_text(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let mut out: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        out.push('…');
    }

    out
}

fn configure_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.interact_size.y = 30.0;

        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(22.0)),
            (TextStyle::Body, FontId::proportional(13.0)),
            (TextStyle::Button, FontId::proportional(12.0)),
            (TextStyle::Small, FontId::proportional(10.0)),
            (TextStyle::Monospace, FontId::monospace(11.0)),
        ]
        .into();

        style.visuals.window_fill = PANEL;
        style.visuals.panel_fill = BG;
        style.visuals.faint_bg_color = PANEL_ALT;
        style.visuals.extreme_bg_color = BG;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.widgets.noninteractive.bg_fill = PANEL;
        style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        style.visuals.widgets.inactive.bg_fill = PANEL_ALT;
        style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(28, 33, 43);
        style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(31, 36, 46);
        style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT);
    });
}

pub fn run() -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Moth · PokéIdle")
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Moth · PokéIdle",
        options,
        Box::new(|cc| Ok(Box::new(ControllerApp::new(cc)))),
    )
    .map_err(|error| format!("could not start controller UI: {error}"))
}