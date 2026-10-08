use crate::accounts;
use crate::config::{AccountConfig, Config, GameProfile};
use crate::firefox;
use crate::kick::KickManager;
use crate::logging;
use crate::monitor::{Health, MonitorHandle};
use serde_json::json;
use eframe::egui::{self, Align, Color32, FontId, Layout, Margin, RichText, Stroke, TextStyle};
use std::process::Child;
use std::time::{Duration, Instant};

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
        ("addon installers, but some failed:
", "instaladores de addons, mas alguns falharam:
"),
        ("Use \"Install Violentmonkey\" first.", "Use \"Instalar Violentmonkey\" primeiro."),
        ("Firefox was not found. Install Firefox or set MOTH_FIREFOX to firefox.exe.", "O Firefox não foi encontrado. Instale o Firefox ou defina MOTH_FIREFOX para firefox.exe."),
        ("Violentmonkey is not installed and active in ", "O Violentmonkey não está instalado e ativo em "),
        ("invalid page probe: ", "sondagem de página inválida: "),
        ("invalid controller bridge snapshot: ", "snapshot inválido da ponte do controlador: "),
        ("invalid BiDi JSON: ", "JSON BiDi inválido: "),
        ("script.evaluate exception: ", "exceção em script.evaluate: "),
        ("Launch enabled requested · monitoring will update as Firefox becomes ready", "Iniciar habilitadas solicitado · o monitor atualizará quando o Firefox estiver pronto"),
        ("Could not determine LOCALAPPDATA for logs.", "Não foi possível determinar LOCALAPPDATA para os logs."),
        ("Could not create log folder: ", "Não foi possível criar a pasta de logs: "),
        ("Could not open log: ", "Não foi possível abrir o log: "),
        ("Could not create profiles folder: ", "Não foi possível criar a pasta de perfis: "),
        ("Could not open folder: ", "Não foi possível abrir a pasta: "),
        ("Twitch opened in ", "Twitch aberto em "),
        ("KICK opened in ", "KICK aberto em "),
        ("Opened KICK login in uncontrolled normal Firefox · close it after authentication", "Login do KICK aberto no Firefox normal sem controle · feche-o após autenticar"),
        ("Violentmonkey installer opened in ", "Instalador do Violentmonkey aberto em "),
        ("could not read Firefox extensions registry: ", "não foi possível ler o registro de extensões do Firefox: "),
        ("invalid Firefox extensions registry: ", "registro de extensões do Firefox inválido: "),
        ("could not create profile directory: ", "não foi possível criar a pasta do perfil: "),
        ("could not open profile folder: ", "não foi possível abrir a pasta do perfil: "),
        ("could not read account configuration: ", "não foi possível ler a configuração das contas: "),
        ("invalid account configuration: ", "configuração de contas inválida: "),
        ("could not create account data directory: ", "não foi possível criar a pasta de dados das contas: "),
        ("could not serialize account configuration: ", "não foi possível serializar a configuração das contas: "),
        ("could not save account configuration: ", "não foi possível salvar a configuração das contas: "),
        ("could not determine the Windows local application data directory", "não foi possível determinar a pasta de dados local do Windows"),
        ("could not start Firefox: ", "não foi possível iniciar o Firefox: "),
        ("refusing to open a non-HTTPS URL", "recusa ao abrir uma URL que não usa HTTPS"),
        ("native KICK browser is only supported on Windows", "o navegador nativo do KICK só é compatível com Windows"),
        ("empty KICK URL", "URL do KICK vazia"),
        ("refusing to open a non-KICK HTTPS URL", "recusa ao abrir uma URL HTTPS que não é do KICK"),
        ("could not create KICK profile: ", "não foi possível criar o perfil do KICK: "),
        ("could not start normal KICK Firefox: ", "não foi possível iniciar o Firefox normal do KICK: "),
        ("could not start managed KICK Firefox: ", "não foi possível iniciar o Firefox gerenciado do KICK: "),
        ("KICK is currently managed by the controller. Close its managed Firefox window before starting KICK login.", "o KICK está sendo gerenciado pelo controlador. Feche a janela gerenciada do Firefox do KICK antes de iniciar o login."),
        ("could not request a new KICK Firefox window: ", "não foi possível solicitar uma nova janela do Firefox do KICK: "),
        (" updated.", " atualizado."),
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
        ("switching Firefox to ", "mudando o Firefox para o modo "),
        (" mode...", " ..."),
        ("started · ", "iniciado · "),
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
        "custom tab requested" => "nova aba personalizada solicitada",
        "Enter a URL." => "Digite uma URL.",
        "Open a custom URL in this Firefox profile" => "Abrir uma URL personalizada neste perfil do Firefox",
        "https://example.com" => "https://exemplo.com",
        "Open" => "Abrir",
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
        "First scan 30s · 10 min" => "Primeiro scan em 30s · a cada 10 min",
        "UI repaint" => "Atualização da interface",
        "1 sec" => "1 s",
        "Map intelligence + observed XP/hour" => "Inteligência do mapa + XP/h observado",
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
        "Match all selected" => "Exigir todos os selecionados",
        "selected" => "selecionados",
        "No weaknesses selected" => "Nenhuma fraqueza selecionada",
        "Clear selection" => "Limpar seleção",
        "Collection" => "Coleção",
        "Caught + uncaught" => "Capturados + não capturados",
        "Uncaught only" => "Só não capturados",
        "Name" => "Nome",
        "Level" => "Nível",
        "Wait" => "Aguarde",
        "Current" => "Atual",
        "Go" => "Ir",
        "Gold" => "Ouro",
        "Gems" => "Gemas",
        "listings" => "ofertas",
        "seller" => "vendedor",
        "Buy" => "Comprar",
        "Profiles" => "Perfis",
        "Account, browser and addon entry points" => "Pontos de acesso a contas, navegador e addons",
        "Close" => "Fechar",
        "Open Game" => "Abrir jogo",
        "Addons" => "Addons",
        "Install Violentmonkey" => "Instalar Violentmonkey",
        "Violentmonkey" => "Violentmonkey",
        "Installed" => "Instalado",
        "Not installed" => "Não instalado",
        "Not configured" => "Não configurado",
        "Not checked" => "Não verificado",
        "Tab open" => "Aba aberta",
        "Scripts" => "Scripts",
        "listing" => "oferta",
        "live" => "ao vivo",
        "opened" => "abertas",
        "Running" => "Em execução",
        "Connecting" => "Conectando",
        "Reconnecting" => "Reconectando",
        "is already running." => "já está em execução.",
        "is already stopped." => "já está parado.",
        "Firefox closed." => "Firefox fechado.",
        "Launch enabled requested · monitoring will update as Firefox becomes ready" => "Iniciar habilitadas solicitado · o monitor atualizará quando o Firefox estiver pronto",
        "Could not determine LOCALAPPDATA for logs." => "Não foi possível determinar LOCALAPPDATA para os logs.",
        "Could not create log folder: " => "Não foi possível criar a pasta de logs: ",
        "Could not open log: " => "Não foi possível abrir o log: ",
        "Could not create profiles folder: " => "Não foi possível criar a pasta de perfis: ",
        "Could not open folder: " => "Não foi possível abrir a pasta: ",
        "before you can change hunts" => "antes de poder mudar a caça",
        "could not create profile directory: " => "não foi possível criar a pasta do perfil: ",
        "could not open profile folder: " => "não foi possível abrir a pasta do perfil: ",
        "could not read account configuration: " => "não foi possível ler a configuração das contas: ",
        "invalid account configuration: " => "configuração de contas inválida: ",
        "could not create account data directory: " => "não foi possível criar a pasta de dados das contas: ",
        "could not serialize account configuration: " => "não foi possível serializar a configuração das contas: ",
        "could not save account configuration: " => "não foi possível salvar a configuração das contas: ",
        "could not determine the Windows local application data directory" => "não foi possível determinar a pasta de dados local do Windows",
        "could not start Firefox: " => "não foi possível iniciar o Firefox: ",
        "refusing to open a non-HTTPS URL" => "recusa ao abrir uma URL que não usa HTTPS",
        "could not start normal KICK Firefox: " => "não foi possível iniciar o Firefox normal do KICK: ",
        "could not request a new KICK Firefox window: " => "não foi possível solicitar uma nova janela do Firefox do KICK: ",
        "could not create KICK profile: " => "não foi possível criar o perfil do KICK: ",
        "BiDi" => "BiDi",
        "nick" => "apelido",
        "Twitch" => "Twitch",
        "KICK" => "KICK",
        "Test KICK headless" => "Testar KICK em modo oculto",
        "Test current KICK streams in headless Firefox. Close the current managed KICK browser before switching modes." => "Testa as streams atuais do KICK no Firefox em modo oculto. Feche o navegador KICK gerenciado antes de mudar de modo.",
        "Dashboard · live health polling enabled" => "Painel · monitoramento de saúde ao vivo ativado",
        "Loaded" => "Carregado",
        "Auto Catch" => "Captura automática",
        "Off" => "Desativado",
        "Performance+" => "Performance+",
        "LOW" => "BAIXO",
        "ago" => "atrás",
        "Twitch low resource" => "Twitch baixo consumo",
        "isolated" => "isolado(s)",
        "Pokémon" => "Pokémon",
        "Trainer Lv" => "Nível do treinador",
        "TYPE ?" => "TIPO ?",
        "model" => "modelo",
        "ignored" => "ignorada",
        "Userscript engine" => "Motor de userscripts",
        "Connected" => "Conectado",
        "Disconnected" => "Desconectado",
        "Items" => "Itens",
        "Candidates" => "Candidatos",
        "protocol" => "protocolo",
        "Purchase command accepted" => "Comando de compra aceito",
        "Purchase pending" => "Compra pendente",
        "References" => "Referências",
        "items" => "itens",
        "units" => "unidades",
        "total" => "total",
        "Coin" => "Moeda",
        "Gem" => "Gema",
        "Irregular" => "Irregular",
        "Currency" => "Moeda",
        "Coins" => "Moedas",
        "Newest detected" => "Mais recentes detectadas",
        "Price: low → high" => "Preço: menor → maior",
        "Price: high → low" => "Preço: maior → menor",
        "Reference: high → low" => "Referência: maior → menor",
        "Quantity: high → low" => "Quantidade: maior → menor",
        "Name: A → Z" => "Nome: A → Z",
        "Biggest discount" => "Maior desconto",
        "listed" => "anunciado",
        "Seller:" => "Vendedor:",
        "retained" => "retido",
        "available" => "disponível",
        "Pending" => "Pendente",
        "Balance" => "Saldo",
        "baseline refresh requested" => "atualização da linha de base solicitada",
        "updated." => "atualizado.",
        "last seen" => "visto pela última vez",
        "Twitch uses this Firefox profile for its session. KICK uses the account's dedicated KICK profile. Opening either action always targets the corresponding account." => "A Twitch usa este perfil do Firefox para sua sessão. O KICK usa o perfil dedicado desta conta. Abrir qualquer uma das opções sempre usa a conta correspondente.",
        "addon installers" => "instaladores de addons",
        "addon installers opened" => "instaladores de addons abertos",
        "Firefox was not found. Install Firefox or set MOTH_FIREFOX to firefox.exe." => "O Firefox não foi encontrado. Instale o Firefox ou defina MOTH_FIREFOX para firefox.exe.",
        "Live state for enabled isolated Firefox profiles" => "Estado atual dos perfis isolados do Firefox habilitados",
        "Recent bot activity" => "Atividade recente do bot",
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

#[derive(Clone, Debug, Default)]
struct AccountSetup {
    profile_ready: bool,
    kick_profile_ready: bool,
    violentmonkey_installed: bool,
}

pub struct ControllerApp {
    games: [GameSlot; 4],
    show_accounts: bool,
    accounts: [AccountConfig; 4],
    status: String,
    status_error: bool,
    kick_manager: KickManager,
    account_setup: [AccountSetup; 4],
    account_setup_checked_at: Option<Instant>,
    kick_headless_test: [bool; 4],
    tab_url_input: [String; 4],
    tab_url_open: [bool; 4],
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
            status: localize_status("Ready · launch only the profiles you need".to_string()),
            status_error: false,
            kick_manager: KickManager::new(),
            account_setup: std::array::from_fn(|_| AccountSetup::default()),
            account_setup_checked_at: None,
            kick_headless_test: [false; 4],
            tab_url_input: std::array::from_fn(|_| String::new()),
            tab_url_open: [false; 4],
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

    fn refresh_account_setup(&mut self) {
        for profile in GameProfile::ALL {
            let index = profile.index();
            let Ok(config) = Config::for_profile(profile) else {
                self.account_setup[index] = AccountSetup::default();
                continue;
            };

            let vm_installed = accounts::violentmonkey_installed(profile).unwrap_or(false);

            self.account_setup[index] = AccountSetup {
                profile_ready: config.profile_dir.exists(),
                kick_profile_ready: config.kick_profile_dir().exists(),
                violentmonkey_installed: vm_installed,
            };
        }

        self.account_setup_checked_at = Some(Instant::now());
    }

    fn sync_kick_streams(&mut self) {
        for profile in GameProfile::ALL {
            let health = self.games[profile.index()].health();
            self.kick_manager.sync(
                profile,
                &health.kick_streams,
                health.kick_state_available,
                self.kick_headless_test[profile.index()],
            );
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

    fn profile_action(&mut self, profile: GameProfile, action: ProfileAction) {
        let result = match action {
            ProfileAction::Game => accounts::open_game(profile).map(|_| "Opened game".to_string()),
            ProfileAction::Twitch => accounts::open_login(profile, accounts::TWITCH_LOGIN, "Twitch"),
            ProfileAction::Kick => self
                .kick_manager
                .open_login(profile, accounts::KICK_LOGIN)
                .map(|_| "Opened KICK login in uncontrolled normal Firefox · close it after authentication".to_string()),
            ProfileAction::InstallViolentmonkey => accounts::open_violentmonkey(profile),
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
        self.kick_manager.shutdown();

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
        self.sync_kick_streams();

        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));

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
    }
}

#[derive(Clone, Copy)]
enum ProfileAction {
    Game,
    Twitch,
    Kick,
    InstallViolentmonkey,
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
                app.set_status(tr("Dashboard · live health polling enabled").to_string(), false);
            }

            if sidebar_button(ui, tr("◫  Accounts"), false).clicked() {
                app.show_accounts = true;
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
                    RichText::new(&account_name)
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
                            format!("{} · {}", tr("Hunting"), compact_text(&health.hunt, 34))
                        }
                        "Center" => tr("Online · Center").to_string(),
                        "Login" => tr("Waiting for login").to_string(),
                        _ => localize_status(health.summary()),
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

                            if health.autocatch_on { tr("ON") } else { tr("OFF") },
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
                tr("Loaded").to_string()
            } else {
                format!("{} FPS", health.performance_fps)
            };

            ui.horizontal_wrapped(|ui| {
                automation_badge(
                    ui,
                    tr("Auto Catch"),
                    if health.autocatch_on { tr("Active") } else { tr("Off") },
                    health.autocatch_on,
                );
                automation_badge(
                    ui,
                    tr("Restock"),
                    if health.autocatch_restock.is_empty() { tr("Off") } else { tr(&health.autocatch_restock) },
                    health.autocatch_on && !health.autocatch_restock.eq_ignore_ascii_case("off"),
                );
                automation_badge(
                    ui,
                    tr("Stream scanner"),
                    if health.stream_scan_status.is_empty() {
                        tr("Waiting")
                    } else {
                        tr(&health.stream_scan_status)
                    },
                    health.stream_scan_status.eq_ignore_ascii_case("ok"),
                );
                if health.stream_scan_live > 0 || health.stream_scan_opened > 0 {
                    let streams_value = format!(
                        "{} {} · {} {}",
                        health.stream_scan_live,
                        tr("live"),
                        health.stream_scan_opened,
                        tr("opened")
                    );

                    automation_badge(
                        ui,
                        "Streams",
                        &streams_value,
                        true,
                    );
                }
                automation_badge(ui, tr("Performance+"), &performance_value, true);
            });

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr("OPEN TABS"))
                        .size(9.0)
                        .strong()
                        .color(DIM),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("+")
                                    .size(13.0)
                                    .strong()
                            )
                            .min_size(egui::vec2(24.0, 22.0)),
                        )
                        .on_hover_text(tr("Open a custom URL in this Firefox profile"))
                        .clicked()
                    {
                        app.tab_url_open[index] = !app.tab_url_open[index];
                    }
                });
            });
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    if app.tab_url_open[index] {
                        ui.horizontal(|ui| {
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut app.tab_url_input[index])
                                    .hint_text(tr("https://example.com"))
                                    .desired_width((width - 150.0).max(180.0))
                            );

                            let enter = response.lost_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::Enter));

                            if ui.button(tr("Open")).clicked() || enter {
                                let raw = app.tab_url_input[index].trim();
                                if raw.is_empty() {
                                    app.set_status(
                                        format!(
                                            "{}: {}",
                                            &account_name,
                                            tr("Enter a URL.")
                                        ),
                                        true,
                                    );
                                } else {
                                    let url = if raw.starts_with("https://") || raw.starts_with("http://") {
                                        raw.to_string()
                                    } else {
                                        format!("https://{}", raw)
                                    };

                                    app.games[index].monitor.as_ref().map(|monitor| {
                                        monitor.send(json!({
                                            "t": "browser.openTab",
                                            "url": url
                                        }));
                                    });

                                    app.tab_url_input[index].clear();
                                    app.tab_url_open[index] = false;
                                    app.set_status(
                                        format!(
                                            "{} · {}",
                                            &account_name,
                                            tr("custom tab requested")
                                        ),
                                        false,
                                    );
                                }
                            }

                            if ui.button("×").clicked() {
                                app.tab_url_open[index] = false;
                            }
                        });

                        ui.add_space(6.0);
                    }

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
                                "{} · +{}% XP",
                                tr("ACTIVE"),
                                format_pct(health.stream_bonus_pct)
                            ))
                            .size(11.0)
                            .strong()
                            .color(GOOD),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                tr("Watching:"),
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
                                "{} {}{}",
                                tr("Open chat:"),
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
                                "{} · {}{}: {}",
                                tr("No active bonus"),
                                last_seen,
                                tr("last seen"),
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
                                if tab.low_resource { tr("LOW") } else { tr("FULL") }
                            } else {
                                tr("OPEN")
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
                        RichText::new(format!("{} {}s {}", tr("DATA"), age, tr("ago")))
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
                    RichText::new(format!("{} {}/{}", tr("ADDONS"), health.addon_ok, health.addon_total))
                        .size(10.0)
                        .strong()
                        .color(addon_color),
                );

                if !health.addon_missing.is_empty() {
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("{} {}", tr("Missing:"), health.addon_missing.join(" · ")))
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
                                "{} {}/{}",
                                tr("Twitch low resource"),
                                health.twitch_low_resource_ok,
                                health.twitch_tabs
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
                ui.label(RichText::new(compact_text(tr(value), 24)).size(9.0).color(TEXT));
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
                runtime_chip(ui, tr("Firefox"), tr("Headless"), GOOD);
                runtime_chip(ui, tr("BiDi"), tr("Active"), GOOD);
                let enabled_profiles = app.accounts.iter().filter(|account| account.enabled).count();
                runtime_chip(
                    ui,
                    tr("Profiles"),
                    &format!("{} {}", enabled_profiles, tr("isolated")),
                    MUTED,
                );
                runtime_chip(ui, tr("Stream chat"), tr("First scan 30s · 10 min"), MUTED);
                runtime_chip(ui, tr("UI repaint"), tr("1 sec"), MUTED);
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

fn draw_accounts_window(app: &mut ControllerApp, ctx: &egui::Context) {
    if app
        .account_setup_checked_at
        .map(|checked| checked.elapsed() >= Duration::from_secs(1))
        .unwrap_or(true)
    {
        app.refresh_account_setup();
    }

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

            ui.add_space(12.0);

            egui::ScrollArea::vertical()
                .id_salt("account_manager_list")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for profile in GameProfile::ALL {
                        let index = profile.index();
                        let enabled = app.accounts[index].enabled;
                        let setup = app.account_setup[index].clone();
                        let health = app.games[index].health();
                        let running = app.games[index].is_running();

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

                                    let checkbox_glyph = if enabled { "✓" } else { "" };
                                    let checkbox = egui::Button::new(
                                        RichText::new(checkbox_glyph)
                                            .size(13.0)
                                            .strong()
                                            .color(if enabled { TEXT } else { DIM }),
                                    )
                                    .fill(if enabled {
                                        ACCENT.linear_multiply(0.22)
                                    } else {
                                        PANEL
                                    })
                                    .stroke(Stroke::new(
                                        1.0,
                                        if enabled { ACCENT } else { BORDER },
                                    ))
                                    .corner_radius(5.0);

                                    if ui.add_sized([28.0, 28.0], checkbox).clicked() {
                                        app.accounts[index].enabled = !enabled;
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
                                        egui::TextEdit::singleline(&mut app.accounts[index].name)
                                            .hint_text(profile.label()),
                                    );
                                    changed |= response.changed();

                                    if app.accounts[index].name.trim().is_empty() {
                                        ui.colored_label(WARN, tr("Name cannot be empty"));
                                    }
                                });

                                ui.add_space(8.0);

                                ui.horizontal_wrapped(|ui| {
                                    let profile_label = if setup.profile_ready {
                                        tr("Profile ready")
                                    } else {
                                        tr("Not configured")
                                    };
                                    let vm_label = if setup.violentmonkey_installed {
                                        tr("Installed")
                                    } else {
                                        tr("Not installed")
                                    };
                                    let twitch_open = health.tabs.iter().any(|tab| tab.kind == "Twitch");
                                    // These checks only observe profile/tab existence. They do not
                                    // authenticate Twitch or KICK, so never label a profile as logged in.
                                    let twitch_label = if twitch_open {
                                        tr("Tab open")
                                    } else {
                                        tr("Not checked")
                                    };
                                    let kick_label = tr("Not checked");

                                    status_chip(ui, "Firefox", profile_label, setup.profile_ready);
                                    status_chip(ui, tr("Violentmonkey"), vm_label, setup.violentmonkey_installed);
                                    status_chip(ui, "Twitch", twitch_label, twitch_open);
                                    status_chip(ui, "KICK", kick_label, false);

                                    let headless_changed = ui
                                        .checkbox(
                                            &mut app.kick_headless_test[index],
                                            tr("Test KICK headless"),
                                        )
                                        .on_hover_text(tr("Test current KICK streams in headless Firefox. Close the current managed KICK browser before switching modes."))
                                        .changed();

                                    if headless_changed {
                                        app.kick_manager.set_headless_test(
                                            profile,
                                            app.kick_headless_test[index],
                                        );
                                    }

                                    if running {
                                        status_chip(
                                            ui,
                                            tr("Scripts"),
                                            &format!("{}/{}", health.addon_ok, health.addon_total),
                                            health.addon_ok == health.addon_total,
                                        );
                                    } else {
                                        status_chip(ui, tr("Scripts"), tr("Not checked"), false);
                                    }
                                });

                                ui.add_space(8.0);

                                ui.label(
                                    RichText::new(
                                        tr("Twitch uses this Firefox profile for its session. KICK uses the account's dedicated KICK profile. Opening either action always targets the corresponding account."),
                                    )
                                    .size(9.0)
                                    .color(DIM),
                                );

                                ui.add_space(8.0);

                                ui.horizontal_wrapped(|ui| {
                                    profile_button(ui, tr("Open Game"), || {
                                        app.profile_action(profile, ProfileAction::Game)
                                    });
                                    profile_button(ui, tr("Twitch"), || {
                                        app.profile_action(profile, ProfileAction::Twitch)
                                    });
                                    profile_button(ui, tr("KICK"), || {
                                        app.profile_action(profile, ProfileAction::Kick)
                                    });
                                    profile_button(ui, tr("Install Violentmonkey"), || {
                                        app.profile_action(profile, ProfileAction::InstallViolentmonkey)
                                    });
                                    profile_button(ui, tr("Addons"), || {
                                        app.profile_action(profile, ProfileAction::Addons)
                                    });
                                    profile_button(ui, tr("Profile folder"), || {
                                        app.profile_action(profile, ProfileAction::Folder)
                                    });
                                });
                            });

                        if changed {
                            let trimmed = app.accounts[index].name.trim().to_string();
                            app.accounts[index].name = if trimmed.is_empty() {
                                profile.label().to_string()
                            } else {
                                trimmed
                            };
                            app.save_account_config();
                        }

                        ui.add_space(10.0);
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

fn status_chip(ui: &mut egui::Ui, name: &str, state: &str, good: bool) {
    let color = if good { GOOD } else { DIM };
    ui.label(
        RichText::new(format!("{}: {}", tr(name), tr(state)))
            .size(9.0)
            .color(color),
    );
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
        RichText::new(format!("● {}", tr(label)))
            .size(10.0)
            .strong()
            .color(color),
    );
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
