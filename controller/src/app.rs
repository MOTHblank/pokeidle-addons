use crate::accounts;
use crate::config::{AccountConfig, Config, GameProfile};
use crate::firefox;
use crate::kick::KickStream;
use crate::logging;
use crate::native_background::NativeSession;
use crate::native_gameplay::GameCommand;
use crate::native_hunts::{self, HuntView};
use crate::native_atlas::{self, Reference as AtlasReference};
use crate::monitor::{
    Health, MonitorHandle, MonitorPace, MONITOR_HEARTBEAT_INTERVAL,
    MONITOR_POLL_INTERVAL, MONITOR_TAB_INTERVAL,
};
use crate::update::{self, SharedUpdateStatus};
use serde_json::json;
use eframe::egui::{self, Align, Color32, FontId, Layout, Margin, RichText, Stroke, TextStyle};
use std::collections::{BTreeSet, VecDeque};
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
        controller_is_background, monitor_pace_for_slot, monitor_repair_available, MonitorPace,
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
        ("could not list addons from GitHub: ", "não foi possível listar addons no GitHub: "),
        ("GitHub addon list unavailable", "lista de addons do GitHub indisponível"),
        ("could not start addon installer", "não foi possível iniciar o instalador de addons"),
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
        "Idle warning: no EXP or Pokéballs used in 5 minutes" => "Aviso de inatividade: sem EXP ou Pokébolas usadas por 5 minutos",
        "Overview" => "Visão geral",
        "Hunts · Atlas" => "Hunts · Atlas",
        "Hunt Atlas" => "Hunt Atlas",
        "Account" => "Conta",
        "Native server catalog · no userscripts" => "Catálogo nativo do servidor · sem userscripts",
        "Start the account to load its available hunts." => "Inicie a conta para carregar as hunts disponíveis.",
        "The native game socket supplies the hunt catalog and active Pokémon after server authentication." => "O socket Rust recebe hunts e Pokémon diretamente do servidor.",
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
        "RUNTIME" => "EXECUÇÃO",
        "Firefox" => "Firefox",
        "Monitor" => "Monitor",
        "Poll" => "Consulta",
        "HEADLESS" => "OCULTO",
        "EXISTING" => "EXISTENTE",
        "VISIBLE" => "VISÍVEL",
        "MIXED" => "MISTO",
        "Rust rewrite · Windows" => "Reescrita em Rust · Windows",
        "Rust rewrite · Linux" => "Reescrita em Rust · Linux",
        "Instances" => "Instâncias",
        "Live state for both isolated Firefox profiles" => "Estado atual dos dois perfis isolados do Firefox",
        "Show Firefox" => "Mostrar Firefox",
        "Hide Firefox" => "Ocultar Firefox",
        "Restart visible" => "Reiniciar visível",
        "Restart headless" => "Reiniciar oculto",
        "Restarts Firefox in the selected mode; the game and streams will briefly disconnect." => "Reinicia o Firefox no modo escolhido; o jogo e as streams serão desconectados temporariamente.",
        "NO READINGS" => "SEM LEITURAS",
        "CONNECTION UNCONFIRMED" => "CONEXÃO NÃO CONFIRMADA",
        "BIDI SESSION BLOCKED" => "SESSÃO BIDI BLOQUEADA",
        "Repair monitor (restart Firefox)" => "Reparar monitor (reiniciar Firefox)",
        "Restarts this account's Firefox to release a stuck BiDi session. The game briefly disconnects." => "Reinicia o Firefox desta conta para liberar uma sessão BiDi travada. O jogo ficará desconectado brevemente.",
        "Firefox is refusing a new BiDi session; use Repair monitor to restart only this account." => "O Firefox está recusando outra sessão BiDi. Use Reparar monitor para reiniciar apenas esta conta.",
        "The controller has stopped receiving telemetry. Firefox and addons may still be running." => "O controlador deixou de receber telemetria. O Firefox e os addons podem continuar funcionando.",
        "The displayed metrics are from the last successful reading." => "Os dados exibidos são da última leitura bem-sucedida.",
        "Installer already running" => "O instalador já está em execução",
        "Checking addons on GitHub…" => "Consultando addons no GitHub…",
        "Opening addon installers…" => "Abrindo instaladores de addons…",
        "Installer details" => "Detalhes da instalação",
        "Addon versions" => "Versões dos addons",
        "Manage Addons" => "Gerenciar addons",
        "Manage addons" => "Gerenciar addons",
        "Check now" => "Verificar agora",
        "Checking for updates…" => "Verificando atualizações…",
        "Checking GitHub addon versions…" => "Verificando versões dos addons no GitHub…",
        "Update check requested" => "Verificação de atualizações solicitada",
        "Could not request addon update check" => "Não foi possível solicitar a verificação de addons",
        "Repair selected" => "Reparar selecionados",
        "Install all (first setup)" => "Instalar todos (primeira configuração)",
        "Choose the scripts to reinstall." => "Selecione os scripts que deseja reinstalar.",
        "No addons selected" => "Nenhum addon selecionado",
        "Switch to visible Firefox before opening installers." => "Mude para o Firefox visível antes de abrir os instaladores.",
        "Select only the affected scripts. Installers open in this Firefox profile; confirm installation in Violentmonkey." => "Selecione somente os scripts afetados. Os instaladores abrem neste perfil do Firefox; confirme a instalação no Violentmonkey.",
        "This only opens installers, it does not confirm installation." => "Esta ação apenas abre instaladores, não confirma a instalação.",
        "Opens installer links through Firefox; confirm installation in Violentmonkey." => "Abre os links de instalação pelo Firefox; confirme a instalação no Violentmonkey.",
        "Running" => "Em execução",
        "Not detected in game" => "Não detectado no jogo",
        "Not installed or inactive" => "Não instalado ou inativo",
        "detected" => "detectados",
        "Running, version not reported" => "Em execução, versão não informada",
        "Not checked yet" => "Ainda não verificado",
        "Up to date" => "Atualizado",
        "Setup needed" => "Configuração necessária",
        "addons need attention" => "addons precisam de atenção",
        "addon needs attention" => "addon precisa de atenção",
        "Initial setup" => "Configuração inicial",
        "Complete these steps once for this profile." => "Conclua estas etapas uma vez para este perfil.",
        "Firefox profile created" => "Perfil do Firefox criado",
        "Violentmonkey installed" => "Violentmonkey instalado",
        "Game signed in" => "Jogo conectado",
        "Scripts detected" => "Scripts detectados",
        "Finish setup" => "Concluir configuração",
        "You can finish with only the addons you want. Unverified steps are not automatically considered complete." => "Você pode concluir apenas com os addons que deseja. Etapas não verificadas não são consideradas concluídas automaticamente.",
        "Show setup guide" => "Mostrar guia de configuração",
        "Hide setup guide" => "Ocultar guia de configuração",
        "Setup completed" => "Configuração concluída",
        "Could not save setup progress" => "Não foi possível salvar o progresso da configuração",
        "Install Violentmonkey before completing setup." => "Instale o Violentmonkey antes de concluir a configuração.",
        "Live addon status requires the game to be running and monitored." => "O status dos addons exige que o jogo esteja em execução e monitorado.",
        "Open Account Manager to resolve addon problems" => "Abra o Gerenciador de contas para resolver problemas dos addons",
        "Reload the game only when you choose; an automatic reload could interrupt gameplay." => "Recarregue o jogo somente quando desejar; uma recarga automática poderia interromper a partida.",
        "Updates" => "Atualizações",
        "outdated" => "desatualizados",
        "unverified" => "não verificados",
        "Current" => "Atual",
        "Version unknown" => "Versão desconhecida",
        "Checking GitHub versions…" => "Consultando versões no GitHub…",
        "GitHub version check unavailable" => "Verificação de versões no GitHub indisponível",
        "Launch an account to inspect running script versions." => "Inicie uma conta para verificar as versões dos scripts em execução.",
        "Violentmonkey handles automatic updates. Reload the game after it installs a new version." => "O Violentmonkey realiza atualizações automáticas. Recarregue o jogo após instalar uma nova versão.",
        "Older addons do not report their version; allow Violentmonkey to update them." => "Addons antigos não informam a versão; permita que o Violentmonkey os atualize.",
        "Recent events" => "Eventos recentes",
        "No events yet" => "Nenhum evento ainda",
        "running" => "em execução",
        "First scan 30s · 5 min" => "Primeiro scan em 30 s · a cada 5 min",
        "Game data" => "Dados do jogo",
        "Game heartbeat" => "Sinal do jogo",
        "Tab inventory" => "Inventário de abas",
        "Tabs checked" => "Abas verificadas",
        "min ago" => "min atrás",
        "Idle detection unavailable without Controller Bridge telemetry" => "Detecção de inatividade indisponível sem telemetria do Controller Bridge",
        "Profile folder exists" => "Pasta do perfil existente",
        "Registered" => "Registrada",
        "Check failed" => "Falha na verificação",
        "Stop" => "Parar",
        "Launch" => "Iniciar",
        "Start native" => "Iniciar nativo",
        "Stop login" => "Parar login",
        "COMPACT OVERVIEW" => "VISÃO COMPACTA",
        "Select an account to inspect its full statistics" => "Selecione uma conta para ver todas as estatísticas",
        "Show details" => "Ver detalhes",
        "Hide details" => "Ocultar detalhes",
        "Details for" => "Detalhes de",
        "Current hunt" => "Caça atual",
        "Active Pokémon" => "Pokémon ativo",
        "Trainer XP" => "XP do treinador",
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
        "Bridge data unavailable" => "Dados da ponte indisponíveis",
        "Twitch XP bonus not checked (Controller Bridge data unavailable)" => "Bônus de XP da Twitch não verificado (dados do Controller Bridge indisponíveis)",
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
        "Google sign-in" => "Login do Google",
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
        "Twitch uses this Firefox profile for its session. All actions target this account." => "A Twitch usa este perfil do Firefox para sua sessão. Todas as ações usam esta conta.",
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
        "Native game + Twitch IRC · browserless" => "Jogo + Twitch IRC nativos · sem navegador",
        "Native" => "Nativo",
        "Native error" => "Erro nativo",
        "Verifying native" => "Verificando Rust",
        "Reading handshake" => "Lendo sessão",
        "Firefox login" => "Login Firefox",
        "Firefox is temporary; Rust takes over after game and Twitch authentication." =>
            "O Firefox é temporário; o Rust assume após autenticar jogo e Twitch.",
        "Game actions" => "Ações do jogo",
        "Pokémon Center" => "Centro Pokémon",
        "Return to hunt" => "Voltar à caça",
        "Enable" => "Ativar",
        "Disable" => "Desativar",
        "Change native Auto Catch for this account" =>
            "Alterar a captura automática nativa desta conta",
        "pending" => "pendentes",
        "throws" => "bolas",
        "Direct IRC" => "IRC direto",
        "Native restock setting; saved without Firefox" =>
            "Reposição nativa: ajuste salvo sem Firefox",
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
        "Reconnecting" => "Reconectando",
        "Rechecking last order" => "Conferindo última compra",
        "ordered" => "pedidos",
        "confirmed" => "confirmados",
        "No restock rules imported for this account" =>
            "Nenhuma regra de reposição importada nesta conta",
        "Native Restock needs ball/item IDs and thresholds; browser settings are imported once during authentication." =>
            "A reposição nativa precisa de IDs e limites; as configurações do navegador são importadas uma vez durante a autenticação.",
        "Authenticating in Firefox · automatic native game + Twitch handoff" =>
            "Autenticando no Firefox · transferência nativa do jogo + Twitch automática",
        "Reading native handshake…" => "Lendo handshake nativo…",
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
    native: Option<NativeSession>,
    handoff_closing: bool,
    child: Option<Child>,
    monitor: Option<MonitorHandle>,
    headless: bool,
    orphaned_pid: Option<u32>,
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
            health.state = "Switching to native".into();
            return health;
        }
        if let Some(native) = self.native.as_ref() {
            let state = native.snapshot();
            let mut health = Health::default();
            health.state = if state.phase == "Online" { "Running".into() }
                else { state.phase.clone() };
            health.activity = format!("Native background · {} kills observed", state.battle_kills);
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
            health.hunt = state.hunt_slug.unwrap_or_default();
            health.hunt_slug = health.hunt.clone();
            health.player_level = state.trainer_level.unwrap_or_default();
            health.player_total_xp = state.trainer_xp;
            health.player_xp = state.trainer_xp.map(|xp| xp.to_string()).unwrap_or_default();
            health.gold = state.gold.unwrap_or_default();
            health.orbs = state.orbs.unwrap_or_default();
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
    violentmonkey_installed: bool,
    setup_complete: bool,
    check_error: Option<String>,
}

struct UiEvent {
    at: Instant,
    message: String,
    is_error: bool,
}

enum AddonInstallEvent {
    Progress { index: usize, message: String },
    Finished { index: usize, result: Result<usize, String> },
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

                    std::thread::sleep(Duration::from_millis(30));
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
    headless: bool,
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
enum PanelTab { Overview, Hunts }
#[derive(Clone, Copy, PartialEq, Eq)]
enum HuntRanking { TrainerXp, PokemonXp, Gold }

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
    hunt_account: usize,
    hunt_query: String,
    hunt_region: String,
    hunt_ranking: HuntRanking,
    hunt_reference: Option<Arc<AtlasReference>>,
    hunt_reference_error: Option<String>,
    hunt_reference_busy: bool,
    hunt_reference_tx: Sender<Result<AtlasReference,String>>,
    hunt_reference_rx: Receiver<Result<AtlasReference,String>>,
    show_accounts: bool,
    accounts: [AccountConfig; 4],
    status: String,
    status_error: bool,
    events: VecDeque<UiEvent>,
    addon_install_tx: Sender<AddonInstallEvent>,
    addon_install_rx: Receiver<AddonInstallEvent>,
    addon_install_busy: [bool; 4],
    addon_install_details: [Vec<String>; 4],
    addon_selection: [BTreeSet<String>; 4],
    addon_selection_initialized: [bool; 4],
    addon_manager_open: [bool; 4],
    addon_focus: Option<usize>,
    show_setup_guide: [bool; 4],
    account_setup: [AccountSetup; 4],
    account_setup_checked_at: Option<Instant>,
    tab_url_input: [String; 4],
    tab_url_open: [bool; 4],
    idle_progress: [IdleProgressTracker; 4],
    xp_rates: [XpRateTracker; 4],
    selected_details: Option<usize>,
    update_status: SharedUpdateStatus,
    addon_version_status: update::SharedAddonVersionStatus,
    addon_refresh_tx: Sender<()>,
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
        let (addon_version_status, addon_refresh_tx) = update::start_addon_version_check();
        let (addon_install_tx, addon_install_rx) = mpsc::channel();
        let (native_handoff_tx, native_handoff_rx) = mpsc::channel();
        let (cached_native_tx, cached_native_rx) = mpsc::channel();
        let (native_closed_tx, native_closed_rx) = mpsc::channel();
        let (native_verified_tx, native_verified_rx) = mpsc::channel();
        let (hunt_reference_tx, hunt_reference_rx) = mpsc::channel();

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
            hunt_account: 0,
            hunt_query: String::new(),
            hunt_region: String::new(),
            hunt_ranking: HuntRanking::TrainerXp,
            hunt_reference: None,
            hunt_reference_error: None,
            hunt_reference_busy: false,
            hunt_reference_tx,
            hunt_reference_rx,
            show_accounts: false,
            status: "Ready · launch only the profiles you need".to_string(),
            status_error: false,
            events: VecDeque::new(),
            addon_install_tx,
            addon_install_rx,
            addon_install_busy: [false; 4],
            addon_install_details: std::array::from_fn(|_| Vec::new()),
            addon_selection: std::array::from_fn(|_| BTreeSet::new()),
            addon_selection_initialized: [false; 4],
            addon_manager_open: [false; 4],
            addon_focus: None,
            show_setup_guide: [false; 4],
            account_setup: std::array::from_fn(|_| AccountSetup::default()),
            account_setup_checked_at: None,
            tab_url_input: std::array::from_fn(|_| String::new()),
            tab_url_open: [false; 4],
            idle_progress: std::array::from_fn(|_| IdleProgressTracker::default()),
            xp_rates: std::array::from_fn(|_| XpRateTracker::default()),
            selected_details: None,
            update_status,
            addon_version_status,
            addon_refresh_tx,
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
        // A visible but unfocused window should be just as inexpensive as
        // one hidden in the tray. Focus changes arrive as native UI events.
        let focused = ctx.input(|input| input.viewport().focused);
        controller_is_background(self.hidden_in_tray(), focused)
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

            let (vm_installed, check_error) = match accounts::violentmonkey_installed(profile) {
                Ok(value) => (value, None),
                Err(error) => (false, Some(error)),
            };

            let mut setup_complete = accounts::setup_complete(profile).unwrap_or(false);
            // Mark fully verified profiles automatically, without treating
            // an absent script marker as evidence of an installation.
            if !setup_complete && vm_installed {
                let health = self.games[index].health();
                if health.telemetry_is_fresh() && health.logged_in
                    && health.addon_ok > 0 && health.addon_missing.is_empty()
                {
                    match accounts::complete_setup(profile) {
                        Ok(()) => setup_complete = true,
                        Err(error) => logging::warn(&error),
                    }
                }
            }
            self.account_setup[index] = AccountSetup {
                profile_ready: config.profile_dir.exists(),
                violentmonkey_installed: vm_installed,
                setup_complete,
                check_error,
            };
        }

        self.account_setup_checked_at = Some(Instant::now());
    }

    fn refresh_processes(&mut self) {
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
                self.set_status("No Moth-owned Firefox process for verified native handoff", true);
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
                            "{} native handoff canceled by Stop", self.account_name(profile)), false);
                        continue;
                    }
                    logging::info(&format!(
                        "{} starting native game + Twitch IRC workers", self.account_name(profile)));
                    self.games[index].native = Some(event.native);
                    self.require_browser_login[index] = false;
                    let cache_note = event.cache_warning
                        .map(|warning| format!(" · secure cache unavailable: {warning}"))
                        .unwrap_or_default();
                    self.set_status(format!(
                        "{} native game + Twitch IRC authenticated and Firefox closed{cache_note}",
                        self.account_name(profile)), false);
                }
                Err(error) => {
                    // Native game is already authenticated; Firefox's game
                    // tab was closed. Retain native ownership and the remaining
                    // Firefox handle for a later explicit cleanup attempt.
                    self.games[index].native = Some(event.native);
                    self.games[index].child = event.child;
                    self.games[index].orphaned_pid = event.orphaned_pid;
                    self.games[index].monitor = event.monitor;
                    self.set_status(format!(
                        "{} native authenticated, but Firefox cleanup needs attention: {error}",
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
                self.launch_firefox_bootstrap(profile, event.headless);
                continue;
            }
            if self.require_browser_login[event.index] {
                self.launch_firefox_bootstrap(profile, event.headless);
                continue;
            }
            match event.result {
                Ok(Some(session)) => {
                    self.games[event.index].native = Some(session);
                    self.set_status(format!(
                        "{}: native game + Twitch IRC started from protected credentials (no Firefox)",
                        self.account_name(profile)), false);
                },
                Ok(None) => {
                    self.require_browser_login[event.index] = true;
                    self.set_status(format!(
                        "{}: first-time native authentication needed",
                        self.account_name(profile)), false);
                    self.launch_firefox_bootstrap(profile, event.headless);
                }
                Err(error) => {
                    self.require_browser_login[event.index] = true;
                    self.set_status(format!(
                        "{} secure native credentials unavailable: {error}",
                        self.account_name(profile)), true);
                    self.launch_firefox_bootstrap(profile, event.headless);
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
                    let is_bootstrap = error.contains("reloaded")
                        || error.contains("waiting") || error.contains("pending")
                        || error.contains("temporary Twitch")
                        || error.contains("hook installed")
                        || error.contains("login has not produced");
                    if !is_bootstrap {
                        self.native_error[event.index] = Some(error.clone());
                    }
                    self.set_status(format!(
                        "{}: {}: {error}",
                        self.account_name(profile),
                        if is_bootstrap { "native login" } else { "native handoff error" }
                    ), !is_bootstrap);
                },
                Ok(bootstrap) => {
                    // Never close Firefox without a usable authenticated hello
                    // and cookies, or if the slot changed while capture ran.
                    if self.games[event.index].native.is_some()
                        || !self.games[event.index].is_running()
                        || self.games[event.index].monitor.is_none()
                    {
                        self.set_status("Native handoff canceled: browser ownership changed", true);
                        continue;
                    }
                    let validated = crate::native_background::validate_bootstrap(&bootstrap);
                    if let Err(error) = validated {
                        self.native_error[event.index] = Some(error.clone());
                        self.set_status(format!("Native handoff rejected: {error}"), true);
                        continue;
                    }
                    // Stage 1: close game TAB and prove Rust is authenticated,
                    // leaving Firefox alive for recovery. Stage 2, above, closes
                    // Firefox only after both native servers welcomed us.
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
                        "{} validating game+Twitch native transport before browser close",
                        self.account_name(profile)));
                    let spawned = std::thread::Builder::new()
                        .name(format!("moth-native-handoff-{}", profile.name()))
                        .spawn(move || {
                            let mut attempted_game_close = false;
                            // A panic in TLS setup previously destroyed the
                            // sender without clearing native_verifying, leaving
                            // Firefox alive but its PokéIdle tab closed.
                            let result = std::panic::catch_unwind(
                                std::panic::AssertUnwindSafe(|| {
                                    crate::native_tls::ensure()?;
                                    attempted_game_close = true;
                                    monitor.close_game_for_handoff(Duration::from_secs(18))?;
                                    NativeSession::start_verified(bootstrap)
                                })
                            ).unwrap_or_else(|_| {
                                Err("Native verification thread panicked; see sanitized Rust panic log".to_owned())
                            });
                            let result = match (result, attempted_game_close) {
                                (Err(error), true) => {
                                    match monitor.restore_game_after_failed_handoff(
                                        Duration::from_secs(20)
                                    ) {
                                        Ok(()) => Err(format!("{error}; game tab reopened")),
                                        Err(recovery) => {
                                            crate::logging::error(&format!(
                                                "Native verification recovery failed: {recovery}"
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
                            "Could not spawn native verification worker: {error}"
                        ));
                        self.set_status(format!(
                            "{}: could not begin native handoff verification: {error}",
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
                recover.push((slot.profile, slot.headless));
            }
        }
        for (profile, headless) in recover {
            self.require_browser_login[profile.index()] = true;
            self.set_status(format!(
                "{} native authentication expired; opening Firefox for renewal",
                self.account_name(profile)), true);
            self.launch_firefox_bootstrap(profile, headless);
        }
        // Drain worker results on the UI thread. Never block egui on GitHub,
        // curl, or Firefox installer tabs.
        while let Ok(event) = self.addon_install_rx.try_recv() {
            match event {
                AddonInstallEvent::Progress { index, message } => {
                    self.addon_install_details[index].push(message.clone());
                    self.set_status(
                        format!("{} · {}", self.accounts[index].name, message), false,
                    );
                }
                AddonInstallEvent::Finished { index, result } => {
                    self.addon_install_busy[index] = false;
                    match result {
                        Ok(opened) => self.set_status(
                            format!("{} · {opened} {}", self.accounts[index].name, tr("addon installers opened")),
                            false,
                        ),
                        Err(error) => {
                            self.addon_install_details[index].push(error.clone());
                            self.set_status(format!("{} · {error}", self.accounts[index].name), true);
                        }
                    }
                }
            }
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
        // Native is the ONLY normal runtime on this experiment branch.
        // Firefox starts solely to restore authenticated game+IRC handshakes
        // and is automatically retired once both are captured.
        for profile in GameProfile::ALL {
            let index = profile.index();
            if !self.accounts[index].enabled || self.native_handoff_busy[index] ||
               self.games[index].native.is_some() || self.native_verifying[index] ||
               self.games[index].handoff_closing || self.games[index].monitor.is_none() {
                continue;
            }
            let health = self.games[index].health();
            if !(health.telemetry_is_fresh() && health.logged_in &&
                 health.bridge_connected) {
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

    fn open_addon_manager(&mut self, profile: GameProfile) {
        let index = profile.index();
        self.show_accounts = true;
        self.addon_manager_open[index] = true;
        self.addon_focus = Some(index);
    }

    fn request_addon_refresh(&mut self) {
        match self.addon_refresh_tx.send(()) {
            Ok(()) => self.set_status(tr("Update check requested"), false),
            Err(error) => self.set_status(
                format!("{}: {error}", tr("Could not request addon update check")), true
            ),
        }
    }

    fn start_addon_install(&mut self, profile: GameProfile, selected: Option<Vec<String>>) {
        let index = profile.index();
        if self.addon_install_busy[index] {
            self.set_status(format!("{} · {}", self.account_name(profile), tr("Installer already running")), true);
            return;
        }
        self.addon_install_busy[index] = true;
        self.addon_install_details[index].clear();
        self.set_status(format!("{} · {}", self.account_name(profile), tr("Checking addons on GitHub…")), false);

        let tx = self.addon_install_tx.clone();
        let spawned = std::thread::Builder::new()
            .name(format!("moth-addons-{}", profile.label()))
            .spawn(move || {
                let progress_tx = tx.clone();
                let result = accounts::open_addons(
                    profile, selected.as_deref(),
                    |message| { let _ = progress_tx.send(AddonInstallEvent::Progress { index, message }); },
                );
                let _ = tx.send(AddonInstallEvent::Finished { index, result });
            });
        if let Err(error) = spawned {
            self.addon_install_busy[index] = false;
            self.set_status(format!("{}: {error}", self.account_name(profile)), true);
        }
    }

    fn start_native_handoff(&mut self, profile: GameProfile) {
        let index = profile.index();
        if self.native_handoff_busy[index] { return; }
        let Some(monitor) = self.games[index].monitor.clone() else {
            self.set_status("Start a Firefox game session before testing native handoff", true);
            return;
        };
        if !self.games[index].health().logged_in {
            self.set_status("Log into PokéIdle before testing native handoff", true);
            return;
        }
        self.native_handoff_busy[index] = true;
        self.native_handoff_last_attempt[index] = Some(Instant::now());
        logging::info(&format!(
            "{} capturing native game + Twitch IRC session via Firefox BiDi",
            self.account_name(profile)));
        self.set_status(format!("{}: transferring game + Twitch IRC authentication to Rust",
            self.account_name(profile)), false);
        let tx = self.native_handoff_tx.clone();
        let result = std::thread::Builder::new()
            .name(format!("moth-handoff-{}", profile.name()))
            .spawn(move || {
                let result = monitor.native_bootstrap(Duration::from_secs(18));
                // Keep browser auth in memory, but cache *valid* credentials
                // under the OS user account for future entirely native starts.
                // Secret Service / DPAPI I/O stays outside the egui thread.
                let mut cache_warning = None;
                if let Ok(value) = &result {
                    if crate::native_background::validate_bootstrap(value).is_ok() {
                        if let Err(error) = crate::native_auth::save(profile, value) {
                            cache_warning = Some(error);
                        }
                    }
                }
                let _ = tx.send(NativeHandoffEvent { index, result, cache_warning });
            });
        if let Err(error) = result {
            self.native_handoff_busy[index] = false;
            self.set_status(format!("Could not start native handoff: {error}"), true);
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
        if self.require_browser_login[index] {
            self.launch_firefox_bootstrap(profile, headless);
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
                                    NativeSession::start_verified(bootstrap.clone())
                                        .or_else(|error| {
                                            if error.contains("AUTH_REQUIRED:") ||
                                                error.contains("rejected the captured OAuth") {
                                                Err(error)
                                            } else {
                                                NativeSession::start(bootstrap)
                                            }
                                        })
                                }).transpose())
                        })
                    })
                ).unwrap_or_else(|_| {
                    Err("Native cache verification panicked; see sanitized controller log".into())
                });
                let _ = tx.send(CachedNativeEvent {
                    index, headless, existing_firefox, result
                });
            });
        if let Err(error) = started {
            self.cached_native_busy[index] = false;
            self.require_browser_login[index] = true;
            self.set_status(format!("Protected native cache could not start: {error}"), true);
            self.launch_firefox_bootstrap(profile, headless);
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
                    "{}: reusing existing Firefox authentication (PID {}) before native handoff",
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

        if self.games[index].orphaned_pid.is_some() {
            self.set_status(
                format!("{}: Close the existing Firefox profile before switching modes.", self.account_name(profile)),
                true,
            );
            return;
        }

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

    fn repair_monitor(&mut self, profile: GameProfile) {
        // An orphaned Firefox BiDi session cannot be reclaimed by a new
        // session.new. Only an explicit user action restarts this profile;
        // never interrupt a running game automatically on telemetry failure.
        let index = Self::game_index(profile);
        let headless = self.games[index].headless;
        logging::warn(&format!(
            "{} repairing an exhausted BiDi session by user-requested Firefox restart",
            self.account_name(profile)
        ));
        if self.stop_one(profile) {
            self.launch_one_mode(profile, headless);
        }
    }

    fn stop_one(&mut self, profile: GameProfile) -> bool {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);
        if self.games[index].handoff_closing {
            // The dedicated shutdown worker already owns and is closing the
            // Firefox process. Do not start native mode after explicit Stop.
            self.native_close_cancelled[index] = true;
            self.set_status(format!(
                "{}: stopping pending native handoff", account_name), false);
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
                self.set_status(format!("{} native game and Twitch stopped.", account_name), false);
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
        } else if let Some(pid) = slot.orphaned_pid {
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
            self.set_status("Launch enabled requested · native game and Twitch starting; Firefox only if login needed", false);
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
            self.set_status("All native game/Twitch sessions and Firefox authentication windows stopped.", false);
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
        // "Open Game" uses the controller-owned visible Firefox process so it
        // remains interactive and is tracked by the dashboard's process monitor.
        if let ProfileAction::Game = action {
            self.set_browser_mode(profile, false);
            return;
        }

        let result = match action {
            ProfileAction::Game => unreachable!("game action handled above"),
            ProfileAction::GoogleSignIn => {
                self.open_game_for_google_sign_in(profile);
                return;
            }
            ProfileAction::Twitch => accounts::open_login(profile, accounts::TWITCH_LOGIN, "Twitch"),
            ProfileAction::InstallViolentmonkey => accounts::open_violentmonkey(profile),
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
        if self.account_setup_checked_at
            .map(|when| when.elapsed() >= Duration::from_secs(5 * 60))
            .unwrap_or(true)
        {
            self.refresh_account_setup();
        }

        ui.ctx().request_repaint_after(
            if self.controller_in_background(ui.ctx()) {
                UI_BACKGROUND_REPAINT_INTERVAL
            } else {
                UI_REPAINT_INTERVAL
            }
        );

        draw_sidebar(self, ui);

        egui::Panel::top("header")
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(24, 18)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr(match self.panel_tab {
                            PanelTab::Overview => "Overview",
                            PanelTab::Hunts => "Hunt Atlas",
                        }))
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

                        #[cfg(windows)]
                        {
                            ui.add_space(8.0);

                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(tr("MINIMIZE TO TRAY"))
                                        .size(12.0)
                                        .strong()
                                        .color(TEXT),
                                )
                                .fill(PANEL)
                                .stroke(Stroke::new(1.0, BORDER))
                                .corner_radius(8.0),
                            )
                            .on_hover_text(tr("Hide this window in the system tray; click the tray icon to restore it"))
                            .clicked()
                        {
                            #[cfg(windows)]
                            {
                                if self.tray.is_some() {
                                    self.tray_hidden.store(true, Ordering::Relaxed);
                                    ui.ctx().send_viewport_cmd(
                                        egui::ViewportCommand::Visible(false),
                                    );
                                    logging::info("controller window hidden to system tray");
                                } else {
                                    self.set_status(
                                        tr("System tray is unavailable; the window cannot be hidden.").to_string(),
                                        true,
                                    );
                                }
                            }
                        }
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
                    let localized = localize_status(self.status.clone());
                    ui.add(egui::Label::new(
                        RichText::new(&localized)
                            .size(12.0)
                            .color(if self.status_error { TEXT } else { MUTED }),
                    ).truncate()).on_hover_text(localized);
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(Margin::symmetric(24, 16)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.panel_tab, PanelTab::Overview, tr("Overview"));
                    ui.selectable_value(&mut self.panel_tab, PanelTab::Hunts, tr("Hunts · Atlas"));
                });
                ui.add_space(11.0);
                match self.panel_tab {
                    PanelTab::Overview => draw_instance_section(self, ui),
                    PanelTab::Hunts => draw_hunt_atlas_panel(self, ui),
                }
            });

        if self.show_accounts {
            draw_accounts_window(self, ui.ctx());
        }
        // Apply after widgets so Show/Hide Details and tray hiding change
        // polling in the same frame. Alt-tab also switches every account to
        // the cheaper background pace; refocusing restores the open panel.
        self.sync_monitor_paces(self.controller_in_background(ui.ctx()));
    }
}

#[derive(Clone, Copy)]
enum ProfileAction {
    Game,
    GoogleSignIn,
    Twitch,
    InstallViolentmonkey,
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
                app.panel_tab = PanelTab::Overview;
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
                    let active: Vec<_> = app.games.iter()
                        .filter(|game| game.child.is_some() || game.orphaned_pid.is_some())
                        .collect();
                    let mode = if active.is_empty() {
                        tr("STOPPED")
                    } else if active.iter().all(|game| game.headless) {
                        tr("HEADLESS")
                    } else if active.iter().all(|game| !game.headless) {
                        tr("VISIBLE")
                    } else {
                        tr("MIXED")
                    };
                    let connected = active.iter()
                        .filter(|game| game.monitor.as_ref().is_some_and(|monitor| monitor.health().heartbeat_is_fresh()))
                        .count();
                    runtime_row(ui, "Firefox", mode, if active.is_empty() { MUTED } else { GOOD });
                    runtime_row(ui, "Monitor", &format!("{connected}/{} {}", active.len(), tr("Connected")), if connected == active.len() && !active.is_empty() { GOOD } else { MUTED });
                });

            ui.add_space(12.0);
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.add_space(6.0);
                draw_credits(ui, &app.update_status);
                ui.add_space(10.0);
                ui.label(
                    RichText::new(if cfg!(windows) { tr("Rust rewrite · Windows") } else { tr("Rust rewrite · Linux") })
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
        ui.label(
            RichText::new(tr("COMPACT OVERVIEW"))
                .font(FontId::proportional(17.0))
                .strong()
                .color(TEXT),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(tr("Select an account to inspect its full statistics"))
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
                // Two compact cards fit at the app's default 1120px window
                // without squeezing the old multi-section details side by side.
                let gap = 12.0;
                let available = ui.available_width();
                let columns = if available >= 650.0 { 2 } else { 1 };
                let card_width = if columns == 2 {
                    (available - gap) / 2.0
                } else {
                    available
                };

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

                // Only one account opens at a time, at full available width.
                // Other overview cards never grow or move into a giant row.
                if let Some(index) = app.selected_details.filter(|index| profiles.contains(index)) {
                    ui.add_space(9.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!(
                            "{} {}",
                            tr("Details for"),
                            app.account_name(app.games[index].profile)
                        )).size(16.0).strong().color(TEXT));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button(tr("Hide details")).clicked() {
                                app.selected_details = None;
                            }
                        });
                    });
                    ui.add_space(8.0);
                    if app.selected_details == Some(index) {
                        // Keep the expanded dashboard at a readable measure on wide windows.
                        let width = ui.available_width().min(820.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(width, 0.0),
                            Layout::top_down(Align::Min),
                            |ui| draw_game_details(app, ui, index, width),
                        );
                        ui.add_space(12.0);
                    }
                }
            }

            ui.add_space(12.0);
            ui.collapsing(tr("Technical details"), |ui| {
                draw_runtime_section(app, ui);
            });
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

/// Manager-like Where to Hunt presentation, merged with server-only Atlas
/// filters and separately labeled real samples vs current-pace scenarios.
fn draw_hunt_atlas_panel(app: &mut ControllerApp, ui: &mut egui::Ui) {
    // Only load the independent public reference when this tab is actually
    // opened; game socket, Twitch and normal dashboard stay extremely light.
    if app.hunt_reference.is_none() && app.hunt_reference_error.is_none() &&
        !app.hunt_reference_busy {
        app.hunt_reference_busy = true;
        let tx = app.hunt_reference_tx.clone();
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(native_atlas::load)
                .unwrap_or_else(|_| Err("Guide reference loader panicked".into()));
            let _ = tx.send(result);
        });
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tr("Account")).size(11.0).color(DIM));
        egui::ComboBox::from_id_salt("hunt-account")
            .selected_text(app.account_name(app.games[app.hunt_account].profile))
            .show_ui(ui, |ui| {
                for index in 0..app.games.len() {
                    let name = app.account_name(app.games[index].profile);
                    ui.selectable_value(&mut app.hunt_account, index, name);
                }
            });
        ui.add_space(8.0);
        ui.label(RichText::new(tr("Native server catalog · no userscripts"))
            .size(10.0).color(MUTED));
    });
    ui.add_space(4.0);
    if app.hunt_reference_busy {
        ui.label(RichText::new(tr("Loading species and move reference..."))
            .size(10.0).color(MUTED));
    } else if let Some(error) = app.hunt_reference_error.clone() {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("{}: {}",
                tr("Guide reference unavailable"), error)).size(10.0).color(WARN));
            if ui.small_button(tr("Retry")).clicked() {
                app.hunt_reference_error = None;
            }
        });
    } else if let Some(reference) = app.hunt_reference.as_deref() {
        ui.label(RichText::new(format!(
            "{} {} · {}",
            reference.species.len(), tr("species in guide"),
            tr("Server determines accessible hunts")
        )).size(10.0).color(MUTED));
    }
    ui.add_space(10.0);
    let index = app.hunt_account;
    let Some(native) = app.games[index].native.as_ref() else {
        ui.add_space(20.0);
        ui.label(RichText::new(tr("Start the account to load its available hunts."))
            .size(13.0).color(WARN));
        ui.label(tr("The native game socket supplies the hunt catalog and active Pokémon after server authentication."));
        return;
    };
    let hunts: HuntView = native.hunt_snapshot();
    if !hunts.catalog_ready {
        ui.label(RichText::new(tr("Waiting for authenticated game hunt catalog..."))
            .size(12.0).color(WARN));
        return;
    }

    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .inner_margin(egui::Margin::same(14)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr("Active Pokémon")).size(10.0).color(DIM));
                    if let Some(p) = hunts.active_pokemon.as_ref() {
                        ui.label(RichText::new(format!("{} · Lv. {}", p.name, p.level))
                            .size(16.0).strong().color(TEXT));
                        ui.label(RichText::new(format!(
                            "P{} · Q {:.3} · {} · {} Pokémon",
                            p.potency.unwrap_or(0), p.quality.unwrap_or(0.0),
                            if p.shiny { "Shiny" } else { "Normal" },
                            hunts.pokemon_count
                        )).size(10.0).color(DIM));
                        let labels = ["HP", "ATK", "DEF", "SpA", "SpD", "SPD"];
                        let stats = labels.iter().enumerate()
                            .map(|(i,k)| format!("{} {}", k,
                                p.ivs[i].map(|v|v.to_string()).unwrap_or_else(|| "—".into())))
                            .collect::<Vec<_>>().join(" · ");
                        ui.label(RichText::new(stats).size(10.0).color(MUTED));
                    } else {
                        ui.label(tr("Waiting for active Pokémon state"));
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr("Bonuses considered")).size(10.0).color(DIM));
                    let b = &hunts.bonuses;
                    ui.label(format!("Twitch +{:.0}% · Guild +{:.0}%",
                        b.twitch_pct, b.guild_pct));
                    ui.label(format!("Event: Trainer +{:.0}% · Pokémon +{:.0}%",
                        b.trainer_event_pct, b.pokemon_event_pct));
                    ui.label(RichText::new(format!(
                        "VIP: {} · Guild boost: {}",
                        if b.vip_active { "on" } else { "off/unknown" },
                        if b.guild_boost_active { "on" } else { "off/unknown" },
                    )).size(10.0).color(DIM));
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr("Current hunt")).size(10.0).color(DIM));
                    let title = hunts.hunts.iter().find(|h|Some(h.slug.as_str()) == hunts.hunt_slug.as_deref())
                        .map(|h|h.name.as_str()).or(hunts.hunt_slug.as_deref()).unwrap_or("—");
                    ui.label(RichText::new(title).size(14.0).strong().color(TEXT));
                    ui.label(RichText::new(format!(
                        "{} {} · {} {}",
                        hunts.hunts.len(), tr("available hunts"),
                        hunts.battle_kills, tr("observed kills")
                    )).size(10.0).color(DIM));
                });
            });
        });
    ui.add_space(10.0);
    let m = &hunts.measurements;
    egui::Frame::new().fill(PANEL_ALT).corner_radius(10.0)
        .inner_margin(egui::Margin::same(13)).show(ui, |ui| {
            ui.label(RichText::new(tr("Measured during current hunt"))
                .size(12.0).strong().color(TEXT));
            ui.horizontal_wrapped(|ui| {
                for (label, amount, valid) in [
                    (tr("Trainer XP/h"), m.trainer_xp_h, m.valid_trainer),
                    (tr("Pokémon XP/h"), m.pokemon_xp_h, m.valid_pokemon),
                    (tr("Net gold/h"), m.gold_net_h, m.valid_gold),
                    (tr("Kills/h"), m.kills_h, m.seconds >= 20.0),
                ] {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(label).size(10.0).color(DIM));
                        ui.label(RichText::new(if valid {
                            format_hunt_amount(amount)
                        } else { "—".into() }).size(15.0).strong()
                            .color(if valid { GOOD } else { MUTED }));
                    });
                    ui.add_space(13.0);
                }
            });
            ui.label(RichText::new(format!(
                "{} · {:.0}s · {} {}",
                hunts.sample_quality, m.seconds, m.kills, tr("kills in window")
            )).size(10.0).color(MUTED));
        });
    ui.add_space(12.0);

    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tr("Available hunts · Hunt Atlas"))
            .size(14.0).strong().color(TEXT));
        ui.add_space(8.0);
        ui.selectable_value(&mut app.hunt_ranking, HuntRanking::TrainerXp, tr("Trainer XP"));
        ui.selectable_value(&mut app.hunt_ranking, HuntRanking::PokemonXp, tr("Pokémon XP"));
        ui.selectable_value(&mut app.hunt_ranking, HuntRanking::Gold, tr("Gold"));
    });
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(&mut app.hunt_query)
            .hint_text(tr("Search hunt, region or species ID"))
            .desired_width(245.0));
        let mut regions = hunts.hunts.iter().map(|h| h.region.as_str())
            .filter(|r| !r.is_empty()).collect::<Vec<_>>();
        regions.sort_unstable();
        regions.dedup();
        let selected_region = if app.hunt_region.is_empty() {
            tr("All regions").to_owned()
        } else { app.hunt_region.clone() };
        egui::ComboBox::from_id_salt("hunt-region")
            .selected_text(selected_region)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.hunt_region, String::new(), tr("All regions"));
                for region in regions {
                    ui.selectable_value(&mut app.hunt_region, region.to_owned(), region);
                }
            });
        if ui.small_button(tr("Clear filters")).clicked() {
            app.hunt_region.clear(); app.hunt_query.clear();
        }
    });
    let query = app.hunt_query.trim().to_lowercase();
    let mut rows: Vec<_> = hunts.hunts.iter().filter(|h| {
        (app.hunt_region.is_empty() || h.region == app.hunt_region) &&
        (query.is_empty() || h.name.to_lowercase().contains(&query) ||
            h.region.to_lowercase().contains(&query) ||
            h.area.to_lowercase().contains(&query) ||
            h.slug.to_lowercase().contains(&query) ||
            h.species.iter().any(|sp| sp.id.to_string().contains(&query)))
    }).collect();
    // Baseline XP/kill is authoritative, but actual kills/hour for another
    // hunt is unknown; show only a scenario at the observed CURRENT pace.
    let kills_per_hour = if m.seconds >= 20.0 && m.kills >= 5 {
        m.kills_h
    } else { 0.0 };
    let bonuses = &hunts.bonuses;
    let trainer_factor = 1.0 +
        (bonuses.trainer_event_pct + bonuses.guild_pct + bonuses.twitch_pct).max(0.0) / 100.0;
    let pokemon_factor = 1.0 +
        (bonuses.pokemon_event_pct + bonuses.guild_pct + bonuses.twitch_pct).max(0.0) / 100.0;
    // Compute forecasts once per visible hunt rather than recalculating the
    // combat simulator O(n log n) times during every sort comparison.
    let reference = app.hunt_reference.as_deref();
    let mut ranked: Vec<_> = rows.drain(..).map(|h| {
        let forecast = reference.and_then(|refdata| native_atlas::estimate(&hunts,h,refdata));
        let (xp, gold) = native_hunts::atlas_pace_scenario(h,kills_per_hour)
            .unwrap_or((0.0,0.0));
        let value = if let Some(forecast) = &forecast {
            match app.hunt_ranking {
                HuntRanking::TrainerXp => forecast.trainer_xp_h,
                HuntRanking::PokemonXp => forecast.pokemon_xp_h,
                HuntRanking::Gold => forecast.gold_h,
            }
        } else {
            match app.hunt_ranking {
                HuntRanking::TrainerXp => xp * trainer_factor,
                HuntRanking::PokemonXp => xp * pokemon_factor,
                HuntRanking::Gold => gold,
            }
        };
        (h, forecast, value)
    }).collect();
    ranked.sort_by(|a,b| b.2.total_cmp(&a.2)
        .then_with(|| a.0.level.cmp(&b.0.level))
        .then_with(|| a.0.name.cmp(&b.0.name)));
    ui.label(RichText::new(format!(
        "{} {} · {}",
        ranked.len(), tr("matching hunts"),
        if reference.is_some() {
            tr("Guide model: forecasts are uncertain; calibrated to observed combat where available.")
        } else {
            tr("Other-hunt rates are scenarios at CURRENT observed kill speed, not predictions of combat performance.")
        }
    )).size(10.0).color(WARN));
    ui.add_space(7.0);

    egui::ScrollArea::vertical().id_salt("atlas-hunt-results")
        .max_height(ui.available_height().max(240.0)).show(ui, |ui| {
            for (index, (h, forecast, value)) in ranked.iter().take(200).enumerate() {
                let current = hunts.hunt_slug.as_deref() == Some(h.slug.as_str());
                let pending = hunts.pending_hunt_slug.as_deref() == Some(h.slug.as_str());
                egui::Frame::new().fill(if current { ACCENT.linear_multiply(0.10) } else { PANEL })
                    .stroke(Stroke::new(1.0, if current { ACCENT } else { BORDER }))
                    .corner_radius(7.0).inner_margin(egui::Margin::symmetric(11, 9))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(format!("{:02}",index+1))
                                .color(MUTED).size(10.0));
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&h.name).strong().size(12.0).color(TEXT));
                                ui.label(RichText::new(format!(
                                    "{} · {} · Lv. {}",
                                    h.area, h.region, h.level
                                )).size(10.0).color(DIM));
                            });
                            let species = h.species.iter()
                                .map(|sp|format!("{} {:.0}%",
                                    reference.and_then(|r|r.species.get(&sp.id))
                                        .map(|s|s.name.clone())
                                        .unwrap_or_else(||format!("#{}",sp.id)),
                                    if h.species.iter().map(|p|p.weight).sum::<f64>() > 0.0 {
                                        sp.weight*100.0/h.species.iter().map(|p|p.weight).sum::<f64>()
                                    }else{0.0}))
                                .collect::<Vec<_>>().join(" · ");
                            ui.label(RichText::new(format!("{} {}",h.species.len(),tr("species")))
                                .size(10.0).color(DIM)).on_hover_text(
                                    if species.is_empty() { tr("No species weights reported").to_owned() } else {species}
                                );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if current {
                                    ui.label(RichText::new(tr("CURRENT")).color(GOOD));
                                } else if pending {
                                    ui.label(RichText::new(tr("Pending")).color(WARN));
                                } else if ui.small_button(tr("Hunt")).clicked() {
                                    if let Err(error) = native.send_command(GameCommand::Hunt(h.slug.clone())) {
                                        crate::logging::warn(&format!("Hunt Atlas command rejected: {error}"));
                                    }
                                }
                                ui.vertical(|ui| {
                                    let show_value = forecast.is_some() || kills_per_hour > 0.0;
                                    ui.label(RichText::new(if show_value {
                                        format_hunt_amount(*value)
                                    } else { "—".into() })
                                        .strong().color(if current { GOOD } else { TEXT }))
                                        .on_hover_text(if let Some(forecast) = forecast {
                                            format!(
                                                "{} · {:.0} kills/h · {:.1}s/kill · {:.0}% species coverage",
                                                if forecast.calibrated { "Calibrated guide model" }
                                                else { "Uncalibrated guide model" },
                                                forecast.kills_h, forecast.seconds_per_kill,
                                                forecast.coverage*100.0
                                            )
                                        } else {
                                            tr("Conditional XP/gold per hour if the CURRENT observed kill rate were maintained.").into()
                                        });
                                    if let Some(forecast) = forecast {
                                        ui.label(RichText::new(tr(if forecast.calibrated {
                                            "Calibrated estimate"
                                        } else { "Experimental estimate" }))
                                            .size(9.0).color(if forecast.calibrated { WARN } else { MUTED }));
                                    }
                                });
                            });
                        });
                    });
                ui.add_space(5.0);
            }
            if ranked.len() > 200 {
                ui.label(tr("Showing first 200 hunts; refine search or region."));
            }
        });
}

/// At-a-glance card: keep the operational summary visible for every enabled
/// account, independently of the much longer optional diagnostics panel.
fn draw_overview_card(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize, width: f32) {
    ui.set_width(width);
    let profile = app.games[index].profile;
    let account_name = app.account_name(profile);
    let health = app.games[index].health();
    let running = app.games[index].is_running();
    let xp_per_hour = if running {
        app.xp_rates[index].observe(&health)
    } else {
        app.xp_rates[index] = XpRateTracker::default();
        None
    };
    let idle_warning = app.idle_warning_for(index, &health);
    let addon_versions = app.addon_version_status.lock()
        .map(|status| status.clone()).unwrap_or_default();
    // Native accounts do not execute browser userscripts. Their old addon
    // version inventory must not show a spurious maintenance warning.
    let browser_runtime = app.games[index].native.is_none();
    let addon_problems = if browser_runtime {
        addon_needs_attention(&health, running, &addon_versions)
    } else { 0 };
    let needs_setup = browser_runtime && !app.account_setup[index].setup_complete;
    let fresh = running && health.telemetry_is_fresh();
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
                    }
                });
            });
            ui.add_space(2.0);
            status_badge(ui, &health, running);
            ui.add_space(9.0);

            ui.columns(2, |columns| {
                let hunt = if !running { "—" } else if health.hunt.is_empty() {
                    match health.activity.as_str() {
                        "Center" => tr("Online · Center"),
                        "Login" => tr("Waiting for login"),
                        "Hunting" => tr("Hunting"),
                        _ => "—",
                    }
                } else { &health.hunt };
                overview_value(&mut columns[0], "Current hunt", hunt);

                let pokemon = if !running || health.active_pokemon.is_empty() {
                    "—"
                } else {
                    health.active_pokemon.as_str()
                };
                overview_value(&mut columns[1], "Active Pokémon", pokemon);
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(5.0);

            // Keep the compact card focused on spendable resources and income
            // rate. Currencies come from the same verified bridge snapshot as
            // the detailed resource panel; never present an unverified zero.
            ui.columns(3, |columns| {
                let balances_ready = fresh && health.bridge_snapshot_ok;
                let gold = if balances_ready { format_number(health.gold) } else { "—".to_string() };
                let gems = if balances_ready { format_number(health.orbs) } else { "—".to_string() };
                overview_value(&mut columns[0], "Gold", &gold);
                overview_value(&mut columns[1], "Gems", &gems);

                columns[2].label(RichText::new(tr("EXP/hour")).size(9.0).color(DIM));
                let rate_text = xp_per_hour
                    .map(|rate| format_number(rate.round() as u64))
                    .unwrap_or_else(|| "—".to_string());
                columns[2].add(
                    egui::Label::new(
                        RichText::new(rate_text)
                            .size(14.0).strong()
                            .color(if xp_per_hour.is_some() && fresh { GOOD } else { MUTED }),
                    ).truncate(),
                ).on_hover_text(tr(match xp_per_hour {
                    Some(rate) if rate > 0.0 => "Based on recent EXP gains",
                    Some(_) => "No EXP gained recently",
                    None => "Calculating EXP rate",
                }));
            });

            ui.add_space(9.0);
            ui.horizontal_wrapped(|ui| {
                let catch_status = if !fresh || !health.bridge_snapshot_ok {
                    tr("Not checked")
                } else if health.autocatch_on {
                    tr("Active")
                } else {
                    tr("Off")
                };
                automation_badge(
                    ui, "Auto Catch", catch_status,
                    fresh && health.bridge_snapshot_ok && health.autocatch_on,
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

            if needs_setup || addon_problems > 0 {
                ui.add_space(7.0);
                let warning = if needs_setup {
                    tr("Setup needed").to_string()
                } else {
                    format!("{} {}", addon_problems, tr(if addon_problems == 1 {
                        "addon needs attention"
                    } else {
                        "addons need attention"
                    }))
                };
                if ui.small_button(RichText::new(format!("⚠ {warning}")).color(WARN)).clicked() {
                    app.open_addon_manager(profile);
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
                    RichText::new(tr("Repair monitor (restart Firefox)")).color(WARN)
                )
                .on_hover_text(tr("Restarts this account's Firefox to release a stuck BiDi session. The game briefly disconnects."))
                .clicked() {
                    app.repair_monitor(profile);
                }
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                // Match the global STOP ALL / LAUNCH ENABLED visual language,
                // but retain exactly one action for this account.
                let action = tr(if !running { "Start native" }
                    else if app.games[index].native.is_none() { "Stop login" }
                    else { "Stop" });
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
                // Only a short status goes beside Stop. Diagnostics and
                // game actions belong to the full-width account details.
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(native) = app.games[index].native.as_ref() {
                        let game = native.snapshot();
                        let twitch = native.twitch_snapshot();
                        let online = game.phase == "Online" && twitch.phase == "Online";
                        let summary = if online {
                            format!("{} · IRC {}/{}", tr("Native"), twitch.joined, twitch.desired)
                        } else {
                            format!("{} · IRC {}", tr("Native"), twitch.phase)
                        };
                        ui.add(egui::Label::new(
                            RichText::new(summary).size(10.0)
                                .color(if online { GOOD } else { WARN })
                        ).truncate()).on_hover_text(format!(
                            "PokéIdle: {} · {} frames · {} kills · {} captures\nTwitch IRC: {} · {}/{} rooms",
                            game.phase, game.frames, game.battle_kills, game.captures,
                            twitch.phase, twitch.joined, twitch.desired
                        ));
                    } else if running {
                        let stage = if app.native_verifying[index] {
                            tr("Verifying native")
                        } else if app.native_handoff_busy[index] {
                            tr("Reading handshake")
                        } else {
                            tr("Firefox login")
                        };
                        ui.add(egui::Label::new(
                            RichText::new(stage).size(10.0).color(WARN)
                        ).truncate()).on_hover_text(tr(
                            "Firefox is temporary; Rust takes over after game and Twitch authentication."
                        ));
                    }
                });
            });
            if app.games[index].native.is_none() {
                if let Some(error) = app.native_error[index].as_deref() {
                    ui.add_space(5.0);
                    ui.add(egui::Label::new(
                        RichText::new(format!("{}: {}", tr("Native error"), compact_text(error, 60)))
                            .size(10.0).color(BAD)
                    ).truncate()).on_hover_text(error);
                }
            }
        });
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
                let irc = format!("{} · {}/{}", twitch.phase, twitch.joined, twitch.desired);
                automation_badge(ui, "Twitch IRC", &irc, twitch.phase == "Online");
            });
            ui.add_space(7.0);
            ui.separator();
            ui.add_space(6.0);

            // The native capture switch belongs to Automation, not the
            // overview card footer beside the account's Stop button.
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Auto Catch")).strong().size(11.0).color(TEXT));
                ui.label(RichText::new(if game.native_auto_catch {
                    tr("Active")
                } else {
                    tr("Off")
                }).size(10.0).color(if game.native_auto_catch { GOOD } else { MUTED }));
                ui.add_space(8.0);
                if ui.small_button(tr(if game.native_auto_catch { "Disable" } else { "Enable" }))
                    .on_hover_text(tr("Change native Auto Catch for this account"))
                    .clicked()
                {
                    let enabled = !game.native_auto_catch;
                    native.set_auto_catch(enabled);
                    std::thread::spawn(move || {
                        if let Err(error) = crate::native_auth::set_catch_enabled(profile, enabled) {
                            crate::logging::warn(&format!(
                                "{} could not save native Auto Catch preference: {error}",
                                profile.label()
                            ));
                        }
                    });
                }
                ui.label(RichText::new(format!(
                    "{} {} · {} {}",
                    game.native_capture_queue, tr("pending"),
                    game.native_throws, tr("throws")
                )).size(10.0).color(DIM));
            });

            ui.add_space(7.0);
            ui.separator();
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                let restock = &game.native_restock;
                ui.label(RichText::new(tr("Restock")).size(11.0).strong().color(TEXT));
                let active = restock.enabled && game.native_auto_catch;
                ui.label(RichText::new(tr(&restock.phase))
                    .size(10.0).color(if active && restock.phase == "Stock OK" {
                        GOOD
                    } else if restock.last_error.is_some() { WARN } else { DIM }));
                if restock.configured {
                    ui.add_space(8.0);
                    if ui.small_button(tr(if restock.enabled { "Disable" } else { "Enable" }))
                        .on_hover_text(tr("Native restock setting; saved without Firefox"))
                        .clicked()
                    {
                        let enabled = !restock.enabled;
                        native.set_restock_enabled(enabled);
                        std::thread::spawn(move || {
                            if let Err(error) = crate::native_auth::set_restock_enabled(profile, enabled) {
                                crate::logging::warn(&format!(
                                    "{} could not save native restock preference: {error}",
                                    profile.label()
                                ));
                            }
                        });
                    }
                }
                let details = format!(
                    "{} {} · {} {}",
                    restock.ordered, tr("ordered"),
                    restock.confirmed, tr("confirmed")
                );
                ui.label(RichText::new(details).size(10.0).color(DIM))
                    .on_hover_text(format!(
                        "{}\n{}",
                        restock.last_order.as_deref().unwrap_or("No orders yet"),
                        restock.last_error.as_deref().unwrap_or("No errors")
                    ));
            });
            if !game.native_restock.configured {
                ui.label(RichText::new(tr("No restock rules imported for this account"))
                    .size(10.0).color(DIM))
                    .on_hover_text(tr(
                        "Native Restock needs ball/item IDs and thresholds; browser settings are imported once during authentication."
                    ));
            }

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
}

fn overview_value(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(RichText::new(tr(label)).size(9.0).color(DIM));
    ui.add(
        egui::Label::new(
            RichText::new(compact_text(value, 35)).size(11.0).strong().color(TEXT)
        ).truncate(),
    ).on_hover_text(value);
}

fn draw_game_details(
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
            if !matches_hunt && health.addon_versions.contains_key("hunt-atlas.user.js") {
                ui.label(RichText::new(tr("HuntAtlas metrics unavailable — update HuntAtlas and Controller Bridge, then refresh the game."))
                    .size(10.0).color(MUTED));
            }
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
                // A browser FPS cap does not establish that Performance+ is enabled.
                let performance_active = health.telemetry_is_fresh()
                    && health.performance_enabled == Some(true);
                let performance_value = if !health.telemetry_is_fresh() {
                    tr("Not checked yet").to_string()
                } else {
                    match health.performance_enabled {
                        Some(false) => tr("Off").to_string(),
                        Some(true) if health.performance_fps.is_empty() => tr("Active").to_string(),
                        Some(true) => format!("{} FPS", health.performance_fps),
                        None => tr("Not detected in game").to_string(),
                    }
                };

                ui.horizontal_wrapped(|ui| {
                    automation_badge(
                        ui,
                        tr("Auto Catch"),
                        if !health.telemetry_is_fresh() { tr("Not checked") } else if health.addon_missing.iter().any(|name| name == "Auto Catch+") { tr("Not detected in game") } else if !health.bridge_snapshot_ok { tr("Bridge data unavailable") } else if health.autocatch_on { tr("Active") } else { tr("Off") },
                        health.telemetry_is_fresh() && health.bridge_snapshot_ok && health.autocatch_on,
                    );
                    automation_badge(
                        ui,
                        tr("Restock"),
                        if !health.telemetry_is_fresh() || !health.bridge_snapshot_ok { tr("Not checked") } else if health.autocatch_restock.is_empty() { tr("Not reported") } else { tr(&health.autocatch_restock) },
                        health.telemetry_is_fresh() && health.bridge_snapshot_ok && health.autocatch_on && !health.autocatch_restock.is_empty() && !health.autocatch_restock.eq_ignore_ascii_case("off"),
                    );
                    automation_badge(
                        ui,
                        tr("Stream scanner"),
                        if !health.telemetry_is_fresh() || !health.bridge_snapshot_ok {
                            tr("Not checked")
                        } else if health.stream_scan_status.is_empty() {
                            tr("Waiting")
                        } else {
                            tr(&health.stream_scan_status)
                        },
                        health.telemetry_is_fresh() && health.stream_scan_status.eq_ignore_ascii_case("ok"),
                    );
                    if health.telemetry_is_fresh() && health.stream_scan_status.eq_ignore_ascii_case("ok") {
                        if let Some(channels) = health.stream_scan_twitch_channels {
                            let shared_tabs = health.tabs.iter()
                                .filter(|tab| tab.kind == "Twitch" && tab.twitch_channels.is_some())
                                .count();
                            let (value, ready) = if channels == 0 {
                                (tr("No live Twitch channels in last scan").to_string(), false)
                            } else if shared_tabs == 0 {
                                (format!("{} {} · {}", channels, tr("Twitch channels"), tr("shared chat tab not observed yet")), false)
                            } else if shared_tabs == 1 {
                                (format!("{} {} · 1 {}", channels, tr("Twitch channels"), tr("shared chat tab")), true)
                            } else {
                                (format!("{} {} · {} {}", channels, tr("Twitch channels"), shared_tabs, tr("shared chat tabs (refreshing)")), true)
                            };
                            automation_badge(ui, tr("Twitch chats"), &value, ready)
                                .on_hover_text(tr("One chat tab can include many Twitch channels; a second tab may briefly appear during the five-minute refresh."));
                        }
                    }
                    automation_badge(ui, tr("Performance+"), &performance_value, performance_active);
                });

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
                        ui.label(RichText::new(tr("Twitch XP bonus not checked (Controller Bridge data unavailable)")).size(10.0).color(DIM));
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

            if app.games[index].native.is_none() {
                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    // Unverified addon counts must not look like a live warning or success.
                    let addon_color = if !health.telemetry_is_fresh() {
                        DIM
                    } else if health.addon_ok == health.addon_total {
                        GOOD
                    } else {
                        WARN
                    };

                    ui.label(
                        RichText::new(if health.telemetry_is_fresh() { format!("{} {}/{}", tr("ADDONS"), health.addon_ok, health.addon_total) } else { format!("{} —/—", tr("ADDONS")) })
                            .size(10.0)
                            .strong()
                            .color(addon_color),
                    );

                    if health.telemetry_is_fresh() && !health.addon_missing.is_empty() {
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
                        if health.telemetry_is_fresh() && health.twitch_tabs > 0 {
                            ui.label(
                                RichText::new(format!(
                                    "{} {}/{}",
                                    tr("Optimized Twitch tabs"),
                                    health.twitch_low_resource_ok,
                                    health.twitch_tabs
                                ))
                                .size(10.0)
                                .color(MUTED),
                            ).on_hover_text(tr("This ratio counts Twitch browser tabs, not livestream channels."));
                        }
                    });
                });
            }
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

// Compact key/value rows keep stats aligned in narrow two-column game cards.
// Full untruncated values remain available on hover.
fn stat_line(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr(label)).size(9.0).color(DIM));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(compact_text(value, 25)).size(11.0).strong().color(TEXT)
                ).truncate()
            ).on_hover_text(value);
        });
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
                let firefox_state = format!("{running} {} · {headless} {}", tr("running"), tr("Headless"));
                let bidi_state = format!("{bidi_connected}/{running} {}", tr("Connected"));
                runtime_chip(ui, tr("Firefox"), &firefox_state, if running > 0 { WARN } else { GOOD });
                runtime_chip(ui, "Native game", &format!("{native_online} online"), if native_online > 0 { GOOD } else { MUTED });
                runtime_chip(ui, "Native Twitch IRC", &format!("{native_twitch_online} online"), if native_twitch_online > 0 { GOOD } else { MUTED });
                runtime_chip(ui, tr("BiDi"), &bidi_state, if running == 0 { MUTED } else if bidi_connected == running { GOOD } else { WARN });
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
                runtime_chip(ui, tr("Stream chat"), tr("Direct IRC"), MUTED);
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


/// Only classify a script as missing if the monitored game explicitly reports
/// it missing. Unmonitored/headless-loading accounts cannot be called broken.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AddonCondition {
    Current,
    Outdated,
    NotDetected,
    Unverified,
}

fn addon_label(file: &str) -> &str {
    match file {
        "autocatch.user.js" => "Auto Catch+",
        "controller-bridge.user.js" => "Controller Bridge",
        "hunt-atlas.user.js" => "Hunt Atlas",
        "market-bot.user.js" => "Moth Watch",
        "performance.user.js" => "Performance+",
        "stream-auto-open.user.js" => "Stream Scanner",
        "twitch-low-resource.user.js" => "Twitch Low Resource",
        _ => file,
    }
}

fn addon_condition(
    health: &Health,
    running: bool,
    latest: &update::AddonVersionStatus,
    file: &str,
) -> AddonCondition {
    if !running || !health.telemetry_is_fresh() || !latest.checked {
        return AddonCondition::Unverified;
    }
    if let Some(installed) = health.addon_versions.get(file) {
        if latest.versions.get(file).is_some_and(|v| update::is_newer_version(v, installed)) {
            AddonCondition::Outdated
        } else {
            AddonCondition::Current
        }
    } else if health.addon_missing.iter().any(|name| name == addon_label(file)) {
        // The script reported a marker but its runtime check failed.
        AddonCondition::NotDetected
    } else {
        // Missing markers do not prove a missing mandatory installation:
        // users can choose which published userscripts to enable.
        AddonCondition::Unverified
    }
}

fn addon_needs_attention(
    health: &Health, running: bool, latest: &update::AddonVersionStatus,
) -> usize {
    if !latest.checked { return 0; }
    latest.versions.keys().filter(|file| matches!(
        addon_condition(health, running, latest, file),
        AddonCondition::Outdated | AddonCondition::NotDetected
    )).count()
}

fn draw_accounts_window(app: &mut ControllerApp, ctx: &egui::Context) {
    if app
        .account_setup_checked_at
        .map(|checked| checked.elapsed() >= Duration::from_secs(30))
        .unwrap_or(true)
    {
        app.refresh_account_setup();
    }

    let remote_addons = app.addon_version_status.lock()
        .map(|state| state.clone()).unwrap_or_default();
    let mut open = app.show_accounts;
    egui::Window::new(tr("Account Manager"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(820.0)
        .default_height(620.0)
        .min_width(480.0)
        .min_height(380.0)
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
                    RichText::new("Up to four native game + Twitch accounts. Firefox is only for authentication.")
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

                        let card = egui::Frame::new()
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
                                        [(ui.available_width() - 130.0).clamp(145.0, 360.0), 30.0],
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
                                        tr("Profile folder exists")
                                    } else {
                                        tr("Not configured")
                                    };
                                    let vm_label = if setup.violentmonkey_installed {
                                        tr("Registered")
                                    } else {
                                        if setup.check_error.is_some() { tr("Check failed") } else { tr("Not installed") }
                                    };
                                    let twitch_open = health.telemetry_is_fresh() && health.tabs.iter().any(|tab| tab.kind == "Twitch");
                                    // This check only observes tab existence. It does not
                                    // authenticate Twitch, so never label a profile as logged in.
                                    let twitch_native = app.games[index].native.as_ref()
                                        .is_some_and(|native| native.twitch_snapshot().phase == "Online");
                                    let twitch_label = if twitch_native {
                                        "Native IRC connected"
                                    } else if twitch_open {
                                        tr("Tab open")
                                    } else if running && health.telemetry_is_fresh() {
                                        tr("No Twitch tab open")
                                    } else {
                                        tr("Not checked")
                                    };

                                    status_chip(ui, "Firefox", profile_label, setup.profile_ready);
                                    let vm_chip = status_chip(ui, tr("Violentmonkey"), vm_label, setup.violentmonkey_installed);
                                    if let Some(error) = &setup.check_error {
                                        vm_chip.on_hover_text(error.as_str());
                                    }
                                    status_chip(ui, "Twitch", twitch_label, twitch_open || twitch_native);

                                    if running && health.telemetry_is_fresh() {
                                        let script_status = status_chip(
                                            ui,
                                            tr("Scripts"),
                                            &format!("{} {}", health.addon_ok, tr("detected")),
                                            health.addon_ok > 0 && health.addon_missing.is_empty(),
                                        );
                                        if !health.addon_missing.is_empty() {
                                            script_status.on_hover_text(
                                                format!("{} {}", tr("Missing:"), health.addon_missing.join(", "))
                                            );
                                        }
                                    } else {
                                        status_chip(ui, tr("Scripts"), tr("Not checked"), false);
                                    }
                                });

                                if enabled && (!setup.setup_complete || app.show_setup_guide[index]) {
                                    ui.add_space(9.0);
                                    egui::Frame::new()
                                        .fill(PANEL)
                                        .corner_radius(7.0)
                                        .inner_margin(10.0)
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new(tr("Initial setup")).strong().color(TEXT));
                                                if setup.setup_complete && ui.small_button(tr("Hide setup guide")).clicked() {
                                                    app.show_setup_guide[index] = false;
                                                }
                                            });
                                            ui.label(RichText::new(tr("Complete these steps once for this profile."))
                                                .size(10.0).color(MUTED));
                                            ui.horizontal_wrapped(|ui| {
                                                status_chip(ui, tr("Firefox profile created"), if setup.profile_ready { "✓" } else { "…" }, setup.profile_ready);
                                                status_chip(ui, tr("Violentmonkey installed"), if setup.violentmonkey_installed { "✓" } else { "…" }, setup.violentmonkey_installed);
                                                let verified = running && health.telemetry_is_fresh();
                                                status_chip(ui, tr("Game signed in"), if verified && health.logged_in { "✓" } else { "…" }, verified && health.logged_in);
                                                let script_count = format!("{} {}", health.addon_ok, tr("detected"));
                                                status_chip(ui, tr("Scripts detected"),
                                                    if verified { script_count.as_str() } else { "—" },
                                                    verified && health.addon_ok > 0 && health.addon_missing.is_empty());
                                            });
                                            ui.label(RichText::new(tr("You can finish with only the addons you want. Unverified steps are not automatically considered complete."))
                                                .size(9.0).color(DIM));
                                            ui.horizontal_wrapped(|ui| {
                                                if !setup.violentmonkey_installed {
                                                    if ui.button(tr("Install Violentmonkey")).clicked() {
                                                        app.profile_action(profile, ProfileAction::InstallViolentmonkey);
                                                    }
                                                } else {
                                                    if ui.button(tr("Manage Addons")).clicked() {
                                                        app.open_addon_manager(profile);
                                                    }
                                                }
                                                if ui.add_enabled(setup.profile_ready && setup.violentmonkey_installed,
                                                    egui::Button::new(tr("Finish setup"))).clicked()
                                                {
                                                    match accounts::complete_setup(profile) {
                                                        Ok(()) => {
                                                            app.account_setup[index].setup_complete = true;
                                                            app.show_setup_guide[index] = false;
                                                            app.set_status(tr("Setup completed"), false);
                                                        }
                                                        Err(error) => app.set_status(error, true),
                                                    }
                                                }
                                            });
                                        });
                                } else if enabled {
                                    if ui.small_button(tr("Show setup guide")).clicked() {
                                        app.show_setup_guide[index] = true;
                                    }
                                }

                                ui.add_space(8.0);
                                let attention = addon_needs_attention(&health, running, &remote_addons);
                                ui.horizontal_wrapped(|ui| {
                                    if ui.button(tr("Manage Addons")).clicked() {
                                        app.addon_manager_open[index] = !app.addon_manager_open[index];
                                    }
                                    if attention > 0 {
                                        ui.colored_label(WARN, format!("{attention} {}",
                                            tr(if attention == 1 { "addon needs attention" } else { "addons need attention" })));
                                    }
                                });

                                if app.addon_manager_open[index] {
                                    egui::Frame::new()
                                        .fill(PANEL)
                                        .stroke(Stroke::new(1.0, BORDER))
                                        .corner_radius(8.0)
                                        .inner_margin(12.0)
                                        .show(ui, |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(RichText::new(tr("Manage Addons"))
                                                    .size(13.0).strong().color(TEXT));
                                                if ui.add_enabled(!remote_addons.checking,
                                                    egui::Button::new(tr("Check now"))).clicked()
                                                {
                                                    app.request_addon_refresh();
                                                }
                                                if remote_addons.checking {
                                                    ui.spinner();
                                                    ui.label(tr("Checking for updates…"));
                                                }
                                            });
                                            if !running || !health.telemetry_is_fresh() {
                                                ui.label(RichText::new(tr("Live addon status requires the game to be running and monitored."))
                                                    .size(10.0).color(DIM));
                                            }
                                            if !remote_addons.checked {
                                                ui.label(RichText::new(tr(if remote_addons.error.is_some() {
                                                    "GitHub version check unavailable"
                                                } else {
                                                    "Checking GitHub versions…"
                                                })).size(10.0).color(WARN));
                                                if let Some(error) = &remote_addons.error {
                                                    ui.label(RichText::new(error.as_str()).size(9.0).color(DIM));
                                                }
                                            } else {
                                                if !app.addon_selection_initialized[index]
                                                    && running && health.telemetry_is_fresh()
                                                {
                                                    for file in remote_addons.versions.keys() {
                                                        if matches!(addon_condition(&health, running, &remote_addons, file),
                                                            AddonCondition::Outdated | AddonCondition::NotDetected)
                                                        {
                                                            app.addon_selection[index].insert(file.clone());
                                                        }
                                                    }
                                                    app.addon_selection_initialized[index] = true;
                                                }
                                                for (file, latest) in &remote_addons.versions {
                                                    let condition = addon_condition(&health, running, &remote_addons, file);
                                                    let mut checked = app.addon_selection[index].contains(file);
                                                    ui.horizontal_wrapped(|ui| {
                                                        if ui.checkbox(&mut checked, "").changed() {
                                                            if checked {
                                                                app.addon_selection[index].insert(file.clone());
                                                            } else {
                                                                app.addon_selection[index].remove(file);
                                                            }
                                                        }
                                                        ui.label(RichText::new(addon_label(file)).size(11.0).strong().color(TEXT));
                                                        let (label, color) = match condition {
                                                            AddonCondition::Current => (tr("Up to date"), GOOD),
                                                            AddonCondition::Outdated => (tr("outdated"), WARN),
                                                            AddonCondition::NotDetected => (tr("Not detected in game"), WARN),
                                                            AddonCondition::Unverified => {
                                                                if running && health.telemetry_is_fresh() && remote_addons.checked {
                                                                    (tr("Not installed or inactive"), DIM)
                                                                } else {
                                                                    (tr("Not checked yet"), DIM)
                                                                }
                                                            }
                                                        };
                                                        let versions = match health.addon_versions.get(file) {
                                                            Some(installed) => format!("v{installed} → v{latest}"),
                                                            None => format!("v{latest}"),
                                                        };
                                                        ui.label(RichText::new(versions).size(10.0).color(DIM));
                                                        ui.label(RichText::new(label).size(10.0).color(color));
                                                    });
                                                }
                                            }
                                            ui.add_space(7.0);
                                            ui.label(RichText::new(tr("Select only the affected scripts. Installers open in this Firefox profile; confirm installation in Violentmonkey."))
                                                .size(9.0).color(DIM));
                                            let installer_visible = !running || !app.games[index].headless;
                                            if !installer_visible {
                                                ui.colored_label(WARN, tr("Switch to visible Firefox before opening installers."));
                                                if ui.button(tr("Restart visible")).clicked() {
                                                    app.set_browser_mode(profile, false);
                                                }
                                            }
                                            ui.horizontal_wrapped(|ui| {
                                                let can_repair = setup.violentmonkey_installed && installer_visible
                                                    && remote_addons.checked
                                                    && !app.addon_selection[index].is_empty()
                                                    && !app.addon_install_busy[index];
                                                if ui.add_enabled(can_repair, egui::Button::new(tr("Repair selected"))).clicked() {
                                                    let selected: Vec<String> = app.addon_selection[index].iter().cloned().collect();
                                                    app.start_addon_install(profile, Some(selected));
                                                }
                                                if ui.add_enabled(setup.violentmonkey_installed && installer_visible && !app.addon_install_busy[index],
                                                    egui::Button::new(tr("Install all (first setup)"))).clicked()
                                                {
                                                    app.start_addon_install(profile, None);
                                                }
                                            });
                                            ui.label(RichText::new(tr("Opens installer links through Firefox; confirm installation in Violentmonkey."))
                                                .size(9.0).color(DIM));
                                            ui.label(RichText::new(tr("Reload the game only when you choose; an automatic reload could interrupt gameplay."))
                                                .size(9.0).color(DIM));
                                        });
                                }

                                ui.add_space(8.0);

                                ui.label(
                                    RichText::new(
                                        "Firefox stores login sessions; Rust owns game + Twitch IRC after automatic handoff.",
                                    )
                                    .size(9.0)
                                    .color(DIM),
                                );

                                ui.add_space(8.0);

                                ui.horizontal_wrapped(|ui| {
                                    profile_button(ui, tr("Open Game"), || {
                                        app.profile_action(profile, ProfileAction::Game)
                                    });
                                    profile_button(ui, tr("Google sign-in"), || {
                                        app.profile_action(profile, ProfileAction::GoogleSignIn)
                                    });
                                    profile_button(ui, tr("Twitch"), || {
                                        app.profile_action(profile, ProfileAction::Twitch)
                                    });
                                    profile_button(ui, tr("Install Violentmonkey"), || {
                                        app.profile_action(profile, ProfileAction::InstallViolentmonkey)
                                    });
                                    // All addon actions live under Manage Addons above.
                                    profile_button(ui, tr("Profile folder"), || {
                                        app.profile_action(profile, ProfileAction::Folder)
                                    });
                                });
                            });
                        if app.addon_focus == Some(index) {
                            ui.scroll_to_rect(card.response.rect, Some(egui::Align::Center));
                            app.addon_focus = None;
                        }

                        if app.addon_install_busy[index] {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(tr("Opening addon installers…"));
                            });
                        }
                        if !app.addon_install_details[index].is_empty() {
                            ui.collapsing(tr("Installer details"), |ui| {
                                for message in &app.addon_install_details[index] {
                                    ui.label(RichText::new(message).size(10.0).color(MUTED));
                                }
                            });
                        }

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

fn status_chip(ui: &mut egui::Ui, name: &str, state: &str, good: bool) -> egui::Response {
    let color = if good { GOOD } else { DIM };
    ui.label(
        RichText::new(format!("{}: {}", tr(name), tr(state)))
            .size(9.0)
            .color(color),
    )
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
    // Loss of BiDi telemetry does not imply that the game or its userscripts stopped.
    // Keep the state accessible without a constantly growing, alarm-colored warning.
    let no_readings = running
        && health.last_heartbeat_at.is_some()
        && !health.heartbeat_is_fresh();
    let session_blocked = running && health.last_error.as_deref()
        .is_some_and(|error| error.contains("Maximum number of active sessions"));
    let (label, color) = if !running {
        ("STOPPED", DIM)
    } else if session_blocked {
        ("BIDI SESSION BLOCKED", WARN)
    } else if no_readings {
        ("CONNECTION UNCONFIRMED", DIM)
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
    if no_readings || session_blocked {
        response.on_hover_ui(|ui| {
            if session_blocked {
                ui.label(tr("Firefox is refusing a new BiDi session; use Repair monitor to restart only this account."));
            } else {
                ui.label(tr("The controller has stopped receiving telemetry. Firefox and addons may still be running."));
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
