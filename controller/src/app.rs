use crate::accounts;
use crate::config::{AccountConfig, Config, GameProfile, UiOptions};
use crate::firefox;
use crate::kick::KickStream;
use crate::native_kick::KickMonitor;
use crate::logging;
use crate::native_background::NativeSession;
use crate::native_gameplay::GameCommand;
use crate::native_hunts::{self, HuntView};
use crate::native_capture_config::CatchSettings;
use crate::native_items::{self, ItemCatalog};
use crate::native_visuals::{Kind as SpriteKind, VisualCache};
use crate::native_atlas::{self, Reference as AtlasReference};
mod hunt_panel;
mod market_panel;
mod kick_panel;
mod translations;
use hunt_panel::HuntSubtab;
use crate::monitor::{
    Health, MonitorHandle, MonitorPace, MONITOR_HEARTBEAT_INTERVAL,
    MONITOR_POLL_INTERVAL, MONITOR_TAB_INTERVAL,
};
use crate::update::{self, SharedUpdateStatus};
use serde_json::json;
use eframe::egui::{self, Align, Color32, FontId, Layout, Margin, RichText, Stroke, TextStyle};
use std::collections::VecDeque;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use std::sync::atomic::{AtomicBool, Ordering};

use std::sync::Arc;
#[cfg(windows)]
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

static PT_BR: AtomicBool = AtomicBool::new(true);

// Redraw cached information more often without triggering additional BiDi reads.
const UI_REPAINT_INTERVAL: Duration = Duration::from_secs(3);
const UI_BACKGROUND_REPAINT_INTERVAL: Duration = Duration::from_secs(30);

// eframe/egui reports native window focus via ViewportInfo. Treat missing
// focus information as foreground instead of disabling live monitoring on
// platforms where the OS cannot report it.
fn controller_is_background(hidden_in_tray: bool, focused: Option<bool>) -> bool {
    hidden_in_tray || focused == Some(false)
}

/// Tray state has precedence over focus. The two toggles are independent:
/// disabling blur throttling need not disable the tray resource-saving mode.
fn reduce_updates_for_window(
    hidden_in_tray: bool,
    focused: Option<bool>,
    options: UiOptions,
) -> bool {
    if hidden_in_tray {
        options.reduce_updates_in_tray
    } else {
        options.reduce_updates_unfocused && controller_is_background(false, focused)
    }
}

fn monitor_pace_for_slot(
    expanded: Option<usize>,
    index: usize,
    enabled: bool,
    background: bool,
) -> MonitorPace {
    if background {
        MonitorPace::Background
    } else if enabled && expanded == Some(index) {
        MonitorPace::Detailed
    } else {
        MonitorPace::Compact
    }
}

// Normal throttling, an old heartbeat, and transient BiDi probe failures
// must not encourage a Firefox restart. Only a confirmed session-slot
// failure or an existing Firefox profile with no monitor offers repair.
fn monitor_repair_available(
    running: bool,
    last_error: Option<&str>,
    orphaned_without_monitor: bool,
) -> bool {
    running
        && (orphaned_without_monitor
            || last_error.is_some_and(|error| {
                error.contains("Maximum number of active sessions")
            }))
}

#[cfg(test)]
mod adaptive_monitor_ui_tests {
    use super::{
        controller_is_background, monitor_pace_for_slot, monitor_repair_available,
        reduce_updates_for_window, MonitorPace, UiOptions,
    };

    #[test]
    fn inactive_window_and_system_tray_share_background_mode() {
        assert!(!controller_is_background(false, Some(true)));
        assert!(!controller_is_background(false, None)); // unknown: don't throttle
        assert!(controller_is_background(false, Some(false))); // alt-tab
        assert!(controller_is_background(true, Some(true))); // tray override
        assert!(controller_is_background(true, Some(false)));
        assert!(controller_is_background(true, None));
    }

    #[test]
    fn background_update_switches_are_independent_and_default_to_old_behavior() {
        let default = UiOptions::default();
        assert!(reduce_updates_for_window(false, Some(false), default));
        assert!(reduce_updates_for_window(true, Some(true), default));
        assert!(!reduce_updates_for_window(false, Some(true), default));

        let normal_unfocused = UiOptions {
            reduce_updates_unfocused: false,
            reduce_updates_in_tray: true,
        };
        assert!(!reduce_updates_for_window(false, Some(false), normal_unfocused));
        assert!(reduce_updates_for_window(true, Some(false), normal_unfocused));

        let full_speed_tray = UiOptions {
            reduce_updates_unfocused: true,
            reduce_updates_in_tray: false,
        };
        assert!(reduce_updates_for_window(false, Some(false), full_speed_tray));
        assert!(!reduce_updates_for_window(true, Some(false), full_speed_tray));

        let full_speed_everywhere = UiOptions {
            reduce_updates_unfocused: false,
            reduce_updates_in_tray: false,
        };
        for hidden in [true, false] {
            for focused in [Some(true), Some(false), None] {
                assert!(!reduce_updates_for_window(hidden, focused, full_speed_everywhere));
            }
        }
    }

    #[test]
    fn only_the_visible_expanded_account_gets_fast_polling() {
        assert_eq!(monitor_pace_for_slot(Some(2), 2, true, false), MonitorPace::Detailed);
        assert_eq!(monitor_pace_for_slot(Some(2), 1, true, false), MonitorPace::Compact);
        assert_eq!(monitor_pace_for_slot(Some(2), 2, false, false), MonitorPace::Compact);
        assert_eq!(monitor_pace_for_slot(None, 2, true, false), MonitorPace::Compact);
        for index in 0..4 {
            assert_eq!(
                monitor_pace_for_slot(Some(2), index, true, true),
                MonitorPace::Background
            );
        }
        // The expanded selection remains set while focus is lost, but no
        // account may keep five-second polling.
        let background = controller_is_background(false, Some(false));
        for index in 0..4 {
            assert_eq!(
                monitor_pace_for_slot(Some(2), index, true, background),
                MonitorPace::Background
            );
        }
        assert_eq!(
            monitor_pace_for_slot(Some(2), 2, true, controller_is_background(false, Some(true))),
            MonitorPace::Detailed
        );
    }

    #[test]
    fn transient_monitor_failures_do_not_prompt_firefox_restart() {
        // Normal compact, expanded, and background monitoring never requires
        // a restart merely because its most recent reading is stale.
        assert!(!monitor_repair_available(true, None, false));
        for error in [
            "IO error: A connection attempt failed (os error 10060)",
            "Firefox has no PokéIdle app context",
            "BiDi command 7 timed out while waiting for its response",
            "IO error: An established connection was aborted (os error 10053)",
        ] {
            assert!(!monitor_repair_available(true, Some(error), false), "{error}");
        }

        // The repair button remains for actual session exhaustion or an
        // existing Firefox process whose monitor no longer exists.
        assert!(monitor_repair_available(
            true, Some("Maximum number of active sessions"), false
        ));
        assert!(monitor_repair_available(true, None, true));
        assert!(!monitor_repair_available(
            false, Some("Maximum number of active sessions"), true
        ));
    }
}

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
        ("Firefox was not found. Install Firefox or set MOTH_FIREFOX to its executable path.", "O Firefox não foi encontrado. Instale o Firefox ou defina MOTH_FIREFOX para o caminho do executável."),
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
        ("Some Firefox instances could not be stopped; check the log.", "Algumas instâncias do Firefox não puderam ser fechadas; consulte o log."),
        ("could not determine XDG_DATA_HOME or HOME", "não foi possível determinar XDG_DATA_HOME ou HOME"),
        ("Could not create log: ", "Não foi possível criar o log: "),
        ("Firefox already uses this profile", "O Firefox já está usando este perfil"),
        ("Close the existing Firefox profile before switching modes.", "Feche o perfil do Firefox existente antes de trocar de modo."),
        ("Use Stop before launching.", "Use Parar antes de iniciar."),
        ("Firefox profile inspection failed: ", "Falha ao verificar os perfis do Firefox: "),
        ("could not inspect running Firefox profiles: ", "não foi possível verificar os perfis ativos do Firefox: "),
        ("Firefox BiDi port ", "A porta BiDi do Firefox "),
        (" is already in use or unavailable: ", " já está em uso ou indisponível: "),
        ("Firefox profile process changed; refresh and try again", "O processo do perfil do Firefox mudou; atualize e tente novamente"),
        ("Firefox profile process changed during shutdown", "O processo do perfil do Firefox mudou durante o encerramento"),
        ("could not terminate recovered Firefox: ", "não foi possível encerrar o Firefox recuperado: "),
        ("recovered Firefox did not exit after termination request", "O Firefox recuperado não encerrou após a solicitação"),
        ("could not check Firefox process: ", "não foi possível verificar o processo do Firefox: "),
        ("could not terminate Firefox: ", "não foi possível encerrar o Firefox: "),
        ("could not reap Firefox: ", "não foi possível finalizar a limpeza do processo do Firefox: "),
        ("Opened controller log · ", "Log do controlador aberto · "),
        ("Opened profile folder", "Pasta do perfil aberta"),
        ("Opened game", "Jogo aberto"),
        (" opened Firefox without remote debugging for Google sign-in. After signing in, close that window and launch this account normally from the dashboard.", " abriu o Firefox sem depuração remota para login do Google. Após entrar, feche essa janela e inicie esta conta normalmente pelo painel."),
        ("Opened profile folder", "Pasta do perfil aberta"),
        ("Opened ", "Aberto: "),
        ("Could not ", "Não foi possível "),
        (" is not running.", " não está em execução."),
        ("switching Firefox to ", "mudando o Firefox para o modo "),
        (" mode...", " ..."),
        ("started · ", "iniciado · "),
        (": installer opened", ": instalador aberto"),
        ("installer opened", "instalador aberto"),
    ] {
        value = value.replace(en, pt);
    }
    value
}

fn tr_dynamic(value: String) -> String {
    if pt_br() { translations::dynamic(tr(&value).to_owned()) } else { value }
}

fn tr<'a>(en: &'a str) -> &'a str {
    if !pt_br() {
        return en;
    }

    match en {
        "Idle warning: no EXP or Pokéballs used in 5 minutes" => "Aviso de inatividade: sem EXP ou Pokébolas usadas por 5 minutos",
        "Overview" => "Visão geral",
        "ACCOUNT OVERVIEW" => "VISÃO GERAL DAS CONTAS",
        "Summary" => "Resumo",
        "XP/h total" => "XP/h total",
        "Gold/h total" => "Ouro/h total",
        "Online accounts" => "Contas online",
        "Live gold" => "Ouro atual",
        "Live gems" => "Gemas atuais",
        "Verified online balances only; unavailable accounts are not counted." =>
            "Somente saldos online confirmados; contas indisponíveis não são somadas.",
        "Rates are measured, never projected; unavailable data stays blank." =>
            "Taxas medidas, não projetadas; dados indisponíveis ficam em branco.",
        "Start an account to see session details" =>
            "Inicie uma conta para ver os detalhes da sessão",
        "Session duration" => "Tempo de sessão",
        "Rare drops" => "Drops raros",
        "Boss Token" => "Token de boss",
        "Kills" => "Abates",
        "Captures" => "Capturas",
        "Key" => "Chave",
        "Bicycle" => "Bicicleta",
        "HP" => "HP",
        "Your Pokémon" => "Seu Pokémon",
        "Opponent" => "Adversário",
        "Fighting" => "Lutando",
        "Between encounters" => "Entre batalhas",
        "At Pokémon Center" => "No Centro Pokémon",
        "Waiting for combat" => "Aguardando combate",
        "No combat activity" => "Sem atividade de combate",
        "No live connection" => "Sem conexão ativa",
        "Blocked" => "Bloqueada",
        "Auto Catch disabled" => "Captura automática desligada",
        "Auto Catch on" => "Ligado",
        "Auto Catch off" => "Desligado",
        "Capture operation" => "Situação da captura",
        "Auto Catch awaiting game connection" => "Aguardando conexão com o jogo",
        "Auto Catch waiting for capture results" => "Aguardando resposta de capturas pendentes",
        "Auto Catch ready for targets" => "Pronto para capturar",
        "Auto Catch waiting for conditions" => "Aguardando condições de captura",
        "Throws sent" => "Arremessos enviados",
        "Auto Catch matched captures" => "Capturas vinculadas",
        "Auto Catch matched failures" => "Falhas vinculadas",
        "Pending capture receipts" => "Arremessos sem resposta",
        "Saving Auto Catch settings" => "Salvando configurações de captura",
        "Capture diagnostics" => "Diagnóstico da captura",
        "Auto Catch · ball statistics" => "Captura automática · estatísticas por bola",
        "Native capture receipt overdue; waiting without disconnecting" =>
            "Resposta da captura atrasada; mantendo conexão com o jogo",
        "Waiting for delayed capture results" => "Aguardando resultados de capturas atrasadas",
        "Waiting for authenticated game state" => "Aguardando estado autenticado do jogo",
        "Pokémon captured" => "Pokémon capturado",
        "The Pokémon resisted; retrying" => "Pokémon resistiu; tentando novamente",
        "Throwing a Poké Ball" => "Arremessando Pokébola",
        "Trying another Poké Ball" => "Tentando outra Pokébola",
        "At center or waiting for authenticated hunt state" => "No centro ou aguardando hunt autenticada",
        "No capture ball selected" => "Nenhuma Pokébola escolhida",
        "Waiting for server Poké Ball catalog" => "Aguardando catálogo de Pokébolas do servidor",
        "Waiting for server Poké Ball inventory" => "Aguardando inventário de Pokébolas do servidor",
        "Ball-event captures" => "Capturas em eventos de bola",
        "Ball-event failures" => "Falhas em eventos de bola",
        "Native interval: 0.6 s, with a bounded pending-throw window" =>
            "Intervalo nativo: 0,6 s, com limite de lançamentos pendentes",
        "Capture server notice" => "Aviso do servidor sobre captura",
        "Capture resyncs" => "Reconexões para recuperar capturas",
        "Capture timeouts" => "Capturas com resposta atrasada",
        "Only observed ball battle events are counted; built-in VIP captures may not generate them." =>
            "Só são contados eventos de bola observados; capturas automáticas VIP do jogo podem não gerar esses eventos.",
        "No server response to native Poké Ball throw; refreshing game state" =>
            "Sem resposta do servidor ao lançamento; atualizando a sessão do jogo",
        "Wild death events lack capture slots; cannot target throws" =>
            "Eventos de derrota não trazem slot de captura; impossível lançar Pokébolas",
        "No selected Poké Balls above reserve; check restock and ball priorities" =>
            "Sem Pokébolas selecionadas acima da reserva; confira reposição e prioridades",
        "Selected Poké Balls reserved; waiting for server consumption confirmation" =>
            "Pokébolas reservadas; aguardando confirmação de consumo do servidor",
        "Waiting for in-flight capture results" =>
            "Aguardando resposta de capturas pendentes",
        "Confirmed by server" => "Confirmadas pelo servidor",
        "Downed targets" => "Alvos caídos",
        "Server failures" => "Falhas no servidor",
        "Native-matched" => "Vinculadas ao Auto Catch",
        "Unmatched receipts" => "Respostas sem correspondência",
        "Stock awaiting confirmation" => "Estoque aguardando confirmação",
        "Server receipt totals may also include captures outside native Auto Catch." =>
            "As confirmações do servidor também podem incluir capturas fora do Auto Catch nativo.",
        "Restock buys a ball that Auto Catch does not use" =>
            "A reposição compra uma Pokébola que a captura automática não utiliza",
        "wild death events without capture slots" =>
            "eventos de derrota sem slot de captura",
        "Last combat activity" => "Último combate",
        "Gold/h" => "Ouro/h",
        "XP/h" => "XP/h",
        "Live" => "Ao vivo",
        "Offline" => "Desconectado",
        "Options" => "Opções",
        "Automation" => "Automação",
        "Selected item only" => "Apenas o item selecionado",
        "No restock item selected" => "Nenhum item de reposição selecionado",
        "Stale" => "Dados antigos",
        "Explore Pokémon" => "Explorar Pokémon",
        "Hunt rankings" => "Ranking de hunts",
        "Capture · Outland · Economy" => "Captura · Outland · Economia",
        "Session & rewards" => "Sessão e recompensas",
        "Compare accounts" => "Comparar contas",
        "← Back to explorer" => "← Voltar à exploração",
        "Account settings" => "Configurações da conta",
        "Waiting for data" => "Aguardando dados",
        "Capture preferences and live statistics" => "Preferências e estatísticas de captura",
        "Server settings and restock status" => "Opções do jogo e status da reposição",
        "Recovery & selling" => "Recuperação e venda",
        "Pokémon protection" => "Proteção de Pokémon",
        "Automatic buying rules and Snatch" => "Regras de compra e Snatch",
        "Game & restock" => "Jogo e reposição",
        "Market" => "Mercado",
        "Launch enabled" => "Iniciar habilitadas",
        "Stop all" => "Parar todas",
        "To tray" => "Para bandeja",
        "Configure automation" => "Configurar automação",
        "Open restock on Overview" => "Abrir reposição na Visão geral",
        "Start this account to configure Auto Catch" => "Inicie esta conta para configurar a captura automática",
        "History" => "Histórico",
        "Battle history" => "Histórico de batalhas",
        "Up to 90 days of battle history, saved locally for each account." =>
            "Até 90 dias de histórico de batalhas, salvos no próprio dispositivo para cada conta.",
        "Recorded totals" => "Totais registrados",
        "Reload saved history" => "Recarregar histórico salvo",
        "Daily hunts" => "Hunts por dia",
        "Lifetime totals" => "Totais de todos os tempos",
        "No recorded battles yet" => "Nenhuma batalha registrada",
        "Start this account to load its saved history." =>
            "Inicie esta conta para carregar seu histórico salvo.",
        "History begins when this version first records a battle. Previous in-memory sessions cannot be recovered retroactively." =>
            "O histórico começa nas primeiras batalhas desta versão. Sessões anteriores não podem ser recuperadas.",

        "Pokémon management" => "Gerenciamento de Pokémon",
        "Pokémon on account" => "Pokémon na conta",
        "Shiny only" => "Apenas shiny",
        "Sort by" => "Ordenar por",
        "Grade" => "Nota",
        "Power" => "Poder",
        "Search Pokémon" => "Buscar Pokémon",
        "Minimum total IV" => "IV total mínimo",
        "All types" => "Todos os tipos",
        "Quality" => "Qualidade",
        "IV total" => "IV total",
        "Select a Pokémon to view its details." => "Selecione um Pokémon para ver seus detalhes.",
        "Selected Pokémon" => "Pokémon selecionado",
        "Sell Pokémon" => "Vender Pokémon",
        "Confirm sale" => "Confirmar venda",
        "Cannot sell active or shiny Pokémon." => "Não é possível vender Pokémon ativo ou shiny.",
        "The sale is complete only when the game confirms it." =>
            "A venda só é concluída quando o jogo a confirma.",
        "Waiting for Pokémon roster" => "Aguardando lista de Pokémon",
        "Open Automation" => "Abrir automação",
        "Game automation" => "Automação do jogo",
        "Reload game settings" => "Recarregar configurações do jogo",
        "Awaiting confirmation" => "Aguardando confirmação",
        "Confirmed" => "Confirmado",
        "Changes only take effect after PokéIdle confirms them." => "As alterações só entram em vigor depois da confirmação do PokéIdle.",
        "Potion HP threshold (%)" => "Limiar de HP para poção (%)",
        "Minimum lock grade" => "Nota mínima de bloqueio",
        "Apply HP" => "Aplicar HP",
        "Apply grade" => "Aplicar nota",
        "Apply" => "Aplicar",
        "In-game nickname" => "Apelido no jogo",
        "Field map" => "Mapa de combate",
        "in stock" => "em estoque",
        "Price unavailable" => "Preço indisponível",
        "No valid restock rules configured" => "Nenhuma regra válida de reposição configurada",
        "These controls are available after signing in." =>
            "Estes controles ficam disponíveis depois do login.",
        "Choose an account to inspect its automation and restock status." =>
            "Escolha uma conta para verificar a automação e o estado da reposição.",
        "Restock details" => "Detalhes da reposição",
        "Waiting for account connection" => "Aguardando conexão com a conta",
        "Performance and updates" => "Desempenho e atualizações",
        "Background refresh" => "Atualização em segundo plano",
        "Reduce updates when the window is not focused" =>
            "Reduzir atualizações quando a janela estiver sem foco",
        "Reduce updates when minimized to the system tray" =>
            "Reduzir atualizações ao minimizar para a bandeja",
        "Choose when Moth Controller uses fewer resources in the background." =>
            "Escolha quando o Moth Controller deve consumir menos recursos em segundo plano.",
        "When enabled, refresh this window less often while you use other apps. The game keeps running normally." =>
            "Quando ativado, atualiza esta janela com menos frequência ao usar outros aplicativos. O jogo continua funcionando normalmente.",
        "On: slower refresh while hidden in the tray. Off: normal foreground refresh continues while hidden." =>
            "Ativado: atualizações mais lentas na bandeja. Desativado: mantém o ritmo normal mesmo oculto.",
        "PokéIdle and Twitch keep running normally. These options only change how often the window and Firefox update." =>
            "PokéIdle e Twitch continuam funcionando normalmente. Estas opções só alteram a frequência de atualização da janela e do Firefox.",
        "Changes are saved automatically and restored on the next launch." =>
            "Alterações são salvas automaticamente e restauradas na próxima inicialização.",
        "Controller options saved" => "Opções do controlador salvas",
        "Hunts · Atlas" => "Hunts · Atlas",
        "Hunt Atlas" => "Hunt Atlas",
        "Inventory" => "Inventário",
        "All accounts" => "Todas as contas",
        "Search items or drops" => "Buscar itens ou drops",
        "All items" => "Todos os itens",
        "Balls" => "Pokébolas",
        "Potions" => "Poções",
        "Stones" => "Pedras",
        "Other" => "Outros",
        "balls" => "Pokébolas",
        "potions" => "Poções",
        "stones" => "Pedras",
        "other" => "Outros",
        "Start an account to see its inventory" => "Inicie uma conta para ver seu inventário",
        "Loading public item names..." => "Carregando nomes públicos de itens...",
        "Item catalog unavailable" => "Catálogo de itens indisponível",
        "Current inventory" => "Inventário atual",
        "items" => "itens",
        "units" => "unidades",
        "Latest inventory reported by the game" => "Último estoque informado pelo jogo",
        "No inventory items match the filters" => "Nenhum item corresponde aos filtros",
        "Items acquired in session" => "Itens obtidos na sessão",
        "kills" => "abates",
        "captures" => "capturas",
        "From wild battle drops only. Does not count shop purchases or starting inventory." =>
            "Somente drops de batalhas. Não inclui compras nem estoque inicial.",
        "No drops acquired in this session yet" => "Nenhum drop obtido nesta sessão",
        "Current hunt details" => "Detalhes da hunt atual",
        "spawn slots" => "slots de spawn",
        "Switch pending" => "Mudança pendente",
        "Wild encounter" => "Selvagem",
        "Session" => "Sessão",
        "shinies seen" => "shinies vistos",
        "shinies caught" => "shinies capturados",
        "Trainer XP earned" => "XP de treinador obtido",
        "Pokémon XP earned" => "XP de Pokémon obtido",
        "Combat gold" => "Ouro de combate",
        "Auto-sale gold" => "Ouro de venda automática",
        "Reset session counters" => "Resetar contadores da sessão",
        "Reset all session loot/XP counters?" => "Resetar todos os contadores de drops/XP?",
        "Confirm reset" => "Confirmar reset",
        "Cancel" => "Cancelar",
        "Drops acquired" => "Drops obtidos",
        "Configure Auto Catch" => "Configurar captura automática",
        "Enable Auto Catch" => "Ativar captura automática",
        "Preferred balls (in order; Master Balls excluded)" =>
            "Bolas preferidas (em ordem; Master Balls excluídas)",
        "Primary" => "Principal",
        "Fallback" => "Reserva",
        "None" => "Nenhuma",
        "Ball inventory reserves" => "Estoque reservado por bola",
        "Automatic restock" => "Reposição automática",
        "Configure restock" => "Configurar reposição",
        "Start this account to configure restock" => "Inicie esta conta para configurar a reposição",
        "Restock is configured on each Overview account card" => "Configure a reposição no cartão de cada conta na Visão geral",
        "Gold reserve" => "Reserva de ouro",
        "ID" => "ID",
        "Buy at" => "Comprar ao chegar a",
        "Fill to" => "Repor até",
        "Select game ball" => "Selecionar bola do jogo",
        "Select potion" => "Selecionar poção",
        "Select revive" => "Selecionar revive",
        "Revives" => "Revives",
        "Restock changes queued; saving previous selection..." =>
            "Alterações da reposição na fila; salvando seleção anterior...",
        "ID 0 disables that category; unknown shop prices cannot be spent." =>
            "ID 0 desativa a categoria; compras exigem preço conhecido pelo servidor.",
        "Save settings" => "Salvar configurações",
        "Discard" => "Descartar",
        "Saving encrypted preferences..." => "Salvando preferências criptografadas...",
        "Account" => "Conta",
        "Hunts available to your account" => "Caças disponíveis para sua conta",
        "Start the account to load its available hunts." => "Inicie a conta para carregar as hunts disponíveis.",
        "Available hunts and your active Pokémon are loaded after signing in." => "As caças disponíveis e o Pokémon ativo são carregados após o login.",
        "Waiting for authenticated game hunt catalog..." => "Aguardando catálogo de hunts do servidor...",
        "Active Pokémon" => "Pokémon ativo",
        "Waiting for active Pokémon state" => "Aguardando dados do Pokémon ativo",
        "Bonuses considered" => "Bônus considerados",
        "Current hunt" => "Hunt atual",
        "available hunts" => "hunts disponíveis",
        "observed kills" => "abates observados",
        "Measured during current hunt" => "Medido durante a hunt atual",
        "Trainer XP/h" => "XP treinador/h",
        "Pokémon XP/h" => "XP Pokémon/h",
        "Net gold/h" => "Ouro líquido/h",
        "Kills/h" => "Abates/h",
        "kills in window" => "abates na janela",
        "Available hunts · Hunt Atlas" => "Hunts disponíveis · Hunt Atlas",
        "Trainer XP" => "XP treinador",
        "Pokémon XP" => "XP Pokémon",
        "Gold" => "Ouro",
        "Search hunt, region or species ID" => "Buscar hunt, região ou ID da espécie",
        "All regions" => "Todas as regiões",
        "Clear filters" => "Limpar filtros",
        "matching hunts" => "hunts encontradas",
        "Other-hunt rates are scenarios at CURRENT observed kill speed, not predictions of combat performance." =>
            "Outras hunts usam o ritmo atual como cenário, não como previsão de combate.",
        "Conditional XP/gold per hour if the CURRENT observed kill rate were maintained." =>
            "XP/ouro por hora apenas se o ritmo atual de abates fosse mantido.",
        "species" => "espécies",
        "No species weights reported" => "Sem pesos de spawn informados",
        "CURRENT" => "ATUAL",
        "Pending" => "Pendente",
        "Hunt" => "Caçar",
        "Showing first 200 hunts; refine search or region." => "Mostrando até 200 hunts; refine a busca.",
        "Loading species and move reference..." => "Carregando espécies e golpes do guia...",
        "Guide reference unavailable" => "Guia de referência indisponível",
        "Retry" => "Tentar novamente",
        "species in guide" => "espécies no guia",
        "Server determines accessible hunts" => "Servidor define quais hunts estão liberadas",
        "Guide model: forecasts are uncertain; calibrated to observed combat where available." =>
            "Modelo do guia: estimativas incertas, calibradas com combate real quando possível.",
        "Calibrated estimate" => "Estimativa calibrada",
        "Measured" => "Medido",
        "Actual observed rate during current hunt" => "Taxa real observada na hunt atual",
        "Spawn composition" => "Composição de spawns",
        "Experimental estimate" => "Estimativa experimental",

        "PokéIdle controller" => "Controlador do PokéIdle",
        "STOP ALL" => "PARAR TUDO",
        "LAUNCH BOTH" => "INICIAR AMBOS",
        "LAUNCH ENABLED" => "INICIAR HABILITADAS",
        "MINIMIZE TO TRAY" => "MINIMIZAR NA BANDEJA",
        "Hide this window in the system tray; click the tray icon to restore it" => "Oculta esta janela na bandeja do sistema; clique no ícone para restaurá-la",
        "System tray is unavailable; the window cannot be hidden." => "A bandeja do sistema está indisponível; a janela não pode ser ocultada.",
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
        "Manage up to four accounts, each with its own Firefox login." => "Gerencie até 4 perfis exclusivos do Firefox. Cada conta possui seu próprio armazenamento do navegador e conexão BiDi.",
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
        "Version" => "Versão",
        "Update available" => "Atualização disponível",
        "MOTH" => "MOTH",
        "POKEIDLE" => "POKEIDLE",
        "WORKSPACE" => "ÁREA DE TRABALHO",
        "▦  Dashboard" => "▦  Painel",
        "◫  Profiles" => "◫  Perfis",
        "≡  Logs" => "≡  Logs",
        "OPERATIONS" => "OPERAÇÕES",
        "▶  Launch both" => "▶  Iniciar ambos",
        "■  Stop all" => "■  Parar tudo",
        "CONNECTIONS" => "CONEXÕES",
        "Firefox" => "Firefox",
        "Monitor" => "Monitor",
        "Poll" => "Consulta",
        "HIDDEN" => "OCULTO",
        "EXISTING" => "EXISTENTE",
        "VISIBLE" => "VISÍVEL",
        "MIXED" => "MISTO",
        "Moth Controller · Windows" => "Moth Controller · Windows",
        "Moth Controller · Linux" => "Moth Controller · Linux",
        "Instances" => "Instâncias",
        "Live state for both isolated Firefox profiles" => "Estado atual dos dois perfis isolados do Firefox",
        "Show Firefox" => "Mostrar Firefox",
        "Hide Firefox" => "Ocultar Firefox",
        "Restart visible" => "Reiniciar visível",
        "Restart headless" => "Reiniciar oculto",
        "Restarts Firefox in the selected mode; the game and streams will briefly disconnect." => "Reinicia o Firefox no modo escolhido; o jogo e as streams serão desconectados temporariamente.",
        "NO READINGS" => "SEM LEITURAS",
        "CONNECTION UNCONFIRMED" => "CONEXÃO NÃO CONFIRMADA",
        "RECONNECTING" => "RECONECTANDO",
        "UPDATES DELAYED" => "ATUALIZAÇÕES ATRASADAS",
        "Last confirmed data; reconnecting" => "Últimos dados recebidos; reconectando",
        "Capture timeouts" => "Capturas sem resposta",
        "FIREFOX CONNECTION BLOCKED" => "SESSÃO BIDI BLOQUEADA",
        "Restart Firefox to reconnect" => "Reiniciar Firefox para reconectar",
        "Restarts Firefox for this account. The game will disconnect briefly." => "Reinicia o Firefox desta conta. O jogo será desconectado brevemente.",
        "Firefox isn't responding for this account. Restart Firefox to reconnect." => "O Firefox não está respondendo para esta conta. Reinicie o Firefox para reconectar.",
        "Game updates are temporarily unavailable. Firefox may still be running." => "As atualizações do jogo estão temporariamente indisponíveis. O Firefox pode continuar aberto.",
        "The displayed metrics are from the last successful reading." => "Os dados exibidos são da última leitura bem-sucedida.",
        "Installer already running" => "O instalador já está em execução",
        "Installer details" => "Detalhes da instalação",
        "Check now" => "Verificar agora",
        "Checking for updates…" => "Verificando atualizações…",
        "Update check requested" => "Verificação de atualizações solicitada",
        "Repair selected" => "Reparar selecionados",
        "Install all (first setup)" => "Instalar todos (primeira configuração)",
        "Choose the scripts to reinstall." => "Selecione os scripts que deseja reinstalar.",
        "Switch to visible Firefox before opening installers." => "Mude para o Firefox visível antes de abrir os instaladores.",
        "This only opens installers, it does not confirm installation." => "Esta ação apenas abre instaladores, não confirma a instalação.",
        "Running" => "Em execução",
        "Not detected in game" => "Não detectado no jogo",
        "Not installed or inactive" => "Não instalado ou inativo",
        "detected" => "detectados",
        "Running, version not reported" => "Em execução, versão não informada",
        "Not checked yet" => "Ainda não verificado",
        "Up to date" => "Atualizado",
        "Setup needed" => "Configuração necessária",
        "Initial setup" => "Configuração inicial",
        "Complete these steps once for this profile." => "Conclua estas etapas uma vez para este perfil.",
        "Firefox profile created" => "Perfil do Firefox criado",
        "Game signed in" => "Jogo conectado",
        "Scripts detected" => "Scripts detectados",
        "Finish setup" => "Concluir configuração",
        "Show setup guide" => "Mostrar guia de configuração",
        "Hide setup guide" => "Ocultar guia de configuração",
        "Setup completed" => "Configuração concluída",
        "Could not save setup progress" => "Não foi possível salvar o progresso da configuração",
        "Reload the game only when you choose; an automatic reload could interrupt gameplay." => "Recarregue o jogo somente quando desejar; uma recarga automática poderia interromper a partida.",
        "Updates" => "Atualizações",
        "outdated" => "desatualizados",
        "unverified" => "não verificados",
        "Current" => "Atual",
        "Version unknown" => "Versão desconhecida",
        "Checking GitHub versions…" => "Consultando versões no GitHub…",
        "GitHub version check unavailable" => "Verificação de versões no GitHub indisponível",
        "Launch an account to inspect running script versions." => "Inicie uma conta para verificar as versões dos scripts em execução.",
        "Recent events" => "Eventos recentes",
        "No events yet" => "Nenhum evento ainda",
        "running" => "em execução",
        "First scan 30s · 5 min" => "Primeiro scan em 30 s · a cada 5 min",
        "Game data" => "Dados do jogo",
        "Game heartbeat" => "Sinal do jogo",
        "Tab inventory" => "Inventário de abas",
        "Tabs checked" => "Abas verificadas",
        "min ago" => "min atrás",
        "Idle alerts are unavailable until the game reconnects" => "Alertas de inatividade indisponíveis até o jogo reconectar",
        "Profile folder exists" => "Pasta do perfil existente",
        "Registered" => "Registrada",
        "Check failed" => "Falha na verificação",
        "Stop" => "Parar",
        "Launch" => "Iniciar",
        "Start account" => "Iniciar conta",
        "Open in Firefox" => "Abrir no Firefox",
        "Stop Firefox" => "Fechar Firefox",
        "Firefox running" => "Firefox aberto",
        "Play this account in a Firefox window instead of the background connection." =>
            "Jogue com esta conta em uma janela do Firefox em vez da conexão em segundo plano.",
        "Firefox remains open until you stop this account." =>
            "O Firefox permanece aberto até você parar esta conta.",
        "Stop login" => "Parar login",
        "COMPACT OVERVIEW" => "VISÃO COMPACTA",
        "Select an account to inspect its full statistics" => "Selecione uma conta para ver todas as estatísticas",
        "Show details" => "Ver detalhes",
        "Hide details" => "Ocultar detalhes",
        "Details for" => "Detalhes de",
        "EXP/hour" => "EXP/h",
        "Next level" => "Próximo nível",
        "Estimated from recent gains; actual progress may vary" => "Estimativa com base no ganho recente de EXP; o tempo real pode variar",
        "Calculating EXP rate" => "Calculando ritmo de EXP",
        "Based on recent EXP gains" => "Baseado nos ganhos recentes de EXP",
        "No EXP gained recently" => "Sem ganho recente de EXP",
        "Game progress unavailable" => "Progresso do jogo indisponível",
        "Technical details" => "Detalhes técnicos",
        "No recent game data" => "Sem leitura recente do jogo",
        "Not confirmed" => "Não confirmado",
        "XP bonus" => "Bônus XP",
        "CURRENT ACTIVITY" => "ATIVIDADE ATUAL",
        "LAST OBSERVED STATE" => "ÚLTIMO ESTADO REGISTRADO",
        "STATE" => "ESTADO",
        "TRAINER" => "TREINADOR",
        "POKÉMON" => "POKÉMON",
        "CURRENCY" => "SALDOS",
        "CAPTURE RESULTS" => "RESULTADOS DE CAPTURA",
        "BALL STOCK" => "ESTOQUE DE POKÉBOLAS",
        "BALLS USED" => "BOLAS USADAS",
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
        "Twitch chats" => "Chats da Twitch",
        "Twitch channels" => "canais Twitch",
        "shared chat tab" => "aba compartilhada",
        "shared chat tabs (refreshing)" => "abas compartilhadas (atualizando)",
        "shared chat tab not observed yet" => "aba compartilhada ainda não detectada",
        "No live Twitch channels in last scan" => "Nenhum canal Twitch ao vivo no último scan",
        "One chat tab can include many Twitch channels; a second tab may briefly appear during the five-minute refresh." => "Uma aba pode conter vários canais Twitch; uma segunda aba pode aparecer brevemente na atualização a cada cinco minutos.",
        "Shared Twitch chat" => "Chat Twitch compartilhado",
        "channels in this tab" => "canais nesta aba",
        "Last reported tabs" => "Últimas abas detectadas",
        "Tab list unavailable while game loads" => "Lista de abas indisponível durante o carregamento do jogo",
        "No active browser tabs" => "Nenhuma aba ativa do navegador",
        "Tab list not currently verified" => "Lista de abas ainda não verificada",
        "No Twitch tab open" => "Nenhuma aba da Twitch aberta",
        "Optimized Twitch tabs" => "Abas Twitch otimizadas",
        "This ratio counts Twitch browser tabs, not livestream channels." => "Esta proporção conta abas Twitch do navegador, não canais ao vivo.",
        "Not optimized" => "Não otimizado",
        "not optimized" => "não otimizado",
        "No Twitch XP bonus confirmed by the game" => "Nenhum bônus de XP da Twitch confirmado pelo jogo",
        "Twitch channels and game-reported XP bonuses are different signals." => "Canais Twitch e bônus de XP informados pelo jogo são informações diferentes.",
        "Bonus not registered for:" => "Bônus ainda não registrado para:",
        "Chat activity reported; XP bonus not confirmed" => "Jogo registra canais assistidos; bônus de XP não confirmado",
        "Not reported" => "Não informado",
        "Game data unavailable" => "Dados do jogo indisponíveis",
        "Twitch XP bonus unavailable until game data loads" => "Bônus de XP da Twitch indisponível até carregar os dados do jogo",
        "XP bonus data unavailable" => "Dados de bônus de XP indisponíveis",
        "scanning" => "verificando",
        "not-ready" => "jogo não pronto",
        "failed" => "falhou",
        "ok" => "OK",
        "OPEN TABS" => "ABAS ABERTAS",
        "No top-level tabs reported" => "Nenhuma aba principal detectada",
        "custom tab requested" => "nova aba personalizada solicitada",
        "custom tab queued" => "aba personalizada na fila",
        "Tab" => "Aba",
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
        "No Twitch bonus detected" => "Nenhum bônus da Twitch detectado",
        "OPEN" => "ABERTA",
        "XP BONUS SOURCES" => "FONTES DE BÔNUS DE XP",
        "No active XP bonus detected" => "Nenhum bônus de XP ativo detectado",
        "DATA" => "DADOS",
        "Missing:" => "Ausentes:",
        "Runtime" => "Execução",
        "Controller-side diagnostics" => "Diagnóstico do controlador",
        "Open Profiles" => "Abrir perfis",
        "Open Logs" => "Abrir logs",
        "Hidden Firefox" => "Firefox oculto",
        "2 isolated" => "2 isolados",
        "Stream chat" => "Chat das streams",
        "First scan 30s · 10 min" => "Primeiro scan em 30s · a cada 10 min",
        "UI repaint" => "Atualização da interface",
        "1 sec" => "1 s",
        "Map intelligence + observed XP/hour" => "Inteligência do mapa + XP/h observado",
        "Region" => "Região",
        "Min" => "Mín.",
        "Max" => "Máx.",
        "Availability" => "Disponibilidade",
        "All" => "Todas",
        "Unlocked" => "Liberadas",
        "Locked" => "Bloqueadas",
        "Type" => "Tipo",
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
        "Go" => "Ir",
        "Gems" => "Gemas",
        "listings" => "ofertas",
        "seller" => "vendedor",
        "Buy" => "Comprar",
        "Profiles" => "Perfis",
        "Account and browser tools" => "Ferramentas da conta e do navegador",
        "Close" => "Fechar",
        "Open Game" => "Abrir jogo",
        "Google sign-in" => "Login do Google",
        "Installed" => "Instalado",
        "Not installed" => "Não instalado",
        "Not configured" => "Não configurado",
        "Not checked" => "Não verificado",
        "Tab open" => "Aba aberta",
        "Scripts" => "Scripts",
        "listing" => "oferta",
        "live" => "ao vivo",
        "opened" => "abertas",
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
        "Firefox connection" => "Conexão com Firefox",
        "nick" => "apelido",
        "Twitch" => "Twitch",
        "KICK" => "KICK",
        "KICK MULTISTREAM" => "KICK MULTISTREAM",
        "No live KICK channels detected." => "Nenhum canal KICK ao vivo detectado.",
        "live KICK channels:" => "canais KICK ao vivo:",
        "Copy link" => "Copiar link",
        "Open live channels" => "Abrir canais ao vivo",
        "Opened live KICK channels in the regular browser." => "Canais KICK ao vivo abertos no navegador normal.",
        "Copied MultiKick link. Open it in a regular browser, outside the controller-managed Firefox." => "Link MultiKick copiado. Abra-o em um navegador normal, fora do Firefox gerenciado pelo controlador.",
        "Copy link copies a MultiKick URL; Open live channels opens each stream individually in your regular browser." => "Copiar link copia uma URL do MultiKick; Abrir canais ao vivo abre cada transmissão individualmente no navegador normal.",
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
        "Connected" => "Conectado",
        "Disconnected" => "Desconectado",
        "Items" => "Itens",
        "Candidates" => "Candidatos",
        "protocol" => "protocolo",
        "Purchase command accepted" => "Comando de compra aceito",
        "Purchase pending" => "Compra pendente",
        "References" => "Referências",
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
        "Balance" => "Saldo",
        "baseline refresh requested" => "atualização da linha de base solicitada",
        "updated." => "atualizado.",
        "Twitch uses this Firefox profile for its session. All actions target this account." => "A Twitch usa este perfil do Firefox para sua sessão. Todas as ações usam esta conta.",
        "Firefox was not found. Install Firefox or set MOTH_FIREFOX to its executable path." => "O Firefox não foi encontrado. Instale o Firefox ou defina MOTH_FIREFOX para o caminho do executável.",
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
        "PokéIdle and Twitch run without keeping Firefox open" => "PokéIdle e Twitch funcionam sem manter o Firefox aberto",
        "PokéIdle" => "PokéIdle",
        "Connection error" => "Erro de conexão",
        "Connecting to PokéIdle" => "Conectando ao PokéIdle",
        "Checking sign-in" => "Verificando login",
        "Firefox login" => "Login Firefox",
        "Firefox closes after sign-in. PokéIdle and Twitch keep running in the app." =>
            "O Firefox fecha depois do login. PokéIdle e Twitch continuam funcionando no aplicativo.",
        "Game actions" => "Ações do jogo",
        "Pokémon Center" => "Centro Pokémon",
        "Return to hunt" => "Voltar à caça",
        "Enable" => "Ativar",
        "Disable" => "Desativar",
        "Turn Auto Catch on or off for this account" =>
            "Ativar ou desativar captura automática para esta conta",
        "pending" => "pendentes",
        "throws" => "bolas",
        "Connected in app" => "Conectado no aplicativo",
        "Restock settings are saved for this account" =>
            "As opções de reposição são salvas para esta conta",
        "Stock OK" => "Estoque OK",
        "Waiting for inventory" => "Aguardando estoque",
        "Waiting for gold" => "Aguardando ouro",
        "Waiting for shop" => "Aguardando loja",
        "Waiting for purchase" => "Aguardando compra",
        "Confirming inventory" => "Confirmando estoque",
        "Stock synced" => "Estoque atualizado",
        "Backoff" => "Repetindo em breve",
        "Gold reserve protected" => "Reserva de ouro protegida",
        "Needs settings" => "Precisa configurar",
        "Rechecking last order" => "Conferindo última compra",
        "ordered" => "pedidos",
        "confirmed" => "confirmados",
        "Restock is not configured for this account" =>
            "A reposição não está configurada nesta conta",
        "Choose which items to buy and when to restock. Existing settings are loaded when you sign in." =>
            "Escolha quais itens comprar e quando repor. As configurações existentes são carregadas ao entrar.",
        "Signing in with Firefox · PokéIdle and Twitch will connect automatically" =>
            "Entrando pelo Firefox · PokéIdle e Twitch se conectarão automaticamente",
        "Checking your login…" => "Verificando seu login…",
         "enabled isolated Firefox profiles" => "perfis isolados do Firefox habilitados",
        "Loading your Pokémon" => "Carregando seus Pokémon",
        "PokéIdle connection" => "Conexão com PokéIdle",
        "Auto Catch · Restock · Market · Game settings" => "Captura automática · Reposição · Mercado · Opções do jogo",
        "ACCOUNT DETAILS" => "DETALHES DA CONTA",
        "Waiting for sign-in" => "Aguardando login",
        "Account is stopped" => "Conta parada",
        "Choose an account to get started" => "Escolha uma conta para começar",
        "Start this account to manage game settings" => "Inicie esta conta para gerenciar as configurações do jogo",
        "Manage up to four accounts. Firefox is only needed to sign in." => "Gerencie até quatro contas. O Firefox só é necessário para entrar.",
        "Account setup" => "Configuração da conta",
        "Select Start account. If Firefox opens, sign in to PokéIdle there. The account will connect automatically." => "Selecione Iniciar conta. Se o Firefox abrir, faça login no PokéIdle. A conta será conectada automaticamente.",
        "No browser extensions are required. Setup finishes automatically after you sign in." => "Não é necessário instalar extensões no navegador. A configuração termina automaticamente depois do login.",
        "These scripts are optional for playing in Firefox. Auto Catch, Restock, Twitch and Market work without them." => "Estes scripts são opcionais para jogar no Firefox. Captura automática, reposição, Twitch e mercado funcionam sem eles.",
        "Firefox is only needed to sign in or play in the browser. PokéIdle and Twitch can keep running after it closes." => "O Firefox só é necessário para entrar ou jogar no navegador. PokéIdle e Twitch podem continuar funcionando depois que ele fecha.",
        "No Auto Catch attempts recorded yet" => "Nenhuma tentativa de captura automática registrada ainda",
        "These numbers show Auto Catch only. Manual catches are included in the dashboard totals." => "Estes números mostram apenas a captura automática. As capturas manuais entram nos totais do painel.",
        "Auto Catch settings saved" => "Configurações de captura automática salvas",
        "Loading game automation settings" => "Carregando configurações de automação do jogo",
        "Sign-in checklist" => "Verificação de login",
        "Twitch login" => "Login da Twitch",
        "Sign-in required" => "Login necessário",
        "Waiting for Twitch sign-in" => "Aguardando login na Twitch",
        "Not verified" => "Não verificado",
        "Start the account, then sign in to PokéIdle in the Firefox window." => "Inicie a conta e entre no PokéIdle pela janela do Firefox.",
        "PokéIdle is ready. Sign in to Twitch in the same Firefox profile; both connections are required to finish setup." => "PokéIdle está pronto. Entre na Twitch no mesmo perfil do Firefox; os dois logins são necessários.",
        "Both logins are connected. Future starts can reuse your saved login." => "Os dois logins estão conectados. As próximas sessões poderão usar o login salvo.",
        "Twitch sign-in" => "Entrar na Twitch",
        "Automation is managed in the controller. Start this account to see its current settings." =>
            "A automação é gerenciada no aplicativo. Inicie a conta para ver as configurações atuais.",
        _ => translations::pt(en),
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
    native: Option<NativeSession>,
    handoff_closing: bool,
    child: Option<Child>,
    monitor: Option<MonitorHandle>,
    headless: bool,
    orphaned_pid: Option<u32>,
    // User explicitly chose visible Firefox; never auto-handoff this browser.
    browser_only: bool,
}

impl GameSlot {
    fn new(profile: GameProfile) -> Self {
        Self {
            profile,
            native: None,
            handoff_closing: false,
            child: None,
            monitor: None,
            headless: true,
            orphaned_pid: None,
            browser_only: false,
        }
    }

    fn is_running(&mut self) -> bool {
        if self.native.is_some() || self.handoff_closing { return true; }
        if let Some(child) = self.child.as_mut() {
            // On an OS inspection error, conservatively keep ownership rather
            // than offer a duplicate launch into a potentially locked profile.
            match child.try_wait() {
                Ok(None) | Err(_) => return true,
                Ok(Some(_)) => {}
            }
        }
        if let Some(pid) = self.orphaned_pid {
            if firefox::process_alive(pid) {
                return true;
            }
            self.orphaned_pid = None;
        }
        false
    }

    fn health(&self) -> Health {
        if self.handoff_closing {
            let mut health = Health::default();
            health.state = "Connecting".into();
            return health;
        }
        if let Some(native) = self.native.as_ref() {
            let state = native.snapshot();
            let mut health = Health::default();
            health.state = if state.phase == "Online" { "Running".into() }
                else { state.phase.clone() };
            // Project the same authoritative native account domains used by
            // Hunts / Inventory. A Firefox Bridge snapshot is not required.
            let view = native.hunt_snapshot();
            health.activity = if view.in_center { "Center".into() }
                else if view.hunt_slug.is_some() { "Hunting".into() }
                else { "Online".into() };
            health.active_pokemon = view.active_pokemon.as_ref()
                .map(|pokemon| pokemon.name.clone()).unwrap_or_default();
            health.pokemon_level = view.active_pokemon.as_ref()
                .map(|pokemon| pokemon.level.to_string()).unwrap_or_default();
            health.pokemon_xp = view.active_pokemon.as_ref()
                .and_then(|pokemon| pokemon.xp).map(|xp| xp.to_string())
                .unwrap_or_default();
            health.fallen_count = view.pokemon_roster.iter()
                .filter(|pokemon| pokemon.hp == Some(0)).count()
                .min(u32::MAX as usize) as u32;
            health.ball_stock = view.balls.iter().map(|(id, qty)| {
                format!("{}: {}", view.ball_names.get(id)
                    .map(String::as_str).unwrap_or("Ball"), qty)
            }).collect();
            health.autocatch_rate = format!("{} throws · {} captures",
                state.native_throws, state.captures);
            health.autocatch_restock = state.native_restock.phase.clone();
            health.xp_bonuses = vec![
                format!("Twitch +{:.0}%", view.bonuses.twitch_pct),
                format!("Guild +{:.0}%", view.bonuses.guild_pct),
                format!("Trainer event +{:.0}%", view.bonuses.trainer_event_pct),
                format!("Pokémon event +{:.0}%", view.bonuses.pokemon_event_pct),
            ];
            // Existing widgets consume hunt_atlas, but never claim a
            // speculative rate is a measured one.
            health.hunt_atlas = json!({
                "huntSlug": view.hunt_slug.clone(),
                "xpH": if view.measurements.valid_trainer {
                    Some(view.measurements.trainer_xp_h)
                } else { None },
                "coinsH": if view.measurements.valid_gold {
                    Some(view.measurements.gold_net_h)
                } else { None },
                "xpMeasured": view.measurements.valid_trainer,
            });
            health.bridge_connected = state.phase == "Online";
            health.bridge_snapshot_ok = state.phase == "Online";
            health.autocatch_on = state.native_auto_catch;
            health.autocatch_balls_used = state.native_throws.min(u32::MAX as u64) as u32;
            health.autocatch_captures = state.captures.min(u32::MAX as u64) as u32;
            health.stream_watching = state.twitch_watching.clone();
            health.stream_bonus_pct = state.twitch_bonus_pct;
            health.stream_scan_twitch_channels = Some(state.twitch_live.len() as u32);
            health.stream_bonus = if state.twitch_bonus_pct > 0.0 {
                format!("+{}% XP", state.twitch_bonus_pct)
            } else { String::new() };
            health.game_ready = state.phase == "Online";
            health.logged_in = state.phase == "Online";
            health.hunt_slug = view.hunt_slug.clone().or(state.hunt_slug.clone()).unwrap_or_default();
            health.hunt = view.hunts.iter()
                .find(|hunt| hunt.slug == health.hunt_slug)
                .map(|hunt| hunt.name.clone())
                .unwrap_or_else(|| health.hunt_slug.clone());
            health.player_level = view.trainer_level.or(state.trainer_level).unwrap_or_default();
            health.player_total_xp = view.trainer_xp.or(state.trainer_xp);
            health.player_xp = health.player_total_xp.map(|xp| xp.to_string()).unwrap_or_default();
            health.gold = view.gold.or(state.gold).unwrap_or_default();
            health.orbs = view.orbs.or(state.orbs).unwrap_or_default();
            health.last_successful_probe_at = state.last_received;
            health.last_heartbeat_at = state.last_received;
            health.last_error = state.error;
            return health;
        }
        // The launcher process can exit while the exact-profile Firefox process
        // and its BiDi session remain alive. Preserve live monitoring in that case.
        if let Some(monitor) = self.monitor.as_ref() {
            return monitor.health();
        }
        if self.orphaned_pid.is_some() {
            let mut health = Health::default();
            health.state = "Existing Firefox".to_string();
            return health;
        }
        Health::default()
    }
}

#[derive(Clone, Debug, Default)]
struct IdleProgressTracker {
    last_xp: Option<String>,
    last_level: Option<u32>,
    last_balls_used: Option<u32>,
    last_ball_stock: Vec<String>,
    unchanged_since: Option<Instant>,
    last_sample_at: Option<Instant>,
    warning: bool,
}

impl IdleProgressTracker {
    /// Progress is evaluated only when a new successful BiDi sample arrives.
    /// Re-rendering the same sample must never produce a false inactivity alert.
    fn observe(&mut self, health: &Health) -> bool {
        let Some(sample_at) = health.last_successful_probe_at else {
            return false;
        };
        if self.last_sample_at == Some(sample_at) {
            return self.warning;
        }

        let xp = health.player_xp.clone();
        let level = health.player_level;
        let balls_used = health.autocatch_balls_used;
        let changed = self.last_xp.as_ref() != Some(&xp)
            || self.last_level != Some(level)
            || self.last_balls_used != Some(balls_used)
            || ball_stock_decreased(&self.last_ball_stock, &health.ball_stock);

        if changed || self.unchanged_since.is_none() {
            self.last_xp = Some(xp);
            self.last_level = Some(level);
            self.last_balls_used = Some(balls_used);
            self.unchanged_since = Some(sample_at);
            self.warning = false;
        } else {
            self.warning = self.unchanged_since.is_some_and(|since| {
                sample_at.duration_since(since) >= Duration::from_secs(5 * 60)
            });
        }

        self.last_ball_stock = health.ball_stock.clone();
        self.last_sample_at = Some(sample_at);
        self.warning
    }
}

/// Estimate earned trainer XP per hour from a short rolling history.
/// Samples are cumulative XP and monotonic timestamps, never UI redraws.
/// A four-to-six-sample window smooths combat bursts without endless averaging.
#[derive(Debug, Default, Clone)]
struct XpRateTracker {
    samples: VecDeque<(Instant, u64)>,
    rate_per_hour: Option<f64>,
}

impl XpRateTracker {
    const MAX_SAMPLES: usize = 6;
    const MAX_WINDOW: Duration = Duration::from_secs(20 * 60);

    fn observe(&mut self, health: &Health) -> Option<f64> {
        if !health.telemetry_is_fresh() || !health.logged_in || !health.bridge_snapshot_ok {
            *self = Self::default();
            return None;
        }
        let (Some(at), Some(total)) =
            (health.last_successful_probe_at, health.player_total_xp)
        else {
            *self = Self::default();
            return None;
        };

        if let Some(&(previous_at, previous_total)) = self.samples.back() {
            // Ignore cached UI redraws and older readings.
            if at <= previous_at {
                return self.rate_per_hour;
            }
            // A reset, character switch, or unexpected counter decrease
            // must never produce an absurd negative rate.
            if total < previous_total
                || at.duration_since(previous_at) > Self::MAX_WINDOW
            {
                // A long gap would blend inactive time into the rate. Begin
                // again instead of displaying an unreliable average.
                self.samples.clear();
                self.rate_per_hour = None;
            }
        }
        self.samples.push_back((at, total));
        while self.samples.len() > Self::MAX_SAMPLES
            || self.samples.front().is_some_and(|(old_at, _)|
                at.duration_since(*old_at) > Self::MAX_WINDOW
                && self.samples.len() > 2)
        {
            self.samples.pop_front();
        }

        self.rate_per_hour = self.samples.front().zip(self.samples.back())
            .and_then(|(&(start_at, start_total), &(end_at, end_total))| {
                let seconds = end_at.duration_since(start_at).as_secs_f64();
                if seconds < 60.0 { return None; }
                let earned = end_total.checked_sub(start_total)?;
                Some((earned as f64 * 3600.0) / seconds)
            });
        self.rate_per_hour
    }
}

#[cfg(test)]
mod xp_rate_tests {
    use super::{Health, XpRateTracker};
    use std::time::{Duration, Instant};

    fn sample(at: Instant, total: Option<u64>) -> Health {
        let mut health = Health::default();
        health.state = "Running".into();
        health.logged_in = true;
        health.bridge_snapshot_ok = true;
        health.player_total_xp = total;
        health.last_successful_probe_at = Some(at);
        health
    }

    #[test]
    fn rate_normalizes_elapsed_time_and_ignores_ui_redraws() {
        let now = Instant::now();
        let mut tracker = XpRateTracker::default();
        let baseline = sample(now - Duration::from_secs(180), Some(1500));
        assert_eq!(tracker.observe(&baseline), None);
        assert_eq!(tracker.observe(&baseline), None);
        let next = sample(now, Some(1620));
        assert_eq!(tracker.observe(&next), Some(2400.0));
        assert_eq!(tracker.observe(&next), Some(2400.0));
        assert_eq!(tracker.observe(&baseline), Some(2400.0));
    }

    #[test]
    fn gains_are_smoothed_across_readings_and_level_ups() {
        let now = Instant::now();
        let mut tracker = XpRateTracker::default();
        tracker.observe(&sample(now - Duration::from_secs(240), Some(9800)));
        assert_eq!(tracker.observe(&sample(now - Duration::from_secs(120), Some(10100))), Some(9000.0));
        // No progress in the latest interval lowers the rolling rate.
        assert_eq!(tracker.observe(&sample(now, Some(10100))), Some(4500.0));
        // A genuine counter reset creates a new baseline.
        assert_eq!(tracker.observe(&sample(now + Duration::from_secs(1), Some(50))), None);
    }

    #[test]
    fn stale_or_missing_game_data_clears_rate() {
        let now = Instant::now();
        let mut tracker = XpRateTracker::default();
        tracker.observe(&sample(now - Duration::from_secs(180), Some(20)));
        tracker.observe(&sample(now, Some(40)));
        assert_eq!(tracker.observe(&sample(now, None)), None);
        assert_eq!(tracker.observe(&sample(now, Some(50))), None);
        let mut disconnected = sample(now, Some(70));
        disconnected.state = "Reconnecting".into();
        assert_eq!(tracker.observe(&disconnected), None);
        assert_eq!(tracker.observe(&sample(now, Some(70))), None);
    }

    #[test]
    fn long_gap_does_not_produce_misleading_exp_rate() {
        let now = Instant::now();
        let mut tracker = XpRateTracker::default();
        tracker.samples.push_back((now - Duration::from_secs(25 * 60), 1000));
        tracker.rate_per_hour = Some(5000.0);
        assert_eq!(tracker.observe(&sample(now, Some(4000))), None);
        assert_eq!(tracker.samples.len(), 1);
    }


}

#[cfg(test)]
mod idle_progress_tests {
    use super::{Health, IdleProgressTracker};
    use std::time::{Duration, Instant};

    #[test]
    fn unchanged_rendering_does_not_create_an_idle_warning() {
        let base = Instant::now() - Duration::from_secs(11 * 60);
        let mut sample = Health::default();
        sample.player_xp = "100".to_string();
        sample.player_level = 5;
        sample.last_successful_probe_at = Some(base);
        let mut tracker = IdleProgressTracker::default();

        assert!(!tracker.observe(&sample));
        for _ in 0..100 {
            assert!(!tracker.observe(&sample));
        }
        sample.last_successful_probe_at = Some(base + Duration::from_secs(5 * 60));
        assert!(tracker.observe(&sample));
        sample.player_xp = "101".to_string();
        sample.last_successful_probe_at = Some(base + Duration::from_secs(10 * 60));
        assert!(!tracker.observe(&sample));
    }
}

fn ball_stock_decreased(previous: &[String], current: &[String]) -> bool {
    previous.iter().any(|before| {
        let Some((before_name, before_count)) = before.rsplit_once(' ') else {
            return false;
        };
        let Ok(before_count) = before_count.parse::<u64>() else {
            return false;
        };

        let after_count = current
            .iter()
            .find_map(|item| {
                let (name, count) = item.rsplit_once(' ')?;
                if name == before_name {
                    count.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);

        after_count < before_count
    })
}

#[derive(Clone, Debug, Default)]
struct AccountSetup {
    profile_ready: bool,
    setup_complete: bool,
    check_error: Option<String>,
}

struct UiEvent {
    at: Instant,
    message: String,
    is_error: bool,
}

#[cfg(windows)]
struct TrayRuntime {
    _icon: TrayIcon,
    stop_worker: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

#[cfg(windows)]
impl TrayRuntime {
    fn new(ctx: egui::Context, hidden: Arc<AtomicBool>) -> Result<Self, String> {
        let menu = Menu::new();
        let show_item = MenuItem::new("Mostrar / Show Moth", true, None);
        let separator = PredefinedMenuItem::separator();
        let quit_item = MenuItem::new("Sair / Exit", true, None);

        menu.append(&show_item)
            .map_err(|error| format!("could not add tray Show menu item: {error}"))?;
        menu.append(&separator)
            .map_err(|error| format!("could not add tray separator: {error}"))?;
        menu.append(&quit_item)
            .map_err(|error| format!("could not add tray Exit menu item: {error}"))?;

        let icon = TrayIconBuilder::new()
            .with_id("moth-pokeidle-controller")
            .with_tooltip("Moth · PokéIdle")
            .with_icon(make_tray_icon()?)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .map_err(|error| format!("could not create system tray icon: {error}"))?;

        let tray_id = icon.id().clone();
        let show_id = show_item.id().clone();
        let quit_id = quit_item.id().clone();
        let stop_worker = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop_worker);

        let worker = std::thread::Builder::new()
            .name("moth-tray-events".to_string())
            .spawn(move || {
                while !worker_stop.load(Ordering::Acquire) {
                    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                        match event {
                            TrayIconEvent::Click {
                                id,
                                button: MouseButton::Left,
                                button_state: MouseButtonState::Up,
                                ..
                            } if id == tray_id => {
                                restore_from_tray(&ctx, &hidden);
                            }
                            TrayIconEvent::DoubleClick {
                                id,
                                button: MouseButton::Left,
                                ..
                            } if id == tray_id => {
                                restore_from_tray(&ctx, &hidden);
                            }
                            _ => {}
                        }
                    }

                    while let Ok(event) = MenuEvent::receiver().try_recv() {
                        if event.id == show_id {
                            restore_from_tray(&ctx, &hidden);
                        } else if event.id == quit_id {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            ctx.request_repaint();
                        }
                    }

                    std::thread::sleep(Duration::from_millis(100));
                }
            })
            .map_err(|error| format!("could not start tray event worker: {error}"))?;

        Ok(Self {
            _icon: icon,
            stop_worker,
            worker: Some(worker),
        })
    }

    fn shutdown(&mut self) {
        self.stop_worker.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(windows)]
impl Drop for TrayRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(windows)]
fn restore_from_tray(ctx: &egui::Context, hidden: &AtomicBool) {
    hidden.store(false, Ordering::Relaxed);
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.request_repaint();
}

#[cfg(windows)]
fn make_tray_icon() -> Result<tray_icon::Icon, String> {
    const SIZE: u32 = 32;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];

    for y in 0..SIZE as i32 {
        for x in 0..SIZE as i32 {
            let dx = x - 16;
            let dy = y - 16;
            let in_badge = dx * dx + dy * dy <= 225;

            let upper_left = ((x as f32 - 10.0) / 6.0).powi(2)
                + ((y as f32 - 12.0) / 5.0).powi(2)
                <= 1.0;
            let upper_right = ((x as f32 - 22.0) / 6.0).powi(2)
                + ((y as f32 - 12.0) / 5.0).powi(2)
                <= 1.0;
            let lower_left = ((x as f32 - 11.0) / 5.0).powi(2)
                + ((y as f32 - 20.0) / 4.0).powi(2)
                <= 1.0;
            let lower_right = ((x as f32 - 21.0) / 5.0).powi(2)
                + ((y as f32 - 20.0) / 4.0).powi(2)
                <= 1.0;
            let body = (15..=17).contains(&x) && (8..=24).contains(&y);

            let color = if body {
                [245, 242, 255, 255]
            } else if upper_left || upper_right {
                [123, 97, 255, 255]
            } else if lower_left || lower_right {
                [76, 160, 255, 255]
            } else if in_badge {
                [18, 21, 33, 255]
            } else {
                [0, 0, 0, 0]
            };

            let offset = ((y as u32 * SIZE + x as u32) * 4) as usize;
            rgba[offset..offset + 4].copy_from_slice(&color);
        }
    }

    tray_icon::Icon::from_rgba(rgba, SIZE, SIZE)
        .map_err(|error| format!("could not create tray icon image: {error}"))
}

struct NativeHandoffEvent {
    index: usize,
    result: Result<serde_json::Value, String>,
    cache_warning: Option<String>,
}

struct CachedNativeEvent {
    index: usize,
    existing_firefox: bool,
    result: Result<Option<NativeSession>, String>,
}

/// Authenticated Rust game WSS + Twitch IRC must both be confirmed before
/// the complete Firefox process may close.
struct NativeVerifiedEvent {
    index: usize,
    result: Result<NativeSession, String>,
    cache_warning: Option<String>,
}

/// The child process and monitor are owned exclusively by the shutdown worker.
/// Returning them to the UI on error preserves ownership for retry/recovery.
struct NativeBrowserClosedEvent {
    index: usize,
    native: NativeSession,
    cache_warning: Option<String>,
    child: Option<Child>,
    orphaned_pid: Option<u32>,
    monitor: Option<MonitorHandle>,
    result: Result<(), String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PanelTab { Overview, Hunts, Accounts, Automation, Options }

#[derive(Clone, Copy, PartialEq, Eq)]
enum AccountsSection { Profiles, Kick }

/// Account-local automation workspaces. Keep the selected section while
/// switching accounts; the scroll position is isolated per account/section.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AutomationSection { Capture, Game, Market }
#[derive(Clone,Default)]
struct AutomationDraft {
    initialized: bool,
    hp_limit: f64,
    lock_grade: f64,
}
impl AutomationDraft {
    fn from_server(value:&serde_json::Value) -> Self {
        Self { initialized:true,
            hp_limit:value.get("hpLimiar").and_then(serde_json::Value::as_f64).unwrap_or(0.0),
            lock_grade:value.get("autoLockNotaMin").and_then(serde_json::Value::as_f64).unwrap_or(0.0) }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum HuntRanking { TrainerXp, PokemonXp, Gold }

struct CaptureSavedEvent {
    index:usize,
    config:CatchSettings,
    // A switch-only save must not discard unsaved editor changes.
    toggle_only:bool,
    result:Result<(),String>
}
pub struct ControllerApp {
    native_handoff_tx: Sender<NativeHandoffEvent>,
    native_handoff_rx: Receiver<NativeHandoffEvent>,
    native_handoff_busy: [bool; 4],
    native_handoff_last_attempt: [Option<Instant>; 4],
    native_verified_tx: Sender<NativeVerifiedEvent>,
    native_verified_rx: Receiver<NativeVerifiedEvent>,
    native_verifying: [bool; 4],
    native_error: [Option<String>; 4],
    native_closed_tx: Sender<NativeBrowserClosedEvent>,
    native_closed_rx: Receiver<NativeBrowserClosedEvent>,
    native_close_cancelled: [bool; 4],
    cached_native_tx: Sender<CachedNativeEvent>,
    cached_native_rx: Receiver<CachedNativeEvent>,
    cached_native_busy: [bool; 4],
    require_browser_login: [bool; 4],
    games: [GameSlot; 4],
    panel_tab: PanelTab,
    accounts_section: AccountsSection,
    kick_selected_account: usize,
    dashboard_show_history: bool,
    dashboard_session_view: bool,
    ui_options: UiOptions,
    hunt_account: usize,
    automation_account: usize,
    automation_section: AutomationSection,
    automation_drafts: [AutomationDraft;4],
    market_drafts: [Option<crate::native_market::MarketRules>;4],
    market_snatch_drafts: [crate::native_market::SnatchTarget;4],
    market_imports: [String;4],
    market_notices: [String;4],
    market_copy_targets: [bool;4],
    market_copy_activation: bool,
    history_rows_limit: usize,
    offline_histories: [Option<crate::native_history::HistorySnapshot>;4],
    hunt_query: String,
    hunt_region: String,
    hunt_type: String,
    hunt_ranking: HuntRanking,
    hunt_subtab: HuntSubtab,
    hunt_page: usize,
    hunt_action_error: [Option<String>;4],
    hunt_estimate_cache: [hunt_panel::EstimateCache;4],
    atlas_results: [hunt_panel::ExplorerCache;4],
    atlas_filters: [hunt_panel::AtlasFilters;4],
    atlas_scenarios: [hunt_panel::AtlasScenario;4],
    atlas_page: [usize;4],
    hunt_reference: Option<Arc<AtlasReference>>,
    hunt_reference_error: Option<String>,
    hunt_reference_busy: bool,
    hunt_reference_tx: Sender<Result<AtlasReference,String>>,
    hunt_reference_rx: Receiver<Result<AtlasReference,String>>,
    capture_drafts:[Option<CatchSettings>;4],
    // Restock has optimistic per-account editing state. Never rebuild the
    // picker from the last confirmed worker snapshot while a save is in flight.
    restock_drafts:[Option<CatchSettings>;4],
    restock_dirty:[bool;4],
    restock_open:[bool;4],
    capture_save_busy:[bool;4],
    capture_save_error:[Option<String>;4],
    capture_save_tx:Sender<CaptureSavedEvent>,
    capture_save_rx:Receiver<CaptureSavedEvent>,
    session_reset_confirm:[bool;4],
    inventory_catalog:Option<Arc<ItemCatalog>>,
    visuals:VisualCache,
    inventory_catalog_error:Option<String>,
    inventory_catalog_busy:bool,
    inventory_catalog_tx:Sender<Result<ItemCatalog,String>>,
    inventory_catalog_rx:Receiver<Result<ItemCatalog,String>>,
    accounts: [AccountConfig; 4],
    /// Kick website sessions run independently per account. Bearer tokens
    /// are entered in masked fields and optionally stored in the OS vault.
    kick_monitors: [Option<KickMonitor>; 4],
    kick_api_token_input: [String; 4],
    kick_channels_input: [String; 4],
    // Live official channels are discovered from the native PokéIdle socket,
    // not copied into the user's saved manual preferences.
    kick_discovered_live: [Vec<String>; 4],
    kick_discovery_available: [bool; 4],
    kick_channel_new: [String; 4],
    kick_saved_login: [bool; 4],
    kick_remember_login: [bool; 4],
    account_names_dirty: [bool; 4],
    status: String,
    status_error: bool,
    events: VecDeque<UiEvent>,
    show_setup_guide: [bool; 4],
    account_setup: [AccountSetup; 4],
    account_setup_checked_at: Option<Instant>,
    tab_url_input: [String; 4],
    tab_url_open: [bool; 4],
    idle_progress: [IdleProgressTracker; 4],
    xp_rates: [XpRateTracker; 4],
    selected_details: Option<usize>,
    update_status: SharedUpdateStatus,
    #[cfg(windows)]
    tray: Option<TrayRuntime>,
    #[cfg(windows)]
    tray_hidden: Arc<AtomicBool>,
}

impl ControllerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);

        logging::init();
        logging::info(&format!(
            "Moth native controller v{} · verified TLS provider initialized: {}",
            env!("CARGO_PKG_VERSION"),
            rustls::crypto::CryptoProvider::get_default().is_some()
        ));
        logging::info("controller UI initialized with egui/eframe");
        let update_status = update::start_update_check();
        let (native_handoff_tx, native_handoff_rx) = mpsc::channel();
        let (cached_native_tx, cached_native_rx) = mpsc::channel();
        let (native_closed_tx, native_closed_rx) = mpsc::channel();
        let (native_verified_tx, native_verified_rx) = mpsc::channel();
        let (hunt_reference_tx, hunt_reference_rx) = mpsc::channel();
        let (capture_save_tx,capture_save_rx)=mpsc::channel();
        let (inventory_catalog_tx,inventory_catalog_rx)=mpsc::channel();

        #[cfg(windows)]
        let tray_hidden = Arc::new(AtomicBool::new(false));
        #[cfg(windows)]
        let tray = match TrayRuntime::new(cc.egui_ctx.clone(), Arc::clone(&tray_hidden)) {
            Ok(tray) => Some(tray),
            Err(error) => {
                logging::warn(&format!("system tray initialization failed: {error}"));
                None
            }
        };

        Self {
            native_handoff_tx,
            native_handoff_rx,
            native_handoff_busy: [false; 4],
            native_handoff_last_attempt: [None; 4],
            native_verified_tx,
            native_verified_rx,
            native_verifying: [false; 4],
            native_error: std::array::from_fn(|_| None),
            native_closed_tx,
            native_closed_rx,
            native_close_cancelled: [false; 4],
            cached_native_tx,
            cached_native_rx,
            cached_native_busy: [false; 4],
            require_browser_login: [false; 4],
            games: std::array::from_fn(|i| {
                GameSlot::new(GameProfile::from_index(i).expect("valid profile slot"))
            }),
            panel_tab: PanelTab::Overview,
            accounts_section: AccountsSection::Profiles,
            kick_selected_account: 0,
            dashboard_show_history: false,
            dashboard_session_view: false,
            ui_options: Config::load_ui_options().unwrap_or_else(|error| {
                logging::warn(&format!("controller options load failed: {error}"));
                UiOptions::default()
            }),
            hunt_account: 0,
            automation_account: 0,
            automation_section: AutomationSection::Capture,
            automation_drafts: std::array::from_fn(|_|AutomationDraft::default()),
            market_drafts: std::array::from_fn(|_|None),
            market_snatch_drafts: std::array::from_fn(|_|crate::native_market::SnatchTarget{
                kind:"item".into(),currency:"gold".into(),..Default::default()
            }),
            market_imports: std::array::from_fn(|_|String::new()),
            market_notices: std::array::from_fn(|_|String::new()),
            market_copy_targets: [false;4],
            market_copy_activation: false,
            history_rows_limit: 100,
            offline_histories: std::array::from_fn(|_|None),
            hunt_query: String::new(),
            hunt_region: String::new(),
            hunt_type: String::new(),
            hunt_ranking: HuntRanking::TrainerXp,
            hunt_subtab: HuntSubtab::Explorer,
            hunt_page: 0,
            hunt_action_error: std::array::from_fn(|_|None),
            hunt_estimate_cache: std::array::from_fn(|_|hunt_panel::EstimateCache::default()),
            atlas_results: std::array::from_fn(|_|hunt_panel::ExplorerCache::default()),
            atlas_filters: std::array::from_fn(|_|hunt_panel::AtlasFilters::default()),
            atlas_scenarios: std::array::from_fn(|_|hunt_panel::AtlasScenario::default()),
            atlas_page: [0;4],
            hunt_reference: None,
            hunt_reference_error: None,
            hunt_reference_busy: false,
            hunt_reference_tx,
            hunt_reference_rx,
            capture_drafts:std::array::from_fn(|_|None),
            restock_drafts:std::array::from_fn(|_|None),
            restock_dirty:[false;4],
            restock_open:[false;4],
            capture_save_busy:[false;4],
            capture_save_error:std::array::from_fn(|_|None),
            capture_save_tx,
            capture_save_rx,
            session_reset_confirm:[false;4],
            inventory_catalog:None,
            visuals:VisualCache::default(),
            inventory_catalog_error:None,
            inventory_catalog_busy:false,
            inventory_catalog_tx,
            inventory_catalog_rx,
            kick_monitors: std::array::from_fn(|_| None),
            kick_api_token_input: std::array::from_fn(|_| String::new()),
            kick_channels_input: Config::load_kick_channels().unwrap_or_else(|error| {
                logging::warn(&error);
                std::array::from_fn(|_| String::new())
            }),
            kick_discovered_live: std::array::from_fn(|_| Vec::new()),
            kick_discovery_available: [false; 4],
            kick_channel_new: std::array::from_fn(|_| String::new()),
            kick_saved_login: std::array::from_fn(|index| {
                GameProfile::from_index(index).is_some_and(|profile|
                    crate::native_auth::load_kick_token(profile)
                        .ok().flatten().is_some())
            }),
            kick_remember_login: [true; 4],
            account_names_dirty: [false; 4],
            status: "Ready · launch only the profiles you need".to_string(),
            status_error: false,
            events: VecDeque::new(),
            show_setup_guide: [false; 4],
            account_setup: std::array::from_fn(|_| AccountSetup::default()),
            account_setup_checked_at: None,
            tab_url_input: std::array::from_fn(|_| String::new()),
            tab_url_open: [false; 4],
            idle_progress: std::array::from_fn(|_| IdleProgressTracker::default()),
            xp_rates: std::array::from_fn(|_| XpRateTracker::default()),
            selected_details: None,
            update_status,
            #[cfg(windows)]
            tray,
            #[cfg(windows)]
            tray_hidden,
            accounts: Config::load_accounts().unwrap_or_else(|error| {
                logging::warn(&format!("account configuration load failed: {error}"));
                std::array::from_fn(|i| AccountConfig::default_for(GameProfile::from_index(i).expect("valid account slot")))
            }),
        }
    }

    fn game_index(profile: GameProfile) -> usize {
        profile.index()
    }

    fn hidden_in_tray(&self) -> bool {
        #[cfg(windows)]
        { self.tray_hidden.load(Ordering::Relaxed) }
        #[cfg(not(windows))]
        { false }
    }

    fn controller_in_background(&self, ctx: &egui::Context) -> bool {
        // Keep native background transport untouched; only UI repaint and
        // Firefox/BiDi monitoring pace respond to these preferences.
        let focused = ctx.input(|input| input.viewport().focused);
        reduce_updates_for_window(self.hidden_in_tray(), focused, self.ui_options)
    }

    fn save_ui_options(&mut self, updated: UiOptions) {
        match Config::save_ui_options(&updated) {
            Ok(()) => {
                self.ui_options = updated;
                self.set_status(tr("Controller options saved").to_owned(), false);
            }
            Err(error) => self.set_status(error, true),
        }
    }

    fn sync_monitor_paces(&self, background: bool) {
        // Exactly one account may be expanded when focused. Background mode
        // overrides that selection without clearing the open details panel.
        for (index, slot) in self.games.iter().enumerate() {
            let pace = monitor_pace_for_slot(
                self.selected_details, index, self.accounts[index].enabled, background
            );
            if let Some(monitor) = slot.monitor.as_ref() {
                monitor.set_pace(pace);
            }
        }
    }

    /// Switching accounts changes the workspace selection, not a live session.
    fn select_workspace_account(&mut self, index: usize) {
        if index >= self.games.len() || index == self.hunt_account { return; }
        self.save_dirty_account_names();
        self.history_rows_limit = 100;
        self.dashboard_show_history = false;
        self.hunt_account = index;
        if self.panel_tab == PanelTab::Hunts && self.hunt_subtab == HuntSubtab::Compare {
            self.hunt_subtab = HuntSubtab::Explorer;
        }
        self.automation_account = index;
        self.hunt_page = 0;
    }

    /// Account edits are buffered until focus is lost, the selected account
    /// changes, or the user navigates away from Accounts.
    fn save_dirty_account_names(&mut self) {
        for profile in GameProfile::ALL {
            let index = profile.index();
            if self.account_names_dirty[index] {
                let name = self.accounts[index].name.trim();
                self.accounts[index].name = if name.is_empty() {
                    profile.label().to_owned()
                } else {
                    name.to_owned()
                };
                if self.save_account_config() {
                    self.account_names_dirty[index] = false;
                }
            }
        }
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
            let config = match Config::for_profile(profile) {
                Ok(config) => config,
                Err(error) => {
                    self.account_setup[index] = AccountSetup {
                        check_error: Some(error),
                        ..AccountSetup::default()
                    };
                    continue;
                }
            };

            // The setup marker is earned only after both PokéIdle and Twitch
            // report authenticated connections.
            let mut setup_complete = accounts::setup_complete(profile).unwrap_or(false);
            let verified = self.games[index].native.as_ref().is_some_and(|session| {
                session.snapshot().phase == "Online"
                    && session.twitch_snapshot().phase == "Online"
            });
            if verified && !setup_complete {
                match accounts::complete_setup(profile) {
                    Ok(()) => setup_complete = true,
                    Err(error) => logging::warn(&error),
                }
            }
            self.account_setup[index] = AccountSetup {
                profile_ready: config.profile_dir.exists(),
                setup_complete,
                check_error: None,
            };
        }

        self.account_setup_checked_at = Some(Instant::now());
    }

    fn refresh_processes(&mut self) {
        while let Ok(event)=self.capture_save_rx.try_recv() {
            self.capture_save_busy[event.index]=false;
            match event.result {
                Ok(()) => {
                    if let Some(native)=self.games[event.index].native.as_ref() {
                        if let Err(error)=native.set_capture_settings(event.config.clone()) {
                            self.capture_save_error[event.index]=Some(error);
                            continue;
                        }
                    }
                    if event.toggle_only {
                        if let Some(draft)=self.capture_drafts[event.index].as_mut() {
                            draft.enabled=event.config.enabled;
                        }
                    } else {
                        self.capture_drafts[event.index]=None;
                    }
                    if self.restock_drafts[event.index].as_ref()
                        .is_some_and(|draft| restock_fields_equal(draft, &event.config))
                    {
                        // The save has caught up with what the user sees.
                        // A different staged selection stays queued instead.
                        self.restock_drafts[event.index]=Some(event.config);
                        self.restock_dirty[event.index]=false;
                    }
                    self.capture_save_error[event.index]=None;
                    self.set_status(tr("Auto Catch settings saved"),false);
                }
                Err(error) => {
                    self.capture_save_error[event.index]=Some(error.clone());
                    self.set_status(format!("Could not save capture settings: {error}"),true);
                }
            }
        }
        while let Ok(result)=self.inventory_catalog_rx.try_recv() {
            self.inventory_catalog_busy=false;
            match result {
                Ok(catalog)=>{
                    logging::info(&format!("Native inventory catalog ready: {} items",catalog.items.len()));
                    self.inventory_catalog=Some(Arc::new(catalog));
                    self.inventory_catalog_error=None;
                }
                Err(error)=>{
                    logging::warn(&format!("Native inventory catalog unavailable: {error}"));
                    self.inventory_catalog_error=Some(error);
                }
            }
        }
        while let Ok(loaded) = self.hunt_reference_rx.try_recv() {
            self.hunt_reference_busy = false;
            match loaded {
                Ok(reference) => {
                    logging::info(&format!(
                        "Native Hunt Atlas reference ready: {} species",
                        reference.species.len()
                    ));
                    self.hunt_reference = Some(Arc::new(reference));
                    self.hunt_reference_error = None;
                }
                Err(error) => {
                    logging::warn(&format!("Native Hunt Atlas reference unavailable: {error}"));
                    self.hunt_reference_error = Some(error);
                }
            }
        }
        // Only the game TAB is closed before proof. The authentication browser
        // stays alive until both native sockets have received real welcome.
        while let Ok(event) = self.native_verified_rx.try_recv() {
            let index = event.index;
            if !self.native_verifying[index] { continue; }
            self.native_verifying[index] = false;
            let profile = GameProfile::from_index(index).expect("known slot");
            let native = match event.result {
                Ok(native) => native,
                Err(error) => {
                    self.native_error[index] = Some(error.clone());
                    self.set_status(format!(
                        "{}: native verification failed: {error}; Firefox game restored or recovery reported",
                        self.account_name(profile)), true);
                    continue;
                }
            };
            if self.games[index].handoff_closing || self.games[index].native.is_some() {
                native.stop();
                continue;
            }
            let mut child = self.games[index].child.take();
            let orphaned_pid = self.games[index].orphaned_pid.take();
            if child.is_none() && orphaned_pid.is_none() {
                native.stop();
                self.set_status("Could not find the Firefox window used to sign in", true);
                continue;
            }
            let monitor = self.games[index].monitor.take();
            self.games[index].handoff_closing = true;
            self.native_close_cancelled[index] = false;
            let tx = self.native_closed_tx.clone();
            logging::info(&format!(
                "{} native game welcome + Twitch IRC 001 confirmed; closing Firefox",
                self.account_name(profile)));
            std::thread::spawn(move || {
                let mut result = if let Some(child) = child.as_mut() {
                    firefox::stop_child(child, monitor.as_ref())
                } else {
                    Ok(())
                };
                if result.is_ok() {
                    result = crate::config::Config::for_profile(profile)
                        .and_then(|config| firefox::stop_profile_processes(&config));
                }
                if result.is_ok() {
                    if let Some(monitor) = monitor.as_ref() { monitor.stop(); }
                }
                let _ = tx.send(NativeBrowserClosedEvent {
                    index, native, cache_warning: event.cache_warning,
                    child, orphaned_pid, monitor, result
                });
            });
        }
        // Browser shutdown is asynchronous: the UI never blocks for BiDi
        // browser.close, process reaping or a force-kill fallback.
        while let Ok(event) = self.native_closed_rx.try_recv() {
            let index = event.index;
            let profile = GameProfile::from_index(index).expect("known slot");
            self.games[index].handoff_closing = false;
            let cancelled = std::mem::replace(&mut self.native_close_cancelled[index], false);
            match event.result {
                Ok(()) => {
                    self.native_error[index] = None;
                    // Successful Firefox close must be logged before any
                    // attempt to start native sockets, so a crash is localizable.
                    logging::info(&format!(
                        "{} Firefox exited after authenticated handoff",
                        self.account_name(profile)));
                    if cancelled {
                        self.set_status(format!(
                            "{}: sign-in canceled", self.account_name(profile)), false);
                        continue;
                    }
                    logging::info(&format!(
                        "{}: connecting to PokéIdle and Twitch", self.account_name(profile)));
                    self.games[index].native = Some(event.native);
                    self.capture_drafts[index]=None;
                    self.restock_drafts[index]=None;
                    self.restock_dirty[index]=false;
                    self.automation_drafts[index]=AutomationDraft::default();
                    self.require_browser_login[index] = false;
                    let cache_note = event.cache_warning
                        .map(|warning| format!(" · You may need to sign in next time: {warning}"))
                        .unwrap_or_default();
                    self.set_status(format!(
                        "{}: PokéIdle and Twitch connected. Firefox closed{cache_note}",
                        self.account_name(profile)), false);
                }
                Err(error) => {
                    // Native game is already authenticated; Firefox's game
                    // tab was closed. Retain native ownership and the remaining
                    // Firefox handle for a later explicit cleanup attempt.
                    self.games[index].native = Some(event.native);
                    self.capture_drafts[index]=None;
                    self.restock_drafts[index]=None;
                    self.restock_dirty[index]=false;
                    self.automation_drafts[index]=AutomationDraft::default();
                    self.games[index].child = event.child;
                    self.games[index].orphaned_pid = event.orphaned_pid;
                    self.games[index].monitor = event.monitor;
                    self.set_status(format!(
                        "{}: connected, but Firefox could not close automatically: {error}",
                        self.account_name(profile)), true);
                }
            }
        }
        // Try the protected native credential vault BEFORE constructing any
        // Firefox process. OS keyring requests run off the UI thread.
        while let Ok(event) = self.cached_native_rx.try_recv() {
            if !self.cached_native_busy[event.index] { continue; }
            self.cached_native_busy[event.index] = false;
            let profile = GameProfile::from_index(event.index).expect("known slot");
            if self.games[event.index].is_running() { continue; }
            if event.existing_firefox {
                // A stale managed Firefox process still owns this profile.
                // Do not create a competing native game socket from cache.
                // Adopt its BiDi session and let the two-phase handoff close it.
                self.require_browser_login[event.index] = true;
                self.launch_firefox_bootstrap(profile, false);
                continue;
            }
            if self.require_browser_login[event.index] {
                self.launch_firefox_bootstrap(profile, false);
                continue;
            }
            match event.result {
                Ok(Some(session)) => {
                    self.games[event.index].native = Some(session);
                    self.capture_drafts[event.index]=None;
                    self.automation_drafts[event.index]=AutomationDraft::default();
                    self.set_status(format!(
                        "{}: PokéIdle and Twitch connected with your saved login",
                        self.account_name(profile)), false);
                },
                Ok(None) => {
                    self.require_browser_login[event.index] = true;
                    self.set_status(format!(
                        "{}: please sign in to continue",
                        self.account_name(profile)), false);
                    self.launch_firefox_bootstrap(profile, false);
                }
                Err(error) => {
                    self.require_browser_login[event.index] = true;
                    self.set_status(format!(
                        "{}: saved login unavailable: {error}",
                        self.account_name(profile)), true);
                    self.launch_firefox_bootstrap(profile, false);
                }
            }
        }
        // Finish BiDi capture on the UI thread. The authenticated bootstrap
        // is never logged or written to a file.
        while let Ok(event) = self.native_handoff_rx.try_recv() {
            if !self.native_handoff_busy[event.index] { continue; }
            self.native_handoff_busy[event.index] = false;
            let profile = GameProfile::from_index(event.index).expect("known slot");
            match event.result {
                Err(error) => {
                    let pending = error.to_ascii_lowercase();
                    let is_bootstrap = pending.contains("reloaded")
                        || pending.contains("waiting") || pending.contains("pending")
                        || pending.contains("temporary twitch")
                        || pending.contains("hook installed")
                        || pending.contains("login has not produced");
                    if is_bootstrap {
                        self.native_error[event.index] = None;
                    } else {
                        self.native_error[event.index] = Some(error.clone());
                    }
                    self.set_status(format!(
                        "{}: {}: {error}",
                        self.account_name(profile),
                        if is_bootstrap { "waiting for login" } else { "login check" }
                    ), !is_bootstrap);
                },
                Ok(bootstrap) => {
                    // Never close Firefox without a usable authenticated hello
                    // and cookies, or if the slot changed while capture ran.
                    if self.games[event.index].native.is_some()
                        || !self.games[event.index].is_running()
                        || self.games[event.index].monitor.is_none()
                    {
                        self.set_status("Connection interrupted because the Firefox session changed", true);
                        continue;
                    }
                    let validated = crate::native_background::validate_bootstrap(&bootstrap);
                    if let Err(error) = validated {
                        self.native_error[event.index] = Some(error.clone());
                        self.set_status(format!("Could not connect to the game: {error}"), true);
                        continue;
                    }
                    // First prove Rust is authenticated while Firefox's live game
                    // WebSocket still exists, matching pokeidle-manager-moth.
                    // Only then close the game tab and finally Firefox.
                    if self.games[event.index].handoff_closing ||
                        self.native_verifying[event.index] { continue; }
                    let Some(monitor) = self.games[event.index].monitor.clone() else {
                        continue;
                    };
                    self.native_verifying[event.index] = true;
                    self.native_error[event.index] = None;
                    let tx = self.native_verified_tx.clone();
                    let index = event.index;
                    let cache_warning = event.cache_warning;
                    logging::info(&format!(
                        "{}: checking PokéIdle and Twitch connections before closing Firefox",
                        self.account_name(profile)));
                    let spawned = std::thread::Builder::new()
                        .name(format!("moth-native-handoff-{}", profile.name()))
                        .spawn(move || {
                            let mut attempted_game_close = false;
                            // A successful native welcome must precede the game
                            // tab close. Closing Firefox's original socket first
                            // could invalidate the just-captured hello/token.
                            let result = std::panic::catch_unwind(
                                std::panic::AssertUnwindSafe(|| {
                                    crate::native_tls::ensure()?;
                                    let native = NativeSession::start_verified(profile, bootstrap)?;
                                    attempted_game_close = true;
                                    monitor.close_game_for_handoff(Duration::from_secs(18))?;
                                    Ok(native)
                                })
                            ).unwrap_or_else(|_| {
                                Err("Login check failed unexpectedly. See the controller log.".to_owned())
                            });
                            let result = match (result, attempted_game_close) {
                                (Err(error), true) => {
                                    match monitor.restore_game_after_failed_handoff(
                                        Duration::from_secs(20)
                                    ) {
                                        Ok(()) => Err(format!("{error}; game tab reopened")),
                                        Err(recovery) => {
                                            crate::logging::error(&format!(
                                                "Could not recover the connection: {recovery}"
                                            ));
                                            Err(format!("{error}; game tab recovery failed: {recovery}"))
                                        }
                                    }
                                }
                                (other, _) => other,
                            };
                            let _ = tx.send(NativeVerifiedEvent {
                                index, result, cache_warning
                            });
                        });
                    if let Err(error) = spawned {
                        self.native_verifying[index] = false;
                        self.native_error[index] = Some(format!(
                            "Could not start the login check: {error}"
                        ));
                        self.set_status(format!(
                            "{}: could not verify login: {error}",
                            self.account_name(profile)), true);
                    }
                }
            }
        }
        let mut recover = Vec::new();
        for slot in &mut self.games {
            if slot.native.as_ref().is_some_and(|session|
                session.snapshot().phase == "NeedsLogin" ||
                session.twitch_snapshot().phase == "NeedsLogin"
            ) {
                if let Some(session) = slot.native.take() { session.stop(); }
                recover.push(slot.profile);
            }
        }
        for profile in recover {
            self.require_browser_login[profile.index()] = true;
            self.set_status(format!(
                "{}: login expired, opening Firefox to sign in again",
                self.account_name(profile)), true);
            self.launch_firefox_bootstrap(profile, false);
        }
        let mut command_feedback = Vec::new();
        for slot in &mut self.games {
            if let Some(monitor) = slot.monitor.as_ref() {
                while let Some(result) = monitor.take_command_feedback() {
                    command_feedback.push((
                        self.accounts[slot.profile.index()].name.clone(),
                        result,
                    ));
                }
            }
            let exited = match slot.child.as_mut() {
                Some(child) => match child.try_wait() {
                    Ok(Some(_)) => true,
                    Ok(None) => false,
                    Err(error) => {
                        logging::warn(&format!(
                            "{} Firefox process check failed: {error}", slot.profile.label()
                        ));
                        false
                    }
                },
                None => false,
            };
            if exited {
                let account_name = self.accounts[slot.profile.index()].name.clone();
                logging::info(&format!("{} Firefox child process exited", account_name));
                slot.child = None;

                // Firefox can outlive the original launcher handle. Keep the
                // existing BiDi monitor for that same profile rather than
                // permanently discarding telemetry from a still-open game.
                match Config::for_profile(slot.profile)
                    .and_then(|config| firefox::find_profile_process(&config))
                {
                    Ok(Some(pid)) => {
                        slot.orphaned_pid = Some(pid);
                        logging::warn(&format!(
                            "{} Firefox launcher exited; profile PID {} is still running, preserving monitor",
                            account_name, pid
                        ));
                    }
                    Ok(None) => {
                        slot.orphaned_pid = None;
                        if let Some(monitor) = slot.monitor.take() {
                            monitor.stop();
                        }
                    }
                    Err(error) => logging::warn(&format!(
                        "{} profile recovery failed (keeping monitor until ownership is known): {}",
                        account_name, error
                    )),
                }
            }
            if let Some(pid) = slot.orphaned_pid {
                if !firefox::process_alive(pid) {
                    slot.orphaned_pid = None;
                    if let Some(monitor) = slot.monitor.take() {
                        monitor.stop();
                    }
                }
            }
        }
        for (account_name, result) in command_feedback {
            self.set_status(
                format!("{} · {} #{}: {}", account_name, tr("Tab"), result.id, result.message),
                !result.ok,
            );
        }
        // Temporary Firefox authentication is auto-handed off to native.
        // An explicitly opened interactive Firefox must stay open instead.
        for profile in GameProfile::ALL {
            let index = profile.index();
            if !self.accounts[index].enabled || self.games[index].browser_only ||
               self.native_handoff_busy[index] || self.games[index].native.is_some() ||
               self.native_verifying[index] ||
               self.games[index].handoff_closing || self.games[index].monitor.is_none() {
                continue;
            }
            let health = self.games[index].health();
            // BiDi's captured game hello, not a DOM telemetry sample, is the
            // authentication signal. The reference manager also observes the
            // hello while the login page is still transitioning.
            if health.state != "Running" {
                continue;
            }
            if self.native_handoff_last_attempt[index]
                .is_some_and(|at| at.elapsed() < Duration::from_secs(8)) { continue; }
            self.start_native_handoff(profile);
        }
    }

    fn idle_warning_for(&mut self, index: usize, health: &Health) -> bool {
        let tracker = &mut self.idle_progress[index];
        let eligible = health.telemetry_is_fresh()
            && health.game_ready
            && health.logged_in
            && health.bridge_connected
            && health.activity == "Hunting"
            && !health.player_xp.is_empty();

        if !eligible {
            *tracker = IdleProgressTracker::default();
            return false;
        }

        tracker.observe(health)
    }

    fn set_status(&mut self, message: impl Into<String>, error: bool) {
        self.status = message.into();
        self.status_error = error;
        self.events.push_front(UiEvent {
            at: Instant::now(),
            message: self.status.clone(),
            is_error: error,
        });
        self.events.truncate(80);
        if error {
            logging::warn(&self.status);
        }
    }

    fn start_native_handoff(&mut self, profile: GameProfile) {
        let index = profile.index();
        if self.native_handoff_busy[index] { return; }
        let Some(monitor) = self.games[index].monitor.clone() else {
            self.set_status("Open Firefox and sign in to PokéIdle first", true);
            return;
        };
        self.native_handoff_busy[index] = true;
        self.native_handoff_last_attempt[index] = Some(Instant::now());
        logging::info(&format!(
            "{} capturing native game + Twitch IRC session via Firefox BiDi",
            self.account_name(profile)));
        self.set_status(format!("{}: signing in to PokéIdle and Twitch",
            self.account_name(profile)), false);
        let tx = self.native_handoff_tx.clone();
        let result = std::thread::Builder::new()
            .name(format!("moth-handoff-{}", profile.name()))
            .spawn(move || {
                let mut result = monitor.native_bootstrap(Duration::from_secs(18));
                // Firefox provides fresh authentication, not authoritative
                // native preferences. Preserve this account's saved Auto Catch
                // configuration, even after the browser login is renewed.
                // Cache and worker must receive the SAME merged bootstrap.
                let mut cache_warning = None;
                if let Ok(value) = &mut result {
                    if crate::native_background::validate_bootstrap(value).is_ok() {
                        let (merged,warning) =
                            crate::native_auth::cache_browser_handoff(profile,value);
                        *value=merged;
                        cache_warning=warning;
                    }
                }
                let _ = tx.send(NativeHandoffEvent { index, result, cache_warning });
            });
        if let Err(error) = result {
            self.native_handoff_busy[index] = false;
            self.set_status(format!("Could not finish sign-in: {error}"), true);
        }
    }

    fn launch_one(&mut self, profile: GameProfile) {
        self.launch_one_mode(profile, true);
    }

    fn launch_one_mode(&mut self, profile: GameProfile, headless: bool) {
        let index = Self::game_index(profile);
        if self.games[index].is_running() || self.cached_native_busy[index] ||
            self.native_handoff_busy[index] {
            return;
        }
        self.games[index].headless = headless;
        self.games[index].browser_only = false;
        if self.require_browser_login[index] {
            self.launch_firefox_bootstrap(profile, false);
            return;
        }
        self.cached_native_busy[index] = true;
        let tx = self.cached_native_tx.clone();
        let started = std::thread::Builder::new()
            .name(format!("moth-cache-{}", profile.name()))
            .spawn(move || {
                // A managed Firefox profile from a previous crash may still
                // have an active game socket. Determine ownership BEFORE using
                // saved credentials, or the same account may connect twice.
                let detected = Config::for_profile(profile)
                    .and_then(|config| firefox::find_profile_process(&config));
                let existing_firefox = matches!(detected.as_ref(), Ok(Some(_)));
                // A cache preflight panic must never strand the UI in
                // "starting" state. Report it, then authenticate via Firefox.
                let result = std::panic::catch_unwind(
                    std::panic::AssertUnwindSafe(|| {
                        crate::native_tls::ensure()?;
                        detected.and_then(|_| {
                            if existing_firefox { return Ok(None); }
                            crate::native_auth::load(profile)
                                .and_then(|saved| saved.map(|bootstrap| {
                                    // Never project a cached, unverified connection as
                                    // started. A failed welcome needs a fresh Firefox
                                    // authentication, not infinite background retries.
                                    NativeSession::start_verified(profile, bootstrap)
                                }).transpose())
                        })
                    })
                ).unwrap_or_else(|_| {
                    Err("Native cache verification panicked; see sanitized controller log".into())
                });
                let _ = tx.send(CachedNativeEvent {
                    index, existing_firefox, result
                });
            });
        if let Err(error) = started {
            self.cached_native_busy[index] = false;
            self.require_browser_login[index] = true;
            self.set_status(format!("Saved login could not be used: {error}"), true);
            self.launch_firefox_bootstrap(profile, false);
        }
    }

    /// Authentication-only browser fallback. Never call as the ordinary
    /// Launch path while reusable native game + Twitch credentials exist.
    fn launch_firefox_bootstrap(&mut self, profile: GameProfile, headless: bool) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);
        if self.games[index].is_running() { return; }

        let config = match crate::config::Config::for_profile(profile) {
            Ok(config) => config,
            Err(error) => {
                self.set_status(format!("{}: {}", self.account_name(profile), error), true);
                return;
            }
        };

        // An existing Firefox may own this profile even if the controller was
        // restarted or its Child handle was lost. Never spawn over that session.
        match firefox::find_profile_process(&config) {
            Ok(Some(pid)) => {
                // A previous crash may have left an authenticated, controlled
                // Moth Firefox profile alive. Attach to its existing BiDi
                // endpoint and complete native handoff; never spawn another
                // Firefox on the same profile or kill it before Rust is ready.
                self.games[index].orphaned_pid = Some(pid);
                if self.games[index].monitor.is_none() {
                    self.games[index].monitor = Some(MonitorHandle::start(
                        config.remote_debug_port, headless));
                }
                logging::info(&format!(
                    "{} adopting existing Moth Firefox profile PID {} for native handoff",
                    account_name, pid
                ));
                self.set_status(format!(
                    "{}: using your existing Firefox login to connect (process {})",
                    account_name, pid
                ), false);
                return;
            }
            Ok(None) => {}
            Err(error) => {
                self.set_status(format!("{}: {}", account_name, error), true);
                return;
            }
        }

        self.games[index].child = None;
        self.games[index].monitor = None;
        self.games[index].orphaned_pid = None;
        self.games[index].headless = headless;

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
                        "{}: Firefox started in {} for sign-in",
                        self.account_name(profile),
                        if headless { "the background" } else { "a window" }
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

    /// Explicit interactive browser mode, separate from the temporary native
    /// authentication bootstrap. Do not open two game sockets for one account.
    fn open_in_firefox(&mut self, profile: GameProfile) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);

        // A handoff worker might already be closing the Firefox game tab or
        // shutting down its process. Never race it with a new browser session.
        if self.native_verifying[index] || self.games[index].handoff_closing {
            self.set_status(format!(
                "{account_name}: Sign-in is still finishing. Open Firefox after it completes."
            ), true);
            return;
        }

        if self.games[index].browser_only && !self.games[index].headless &&
            self.games[index].is_running() {
            self.set_status(format!("{account_name}: Firefox is already open."), false);
            return;
        }

        // Stop native game/IRC or an authentication-only Firefox first. In
        // particular, don't run native and Firefox on the same game account.
        if !self.stop_one(profile) { return; }
        self.games[index].browser_only = true;
        self.launch_firefox_bootstrap(profile, false);
    }

    fn repair_monitor(&mut self, profile: GameProfile) {
        // An orphaned Firefox BiDi session cannot be reclaimed by a new
        // session.new. Only an explicit user action restarts this profile;
        // never interrupt a running game automatically on telemetry failure.
        let index = Self::game_index(profile);
        let headless = self.games[index].headless;
        let browser_only = self.games[index].browser_only;
        logging::warn(&format!(
            "{} repairing an exhausted BiDi session by user-requested Firefox restart",
            self.account_name(profile)
        ));
        if self.stop_one(profile) {
            if browser_only {
                // A manual Firefox launch must remain a manual Firefox launch
                // even after an explicit monitor repair/restart.
                self.games[index].browser_only = true;
                self.launch_firefox_bootstrap(profile, false);
            } else {
                self.launch_one_mode(profile, headless);
            }
        }
    }

    fn stop_one(&mut self, profile: GameProfile) -> bool {
        let index = Self::game_index(profile);
        // A stopped account must not keep an independent Kick worker alive.
        self.kick_monitors[index].take();
        let account_name = self.account_name(profile);
        if self.games[index].handoff_closing {
            // The dedicated shutdown worker already owns and is closing the
            // Firefox process. Do not start native mode after explicit Stop.
            self.native_close_cancelled[index] = true;
            self.set_status(format!(
                "{}: stopping sign-in", account_name), false);
            return true;
        }
        // Cancel in-flight cache lookups and browser handoffs. Results from
        // canceled workers are ignored even if a Stop is pressed mid-login.
        self.cached_native_busy[index] = false;
        self.native_handoff_busy[index] = false;
        self.native_verifying[index] = false;
        let slot = &mut self.games[index];
        if let Some(native) = slot.native.take() {
            native.stop();
            // A failed browser cleanup can leave an authenticated native game
            // plus an owned Firefox launcher. Stop must close BOTH owners.
            if slot.child.is_none() && slot.orphaned_pid.is_none() {
                if let Some(monitor) = slot.monitor.take() { monitor.stop(); }
                self.set_status(format!("{}: PokéIdle and Twitch stopped.", account_name), false);
                return true;
            }
        }

        // Stop also works after restarting the controller, before any attempt
        // to launch that profile has made the orphan process visible.
        if slot.child.is_none() && slot.orphaned_pid.is_none() {
            if let Ok(config) = Config::for_profile(profile) {
                match firefox::find_profile_process(&config) {
                    Ok(Some(pid)) => slot.orphaned_pid = Some(pid),
                    Ok(None) => {}
                    Err(error) => {
                        self.set_status(format!("{}: {}", account_name, error), true);
                        return false;
                    }
                }
            }
        }

        if let Some(mut child) = slot.child.take() {
            logging::info(&format!(
                "stopping {} Firefox PID {}", account_name, child.id()
            ));
            let closed = firefox::stop_child(&mut child, slot.monitor.as_ref())
                .and_then(|_| Config::for_profile(profile))
                .and_then(|config| firefox::stop_profile_processes(&config));
            match closed {
                Ok(()) => {
                    if let Some(monitor) = slot.monitor.take() {
                        monitor.stop();
                    }
                    slot.orphaned_pid = None;
                    self.set_status(format!("{} Firefox profile completely closed.", account_name), false);
                    true
                }
                Err(error) => {
                    logging::error(&format!("{} Firefox shutdown failed: {}", account_name, error));
                    slot.child = Some(child);
                    self.set_status(format!("{}: could not close Firefox: {}", account_name, error), true);
                    false
                }
            }
        } else if slot.orphaned_pid.is_some() {
            // Stop was explicitly clicked; never terminate a recovered process
            // automatically when the controller itself exits.
            let result = Config::for_profile(profile)
                .and_then(|config| firefox::stop_profile_processes(&config));
            match result {
                Ok(()) => {
                    slot.orphaned_pid = None;
                    if let Some(monitor) = slot.monitor.take() {
                        monitor.stop();
                    }
                    self.set_status(format!("{} Firefox closed.", account_name), false);
                    true
                }
                Err(error) => {
                    self.set_status(format!("{}: could not close Firefox: {}", account_name, error), true);
                    false
                }
            }
        } else {
            if let Some(monitor) = slot.monitor.take() {
                monitor.stop();
            }
            self.set_status(format!("{} is already stopped.", account_name), false);
            true
        }
    }

    fn launch_enabled(&mut self) {
        for profile in GameProfile::ALL {
            if self.account_enabled(profile) {
                self.launch_one(profile);
            }
        }
        // Keep any launch error visible rather than overwriting it with a
        // success-shaped summary when a profile is already in use.
        if !self.status_error {
            self.set_status("Starting enabled accounts. Firefox will open only if sign-in is needed.", false);
        }
    }

    fn stop_all(&mut self) {
        let mut all_stopped = true;
        for profile in GameProfile::ALL {
            if !self.stop_one(profile) {
                all_stopped = false;
            }
        }
        if all_stopped {
            self.set_status("All accounts stopped. Firefox login windows were closed.", false);
        } else {
            self.set_status("Some Firefox instances could not be stopped; check the log.", true);
        }
    }

    fn open_logs(&mut self) {
        let path = match crate::config::data_root() {
            Ok(root) => root.join("moth-controller.log"),
            Err(error) => {
                self.set_status(error, true);
                return;
            }
        };
        if let Some(parent) = path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                self.set_status(format!("Could not create log folder: {error}"), true);
                return;
            }
        }
        if !path.exists() {
            if let Err(error) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                self.set_status(format!("Could not create log: {error}"), true);
                return;
            }
        }
        let opener = if cfg!(windows) { "notepad.exe" } else { "xdg-open" };
        match std::process::Command::new(opener).arg(&path).spawn() {
            Ok(_) => self.set_status(format!("Opened controller log · {}", path.display()), false),
            Err(error) => self.set_status(format!("Could not open log: {error}"), true),
        }
    }

    fn open_game_for_google_sign_in(&mut self, profile: GameProfile) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);
        self.refresh_processes();

        // Google may reject authentication in a browser launched with a remote
        // debugging/automation endpoint. Open a visible, non-debuggable window
        // solely for the user to complete sign-in; normal launches keep BiDi.
        if !self.stop_one(profile) {
            return;
        }

        let config = match Config::for_profile(profile) {
            Ok(config) => config,
            Err(error) => {
                self.set_status(format!("{}: {}", account_name, error), true);
                return;
            }
        };

        // Google sign-in uses the same profile with debugging disabled; it
        // still must not race another Firefox that owns that profile.
        match firefox::find_profile_process(&config) {
            Ok(Some(pid)) => {
                self.games[index].orphaned_pid = Some(pid);
                self.set_status(
                    format!("{}: Firefox already uses this profile (PID {}). Use Stop before launching.", account_name, pid),
                    true,
                );
                return;
            }
            Ok(None) => {}
            Err(error) => {
                self.set_status(format!("{}: {}", account_name, error), true);
                return;
            }
        }

        match firefox::launch_unmonitored(&config) {
            Ok(child) => {
                let pid = child.id();
                self.games[index].child = Some(child);
                self.games[index].monitor = None;
                self.games[index].orphaned_pid = None;
                self.games[index].headless = false;
                logging::info(&format!(
                    "{} Firefox spawned for Google sign-in without remote debugging (PID {})",
                    account_name, pid
                ));
                self.set_status(
                    format!(
                        "{} opened Firefox without remote debugging for Google sign-in. After signing in, close that window and launch this account normally from the dashboard.",
                        account_name
                    ),
                    false,
                );
            }
            Err(error) => {
                logging::error(&format!(
                    "{} Google sign-in launch failed: {}",
                    account_name, error
                ));
                self.set_status(format!("{}: {}", account_name, error), true);
            }
        }
    }

    fn profile_action(&mut self, profile: GameProfile, action: ProfileAction) {
        let result = match action {
            ProfileAction::GoogleSignIn => {
                self.open_game_for_google_sign_in(profile);
                return;
            }
            ProfileAction::Twitch => accounts::open_login(profile, accounts::TWITCH_LOGIN, "Twitch"),
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
        #[cfg(windows)]
        if let Some(tray) = self.tray.as_mut() {
            tray.shutdown();
        }

        for slot in &mut self.games {
            if let Some(mut child) = slot.child.take() {
                let account_name = self.accounts[slot.profile.index()].name.clone();
                logging::info(&format!(
                    "controller shutting down; closing {} Firefox PID {}",
                    account_name,
                    child.id()
                ));
                if let Err(error) = firefox::stop_child(&mut child, slot.monitor.as_ref()) {
                    logging::error(&format!("{} shutdown failed: {}", account_name, error));
                }
            }
            if let Some(monitor) = slot.monitor.take() {
                monitor.stop();
            }
            // Recovered processes are intentionally not owned by this controller
            // instance. Only an explicit Stop action may close those browsers.
        }
    }
}

impl eframe::App for ControllerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.refresh_processes();
        self.sync_pokeidle_kick_streamers();
        // Drain replies from requests made on the previous UI frame before
        // resetting the current frame's Pokémon sprite usage.
        self.visuals.poll(ui.ctx());
        self.visuals.begin_frame();
        if self.account_setup_checked_at
            .map(|when| when.elapsed() >= Duration::from_secs(5 * 60))
            .unwrap_or(true)
        {
            self.refresh_account_setup();
        }

        // Live native panels publish snapshots at one-second cadence.
        // Repaint only while visible; tray/background retains the cheap timer.
        let background = self.controller_in_background(ui.ctx());
        let native_detail = self.games.iter().any(|game| game.native.is_some())
            && (matches!(self.panel_tab, PanelTab::Hunts | PanelTab::Automation)
                || self.panel_tab == PanelTab::Overview);
        ui.ctx().request_repaint_after(
            if background { UI_BACKGROUND_REPAINT_INTERVAL }
            else if native_detail { Duration::from_secs(1) }
            else { UI_REPAINT_INTERVAL }
        );

        draw_sidebar(self, ui);

        egui::Panel::top("header")
            .frame(egui::Frame::new().fill(BG).inner_margin(Margin::symmetric(20, 13)))
            .show(ui, |ui| {
                // On smaller windows give actions their own row instead of
                // allowing right-aligned buttons to overlap the page title.
                let compact = ui.available_width() < 780.0;
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr(match self.panel_tab {
                            PanelTab::Overview => "Overview",
                            PanelTab::Hunts => "Hunt Atlas",
                            PanelTab::Accounts => "Accounts",
                            PanelTab::Automation => "Automation",
                            PanelTab::Options => "Options",
                        }))
                            .font(FontId::proportional(23.0))
                            .strong()
                            .color(TEXT),
                    );
                    if !compact {
                        ui.add_space(12.0);
                        ui.label(RichText::new(tr("PokéIdle controller"))
                            .size(12.0).color(MUTED));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            draw_header_actions(self, ui);
                        });
                    }
                });
                if compact {
                    ui.add_space(5.0);
                    ui.horizontal_wrapped(|ui| draw_header_actions(self, ui));
                }
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
                    let localized = localize_status(self.status.clone());
                    ui.add(egui::Label::new(
                        RichText::new(&localized)
                            .size(12.0)
                            .color(if self.status_error { TEXT } else { MUTED }),
                    ).truncate()).on_hover_text(localized);
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(Margin::symmetric(16, 10)))
            .show(ui, |ui| {
                // Accounts has its own full page with an account selector.
                // All other native workspaces share the same compact strip.
                let active = match self.panel_tab {
                    PanelTab::Overview => Some(self.selected_details),
                    PanelTab::Hunts if self.hunt_subtab == HuntSubtab::Compare => None,
                    PanelTab::Hunts => Some(Some(self.hunt_account)),
                    PanelTab::Automation => Some(Some(self.automation_account)),
                    _ => None,
                };
                if let Some(selected) = active {
                    let next = draw_account_switcher(
                        self, ui, selected, self.panel_tab == PanelTab::Overview
                    );
                    if next != selected {
                        if let Some(index) = next {
                            self.select_workspace_account(index);
                            if self.panel_tab == PanelTab::Overview {
                                self.selected_details = Some(index);
                            }
                        } else if self.panel_tab == PanelTab::Overview {
                            self.selected_details = None;
                        }
                    }
                    ui.add_space(6.0);
                }
                match self.panel_tab {
                    PanelTab::Overview => draw_instance_section(self, ui),
                    PanelTab::Hunts => hunt_panel::draw_hunt_atlas_panel(self, ui),
                    PanelTab::Accounts => draw_accounts_panel(self, ui),
                    PanelTab::Automation => draw_automation_panel(self, ui),
                    PanelTab::Options => draw_options_panel(self, ui),
                }
            });

        // Keep exactly the Pokémon art used by Overview this frame (at most
        // two sprites per account). Hunt Atlas and other tabs keep none.
        self.visuals.end_frame();

        // Only keep Explorer results for the account actually being viewed.
        // Rankings can be rebuilt on demand; holding all four accounts'
        // destination copies throughout Overview/Automation wastes RAM.
        let active_explorer=if self.panel_tab==PanelTab::Hunts &&
            self.hunt_subtab==HuntSubtab::Explorer {
            Some(self.hunt_account)
        } else {None};
        for (index,results) in self.atlas_results.iter_mut().enumerate() {
            if active_explorer!=Some(index) {results.release();}
        }

        // If the user leaves Accounts while editing the name, persist it
        // without needing to close a dialog that no longer exists.
        if self.panel_tab != PanelTab::Accounts {
            self.save_dirty_account_names();
        }
        // Apply the persisted preference immediately to all Firefox monitors.
        // Neither native game nor Twitch connections are throttled.
        self.sync_monitor_paces(self.controller_in_background(ui.ctx()));
    }
}

/// Shared compact toolbar for both single-row and wrapped headers.
fn draw_header_actions(app: &mut ControllerApp, ui: &mut egui::Ui) {
    if ui.add(
        egui::Button::new(RichText::new(tr("Launch enabled")).strong().size(11.0))
            .fill(ACCENT.linear_multiply(0.8))
            .stroke(Stroke::new(1.0, ACCENT))
            .corner_radius(8.0),
    ).on_hover_text(tr("Start all enabled accounts")).clicked() {
        app.launch_enabled();
    }
    if ui.add(
        egui::Button::new(RichText::new(tr("Stop all")).strong().size(11.0))
            .fill(BAD.linear_multiply(0.14))
            .stroke(Stroke::new(1.0, BAD.linear_multiply(0.55)))
            .corner_radius(8.0),
    ).on_hover_text(tr("Stop all running accounts")).clicked() {
        app.stop_all();
    }
    #[cfg(windows)]
    if ui.add(
        egui::Button::new(RichText::new(tr("To tray")).size(11.0))
            .fill(PANEL)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(8.0),
    ).on_hover_text(tr("Hide this window in the system tray; click the tray icon to restore it"))
    .clicked() {
        if app.tray.is_some() {
            app.tray_hidden.store(true, Ordering::Relaxed);
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(false));
            logging::info("controller window hidden to system tray");
        } else {
            app.set_status(tr("System tray is unavailable; the window cannot be hidden.").to_owned(), true);
        }
    }
}

/// Same compact account navigation is reused by every native data page and
/// the account manager. Clicking one account changes the workspace selection,
/// not the running game. Status always reflects the real native session.
fn draw_account_switcher(
    app: &ControllerApp,
    ui: &mut egui::Ui,
    selected: Option<usize>,
    include_all: bool,
) -> Option<usize> {
    let mut next = selected;
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8.0)
        .inner_margin(Margin::symmetric(9, 7))
        .show(ui, |ui| {
            let button_width = if ui.available_width() < 780.0 { 112.0 } else { 128.0 };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("ACCOUNT")).size(10.0).strong().color(DIM));
                if include_all {
                    let is_all = selected.is_none();
                    if ui.add_sized([105.0, 34.0],
                        egui::Button::new(RichText::new(tr("All accounts")).size(11.0))
                            .fill(if is_all { ACCENT.linear_multiply(0.20) } else { PANEL_ALT })
                            .stroke(Stroke::new(1.0, if is_all { ACCENT } else { BORDER }))
                            .corner_radius(6.0)).clicked() {
                        next = None;
                    }
                }
                for index in 0..app.games.len() {
                    let slot = &app.games[index];
                    let profile = slot.profile;
                    let (state, color) = if let Some(native) = slot.native.as_ref() {
                        let snapshot = native.snapshot();
                        match snapshot.phase.as_str() {
                            "Online" if snapshot.last_received
                                .is_some_and(|at| at.elapsed() <= Duration::from_secs(90)) =>
                                (tr("Online"), GOOD),
                            "Online" => (tr("Stale"), WARN),
                            "NeedsLogin" => (tr("Sign-in required"), WARN),
                            "Reconnecting" => (tr("Reconnecting"), WARN),
                            "Offline" => (tr("Offline"), DIM),
                            _ => (tr("Waiting for data"), WARN),
                        }
                    } else if app.cached_native_busy[index] || app.native_verifying[index]
                        || app.native_handoff_busy[index] || slot.handoff_closing {
                        (tr("Starting"), WARN)
                    } else if slot.child.is_some() || slot.orphaned_pid.is_some() {
                        (tr("Firefox"), WARN)
                    } else if !app.accounts[index].enabled {
                        (tr("Disabled"), DIM)
                    } else {
                        (tr("Offline"), DIM)
                    };
                    let selected_here = selected == Some(index);
                    let name = compact_text(&app.accounts[index].name, 15);
                    let text = format!("{} {}\n{}", if color == GOOD { "●" } else if color == WARN { "◐" } else { "○" },
                        name, state);
                    let response = ui.add_sized([button_width, 40.0],
                        egui::Button::new(RichText::new(text).size(10.0)
                            .color(if selected_here { TEXT } else { MUTED }))
                            .fill(if selected_here {
                                ACCENT.linear_multiply(0.20)
                            } else {
                                PANEL_ALT
                            })
                            .stroke(Stroke::new(1.0, if selected_here { ACCENT }
                                else if color == GOOD { GOOD.linear_multiply(0.45) }
                                else if color == WARN { WARN.linear_multiply(0.45) }
                                else { BORDER }))
                            .corner_radius(6.0));
                    if response.on_hover_text(format!("{} · {}", profile.label(), state)).clicked() {
                        next = Some(index);
                    }
                }
            });
        });
    next
}

/// Performance preferences persist independently of accounts and profiles.
fn draw_options_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_salt("options-page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let width = ui.available_width().min(720.0);
            ui.allocate_ui_with_layout(
                egui::vec2(width, 0.0), Layout::top_down(Align::Min),
                |ui| draw_options_contents(app, ui),
            );
        });
}

fn draw_options_contents(app: &mut ControllerApp, ui: &mut egui::Ui) {
    ui.label(RichText::new(tr("Performance and updates"))
        .size(17.0).strong().color(TEXT));
    ui.add_space(5.0);
    ui.label(RichText::new(tr("Choose when Moth Controller uses fewer resources in the background."))
        .size(11.0).color(MUTED));
    ui.add_space(12.0);

    let mut next = app.ui_options;
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(egui::Margin::same(16)).show(ui, |ui| {
            ui.label(RichText::new(tr("Background refresh"))
                .size(13.0).strong().color(TEXT));
            ui.add_space(8.0);

            ui.checkbox(
                &mut next.reduce_updates_unfocused,
                tr("Reduce updates when the window is not focused"),
            );
            ui.label(RichText::new(tr(
                "When enabled, refresh this window less often while you use other apps. The game keeps running normally."
            )).size(10.0).color(MUTED));

            ui.add_space(10.0);
            ui.checkbox(
                &mut next.reduce_updates_in_tray,
                tr("Reduce updates when minimized to the system tray"),
            );
            ui.label(RichText::new(tr(
                "On: slower refresh while hidden in the tray. Off: normal foreground refresh continues while hidden."
            )).size(10.0).color(MUTED));

            ui.add_space(10.0);
            ui.separator();
            ui.label(RichText::new(tr(
                "PokéIdle and Twitch keep running normally. These options only change how often the window and Firefox update."
            )).size(10.0).color(DIM));
        });

    if next != app.ui_options {
        // The change is applied only after a successful disk write. If the
        // options directory is unavailable, keep the previous live setting.
        app.save_ui_options(next);
        ui.ctx().request_repaint();
    }

    ui.add_space(12.0);
    let slowed=app.controller_in_background(ui.ctx());
    ui.label(RichText::new(format!("{}: {}",
        tr("Current refresh mode"),
        tr(if slowed {"Reduced background updates"} else {"Normal foreground updates"})))
        .size(11.0).color(if slowed { WARN } else { GOOD }));
    if ui.small_button(tr("Restore default performance settings")).clicked() {
        app.save_ui_options(UiOptions::default());
    }
    ui.label(RichText::new(tr("Changes are saved automatically and restored on the next launch."))
        .size(11.0).color(DIM));
}

#[derive(Clone, Copy)]
enum ProfileAction {
    GoogleSignIn,
    Twitch,
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

            if sidebar_button(ui, tr("Dashboard"), app.panel_tab == PanelTab::Overview).clicked() {
                app.panel_tab = PanelTab::Overview;
            }

            if sidebar_button(ui, &format!("  ·  {}", tr("Hunt Atlas")), app.panel_tab == PanelTab::Hunts).clicked() {
                app.panel_tab = PanelTab::Hunts;
            }

            if sidebar_button(ui, tr("Accounts"),
                app.panel_tab == PanelTab::Accounts &&
                app.accounts_section == AccountsSection::Profiles).clicked() {
                app.panel_tab = PanelTab::Accounts;
                app.accounts_section = AccountsSection::Profiles;
            }
            if sidebar_button(ui, "  ·  Kick",
                app.panel_tab == PanelTab::Accounts &&
                app.accounts_section == AccountsSection::Kick).clicked() {
                app.panel_tab = PanelTab::Accounts;
                app.accounts_section = AccountsSection::Kick;
            }

            if sidebar_button(ui, tr("Automation"), app.panel_tab == PanelTab::Automation).clicked() {
                app.panel_tab = PanelTab::Automation;
            }

            if sidebar_button(ui, tr("Logs"), false).clicked() {
                app.open_logs();
            }

            if sidebar_button(ui, tr("Options"), app.panel_tab == PanelTab::Options).clicked() {
                app.panel_tab = PanelTab::Options;
            }

            // Global start/stop actions are in the header; avoid a duplicate
            // control set that crowds the persistent navigation.
            ui.add_space(18.0);
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
                        RichText::new(tr("CONNECTIONS"))
                            .size(10.0)
                            .strong()
                            .color(DIM),
                    );
                    ui.add_space(7.0);
                    let active: Vec<_> = app.games.iter()
                        .filter(|game| game.child.is_some() || game.orphaned_pid.is_some())
                        .collect();
                    let native_online = app.games.iter().filter(|game|
                        game.native.as_ref().is_some_and(|native|
                            native.snapshot().phase == "Online")).count();
                    let mode = if active.is_empty() {
                        tr("STOPPED")
                    } else if active.iter().all(|game| game.headless) {
                        tr("HIDDEN")
                    } else if active.iter().all(|game| !game.headless) {
                        tr("VISIBLE")
                    } else {
                        tr("MIXED")
                    };
                    let connected = active.iter()
                        .filter(|game| game.monitor.as_ref().is_some_and(|monitor| monitor.health().heartbeat_is_fresh()))
                        .count();
                    runtime_row(ui, tr("PokéIdle"), &format!("{native_online}/4 online"),
                        if native_online>0 {GOOD} else {MUTED});
                    runtime_row(ui, "Firefox", mode, if active.is_empty() { MUTED } else { GOOD });
                    if !active.is_empty() {
                        runtime_row(ui, tr("Firefox login"), &format!("{connected}/{} {}", active.len(), tr("Connected")),
                            if connected == active.len() { GOOD } else { MUTED });
                    }
                });

            ui.add_space(12.0);
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.add_space(6.0);
                draw_credits(ui, &app.update_status);
                ui.add_space(10.0);
                ui.label(
                    RichText::new(if cfg!(windows) { tr("Moth Controller · Windows") } else { tr("Moth Controller · Linux") })
                        .size(10.0)
                        .color(DIM),
                );
            });
        });
}

fn multikick_link(streams: &[KickStream]) -> Option<(String, Vec<String>)> {
    let mut channels: Vec<String> = Vec::new();

    for stream in streams.iter() {
        let url = stream.url.trim();
        let Some(path) = url
            .strip_prefix("https://kick.com/")
            .or_else(|| url.strip_prefix("https://www.kick.com/"))
        else {
            continue;
        };

        let channel = path
            .split(|character| character == '/' || character == '?' || character == '#')
            .next()
            .unwrap_or_default()
            .trim();

        if channel.is_empty()
            || !channel
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '-')
        {
            continue;
        }

        if !channels.iter().any(|existing| existing.eq_ignore_ascii_case(channel)) {
            channels.push(channel.to_string());
        }
    }

    if channels.is_empty() {
        None
    } else {
        Some((format!("https://multikick.com/{}", channels.join("/")), channels))
    }
}

fn open_kick_channels_in_regular_browser(channels: &[String]) -> Result<usize, String> {
    if channels.is_empty() {
        return Err("no live KICK channels to open".to_string());
    }
    let mut opened = 0usize;
    for channel in channels {
        if channel.is_empty() || !channel.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') {
            continue;
        }
        let url = format!("https://kick.com/{channel}");
        let mut command = Command::new(if cfg!(windows) { "explorer.exe" } else { "xdg-open" });
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        command.arg(&url).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("could not open live KICK channel {channel}: {error}"))?;
        opened += 1;
    }
    if opened == 0 { Err("no valid live KICK channels to open".to_string()) } else { Ok(opened) }
}

fn draw_instance_section(app: &mut ControllerApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("ACCOUNT OVERVIEW"))
            .font(FontId::proportional(17.0)).strong().color(TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.selectable_label(app.dashboard_session_view, tr("Session")).clicked() {
                app.dashboard_session_view = true;
            }
            if ui.selectable_label(!app.dashboard_session_view, tr("Summary")).clicked() {
                app.dashboard_session_view = false;
            }
        });
    });
    ui.add_space(9.0);

    egui::ScrollArea::vertical()
        .id_salt("instances_page")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let profiles: Vec<usize> = GameProfile::ALL.iter()
                .map(|profile| profile.index())
                .filter(|index| app.accounts[*index].enabled)
                .collect();

            if profiles.is_empty() && app.selected_details.is_none() {
                ui.label(RichText::new(tr(
                    "No accounts enabled. Open Account Manager to add or enable one."
                )).size(12.0).color(DIM));
            } else {
                // The dashboard matches the React manager: aggregate metrics
                // above a responsive grid of account cards. Details replace
                // the grid rather than leaving an empty two-column workspace.
                if app.selected_details.is_none() {
                    let totals = collect_overview_totals(app, &profiles);
                    draw_overview_totals(ui, &totals);
                    ui.add_space(12.0);
                    let gap = 12.0;
                    let available = ui.available_width();
                    let columns = if available >= 800.0 { 2 } else { 1 };
                    let card_width = if columns == 2 {
                        (available - gap) / 2.0
                    } else { available };
                    for chunk in profiles.chunks(columns) {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for index in chunk {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(card_width, 0.0),
                                    Layout::top_down(Align::Min),
                                    |ui| draw_overview_card(app, ui, *index, card_width),
                                );
                            }
                        });
                        ui.add_space(10.0);
                    }
                }

                if let Some(index) = app.selected_details {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(format!(
                            "{} {}", tr("Details for"),
                            app.account_name(app.games[index].profile)
                        )).size(15.0).strong().color(TEXT));
                        ui.add_space(8.0);
                        if ui.selectable_label(!app.dashboard_show_history, tr("Live details")).clicked() {
                            app.dashboard_show_history = false;
                        }
                        if ui.selectable_label(app.dashboard_show_history, tr("History")).clicked() {
                            app.dashboard_show_history = true;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button(tr("Hide details")).clicked() {
                                app.selected_details = None;
                            }
                        });
                    });
                    ui.add_space(8.0);
                    if app.selected_details == Some(index) {
                        let width = ui.available_width().min(900.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(width, 0.0),
                            Layout::top_down(Align::Min),
                            |ui| {
                                if app.dashboard_show_history {
                                    draw_history_panel(app, ui, index);
                                } else {
                                    draw_game_details(app, ui, index, width);
                                }
                            },
                        );
                    }
                }
            }

            ui.add_space(10.0);
            ui.collapsing(tr("Technical details"), |ui| {
                draw_runtime_section(app, ui);
            });
        });
}

#[derive(Default)]
struct OverviewTotals {
    online: usize,
    xp_h: f64,
    xp_samples: usize,
    gold_h: f64,
    gold_samples: usize,
    gold: u64,
    gold_balances: usize,
    gems: u64,
    gem_balances: usize,
}

// Only confirmed live snapshots contribute. Offline accounts are not assumed
// to have zero balances; these totals explicitly describe online balances.
fn collect_overview_totals(app: &mut ControllerApp, profiles: &[usize]) -> OverviewTotals {
    let mut result = OverviewTotals::default();
    for &index in profiles {
        let health = app.games[index].health();
        let running = app.games[index].is_running();
        if !running || !health.logged_in || !health.bridge_snapshot_ok ||
            (app.games[index].native.is_none() && !health.telemetry_is_fresh()) {
            continue;
        }
        result.online += 1;
        if let Some(native) = app.games[index].native.as_ref() {
            let view = native.hunt_snapshot();
            let snapshot = native.snapshot();
            if view.measurements.valid_trainer {
                result.xp_h += view.measurements.trainer_xp_h;
                result.xp_samples += 1;
            }
            if view.measurements.valid_gold {
                result.gold_h += view.measurements.gold_net_h;
                result.gold_samples += 1;
            }
            if let Some(value) = view.gold.or(snapshot.gold) {
                result.gold = result.gold.saturating_add(value);
                result.gold_balances += 1;
            }
            if let Some(value) = view.orbs.or(snapshot.orbs) {
                result.gems = result.gems.saturating_add(value);
                result.gem_balances += 1;
            }
        } else {
            if let Some(rate) = app.xp_rates[index].observe(&health) {
                result.xp_h += rate;
                result.xp_samples += 1;
            }
            // Gold/h is not provided as an observed browser rate; do not
            // silently include Hunt Atlas estimates in a measured total.
            result.gold = result.gold.saturating_add(health.gold);
            result.gems = result.gems.saturating_add(health.orbs);
            result.gold_balances += 1;
            result.gem_balances += 1;
        }
    }
    result
}

fn draw_overview_totals(ui: &mut egui::Ui, totals: &OverviewTotals) {
    let entries = [
        ("✧", "XP/h total", if totals.xp_samples > 0 {
            format_hunt_amount(totals.xp_h)
        } else { "—".into() }, ACCENT),
        ("◈", "Gold/h total", if totals.gold_samples > 0 {
            format_hunt_amount(totals.gold_h)
        } else { "—".into() }, WARN),
        ("●", "Online accounts", format!("{}/4", totals.online), GOOD),
        ("◆", "Live gold", if totals.gold_balances > 0 {
            format_number(totals.gold)
        } else { "—".into() }, WARN),
        ("◇", "Live gems", if totals.gem_balances > 0 {
            format_number(totals.gems)
        } else { "—".into() }, ACCENT),
    ];
    let gap = 8.0;
    let available = ui.available_width();
    let columns = if available >= 900.0 { 5 }
        else if available >= 550.0 { 3 } else { 2 };
    let tile_width = ((available - gap * (columns - 1) as f32) / columns as f32).max(90.0);
    for chunk in entries.chunks(columns) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (symbol, label, value, color) in chunk {
                ui.allocate_ui_with_layout(egui::vec2(tile_width, 0.0),
                    Layout::top_down(Align::Min), |ui| {
                        egui::Frame::new().fill(PANEL)
                            .stroke(Stroke::new(1.0, BORDER))
                            .corner_radius(9.0).inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.set_width((tile_width - 20.0).max(70.0));
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(*symbol).size(15.0).color(*color));
                                    ui.add(egui::Label::new(
                                        RichText::new(tr(label)).size(10.0).color(MUTED)
                                    ).truncate());
                                });
                                ui.add(egui::Label::new(
                                    RichText::new(value).size(17.0).strong().color(TEXT)
                                ).truncate());
                            });
                    });
            }
        });
        ui.add_space(7.0);
    }
    ui.label(RichText::new(tr(
        "Verified online balances only; unavailable accounts are not counted."
    )).size(9.0).color(DIM))
        .on_hover_text(tr("Rates are measured, never projected; unavailable data stays blank."));
}

/// Current game inventory (server quantities) and separately battle-acquired
/// drops. Shop purchases are never labeled as loot.
fn request_item_catalog(app:&mut ControllerApp) {
    if app.inventory_catalog.is_none() && app.inventory_catalog_error.is_none() &&
        !app.inventory_catalog_busy {
        app.inventory_catalog_busy=true;
        let tx=app.inventory_catalog_tx.clone();
        std::thread::spawn(move || {
            let result=std::panic::catch_unwind(native_items::load)
                .unwrap_or_else(|_|Err("Item catalog loader panicked".into()));
            let _=tx.send(result);
        });
    }
}
/// Offline pages should offer one useful next action instead of leaving an
/// almost entirely empty workspace. Authentication is never inferred complete.
fn draw_native_missing_panel(
    app:&mut ControllerApp,
    ui:&mut egui::Ui,
    selected:Option<usize>,
    purpose:&str,
) {
    egui::Frame::new().fill(PANEL_ALT).corner_radius(8.0)
        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
            ui.label(RichText::new(purpose).size(13.0).strong().color(TEXT));
            if let Some(index)=selected {
                let profile=app.games[index].profile;
                let running=app.games[index].is_running();
                let name=app.account_name(profile);
                ui.label(RichText::new(format!("{} · {}",name,
                    if running { tr("Waiting for sign-in") }
                    else { tr("Account is stopped") }))
                    .size(11.0).color(if running { WARN } else { DIM }));
                ui.horizontal_wrapped(|ui|{
                    if ui.add_enabled(!running,egui::Button::new(tr("Start account")))
                        .clicked() {
                        app.launch_one(profile);
                    }
                    if ui.small_button(tr("Account settings")).clicked() {
                        app.panel_tab=PanelTab::Accounts;
                    }
                });
            } else {
                ui.label(RichText::new(tr("Choose an account to get started"))
                    .size(11.0).color(DIM));
                ui.horizontal_wrapped(|ui|{
                    if ui.small_button(tr("Launch enabled accounts")).clicked() {
                        app.launch_enabled();
                    }
                    if ui.small_button(tr("Manage accounts")).clicked() {
                        app.panel_tab=PanelTab::Accounts;
                    }
                });
            }
        });
}

fn format_hunt_amount(value: f64) -> String {
    if !value.is_finite() { return "—".into(); }
    let negative = if value < 0.0 { "-" } else { "" };
    let num = value.abs();
    if num >= 1_000_000_000.0 { format!("{negative}{:.2}B", num / 1_000_000_000.0) }
    else if num >= 1_000_000.0 { format!("{negative}{:.2}M", num / 1_000_000.0) }
    else if num >= 1_000.0 { format!("{negative}{:.1}K", num / 1_000.0) }
    else { format!("{negative}{num:.0}") }
}

// Hunt Atlas lives in a dedicated child module, sharing the native state and
// rendering primitives without pulling browser JavaScript into the controller.
/// At-a-glance card: keep the operational summary visible for every enabled
/// account, independently of the much longer optional diagnostics panel.
fn draw_overview_card(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize, width: f32) {
    ui.set_width(width);
    let profile = app.games[index].profile;
    let health = app.games[index].health();
    let running = app.games[index].is_running();
    let native_view = app.games[index].native.as_ref().map(|native| native.hunt_snapshot());
    let account_name = native_view.as_ref()
        .and_then(|view| view.account_nick.as_deref())
        .filter(|name| !name.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| app.account_name(profile));
    let xp_per_hour = if let Some(view) = native_view.as_ref() {
        if view.measurements.valid_trainer { Some(view.measurements.trainer_xp_h) } else { None }
    } else if running {
        app.xp_rates[index].observe(&health)
    } else {
        app.xp_rates[index] = XpRateTracker::default();
        None
    };
    // Bridge inactivity checks require Firefox telemetry; native battle
    // receipts are assessed independently by the Hunts panel.
    let idle_warning = if app.games[index].native.is_some() {
        app.idle_progress[index] = IdleProgressTracker::default();
        false
    } else { app.idle_warning_for(index, &health) };
    // Only authentication/setup issues belong on the dashboard.
    // Only actual account sign-in problems appear here.
    let needs_setup = !app.account_setup[index].setup_complete
        && app.games[index].native.is_none();
    // The authenticated game socket supplies the data directly. Browser
    // sampling intervals must not hide its last confirmed snapshot.
    let fresh = running && (health.telemetry_is_fresh()
        || app.games[index].native.as_ref().is_some_and(|session| {
            let state = session.snapshot();
            state.phase == "Online" && state.last_received.is_some()
        }));
    let repair_available = monitor_repair_available(
        running,
        health.last_error.as_deref(),
        app.games[index].orphaned_pid.is_some() && app.games[index].monitor.is_none(),
    );

    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, if app.selected_details == Some(index) { ACCENT } else { BORDER }))
        .corner_radius(12.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(
                    RichText::new(compact_text(&account_name, 21))
                        .size(14.0).strong().color(TEXT),
                ).truncate()).on_hover_text(account_name.as_str());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let expanded = app.selected_details == Some(index);
                    let label = tr(if expanded { "Hide details" } else { "Show details" });
                    if ui.small_button(label).clicked() {
                        app.selected_details = if expanded { None } else { Some(index) };
                        if !expanded { app.select_workspace_account(index); }
                    }
                });
            });
            ui.add_space(3.0);
            ui.horizontal_wrapped(|ui| {
                status_badge(ui, &health, running);
                ui.label(RichText::new(if app.games[index].native.is_some() {
                    tr("PokéIdle")
                } else if running { "Firefox" } else { tr("Offline") })
                    .size(10.0).color(DIM));
                if fresh {
                    if let Some(view) = native_view.as_ref() {
                        if view.bonuses.vip_active {
                            ui.label(RichText::new("VIP").size(10.0).strong().color(WARN));
                        }
                        if view.bonuses.guild_boost_active {
                            ui.label(RichText::new(tr("Guild boost")).size(10.0).color(GOOD));
                        }
                        for (label, percent, known) in [
                            ("Twitch", view.bonuses.twitch_pct, view.bonuses.twitch_known),
                            ("Guild", view.bonuses.guild_pct, view.bonuses.guild_known),
                            ("Event", view.bonuses.trainer_event_pct, view.bonuses.trainer_event_known),
                        ] {
                            if known && percent > 0.0 {
                                ui.label(RichText::new(format!("{label} +{percent:.0}%"))
                                    .size(10.0).color(GOOD));
                            }
                        }
                    }
                }
            });
            ui.add_space(8.0);

            // Preserve the last confirmed account snapshot through a temporary
            // transport outage. Never portray it as fresh live activity.
            if !fresh && native_view.is_some() && health.last_successful_probe_at.is_some() {
                ui.label(RichText::new(tr("Last confirmed data; reconnecting"))
                    .size(10.0).color(WARN));
            }
            draw_overview_matchup(app, ui, &health, fresh, native_view.as_ref());
            ui.add_space(6.0);
            if app.dashboard_session_view {
                draw_overview_session(ui, native_view.as_ref());
            } else {
                draw_overview_account_summary(app, ui, index, fresh,
                    xp_per_hour, native_view.as_ref());
            }
            draw_overview_restock(app, ui, index);
            ui.add_space(7.0);
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                let hunt = if !running { "—" } else if health.hunt.is_empty() {
                    match health.activity.as_str() {
                        "Center" => tr("Online · Center"),
                        "Login" => tr("Waiting for login"),
                        "Hunting" => tr("Hunting"),
                        _ => "—",
                    }
                } else { &health.hunt };
                ui.label(RichText::new(format!("⌖ {}: {}", tr("Current hunt"),
                    compact_text(hunt, 36))).size(10.0).color(MUTED))
                    .on_hover_text(hunt);
                // Native fields are Option-valued; an unreceived balance
                // must not masquerade as a legitimate zero.
                let verified = fresh && health.bridge_snapshot_ok;
                let balances = app.games[index].native.as_ref()
                    .map(|native| native.snapshot());
                let cached_native = running && native_view.is_some()
                    && balances.as_ref().is_some_and(|s| s.last_received.is_some());
                let gold = if let Some(view) = native_view.as_ref().filter(|_| verified || cached_native) {
                    view.gold.or_else(|| balances.as_ref().and_then(|s| s.gold))
                } else if verified { Some(health.gold) } else { None };
                let gems = if let Some(view) = native_view.as_ref().filter(|_| verified || cached_native) {
                    view.orbs.or_else(|| balances.as_ref().and_then(|s| s.orbs))
                } else if verified { Some(health.orbs) } else { None };
                let suffix = if cached_native && !verified {
                    " · último dado"
                } else { "" };
                ui.label(RichText::new(format!("◆ {}: {}{}",
                    tr("Gold"), gold.map(format_number).unwrap_or_else(|| "—".into()), suffix))
                    .size(10.0).color(WARN));
                ui.label(RichText::new(format!("◇ {}: {}{}",
                    tr("Gems"), gems.map(format_number).unwrap_or_else(|| "—".into()), suffix))
                    .size(10.0).color(ACCENT));
            });

            ui.add_space(9.0);
            ui.horizontal_wrapped(|ui| {
                // Overview reports the saved Auto Catch switch, NOT temporary
                // throw readiness, inventory state or receipt timing. Those
                // operational diagnostics belong exclusively to Automation.
                let catch_enabled = app.games[index].native.as_ref()
                    .map(|native|native.capture_settings().enabled)
                    .unwrap_or(health.autocatch_on);
                automation_badge(
                    ui, "Auto Catch",
                    tr(auto_catch_setting_label(catch_enabled)),
                    catch_enabled,
                );
                let bonus_confirmed = fresh && health.bridge_snapshot_ok
                    && health.stream_bonus_pct > 0.0 && !health.stream_watching.is_empty();
                let bonus_text = if bonus_confirmed {
                    format!("+{}% XP", format_pct(health.stream_bonus_pct))
                } else {
                    tr("Not confirmed").to_string()
                };
                automation_badge(ui, "XP bonus", &bonus_text, bonus_confirmed);
            });

            if needs_setup {
                ui.add_space(7.0);
                if ui.small_button(RichText::new(tr("Setup needed")).color(WARN)).clicked() {
                    app.select_workspace_account(index);
                    app.panel_tab = PanelTab::Accounts;
                    app.show_setup_guide[index] = true;
                }
            }
            if idle_warning {
                ui.add_space(5.0);
                ui.label(
                    RichText::new(tr("Idle warning: no EXP or Pokéballs used in 5 minutes"))
                        .size(10.0).color(WARN),
                );
            }
            if running && health.game_ready && health.logged_in
                && health.activity == "Hunting" && !health.bridge_connected {
                ui.label(
                    RichText::new(tr("Game progress unavailable"))
                        .size(9.0).color(DIM),
                );
            }
            if repair_available {
                ui.add_space(6.0);
                if ui.small_button(
                    RichText::new(tr("Restart Firefox to reconnect")).color(WARN)
                )
                .on_hover_text(tr("Restarts Firefox for this account. The game will disconnect briefly."))
                .clicked() {
                    app.repair_monitor(profile);
                }
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                // Native and Firefox are mutually exclusive launch options.
                let action = tr(if !running { "Start account" }
                    else if app.games[index].native.is_some() { "Stop" }
                    else if app.games[index].browser_only { "Stop Firefox" }
                    else { "Stop login" });
                let (fill, stroke) = if running {
                    (BAD.linear_multiply(0.18), Stroke::new(1.0, BAD.linear_multiply(0.55)))
                } else {
                    (ACCENT.linear_multiply(0.8), Stroke::new(1.0, ACCENT))
                };
                if ui.add(
                    egui::Button::new(
                        RichText::new(action.to_uppercase())
                            .size(12.0)
                            .strong()
                            .color(TEXT),
                    )
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(8.0),
                ).clicked() {
                    if running {
                        app.stop_one(profile);
                    } else {
                        app.launch_one(profile);
                    }
                }
                if ui.add(
                    egui::Button::new(
                        RichText::new(tr("Open in Firefox").to_uppercase())
                            .size(10.0).strong().color(TEXT),
                    )
                    .fill(PANEL_ALT)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(8.0),
                ).on_hover_text(tr("Play this account in a Firefox window instead of the background connection."))
                .clicked() {
                    app.open_in_firefox(profile);
                }
                // Keep diagnostics in the account details; show only a compact status.
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(native) = app.games[index].native.as_ref() {
                        let game = native.snapshot();
                        let twitch = native.twitch_snapshot();
                        let online = game.phase == "Online" && twitch.phase == "Online";
                        let summary = if online {
                            format!("Twitch: {}/{} channels", twitch.joined, twitch.desired)
                        } else {
                            format!("Twitch: {}", tr(&twitch.phase))
                        };
                        ui.add(egui::Label::new(
                            RichText::new(summary).size(10.0)
                                .color(if online { GOOD } else { WARN })
                        ).truncate()).on_hover_text(format!(
                            "PokéIdle: {} · {} updates · {} battles · {} captures\nTwitch: {} · {}/{} channels",
                            game.phase, game.frames, game.battle_kills, game.captures,
                            twitch.phase, twitch.joined, twitch.desired
                        ));
                    } else if running {
                        let stage = if app.games[index].browser_only {
                            tr("Firefox running")
                        } else if app.native_verifying[index] {
                            tr("Connecting to PokéIdle")
                        } else if app.native_handoff_busy[index] {
                            tr("Checking sign-in")
                        } else {
                            tr("Firefox login")
                        };
                        ui.add(egui::Label::new(
                            RichText::new(stage).size(10.0).color(WARN)
                        ).truncate()).on_hover_text(tr(if app.games[index].browser_only {
                            "Firefox remains open until you stop this account."
                        } else {
                            "Firefox closes after sign-in. PokéIdle and Twitch keep running in the app."
                        }));
                    }
                });
            });
            if app.games[index].native.is_none() {
                if let Some(error) = app.native_error[index].as_deref() {
                    ui.add_space(5.0);
                    ui.add(egui::Label::new(
                        RichText::new(format!("{}: {}", tr("Connection error"), compact_text(error, 60)))
                            .size(10.0).color(BAD)
                    ).truncate()).on_hover_text(error);
                }
            }
        });
}

fn overview_stat_tile(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    egui::Frame::new().fill(PANEL_ALT).corner_radius(7.0)
        .inner_margin(Margin::symmetric(9, 6)).show(ui, |ui| {
            ui.label(RichText::new(tr(label)).size(9.0).color(DIM));
            ui.add(egui::Label::new(
                RichText::new(value).size(14.0).strong().color(color)
            ).truncate()).on_hover_text(value);
        });
}


// Dashboard stock always refers to the selected restock item, never the sum
// of every Poké Ball, potion or revive. Inventory snapshots omit zero-stock
// entries; a missing selected ID means zero only when that inventory category
// has actually been received. An empty map remains unknown.
fn selected_restock_stock(inventory: &std::collections::BTreeMap<u64, u64>,
                          selected_id: u64) -> Option<u64> {
    if selected_id == 0 || inventory.is_empty() {
        None
    } else {
        Some(inventory.get(&selected_id).copied().unwrap_or(0))
    }
}

#[cfg(test)]
mod selected_restock_stock_tests {
    use super::selected_restock_stock;
    use std::collections::BTreeMap;

    #[test]
    fn counts_only_the_selected_ball_or_healing_item() {
        let balls = BTreeMap::from([(1, 125), (2, 73), (4, 8), (5, 1)]);
        assert_eq!(selected_restock_stock(&balls, 1), Some(125));
        assert_eq!(selected_restock_stock(&balls, 4), Some(8));
        assert_eq!(selected_restock_stock(&balls, 3), Some(0));

        let items = BTreeMap::from([
            (200, 14), (201, 9), (204, 6), (70070, 2),
            (205, 11), (206, 3), (999, 500),
        ]);
        assert_eq!(selected_restock_stock(&items, 200), Some(14));
        assert_eq!(selected_restock_stock(&items, 70070), Some(2));
        assert_eq!(selected_restock_stock(&items, 205), Some(11));
        assert_eq!(selected_restock_stock(&items, 206), Some(3));
    }

    #[test]
    fn unselected_and_unavailable_stock_is_not_reported_as_zero() {
        let available = BTreeMap::from([(200, 20), (205, 4)]);
        assert_eq!(selected_restock_stock(&available, 0), None);
        assert_eq!(selected_restock_stock(&BTreeMap::new(), 200), None);
        assert_eq!(selected_restock_stock(&BTreeMap::new(), 0), None);
    }
}

fn overview_resource_tile(app: &mut ControllerApp, ui: &mut egui::Ui,
                          index: usize, label: &str, count: Option<u64>,
                          kind: SpriteKind, selected: u64) {
    let expanded = app.restock_open[index];
    let response = egui::Frame::new().fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, if expanded { ACCENT } else { BORDER }))
        .corner_radius(7.0).inner_margin(Margin::symmetric(6, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr(label)).size(9.0).color(DIM));
                    ui.label(RichText::new(count.map(format_number)
                        .unwrap_or_else(|| "—".into()))
                        .size(12.0).strong().color(TEXT));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !app.visuals.show(ui, kind, selected, None, 23.0) {
                        ui.label(RichText::new(if selected == 0 { "×" } else { "◆" })
                            .size(15.0).color(ACCENT));
                    }
                });
            });
        }).response.interact(egui::Sense::click());
    if response.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(format!("{} · {} · {}", tr(label),
            tr(if selected == 0 { "No restock item selected" } else { "Selected item only" }),
            tr("Configure restock")))
        .clicked() {
        app.restock_open[index] = !expanded;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OverviewCombatState { Offline, Center, Active, Hunting, Waiting, Stalled }

// A wild encounter is short-lived; the account's confirmed hunt is not.
// Never switch the overview to "between encounters" or clear its hunt artwork
// just because the battle server briefly sends selvagem:null.
fn overview_combat_state(view: Option<&HuntView>, live: bool) -> OverviewCombatState {
    if !live { return OverviewCombatState::Offline; }
    let Some(view) = view else { return OverviewCombatState::Waiting; };
    if view.in_center { return OverviewCombatState::Center; }
    if view.hunt_slug.is_none() { return OverviewCombatState::Waiting; }
    // An encounter may be absent immediately after a kill. Recent combat
    // receipts still confirm that the selected hunt is producing activity.
    if view.measurements.combat_event_age_secs.is_some_and(|age| age <= 180) {
        return OverviewCombatState::Active;
    }
    if view.measurements.combat_event_age_secs.is_some_and(|age| age > 180) {
        return OverviewCombatState::Stalled;
    }
    OverviewCombatState::Hunting
}

// Use the hunt catalog, not the ephemeral wild slot, for a stable right card.
// Choosing the highest-weight species is representative artwork only; the
// card names the hunt rather than asserting this is the exact current enemy.
fn overview_hunt(view: &HuntView) -> Option<&native_hunts::HuntEntry> {
    let slug = view.hunt_slug.as_deref()?;
    view.hunts.iter().find(|entry| entry.slug == slug)
}
fn overview_hunt_art_species(view: &HuntView) -> Option<u64> {
    // Artwork follows the hunt's own species ID, never the current wild
    // encounter. A live HuntView retains the selected ID across kills.
    view.hunt_art_species.or_else(|| {
        overview_hunt(view).and_then(native_hunts::representative_hunt_species)
    })
}

fn overview_hunt_art_looktype(view: &HuntView) -> Option<u64> {
    overview_hunt(view).and_then(|hunt| hunt.looktype).filter(|id| *id > 0)
}

// Looktypes are the game's visual IDs; species IDs require a separate
// catalog lookup. Never treat an arbitrary species ID as a looktype.
// 50% larger than the previous 48px overview sprites on both sides.
const OVERVIEW_POKEMON_SPRITE_SIZE: f32 = 72.0;

fn overview_combat_image(
    visuals: &mut VisualCache,
    ui: &mut egui::Ui,
    preferred_looktype: Option<u64>,
    regular_looktype: Option<u64>,
    species_id: Option<u64>,
    opponent: bool,
) {
    let texture = [
        (SpriteKind::PokemonLooktype, preferred_looktype),
        (SpriteKind::PokemonLooktype, regular_looktype),
        (SpriteKind::Pokemon, species_id),
    ].into_iter().find_map(|(kind, id)| {
        id.filter(|id| *id > 0)
            .and_then(|id| visuals.pokemon_texture_id(ui, kind, id))
    });
    egui::Frame::new()
        .fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            if let Some(texture) = texture {
                ui.image((texture, egui::vec2(OVERVIEW_POKEMON_SPRITE_SIZE, OVERVIEW_POKEMON_SPRITE_SIZE)));
            } else {
                ui.add_sized([OVERVIEW_POKEMON_SPRITE_SIZE; 2], egui::Label::new(
                    RichText::new(if opponent { "◇" } else { "◈" })
                        .size(26.0).color(if opponent { WARN } else { ACCENT })));
            }
        });
}

/// Matchup is always visible, even when the card's lower metrics show Session.
fn draw_overview_matchup(
    app: &mut ControllerApp,
    ui: &mut egui::Ui,
    health: &Health,
    fresh: bool,
    view: Option<&HuntView>,
) {
    // Preserve the last confirmed identity and artwork during a reconnect.
    // Connectivity and combat activity below still require fresh telemetry.
    let pokemon = view.and_then(|v|v.active_pokemon.as_ref());
    let hunt = view.filter(|v| !v.in_center).and_then(overview_hunt);
    let hunt_species = view.filter(|v| !v.in_center)
        .and_then(overview_hunt_art_species);
    let hunt_looktype = view.filter(|v| !v.in_center)
        .and_then(overview_hunt_art_looktype);
    let pokemon_name = pokemon.map(|p|p.name.as_str())
        .filter(|name|!name.is_empty())
        .or_else(|| (fresh && !health.active_pokemon.is_empty())
            .then_some(health.active_pokemon.as_str())).unwrap_or("—");
    let pokemon_level = pokemon.map(|p|p.level.to_string())
        .filter(|level|level!="0")
        .unwrap_or_else(|| if fresh && !health.pokemon_level.is_empty() {
            health.pokemon_level.clone()
        } else { "—".into() });
    let combat = overview_combat_state(view, fresh);
    ui.columns(2, |cols| {
        cols[0].label(RichText::new(tr("Your Pokémon"))
            .size(9.0).color(DIM));
        cols[0].horizontal(|ui| {
            overview_combat_image(&mut app.visuals, ui,
                pokemon.and_then(|p| if p.shiny { p.shiny_looktype } else { p.looktype }),
                pokemon.and_then(|p| p.looktype),
                pokemon.and_then(|p| p.species_id), false);
            ui.vertical(|ui| {
                ui.add(egui::Label::new(RichText::new(
                    compact_text(pokemon_name, 16)).size(11.0)
                    .strong().color(TEXT)).truncate()).on_hover_text(pokemon_name);
                ui.label(RichText::new(format!("Lv. {pokemon_level}"))
                    .size(9.0).color(DIM));
            });
        });
        // The right half tracks the selected hunt, not the transient opponent
        // slot. The artwork represents a species encountered in that hunt.
        cols[1].with_layout(Layout::top_down(Align::Max), |ui| {
            ui.label(RichText::new(tr("Current hunt"))
                .size(9.0).color(DIM));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                overview_combat_image(&mut app.visuals, ui,
                    None, hunt_looktype, hunt_species, true);
                ui.vertical(|ui| {
                    let name = if combat == OverviewCombatState::Center {
                        tr("At Pokémon Center")
                    } else {
                        hunt.map(|h| h.name.as_str())
                            .filter(|s| !s.is_empty())
                            .or_else(|| (fresh && !health.hunt.is_empty())
                                .then_some(health.hunt.as_str()))
                            .unwrap_or("—")
                    };
                    ui.add(egui::Label::new(RichText::new(compact_text(name, 21))
                        .size(11.0).strong().color(TEXT))
                        .truncate()).on_hover_text(name);
                    let level = hunt.filter(|h| h.level > 0)
                        .map(|h| format!("Lv. {}", h.level))
                        .unwrap_or_else(|| "—".into());
                    ui.label(RichText::new(level).size(9.0).color(DIM));
                });
            });
        });
    });
    let (state_label, color)=match combat {
        OverviewCombatState::Active=>("Hunt active",GOOD),
        OverviewCombatState::Hunting=>(tr("Hunting"),GOOD),
        OverviewCombatState::Center=>(tr("At Pokémon Center"),WARN),
        OverviewCombatState::Waiting=>(tr("Waiting for combat"),WARN),
        OverviewCombatState::Stalled=>("In hunt · no recent activity",WARN),
        OverviewCombatState::Offline=>(tr("No live connection"),DIM),
    };
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("● {state_label}"))
            .size(11.0).strong().color(color));
        if let Some(age)=view.filter(|_|fresh)
            .and_then(|v|v.measurements.combat_event_age_secs) {
            ui.label(RichText::new(format!("{}: {age}s",
                tr("Last combat activity"))).size(9.0).color(DIM));
        }
    });
    ui.add_space(5.0);
}

fn draw_overview_account_summary(
    app: &mut ControllerApp,
    ui: &mut egui::Ui,
    index: usize,
    fresh: bool,
    xp_per_hour: Option<f64>,
    view: Option<&HuntView>,
) {
    // Cached native measurements remain visible while marked as historical;
    // they must never contribute to the live aggregate totals.
    let showing_last_known = !fresh && view.is_some();
    let gold_h = view.and_then(|v|
        v.measurements.valid_gold.then_some(v.measurements.gold_net_h));
    // The restock editor stages selections while the encrypted save runs.
    // Use those staged IDs immediately so the icon and count always agree.
    let restock_selection = app.restock_drafts[index].as_ref().cloned()
        .or_else(|| app.games[index].native.as_ref().map(|n| n.capture_settings()))
        .unwrap_or_default();
    let balls = view.and_then(|v|
        selected_restock_stock(v.balls.as_ref(), restock_selection.ball_id));
    let potions = view.and_then(|v|
        selected_restock_stock(v.items.as_ref(), restock_selection.potion_id));
    let revives = view.and_then(|v|
        selected_restock_stock(v.items.as_ref(), restock_selection.revive_id));
    ui.columns(2, |cols| {
        overview_stat_tile(&mut cols[0],"XP/h",
            &if fresh || showing_last_known {xp_per_hour.map(format_hunt_amount)
                .unwrap_or_else(||"—".into())}else{"—".into()},
            if showing_last_known { DIM } else { ACCENT });
        overview_stat_tile(&mut cols[1],"Gold/h",
            &gold_h.map(format_hunt_amount)
                .unwrap_or_else(||"—".into()),WARN);
    });
    ui.add_space(4.0);
    ui.columns(3, |cols| {
        overview_resource_tile(app,&mut cols[0],index,"Potions",
            potions,SpriteKind::Item,restock_selection.potion_id);
        overview_resource_tile(app,&mut cols[1],index,"Balls",
            balls,SpriteKind::Ball,restock_selection.ball_id);
        overview_resource_tile(app,&mut cols[2],index,"Revives",
            revives,SpriteKind::Item,restock_selection.revive_id);
    });
}

#[cfg(test)]
mod overview_combat_tests {
    use super::{overview_combat_state, overview_hunt_art_species, OverviewCombatState};
    use crate::native_hunts::{HuntEntry, HuntSpecies, HuntView, WildInfo};
    use std::sync::Arc;

    #[test]
    fn confirmed_hunt_stays_visible_across_encounters() {
        let mut view = HuntView { hunt_slug: Some("scyther-field".into()),
            ..Default::default() };
        assert_eq!(overview_combat_state(Some(&view), true), OverviewCombatState::Hunting);
        view.wild = Some(WildInfo { name: "Scyther".into(), ..Default::default() });
        view.measurements.combat_event_age_secs = Some(10);
        assert_eq!(overview_combat_state(Some(&view), true), OverviewCombatState::Active);
        view.wild = None; // selvagem:null between fights
        assert_eq!(overview_combat_state(Some(&view), true), OverviewCombatState::Active);
        view.measurements.combat_event_age_secs = Some(181);
        assert_eq!(overview_combat_state(Some(&view), true), OverviewCombatState::Stalled);
        assert_eq!(overview_combat_state(Some(&view), false), OverviewCombatState::Offline);
        view.in_center = true;
        assert_eq!(overview_combat_state(Some(&view), true), OverviewCombatState::Center);
    }

    #[test]
    fn overview_art_is_the_hunt_species_not_the_current_wild() {
        let mut view = HuntView { hunt_slug: Some("field-a".into()),
            hunts: Arc::new(vec![
                HuntEntry { slug: "field-a".into(), species: vec![
                    HuntSpecies { id: 2006, weight: 1.0 },
                ], ..Default::default() },
                HuntEntry { slug: "field-b".into(), species: vec![
                    HuntSpecies { id: 14009, weight: 1.0 }
                ], ..Default::default() },
            ]), ..Default::default() };
        assert_eq!(overview_hunt_art_species(&view), Some(2006));
        view.wild = Some(WildInfo { species_id: Some(127), ..Default::default() });
        assert_eq!(overview_hunt_art_species(&view), Some(2006));
        view.wild = None;
        assert_eq!(overview_hunt_art_species(&view), Some(2006));
        view.hunt_slug = Some("field-b".into());
        assert_eq!(overview_hunt_art_species(&view), Some(14009));
        view.hunt_art_species = Some(14009);
        assert_eq!(overview_hunt_art_species(&view), Some(14009));
    }
}

fn rare_session_drop_count(drops: &std::collections::BTreeMap<String, u64>,
                           matches: &[&str]) -> u64 {
    drops.iter()
        .filter(|(name, _)| {
            let name = name.to_lowercase();
            matches.iter().any(|term| name.contains(term))
        })
        .map(|(_, quantity)| *quantity)
        .fold(0u64, u64::saturating_add)
}

fn draw_overview_session(ui: &mut egui::Ui, view: Option<&HuntView>) {
    let Some(view) = view else {
        ui.label(RichText::new(tr("Start an account to see session details"))
            .size(11.0).color(MUTED));
        return;
    };
    let session = &view.session;
    ui.columns(3, |cols| {
        overview_stat_tile(&mut cols[0], "Session duration",
            &format!("{:02}:{:02}:{:02}",
                session.duration_secs / 3600,
                (session.duration_secs / 60) % 60,
                session.duration_secs % 60), TEXT);
        overview_stat_tile(&mut cols[1], "Kills", &format_number(session.kills), TEXT);
        overview_stat_tile(&mut cols[2], "Captures", &format_number(session.captures), TEXT);
    });
    ui.add_space(6.0);
    ui.label(RichText::new(tr("Rare drops")).size(9.0).strong().color(DIM));
    let drops: [(&str, &[&str]); 4] = [
        ("Boss Token", &["boss token"]),
        ("Key", &["chave", "key"]),
        ("Shiny Stone", &["shiny stone"]),
        ("Bicycle", &["bicicleta", "bicycle"]),
    ];
    ui.columns(4, |cols| {
        for (col, (label, matches)) in cols.iter_mut().zip(drops.iter()) {
            overview_stat_tile(col, label,
                &format_number(rare_session_drop_count(&session.acquired, matches)), TEXT);
        }
    });
    ui.add_space(6.0);
    ui.columns(2, |cols| {
        overview_stat_tile(&mut cols[0], "shinies seen",
            &format_number(session.shiny_seen), ACCENT);
        overview_stat_tile(&mut cols[1], "shinies caught",
            &format_number(session.shiny_captured), GOOD);
    });
}

#[cfg(test)]
mod dashboard_overview_tests {
    use super::rare_session_drop_count;
    use std::collections::BTreeMap;

    #[test]
    fn session_drop_tiles_count_only_matching_battle_drops() {
        let drops = BTreeMap::from([
            ("Boss Token".to_string(), 2),
            ("Boss Token Fragment".to_string(), 3),
            ("Great Ball".to_string(), 100),
            ("Chave Dourada".to_string(), 1),
        ]);
        assert_eq!(rare_session_drop_count(&drops, &["boss token"]), 5);
        assert_eq!(rare_session_drop_count(&drops, &["chave", "key"]), 1);
        assert_eq!(rare_session_drop_count(&drops, &["bicycle"]), 0);
    }
}

/// Aggregated, 90-day battle history is loaded from the same account's
/// local disk file and updated by the native game worker. Never reconstruct
/// historical XP by adding Pokémon and trainer rewards together.
fn draw_history_panel(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize) {
    ui.label(RichText::new(tr("Battle history")).size(14.0)
        .strong().color(TEXT));
    let profile=app.games[index].profile;
    let history=if let Some(native)=app.games[index].native.as_ref() {
        app.offline_histories[index]=None;
        native.history_snapshot()
    } else {
        if ui.small_button(tr("Reload saved history")).clicked() {
            app.offline_histories[index]=None;
        }
        app.offline_histories[index]
            .get_or_insert_with(||crate::native_history::load_offline(profile)).clone()
    };
    if !history.owner.is_empty() {
        ui.label(RichText::new(format!("{}: {}",tr("Account"),history.owner))
            .size(11.0).color(MUTED));
    }
    let totals=history.totals();
    ui.add_space(8.0);
    ui.label(RichText::new(tr(
        "Up to 90 days of battle history, saved locally for each account."
    )).size(10.0).color(DIM));
    if let Some(error)=history.load_error.as_deref(){
        ui.colored_label(WARN,tr_dynamic(format!("History load: {error}")));
    }
    if let Some(error)=history.save_error.as_deref(){
        ui.colored_label(WARN,tr_dynamic(format!("History save: {error}")));
    }
    ui.add_space(9.0);
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
        ui.label(RichText::new(tr("Lifetime totals")).size(13.0).strong().color(TEXT));
        ui.columns(3,|cols|{
            detail_stat_line(&mut cols[0],tr("kills"),&format_number(totals.kills));
            detail_stat_line(&mut cols[0],tr("captures"),&format_number(totals.captures));
            detail_stat_line(&mut cols[1],tr("Trainer XP earned"),&format_number(totals.trainer_xp));
            detail_stat_line(&mut cols[1],tr("Pokémon XP earned"),&format_number(totals.pokemon_xp));
            detail_stat_line(&mut cols[2],tr("Combat gold"),&format_number(totals.combat_gold));
            detail_stat_line(&mut cols[2],tr("Auto-sale gold"),&format_number(totals.auto_sale_gold));
        });
        ui.label(format!("{}: {} · {}: {}",tr("shinies seen"),totals.shiny_seen,
            tr("shinies caught"),totals.shiny_captured));
    });
    // Only show days containing activity. Rendering fourteen empty progress
    // bars made history look like an enormous blank table for new accounts.
    if let Some(last_day)=history.days.iter().map(|entry|entry.day).max() {
        let mut points=[(0u64,0u64,0u64);14];
        for (index,point) in points.iter_mut().enumerate() {
            point.0=last_day.saturating_sub((13-index) as u64);
        }
        for entry in &history.days {
            let age=last_day.saturating_sub(entry.day);
            if entry.day<=last_day && age<14 {
                let point=&mut points[13-age as usize];
                point.1=point.1.saturating_add(entry.kills);
                point.2=point.2.saturating_add(entry.captures);
            }
        }
        let active=points.iter().filter(|(_,kills,captures)|
            *kills>0 || *captures>0).collect::<Vec<_>>();
        if !active.is_empty() {
            let max_kills=active.iter().map(|(_,kills,_)|*kills)
                .max().unwrap_or(0).max(1);
            ui.add_space(6.0);
            egui::Frame::new().fill(PANEL_ALT).corner_radius(8.0)
                .inner_margin(egui::Margin::same(10)).show(ui,|ui|{
                    ui.label(RichText::new(tr("Recent active days · kills / captures"))
                        .size(12.0).strong().color(TEXT));
                    for (day,kills,captures) in active.into_iter().rev().take(7) {
                        ui.horizontal(|ui|{
                            ui.add_sized([83.0,16.0],egui::Label::new(
                                RichText::new(history_day_label(*day)).size(10.0).color(DIM)));
                            ui.add(egui::ProgressBar::new(*kills as f32/max_kills as f32)
                                .desired_width(ui.available_width().clamp(100.0,350.0))
                                .text(format!("{kills} / {captures}")));
                        });
                    }
                });
        }
    }
    ui.add_space(8.0);
    let mut entries=history.days.iter().collect::<Vec<_>>();
    entries.sort_by(|a,b|b.day.cmp(&a.day).then_with(||a.hunt.cmp(&b.hunt)));
    let total_entries=entries.len();
    let displayed_entries=total_entries.min(app.history_rows_limit);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tr("Hunt activity")).size(13.0).strong().color(TEXT));
        ui.label(RichText::new(format!("{} / {} {}", displayed_entries,
            total_entries, tr("recorded hunt-days"))).size(10.0).color(DIM));
    });
    egui::Frame::new().fill(PANEL).corner_radius(8.0)
        .inner_margin(egui::Margin::same(8)).show(ui,|ui|{
            egui::ScrollArea::vertical().id_salt(("history-rows",index))
                .max_height(360.0).show(ui,|ui|{
                    if entries.is_empty() {
                        ui.label(tr("No recorded battles yet"));
                    } else {
                        egui::Grid::new(("history-grid",index))
                            .striped(true).spacing([12.0,6.0])
                            .show(ui,|ui|{
                                for heading in ["Date","Hunt","Kills","Caught",
                                    "Trainer XP","Pokémon XP","Gold","Drops"] {
                                    ui.label(RichText::new(tr(heading))
                                        .size(10.0).strong().color(MUTED));
                                }
                                ui.end_row();
                                for day in entries.into_iter().take(displayed_entries) {
                                    ui.label(RichText::new(history_day_label(day.day)).size(10.0));
                                    ui.label(RichText::new(compact_text(&day.hunt,25)).size(10.0))
                                        .on_hover_text(&day.hunt);
                                    ui.label(format_number(day.kills));
                                    ui.label(format_number(day.captures));
                                    ui.label(format_number(day.trainer_xp));
                                    ui.label(format_number(day.pokemon_xp));
                                    ui.label(format_number(day.combat_gold.saturating_add(day.auto_sale_gold)))
                                        .on_hover_text(tr_dynamic(format!("Combat: {} · Auto-sale: {}",
                                            format_number(day.combat_gold),
                                            format_number(day.auto_sale_gold))));
                                    let drop_text=if day.drops.is_empty() {
                                        "—".to_owned()
                                    } else {
                                        format!("{}",day.drops.len())
                                    };
                                    let drops=day.drops.iter()
                                        .map(|(name,qty)|format!("{name} ×{qty}"))
                                        .collect::<Vec<_>>().join("\n");
                                    ui.label(drop_text).on_hover_text(if drops.is_empty() {
                                        tr("No drops recorded").to_owned()
                                    } else {drops});
                                    ui.end_row();
                                }
                            });
                    }
                });
        });
    if displayed_entries<total_entries {
        if ui.small_button(tr("Show 100 more hunt-days")).clicked() {
            app.history_rows_limit=app.history_rows_limit.saturating_add(100);
        }
    }
    ui.add_space(7.0);
    ui.label(RichText::new(tr(
        "History begins when this version first records a battle. Previous in-memory sessions cannot be recovered retroactively."
    )).size(10.0).color(DIM));
}
fn history_day_label(day:u64)->String {
    // Civil-from-days (Gregorian UTC), no OS locale or timezone guesses.
    let z=day as i64+719_468;
    let era=if z>=0{z}else{z-146_096}/146_097;
    let doe=z-era*146_097;
    let yoe=(doe-doe/1460+doe/36_524-doe/146_096)/365;
    let mut year=yoe+era*400;
    let doy=doe-(365*yoe+yoe/4-yoe/100);
    let mp=(5*doy+2)/153;
    let d=doy-(153*mp+2)/5+1;
    let month=mp+if mp<10{3}else{-9};
    if month<=2 {year+=1;}
    format!("{year:04}-{month:02}-{d:02}")
}
#[cfg(test)]
mod history_ui_tests {
    use super::history_day_label;
    #[test]
    fn utc_epoch_date_and_leap_day_are_stable() {
        assert_eq!(history_day_label(0),"1970-01-01");
        assert_eq!(history_day_label(19782),"2024-02-29");
    }
}

/// Dedicated native automation workspace. Keeps settings out of the
/// scrollable dashboard and provides a stable per-account selection.
/// Consistent selected/unselected workspace navigation used by Automation
/// and Hunt Atlas. Tooltips explain controls without inflating the toolbar.
fn workspace_tab_button(
    ui: &mut egui::Ui, label: &str, selected: bool, hint: &str,
) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(tr(label)).size(12.0)
            .strong().color(if selected { TEXT } else { MUTED }))
            .fill(if selected { ACCENT.linear_multiply(0.20) } else { PANEL })
            .stroke(Stroke::new(1.0, if selected { ACCENT } else { BORDER }))
            .corner_radius(7.0),
    ).on_hover_text(tr(hint))
}

fn draw_automation_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    // Keep section navigation above the only vertical scroll region. Market
    // rules can be very long; they should never push Catch controls offscreen.
    ui.horizontal_wrapped(|ui| {
        for (section, label, hint) in [
            (AutomationSection::Capture, "Auto Catch", "Capture preferences and live statistics"),
            (AutomationSection::Game, "Game & restock", "Server settings and restock status"),
            (AutomationSection::Market, "Market", "Automatic buying rules and Snatch"),
        ] {
            if workspace_tab_button(ui, label,
                app.automation_section == section, hint).clicked() {
                app.automation_section = section;
            }
        }
    });
    ui.add_space(7.0);
    ui.separator();
    let index = app.automation_account;
    egui::ScrollArea::vertical()
        .id_salt(("automation-page", index, app.automation_section as u8))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            draw_automation_contents(app, ui);
        });
}

fn draw_automation_contents(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let index = app.automation_account;
    match app.automation_section {
        AutomationSection::Capture => {
            if app.games[index].native.is_some() {
                draw_native_automation(app, ui, index);
            } else {
                draw_native_missing_panel(app, ui, Some(index),
                    tr("Start this account to configure Auto Catch"));
            }
        }
        AutomationSection::Game => draw_game_automation_contents(app, ui),
        AutomationSection::Market => market_panel::draw(app, ui, index),
    }
}

fn draw_game_automation_contents(app: &mut ControllerApp, ui: &mut egui::Ui) {
    let index = app.automation_account;
    let Some(native) = app.games[index].native.as_ref() else {
        draw_native_missing_panel(app,ui,Some(index),
            tr("Start this account to manage game settings"));
        return;
    };
    let snapshot = native.snapshot();
    let view = native.hunt_snapshot();
    if let Some(automation) = view.automation.as_ref() {
        if !app.automation_drafts[index].initialized {
            app.automation_drafts[index]=AutomationDraft::from_server(automation);
        }
        let mut commands:Vec<(String,serde_json::Value)>=Vec::new();
        let can_edit=snapshot.phase=="Online" &&
            snapshot.native_automation.pending_field.is_none();
        egui::Frame::new().fill(PANEL).corner_radius(10.0)
            .inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Game automation")).size(13.0).strong().color(TEXT));
                if ui.small_button(tr("Reload game settings")).clicked() {
                    app.automation_drafts[index]=AutomationDraft::from_server(automation);
                }
                if let Some(field)=snapshot.native_automation.pending_field.as_deref() {
                    ui.label(RichText::new(format!("{}: {field}",tr("Awaiting confirmation")))
                        .size(10.0).color(WARN));
                }
                ui.label(RichText::new(format!("{}: {} · {}: {}",
                    tr("Confirmed"),snapshot.native_automation.confirmed,
                    tr("Queued"),snapshot.native_automation.queued))
                    .size(10.0).color(DIM));
            });
            if let Some(error)=snapshot.native_automation.error.as_deref(){
                ui.label(RichText::new(error).size(10.0).color(WARN));
            }
            ui.label(RichText::new(tr(
                "Changes only take effect after PokéIdle confirms them."
            )).size(10.0).color(DIM));
            ui.add_space(8.0);
            let groups: [(&str, &[(&str, &str)]); 2] = [
                ("Recovery & selling", &[
                    ("autoPotion", "Auto Potion"),
                    ("autoRevive", "Auto Revive"),
                    ("autoVendaLoot", "Auto Sell"),
                    ("autoVoltarHunt", "Return to Hunt"),
                ]),
                ("Pokémon protection", &[
                    ("autoLockShiny", "Lock Shiny"),
                    ("autoLockNota9", "Lock Grade 9"),
                    ("autoLockP5", "Lock Potency 5"),
                ]),
            ];
            for (heading, fields) in groups {
                ui.label(RichText::new(tr(heading)).size(11.0).strong().color(MUTED));
                ui.horizontal_wrapped(|ui| {
                    for (key, label) in fields {
                        if let Some(current)=automation.get(*key).and_then(serde_json::Value::as_bool) {
                            let mut enabled=current;
                            if ui.add_enabled(can_edit,
                                egui::Checkbox::new(&mut enabled,tr(label))).changed() {
                                commands.push(((*key).into(),serde_json::json!(enabled)));
                            }
                        }
                    }
                });
                ui.add_space(5.0);
            }
            ui.add_space(10.0);
            let draft=&mut app.automation_drafts[index];
            ui.horizontal_wrapped(|ui| {
                if automation.get("hpLimiar").is_some() {
                    ui.label(tr("Potion HP threshold (%)"));
                    ui.add(egui::DragValue::new(&mut draft.hp_limit)
                        .range(0.0..=100.0).speed(0.5));
                    if ui.add_enabled(can_edit,egui::Button::new(tr("Apply HP"))).clicked() {
                        commands.push(("hpLimiar".into(),serde_json::json!(draft.hp_limit)));
                    }
                }
                if automation.get("autoLockNotaMin").is_some() {
                    ui.label(tr("Minimum lock grade"));
                    ui.add(egui::DragValue::new(&mut draft.lock_grade)
                        .range(0.0..=10.0).speed(0.1));
                    if ui.add_enabled(can_edit,egui::Button::new(tr("Apply grade"))).clicked() {
                        commands.push(("autoLockNotaMin".into(),
                            serde_json::json!(draft.lock_grade)));
                    }
                }
            });
        });
        for (field,value) in commands {
            if let Err(error)=native.send_command(GameCommand::SetAutomation {field,value}) {
                app.status=format!("Server automation: {error}");
                app.status_error=true;
            }
        }
        ui.add_space(10.0);
    } else {
        ui.label(RichText::new(tr("Loading game automation settings"))
            .size(11.0).color(WARN));
    }
    let restock = &snapshot.native_restock;
    egui::Frame::new().fill(PANEL_ALT).corner_radius(10.0)
        .inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Restock details"))
                    .size(13.0).strong().color(TEXT));
                automation_badge(ui, "Restock", &restock.phase,
                    restock.enabled && restock.phase == "Stock OK");
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("{} {} · {} {}", restock.ordered, tr("ordered"),
                    restock.confirmed, tr("confirmed")));
                if restock.pending { ui.label(RichText::new(tr("Pending")).color(WARN)); }
                if let Some(order) = restock.last_order.as_deref() {
                    ui.label(RichText::new(order).size(10.0).color(MUTED));
                }
            });
            if let Some(error) = restock.last_error.as_deref() {
                ui.label(RichText::new(error).size(10.0).color(WARN));
            }
            ui.add_space(8.0);
            for rule in &restock.rules {
                ui.horizontal_wrapped(|ui| {
                    let below_target=rule.current.is_some_and(|stock|stock<rule.low);
                    ui.label(RichText::new(&rule.name).size(11.0).strong()
                        .color(if below_target{WARN}else{TEXT}))
                        .on_hover_text(tr_dynamic(format!("Item #{}",rule.id)));
                    ui.label(RichText::new(format!(
                        "{} {} / {} · {} {}",
                        rule.current.map(|n|n.to_string()).unwrap_or_else(||"—".into()),
                        tr("in stock"),rule.target,
                        tr("buy at"),rule.low
                    )).size(10.0).color(DIM));
                    let price = rule.price.map(|n|format!("{} gold",format_number(n)))
                        .unwrap_or_else(||tr("Price unavailable").into());
                    ui.label(RichText::new(format!("{} · {}",price,rule.price_source))
                        .size(10.0).color(if rule.price.is_some() { MUTED } else { WARN }));
                });
            }
            if restock.enabled && restock.rules.is_empty() {
                ui.label(RichText::new(tr("No valid restock rules configured"))
                    .size(10.0).color(WARN));
            }
        });
    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tr("Restock is configured on each Overview account card"))
            .size(11.0).color(MUTED));
        if ui.small_button(tr("Open restock on Overview")).clicked() {
            app.restock_open[index] = true;
            app.selected_details = None;
            app.panel_tab = PanelTab::Overview;
        }
    });
}

// The Overview never infers this setting from temporary capture readiness.
fn auto_catch_setting_label(enabled:bool)->&'static str {
    if enabled {"Auto Catch on"} else {"Auto Catch off"}
}

// A separate, concrete operational signal for Automation only.
// Ready means the next valid target can be thrown at, not proof of a capture.
fn auto_catch_operational_label(connected:bool,
    capture:&crate::native_gameplay::GameplayStatus)->&'static str {
    if !connected {
        "Auto Catch awaiting game connection"
    } else if capture.pending_results>0 {
        "Auto Catch waiting for capture results"
    } else if capture.capture_ready {
        "Auto Catch ready for targets"
    } else {
        "Auto Catch waiting for conditions"
    }
}

#[cfg(test)]
mod auto_catch_ui_state_tests {
    use super::{auto_catch_setting_label,auto_catch_operational_label};
    use crate::native_gameplay::GameplayStatus;

    #[test]
    fn setting_is_independent_of_temporary_capture_readiness() {
        let mut capture=GameplayStatus::default();
        assert_eq!(auto_catch_setting_label(true),"Auto Catch on");
        assert_eq!(auto_catch_setting_label(false),"Auto Catch off");
        assert_eq!(auto_catch_operational_label(false,&capture),
            "Auto Catch awaiting game connection");
        capture.capture_ready=true;
        assert_eq!(auto_catch_setting_label(true),"Auto Catch on");
        assert_eq!(auto_catch_operational_label(true,&capture),
            "Auto Catch ready for targets");
        capture.pending_results=2;
        assert_eq!(auto_catch_setting_label(true),"Auto Catch on");
        assert_eq!(auto_catch_operational_label(true,&capture),
            "Auto Catch waiting for capture results");
        capture.capture_ready=false;
        capture.pending_results=0;
        assert_eq!(auto_catch_operational_label(true,&capture),
            "Auto Catch waiting for conditions");
    }
}

fn draw_native_automation(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize) {
    let profile = app.games[index].profile;
    let Some(native) = app.games[index].native.as_ref() else { return; };
    let game = native.snapshot();
    let twitch = native.twitch_snapshot();

    egui::Frame::new()
        .fill(PANEL_ALT)
        .corner_radius(10.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                automation_badge(ui, "PokéIdle", &game.phase, game.phase == "Online");
                let twitch_status = format!("{} · {}/{} channels", tr(&twitch.phase), twitch.joined, twitch.desired);
                automation_badge(ui, "Twitch", &twitch_status, twitch.phase == "Online");
            });
            ui.add_space(7.0);
            ui.separator();
            ui.add_space(6.0);

            // The switch is a saved preference. Operational readiness is
            // independent and shown only on this Automation page.
            let capture_enabled=native.capture_settings().enabled;
            let capture=&game.native_capture_status;
            let capture_connected=game.phase=="Online";
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Auto Catch")).strong().size(11.0).color(TEXT));
                ui.label(RichText::new(tr(auto_catch_setting_label(capture_enabled)))
                    .size(11.0).strong()
                    .color(if capture_enabled {GOOD} else {MUTED}));
                ui.add_space(8.0);
                if ui.add_enabled(!app.capture_save_busy[index],
                    egui::Button::new(tr(if capture_enabled {"Disable"} else {"Enable"})))
                    .on_hover_text(tr("Turn Auto Catch on or off for this account"))
                    .clicked()
                {
                    // Use the same confirmed save/event flow as the editor.
                    // A failed vault write must not silently change the switch.
                    // Toggle ONLY the current saved switch. Do not silently
                    // commit unrelated unsaved changes in the editor.
                    let mut next=native.capture_settings();
                    next.enabled=!capture_enabled;
                    if let Err(error)=next.validate() {
                        app.capture_save_error[index]=Some(error);
                    } else {
                        app.capture_save_error[index]=None;
                        app.capture_save_busy[index]=true;
                        let tx=app.capture_save_tx.clone();
                        std::thread::spawn(move || {
                            let result=std::panic::catch_unwind(
                                std::panic::AssertUnwindSafe(||
                                    crate::native_auth::set_capture_settings(profile,&next)))
                                .unwrap_or_else(|_|Err("Encrypted native settings save panicked".into()));
                            let _=tx.send(CaptureSavedEvent{
                                index,config:next,toggle_only:true,result
                            });
                        });
                    }
                }
                if app.capture_save_busy[index] {
                    ui.label(RichText::new(tr("Saving Auto Catch settings"))
                        .size(10.0).color(WARN));
                }
            });
            if capture_enabled {
                let operational=auto_catch_operational_label(capture_connected,capture);
                ui.label(RichText::new(format!("{}: {}",tr("Capture operation"),tr(operational)))
                    .size(11.0).color(if capture_connected && capture.capture_ready &&
                        capture.pending_results==0 {GOOD} else {WARN}));
                if capture_connected && !capture.capture_ready {
                    if let Some(reason)=capture.blocked_reason.as_deref() {
                        ui.label(RichText::new(tr(reason)).size(10.0).color(MUTED));
                    }
                }
            }
            // Only native-matched results are attributed to Auto Catch.
            // The server's broader ball-event totals may include VIP/manual.
            ui.horizontal_wrapped(|ui| {
                for (label,value) in [
                    ("Throws sent",capture.throws),
                    ("Auto Catch matched captures",capture.catches),
                    ("Auto Catch matched failures",capture.failures),
                    ("Pending capture receipts",capture.pending_results)
                ] {
                    ui.label(RichText::new(format!("{}: {}",tr(label),value))
                        .size(10.0).color(MUTED));
                }
            });

            ui.collapsing(tr("Capture diagnostics"),|ui| {
            if capture_enabled {
                let restock=&game.native_restock;
                let selected_ball=restock.rules.iter().find(|r|r.kind=="ball");
                let ball_detail=selected_ball.map(|ball| {
                    format!(" · {} #{}: {}/{}",
                        ball.name,ball.id,
                        ball.current.map(|n|n.to_string())
                            .unwrap_or_else(||"—".into()),ball.target)
                }).unwrap_or_default();
                ui.label(RichText::new(format!(
                    "{}: {}{} · {}: {}",
                    tr("Restock"),tr(&restock.phase),ball_detail,
                    tr("Stock awaiting confirmation"),
                    game.native_capture_status.unreflected_throws
                )).size(10.0).color(
                    if restock.last_error.is_some() {WARN} else {DIM}
                ));
                if let Some(error)=restock.last_error.as_deref() {
                    ui.label(RichText::new(error).size(10.0).color(WARN));
                }
                if game.native_capture_status.capture_resyncs>0 {
                    ui.label(RichText::new(format!(
                        "{}: {}",tr("Capture timeouts"),
                        game.native_capture_status.capture_resyncs
                    )).size(10.0).color(WARN));
                }
                if let Some(notice)=game.native_capture_status.last_server_notice.as_deref() {
                    ui.label(RichText::new(format!(
                        "{}: {}",tr("Capture server notice"),notice
                    )).size(10.0).color(WARN));
                }
                if let Some(warning)=game.native_capture_status.warning.as_deref() {
                    ui.label(RichText::new(tr(warning)).size(10.0).color(WARN));
                }
            }
            if game.native_capture_status.deaths_without_slot>0 {
                ui.label(RichText::new(format!("{} {}",
                    game.native_capture_status.deaths_without_slot,
                    tr("wild death events without capture slots")))
                    .size(10.0).color(WARN));
            }
            if !game.native_capture_status.last_action.is_empty() {
                ui.label(RichText::new(tr(&game.native_capture_status.last_action))
                    .size(10.0).color(MUTED));
            }
            if let Some(error)=game.native_capture_status.error.as_deref() {
                ui.label(RichText::new(tr(error)).size(10.0).color(WARN));
            }
            }); // Capture diagnostics: keep transient messages off account cards.
            ui.collapsing(tr("Auto Catch · ball statistics"),|ui|{
                let capture=&game.native_capture_status;
                if capture.used_by_ball.is_empty() &&
                    capture.server_caught_by_ball.is_empty() {
                    ui.label(RichText::new(tr("No Auto Catch attempts recorded yet"))
                        .size(10.0).color(DIM));
                }
                let ids=capture.used_by_ball.keys()
                    .chain(capture.server_caught_by_ball.keys())
                    .copied().collect::<std::collections::BTreeSet<_>>();
                for id in ids.iter() {
                    let sent=capture.used_by_ball.get(id).copied().unwrap_or(0);
                    let confirmed=capture.server_caught_by_ball.get(id).copied().unwrap_or(0);
                    let matched=capture.caught_by_ball.get(id).copied().unwrap_or(0);
                    ui.label(RichText::new(format!(
                        "Ball #{id}: {sent} sent · {confirmed} {} · {matched} {}",
                        tr("Ball-event captures"),tr("Native-matched")
                    )).size(10.0).color(MUTED));
                }
                ui.label(RichText::new(format!(
                    "{}: {} · {}: {} · {}: {}",
                    tr("Ball-event captures"),capture.server_catches,
                    tr("Unmatched receipts"),capture.unmatched_successes,
                    tr("Stock awaiting confirmation"),capture.unreflected_throws
                )).size(10.0).color(DIM));
                ui.label(RichText::new(
                    tr("Only observed ball battle events are counted; built-in VIP captures may not generate them.")
                ).size(10.0).color(DIM));
            });
            ui.add_space(7.0);
            ui.separator();
            ui.add_space(6.0);
            if game.phase == "Online" {
                ui.add_space(5.0);
                ui.collapsing(tr("Game actions"), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.small_button(tr("Pokémon Center")).clicked() {
                            if let Err(error) = native.send_command(GameCommand::Center) {
                                crate::logging::warn(&format!(
                                    "Native center command rejected: {error}"
                                ));
                            }
                        }
                        if let Some(slug) = game.hunt_slug.as_deref() {
                            if ui.small_button(tr("Return to hunt"))
                                .on_hover_text(slug).clicked()
                            {
                                if let Err(error) = native.send_command(
                                    GameCommand::Hunt(slug.to_owned())
                                ) {
                                    crate::logging::warn(&format!(
                                        "Native hunt command rejected: {error}"
                                    ));
                                }
                            }
                        }
                    });
                });
            }
        });
    ui.add_space(8.0);
    egui::CollapsingHeader::new(tr("Configure Auto Catch"))
        .id_salt(("native-capture-editor",index))
        .default_open(true)
        .show(ui,|ui|{
            draw_native_capture_config(app,ui,index);
        });
}

fn capture_ball_name(k:&str)->&'static str {
    match k {"beast"=>"Beast Ball","ultra"=>"Ultra Ball","super"=>"Super Ball",
        "great"=>"Great Ball","poke"=>"Poké Ball",_=>"None"}
}
// Auto Catch+ restock picker: compact selectable items instead of raw IDs.
// IDs are the game protocol IDs. Public metadata
// only enriches captions; absent metadata never switches a saved selection.
// Compact, icon-only Auto Catch+ choice. Names and stock remain in tooltips.
// Every variant must occupy the same fixed-sized cell: a centered/justified
// fallback label used to stretch the "Off" cell across an entire column,
// hiding the real ball/potion icons on the next wrapped row.
const RESTOCK_ICON_SIZE: f32 = 23.0;
const RESTOCK_ICON_PADDING: i8 = 4;

fn restock_selector_columns(available_width: f32) -> usize {
    // Each full picker needs roughly 240 px for its six or seven icon cells.
    // Overview account cards are often only 350–500 px wide, so use stacked
    // full-width groups there instead of three unusably narrow columns.
    if available_width >= 900.0 { 3 } else { 1 }
}

#[cfg(test)]
mod restock_picker_layout_tests {
    use super::restock_selector_columns;
    #[test]
    fn full_width_groups_on_account_cards_and_three_columns_on_wide_panels() {
        for width in [230.0, 350.0, 480.0, 779.0, 899.0] {
            assert_eq!(restock_selector_columns(width), 1);
        }
        for width in [900.0, 1100.0, 1400.0] {
            assert_eq!(restock_selector_columns(width), 3);
        }
    }
}

// Rebase only the restock portion onto the latest worker preferences. This
// prevents an old restock draft from resetting Auto Catch's ball priorities,
// reserves or enabled state when the two settings pages are edited in sequence.
fn copy_restock_fields(to: &mut CatchSettings, from: &CatchSettings) {
    to.restock_enabled = from.restock_enabled;
    to.gold_reserve = from.gold_reserve;
    to.ball_id = from.ball_id;
    to.potion_id = from.potion_id;
    to.revive_id = from.revive_id;
    to.ball_at = from.ball_at;
    to.ball_to = from.ball_to;
    to.potion_at = from.potion_at;
    to.potion_to = from.potion_to;
    to.revive_at = from.revive_at;
    to.revive_to = from.revive_to;
}

fn restock_fields_equal(a: &CatchSettings, b: &CatchSettings) -> bool {
    let mut updated = b.clone();
    copy_restock_fields(&mut updated, a);
    updated == *b
}

#[cfg(test)]
mod restock_selection_tests {
    use super::*;
    #[test]
    fn golden_potion_and_max_revive_preserve_independent_selections() {
        let mut staged = CatchSettings::default();
        staged.potion_id = 70070;
        staged.revive_id = 206;
        staged.restock_enabled = true;
        staged.validate().unwrap();

        let mut latest = CatchSettings::default();
        latest.enabled = true;
        latest.interval_ms = 800;
        copy_restock_fields(&mut latest, &staged);
        assert_eq!(latest.potion_id, 70070);
        assert_eq!(latest.revive_id, 206);
        assert!(latest.enabled);
        assert_eq!(latest.interval_ms, 800);
        assert!(restock_fields_equal(&staged, &latest));
        assert_eq!(CatchSettings::from_value(Some(&latest.as_value())), latest);

        staged.revive_id = 205;
        assert!(!restock_fields_equal(&staged, &latest));
        copy_restock_fields(&mut latest, &staged);
        assert_eq!(latest.potion_id, 70070);
        assert_eq!(latest.revive_id, 205);
    }
}

fn capture_restock_chip(
    ui: &mut egui::Ui, visuals: &mut VisualCache, kind: SpriteKind,
    group: &'static str, selected: &mut u64, id: u64, name: &str, icon: &str,
    icon_hint: Option<&str>, price: Option<u64>, stock: Option<u64>,
) {
    let active = *selected == id;
    // A real Button owns both its artwork and pointer/keyboard interaction.
    // Interacting with a Frame's response after placing an Image inside it
    // let adjacent/wrapped controls compete for clicks, particularly at the
    // Golden Potion → Revives boundary. The ID is stable across icon loads.
    ui.push_id(("restock-item", group, id), |ui| {
        let button = if let Some(texture) = visuals.texture_id(ui, kind, id, icon_hint) {
            // Button::image clamps artwork to the text height; use the
            // regular Button atom to preserve the full 23px game icon.
            egui::Button::new(egui::Image::new((
                texture, egui::vec2(RESTOCK_ICON_SIZE, RESTOCK_ICON_SIZE)
            )))
        } else {
            egui::Button::new(RichText::new(icon).size(15.0))
        }
        .fill(if active { ACCENT.linear_multiply(0.25) } else { PANEL_ALT })
        .stroke(Stroke::new(if active { 1.5 } else { 1.0 },
            if active { ACCENT } else { BORDER }))
        .corner_radius(5.0)
        .selected(active);
        let price_hint = price.map(|p| format!(" · {p} gold")).unwrap_or_default();
        let stock_hint = stock.map(|n| format!(" · {n} in bag")).unwrap_or_default();
        if ui.add_sized(
            [RESTOCK_ICON_SIZE + 2.0 * RESTOCK_ICON_PADDING as f32;
             2], button
        ).on_hover_text(format!("{name}{price_hint}{stock_hint}")).clicked() {
            *selected = id;
        }
    });
}
fn capture_restock_threshold(
    ui:&mut egui::Ui,label:&str,at:&mut u64,to:&mut u64,
    current:Option<u64>
) {
    ui.horizontal_wrapped(|ui|{
        ui.add_sized([77.0,18.0],egui::Label::new(
            RichText::new(label).strong().size(10.0).color(TEXT)));
        ui.label(RichText::new(tr("at ≤")).size(10.0).color(DIM));
        ui.add_sized([75.0,20.0],egui::DragValue::new(at).range(0..=1_000_000));
        ui.label(RichText::new(tr("to")).size(10.0).color(DIM));
        ui.add_sized([75.0,20.0],egui::DragValue::new(to).range(0..=1_000_000));
        if let Some(count)=current {
            ui.label(RichText::new(format!("{count} {}",tr("in bag"))).size(10.0)
                .color(if count <= *at {WARN} else {DIM}));
        }
    });
}
/// Live restock picker ported from Auto Catch+. Each account card edits only its
/// own authenticated native settings, with no item-ID input or explicit save step.
fn draw_overview_restock(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize) {
    if !app.restock_open[index] { return; }
    request_item_catalog(app);
    app.visuals.poll(ui.ctx());
    let Some(native) = app.games[index].native.as_ref() else {
        ui.label(RichText::new(tr("Start this account to configure restock"))
            .size(10.0).color(WARN));
        return;
    };
    let profile = app.games[index].profile;
    let current = native.capture_settings();
    if app.restock_drafts[index].is_none() {
        app.restock_drafts[index] = Some(current.clone());
    }
    let cfg = app.restock_drafts[index].as_mut().expect("restock draft initialized");
    let previous = cfg.clone();
    {
    let restock_status=native.snapshot().native_restock;
    egui::Frame::new().fill(PANEL).corner_radius(8.0)
        .inner_margin(egui::Margin::same(10)).show(ui,|ui|{
        ui.horizontal_wrapped(|ui|{
            ui.checkbox(&mut cfg.restock_enabled,tr("Automatic restock"));
            ui.label(RichText::new(tr(&restock_status.phase)).size(10.0)
                .color(if restock_status.pending {WARN} else {DIM}));
            ui.label(RichText::new(format!(
                "{} ordered · {} confirmed",
                restock_status.ordered,restock_status.confirmed
            )).size(10.0).color(DIM));
        });
        let game_catalog=native.hunt_snapshot();
        let catalog=app.inventory_catalog.as_deref();
        let selected_stock=|kind:&str,id:u64| -> Option<u64> {
            // The authoritative HuntView inventory contains every owned item.
            // RestockStatus only contains already-saved rules, so it cannot
            // give stock for a newly selected potion/revive until saving ends.
            let inventory = if kind == "ball" {
                game_catalog.balls.as_ref()
            } else {
                game_catalog.items.as_ref()
            };
            selected_restock_stock(inventory, id).or_else(||
                restock_status.rules.iter()
                    .find(|rule|rule.kind==kind && rule.id==id)
                    .and_then(|rule|rule.current))
        };
        let selected_price=|kind:&str,id:u64| -> Option<u64> {
            restock_status.rules.iter()
                .find(|rule|rule.kind==kind && rule.id==id)
                .and_then(|rule|rule.price)
        };
        ui.add_space(5.0);
        let columns = restock_selector_columns(ui.available_width());
        ui.columns(columns, |cols| {
        let potion_col = if columns == 3 { 1 } else { 0 };
        let revive_col = if columns == 3 { 2 } else { 0 };
        cols[0].label(RichText::new(tr("Balls")).strong().size(11.0).color(TEXT));
        cols[0].horizontal_wrapped(|ui|{
            capture_restock_chip(ui,&mut app.visuals,SpriteKind::Ball,
                "ball", &mut cfg.ball_id,0,"Off","—",None,None,None);
            for (id, fallback) in [(1,"Poké Ball"),(2,"Great Ball"),
                (3,"Super Ball"),(4,"Ultra Ball"),(5,"Beast Ball")] {
                let name = game_catalog.ball_names.get(&id)
                    .map(String::as_str).unwrap_or(fallback);
                capture_restock_chip(ui,&mut app.visuals,SpriteKind::Ball,
                "ball", &mut cfg.ball_id,id,name,"●",None,
                    selected_price("ball",id),selected_stock("ball",id));
            }
            if cfg.ball_id>5 && !game_catalog.ball_names.contains_key(&cfg.ball_id) {
                let id=cfg.ball_id;
                capture_restock_chip(ui,&mut app.visuals,SpriteKind::Ball,
                "ball", &mut cfg.ball_id,id,&format!("Saved ball #{id}"),"●",None,None,None);
            }
        });
        if columns == 1 { cols[0].add_space(8.0); }
        cols[potion_col].label(RichText::new(tr("Potions")).strong().size(11.0).color(TEXT));
        // Match Auto Catch+: Small, Ultimate and Golden Potion. Preserve
        // any existing selection even if the downloaded catalog is old.
        let mut potion_choices=vec![
            (200_u64,String::from("Small Potion")),
            (201_u64,String::from("Great Potion")),
            (202_u64,String::from("Ultra Potion")),
            (203_u64,String::from("Hyper Potion")),
            (204_u64,String::from("Ultimate Potion")),
            (70070_u64,String::from("Golden Potion"))
        ];
        if let Some(c) = catalog {
            for (id, item) in &c.items {
                let name = item.name.to_lowercase();
                if (name.contains("potion") || name.contains("poção") ||
                    name.contains("pocao")) && item.price_gold.unwrap_or(0)>0 &&
                    !potion_choices.iter().any(|(existing,_)|existing==id) {
                    potion_choices.push((*id,item.name.clone()));
                }
            }
        }
        if cfg.potion_id>0 && !potion_choices.iter().any(|(id,_)|*id==cfg.potion_id) {
            let name=catalog.map(|c|c.name(cfg.potion_id))
                .unwrap_or_else(||format!("Saved potion #{}",cfg.potion_id));
            potion_choices.push((cfg.potion_id,name));
        }
        cols[potion_col].horizontal_wrapped(|ui|{
            capture_restock_chip(ui,&mut app.visuals,SpriteKind::Item,
                "potion", &mut cfg.potion_id,0,"Off","—",None,None,None);
            for (id,name) in potion_choices {
                let price=selected_price("item",id)
                    .or_else(||catalog.and_then(|c|c.items.get(&id))
                        .and_then(|item|item.price_gold));
                capture_restock_chip(ui,&mut app.visuals,SpriteKind::Item,
                "potion", &mut cfg.potion_id,id,&name,"💊",
                    catalog.and_then(|c|c.items.get(&id)).and_then(|m|m.icon.as_deref()),
                    price,selected_stock("item",id));
            }
        });
        if columns == 1 { cols[0].add_space(8.0); }
        cols[revive_col].label(RichText::new(tr("Revives")).strong().size(11.0).color(TEXT));
        let mut revive_choices=vec![
            (205_u64,String::from("Revive")),
            (206_u64,String::from("Max Revive"))
        ];
        if let Some(c)=catalog {
            for (id,item) in &c.items {
                if item.name.to_lowercase().contains("revive") &&
                    !revive_choices.iter().any(|(existing,_)|existing==id) {
                    revive_choices.push((*id,item.name.clone()));
                }
            }
        }
        if cfg.revive_id>0 && !revive_choices.iter().any(|(id,_)|*id==cfg.revive_id) {
            revive_choices.push((cfg.revive_id,format!("Saved revive #{}",cfg.revive_id)));
        }
        cols[revive_col].horizontal_wrapped(|ui|{
            capture_restock_chip(ui,&mut app.visuals,SpriteKind::Item,
                "revive", &mut cfg.revive_id,0,"Off","—",None,None,None);
            for (id,name) in revive_choices {
                let price=selected_price("item",id)
                    .or_else(||catalog.and_then(|c|c.items.get(&id))
                        .and_then(|item|item.price_gold));
                capture_restock_chip(ui,&mut app.visuals,SpriteKind::Item,
                "revive", &mut cfg.revive_id,id,&name,"✚",
                    catalog.and_then(|c|c.items.get(&id)).and_then(|m|m.icon.as_deref()),
                    price,selected_stock("item",id));
            }
        });
        });
        ui.add_space(10.0);
        ui.label(RichText::new(tr("Restock at ≤ / Fill to")).size(10.0).color(DIM));
        capture_restock_threshold(ui,tr("Balls"),&mut cfg.ball_at,&mut cfg.ball_to,
            selected_stock("ball",cfg.ball_id));
        capture_restock_threshold(ui,tr("Potions"),&mut cfg.potion_at,&mut cfg.potion_to,
            selected_stock("item",cfg.potion_id));
        capture_restock_threshold(ui,tr("Revives"),&mut cfg.revive_at,&mut cfg.revive_to,
            selected_stock("item",cfg.revive_id));
        ui.horizontal(|ui|{
            ui.label(RichText::new(tr("Gold reserve")).size(10.0).color(MUTED));
            ui.add(egui::DragValue::new(&mut cfg.gold_reserve).range(0..=u64::MAX));
        });
        if let Some(error)=restock_status.last_error.as_deref() {
            ui.label(RichText::new(error).size(10.0).color(WARN));
        }
    });
    }
    if *cfg != previous {
        app.restock_dirty[index] = true;
    }
    if let Some(error) = app.capture_save_error[index].as_deref() {
        ui.label(RichText::new(error).size(10.0).color(WARN));
    }
    if app.capture_save_busy[index] {
        ui.label(RichText::new(if app.restock_dirty[index] {
            tr("Restock changes queued; saving previous selection...")
        } else {
            tr("Saving encrypted preferences...")
        }).size(10.0).color(if app.restock_dirty[index] { WARN } else { DIM }));
    }
    if app.restock_dirty[index] && !app.capture_save_busy[index] {
        // A selection made during an earlier save stays visible and is sent
        // immediately after the previous acknowledgement. The latest current
        // worker configuration is used to avoid overwriting Auto Catch.
        let mut next = current;
        copy_restock_fields(&mut next,
            app.restock_drafts[index].as_ref().expect("restock draft initialized"));
        if let Err(error) = next.validate() {
            app.capture_save_error[index] = Some(error);
            app.restock_dirty[index] = false; // Retry after the next edit.
        } else {
            app.capture_save_error[index] = None;
            app.restock_dirty[index] = false;
            app.capture_save_busy[index] = true;
            let tx = app.capture_save_tx.clone();
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(
                    std::panic::AssertUnwindSafe(||
                        crate::native_auth::set_capture_settings(profile, &next)))
                    .unwrap_or_else(|_| Err("Encrypted restock save panicked".into()));
                let _ = tx.send(CaptureSavedEvent { index, config: next, toggle_only:false, result });
            });
        }
    }
}

fn draw_native_capture_config(app:&mut ControllerApp,ui:&mut egui::Ui,index:usize) {
    let profile=app.games[index].profile;
    let Some(native)=app.games[index].native.as_ref() else {return};
    if app.capture_drafts[index].is_none() {
        app.capture_drafts[index]=Some(native.capture_settings());
    }
    let Some(cfg)=app.capture_drafts[index].as_mut() else{return};
    ui.add_enabled(!app.capture_save_busy[index],
        egui::Checkbox::new(&mut cfg.enabled,tr("Enable Auto Catch")));
    ui.horizontal_wrapped(|ui|{
        ui.label(RichText::new(tr("Throw interval")).size(10.0).color(MUTED));
        ui.add(egui::DragValue::new(&mut cfg.interval_ms)
            .range(600..=3000).speed(50.0).suffix(" ms"));
        ui.label(RichText::new(tr("Native interval: 0.6 s, with a bounded pending-throw window"))
            .size(10.0).color(DIM));
    });
    ui.label(RichText::new(tr("Preferred balls (in order; Master Balls excluded)"))
        .size(11.0).color(TEXT));
    let keys=["beast","ultra","super","great","poke"];
    let mut chosen=[
        cfg.priorities.first().cloned().unwrap_or_default(),
        cfg.priorities.get(1).cloned().unwrap_or_default()
    ];
    ui.horizontal_wrapped(|ui|{
        for slot in 0..2 {
            ui.label(if slot==0{tr("Primary")}else{tr("Fallback")});
            egui::ComboBox::from_id_salt(("cap-choice",index,slot))
                .selected_text(capture_ball_name(&chosen[slot])).show_ui(ui,|ui|{
                    ui.selectable_value(&mut chosen[slot],String::new(),tr("None"));
                    for key in keys {
                        ui.selectable_value(&mut chosen[slot],key.into(),capture_ball_name(key));
                    }
                });
        }
    });
    cfg.priorities=chosen.iter().filter(|v|!v.is_empty()).cloned().collect();
    ui.label(RichText::new(tr("Ball inventory reserves")).size(11.0).color(DIM));
    ui.horizontal_wrapped(|ui|{
        for key in keys {
            ui.vertical(|ui|{
                ui.label(RichText::new(capture_ball_name(key)).size(10.0).color(DIM));
                ui.add(egui::DragValue::new(cfg.reserves.entry(key.into()).or_default())
                    .range(0..=1_000_000));
            });
        }
    });
    ui.separator();
    ui.label(RichText::new(tr("Restock is configured on each Overview account card"))
        .size(10.0).color(DIM));
    if let Some(error)=app.capture_save_error[index].as_deref(){
        ui.colored_label(BAD,error);
    }
    let mut discard_edits=false;
    ui.horizontal(|ui|{
        if ui.add_enabled(!app.capture_save_busy[index],
            egui::Button::new(tr("Save settings"))).clicked() {
            let to_save=cfg.clone();
            if let Err(error)=to_save.validate() {
                app.capture_save_error[index]=Some(error);
            } else {
                app.capture_save_error[index]=None;
                app.capture_save_busy[index]=true;
                let tx=app.capture_save_tx.clone();
                std::thread::spawn(move || {
                    let result=std::panic::catch_unwind(
                        std::panic::AssertUnwindSafe(||
                            crate::native_auth::set_capture_settings(profile,&to_save))
                    ).unwrap_or_else(|_|Err("Encrypted native settings save panicked".into()));
                    let _=tx.send(CaptureSavedEvent{index,config:to_save,toggle_only:false,result});
                });
            }
        }
        if ui.small_button(tr("Discard")).clicked() {
            discard_edits=true;
        }
    });
    if discard_edits {
        app.capture_drafts[index]=Some(native.capture_settings());
        app.capture_save_error[index]=None;
    }
    if app.capture_save_busy[index] {
        ui.label(RichText::new(tr("Saving encrypted preferences..."))
            .size(10.0).color(MUTED));
    }
}

fn draw_native_manager_details(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize) {
    // Take one read-only snapshot for the entire render. All network parsing
    // and commands remain on the game worker, never the egui render thread.
    let (status, view, twitch) = {
        let native = app.games[index].native.as_ref().expect("native details require a session");
        (native.snapshot(), native.hunt_snapshot(), native.twitch_snapshot())
    };
    let phase_online = status.phase == "Online";
    request_item_catalog(app);
    let item_catalog = app.inventory_catalog.clone();
    let hunt_name = view.hunts.iter()
        .find(|hunt| Some(hunt.slug.as_str()) == view.hunt_slug.as_deref())
        .map(|hunt| hunt.name.as_str())
        .or(view.hunt_slug.as_deref()).unwrap_or("—");
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .inner_margin(egui::Margin::same(14)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("ACCOUNT DETAILS")).size(12.0)
                    .strong().color(TEXT));
                automation_badge(ui, "PokéIdle connection", &status.phase, phase_online);
                automation_badge(ui, "Twitch", &twitch.phase,
                    twitch.phase == "Online");
                if let Some(last) = status.last_received {
                    ui.label(RichText::new(tr_dynamic(format!("Last game update: {}s ago",
                        last.elapsed().as_secs()))).size(10.0).color(MUTED));
                }
            });
            if !phase_online {
                ui.colored_label(WARN, tr("Live game state is not confirmed; values below may be stale."));
            }
            if let Some(error) = status.error.as_deref() {
                ui.label(RichText::new(error).size(10.0).color(WARN));
            }
            ui.add_space(9.0);
            ui.columns(3, |cols| {
                detail_stat_line(&mut cols[0], tr("Hunt"),
                    if view.in_center { tr("Pokémon Center") } else { hunt_name });
                detail_stat_line(&mut cols[0], tr("In-game nickname"),
                    view.account_nick.as_deref().unwrap_or("—"));
                detail_stat_line(&mut cols[0], tr("Trainer level"),
                    &view.trainer_level.map(|n| n.to_string()).unwrap_or_else(|| "—".into()));
                detail_stat_line(&mut cols[0], tr("Trainer XP"),
                    &view.trainer_xp.map(format_number).unwrap_or_else(|| "—".into()));

                detail_stat_line(&mut cols[1], tr("Active Pokémon"),
                    view.active_pokemon.as_ref().map(|p| p.name.as_str()).unwrap_or("—"));
                detail_stat_line(&mut cols[1], tr("Pokémon level"),
                    &view.active_pokemon.as_ref().map(|p|p.level.to_string())
                        .unwrap_or_else(|| "—".into()));
                detail_stat_line(&mut cols[1], tr("Pokémon XP"),
                    &view.active_pokemon.as_ref().and_then(|p|p.xp)
                        .map(format_number).unwrap_or_else(|| "—".into()));

                detail_stat_line(&mut cols[2], tr("Gold"),
                    &view.gold.map(format_number).unwrap_or_else(|| "—".into()));
                detail_stat_line(&mut cols[2], tr("Orbs"),
                    &view.orbs.map(format_number).unwrap_or_else(|| "—".into()));
                detail_stat_line(&mut cols[2], tr("Diamonds"),
                    &view.diamonds.map(format_number).unwrap_or_else(|| "—".into()));
            });
            if let Some(hunt) = view.hunts.iter().find(|hunt|
                Some(hunt.slug.as_str()) == view.hunt_slug.as_deref()) {
                ui.label(RichText::new(tr_dynamic(format!("{} · {} · Lv {} · {} spawn slots",
                    hunt.area, hunt.region, hunt.level, hunt.total_spawns)))
                    .size(10.0).color(DIM));
            }
            if let Some(map) = view.field_map.as_deref() {
                ui.label(RichText::new(format!("{}: {}",tr("Field map"),map))
                    .size(10.0).color(DIM));
            }
            if let Some(wild) = view.wild.as_ref() {
                ui.label(RichText::new(tr_dynamic(format!("Wild: {} · Lv {} · HP {}/{}",
                    wild.name, wild.level, wild.hp, wild.max_hp)))
                    .size(10.0).color(MUTED));
            }
            ui.horizontal_wrapped(|ui| {
                let bonus = &view.bonuses;
                for (label, percent) in [
                    ("Twitch", bonus.twitch_pct),
                    ("Guild", bonus.guild_pct),
                    ("Event trainer", bonus.trainer_event_pct),
                    ("Event Pokémon", bonus.pokemon_event_pct),
                ] {
                    ui.label(RichText::new(format!("{label} +{percent:.0}%"))
                        .size(10.0).color(DIM));
                }
                if bonus.vip_active { ui.label(RichText::new("VIP").color(GOOD)); }
                if bonus.guild_boost_active { ui.label(RichText::new(tr("Guild boost")).color(GOOD)); }
            });
        });
    ui.add_space(9.0);
    egui::Frame::new().fill(PANEL_ALT).corner_radius(10.0)
        .inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("HUNT SESSION")).size(12.0)
                    .strong().color(TEXT));
                for text in [
                    format!("{} {}",view.session.kills,tr("kills")),
                    format!("{} {}",view.session.captures,tr("captures")),
                    format!("{} {}",view.session.shiny_captured,tr("shinies caught")),
                    format!("{:02}:{:02}:{:02}",view.session.duration_secs/3600,
                        (view.session.duration_secs/60)%60,view.session.duration_secs%60),
                ] {
                    ui.label(RichText::new(text).size(10.0).color(MUTED));
                }
                if ui.small_button(tr("Open Hunt Atlas")).clicked() {
                    app.hunt_account = index;
                    app.panel_tab = PanelTab::Hunts;
                }
            });
            ui.columns(3, |cols| {
                let m = &view.measurements;
                detail_stat_line(&mut cols[0], tr("Trainer XP/h"),
                    &if m.valid_trainer { format_hunt_amount(m.trainer_xp_h) } else { "—".into() });
                detail_stat_line(&mut cols[1], tr("Pokémon XP/h"),
                    &if m.valid_pokemon { format_hunt_amount(m.pokemon_xp_h) } else { "—".into() });
                detail_stat_line(&mut cols[2], tr("Net gold/h"),
                    &if m.valid_gold { format_hunt_amount(m.gold_net_h) } else { "—".into() });
            });
            ui.label(RichText::new(format!("{} · {}s · {} {}",
                view.sample_quality, view.measurements.seconds.round() as u64,
                format_hunt_amount(view.measurements.kills_h),tr("kills/h")))
                .size(10.0).color(MUTED));
            ui.columns(2, |cols| {
                detail_stat_line(&mut cols[0], tr("Trainer XP earned"),
                    &format_number(view.session.trainer_xp));
                detail_stat_line(&mut cols[0], tr("Pokémon XP earned"),
                    &format_number(view.session.pokemon_xp));
                detail_stat_line(&mut cols[1], tr("Combat gold"),
                    &format_number(view.session.combat_gold));
                detail_stat_line(&mut cols[1], tr("Auto-sale gold"),
                    &format_number(view.session.auto_sale_gold));
            });
            if !view.session.balls_used.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(tr("Balls used")).size(10.0).color(DIM));
                    for (id,quantity) in &view.session.balls_used {
                        ui.horizontal(|ui| {
                            app.visuals.show(ui, SpriteKind::Ball, *id, None, 16.0);
                            ui.label(RichText::new(format!("{} ×{}",view.ball_names.get(id)
                                .map(String::as_str).unwrap_or("Ball"),quantity))
                                .size(10.0).color(TEXT));
                        });
                    }
                });
            }
            if !view.session.recent_drops.is_empty() {
                ui.collapsing(format!("{} ({})",tr("Recent drops"),
                    view.session.recent_drops.len()), |ui| {
                    for (name,qty) in view.session.recent_drops.iter().take(15) {
                        ui.label(format!("{} ×{}", name, qty));
                    }
                });
            }
        });
    ui.add_space(9.0);
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("POKÉMON DEPOT")).size(12.0)
                    .strong().color(TEXT));
                ui.label(RichText::new(format!("{} {}",view.pokemon_roster.len(),
                    tr("Pokémon"))).size(10.0).color(MUTED));
                ui.label(RichText::new(format!("{} / {}",
                    view.pokemon_roster.iter().filter(|p|p.hp==Some(0)).count(),
                    tr("fallen"))).size(10.0).color(MUTED));
            });
            if view.pokemon_roster.is_empty() {
                ui.label(RichText::new(tr("Loading your Pokémon"))
                    .size(11.0).color(DIM));
            } else {
                egui::ScrollArea::vertical().id_salt(("native-roster",index))
                    .max_height(185.0).show(ui, |ui| {
                    for pokemon in view.pokemon_roster.iter() {
                        ui.horizontal_wrapped(|ui| {
                            let active = view.active_pokemon.as_ref().is_some_and(|p|p.id==pokemon.id);
                            let name = format!("{}{} · Lv {}{}",
                                if active {"▶ "} else {""},pokemon.name,pokemon.level,
                                if pokemon.shiny {" ✨"} else {""});
                            let detail = format!(
                                "ID #{} · Species #{} · XP {} · Next {}\nTypes: {} · Power: {} · Held item #{}\nGrade: {} · Looktype: {} / {}\nIVs: {}",
                                pokemon.id,
                                pokemon.species_id.map(|id|id.to_string())
                                    .unwrap_or_else(|| "—".into()),
                                pokemon.xp.map(format_number).unwrap_or_else(|| "—".into()),
                                pokemon.xp_next.map(format_number).unwrap_or_else(|| "—".into()),
                                pokemon.types.join(", "),
                                pokemon.power.map(|n|n.to_string())
                                    .unwrap_or_else(|| "—".into()),
                                pokemon.held_item_id.map(|n|n.to_string())
                                    .unwrap_or_else(|| "—".into()),
                                pokemon.grade.map(|n|format!("{n:.2}"))
                                    .unwrap_or_else(|| "—".into()),
                                pokemon.looktype.map(|n|n.to_string())
                                    .unwrap_or_else(|| "—".into()),
                                pokemon.shiny_looktype.map(|n|n.to_string())
                                    .unwrap_or_else(|| "—".into()),
                                pokemon.ivs.iter().map(|value|value.map(|n|n.to_string())
                                    .unwrap_or_else(|| "—".into()))
                                    .collect::<Vec<_>>().join(" / "),
                            );
                            ui.label(RichText::new(name).size(11.0)
                                .color(if active { GOOD } else { TEXT }))
                                .on_hover_text(detail);
                            ui.label(RichText::new(format!(
                                "HP {} / {} · Q {} · P{}",
                                pokemon.hp.map(|n|n.to_string()).unwrap_or_else(|| "—".into()),
                                pokemon.max_hp.map(|n|n.to_string()).unwrap_or_else(|| "—".into()),
                                pokemon.quality.map(|n|format!("{n:.2}")).unwrap_or_else(|| "—".into()),
                                pokemon.potency.unwrap_or_default()
                            )).size(10.0).color(DIM));
                        });
                        ui.separator();
                    }
                });
            }
        });
    ui.add_space(9.0);
    egui::Frame::new().fill(PANEL_ALT).corner_radius(10.0)
        .inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            let item_count = view.items.values().filter(|n|**n>0).count();
            let ball_count = view.balls.values().filter(|n|**n>0).count();
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("INVENTORY")).size(12.0)
                    .strong().color(TEXT));
                ui.label(format!("{} {} · {} {}",item_count,tr("items"),
                    ball_count,tr("ball types")));
            });
            ui.horizontal_wrapped(|ui| {
                for (id,qty) in view.balls.iter().filter(|(_,qty)|**qty>0) {
                    ui.horizontal(|ui| {
                        app.visuals.show(ui, SpriteKind::Ball, *id, None, 20.0);
                        ui.label(RichText::new(format!("{} ×{}",view.ball_names.get(id)
                            .map(String::as_str).unwrap_or("Ball"),qty))
                            .size(10.0).color(TEXT));
                    });
                }
            });
            let mut items = view.items.iter().filter(|(_,qty)| **qty>0)
                .collect::<Vec<_>>();
            items.sort_by(|a,b|b.1.cmp(a.1).then_with(||a.0.cmp(b.0)));
            ui.horizontal_wrapped(|ui| {
                for (id,qty) in items.into_iter().take(10) {
                    let name = item_catalog.as_deref().map(|catalog|catalog.name(*id))
                        .unwrap_or_else(||format!("Item #{id}"));
                    let hint = item_catalog.as_deref().and_then(|catalog|
                        catalog.items.get(id).and_then(|item|item.icon.as_deref()));
                    ui.horizontal(|ui| {
                        app.visuals.show(ui, SpriteKind::Item, *id, hint, 20.0);
                        ui.label(RichText::new(format!("{name} ×{qty}"))
                            .size(10.0).color(DIM));
                    });
                }
            });
        });
    // Automation has its own sidebar page. Keep the dashboard for game data.
    ui.add_space(7.0);
    if ui.small_button(tr("Open Automation")).clicked() {
        app.automation_account = index;
        app.panel_tab = PanelTab::Automation;
    }
}

fn draw_game_details(
    app: &mut ControllerApp,
    ui: &mut egui::Ui,
    index: usize,
    width: f32,
) {
    ui.set_width(width);
    if app.games[index].native.is_some() {
        draw_native_manager_details(app, ui, index);
        return;
    }

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
            // Browser controls and warnings live in the compact overview.
            // Keep the full-width detail panel dedicated to game data.
            ui.add_space(2.0);

            ui.add_space(14.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(
                            if running && health.last_successful_probe_at.is_some() && !health.telemetry_is_fresh() {
                                tr("LAST OBSERVED STATE")
                            } else if health.activity == "Hunting" {
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

                    // Stable two-column progress summary: trainer and Pokémon
                    // stats no longer wrap into an ambiguous sequence of labels.
                    ui.columns(2, |columns| {
                        columns[0].label(RichText::new(tr("TRAINER")).size(9.0).strong().color(DIM));
                        columns[0].add_space(3.0);
                        detail_stat_line(&mut columns[0], "Level", &trainer_level);
                        detail_stat_line(&mut columns[0], "XP", if health.player_xp.is_empty() { "—" } else { &health.player_xp });

                        columns[1].label(RichText::new(tr("POKÉMON")).size(9.0).strong().color(DIM));
                        columns[1].add_space(3.0);
                        detail_stat_line(&mut columns[1], "Level", if health.pokemon_level.is_empty() { "—" } else { &health.pokemon_level });
                        detail_stat_line(&mut columns[1], "XP", if health.pokemon_xp.is_empty() { "—" } else { &health.pokemon_xp });
                        detail_stat_line(&mut columns[1], "FALLEN", &fallen);
                    });
                });

            // HuntAtlas shares already-calculated results through the Bridge.
            // An absent or mismatched snapshot is not a rate of zero.
            let atlas = &health.hunt_atlas;
            let matches_hunt = atlas.get("huntSlug").and_then(|v| v.as_str())
                .is_some_and(|slug| !slug.is_empty() && slug == health.hunt_slug);
            if matches_hunt {
                let rate = |key: &str| -> String {
                    atlas.get(key).and_then(|v| v.as_f64())
                        .filter(|n| n.is_finite() && *n >= 0.0)
                        .map(|n| if n < 1.0 { format!("{n:.3}") } else { format!("{n:.1}") })
                        .unwrap_or_else(|| "—".to_string())
                };
                ui.add_space(10.0);
                ui.label(RichText::new(tr("HUNT ATLAS")).size(10.0).strong().color(DIM));
                egui::Frame::new().fill(PANEL_ALT).corner_radius(10.0)
                    .inner_margin(10.0).show(ui, |ui| {
                        ui.columns(2, |cols| {
                            detail_stat_line(&mut cols[0], tr("Trainer XP/h"), &rate("xpH"));
                            let eta = match (health.player_xp_to_next,
                                atlas.get("xpH").and_then(|v| v.as_f64())) {
                                (Some(remaining), Some(xp)) if xp > 0.0 =>
                                    format!("{:.1} h", remaining as f64 / xp),
                                _ => "—".to_string(),
                            };
                            detail_stat_line(&mut cols[0], tr("Next level"), &eta);
                            detail_stat_line(&mut cols[0], tr("Coins/h"), &rate("coinsH"));
                            detail_stat_line(&mut cols[1], tr("Captures/h"), &rate("capturesH"));
                            let shiny_interval = atlas.get("shinyCaughtH").and_then(|v| v.as_f64())
                                .filter(|rate| *rate > 0.0).map(|rate| {
                                    let hours = 1.0 / rate;
                                    if hours >= 48.0 { format!("{:.1} days", hours / 24.0) }
                                    else { format!("{hours:.1} h") }
                                }).unwrap_or_else(|| "—".to_string());
                            detail_stat_line(&mut cols[1], tr("Shiny wait"), &shiny_interval);
                            detail_stat_line(&mut cols[1], tr("Balls/capture"), &rate("ballsPerCapture"));
                        });
                        ui.separator();
                        let chance = |key: &str| -> String {
                            atlas.get(key).and_then(|v| v.as_f64())
                                .filter(|n| n.is_finite() && *n >= 0.0)
                                .map(|n| format!("{:.2}%", n * 100.0))
                                .unwrap_or_else(|| "—".to_string())
                        };
                        detail_stat_line(ui, tr("Ultra Ball"), &chance("ultraChance"));
                        detail_stat_line(ui, tr("Beast Ball"), &chance("beastChance"));
                        if let Some(tier) = atlas.get("activeOutlandTier").and_then(|v| v.as_u64()) {
                            detail_stat_line(ui, tr("Outland tier"), &tier.to_string());
                        }
                        if let Some(drops) = atlas.get("rareDrops").and_then(|v| v.as_array()) {
                            for drop in drops {
                                if let (Some(name), Some(rate)) = (drop.get("name").and_then(|v| v.as_str()),
                                    drop.get("unitsH").and_then(|v| v.as_f64())) {
                                    let label = if rate > 0.0 && rate < 0.01 {
                                        format!("{:.1} days/drop", 1.0 / (rate * 24.0))
                                    } else { format!("{rate:.3}/h") };
                                    detail_stat_line(ui, name, &label);
                                }
                            }
                        }
                        ui.label(RichText::new(if atlas.get("xpMeasured").and_then(|v| v.as_bool()) == Some(true) {
                            tr("Measured XP/h · other rates may be estimated")
                        } else { tr("Estimated rates · not measured") }).size(9.0).color(DIM));
                    });
            }

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
                    let gold = format_number(health.gold);
                    let orbs = format_number(health.orbs);
                    let balls_used = health.autocatch_balls_used.to_string();
                    let captures = health.autocatch_captures.to_string();
                    let bridge_ok = health.bridge_snapshot_ok;

                    // Group balances and capture statistics independently.
                    // Auto Catch ON/OFF is already shown under Automation.
                    ui.columns(2, |columns| {
                        columns[0].label(RichText::new(tr("CURRENCY")).size(9.0).strong().color(DIM));
                        columns[0].add_space(3.0);
                        detail_stat_line(&mut columns[0], "GOLD", if bridge_ok { &gold } else { "—" });
                        detail_stat_line(&mut columns[0], "GEMS", if bridge_ok { &orbs } else { "—" });

                        columns[1].label(RichText::new(tr("CAPTURE RESULTS")).size(9.0).strong().color(DIM));
                        columns[1].add_space(3.0);
                        detail_stat_line(&mut columns[1], "BALLS USED", if bridge_ok { &balls_used } else { "—" });
                        detail_stat_line(&mut columns[1], "CAPTURES", if bridge_ok { &captures } else { "—" });
                        detail_stat_line(&mut columns[1], "SUCCESS", if bridge_ok && !health.autocatch_rate.is_empty() { &health.autocatch_rate } else { "—" });
                    });

                    ui.add_space(9.0);
                    ui.separator();
                    ui.add_space(5.0);
                    ui.label(RichText::new(tr("BALL STOCK")).size(9.0).strong().color(DIM));
                    ui.add_space(5.0);
                    if health.ball_stock.is_empty() {
                        ui.label(RichText::new(tr("Pokéballs: unavailable")).size(10.0).color(MUTED));
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for item in &health.ball_stock {
                                resource_chip(ui, item);
                            }
                        });
                    }
                });

            ui.add_space(12.0);

            ui.label(
                RichText::new(tr("AUTOMATION"))
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            if app.games[index].native.is_some() {
                draw_native_automation(app, ui, index);
            } else {
                ui.label(RichText::new(tr(
                    "Automation is managed in the controller. Start this account to see its current settings."
                )).size(10.0).color(MUTED));
            }

            // Browser-only controls are irrelevant after successful native
            // takeover. The runtime and saved session now own game + IRC.
            if app.games[index].native.is_none() {
                ui.add_space(12.0);
                ui.label(
                    RichText::new(tr("KICK MULTISTREAM"))
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
                        if let Some((link, channels)) = multikick_link(&health.kick_streams) {
                            ui.label(
                                RichText::new(format!(
                                    "{} {}",
                                    channels.len(),
                                    tr("live KICK channels:")
                                ))
                                .size(10.0)
                                .strong()
                                .color(GOOD),
                            );
                            ui.label(
                                RichText::new(channels.join(" · "))
                                    .size(10.0)
                                    .color(TEXT),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(link.as_str())
                                    .size(10.0)
                                    .color(ACCENT),
                            );
                            ui.horizontal_wrapped(|ui| {
                                if ui.button(tr("Copy link")).clicked() {
                                    ui.ctx().copy_text(link.clone());
                                    app.set_status(
                                        tr("Copied MultiKick link. Open it in a regular browser, outside the controller-managed Firefox.").to_string(),
                                        false,
                                    );
                                }
                                if ui.button(tr("Open live channels")).clicked() {
                                    match open_kick_channels_in_regular_browser(&channels) {
                                        Ok(_) => app.set_status(
                                            tr("Opened live KICK channels in the regular browser.").to_string(),
                                            false,
                                        ),
                                        Err(error) => app.set_status(error, true),
                                    }
                                }
                            });
                            ui.label(
                                RichText::new(tr("Copy link copies a MultiKick URL; Open live channels opens each stream individually in your regular browser."))
                                    .size(9.0)
                                    .color(MUTED),
                            );
                        } else {
                            ui.label(
                                RichText::new(tr("No live KICK channels detected."))
                                    .size(10.0)
                                    .color(MUTED),
                            );
                        }
                    });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr(if health.telemetry_is_fresh() || health.last_successful_probe_at.is_none() { "OPEN TABS" } else { "Last reported tabs" }))
                            .size(9.0)
                            .strong()
                            .color(DIM),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let Some(age) = health.tab_inventory_age() {
                            ui.label(
                                RichText::new(format!("{} · {} {}",
                                    tr("Tabs checked"), age.as_secs() / 60, tr("min ago")))
                                    .size(9.0).color(DIM),
                            );
                        }
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
                                        // Accept bare domains, pasted URLs, and mixed-case schemes.
                                        // A malformed URL should not be reported as successfully queued.
                                        let candidate = if raw
                                            .get(..7)
                                            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
                                            || raw
                                                .get(..8)
                                                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
                                        {
                                            raw.to_string()
                                        } else if raw.contains("://") {
                                            String::new()
                                        } else {
                                            format!("https://{}", raw)
                                        };
                                        let has_http_scheme = candidate
                                            .get(..7)
                                            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
                                            || candidate
                                                .get(..8)
                                                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"));
                                        let host = candidate
                                            .split_once("://")
                                            .map(|(_, rest)| rest.split(['/', '?', '#']).next().unwrap_or_default())
                                            .unwrap_or_default();
                                        let valid_url = has_http_scheme
                                            && !host.is_empty()
                                            && !host.contains(char::is_whitespace);

                                        if valid_url {
                                            if let Some(monitor) = app.games[index].monitor.as_ref() {
                                                match monitor.send(json!({
                                                    "t": "browser.openTab",
                                                    "url": candidate
                                                })) {
                                                    Ok(id) => {
                                                        app.tab_url_input[index].clear();
                                                        app.tab_url_open[index] = false;
                                                        app.set_status(
                                                            format!("{} · {} #{}", &account_name, tr("custom tab queued"), id),
                                                            false,
                                                        );
                                                    }
                                                    Err(error) => app.set_status(
                                                        format!("{}: {}", &account_name, error),
                                                        true,
                                                    ),
                                                }
                                            } else {
                                                app.set_status(
                                                    format!("{}: {}", &account_name, tr("Browser monitor is not connected; tab was not queued.")),
                                                    true,
                                                );
                                            }
                                        } else {
                                            app.set_status(
                                                format!("{}: {}", &account_name, tr("Enter a valid http:// or https:// URL, or a domain such as example.com.")),
                                                true,
                                            );
                                        }
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
                                RichText::new(tr(if !running { "No active browser tabs" } else if !health.telemetry_is_fresh() { "Tab list not currently verified" } else if !health.game_ready { "Tab list unavailable while game loads" } else { "No top-level tabs reported" }))
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
                                        RichText::new(if tab.kind == "Twitch" && tab.twitch_channels.is_some() { tr("Shared Twitch chat") } else { tr(&tab.kind) })
                                            .size(9.0)
                                            .strong()
                                            .color(TEXT),
                                    );

                                    ui.add_space(5.0);

                                    let channel_label = tab.twitch_channels.map(|n| format!("{} {}", n, tr("channels in this tab")));
                                    let title = channel_label.as_deref().unwrap_or_else(|| if tab.title.is_empty() { &tab.url } else { &tab.title });

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
                                                RichText::new(if tab.low_resource { tr("LOW RESOURCE") } else { tr("Not optimized") })
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

            }

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
                    if !health.bridge_snapshot_ok {
                        ui.label(RichText::new(tr("Twitch XP bonus unavailable until game data loads")).size(10.0).color(DIM));
                    } else if !health.stream_watching.is_empty() && health.stream_bonus_pct > 0.0 {
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
                                tr("Bonus not registered for:"),
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
                    } else {
                        ui.label(
                            RichText::new(tr(if health.stream_watching.is_empty() { "No Twitch XP bonus confirmed by the game" } else { "Chat activity reported; XP bonus not confirmed" }))
                                .size(10.0)
                                .color(DIM),
                        );
                    }
                    ui.label(RichText::new(tr("Twitch channels and game-reported XP bonuses are different signals.")).size(9.0).color(DIM));

                    ui.add_space(6.0);

                    ui.horizontal_wrapped(|ui| {
                        for tab in health.tabs.iter().filter(|tab| {
                            tab.kind == "Twitch" || tab.kind == "KICK"
                        }) {
                            let state = if tab.kind == "Twitch" {
                                if tab.low_resource { tr("LOW") } else { tr("not optimized") }
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
                            RichText::new(tr(if health.bridge_snapshot_ok { "No active XP bonus detected" } else { "XP bonus data unavailable" }))
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

        });
}

// Detailed view uses close label/value pairs instead of pushing numbers to the
// far edge of large columns. Preserve full values in the hover tooltip.
fn detail_stat_line(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_sized(
            [108.0, 18.0],
            egui::Label::new(RichText::new(tr(label)).size(10.0).color(DIM)),
        );
        ui.add(
            egui::Label::new(
                RichText::new(compact_text(value, 32)).size(11.0).strong().color(TEXT),
            ).truncate(),
        ).on_hover_text(value);
    });
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

fn automation_badge(ui: &mut egui::Ui, label: &str, value: &str, good: bool) -> egui::Response {
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
                ui.label(RichText::new(compact_text(tr(value), 54)).size(9.0).color(TEXT));
            });
        }).response
}

fn bonus_chip(ui: &mut egui::Ui, text: &str) {
    let display = compact_text(text, 38);
    let measured = ui
        .painter()
        .layout_no_wrap(display.clone(), FontId::proportional(10.0), TEXT)
        .size();

    // Reserve the whole chip width before laying it out in a wrapping row.
    // Otherwise egui can give a chip only the remaining few pixels in the row,
    // causing each character to wrap vertically.
    let max_width = (ui.max_rect().width() - 20.0).max(44.0);
    let chip_width = (measured.x + 28.0).min(max_width);
    let text_width = (chip_width - 20.0).max(24.0);
    let chip_height = measured.y + 14.0;

    ui.allocate_ui_with_layout(
        egui::vec2(chip_width, chip_height),
        Layout::left_to_right(Align::Center),
        |chip_ui| {
            egui::Frame::new()
                .fill(ACCENT.linear_multiply(0.10))
                .stroke(Stroke::new(1.0, ACCENT.linear_multiply(0.28)))
                .corner_radius(7.0)
                .inner_margin(Margin::symmetric(9, 6))
                .show(chip_ui, |content_ui| {
                    content_ui.add_sized(
                        egui::vec2(text_width, measured.y),
                        egui::Label::new(
                            RichText::new(display)
                                .size(10.0)
                                .strong()
                                .color(TEXT),
                        )
                        .truncate(),
                    );
                });
        },
    );
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
                app.panel_tab = PanelTab::Accounts;
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
                let running = app.games.iter()
                    .filter(|game| game.child.is_some() || game.orphaned_pid.is_some())
                    .count();
                let headless = app.games.iter()
                    .filter(|game| (game.child.is_some() || game.orphaned_pid.is_some()) && game.headless)
                    .count();
                let bidi_connected = app.games.iter()
                    .filter(|game| game.monitor.as_ref().is_some_and(|monitor| monitor.health().heartbeat_is_fresh()))
                    .count();
                let native_online = app.games.iter().filter(|game|
                    game.native.as_ref().is_some_and(|native| native.snapshot().phase == "Online")
                ).count();
                let native_twitch_online = app.games.iter().filter(|game|
                    game.native.as_ref().is_some_and(|native| native.twitch_snapshot().phase == "Online")
                ).count();
                let firefox_state = format!("{running} {} · {headless} {}", tr("running"), tr("Hidden Firefox"));
                let bidi_state = format!("{bidi_connected}/{running} {}", tr("Connected"));
                runtime_chip(ui, tr("Firefox"), &firefox_state, if running > 0 { WARN } else { GOOD });
                runtime_chip(ui, tr("PokéIdle"), &format!("{native_online} online"), if native_online > 0 { GOOD } else { MUTED });
                runtime_chip(ui, tr("Twitch"), &format!("{native_twitch_online} online"), if native_twitch_online > 0 { GOOD } else { MUTED });
                runtime_chip(ui, tr("Firefox connection"), &bidi_state, if running == 0 { MUTED } else if bidi_connected == running { GOOD } else { WARN });
                let enabled_profiles = app.accounts.iter().filter(|account| account.enabled).count();
                runtime_chip(
                    ui,
                    tr("Profiles"),
                    &format!("{} {}", enabled_profiles, tr("isolated")),
                    MUTED,
                );
                runtime_chip(ui, tr("Game heartbeat"), &format!("{} s", MONITOR_HEARTBEAT_INTERVAL.as_secs()), MUTED);
                runtime_chip(ui, tr("Game data"), &format!("{} min", MONITOR_POLL_INTERVAL.as_secs() / 60), MUTED);
                runtime_chip(ui, tr("Tab inventory"), &format!("{} min", MONITOR_TAB_INTERVAL.as_secs() / 60), MUTED);
                runtime_chip(ui, tr("Twitch"), tr("Connected in app"), MUTED);
                runtime_chip(ui, tr("UI repaint"), &format!("{} s", UI_REPAINT_INTERVAL.as_secs()), MUTED);
            });
        });

    ui.add_space(12.0);
    ui.collapsing(tr("Recent events"), |ui| {
        if app.events.is_empty() {
            ui.label(RichText::new(tr("No events yet")).color(DIM));
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("controller_event_history")
            .max_height(190.0)
            .show(ui, |ui| {
                for event in &app.events {
                    let age = event.at.elapsed().as_secs();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(format!("{age}s {}", tr("ago")))
                            .size(9.0).color(DIM));
                        ui.label(RichText::new(localize_status(event.message.clone()))
                            .size(10.0).color(if event.is_error { BAD } else { MUTED }));
                    });
                }
            });
    });
}


fn draw_credits(ui: &mut egui::Ui, update_status: &SharedUpdateStatus) {
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

                ui.label(
                    RichText::new(tr("by MOTHblank"))
                        .size(10.0)
                        .color(MUTED),
                );

                // Keep the links in aligned rows. A wrapped run of independent
                // separators and default-sized hyperlinks breaks in narrow sidebars.
                egui::Grid::new("sidebar_credits_links")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.hyperlink_to(
                            RichText::new(tr("Google Play")).size(10.0),
                            "https://play.google.com/store/apps/developer?id=MOTHblank",
                        );
                        ui.hyperlink_to(
                            RichText::new(tr("X")).size(10.0),
                            "https://x.com/MOTHblank",
                        );
                        ui.end_row();

                        ui.hyperlink_to(
                            RichText::new(tr("WhatsApp / Pix")).size(10.0),
                            "https://wa.me/+5537999933376",
                        );
                        ui.hyperlink_to(
                            RichText::new(tr("source code")).size(10.0),
                            "https://github.com/MOTHblank/pokeidle-addons",
                        );
                        ui.end_row();
                    });

                ui.add_space(5.0);
                ui.label(
                    RichText::new(format!("{} {}", tr("Version"), env!("CARGO_PKG_VERSION")))
                        .size(9.0)
                        .color(DIM),
                );

                let latest_version = update_status
                    .lock()
                    .ok()
                    .and_then(|state| state.latest_version.clone());
                if let Some(latest_version) = latest_version
                    .filter(|latest| update::is_newer_version(latest, env!("CARGO_PKG_VERSION")))
                {
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(Color32::from_rgb(57, 255, 20))
                        .corner_radius(5.0)
                        .inner_margin(Margin::symmetric(6, 3))
                        .show(ui, |ui| {
                            ui.hyperlink_to(
                                RichText::new(format!(
                                    "{} · v{}",
                                    tr("Update available"),
                                    latest_version
                                ))
                                .size(10.0)
                                .strong()
                                .color(Color32::from_rgb(12, 38, 10)),
                                update::RELEASES_URL,
                            );
                        });
                }
            });
        });
}


/// Show all four account slots at once; a card never switches or stops another
/// account. All indicators come from the current game and Twitch connection.
fn draw_accounts_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        if ui.selectable_label(app.accounts_section == AccountsSection::Profiles,
            if pt_br() { "Perfis PokéIdle" } else { "PokéIdle profiles" }).clicked() {
            app.accounts_section = AccountsSection::Profiles;
        }
        if ui.selectable_label(app.accounts_section == AccountsSection::Kick,
            "Kick").clicked() {
            app.accounts_section = AccountsSection::Kick;
        }
    });
    ui.add_space(12.0);
    if app.accounts_section == AccountsSection::Kick {
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
    let setup = app.account_setup[index].clone();
    let health = app.games[index].health();
    let running = app.games[index].is_running();
    let game_phase = app.games[index].native.as_ref().map(|session| session.snapshot().phase);
    let twitch_phase = app.games[index].native.as_ref().map(|session| session.twitch_snapshot());
    let game_online = game_phase.as_deref() == Some("Online")
        || (running && health.telemetry_is_fresh() && health.logged_in);
    let twitch_online = twitch_phase.as_ref().is_some_and(|state| state.phase == "Online");
    let connecting = app.native_verifying[index] || app.native_handoff_busy[index]
        || app.cached_native_busy[index] || app.games[index].handoff_closing;

    ui.set_width(width);
    egui::Frame::new().fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, if game_online && twitch_online { GOOD } else { BORDER }))
        .corner_radius(10.0).inner_margin(12.0).show(ui, |ui| {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(profile.label()).size(15.0).strong().color(TEXT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut active = enabled;
                if ui.checkbox(&mut active, tr("Enabled")).changed() {
                    app.accounts[index].enabled = active;
                    changed = true;
                }
            });
        });
        ui.add_space(5.0);
        let response = ui.add(
            egui::TextEdit::singleline(&mut app.accounts[index].name)
                .hint_text(profile.label()).desired_width(ui.available_width())
        );
        if response.changed() { app.account_names_dirty[index] = true; }
        if response.lost_focus() && app.account_names_dirty[index] { changed = true; }
        ui.add_space(6.0);

        let game_label = if game_online { tr("Connected") }
            else if game_phase.as_deref() == Some("NeedsLogin") { tr("Sign-in required") }
            else if game_phase.as_deref() == Some("Reconnecting") { tr("Reconnecting") }
            else if connecting { tr("Connecting") }
            else if running { tr("Waiting for login") }
            else { tr("Not signed in") };
        let twitch_label = if twitch_online { tr("Connected") }
            else if let Some(state) = twitch_phase.as_ref() {
                if state.phase == "NeedsLogin" { tr("Sign-in required") }
                else if state.phase == "Reconnecting" { tr("Reconnecting") }
                else if connecting { tr("Connecting") }
                else { tr(&state.phase) }
            } else if running { tr("Waiting for Twitch sign-in") }
            else { tr("Not verified") };
        ui.horizontal_wrapped(|ui| {
            status_chip(ui, "PokéIdle", game_label, game_online);
            status_chip(ui, "Twitch", twitch_label, twitch_online);
        });
        if let Some(state) = twitch_phase.as_ref() {
            if twitch_online {
                ui.label(RichText::new(tr_dynamic(format!("{} / {} Twitch channels",
                    state.joined, state.desired))).size(10.0).color(DIM));
            }
            if let Some(error) = &state.error {
                ui.label(RichText::new(error).size(10.0).color(WARN));
            }
        }
        if let Some(error) = app.native_error[index].as_deref() {
            ui.label(RichText::new(error).size(10.0).color(WARN));
        }
        if let Some(error) = setup.check_error.as_deref() {
            ui.label(RichText::new(error).size(10.0).color(WARN));
        }

        ui.add_space(7.0);
        ui.horizontal_wrapped(|ui| {
            if running {
                if ui.small_button(tr("Stop")).clicked() { app.stop_one(profile); }
            } else if ui.add_enabled(enabled,
                egui::Button::new(tr("Start account"))).clicked()
            {
                app.launch_one(profile);
            }
            if ui.small_button(tr("Open in Firefox")).clicked() {
                app.open_in_firefox(profile);
            }
            if ui.small_button(tr("Configure automation"))
                .on_hover_text(tr("Open Auto Catch settings for this account")).clicked()
            {
                app.select_workspace_account(index);
                app.automation_section = AutomationSection::Capture;
                app.panel_tab = PanelTab::Automation;
            }
        });

        // Keep game/Twitch profile cards compact; Kick has its own workspace.
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            let kick_running = app.kick_monitors[index].is_some();
            let kick_channels = kick_panel::desired_kick_channels(app, index).len();
            ui.label(RichText::new(format!("Kick · {} {}",
                kick_channels, if pt_br() { "canais" } else { "channels" }))
                .size(10.0).color(if kick_running { GOOD } else { MUTED }));
            if ui.small_button(if pt_br() { "Gerenciar Kick →" }
                               else { "Manage Kick →" }).clicked() {
                app.kick_selected_account = index;
                app.accounts_section = AccountsSection::Kick;
            }
        });

        let show_guide = !setup.setup_complete || app.show_setup_guide[index];
        ui.add_space(6.0);
        if setup.setup_complete &&
            ui.small_button(if show_guide { tr("Hide setup guide") }
                else { tr("Show setup guide") }).clicked()
        {
            app.show_setup_guide[index] = !show_guide;
        }
        if show_guide && enabled {
            egui::Frame::new().fill(PANEL).corner_radius(6.0)
                .inner_margin(10.0).show(ui, |ui| {
                ui.label(RichText::new(tr("Sign-in checklist")).strong().color(TEXT));
                status_chip(ui, "Firefox profile",
                    if setup.profile_ready { tr("Ready") } else { tr("Created on first launch") },
                    setup.profile_ready);
                status_chip(ui, "PokéIdle login", game_label, game_online);
                status_chip(ui, "Twitch login", twitch_label, twitch_online);
                if !game_online {
                    ui.label(RichText::new(tr(
                        "Start the account, then sign in to PokéIdle in the Firefox window."
                    )).size(10.0).color(MUTED));
                } else if !twitch_online {
                    ui.label(RichText::new(tr(
                        "PokéIdle is ready. Sign in to Twitch in the same Firefox profile; both connections are required to finish setup."
                    )).size(10.0).color(MUTED));
                } else {
                    ui.label(RichText::new(tr(
                        "Both logins are connected. Future starts can reuse your saved login."
                    )).size(10.0).color(GOOD));
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button(tr("Twitch sign-in")).clicked() {
                        app.profile_action(profile, ProfileAction::Twitch);
                    }
                    if ui.small_button(tr("Google sign-in")).clicked() {
                        app.profile_action(profile, ProfileAction::GoogleSignIn);
                    }
                });
            });
        }
        ui.collapsing(tr("Login and profile tools"), |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.small_button(tr("Twitch")).clicked() {
                    app.profile_action(profile, ProfileAction::Twitch);
                }
                if ui.small_button(tr("Profile folder")).clicked() {
                    app.profile_action(profile, ProfileAction::Folder);
                }
            });
        });
        if changed {
            let trimmed = app.accounts[index].name.trim();
            app.accounts[index].name = if trimmed.is_empty() {
                profile.label().to_owned()
            } else { trimmed.to_owned() };
            if app.save_account_config() {
                app.account_names_dirty[index] = false;
            }
        }
    });
}

fn status_chip(ui: &mut egui::Ui, name: &str, state: &str, good: bool) -> egui::Response {
    let waiting = [
        tr("Connecting"), tr("Reconnecting"), tr("Sign-in required"),
        tr("Waiting for login"), tr("Waiting for Twitch sign-in"),
    ].contains(&state);
    let color = if good { GOOD } else if waiting { WARN } else { DIM };
    ui.label(
        RichText::new(format!("● {}: {}", tr(name), tr(state)))
            .size(10.0)
            .color(color),
    )
}



fn status_badge(ui: &mut egui::Ui, health: &Health, running: bool) {
    // Loss of browser monitoring does not imply that the game stopped.
    // Keep the state accessible without a constantly growing, alarm-colored warning.
    let no_readings = running && health.state == "Running"
        && health.last_heartbeat_at.is_some()
        && !health.heartbeat_is_fresh();
    let session_blocked = running && health.last_error.as_deref()
        .is_some_and(|error| error.contains("Maximum number of active sessions"));
    let (label, color) = if !running {
        ("STOPPED", DIM)
    } else if session_blocked {
        ("FIREFOX CONNECTION BLOCKED", WARN)
    } else if health.state == "Reconnecting" {
        ("RECONNECTING", WARN)
    } else if no_readings {
        ("UPDATES DELAYED", DIM)
    } else if health.telemetry_is_fresh()
        && health.heartbeat_is_fresh()
        && health.logged_in
        && health.activity == "Hunting" {
        ("HUNTING", GOOD)
    } else if health.state == "Running" && health.logged_in {
        ("ONLINE", GOOD)
    } else if health.state == "Existing Firefox" {
        ("EXISTING", WARN)
    } else if health.state == "Connecting" || health.state == "Reconnecting" {
        ("CONNECTING", WARN)
    } else if health.state == "Running" && !health.logged_in {
        ("LOGIN", WARN)
    } else {
        ("ERROR", BAD)
    };

    let response = ui.label(
        RichText::new(format!("● {}", tr(label)))
            .size(12.0)
            .strong()
            .color(color),
    );
    if no_readings || session_blocked || health.last_error.is_some() {
        response.on_hover_ui(|ui| {
            if session_blocked {
                ui.label(tr("Firefox isn't responding for this account. Restart Firefox to reconnect."));
            } else {
                ui.label(tr("Game updates are temporarily unavailable. Firefox may still be running."));
            }
            ui.label(RichText::new(tr("The displayed metrics are from the last successful reading.")).color(DIM));
            if let Some(error) = &health.last_error {
                ui.label(RichText::new(error).color(DIM));
            }
        });
    }
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
        [ui.available_width().max(80.0), 34.0],
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
            (TextStyle::Small, FontId::proportional(11.0)),
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
        style.visuals.widgets.hovered.bg_stroke =
            Stroke::new(1.0, ACCENT.linear_multiply(0.40));
        style.visuals.widgets.active.bg_stroke =
            Stroke::new(1.0, ACCENT.linear_multiply(0.65));
        style.visuals.selection.bg_fill = ACCENT.linear_multiply(0.30);
        style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    });
}

pub fn run() -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Moth · PokéIdle")
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([800.0, 560.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Moth · PokéIdle",
        options,
        Box::new(|cc| Ok(Box::new(ControllerApp::new(cc)))),
    )
    .map_err(|error| format!("could not start controller UI: {error}"))
}

fn format_pct(value: f32) -> String {
    if value.fract().abs() < 0.01 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
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
