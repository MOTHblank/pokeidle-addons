// ==UserScript==
// @name         PokéIdle Hunt Atlas
// @namespace    moth.pokeidle
// @author       MOTHblank
// @homepageURL  https://github.com/MOTHblank/pokeidle-huntatlas
// @supportURL   https://github.com/MOTHblank/pokeidle-huntatlas/issues
// @downloadURL  https://raw.githubusercontent.com/MOTHblank/pokeidle-huntatlas/main/hunt-atlas.user.js
// @updateURL    https://raw.githubusercontent.com/MOTHblank/pokeidle-huntatlas/main/hunt-atlas.user.js
// @version      1.6.1
// @description  Hunt finder with measured lead-Pokémon combat speed and personalized trainer XP/hour ranking.
// @match        https://pokeidle.io/*
// @match        https://www.pokeidle.io/*
// @grant        unsafeWindow
// @grant        GM_xmlhttpRequest
// @connect      pokeapi.co
// @run-at       document-start
// ==/UserScript==

(() => {
    'use strict';

    const page =
        typeof unsafeWindow !== 'undefined'
            ? unsafeWindow
            : window;

    const CACHE_KEY =
        'moth-pokeidle-hunt-atlas-cache-v1';

    const TYPE_CACHE_KEY =
        'moth-pokeidle-hunt-atlas-types-v1';

    const FILTER_KEY =
        'moth-pokeidle-hunt-atlas-filters-v2';

    const ENABLED_KEY =
        'moth-pokeidle-hunt-atlas-enabled-v1';

    const PERF_KEY =
        'moth-pokeidle-hunt-atlas-performance-v2';

    const MARKET_CACHE_KEY =
        'moth-pokeidle-hunt-atlas-market-v1';

    const MARKET_REFRESH_MS =
        15 * 60 * 1000;

    const MARKET_CACHE_MAX_AGE_MS =
        7 * 24 * 60 * 60 * 1000;

    const MARKET_HISTORY_MAX_PAGES =
        8;

    const BUTTON_ID =
        'moth-hunt-atlas-button';

    const DRAWER_ID =
        'moth-hunt-atlas-drawer';

    const STYLE_ID =
        'moth-hunt-atlas-style';

    const STANDARD_TYPES = [
        'NORMAL',
        'FIRE',
        'WATER',
        'ELECTRIC',
        'GRASS',
        'ICE',
        'FIGHTING',
        'POISON',
        'GROUND',
        'FLYING',
        'PSYCHIC',
        'BUG',
        'ROCK',
        'GHOST',
        'DRAGON',
        'DARK',
        'STEEL',
        'FAIRY'
    ];

    /*
     * Attack type -> defender type multiplier.
     * Missing entries are neutral (1x).
     */
    const TYPE_CHART = {
        NORMAL: {
            ROCK: 0.5,
            GHOST: 0,
            STEEL: 0.5
        },
        FIRE: {
            FIRE: 0.5,
            WATER: 0.5,
            GRASS: 2,
            ICE: 2,
            BUG: 2,
            ROCK: 0.5,
            DRAGON: 0.5,
            STEEL: 2
        },
        WATER: {
            FIRE: 2,
            WATER: 0.5,
            GRASS: 0.5,
            GROUND: 2,
            ROCK: 2,
            DRAGON: 0.5
        },
        ELECTRIC: {
            WATER: 2,
            ELECTRIC: 0.5,
            GRASS: 0.5,
            GROUND: 0,
            FLYING: 2,
            DRAGON: 0.5
        },
        GRASS: {
            FIRE: 0.5,
            WATER: 2,
            GRASS: 0.5,
            POISON: 0.5,
            GROUND: 2,
            FLYING: 0.5,
            BUG: 0.5,
            ROCK: 2,
            DRAGON: 0.5,
            STEEL: 0.5
        },
        ICE: {
            FIRE: 0.5,
            WATER: 0.5,
            GRASS: 2,
            ICE: 0.5,
            GROUND: 2,
            FLYING: 2,
            DRAGON: 2,
            STEEL: 0.5
        },
        FIGHTING: {
            NORMAL: 2,
            ICE: 2,
            POISON: 0.5,
            FLYING: 0.5,
            PSYCHIC: 0.5,
            BUG: 0.5,
            ROCK: 2,
            GHOST: 0,
            DARK: 2,
            STEEL: 2,
            FAIRY: 0.5
        },
        POISON: {
            GRASS: 2,
            POISON: 0.5,
            GROUND: 0.5,
            ROCK: 0.5,
            GHOST: 0.5,
            STEEL: 0,
            FAIRY: 2
        },
        GROUND: {
            FIRE: 2,
            ELECTRIC: 2,
            GRASS: 0.5,
            POISON: 2,
            FLYING: 0,
            BUG: 0.5,
            ROCK: 2,
            STEEL: 2
        },
        FLYING: {
            ELECTRIC: 0.5,
            GRASS: 2,
            FIGHTING: 2,
            BUG: 2,
            ROCK: 0.5,
            STEEL: 0.5
        },
        PSYCHIC: {
            FIGHTING: 2,
            POISON: 2,
            PSYCHIC: 0.5,
            DARK: 0,
            STEEL: 0.5
        },
        BUG: {
            FIRE: 0.5,
            GRASS: 2,
            FIGHTING: 0.5,
            POISON: 0.5,
            FLYING: 0.5,
            PSYCHIC: 2,
            GHOST: 0.5,
            DARK: 2,
            STEEL: 0.5,
            FAIRY: 0.5
        },
        ROCK: {
            FIRE: 2,
            ICE: 2,
            FIGHTING: 0.5,
            GROUND: 0.5,
            FLYING: 2,
            BUG: 2,
            STEEL: 0.5
        },
        GHOST: {
            NORMAL: 0,
            PSYCHIC: 2,
            GHOST: 2,
            DARK: 0.5
        },
        DRAGON: {
            DRAGON: 2,
            STEEL: 0.5,
            FAIRY: 0
        },
        DARK: {
            FIGHTING: 0.5,
            PSYCHIC: 2,
            GHOST: 2,
            DARK: 0.5,
            FAIRY: 0.5
        },
        STEEL: {
            FIRE: 0.5,
            WATER: 0.5,
            ELECTRIC: 0.5,
            ICE: 2,
            ROCK: 2,
            STEEL: 0.5,
            FAIRY: 2
        },
        FAIRY: {
            FIRE: 0.5,
            FIGHTING: 2,
            POISON: 0.5,
            DRAGON: 2,
            DARK: 2,
            STEEL: 0.5
        }
    };


    const TEXT = {
        en: {
            'title': 'Hunt Atlas',
            'toggle.on': 'Atlas on',
            'toggle.off': 'Atlas off',
            'toggle.onTitle': 'Hunt Atlas is replacing the native map. Click to use PokéIdle’s map.',
            'toggle.offTitle': 'PokéIdle’s native map is active. Click to enable Hunt Atlas.',
            'filter.search': 'Search',
            'filter.searchPlaceholder': 'Pokémon, hunt or region…',
            'filter.region': 'Region',
            'filter.allRegions': 'All regions',
            'filter.level': 'Level',
            'filter.min': 'Min',
            'filter.max': 'Max',
            'filter.minLevelAria': 'Minimum hunt level',
            'filter.maxLevelAria': 'Maximum hunt level',
            'filter.availability': 'Availability',
            'filter.all': 'All',
            'filter.unlocked': 'Unlocked',
            'filter.locked': 'Locked',
            'filter.type': 'Type',
            'filter.allTypes': 'All types',
            'filter.typeLoading': 'Type: loading…',
            'filter.weakTo': 'Weak to',
            'filter.anyWeakness': 'Any weakness',
            'filter.weaknessLoading': 'Weakness: loading…',
            'filter.collection': 'Collection',
            'filter.caughtAndUncaught': 'Caught + uncaught',
            'filter.uncaughtOnly': 'Uncaught only',
            'filter.caughtOnly': 'Caught only',
            'filter.sort': 'Sort',
            'sort.xp': 'Best XP/hour',
            'sort.npcDesc': 'NPC sell value · high first',
            'sort.npcAsc': 'NPC sell value · low first',
            'sort.playerDesc': 'Player Market · high first',
            'sort.playerAsc': 'Player Market · low first',
            'sort.matchup': 'Best matchup',
            'sort.spawnDesc': 'Highest encounter rate',
            'sort.levelAsc': 'Lowest hunt level',
            'sort.levelDesc': 'Highest hunt level',
            'sort.pokedex': 'Pokédex number',
            'sort.name': 'Name A–Z',
            'filter.clear': 'Clear filters',
            'filter.defaults': 'Default filters',
            'filter.changedOne': '1 filter changed from default',
            'filter.changedMany': '{count} filters changed from default',
            'footer.by': 'by MOTHblank',
            'footer.play': 'MOTHblank on Google Play',
            'footer.x': 'MOTHblank on X',
            'footer.whatsapp': 'WhatsApp / Pix',
            'footer.source': 'source code',
            'capture.caught': 'caught',
            'capture.notCaught': 'not caught',
            'capture.typeUnknown': 'type ?',
            'common.lead': 'Lead',
            'price.npc': 'NPC ~{value}',
            'price.players': 'Players ~{value}',
            'price.npcTip': 'NPC sale reference at the visible hunt level{plural} and quality 1.0{range} · actual captured Pokémon vary with quality; shiny ×10',
            'price.playerTip': 'Recent non-shiny completed player Market sales · {count} {sample} · range {min}–{max}',
            'price.sample': 'sample',
            'price.samples': 'samples',
            'price.range': ' · range {min}–{max}',
            'matchup.immune': 'immune',
            'matchup.weak': 'weak',
            'matchup.resists': 'resists',
            'matchup.neutral': 'neutral',
            'matchup.deal': 'Deal {multiplier} · {relation}',
            'matchup.take': 'Take {multiplier} · {relation}',
            'matchup.dealTitle': '{lead}’s best known attack type ({type}) vs {species}: {multiplier}.',
            'matchup.takeTitle': '{species}’s best STAB type ({type}) vs {lead}: {multiplier}.',
            'hunt.level': 'Lv {level}',
            'hunt.weight': '~{value}% weight',
            'hunt.unlocked': 'unlocked',
            'hunt.locked': 'locked',
            'hunt.here': 'Here',
            'hunt.go': 'Go',
            'hunt.requiresLevel': 'Requires level {level}',
            'hunt.current': 'Current hunt',
            'hunt.travel': 'Travel to this hunt',
            'hunt.measuredXp': 'measured {value} trainer XP/h',
            'hunt.modeledXp': 'combat model {value} trainer XP/h',
            'xp.waitingLead': 'Waiting for the active Pokémon before calculating trainer XP/hour.',
            'xp.lead': '{name} Lv {level} · lead · power {power}',
            'xp.warmStart': 'warm-started from nearest level',
            'xp.kills': '{count} kills',
            'xp.attacks': '{count} attacks',
            'xp.cadence': '{seconds}s attack cadence{detail}',
            'xp.throughput': ' (attack throughput)',
            'xp.floor': ' (900ms floor)',
            'xp.travel': '{seconds}s travel',
            'xp.hpSamples': '{count} HP samples',
            'xp.measuredHunt': '{count} measured hunt',
            'xp.measuredHunts': '{count} measured hunts',
            'xp.calibrating': 'combat model calibrating',
            'xp.learning': 'learning this lead Pokémon: waiting for real attack and kill events',
            'xp.killsPerHour': '{count} kills/h',
            'xp.pokemonPerHour': '{value} Pokémon XP/h',
            'xp.measuredShort': 'measured {value} XP/h',
            'xp.modelShort': 'model {value} XP/h',
            'xp.best': 'Best XP in current filters',
            'xp.measuredTrainer': 'measured {value} trainer XP/h',
            'xp.modeledTrainer': 'modeled {value} trainer XP/h',
            'xp.noEstimate': 'No unlocked XP estimate matches the current filters.',
            'xp.details': 'Details',
            'xp.hideDetails': 'Hide details',
            'xp.recommendationNote': '{lead} · recommendations respect the current Atlas filters.',
            'xp.currentMeasured': ' · current measured {value} trainer XP/h',
            'xp.currentModeled': ' · current modeled {value} trainer XP/h',
            'xp.modelNote': ' · exact upstream XP/kill curve; movement, cadence, damage and kill speed come from this lead Pokémon’s real combat.',
            'results.showMore': 'Show more · {count} Pokémon remaining',
            'results.noMatch': 'No Pokémon match these filters. Use Clear filters to reset the Atlas.',
            'results.waiting': 'Waiting for PokéIdle hunt data. Reload the page once after installing Hunt Atlas if this remains empty.',
            'results.count': '{pokemon} Pokémon · {hunts} hunts',
            'results.countPartial': '{shown}/{pokemon} Pokémon · {hunts} hunts',
            'status.waitingGameData': 'Waiting for game data…',
            'status.cachedCatalog': 'Using cached hunt catalog until live state arrives.',
            'status.entered': 'Entered {hunt}.',
            'status.huntsLoaded': '{count} hunt regions loaded.',
            'status.gameNotReady': 'Game connection is not ready yet.',
            'status.traveling': 'Traveling to {hunt}…',
            'status.notAccepted': 'PokéIdle did not accept {hunt}. It may still be locked or hunt switching may be on cooldown.',
            'status.couldNotSend': 'Could not send hunt selection.',
            'status.huntLocked': '{hunt} is locked — requires level {level}.',
            'status.alreadyHere': 'Already hunting in {hunt}.',
            'status.loadingTypes': ' · loading type data',
            'status.typesUnavailable': ' · type data unavailable',
            'status.marketSampling': ' · sampling Market prices',
            'type.NORMAL': 'Normal',
            'type.FIRE': 'Fire',
            'type.WATER': 'Water',
            'type.ELECTRIC': 'Electric',
            'type.GRASS': 'Grass',
            'type.ICE': 'Ice',
            'type.FIGHTING': 'Fighting',
            'type.POISON': 'Poison',
            'type.GROUND': 'Ground',
            'type.FLYING': 'Flying',
            'type.PSYCHIC': 'Psychic',
            'type.BUG': 'Bug',
            'type.ROCK': 'Rock',
            'type.GHOST': 'Ghost',
            'type.DRAGON': 'Dragon',
            'type.DARK': 'Dark',
            'type.STEEL': 'Steel',
            'type.FAIRY': 'Fairy'
        },
        'pt-BR': {
            'title': 'Hunt Atlas',
            'toggle.on': 'Atlas ligado',
            'toggle.off': 'Atlas desligado',
            'toggle.onTitle': 'O Hunt Atlas está substituindo o mapa padrão. Clique para usar o mapa do PokéIdle.',
            'toggle.offTitle': 'O mapa padrão do PokéIdle está ativo. Clique para ativar o Hunt Atlas.',
            'filter.search': 'Buscar',
            'filter.searchPlaceholder': 'Pokémon, hunt ou região…',
            'filter.region': 'Região',
            'filter.allRegions': 'Todas as regiões',
            'filter.level': 'Nível',
            'filter.min': 'Mín.',
            'filter.max': 'Máx.',
            'filter.minLevelAria': 'Nível mínimo da hunt',
            'filter.maxLevelAria': 'Nível máximo da hunt',
            'filter.availability': 'Disponibilidade',
            'filter.all': 'Todas',
            'filter.unlocked': 'Liberadas',
            'filter.locked': 'Bloqueadas',
            'filter.type': 'Tipo',
            'filter.allTypes': 'Todos os tipos',
            'filter.typeLoading': 'Tipo: carregando…',
            'filter.weakTo': 'Fraco contra',
            'filter.anyWeakness': 'Qualquer fraqueza',
            'filter.weaknessLoading': 'Fraqueza: carregando…',
            'filter.collection': 'Coleção',
            'filter.caughtAndUncaught': 'Capturados + não capturados',
            'filter.uncaughtOnly': 'Só não capturados',
            'filter.caughtOnly': 'Só capturados',
            'filter.sort': 'Ordenar',
            'sort.xp': 'Melhor XP/hora',
            'sort.npcDesc': 'Venda ao NPC · maior primeiro',
            'sort.npcAsc': 'Venda ao NPC · menor primeiro',
            'sort.playerDesc': 'Mercado de jogadores · maior primeiro',
            'sort.playerAsc': 'Mercado de jogadores · menor primeiro',
            'sort.matchup': 'Melhor matchup',
            'sort.spawnDesc': 'Maior chance de encontro',
            'sort.levelAsc': 'Menor nível de hunt',
            'sort.levelDesc': 'Maior nível de hunt',
            'sort.pokedex': 'Número da Pokédex',
            'sort.name': 'Nome A–Z',
            'filter.clear': 'Limpar filtros',
            'filter.defaults': 'Filtros padrão',
            'filter.changedOne': '1 filtro alterado',
            'filter.changedMany': '{count} filtros alterados',
            'footer.by': 'por MOTHblank',
            'footer.play': 'MOTHblank no Google Play',
            'footer.x': 'MOTHblank no X',
            'footer.whatsapp': 'WhatsApp / Pix',
            'footer.source': 'código-fonte',
            'capture.caught': 'capturado',
            'capture.notCaught': 'não capturado',
            'capture.typeUnknown': 'tipo ?',
            'common.lead': 'Líder',
            'price.npc': 'NPC ~{value}',
            'price.players': 'Jogadores ~{value}',
            'price.npcTip': 'Referência de venda ao NPC no{plural} nível{plural} de hunt visível{plural} e qualidade 1,0{range} · o valor real varia com a qualidade; shiny ×10',
            'price.playerTip': 'Vendas recentes concluídas de Pokémon não shiny no Mercado de jogadores · {count} {sample} · faixa {min}–{max}',
            'price.sample': 'amostra',
            'price.samples': 'amostras',
            'price.range': ' · faixa {min}–{max}',
            'matchup.immune': 'imune',
            'matchup.weak': 'fraco',
            'matchup.resists': 'resiste',
            'matchup.neutral': 'neutro',
            'matchup.deal': 'Causa {multiplier} · {relation}',
            'matchup.take': 'Recebe {multiplier} · {relation}',
            'matchup.dealTitle': 'Melhor tipo de ataque conhecido de {lead} ({type}) contra {species}: {multiplier}.',
            'matchup.takeTitle': 'Melhor STAB de {species} ({type}) contra {lead}: {multiplier}.',
            'hunt.level': 'Nv {level}',
            'hunt.weight': '~{value}% peso',
            'hunt.unlocked': 'liberada',
            'hunt.locked': 'bloqueada',
            'hunt.here': 'Aqui',
            'hunt.go': 'Ir',
            'hunt.requiresLevel': 'Requer nível {level}',
            'hunt.current': 'Hunt atual',
            'hunt.travel': 'Ir para esta hunt',
            'hunt.measuredXp': 'medido {value} XP de treinador/h',
            'hunt.modeledXp': 'modelo de combate {value} XP de treinador/h',
            'xp.waitingLead': 'Aguardando o Pokémon ativo para calcular XP de treinador/hora.',
            'xp.lead': '{name} Nv {level} · líder · poder {power}',
            'xp.warmStart': 'iniciado com dados do nível mais próximo',
            'xp.kills': '{count} abates',
            'xp.attacks': '{count} ataques',
            'xp.cadence': '{seconds}s entre ataques{detail}',
            'xp.throughput': ' (ritmo observado)',
            'xp.floor': ' (piso de 900ms)',
            'xp.travel': '{seconds}s de deslocamento',
            'xp.hpSamples': '{count} amostras de HP',
            'xp.measuredHunt': '{count} hunt medida',
            'xp.measuredHunts': '{count} hunts medidas',
            'xp.calibrating': 'modelo de combate calibrando',
            'xp.learning': 'aprendendo este Pokémon líder: aguardando ataques e abates reais',
            'xp.killsPerHour': '{count} abates/h',
            'xp.pokemonPerHour': '{value} XP de Pokémon/h',
            'xp.measuredShort': 'medido {value} XP/h',
            'xp.modelShort': 'modelo {value} XP/h',
            'xp.best': 'Melhor XP nos filtros atuais',
            'xp.measuredTrainer': 'medido {value} XP de treinador/h',
            'xp.modeledTrainer': 'modelado {value} XP de treinador/h',
            'xp.noEstimate': 'Nenhuma estimativa de XP liberada corresponde aos filtros atuais.',
            'xp.details': 'Detalhes',
            'xp.hideDetails': 'Ocultar detalhes',
            'xp.recommendationNote': '{lead} · recomendações respeitam os filtros atuais do Atlas.',
            'xp.currentMeasured': ' · atual medido {value} XP de treinador/h',
            'xp.currentModeled': ' · atual modelado {value} XP de treinador/h',
            'xp.modelNote': ' · curva oficial de XP/abate; deslocamento, cadência, dano e velocidade de abate vêm do combate real deste Pokémon líder.',
            'results.showMore': 'Mostrar mais · restam {count} Pokémon',
            'results.noMatch': 'Nenhum Pokémon corresponde aos filtros. Use Limpar filtros para redefinir o Atlas.',
            'results.waiting': 'Aguardando os dados de hunts do PokéIdle. Recarregue a página uma vez após instalar o Hunt Atlas se isto continuar vazio.',
            'results.count': '{pokemon} Pokémon · {hunts} hunts',
            'results.countPartial': '{shown}/{pokemon} Pokémon · {hunts} hunts',
            'status.waitingGameData': 'Aguardando dados do jogo…',
            'status.cachedCatalog': 'Usando o catálogo de hunts em cache até chegarem dados atuais.',
            'status.entered': 'Entrou em {hunt}.',
            'status.huntsLoaded': '{count} regiões de hunt carregadas.',
            'status.gameNotReady': 'A conexão com o jogo ainda não está pronta.',
            'status.traveling': 'Indo para {hunt}…',
            'status.notAccepted': 'O PokéIdle não aceitou {hunt}. A área pode estar bloqueada ou a troca de hunt pode estar em cooldown.',
            'status.couldNotSend': 'Não foi possível enviar a troca de hunt.',
            'status.huntLocked': '{hunt} está bloqueada — requer nível {level}.',
            'status.alreadyHere': 'Você já está caçando em {hunt}.',
            'status.loadingTypes': ' · carregando tipos',
            'status.typesUnavailable': ' · dados de tipo indisponíveis',
            'status.marketSampling': ' · amostrando preços do Mercado',
            'type.NORMAL': 'Normal',
            'type.FIRE': 'Fogo',
            'type.WATER': 'Água',
            'type.ELECTRIC': 'Elétrico',
            'type.GRASS': 'Planta',
            'type.ICE': 'Gelo',
            'type.FIGHTING': 'Lutador',
            'type.POISON': 'Veneno',
            'type.GROUND': 'Terra',
            'type.FLYING': 'Voador',
            'type.PSYCHIC': 'Psíquico',
            'type.BUG': 'Inseto',
            'type.ROCK': 'Pedra',
            'type.GHOST': 'Fantasma',
            'type.DRAGON': 'Dragão',
            'type.DARK': 'Sombrio',
            'type.STEEL': 'Aço',
            'type.FAIRY': 'Fada'
        }
    };

    function currentLocale() {
        let gameLanguage = null;

        try {
            gameLanguage =
                localStorage.getItem(
                    'idioma'
                );
        } catch {}

        if (gameLanguage) {
            return gameLanguage ===
                'pt'
                ? 'pt-BR'
                : 'en';
        }

        const htmlLanguage =
            String(
                document.documentElement
                    ?.lang ||
                ''
            ).toLowerCase();

        if (htmlLanguage) {
            return htmlLanguage.startsWith(
                'pt'
            )
                ? 'pt-BR'
                : 'en';
        }

        const browserLanguage =
            String(
                navigator.language ||
                ''
            ).toLowerCase();

        return browserLanguage.startsWith(
            'pt'
        )
            ? 'pt-BR'
            : 'en';
    }

    function tr(
        key,
        params = null
    ) {
        const dictionary =
            TEXT[currentLocale()] ||
            TEXT.en;

        let value =
            dictionary[key] ??
            TEXT.en[key] ??
            key;

        if (!params) {
            return value;
        }

        return value.replace(
            /\{(\w+)\}/g,
            (
                whole,
                name
            ) =>
                params[name] !==
                    undefined
                    ? String(
                        params[name]
                    )
                    : whole
        );
    }

    function typeLabel(
        type
    ) {
        return tr(
            'type.' +
            String(
                type ||
                ''
            ).toUpperCase()
        );
    }

    function localizedNumber(
        value
    ) {
        return Number(
            value ||
            0
        ).toLocaleString(
            currentLocale()
        );
    }

    function loadAtlasEnabled() {
        try {
            return (
                localStorage.getItem(
                    ENABLED_KEY
                ) !== '0'
            );
        } catch {
            return true;
        }
    }

    function saveAtlasEnabled(
        enabled
    ) {
        try {
            localStorage.setItem(
                ENABLED_KEY,
                enabled
                    ? '1'
                    : '0'
            );
        } catch {}
    }

    const q =
        (selector, root = document) =>
            root.querySelector(selector);

    const qa =
        (selector, root = document) =>
            [...root.querySelectorAll(selector)];

    const initialMarketCache =
        loadMarketCache();

    const state = {
        hunts: [],
        playerLevel: 0,
        currentHuntSlug: null,

        pokedex: new Map(),
        ownedSpecies: new Set(),
        pokemons: new Map(),
        strongestPokemon: null,
        activePokemonId: null,

        huntPerf: loadPerformance(),

        marketValues:
            initialMarketCache.values,
        marketCacheSavedAt:
            initialMarketCache.savedAt,
        marketStatus:
            initialMarketCache.values.size
                ? 'cached'
                : 'idle',
        marketFetch: null,

        combatTargets: new Map(),
        speciesCombat: new Map(),
        combatCatalogStatus: 'idle',
        combatCatalogPromise: null,
        combatRevision: 0,
        combatProfileCache: null,
        estimateCache: new Map(),
        estimateCacheKey: '',

        serverTypeChart: null,
        huntAmplification: 1.5,

        typesBySpecies: new Map(),
        typeStatus: 'idle',

        lockOverrides: new Map(),

        drawerOpen: false,
        atlasEnabled:
            loadAtlasEnabled(),
        lastLocale: null,
        resultLimit: 120,
        xpDetailsOpen: false,
        mapObserver: null,
        mapSyncQueued: false,
        keyboardBound: false,
        websocketHookInstalled: false,
        sockets: 0,
        activeSocket: null,
        pendingTravel: null,

        status: {
            key:
                'status.waitingGameData',
            params: {}
        },

        filters: loadFilters(),

        renderQueued: false
    };

    function normalize(value) {
        return String(value ?? '')
            .normalize('NFD')
            .replace(/[\u0300-\u036f]/g, '')
            .toLowerCase()
            .replace(/\s+/g, ' ')
            .trim();
    }

    function escapeHtml(value) {
        return String(value ?? '')
            .replaceAll('&', '&amp;')
            .replaceAll('<', '&lt;')
            .replaceAll('>', '&gt;')
            .replaceAll('"', '&quot;')
            .replaceAll("'", '&#039;');
    }

    function defaultFilters() {
        return {
            search: '',
            region: 'all',
            minLevel: '',
            maxLevel: '',
            type: 'all',
            weakness: 'all',
            availability: 'unlocked',
            captured: 'all',
            sort: 'xp'
        };
    }

    function loadFilters() {
        const defaults =
            defaultFilters();

        try {
            return {
                ...defaults,
                ...JSON.parse(
                    localStorage.getItem(
                        FILTER_KEY
                    ) || '{}'
                )
            };
        } catch {
            return defaults;
        }
    }

    function activeFilterCount() {
        const defaults =
            defaultFilters();

        return Object.keys(
            defaults
        ).filter(
            key =>
                key !== 'sort' &&
                String(
                    state.filters[key] ??
                    ''
                ) !==
                String(
                    defaults[key] ??
                    ''
                )
        ).length;
    }

    function saveFilters() {
        localStorage.setItem(
            FILTER_KEY,
            JSON.stringify(
                state.filters
            )
        );
    }

    function setStatus(
        key,
        params = {}
    ) {
        state.status = {
            key,
            params
        };
    }

    function localizedStatus() {
        if (
            typeof state.status ===
                'string'
        ) {
            return state.status;
        }

        return tr(
            state.status?.key ||
                'status.waitingGameData',
            state.status?.params ||
                {}
        );
    }

    function loadMarketCache() {
        try {
            const raw =
                JSON.parse(
                    localStorage.getItem(
                        MARKET_CACHE_KEY
                    ) || 'null'
                );

            const savedAt =
                Number(
                    raw?.savedAt ||
                    0
                );

            if (
                !savedAt ||
                Date.now() - savedAt >
                    MARKET_CACHE_MAX_AGE_MS ||
                !raw?.values ||
                typeof raw.values !==
                    'object'
            ) {
                return {
                    savedAt: 0,
                    values: new Map()
                };
            }

            return {
                savedAt,
                values:
                    new Map(
                        Object.entries(
                            raw.values
                        )
                    )
            };
        } catch {
            return {
                savedAt: 0,
                values: new Map()
            };
        }
    }

    function saveMarketCache() {
        try {
            localStorage.setItem(
                MARKET_CACHE_KEY,
                JSON.stringify({
                    savedAt:
                        state.marketCacheSavedAt,
                    values:
                        Object.fromEntries(
                            state.marketValues
                        )
                })
            );
        } catch {}
    }

    function loadPerformance() {
        try {
            const raw =
                JSON.parse(
                    localStorage.getItem(
                        PERF_KEY
                    ) || '{}'
                );

            return new Map(
                Object.entries(raw)
            );
        } catch {
            return new Map();
        }
    }

    function savePerformance() {
        try {
            const entries =
                [...state.huntPerf.entries()]
                    .sort(
                        (a, b) =>
                            Number(
                                b[1]?.updatedAt ||
                                0
                            ) -
                            Number(
                                a[1]?.updatedAt ||
                                0
                            )
                    );

            /*
             * Records are level-specific for accuracy. Keep a generous
             * rolling history without letting localStorage grow forever.
             */
            if (
                entries.length >
                160
            ) {
                for (
                    const [
                        key
                    ] of
                    entries.slice(
                        160
                    )
                ) {
                    state.huntPerf.delete(
                        key
                    );
                }
            }

            localStorage.setItem(
                PERF_KEY,
                JSON.stringify(
                    Object.fromEntries(
                        state.huntPerf
                    )
                )
            );
        } catch {}
    }

    function pokemonPower(
        pokemon
    ) {
        const direct =
            Number(
                pokemon?.poder
            );

        if (
            Number.isFinite(direct) &&
            direct > 0
        ) {
            return direct;
        }

        const stats =
            pokemon?.stats || {};

        return [
            stats.hp,
            stats.atk,
            stats.def,
            stats.spAtk,
            stats.spDef,
            stats.speed
        ]
            .map(Number)
            .filter(Number.isFinite)
            .reduce(
                (sum, value) =>
                    sum + value,
                0
            );
    }

    function refreshStrongestPokemon() {
        const owned =
            [...state.pokemons.values()];

        state.strongestPokemon =
            owned
                .sort(
                    (a, b) =>
                        pokemonPower(b) -
                            pokemonPower(a) ||
                        Number(b.level || 0) -
                            Number(a.level || 0)
                )[0] || null;
    }

    function strongestSignature() {
        const pokemon =
            state.strongestPokemon;

        if (!pokemon) {
            return null;
        }

        return [
            pokemon.id,
            pokemon.speciesId,
            pokemon.level,
            Math.round(
                pokemonPower(pokemon)
            )
        ].join(':');
    }

    function formatRate(
        value
    ) {
        const n =
            Number(value);

        if (
            !Number.isFinite(n) ||
            n <= 0
        ) {
            return '—';
        }

        if (n >= 1000000) {
            return (
                n / 1000000
            ).toFixed(
                n >= 10000000
                    ? 0
                    : 1
            ) + 'm';
        }

        if (n >= 1000) {
            return (
                n / 1000
            ).toFixed(
                n >= 100000
                    ? 0
                    : 1
            ) + 'k';
        }

        return Math.round(n)
            .toLocaleString();
    }

    function leadPokemon() {
        const active =
            state.pokemons.get(
                Number(
                    state.activePokemonId
                )
            );

        return (
            active ||
            state.strongestPokemon ||
            null
        );
    }

    function leadLoadoutSignature(
        pokemon =
            leadPokemon()
    ) {
        if (!pokemon) {
            return null;
        }

        return [
            pokemon.id,
            pokemon.speciesId,
            pokemon.tmElemental,
            pokemon.tmAoe,
            pokemon.heldItemId,
            pokemon.megaId ||
                pokemon.mega ||
                ''
        ]
            .map(
                value =>
                    value ??
                    ''
            )
            .join(':');
    }

    function leadSignature(
        pokemon =
            leadPokemon()
    ) {
        const loadout =
            leadLoadoutSignature(
                pokemon
            );

        if (!loadout) {
            return null;
        }

        /* Exact measurement identity includes level. */
        return (
            loadout +
            ':L' +
            Number(
                pokemon?.level ||
                0
            )
        );
    }
    function currentHunt() {
        return (
            state.hunts.find(
                hunt =>
                    hunt.slug ===
                    state.currentHuntSlug
            ) ||
            null
        );
    }

    function performanceKey(
        huntSlug,
        pokemon =
            leadPokemon()
    ) {
        const signature =
            leadSignature(
                pokemon
            );

        if (
            !huntSlug ||
            !signature
        ) {
            return null;
        }

        return (
            huntSlug +
            '|' +
            signature
        );
    }

    function createPerformanceRecord(
        huntSlug,
        pokemon
    ) {
        return {
            version: 2,
            huntSlug,
            signature:
                leadSignature(
                    pokemon
                ),
            loadoutSignature:
                leadLoadoutSignature(
                    pokemon
                ),
            pokemonId:
                pokemon?.id ??
                null,
            pokemonSpeciesId:
                pokemon?.speciesId ??
                null,
            pokemonName:
                pokemon?.nome ||
                null,
            pokemonLevel:
                Number(
                    pokemon?.level ||
                    0
                ),
            pokemonPower:
                pokemonPower(
                    pokemon
                ),

            kills: 0,
            trainerXp: 0,
            pokemonXp: 0,
            gold: 0,

            killIntervals: [],
            sameTargetIntervals: [],
            transitionIntervals: [],
            allAttackIntervals: [],

            attacks: 0,
            damage: 0,
            moveDamage: {},
            hpSamples: [],
            trainerXpMultSamples: [],
            pokemonXpMultSamples: [],

            firstEventAt: 0,
            lastEventAt: 0,
            lastKillAt: 0,
            lastAttackAt: 0,
            lastAttackSlot: null,

            updatedAt:
                Date.now()
        };
    }

    function performanceRecord(
        huntSlug =
            state.currentHuntSlug,
        create = false
    ) {
        const pokemon =
            leadPokemon();

        const key =
            performanceKey(
                huntSlug,
                pokemon
            );

        if (!key) {
            return null;
        }

        let record =
            state.huntPerf.get(
                key
            );

        if (
            !record &&
            create
        ) {
            record =
                createPerformanceRecord(
                    huntSlug,
                    pokemon
                );

            state.huntPerf.set(
                key,
                record
            );
        }

        return record;
    }

    function pushSample(
        record,
        key,
        value,
        max = 80
    ) {
        const number =
            Number(value);

        if (
            !Number.isFinite(
                number
            ) ||
            number < 0
        ) {
            return;
        }

        if (
            !Array.isArray(
                record[key]
            )
        ) {
            record[key] = [];
        }

        record[key].push(
            number
        );

        if (
            record[key].length >
            max
        ) {
            record[key].splice(
                0,
                record[key].length -
                    max
            );
        }
    }

    function robustMean(
        values
    ) {
        const clean =
            (
                Array.isArray(values)
                    ? values
                    : []
            )
                .map(Number)
                .filter(
                    value =>
                        Number.isFinite(
                            value
                        ) &&
                        value >= 0
                )
                .sort(
                    (a, b) =>
                        a - b
                );

        if (!clean.length) {
            return null;
        }

        let from = 0;
        let to =
            clean.length;

        if (
            clean.length >=
            10
        ) {
            const trim =
                Math.floor(
                    clean.length *
                    0.1
                );

            from =
                trim;

            to =
                clean.length -
                trim;
        }

        const slice =
            clean.slice(
                from,
                to
            );

        return (
            slice.reduce(
                (sum, value) =>
                    sum + value,
                0
            ) /
            slice.length
        );
    }

    function average(
        values
    ) {
        const clean =
            (
                Array.isArray(values)
                    ? values
                    : []
            )
                .map(Number)
                .filter(
                    value =>
                        Number.isFinite(
                            value
                        ) &&
                        value >= 0
                );

        if (!clean.length) {
            return null;
        }

        return (
            clean.reduce(
                (sum, value) =>
                    sum + value,
                0
            ) /
            clean.length
        );
    }

    function median(
        values
    ) {
        const clean =
            (
                Array.isArray(values)
                    ? values
                    : []
            )
                .map(Number)
                .filter(
                    Number.isFinite
                )
                .sort(
                    (a, b) =>
                        a - b
                );

        if (!clean.length) {
            return null;
        }

        const middle =
            Math.floor(
                clean.length /
                2
            );

        return clean.length % 2
            ? clean[middle]
            : (
                clean[
                    middle - 1
                ] +
                clean[
                    middle
                ]
            ) /
                2;
    }

    /*
     * Exact upstream XP curve from shared/sell-value.mjs.
     */
    function xpDoNivel(
        level
    ) {
        const n =
            Math.max(
                1,
                Number(level) ||
                1
            );

        if (n <= 150) {
            return (
                Math.floor(
                    6 *
                        n *
                        n /
                        10
                ) +
                8
            );
        }

        return Math.round(
            13500 *
                Math.pow(
                    n /
                        150,
                    1.25
                )
        );
    }

    function touchRecord(
        record,
        now =
            Date.now()
    ) {
        if (!record) {
            return;
        }

        if (
            !record.firstEventAt
        ) {
            record.firstEventAt =
                now;
        }

        const pokemon =
            leadPokemon();

        if (pokemon) {
            record.pokemonLevel =
                Number(
                    pokemon.level ||
                    0
                );

            record.pokemonPower =
                pokemonPower(
                    pokemon
                );

            record.pokemonSpeciesId =
                pokemon.speciesId ??
                record.pokemonSpeciesId;

            record.pokemonName =
                pokemon.nome ||
                record.pokemonName;
        }

        record.lastEventAt =
            now;

        record.updatedAt =
            now;

        state.combatRevision++;
        state.combatProfileCache =
            null;
    }

    function combatSlot(
        event
    ) {
        const value =
            event?.slot ??
            event?.s;

        if (
            value ===
                undefined ||
            value === null
        ) {
            return null;
        }

        return String(
            value
        );
    }

    function recordTargetHp(
        record,
        target
    ) {
        if (
            !record ||
            !target ||
            target.hpRecorded
        ) {
            return;
        }

        const maxHp =
            Number(
                target.maxHp
            );

        const level =
            Number(
                target.level
            );

        if (
            !Number.isFinite(
                maxHp
            ) ||
            maxHp <= 0 ||
            !Number.isFinite(
                level
            ) ||
            level <= 0
        ) {
            return;
        }

        if (
            !Array.isArray(
                record.hpSamples
            )
        ) {
            record.hpSamples =
                [];
        }

        record.hpSamples.push({
            speciesId:
                Number(
                    target.speciesId
                ) ||
                null,
            level,
            maxHp
        });

        if (
            record.hpSamples.length >
            120
        ) {
            record.hpSamples.splice(
                0,
                record.hpSamples.length -
                    120
            );
        }

        target.hpRecorded =
            true;
    }

    function recordSpawnEvent(
        event
    ) {
        const slot =
            combatSlot(
                event
            );

        if (!slot) {
            return;
        }

        const hp =
            Number(
                event?.maxHp ??
                event?.hp
            );

        state.combatTargets.set(
            slot,
            {
                slot,
                speciesId:
                    Number(
                        event?.speciesId ??
                        event?.pokeId
                    ) ||
                    null,
                name:
                    event?.nome ||
                    null,
                level:
                    Number(
                        event?.level
                    ) ||
                    Number(
                        currentHunt()
                            ?.nivel
                    ) ||
                    1,
                maxHp:
                    Number.isFinite(
                        hp
                    )
                        ? hp
                        : null,
                playerAttacks: 0,
                hpRecorded:
                    false,
                seenAt:
                    Date.now()
            }
        );
    }

    function recordAttackEvent(
        event
    ) {
        if (
            event?.por !==
                'jogador'
        ) {
            return;
        }

        const record =
            performanceRecord(
                state.currentHuntSlug,
                true
            );

        if (!record) {
            return;
        }

        const now =
            Date.now();

        touchRecord(
            record,
            now
        );

        const slot =
            combatSlot(
                event
            );

        if (
            record.lastAttackAt
        ) {
            const gap =
                now -
                record.lastAttackAt;

            /*
             * Multiple AoE damage rows can share one timestamp. Do not
             * mistake those for 0 ms attack cadence.
             */
            if (
                gap >= 100 &&
                gap <= 300000
            ) {
                pushSample(
                    record,
                    'allAttackIntervals',
                    gap
                );

                if (
                    slot &&
                    slot ===
                        record.lastAttackSlot
                ) {
                    pushSample(
                        record,
                        'sameTargetIntervals',
                        gap
                    );
                } else {
                    pushSample(
                        record,
                        'transitionIntervals',
                        gap
                    );
                }
            }
        }

        record.lastAttackAt =
            now;

        record.lastAttackSlot =
            slot;

        record.attacks =
            Number(
                record.attacks ||
                0
            ) +
            1;

        const damage =
            Math.max(
                0,
                Number(
                    event?.dano
                ) ||
                0
            );

        const effectiveness =
            Number(
                event?.ef
            );

        const type =
            typeof event?.tipo ===
                'string'
                ? String(
                    event.tipo
                ).toUpperCase()
                : 'UNKNOWN';

        record.damage =
            Number(
                record.damage ||
                0
            ) +
            damage;

        if (
            damage > 0 &&
            Number.isFinite(
                effectiveness
            ) &&
            effectiveness > 0
        ) {
            if (
                !record.moveDamage ||
                typeof record.moveDamage !==
                    'object'
            ) {
                record.moveDamage =
                    {};
            }

            const sample =
                record.moveDamage[
                    type
                ] || {
                    count: 0,
                    neutralSamples: [],
                    actualSamples: []
                };

            sample.count++;

            sample.neutralSamples.push(
                damage /
                effectiveness
            );

            sample.actualSamples.push(
                damage
            );

            if (
                sample.neutralSamples.length >
                80
            ) {
                sample.neutralSamples.splice(
                    0,
                    sample.neutralSamples.length -
                        80
                );
            }

            if (
                sample.actualSamples.length >
                80
            ) {
                sample.actualSamples.splice(
                    0,
                    sample.actualSamples.length -
                        80
                );
            }

            record.moveDamage[
                type
            ] =
                sample;
        }

        if (slot) {
            let target =
                state.combatTargets.get(
                    slot
                );

            if (!target) {
                target = {
                    slot,
                    speciesId: null,
                    name: null,
                    level:
                        Number(
                            currentHunt()
                                ?.nivel
                        ) ||
                        1,
                    maxHp: null,
                    playerAttacks: 0,
                    hpRecorded:
                        false,
                    seenAt: now
                };

                state.combatTargets.set(
                    slot,
                    target
                );
            }

            if (
                !Number.isFinite(
                    Number(
                        target.maxHp
                    )
                ) &&
                target.playerAttacks ===
                    0
            ) {
                const hpAfter =
                    Number(
                        event?.hpAlvo
                    );

                if (
                    Number.isFinite(
                        hpAfter
                    ) &&
                    damage > 0
                ) {
                    target.maxHp =
                        Math.max(
                            1,
                            hpAfter +
                                damage
                        );
                }
            }

            target.playerAttacks++;

            recordTargetHp(
                record,
                target
            );
        }
    }

    function recordDeathEvent(
        event
    ) {
        if (
            event?.quem !==
                'selvagem'
        ) {
            return;
        }

        const record =
            performanceRecord(
                state.currentHuntSlug,
                true
            );

        if (!record) {
            return;
        }

        const now =
            Date.now();

        touchRecord(
            record,
            now
        );

        if (
            record.lastKillAt
        ) {
            const gap =
                now -
                record.lastKillAt;

            if (
                gap >= 0 &&
                gap <= 600000
            ) {
                pushSample(
                    record,
                    'killIntervals',
                    gap,
                    40
                );
            }
        }

        record.lastKillAt =
            now;

        record.kills =
            Number(
                record.kills ||
                0
            ) +
            1;

        const trainerXp =
            Math.max(
                0,
                Number(
                    event?.xpTreinador ??
                    event?.xp
                ) ||
                0
            );

        const pokemonXp =
            Math.max(
                0,
                Number(
                    event?.xpPokemon ??
                    event?.xp
                ) ||
                0
            );

        record.trainerXp =
            Number(
                record.trainerXp ||
                0
            ) +
            trainerXp;

        record.pokemonXp =
            Number(
                record.pokemonXp ||
                0
            ) +
            pokemonXp;

        record.gold =
            Number(
                record.gold ||
                0
            ) +
            Math.max(
                0,
                Number(
                    event?.ouro
                ) ||
                0
            ) +
            Math.max(
                0,
                Number(
                    event?.ouroVendaAuto
                ) ||
                0
            );

        const hunt =
            currentHunt();

        const baseXp =
            xpDoNivel(
                hunt?.nivel ||
                event?.level ||
                1
            );

        if (
            trainerXp > 0 &&
            baseXp > 0
        ) {
            pushSample(
                record,
                'trainerXpMultSamples',
                trainerXp /
                    baseXp,
                80
            );
        }

        if (
            pokemonXp > 0 &&
            baseXp > 0
        ) {
            pushSample(
                record,
                'pokemonXpMultSamples',
                pokemonXp /
                    baseXp,
                80
            );
        }

        const slot =
            combatSlot(
                event
            );

        if (slot) {
            const target =
                state.combatTargets.get(
                    slot
                );

            recordTargetHp(
                record,
                target
            );

            state.combatTargets.delete(
                slot
            );
        }

        savePerformance();

        if (
            state.drawerOpen
        ) {
            queueRender();
        }
    }

    function processCombatEvents(
        events
    ) {
        if (
            !Array.isArray(
                events
            ) ||
            !state.currentHuntSlug
        ) {
            return;
        }

        for (
            const event of
            events
        ) {
            if (
                event?.k ===
                    'spawn'
            ) {
                recordSpawnEvent(
                    event
                );
            } else if (
                event?.k ===
                    'ataque'
            ) {
                recordAttackEvent(
                    event
                );
            } else if (
                event?.k ===
                    'morte'
            ) {
                recordDeathEvent(
                    event
                );
            }
        }
    }

    function loadCache() {
        try {
            const cache =
                JSON.parse(
                    localStorage.getItem(
                        CACHE_KEY
                    ) || 'null'
                );

            if (!cache) {
                return;
            }

            if (
                Array.isArray(
                    cache.hunts
                )
            ) {
                state.hunts =
                    cache.hunts;
            }

            state.playerLevel =
                Number(
                    cache.playerLevel ||
                    0
                );

            state.currentHuntSlug =
                cache.currentHuntSlug ||
                null;

            if (
                cache.pokedex &&
                typeof cache.pokedex ===
                    'object'
            ) {
                state.pokedex =
                    new Map(
                        Object.entries(
                            cache.pokedex
                        )
                    );
            }

            if (
                Array.isArray(
                    cache.ownedSpecies
                )
            ) {
                state.ownedSpecies =
                    new Set(
                        cache.ownedSpecies
                            .map(Number)
                    );
            }

            if (
                state.hunts.length
            ) {
                setStatus(
                    'status.cachedCatalog'
                );
            }
        } catch {}
    }

    function saveCache() {
        try {
            localStorage.setItem(
                CACHE_KEY,
                JSON.stringify({
                    hunts:
                        state.hunts,
                    playerLevel:
                        state.playerLevel,
                    currentHuntSlug:
                        state.currentHuntSlug,
                    pokedex:
                        Object.fromEntries(
                            state.pokedex
                        ),
                    ownedSpecies:
                        [...state.ownedSpecies]
                })
            );
        } catch {}
    }

    loadCache();

    // ------------------------------------------------------------------
    // PokéIdle protocol
    // ------------------------------------------------------------------

    function mergePokedex(value) {
        if (
            !value ||
            typeof value !== 'object'
        ) {
            return;
        }

        for (
            const [id, entry] of
            Object.entries(value)
        ) {
            state.pokedex.set(
                String(id),
                entry
            );
        }
    }

    function mergeOwnedPokemon(list) {
        if (!Array.isArray(list)) {
            return;
        }

        for (const pokemon of list) {
            const id =
                Number(
                    pokemon?.speciesId
                );

            if (
                Number.isFinite(id)
            ) {
                state.ownedSpecies.add(
                    id
                );
            }

            if (
                pokemon?.id !==
                    undefined &&
                pokemon?.id !==
                    null
            ) {
                state.pokemons.set(
                    Number(pokemon.id),
                    {
                        ...(state.pokemons.get(
                            Number(pokemon.id)
                        ) || {}),
                        ...pokemon
                    }
                );
            }

            if (
                Number.isFinite(id) &&
                Array.isArray(
                    pokemon?.tipos
                )
            ) {
                state.typesBySpecies.set(
                    id,
                    pokemon.tipos.map(
                        type =>
                            String(type)
                                .toUpperCase()
                    )
                );
            }
        }

        refreshStrongestPokemon();
    }

    function mergeState(
        gameState,
        full = false
    ) {
        if (
            !gameState ||
            typeof gameState !==
                'object'
        ) {
            return;
        }

        if (
            Number.isFinite(
                Number(gameState.level)
            )
        ) {
            state.playerLevel =
                Number(
                    gameState.level
                );
        }

        if (
            typeof gameState.huntSlug ===
                'string'
        ) {
            state.currentHuntSlug =
                gameState.huntSlug;
        }

        if (
            gameState.activeId !==
                undefined &&
            gameState.activeId !==
                null
        ) {
            const nextActiveId =
                Number(
                    gameState.activeId
                );

            if (
                Number.isFinite(
                    nextActiveId
                ) &&
                nextActiveId !==
                    Number(
                        state.activePokemonId
                    )
            ) {
                state.combatTargets.clear();
            }

            state.activePokemonId =
                nextActiveId;
        }

        mergePokedex(
            gameState.pokedex ||
            gameState.dexMud
        );

        mergeOwnedPokemon(
            gameState.pokemons
        );

        mergeOwnedPokemon(
            gameState.pkMud
        );

        if (
            gameState.selvagem &&
            Number.isFinite(
                Number(
                    gameState.selvagem
                        .speciesId
                )
            ) &&
            Array.isArray(
                gameState.selvagem.tipos
            )
        ) {
            state.typesBySpecies.set(
                Number(
                    gameState.selvagem
                        .speciesId
                ),
                gameState.selvagem
                    .tipos
                    .map(
                        type =>
                            String(type)
                                .toUpperCase()
                    )
            );
        }

        if (full) {
            saveCache();
        }
    }

    function handleHuntBattleEvent(message) {
        if (
            message?.t !== 'batalha' ||
            !Array.isArray(message.ev)
        ) {
            return false;
        }

        const huntEvent =
            message.ev.find(
                event =>
                    event?.k === 'hunt' &&
                    typeof event.slug === 'string'
            );

        if (!huntEvent) {
            return false;
        }

        state.currentHuntSlug =
            huntEvent.slug;

        state.combatTargets.clear();

        const enteredRecord =
            performanceRecord(
                huntEvent.slug,
                false
            );

        if (enteredRecord) {
            enteredRecord.lastKillAt =
                0;

            enteredRecord.lastAttackAt =
                0;

            enteredRecord.lastAttackSlot =
                null;
        }

        setStatus(
            'status.entered',
            {
                hunt:
                    huntEvent.nome ||
                    huntEvent.slug
            }
        );

        state.pendingTravel = null;

        saveCache();
        queueRender();

        return true;
    }

    function handleProtocolMessage(
        data
    ) {
        if (typeof data !== 'string') {
            return;
        }

        let message;

        try {
            message =
                JSON.parse(data);
        } catch {
            return;
        }

        if (
            message?.t === 'batalha' &&
            Array.isArray(message.ev)
        ) {
            /*
             * Hunt transition first so spawns/attacks in the same packet
             * are attributed to the new hunt.
             */
            handleHuntBattleEvent(
                message
            );

            processCombatEvents(
                message.ev
            );

            return;
        }

        if (
            message?.t ===
                'market' &&
            message?.aba ===
                'historicoGlobal'
        ) {
            handleMarketHistory(
                message
            );
            return;
        }

        if (
            message?.t ===
                'welcome'
        ) {
            if (
                message.tabelaTipos &&
                typeof message.tabelaTipos ===
                    'object'
            ) {
                state.serverTypeChart =
                    message.tabelaTipos;
            }

            if (
                Number.isFinite(
                    Number(
                        message.ampliacaoHunt
                    )
                )
            ) {
                state.huntAmplification =
                    Number(
                        message.ampliacaoHunt
                    );
            }

            if (
                Array.isArray(
                    message.hunts
                )
            ) {
                state.hunts =
                    message.hunts;

                setStatus(
                    'status.huntsLoaded',
                    {
                        count:
                            state.hunts.length
                    }
                );
            }

            mergeState(
                message.estado,
                true
            );

            saveCache();

            if (
                state.drawerOpen
            ) {
                ensureMarketValues();
            }

            queueRender();

            return;
        }

        if (
            message?.t ===
                'estado'
        ) {
            mergeState(
                message.estado,
                false
            );

            saveCache();
            queueRender();
        }
    }

    function addMarketSample(
        bucket,
        key,
        price,
        soldAt
    ) {
        if (
            !key ||
            !Number.isFinite(price) ||
            price <= 0
        ) {
            return;
        }

        const previous =
            bucket.get(
                key
            ) || {
                sum: 0,
                count: 0,
                min: price,
                max: price,
                lastSaleAt: 0
            };

        previous.sum +=
            price;
        previous.count++;
        previous.min =
            Math.min(
                previous.min,
                price
            );
        previous.max =
            Math.max(
                previous.max,
                price
            );
        previous.lastSaleAt =
            Math.max(
                Number(
                    previous.lastSaleAt ||
                    0
                ),
                Number(
                    soldAt ||
                    0
                )
            );

        bucket.set(
            key,
            previous
        );
    }

    function processMarketSale(
        line,
        bucket
    ) {
        if (
            line?.tipo !==
                'pokemon' ||
            line?.moeda !==
                'gold'
        ) {
            return;
        }

        const ficha =
            line.ficha ||
            {};

        if (ficha.shiny) {
            return;
        }

        const price =
            Number(
                line.bruto ??
                line.preco
            );

        if (
            !Number.isFinite(price) ||
            price <= 0
        ) {
            return;
        }

        const speciesId =
            Number(
                ficha.speciesId ??
                ficha.pokeId
            );

        if (
            Number.isFinite(
                speciesId
            ) &&
            speciesId > 0
        ) {
            addMarketSample(
                bucket,
                'id:' + speciesId,
                price,
                line.em
            );
        }

        const name =
            normalize(
                ficha.nome ||
                ficha.name ||
                ''
            );

        if (name) {
            addMarketSample(
                bucket,
                'name:' + name,
                price,
                line.em
            );
        }
    }

    function finalizeMarketFetch() {
        const fetch =
            state.marketFetch;

        if (!fetch) {
            return;
        }

        const now =
            Date.now();

        for (
            const [key, sample] of
            fetch.batch
        ) {
            if (
                !sample.count
            ) {
                continue;
            }

            state.marketValues.set(
                key,
                {
                    average:
                        sample.sum /
                        sample.count,
                    count:
                        sample.count,
                    min:
                        sample.min,
                    max:
                        sample.max,
                    lastSaleAt:
                        sample.lastSaleAt,
                    sampledAt:
                        now
                }
            );
        }

        state.marketCacheSavedAt =
            now;
        state.marketStatus =
            'ready';
        state.marketFetch =
            null;

        saveMarketCache();
        queueRender();
    }

    function requestMarketHistoryPage() {
        const fetch =
            state.marketFetch;

        if (
            !fetch ||
            !state.activeSocket ||
            state.activeSocket
                .readyState !==
                page.WebSocket.OPEN
        ) {
            return false;
        }

        if (
            q('#cm-hist-corpo')
        ) {
            finalizeMarketFetch();
            return false;
        }

        try {
            state.activeSocket.send(
                JSON.stringify({
                    t:
                        'market.historicoGlobal',
                    pagina:
                        fetch.page,
                    tipo:
                        'pokemon',
                    moeda:
                        'gold'
                })
            );

            return true;
        } catch {
            state.marketFetch =
                null;
            state.marketStatus =
                state.marketValues.size
                    ? 'cached'
                    : 'error';
            return false;
        }
    }

    function ensureMarketValues() {
        if (
            state.marketFetch ||
            (
                state.marketCacheSavedAt &&
                Date.now() -
                    state.marketCacheSavedAt <
                    MARKET_REFRESH_MS
            )
        ) {
            return;
        }

        if (
            !state.activeSocket ||
            state.activeSocket
                .readyState !==
                page.WebSocket.OPEN ||
            q('#cm-hist-corpo')
        ) {
            return;
        }

        state.marketFetch = {
            page: 0,
            batch:
                new Map()
        };

        state.marketStatus =
            'loading';

        requestMarketHistoryPage();
        queueRender();
    }

    function handleMarketHistory(
        message
    ) {
        const fetch =
            state.marketFetch;

        if (
            !fetch ||
            Number(
                message?.pagina ??
                0
            ) !==
                fetch.page
        ) {
            return;
        }

        for (
            const line of
            message.linhas ||
            []
        ) {
            processMarketSale(
                line,
                fetch.batch
            );
        }

        if (
            message.temMais &&
            fetch.page + 1 <
                MARKET_HISTORY_MAX_PAGES &&
            !q('#cm-hist-corpo')
        ) {
            fetch.page++;
            requestMarketHistoryPage();
            return;
        }

        finalizeMarketFetch();
    }

    function attachSocket(
        socket
    ) {
        state.sockets++;
        state.activeSocket =
            socket;

        socket.addEventListener(
            'close',
            () => {
                if (
                    state.activeSocket ===
                    socket
                ) {
                    state.activeSocket =
                        null;

                    if (
                        state.marketFetch
                    ) {
                        state.marketFetch =
                            null;
                        state.marketStatus =
                            state.marketValues.size
                                ? 'cached'
                                : 'idle';
                    }
                }
            }
        );

        socket.addEventListener(
            'message',
            event => {
                if (
                    typeof Blob !==
                        'undefined' &&
                    event.data instanceof
                        Blob
                ) {
                    event.data.text()
                        .then(
                            handleProtocolMessage
                        )
                        .catch(() => {});

                    return;
                }

                handleProtocolMessage(
                    event.data
                );
            }
        );
    }

    function installWebSocketHook() {
        if (
            state.websocketHookInstalled
        ) {
            return true;
        }

        const NativeWebSocket =
            page.WebSocket;

        if (
            typeof NativeWebSocket !==
                'function'
        ) {
            return false;
        }

        if (
            page.__mothHuntAtlasWebSocket
        ) {
            state.websocketHookInstalled =
                true;

            return true;
        }

        const WrappedWebSocket =
            new Proxy(
                NativeWebSocket,
                {
                    construct(
                        target,
                        args
                    ) {
                        const socket =
                            Reflect.construct(
                                target,
                                args,
                                target
                            );

                        attachSocket(
                            socket
                        );

                        return socket;
                    }
                }
            );

        try {
            page.WebSocket =
                WrappedWebSocket;

            page.__mothHuntAtlasWebSocket =
                true;

            state.websocketHookInstalled =
                page.WebSocket ===
                WrappedWebSocket;

            return state.websocketHookInstalled;
        } catch {
            return false;
        }
    }

    installWebSocketHook();

    // ------------------------------------------------------------------
    // Type data
    // ------------------------------------------------------------------

    function loadTypeCache() {
        try {
            const cached =
                JSON.parse(
                    localStorage.getItem(
                        TYPE_CACHE_KEY
                    ) || 'null'
                );

            if (
                !cached ||
                !cached.species
            ) {
                return false;
            }

            for (
                const [id, types] of
                Object.entries(
                    cached.species
                )
            ) {
                if (
                    Array.isArray(types)
                ) {
                    state.typesBySpecies.set(
                        Number(id),
                        types
                    );
                }
            }

            const age =
                Date.now() -
                Number(
                    cached.savedAt ||
                    0
                );

            state.typeStatus =
                age <
                    30 *
                    24 *
                    60 *
                    60 *
                    1000
                    ? 'ready'
                    : 'stale';

            return true;
        } catch {
            return false;
        }
    }

    loadTypeCache();

    function requestJson(url) {
        if (
            typeof GM_xmlhttpRequest ===
                'function'
        ) {
            return new Promise(
                (resolve, reject) => {
                    GM_xmlhttpRequest({
                        method: 'GET',
                        url,
                        timeout: 15000,

                        onload:
                            response => {
                                if (
                                    response.status <
                                        200 ||
                                    response.status >=
                                        300
                                ) {
                                    reject(
                                        new Error(
                                            'HTTP ' +
                                            response.status
                                        )
                                    );

                                    return;
                                }

                                try {
                                    resolve(
                                        JSON.parse(
                                            response.responseText
                                        )
                                    );
                                } catch (
                                    error
                                ) {
                                    reject(
                                        error
                                    );
                                }
                            },

                        onerror:
                            reject,

                        ontimeout:
                            () =>
                                reject(
                                    new Error(
                                        'timeout'
                                    )
                                )
                    });
                }
            );
        }

        return fetch(url)
            .then(response => {
                if (!response.ok) {
                    throw new Error(
                        'HTTP ' +
                        response.status
                    );
                }

                return response.json();
            });
    }

    function pokemonIdFromUrl(
        url
    ) {
        const match =
            String(url)
                .match(
                    /\/pokemon\/(\d+)\/?$/i
                );

        if (!match) {
            return null;
        }

        const id =
            Number(match[1]);

        return Number.isFinite(id)
            ? id
            : null;
    }

    let typeLoadPromise =
        null;

    function ensureTypeData() {
        if (
            state.typeStatus ===
                'ready'
        ) {
            return Promise.resolve();
        }

        if (typeLoadPromise) {
            return typeLoadPromise;
        }

        state.typeStatus =
            'loading';

        queueRender();

        typeLoadPromise =
            Promise.all(
                STANDARD_TYPES.map(
                    async type => {
                        const data =
                            await requestJson(
                                'https://pokeapi.co/api/v2/type/' +
                                type.toLowerCase()
                            );

                        return {
                            type,
                            pokemon:
                                Array.isArray(
                                    data?.pokemon
                                )
                                    ? data.pokemon
                                    : []
                        };
                    }
                )
            )
                .then(
                    results => {
                        const discovered =
                            new Map();

                        for (
                            const result of
                            results
                        ) {
                            for (
                                const row of
                                result.pokemon
                            ) {
                                const id =
                                    pokemonIdFromUrl(
                                        row?.pokemon
                                            ?.url
                                    );

                                if (
                                    !id ||
                                    id >
                                        10000
                                ) {
                                    continue;
                                }

                                const types =
                                    discovered.get(
                                        id
                                    ) || [];

                                if (
                                    !types.includes(
                                        result.type
                                    )
                                ) {
                                    types.push(
                                        result.type
                                    );
                                }

                                discovered.set(
                                    id,
                                    types
                                );
                            }
                        }

                        for (
                            const [id, types] of
                            discovered
                        ) {
                            state.typesBySpecies.set(
                                id,
                                types
                            );
                        }

                        state.typeStatus =
                            'ready';

                        try {
                            localStorage.setItem(
                                TYPE_CACHE_KEY,
                                JSON.stringify({
                                    savedAt:
                                        Date.now(),
                                    species:
                                        Object.fromEntries(
                                            [...state.typesBySpecies]
                                                .map(
                                                    ([id, types]) => [
                                                        String(id),
                                                        types
                                                    ]
                                                )
                                        )
                                })
                            );
                        } catch {}

                        queueRender();
                    }
                )
                .catch(
                    error => {
                        console.warn(
                            '[PokéIdle Hunt Atlas] type data unavailable',
                            error
                        );

                        state.typeStatus =
                            'error';

                        queueRender();
                    }
                )
                .finally(
                    () => {
                        typeLoadPromise =
                            null;
                    }
                );

        return typeLoadPromise;
    }

    function weaknessMultiplier(
        speciesTypes,
        attackType
    ) {
        const chart =
            TYPE_CHART[
                attackType
            ] || {};

        return speciesTypes.reduce(
            (multiplier, defender) =>
                multiplier *
                (
                    chart[
                        defender
                    ] ?? 1
                ),
            1
        );
    }

    // ------------------------------------------------------------------
    // Upstream combat catalog
    // ------------------------------------------------------------------

    function mergeCombatCreature(
        creature
    ) {
        const id =
            Number(
                creature?.pokeId
            );

        if (
            !Number.isFinite(id)
        ) {
            return;
        }

        const previous =
            state.speciesCombat.get(
                id
            ) || {};

        const meta = {
            ...previous,
            id,
            name:
                creature?.name ||
                previous.name ||
                null,
            baseHp:
                Number.isFinite(
                    Number(
                        creature?.baseHp
                    )
                )
                    ? Number(
                        creature.baseHp
                    )
                    : previous.baseHp,
            type1:
                creature?.type1 ||
                previous.type1 ||
                null,
            type2:
                creature?.type2 ||
                previous.type2 ||
                null,
            sellValue:
                Number.isFinite(
                    Number(
                        creature?.sellValue ??
                        creature?.priceNpc
                    )
                )
                    ? Number(
                        creature.sellValue ??
                        creature.priceNpc
                    )
                    : previous.sellValue
        };

        state.speciesCombat.set(
            id,
            meta
        );

        const types =
            [
                meta.type1,
                meta.type2
            ]
                .filter(Boolean)
                .map(
                    type =>
                        String(type)
                            .toUpperCase()
                );

        if (types.length) {
            state.typesBySpecies.set(
                id,
                types
            );
        }
    }

    function speciesCombatMeta(
        speciesId
    ) {
        const id =
            Number(
                speciesId
            );

        if (
            !Number.isFinite(id)
        ) {
            return null;
        }

        const direct =
            state.speciesCombat.get(
                id
            );

        if (direct) {
            return direct;
        }

        /*
         * Orre clones use 13xxx ids and inherit the national species'
         * stats in upstream.
         */
        if (id >= 13000) {
            return (
                state.speciesCombat.get(
                    id %
                        1000
                ) ||
                null
            );
        }

        return null;
    }

    function ensureCombatCatalog() {
        if (
            state.combatCatalogStatus ===
                'ready'
        ) {
            return Promise.resolve();
        }

        if (
            state.combatCatalogPromise
        ) {
            return state.combatCatalogPromise;
        }

        state.combatCatalogStatus =
            'loading';

        const load =
            url =>
                fetch(
                    url,
                    {
                        cache:
                            'force-cache',
                        credentials:
                            'same-origin'
                    }
                )
                    .then(
                        response => {
                            if (
                                !response.ok
                            ) {
                                throw new Error(
                                    'HTTP ' +
                                    response.status
                                );
                            }

                            return response.json();
                        }
                    );

        state.combatCatalogPromise =
            Promise.all([
                load(
                    '/assets/creatures.json'
                ),
                load(
                    '/assets/creatures-novos.json'
                ).catch(
                    () => null
                ),
                load(
                    '/assets/creatures-outland-novos.json'
                ).catch(
                    () => null
                ),
                load(
                    '/assets/creatures-audit-overrides.json'
                ).catch(
                    () => null
                )
            ])
                .then(
                    ([
                        base,
                        novos,
                        outland,
                        audit
                    ]) => {
                        for (
                            const data of [
                                base,
                                novos,
                                outland
                            ]
                        ) {
                            for (
                                const creature of
                                data?.creatures ||
                                []
                            ) {
                                mergeCombatCreature(
                                    creature
                                );
                            }
                        }

                        for (
                            const override of
                            audit?.overrides ||
                            []
                        ) {
                            mergeCombatCreature(
                                override
                            );
                        }

                        state.combatCatalogStatus =
                            'ready';

                        state.combatRevision++;
                        state.combatProfileCache =
                            null;

                        queueRender();
                    }
                )
                .catch(
                    error => {
                        state.combatCatalogStatus =
                            'error';

                        console.warn(
                            '[PokéIdle Hunt Atlas] combat catalog failed',
                            error
                        );
                    }
                )
                .finally(
                    () => {
                        state.combatCatalogPromise =
                            null;
                    }
                );

        return state.combatCatalogPromise;
    }

    function typeMultiplierExact(
        attackType,
        defenderType
    ) {
        const attack =
            String(
                attackType ||
                ''
            ).toUpperCase();

        const defender =
            String(
                defenderType ||
                ''
            ).toUpperCase();

        if (
            !attack ||
            !defender
        ) {
            return 1;
        }

        const chart =
            state.serverTypeChart ||
            TYPE_CHART;

        const row =
            chart[attack] ||
            TYPE_CHART[
                attack
            ] ||
            {};

        if (
            attack ===
                defender &&
            row[defender] ==
                null
        ) {
            if (
                attack ===
                    'GHOST' ||
                attack ===
                    'DRAGON'
            ) {
                return 2;
            }

            if (
                attack ===
                    'NORMAL' ||
                attack ===
                    'GROUND' ||
                attack ===
                    'FLYING'
            ) {
                return 1;
            }

            return 0.5;
        }

        if (
            row[
                defender
            ] != null
        ) {
            const value =
                Number(
                    row[
                        defender
                    ]
                );

            return Number.isFinite(
                value
            )
                ? value
                : 1;
        }

        return 1;
    }

    function exactEffectiveness(
        attackType,
        defenderTypes
    ) {
        let value = 1;

        for (
            const defender of
            defenderTypes ||
            []
        ) {
            value *=
                typeMultiplierExact(
                    attackType,
                    defender
                );
        }

        return value;
    }

    function normalizedTypes(
        values
    ) {
        return [
            ...new Set(
                (
                    Array.isArray(values)
                        ? values
                        : []
                )
                    .map(
                        type =>
                            String(
                                type || ''
                            ).toUpperCase()
                    )
                    .filter(
                        type =>
                            STANDARD_TYPES.includes(
                                type
                            )
                    )
            )
        ];
    }

    function leadDefenderTypes() {
        const lead =
            leadPokemon();

        if (!lead) {
            return [];
        }

        const direct =
            normalizedTypes(
                lead.tipos
            );

        if (direct.length) {
            return direct;
        }

        return normalizedTypes(
            state.typesBySpecies.get(
                Number(
                    lead.speciesId
                )
            )
        );
    }

    function leadAttackTypesForMatchup() {
        const observed =
            normalizedTypes(
                attackerTypes()
            );

        if (observed.length) {
            return observed;
        }

        return leadDefenderTypes();
    }

    function bestEffectivenessAgainst(
        attackTypes,
        defenderTypes
    ) {
        const attacks =
            normalizedTypes(
                attackTypes
            );

        const defenders =
            normalizedTypes(
                defenderTypes
            );

        if (
            !attacks.length ||
            !defenders.length
        ) {
            return null;
        }

        let best = null;

        for (const attackType of attacks) {
            const multiplier =
                exactEffectiveness(
                    attackType,
                    defenders
                );

            if (
                !best ||
                multiplier >
                    best.multiplier
            ) {
                best = {
                    attackType,
                    multiplier
                };
            }
        }

        return best;
    }

    function matchupRelation(
        multiplier
    ) {
        if (
            multiplier === null ||
            multiplier === undefined ||
            !Number.isFinite(
                Number(
                    multiplier
                )
            )
        ) {
            return {
                key: 'unknown',
                label: '?'
            };
        }

        const value =
            Number(
                multiplier
            );

        if (value === 0) {
            return {
                key: 'immune',
                label:
                    tr(
                        'matchup.immune'
                    )
            };
        }

        if (value > 1) {
            return {
                key: 'weak',
                label:
                    tr(
                        'matchup.weak'
                    )
            };
        }

        if (value < 1) {
            return {
                key: 'resist',
                label:
                    tr(
                        'matchup.resists'
                    )
            };
        }

        return {
            key: 'neutral',
            label:
                tr(
                    'matchup.neutral'
                )
        };
    }

    function formatMultiplier(
        value
    ) {
        const number =
            Number(
                value
            );

        if (!Number.isFinite(number)) {
            return '×?';
        }

        return (
            '×' +
            (
                Number.isInteger(number)
                    ? String(number)
                    : String(
                        Math.round(
                            number * 100
                        ) / 100
                    )
            )
        );
    }

    function speciesMatchup(
        species
    ) {
        const lead =
            leadPokemon();

        if (!lead) {
            return null;
        }

        const targetTypes =
            normalizedTypes(
                species?.types
            );

        const leadDefense =
            leadDefenderTypes();

        if (
            !targetTypes.length ||
            !leadDefense.length
        ) {
            return null;
        }

        const leadAttackTypes =
            leadAttackTypesForMatchup();

        const offense =
            bestEffectivenessAgainst(
                leadAttackTypes,
                targetTypes
            );

        /*
         * PokéIdle's own Hunt Analyser treats a wild Pokémon's own types as
         * its STAB attack types for matchup analysis. Mirror that rule here.
         */
        const defense =
            bestEffectivenessAgainst(
                targetTypes,
                leadDefense
            );

        return {
            lead,
            targetTypes,
            leadDefense,
            offense,
            defense
        };
    }

    function matchupBadgeMarkup(
        direction,
        result,
        leadName,
        speciesName
    ) {
        if (!result) {
            return '';
        }

        const relation =
            matchupRelation(
                result.multiplier
            );

        const multiplier =
            formatMultiplier(
                result.multiplier
            );

        const isDeal =
            direction === 'deal';

        const label =
            tr(
                isDeal
                    ? 'matchup.deal'
                    : 'matchup.take',
                {
                    multiplier,
                    relation:
                        relation.label
                }
            );

        const title =
            tr(
                isDeal
                    ? 'matchup.dealTitle'
                    : 'matchup.takeTitle',
                {
                    lead:
                        leadName,
                    species:
                        speciesName,
                    type:
                        typeLabel(
                            result.attackType
                        ),
                    multiplier
                }
            );

        const favorability =
            isDeal
                ? result.multiplier > 1
                    ? 'good'
                    : result.multiplier < 1
                        ? 'bad'
                        : 'neutral'
                : result.multiplier > 1
                    ? 'bad'
                    : result.multiplier < 1
                        ? 'good'
                        : 'neutral';

        return `
            <span
                class="mha-matchup ${favorability} ${relation.key}"
                title="${escapeHtml(title)}"
            >${escapeHtml(label)}</span>
        `;
    }

    function huntEffectiveness(
        attackType,
        defenderTypes
    ) {
        const raw =
            exactEffectiveness(
                attackType,
                defenderTypes
            );

        if (
            raw === 0 ||
            raw === 1
        ) {
            return raw;
        }

        const factor =
            Number(
                state.huntAmplification
            ) ||
            1.5;

        return raw > 1
            ? 1 +
                (
                    raw - 1
                ) *
                    factor
            : raw /
                factor;
    }

    // ------------------------------------------------------------------
    // XP/hour model — real combat first, prediction second
    // ------------------------------------------------------------------

    const UPSTREAM_WAVE_MS =
        2600;

    const UPSTREAM_GLOBAL_ATTACK_MS =
        900;

    const UPSTREAM_FALLBACK_MOVE_MS =
        600;

    function attackerTypes() {
        const profile =
            combatProfile();

        if (
            profile &&
            profile.moveTypes.length
        ) {
            return profile.moveTypes;
        }

        return (
            leadPokemon()
                ?.tipos ||
            []
        )
            .map(
                type =>
                    String(type)
                        .toUpperCase()
            );
    }

    function huntMatchupFactor(
        hunt
    ) {
        const attackTypes =
            attackerTypes();

        if (
            !attackTypes.length ||
            !Array.isArray(
                hunt?.especies
            )
        ) {
            return 1;
        }

        let totalWeight = 0;
        let weighted = 0;

        for (
            const species of
            hunt.especies
        ) {
            const weight =
                Math.max(
                    1,
                    Number(
                        species?.pontos ||
                        1
                    )
                );

            const defenderTypes =
                state.typesBySpecies.get(
                    Number(
                        species?.pokeId
                    )
                ) ||
                [];

            const best =
                defenderTypes.length
                    ? Math.max(
                        ...attackTypes.map(
                            attackType =>
                                huntEffectiveness(
                                    attackType,
                                    defenderTypes
                                )
                        )
                    )
                    : 1;

            weighted +=
                weight *
                Math.max(
                    0.01,
                    best
                );

            totalWeight +=
                weight;
        }

        return totalWeight
            ? weighted /
                totalWeight
            : 1;
    }

    function recordsForLead() {
        const signature =
            leadSignature();

        if (!signature) {
            return [];
        }

        return [
            ...state.huntPerf.values()
        ].filter(
            record =>
                record?.version ===
                    2 &&
                record.signature ===
                    signature
        );
    }

    function profileRecordsForLead() {
        const exact =
            recordsForLead();

        const exactAttacks =
            exact.reduce(
                (sum, record) =>
                    sum +
                    Number(
                        record.attacks ||
                        0
                    ),
                0
            );

        const exactHp =
            exact.reduce(
                (sum, record) =>
                    sum +
                    (
                        record.hpSamples
                            ?.length ||
                        0
                    ),
                0
            );

        /*
         * Once the current level has enough direct combat data, it stands
         * alone. Before that, blend the nearest previous level so a level-up
         * does not blank the Atlas for the first few attacks.
         */
        if (
            exact.length &&
            exactAttacks >= 6 &&
            exactHp >= 1
        ) {
            return {
                records: exact,
                warmStart: false
            };
        }

        const pokemon =
            leadPokemon();

        const loadout =
            leadLoadoutSignature(
                pokemon
            );

        if (!loadout) {
            return {
                records: exact,
                warmStart:
                    false
            };
        }

        const currentSignature =
            leadSignature(
                pokemon
            );

        const compatible =
            [...state.huntPerf.values()]
                .filter(
                    record =>
                        record?.version ===
                            2 &&
                        record.loadoutSignature ===
                            loadout &&
                        record.signature !==
                            currentSignature &&
                        Number(
                            record.attacks ||
                            0
                        ) > 0
                );

        if (!compatible.length) {
            return {
                records: exact,
                warmStart: false
            };
        }

        const currentLevel =
            Number(
                pokemon?.level ||
                0
            );

        const nearestDistance =
            Math.min(
                ...compatible.map(
                    record =>
                        Math.abs(
                            Number(
                                record.pokemonLevel ||
                                0
                            ) -
                            currentLevel
                        )
                )
            );

        const nearest =
            compatible.filter(
                record =>
                    Math.abs(
                        Number(
                            record.pokemonLevel ||
                            0
                        ) -
                        currentLevel
                    ) ===
                    nearestDistance
            );

        return {
            records: [
                ...exact,
                ...nearest
            ],
            warmStart: true
        };
    }
    function observedRates(
        record
    ) {
        if (
            !record ||
            Number(
                record.kills ||
                0
            ) <
                2
        ) {
            return null;
        }

        const killMs =
            average(
                record.killIntervals
            );

        if (
            !killMs ||
            killMs <= 0
        ) {
            return null;
        }

        const kills =
            Math.max(
                1,
                Number(
                    record.kills
                ) ||
                1
            );

        const killsH =
            3600000 /
            killMs;

        return {
            killMs,
            killsH,
            trainerXpH:
                Number(
                    record.trainerXp ||
                    0
                ) /
                kills *
                killsH,
            pokemonXpH:
                Number(
                    record.pokemonXp ||
                    0
                ) /
                kills *
                killsH,
            goldH:
                Number(
                    record.gold ||
                    0
                ) /
                kills *
                killsH
        };
    }

    function combatProfile() {
        const cacheKey =
            [
                leadSignature() ||
                    '',
                state.combatRevision,
                state.combatCatalogStatus
            ].join('|');

        if (
            state.combatProfileCache
                ?.key ===
            cacheKey
        ) {
            return state.combatProfileCache
                .value;
        }

        const recordSelection =
            profileRecordsForLead();

        const records =
            recordSelection.records;

        if (!records.length) {
            return null;
        }

        const warmStart =
            recordSelection.warmStart;

        const currentPower =
            Math.max(
                1,
                pokemonPower(
                    leadPokemon()
                )
            );

        const sameTarget = [];
        const transitions = [];
        const allAttacks = [];
        const hpSamples = [];
        const trainerMult = [];
        const pokemonMult = [];

        const moves = {};

        let attacks = 0;
        let kills = 0;

        for (
            const record of
            records
        ) {
            sameTarget.push(
                ...(
                    record.sameTargetIntervals ||
                    []
                )
            );

            transitions.push(
                ...(
                    record.transitionIntervals ||
                    []
                )
            );

            allAttacks.push(
                ...(
                    record.allAttackIntervals ||
                    []
                )
            );

            hpSamples.push(
                ...(
                    record.hpSamples ||
                    []
                )
            );

            trainerMult.push(
                ...(
                    record.trainerXpMultSamples ||
                    []
                ).slice(-12)
            );

            pokemonMult.push(
                ...(
                    record.pokemonXpMultSamples ||
                    []
                ).slice(-12)
            );

            attacks +=
                Number(
                    record.attacks ||
                    0
                );

            kills +=
                Number(
                    record.kills ||
                    0
                );

            for (
                const [
                    type,
                    sample
                ] of
                Object.entries(
                    record.moveDamage ||
                    {}
                )
            ) {
                const bucket =
                    moves[
                        type
                    ] || {
                        samples: []
                    };

                const powerScale =
                    warmStart
                        ? currentPower /
                            Math.max(
                                1,
                                Number(
                                    record.pokemonPower ||
                                    currentPower
                                )
                            )
                        : 1;

                bucket.samples.push(
                    ...(
                        sample
                            ?.neutralSamples ||
                        []
                    )
                        .slice(-50)
                        .map(
                            damage =>
                                Number(
                                    damage
                                ) *
                                powerScale
                        )
                );

                if (
                    bucket.samples.length >
                    100
                ) {
                    bucket.samples.splice(
                        0,
                        bucket.samples.length -
                            100
                    );
                }

                moves[
                    type
                ] =
                    bucket;
            }
        }

        const cadenceMeasured =
            robustMean(
                sameTarget
            );

        const cadenceThroughput =
            robustMean(
                allAttacks
            );

        const cadenceMs =
            cadenceMeasured ||
            cadenceThroughput ||
            UPSTREAM_GLOBAL_ATTACK_MS;

        let movementMs =
            null;

        const cleanTransitions =
            transitions
                .map(Number)
                .filter(
                    value =>
                        Number.isFinite(
                            value
                        ) &&
                        value >= 100
                )
                .sort(
                    (a, b) =>
                        a - b
                );

        if (
            cleanTransitions.length
        ) {
            /*
             * Long transition gaps include the fixed 2.6 s wave pause.
             * The lower 60% isolates ordinary travel/pathfinding.
             */
            const take =
                Math.max(
                    1,
                    Math.ceil(
                        cleanTransitions.length *
                        0.6
                    )
                );

            movementMs =
                robustMean(
                    cleanTransitions.slice(
                        0,
                        take
                    )
                );
        }

        if (!movementMs) {
            movementMs =
                UPSTREAM_FALLBACK_MOVE_MS;
        }

        const hpScaleSamples = [];
        const hpPerLevelSamples = [];

        for (
            const sample of
            hpSamples
        ) {
            const level =
                Number(
                    sample?.level
                );

            const maxHp =
                Number(
                    sample?.maxHp
                );

            if (
                !Number.isFinite(
                    level
                ) ||
                level <= 0 ||
                !Number.isFinite(
                    maxHp
                ) ||
                maxHp <= 0
            ) {
                continue;
            }

            hpPerLevelSamples.push(
                maxHp /
                level
            );

            const meta =
                speciesCombatMeta(
                    sample?.speciesId
                );

            const baseHp =
                Number(
                    meta?.baseHp
                );

            if (
                Number.isFinite(
                    baseHp
                ) &&
                baseHp > 0
            ) {
                hpScaleSamples.push(
                    maxHp /
                    (
                        baseHp *
                        level
                    )
                );
            }
        }

        const moveDamage = {};
        let moveSamples = 0;

        for (
            const [
                type,
                bucket
            ] of
            Object.entries(
                moves
            )
        ) {
            const damage =
                robustMean(
                    bucket.samples
                );

            if (
                !damage ||
                damage <= 0
            ) {
                continue;
            }

            moveDamage[
                type
            ] = {
                damage,
                samples:
                    bucket.samples.length
            };

            moveSamples +=
                bucket.samples.length;
        }

        const value = {
            records,
            warmStart,
            attacks,
            kills,
            cadenceMs,
            cadenceMeasured:
                Boolean(
                    cadenceMeasured
                ),
            cadenceFromThroughput:
                !cadenceMeasured &&
                Boolean(
                    cadenceThroughput
                ),
            movementMs,
            movementMeasured:
                cleanTransitions.length >
                0,
            hpScale:
                median(
                    hpScaleSamples
                ),
            hpPerLevel:
                median(
                    hpPerLevelSamples
                ),
            hpSamples:
                hpSamples.length,
            moveDamage,
            moveTypes:
                Object.keys(
                    moveDamage
                ),
            moveSamples,
            trainerXpMult:
                robustMean(
                    trainerMult
                ) ||
                1,
            pokemonXpMult:
                robustMean(
                    pokemonMult
                ) ||
                1
        };

        state.combatProfileCache = {
            key: cacheKey,
            value
        };

        return value;
    }
    function targetHpEstimate(
        speciesId,
        huntLevel,
        profile
    ) {
        const level =
            Math.max(
                1,
                Number(
                    huntLevel
                ) ||
                1
            );

        const meta =
            speciesCombatMeta(
                speciesId
            );

        const baseHp =
            Number(
                meta?.baseHp
            );

        if (
            profile?.hpScale &&
            Number.isFinite(
                baseHp
            ) &&
            baseHp > 0
        ) {
            return (
                baseHp *
                level *
                profile.hpScale
            );
        }

        if (
            profile?.hpPerLevel
        ) {
            return (
                level *
                profile.hpPerLevel
            );
        }

        return null;
    }

    function damagePerAttackAgainst(
        speciesId,
        profile
    ) {
        const entries =
            Object.entries(
                profile?.moveDamage ||
                {}
            );

        if (!entries.length) {
            return null;
        }

        const meta =
            speciesCombatMeta(
                speciesId
            );

        const defenderTypes =
            (
                state.typesBySpecies.get(
                    Number(
                        speciesId
                    )
                ) ||
                [
                    meta?.type1,
                    meta?.type2
                ].filter(Boolean)
            )
                .map(
                    type =>
                        String(type)
                            .toUpperCase()
                );

        let totalWeight = 0;
        let totalDamage = 0;

        for (
            const [
                type,
                move
            ] of
            entries
        ) {
            const samples =
                Math.max(
                    1,
                    Number(
                        move?.samples ||
                        1
                    )
                );

            const multiplier =
                defenderTypes.length
                    ? huntEffectiveness(
                        type,
                        defenderTypes
                    )
                    : 1;

            totalWeight +=
                samples;

            totalDamage +=
                samples *
                Number(
                    move.damage ||
                    0
                ) *
                multiplier;
        }

        return totalWeight
            ? totalDamage /
                totalWeight
            : null;
    }

    function rawPredictedKillMs(
        hunt,
        profile
    ) {
        if (
            !hunt ||
            !profile ||
            !profile.moveSamples ||
            (
                !profile.hpScale &&
                !profile.hpPerLevel
            )
        ) {
            return null;
        }

        const species =
            Array.isArray(
                hunt.especies
            )
                ? hunt.especies
                : [];

        if (!species.length) {
            return null;
        }

        const totalPoints =
            species.reduce(
                (sum, row) =>
                    sum +
                    Math.max(
                        1,
                        Number(
                            row?.pontos ||
                            1
                        )
                    ),
                0
            ) ||
            1;

        const mobsPerWave =
            Math.max(
                1,
                Math.min(
                    16,
                    totalPoints
                )
            );

        /*
         * Measured ordinary movement + exact upstream fixed wave pause,
         * amortized over mobs in the wave.
         */
        const transitionMs =
            profile.movementMs +
            UPSTREAM_WAVE_MS /
                mobsPerWave;

        let weightedMs = 0;
        let weightSum = 0;

        for (
            const row of
            species
        ) {
            const weight =
                Math.max(
                    1,
                    Number(
                        row?.pontos ||
                        1
                    )
                );

            const hp =
                targetHpEstimate(
                    row?.pokeId,
                    hunt.nivel,
                    profile
                );

            const damage =
                damagePerAttackAgainst(
                    row?.pokeId,
                    profile
                );

            if (
                !hp ||
                !damage ||
                damage <= 0
            ) {
                continue;
            }

            const attacksNeeded =
                Math.max(
                    1,
                    Math.ceil(
                        hp /
                        damage
                    )
                );

            /*
             * The first attack comes after transition. Further attacks use
             * this Pokémon's measured same-target cadence.
             */
            const killMs =
                transitionMs +
                Math.max(
                    0,
                    attacksNeeded -
                        1
                ) *
                    profile.cadenceMs;

            weightedMs +=
                killMs *
                weight;

            weightSum +=
                weight;
        }

        return weightSum
            ? weightedMs /
                weightSum
            : null;
    }

    function predictionCalibration(
        profile
    ) {
        if (
            profile &&
            Number.isFinite(
                profile._calibration
            )
        ) {
            return profile._calibration;
        }

        const ratios = [];

        for (
            const record of
            profile?.records ||
            []
        ) {
            const observed =
                observedRates(
                    record
                );

            if (!observed) {
                continue;
            }

            const hunt =
                state.hunts.find(
                    item =>
                        item.slug ===
                        record.huntSlug
                );

            if (!hunt) {
                continue;
            }

            const predicted =
                rawPredictedKillMs(
                    hunt,
                    profile
                );

            if (
                predicted &&
                predicted > 0
            ) {
                ratios.push(
                    observed.killMs /
                    predicted
                );
            }
        }

        const ratio =
            robustMean(
                ratios
            );

        const calibrated =
            !ratio
                ? 1
                : Math.max(
                    0.4,
                    Math.min(
                        3,
                        ratio
                    )
                );

        if (profile) {
            profile._calibration =
                calibrated;
        }

        return calibrated;
    }

    function huntXpEstimate(
        hunt
    ) {
        const cacheKey =
            [
                leadSignature() ||
                    '',
                state.combatRevision,
                state.combatCatalogStatus
            ].join('|');

        if (
            state.estimateCacheKey !==
            cacheKey
        ) {
            state.estimateCacheKey =
                cacheKey;

            state.estimateCache.clear();
        }

        const huntKey =
            hunt?.slug ||
            '';

        if (
            huntKey &&
            state.estimateCache.has(
                huntKey
            )
        ) {
            return state.estimateCache.get(
                huntKey
            );
        }

        const remember =
            value => {
                if (huntKey) {
                    state.estimateCache.set(
                        huntKey,
                        value
                    );
                }

                return value;
            };

        const record =
            performanceRecord(
                hunt?.slug,
                false
            );

        const observed =
            observedRates(
                record
            );

        if (observed) {
            return remember({
                value:
                    observed.trainerXpH,
                pokemonValue:
                    observed.pokemonXpH,
                goldValue:
                    observed.goldH,
                killsH:
                    observed.killsH,
                killMs:
                    observed.killMs,
                observed:
                    true,
                source:
                    'measured',
                samples:
                    Math.max(
                        0,
                        Number(
                            record.kills ||
                            0
                        )
                    )
            });
        }

        const profile =
            combatProfile();

        const rawKillMs =
            rawPredictedKillMs(
                hunt,
                profile
            );

        if (
            !profile ||
            !rawKillMs ||
            rawKillMs <= 0
        ) {
            return remember({
                value: null,
                pokemonValue: null,
                goldValue: null,
                killsH: null,
                killMs: null,
                observed: false,
                source:
                    'learning',
                samples: 0
            });
        }

        const killMs =
            rawKillMs *
            predictionCalibration(
                profile
            );

        const killsH =
            3600000 /
            killMs;

        const baseXp =
            xpDoNivel(
                hunt?.nivel ||
                1
            );

        const trainerXpPerKill =
            Math.round(
                baseXp *
                profile.trainerXpMult
            );

        const pokemonXpPerKill =
            Math.round(
                baseXp *
                profile.pokemonXpMult
            );

        return remember({
            value:
                trainerXpPerKill *
                killsH,
            pokemonValue:
                pokemonXpPerKill *
                killsH,
            goldValue: null,
            killsH,
            killMs,
            observed:
                false,
            source:
                'combat model',
            samples:
                profile.moveSamples
        });
    }

    function rankedUnlockedHunts(
        speciesResults = null
    ) {
        const sourceHunts =
            speciesResults
                ? [
                    ...new Map(
                        speciesResults
                            .flatMap(
                                species =>
                                    species.hunts.map(
                                        item => [
                                            item.hunt.slug,
                                            item.hunt
                                        ]
                                    )
                            )
                    ).values()
                ]
                : state.hunts;

        return sourceHunts
            .filter(
                isUnlocked
            )
            .map(
                hunt => ({
                    hunt,
                    xp:
                        huntXpEstimate(
                            hunt
                        )
                })
            )
            .sort(
                (a, b) => {
                    const av =
                        Number(
                            a.xp?.value ||
                            0
                        );

                    const bv =
                        Number(
                            b.xp?.value ||
                            0
                        );

                    if (
                        av !== bv
                    ) {
                        return bv - av;
                    }

                    return (
                        Number(
                            b.hunt.nivel ||
                            0
                        ) -
                        Number(
                            a.hunt.nivel ||
                            0
                        )
                    );
                }
            );
    }

    // ------------------------------------------------------------------
    // Hunt / capture model
    // ------------------------------------------------------------------

    function captureCount(
        speciesId
    ) {
        const entry =
            state.pokedex.get(
                String(speciesId)
            );

        const count =
            Number(
                entry?.c ||
                0
            );

        if (
            count > 0
        ) {
            return count;
        }

        return state.ownedSpecies.has(
            Number(speciesId)
        )
            ? 1
            : 0;
    }

    function isCaptured(
        speciesId
    ) {
        return (
            captureCount(
                speciesId
            ) > 0
        );
    }

    function isUnlocked(
        hunt
    ) {
        if (
            state.currentHuntSlug &&
            hunt.slug ===
                state.currentHuntSlug
        ) {
            return true;
        }

        if (
            state.lockOverrides.has(
                hunt.slug
            )
        ) {
            return state.lockOverrides.get(
                hunt.slug
            );
        }

        const required =
            Number(
                hunt.nivel ||
                0
            );

        return (
            state.playerLevel >=
            required
        );
    }

    function buildSpeciesIndex() {
        const index =
            new Map();

        for (
            const hunt of
            state.hunts
        ) {
            if (
                !Array.isArray(
                    hunt?.especies
                )
            ) {
                continue;
            }

            for (
                const species of
                hunt.especies
            ) {
                const id =
                    Number(
                        species?.pokeId
                    );

                if (
                    !Number.isFinite(id)
                ) {
                    continue;
                }

                let entry =
                    index.get(id);

                if (!entry) {
                    entry = {
                        id,
                        name:
                            species.nome ||
                            'Pokémon ' +
                            id,
                        hunts: []
                    };

                    index.set(
                        id,
                        entry
                    );
                }

                entry.hunts.push({
                    hunt,
                    points:
                        Number(
                            species.pontos ||
                            0
                        )
                });
            }
        }

        return index;
    }

    function marketKeyForSpecies(
        species
    ) {
        const id =
            Number(
                species?.id
            );

        if (Number.isFinite(id)) {
            return 'id:' + id;
        }

        const name =
            normalize(
                species?.name
            );

        return name
            ? 'name:' + name
            : null;
    }

    function marketStatsForSpecies(
        species
    ) {
        const idKey =
            Number.isFinite(
                Number(
                    species?.id
                )
            )
                ? 'id:' +
                    Number(
                        species.id
                    )
                : null;

        if (
            idKey &&
            state.marketValues.has(
                idKey
            )
        ) {
            return state.marketValues.get(
                idKey
            );
        }

        const nameKey =
            'name:' +
            normalize(
                species?.name
            );

        return state.marketValues.get(
            nameKey
        ) || null;
    }

    function playerMarketValueForSpecies(
        species
    ) {
        return Number(
            (
                species?.market ||
                marketStatsForSpecies(
                    species
                )
            )?.average ||
            0
        );
    }

    function npcBaseSellValueForSpecies(
        species
    ) {
        return Number(
            speciesCombatMeta(
                species?.id
            )?.sellValue ||
            0
        );
    }

    function npcSellValueAtLevel(
        species,
        level
    ) {
        const base =
            npcBaseSellValueForSpecies(
                species
            );

        const huntLevel =
            Math.max(
                1,
                Number(
                    level ||
                    1
                )
            );

        if (base <= 0) {
            return 0;
        }

        /*
         * PokéIdle's shared sell-value.mjs:
         *   base × (1 + level / 50) × quality
         *
         * Atlas has species/hunt data rather than a captured individual's
         * quality, so use the neutral reference quality 1.0.
         * Shiny is intentionally not included here (actual sale ×10).
         */
        return Math.max(
            1,
            Math.floor(
                base *
                (
                    1 +
                    huntLevel /
                        50
                )
            )
        );
    }

    function npcSellStatsForSpecies(
        species
    ) {
        const values =
            (
                species?.hunts ||
                []
            )
                .map(
                    item =>
                        npcSellValueAtLevel(
                            species,
                            item?.hunt?.nivel
                        )
                )
                .filter(
                    value =>
                        value > 0
                );

        if (!values.length) {
            const base =
                npcBaseSellValueForSpecies(
                    species
                );

            return base > 0
                ? {
                    average: base,
                    min: base,
                    max: base,
                    count: 1
                }
                : null;
        }

        return {
            average:
                values.reduce(
                    (sum, value) =>
                        sum + value,
                    0
                ) /
                values.length,
            min:
                Math.min(
                    ...values
                ),
            max:
                Math.max(
                    ...values
                ),
            count:
                values.length
        };
    }

    function npcSellValueForSpecies(
        species
    ) {
        return Number(
            npcSellStatsForSpecies(
                species
            )?.average ||
            0
        );
    }

    function bestSpawnPercent(
        species
    ) {
        return Math.max(
            ...(
                species?.hunts ||
                []
            ).map(
                item =>
                    Number(
                        huntSpawnPercent(
                            item
                        ) ||
                        0
                    )
            ),
            0
        );
    }

    function bestMatchupScore(
        species
    ) {
        const matchup =
            speciesMatchup(
                species
            );

        if (
            !matchup?.offense ||
            !matchup?.defense
        ) {
            return -Infinity;
        }

        const deal =
            Number(
                matchup.offense
                    .multiplier
            );

        const take =
            Number(
                matchup.defense
                    .multiplier
            );

        if (
            !Number.isFinite(deal) ||
            !Number.isFinite(take)
        ) {
            return -Infinity;
        }

        const safety =
            take === 0
                ? 4
                : 1 /
                    Math.max(
                        0.25,
                        take
                    );

        return deal * safety;
    }

    function regionOptions() {
        return [
            ...new Set(
                state.hunts
                    .map(
                        hunt =>
                            String(
                                hunt.area ||
                                ''
                            )
                    )
                    .filter(Boolean)
            )
        ].sort(
            (a, b) =>
                a.localeCompare(
                    b
                )
        );
    }

    function huntMatchesScope(
        hunt
    ) {
        const regionFilter =
            state.filters.region;

        if (
            regionFilter !== 'all' &&
            String(
                hunt?.area || ''
            ) !== regionFilter
        ) {
            return false;
        }

        const huntLevel =
            Number(
                hunt?.nivel ||
                0
            );

        const minLevelFilter =
            Number.parseInt(
                state.filters.minLevel,
                10
            );

        if (
            Number.isFinite(
                minLevelFilter
            ) &&
            huntLevel <
                minLevelFilter
        ) {
            return false;
        }

        const maxLevelFilter =
            Number.parseInt(
                state.filters.maxLevel,
                10
            );

        if (
            Number.isFinite(
                maxLevelFilter
            ) &&
            huntLevel >
                maxLevelFilter
        ) {
            return false;
        }

        const unlocked =
            isUnlocked(
                hunt
            );

        if (
            state.filters.availability ===
                'unlocked' &&
            !unlocked
        ) {
            return false;
        }

        if (
            state.filters.availability ===
                'locked' &&
            unlocked
        ) {
            return false;
        }

        return true;
    }

    function filteredSpecies() {
        const search =
            normalize(
                state.filters.search
            );

        const typeFilter =
            state.filters.type;

        const weaknessFilter =
            state.filters.weakness;

        const captureFilter =
            state.filters.captured;

        const entries =
            [...buildSpeciesIndex().values()];

        const result = [];

        for (
            const species of
            entries
        ) {
            const captured =
                isCaptured(
                    species.id
                );

            if (
                captureFilter ===
                    'captured' &&
                !captured
            ) {
                continue;
            }

            if (
                captureFilter ===
                    'uncaught' &&
                captured
            ) {
                continue;
            }

            const types =
                state.typesBySpecies.get(
                    species.id
                ) || [];

            if (
                typeFilter !== 'all' &&
                !types.includes(
                    typeFilter
                )
            ) {
                continue;
            }

            if (
                weaknessFilter !==
                    'all'
            ) {
                if (
                    !types.length ||
                    weaknessMultiplier(
                        types,
                        weaknessFilter
                    ) <= 1
                ) {
                    continue;
                }
            }

            const hunts =
                species.hunts
                    .filter(
                        item => {
                            const hunt =
                                item.hunt;

                            if (
                                !huntMatchesScope(
                                    hunt
                                )
                            ) {
                                return false;
                            }

                            if (!search) {
                                return true;
                            }

                            return [
                                species.name,
                                hunt.nome,
                                hunt.slug,
                                hunt.area
                            ]
                                .map(
                                    normalize
                                )
                                .some(
                                    text =>
                                        text.includes(
                                            search
                                        )
                                );
                        }
                    )
                    .sort(
                        (a, b) => {
                            const au =
                                isUnlocked(
                                    a.hunt
                                );

                            const bu =
                                isUnlocked(
                                    b.hunt
                                );

                            if (
                                au !== bu
                            ) {
                                return au
                                    ? -1
                                    : 1;
                            }

                            if (
                                state.filters.sort ===
                                'xp'
                            ) {
                                const ax =
                                    Number(
                                        huntXpEstimate(
                                            a.hunt
                                        )?.value ||
                                        0
                                    );

                                const bx =
                                    Number(
                                        huntXpEstimate(
                                            b.hunt
                                        )?.value ||
                                        0
                                    );

                                if (ax !== bx) {
                                    return bx - ax;
                                }
                            }

                            return (
                                Number(
                                    a.hunt
                                        .nivel ||
                                    0
                                ) -
                                Number(
                                    b.hunt
                                        .nivel ||
                                    0
                                )
                            );
                        }
                    );

            if (!hunts.length) {
                continue;
            }

            result.push({
                ...species,
                captured,
                captureCount:
                    captureCount(
                        species.id
                    ),
                types,
                hunts,
                market:
                    marketStatsForSpecies(
                        species
                    )
            });
        }

        result.sort(
            (a, b) => {
                const sort =
                    state.filters.sort;

                if (sort === 'xp') {
                    const ax =
                        Math.max(
                            ...a.hunts.map(
                                item =>
                                    Number(
                                        huntXpEstimate(
                                            item.hunt
                                        )?.value ||
                                        0
                                    )
                            ),
                            0
                        );

                    const bx =
                        Math.max(
                            ...b.hunts.map(
                                item =>
                                    Number(
                                        huntXpEstimate(
                                            item.hunt
                                        )?.value ||
                                        0
                                    )
                            ),
                            0
                        );

                    if (ax !== bx) {
                        return bx - ax;
                    }
                }

                if (
                    sort ===
                        'npc_desc' ||
                    sort ===
                        'npc_asc'
                ) {
                    const av =
                        npcSellValueForSpecies(
                            a
                        );

                    const bv =
                        npcSellValueForSpecies(
                            b
                        );

                    if (av !== bv) {
                        return sort ===
                            'npc_desc'
                            ? bv - av
                            : av - bv;
                    }
                }

                if (
                    sort ===
                        'player_market_desc' ||
                    sort ===
                        'player_market_asc'
                ) {
                    const av =
                        playerMarketValueForSpecies(
                            a
                        );

                    const bv =
                        playerMarketValueForSpecies(
                            b
                        );

                    if (
                        av > 0 ||
                        bv > 0
                    ) {
                        if (av <= 0) {
                            return 1;
                        }

                        if (bv <= 0) {
                            return -1;
                        }

                        if (av !== bv) {
                            return sort ===
                                'player_market_desc'
                                ? bv - av
                                : av - bv;
                        }
                    }
                }

                if (
                    sort ===
                    'spawn_desc'
                ) {
                    const difference =
                        bestSpawnPercent(
                            b
                        ) -
                        bestSpawnPercent(
                            a
                        );

                    if (difference) {
                        return difference;
                    }
                }

                if (
                    sort ===
                        'level_asc' ||
                    sort ===
                        'level_desc'
                ) {
                    const levelValue =
                        species =>
                            sort ===
                                'level_desc'
                                ? Math.max(
                                    ...species.hunts.map(
                                        item =>
                                            Number(
                                                item.hunt
                                                    .nivel ||
                                                0
                                            )
                                    ),
                                    0
                                )
                                : Math.min(
                                    ...species.hunts.map(
                                        item =>
                                            Number(
                                                item.hunt
                                                    .nivel ||
                                                Infinity
                                            )
                                    )
                                );

                    const aLevel =
                        levelValue(
                            a
                        );

                    const bLevel =
                        levelValue(
                            b
                        );

                    if (aLevel !== bLevel) {
                        return sort ===
                            'level_desc'
                            ? bLevel - aLevel
                            : aLevel - bLevel;
                    }
                }

                if (
                    sort ===
                    'matchup'
                ) {
                    const difference =
                        bestMatchupScore(
                            b
                        ) -
                        bestMatchupScore(
                            a
                        );

                    if (
                        Number.isFinite(
                            difference
                        ) &&
                        difference
                    ) {
                        return difference;
                    }
                }

                if (
                    sort ===
                    'pokedex'
                ) {
                    const difference =
                        Number(a.id) -
                        Number(b.id);

                    if (difference) {
                        return difference;
                    }
                }

                if (
                    sort !== 'name' &&
                    sort !== 'pokedex' &&
                    a.captured !==
                        b.captured
                ) {
                    return a.captured
                        ? 1
                        : -1;
                }

                return a.name.localeCompare(
                    b.name
                );
            }
        );

        return result;
    }

    // ------------------------------------------------------------------
    // Hunt navigation
    // ------------------------------------------------------------------

    function socketReady() {
        return Boolean(
            state.activeSocket &&
            state.activeSocket.readyState ===
                WebSocket.OPEN
        );
    }

    function sendHuntSelect(
        hunt
    ) {
        if (
            !hunt ||
            !hunt.slug
        ) {
            return false;
        }

        if (!socketReady()) {
            setStatus(
                'status.gameNotReady'
            );

            queueRender();
            return false;
        }

        try {
            state.pendingTravel = {
                slug:
                    hunt.slug,
                startedAt:
                    Date.now()
            };

            setStatus(
                'status.traveling',
                {
                    hunt:
                        hunt.nome ||
                        hunt.slug
                }
            );

            /*
             * Explorer captured PokéIdle's exact map action:
             *   { t: "hunt.select", slug: "<hunt-slug>" }
             *
             * The server remains authoritative for cooldowns and any
             * progression restrictions Atlas does not know about.
             */
            state.activeSocket.send(
                JSON.stringify({
                    t:
                        'hunt.select',
                    slug:
                        hunt.slug
                })
            );

            queueRender();

            setTimeout(
                () => {
                    if (
                        state.pendingTravel?.slug ===
                            hunt.slug &&
                        state.currentHuntSlug !==
                            hunt.slug
                    ) {
                        setStatus(
                            'status.notAccepted',
                            {
                                hunt:
                                    hunt.nome ||
                                    hunt.slug
                            }
                        );

                        state.pendingTravel =
                            null;

                        queueRender();
                    }
                },
                2500
            );

            return true;
        } catch (error) {
            console.warn(
                '[PokéIdle Hunt Atlas] hunt.select failed',
                error
            );

            state.pendingTravel =
                null;

            setStatus(
                'status.couldNotSend'
            );

            queueRender();
            return false;
        }
    }

    function navigateToHunt(
        hunt
    ) {
        if (
            !isUnlocked(hunt)
        ) {
            setStatus(
                'status.huntLocked',
                {
                    hunt:
                        hunt.nome,
                    level:
                        hunt.nivel
                }
            );

            queueRender();
            return;
        }

        if (
            hunt.slug ===
            state.currentHuntSlug
        ) {
            setStatus(
                'status.alreadyHere',
                {
                    hunt:
                        hunt.nome
                }
            );

            queueRender();
            return;
        }

        if (
            sendHuntSelect(
                hunt
            )
        ) {
            /*
             * Keep the Atlas open briefly so the user sees "Traveling…".
             * The confirmed hunt event will update the current state.
             */
            setTimeout(
                () => {
                    if (
                        state.currentHuntSlug ===
                        hunt.slug
                    ) {
                        q(
                            '#modal-fechar'
                        )?.click();
                    }
                },
                300
            );
        }
    }

    // ------------------------------------------------------------------
    // UI
    // ------------------------------------------------------------------

    function injectStyles() {
        if (
            q('#' + STYLE_ID)
        ) {
            return;
        }

        const style =
            document.createElement(
                'style'
            );

        style.id =
            STYLE_ID;

        style.textContent = `
            #${BUTTON_ID} {
                display: none !important;
            }

            #mha-map-toggle {
                margin-left: 8px;
                padding: 3px 8px;
                border: 1px solid rgba(255,255,255,.14);
                border-radius: 999px;
                background: rgba(255,255,255,.06);
                color: #a99fa3;
                cursor: pointer;
                font: 700 9px/1 system-ui,sans-serif;
                white-space: nowrap;
            }

            #mha-map-toggle[data-enabled="1"] {
                border-color: rgba(104,190,130,.28);
                background: rgba(104,190,130,.10);
                color: #a8ddb7;
            }

            #modal .modal-caixa[data-mha-atlas-active="1"] {
                width: min(1180px, 96vw) !important;
                max-width: 96vw !important;
                height: min(880px, 92vh) !important;
                max-height: 92vh !important;
            }

            #modal .modal-caixa[data-mha-atlas-active="1"] #modal-corpo {
                display: flex !important;
                flex-direction: column !important;
                min-height: 0 !important;
                height: auto !important;
                flex: 1 1 auto !important;
                overflow: hidden !important;
                padding: 0 !important;
            }

            #modal .modal-caixa[data-mha-atlas-active="1"]
            #modal-corpo > :not(#${DRAWER_ID}) {
                display: none !important;
            }

            #${DRAWER_ID} {
                width: 100% !important;
                min-width: 0 !important;
                min-height: 0 !important;
                flex: 1 1 auto !important;
                display: flex !important;
                flex-direction: column !important;
                overflow: hidden !important;
                background: #171316 !important;
                color: #eee7ea !important;
                font: 11px/1.35 system-ui,sans-serif !important;
            }

            #${DRAWER_ID} .mha-head {
                display: none !important;
            }

            #${DRAWER_ID}[hidden] {
                display: none !important;
            }

            #${DRAWER_ID},
            #${DRAWER_ID} * {
                box-sizing: border-box !important;
            }

            .mha-head {
                display: flex;
                align-items: center;
                gap: 7px;
                min-height: 42px;
                padding: 7px 9px;
                border-bottom: 1px solid rgba(255,255,255,.08);
            }

            .mha-head strong {
                flex: 1;
                font-size: 12px;
            }

            .mha-head button,
            .mha-go {
                border: 1px solid rgba(255,255,255,.13);
                border-radius: 5px;
                background: rgba(255,255,255,.06);
                color: #eee;
                cursor: pointer;
                font: inherit;
            }

            .mha-head button {
                padding: 4px 7px;
            }

            .mha-filters {
                display: grid;
                grid-template-columns: repeat(4, minmax(0, 1fr));
                gap: 8px;
                padding: 10px 12px;
                border-bottom: 1px solid rgba(255,255,255,.07);
                background: rgba(255,255,255,.015);
            }

            .mha-field {
                display: grid;
                gap: 3px;
                min-width: 0;
                color: #8f858a;
                font-size: 8px;
                font-weight: 700;
                letter-spacing: .02em;
            }

            .mha-field > span {
                padding-left: 1px;
            }

            .mha-filters input,
            .mha-filters select,
            .mha-filter-actions button {
                width: 100%;
                min-width: 0;
                height: 30px;
                padding: 4px 6px;
                border: 1px solid rgba(255,255,255,.12);
                border-radius: 5px;
                background: #211a1e;
                color: #fff;
                font: inherit;
            }

            .mha-search {
                grid-column: span 2;
            }

            .mha-level-inputs {
                display: grid;
                grid-template-columns: 1fr auto 1fr;
                align-items: center;
                gap: 4px;
            }

            .mha-level-inputs > span {
                color: #71686c;
                font-size: 10px;
            }

            .mha-filter-actions {
                grid-column: span 3;
                display: grid;
                grid-template-columns: minmax(0,1fr) auto;
                align-items: end;
                gap: 6px;
            }

            .mha-filter-state {
                align-self: center;
                min-width: 0;
                overflow: hidden;
                color: #93898e;
                font-size: 9px;
                text-overflow: ellipsis;
                white-space: nowrap;
            }

            .mha-filter-actions button {
                width: auto;
                padding-inline: 9px;
                cursor: pointer;
            }

            .mha-filter-actions button:disabled {
                cursor: default;
                opacity: .45;
            }

            .mha-xp-summary {
                padding: 7px 8px;
                border-bottom: 1px solid rgba(255,255,255,.07);
                background: rgba(102,176,255,.055);
            }

            .mha-xp-primary {
                display: grid;
                grid-template-columns: minmax(0,1fr) auto auto;
                gap: 6px;
                align-items: center;
            }

            .mha-xp-primary-main {
                min-width: 0;
            }

            .mha-xp-primary-main strong {
                display: block;
                overflow: hidden;
                color: #e8e1e4;
                font-size: 10px;
                text-overflow: ellipsis;
                white-space: nowrap;
            }

            .mha-xp-primary-main span,
            .mha-xp-note {
                color: #9e9398;
                font-size: 8px;
            }

            .mha-xp-toggle {
                border: 0;
                background: transparent;
                color: #a9d2ff;
                cursor: pointer;
                font: inherit;
                font-size: 9px;
            }

            .mha-xp-details {
                margin-top: 6px;
                padding-top: 6px;
                border-top: 1px solid rgba(255,255,255,.07);
            }

            .mha-xp-details[hidden] {
                display: none;
            }

            .mha-best-list {
                display: grid;
                gap: 3px;
                margin-top: 6px;
            }

            .mha-best-row {
                display: grid;
                grid-template-columns: 18px minmax(0,1fr) auto auto;
                gap: 5px;
                align-items: center;
                min-height: 26px;
                padding: 3px 5px;
                border-radius: 5px;
                background: rgba(0,0,0,.18);
            }

            .mha-best-rank {
                color: #7f7479;
                font-size: 9px;
                text-align: center;
            }

            .mha-best-name {
                min-width: 0;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
            }

            .mha-xp-value {
                color: #a9d2ff;
                font-size: 9px;
                white-space: nowrap;
            }

            .mha-meta {
                display: flex;
                align-items: center;
                gap: 8px;
                min-height: 29px;
                padding: 5px 8px;
                border-bottom: 1px solid rgba(255,255,255,.07);
                color: #a99fa3;
                font-size: 9px;
            }

            .mha-meta .mha-status {
                flex: 1;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
            }

            .mha-credit {
                display: flex;
                align-items: center;
                justify-content: flex-end;
                gap: 5px;
                padding: 4px 8px 5px;
                border-top: 1px solid rgba(255,255,255,.045);
                color: #665e62;
                font-size: 7px;
            }

            .mha-credit a {
                color: #776d72;
                text-decoration: none;
            }

            .mha-credit a:hover,
            .mha-credit a:focus-visible {
                color: #b8adb2;
                text-decoration: underline;
            }

            .mha-credit-icon {
                display: inline-grid;
                width: 16px;
                height: 16px;
                place-items: center;
                border: 1px solid rgba(255,255,255,.08);
                border-radius: 50%;
                color: #857a80;
                text-decoration: none !important;
            }

            .mha-credit-icon svg {
                width: 10px;
                height: 10px;
                fill: currentColor;
            }

            .mha-credit-sep {
                color: #50494d;
            }

            .mha-body {
                flex: 1;
                min-height: 0;
                overflow: auto;
                padding: 10px 12px;
                display: grid;
                grid-template-columns: repeat(2, minmax(0, 1fr));
                gap: 9px;
                align-content: start;
            }

            .mha-species {
                margin-bottom: 0;
                padding: 7px;
                border: 1px solid rgba(255,255,255,.08);
                border-radius: 7px;
                background: rgba(255,255,255,.035);
            }

            .mha-species-head {
                display: flex;
                align-items: center;
                gap: 6px;
                margin-bottom: 6px;
                flex-wrap: wrap;
            }

            .mha-species-name {
                flex: 1;
                min-width: 0;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
                font-weight: 750;
                font-size: 12px;
            }

            .mha-caught {
                padding: 2px 5px;
                border-radius: 999px;
                background: rgba(88,190,112,.13);
                color: #93e3a8;
                font-size: 9px;
                white-space: nowrap;
            }

            .mha-uncaught {
                padding: 2px 5px;
                border-radius: 999px;
                background: rgba(235,178,74,.11);
                color: #ffd27d;
                font-size: 9px;
                white-space: nowrap;
            }

            .mha-market,
            .mha-npc-value {
                padding: 2px 5px;
                border: 1px solid rgba(236,198,98,.16);
                border-radius: 999px;
                background: rgba(236,198,98,.07);
                color: #d8c58c;
                font-size: 8px;
                white-space: nowrap;
            }

            .mha-market {
                border-color: rgba(123,171,230,.15);
                background: rgba(123,171,230,.06);
                color: #adc9ec;
            }

            .mha-matchups {
                display: inline-flex;
                gap: 3px;
                flex-wrap: wrap;
            }

            .mha-matchup {
                padding: 2px 5px;
                border: 1px solid rgba(255,255,255,.08);
                border-radius: 999px;
                background: rgba(255,255,255,.05);
                color: #b8adb2;
                font-size: 8px;
                white-space: nowrap;
            }

            .mha-matchup.good {
                border-color: rgba(91,190,119,.22);
                background: rgba(91,190,119,.10);
                color: #9be3ae;
            }

            .mha-matchup.bad {
                border-color: rgba(225,102,102,.22);
                background: rgba(225,102,102,.09);
                color: #f0a0a0;
            }

            .mha-matchup.immune {
                font-weight: 750;
            }

            .mha-types {
                display: inline-flex;
                gap: 3px;
                flex-wrap: wrap;
            }

            .mha-type {
                padding: 2px 4px;
                border-radius: 4px;
                background: rgba(255,255,255,.07);
                color: #cfc4c9;
                font-size: 8px;
            }

            .mha-hunts {
                display: grid;
                gap: 4px;
            }

            .mha-hunt {
                display: grid;
                grid-template-columns: minmax(0,1fr) auto;
                gap: 6px;
                align-items: center;
                min-height: 33px;
                padding: 5px 6px;
                border-radius: 5px;
                background: rgba(0,0,0,.18);
            }

            .mha-hunt.current {
                outline: 1px solid rgba(100,210,130,.24);
                background: rgba(88,190,112,.08);
            }

            .mha-hunt.locked {
                opacity: .72;
            }

            .mha-hunt-main {
                min-width: 0;
            }

            .mha-hunt-title {
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
                color: #e7dfe2;
                font-size: 10px;
            }

            .mha-hunt-sub {
                margin-top: 2px;
                color: #93898e;
                font-size: 8px;
            }

            .mha-go {
                min-width: 58px;
                padding: 5px 7px;
            }

            .mha-go:hover:not(:disabled) {
                background: rgba(255,255,255,.12);
            }

            .mha-go:disabled {
                cursor: default;
                opacity: .46;
            }

            .mha-go.current {
                color: #8fdda4;
                border-color: rgba(100,210,130,.28);
            }

            .mha-empty {
                grid-column: 1 / -1;
                padding: 28px 12px;
                color: #9d9397;
                text-align: center;
            }

            .mha-show-more {
                grid-column: 1 / -1;
                width: 100%;
                margin-top: 2px;
                padding: 8px;
                border: 1px solid rgba(255,255,255,.12);
                border-radius: 6px;
                background: rgba(255,255,255,.05);
                color: #d9d0d4;
                cursor: pointer;
                font: inherit;
            }

            .mha-show-more:hover {
                background: rgba(255,255,255,.09);
            }

            @media (max-width: 900px) {
                .mha-body {
                    grid-template-columns: 1fr;
                }
            }

            @media (max-width: 620px) {
                #modal .modal-caixa[data-mha-atlas-active="1"] {
                    width: 100vw !important;
                    max-width: 100vw !important;
                    height: 100dvh !important;
                    max-height: 100dvh !important;
                }

                .mha-filters {
                    grid-template-columns: 1fr 1fr;
                    padding: 8px;
                }

                .mha-search,
                .mha-filter-actions {
                    grid-column: 1 / -1;
                }

                .mha-body {
                    padding: 7px;
                }

                .mha-xp-primary {
                    grid-template-columns: minmax(0,1fr) auto;
                }

                .mha-xp-toggle {
                    grid-column: 1 / -1;
                    justify-self: start;
                    padding: 0;
                }

                .mha-credit {
                    justify-content: center;
                }
            }
        `;

        (
            document.head ||
            document.documentElement
        ).appendChild(
            style
        );
    }

    function isMapModalOpen() {
        const modal =
            q('#modal');

        const box =
            q(
                '#modal .modal-caixa'
            );

        return Boolean(
            modal &&
            box &&
            !modal.classList.contains(
                'hidden'
            ) &&
            box.dataset.modal ===
                'mapa'
        );
    }

    function restoreNativeMapView() {
        const box =
            q(
                '#modal .modal-caixa'
            );

        box?.removeAttribute(
            'data-mha-atlas-active'
        );

        const drawer =
            q(
                '#' + DRAWER_ID
            );

        if (drawer) {
            drawer.hidden =
                true;
        }

    }

    function cleanupMapIntegration() {
        restoreNativeMapView();

        q(
            '#mha-map-toggle'
        )?.remove();

        state.drawerOpen =
            false;
    }

    function ensureMapToggle() {
        if (!isMapModalOpen()) {
            return null;
        }

        const title =
            q('#modal-titulo');

        if (!title) {
            return null;
        }

        let toggle =
            q('#mha-map-toggle');

        if (
            !toggle ||
            toggle.parentElement !==
                title.parentElement
        ) {
            toggle?.remove();

            toggle =
                document.createElement(
                    'button'
                );

            toggle.type =
                'button';
            toggle.id =
                'mha-map-toggle';

            title.insertAdjacentElement(
                'afterend',
                toggle
            );

            toggle.addEventListener(
                'click',
                () => {
                    state.atlasEnabled =
                        !state.atlasEnabled;

                    saveAtlasEnabled(
                        state.atlasEnabled
                    );

                    if (
                        !state.atlasEnabled
                    ) {
                        restoreNativeMapView();
                    }

                    scheduleMapSync();
                }
            );
        }

        toggle.dataset.enabled =
            state.atlasEnabled
                ? '1'
                : '0';

        toggle.textContent =
            tr(
                state.atlasEnabled
                    ? 'toggle.on'
                    : 'toggle.off'
            );

        toggle.title =
            tr(
                state.atlasEnabled
                    ? 'toggle.onTitle'
                    : 'toggle.offTitle'
            );

        toggle.setAttribute(
            'aria-pressed',
            state.atlasEnabled
                ? 'true'
                : 'false'
        );

        return toggle;
    }

    function scheduleMapSync() {
        if (
            state.mapSyncQueued
        ) {
            return;
        }

        state.mapSyncQueued =
            true;

        queueMicrotask(
            () => {
                state.mapSyncQueued =
                    false;

                const open =
                    isMapModalOpen();

                const wasOpen =
                    state.drawerOpen;

                if (!open) {
                    cleanupMapIntegration();
                    return;
                }

                ensureMapToggle();

                if (
                    !state.atlasEnabled
                ) {
                    state.drawerOpen =
                        false;
                    restoreNativeMapView();
                    return;
                }

                state.drawerOpen =
                    true;

                if (!wasOpen) {
                    ensureTypeData();
                    ensureCombatCatalog();
                    ensureMarketValues();
                }

                renderDrawer();

                if (!wasOpen) {
                    queueMicrotask(
                        () =>
                            q(
                                '[data-mha-filter="search"]',
                                q(
                                    '#' +
                                    DRAWER_ID
                                )
                            )?.focus()
                    );
                }
            }
        );
    }

    function installMapObserver() {
        if (
            state.mapObserver
        ) {
            return;
        }

        const modal =
            q('#modal');

        const box =
            q(
                '#modal .modal-caixa'
            );

        const body =
            q('#modal-corpo');

        if (
            !modal ||
            !box ||
            !body
        ) {
            return;
        }

        state.mapObserver =
            new MutationObserver(
                scheduleMapSync
            );

        state.mapObserver.observe(
            modal,
            {
                attributes: true,
                attributeFilter: [
                    'class'
                ]
            }
        );

        state.mapObserver.observe(
            box,
            {
                attributes: true,
                attributeFilter: [
                    'data-modal'
                ]
            }
        );

        state.mapObserver.observe(
            body,
            {
                childList: true
            }
        );
    }

    function ensureButton() {
        q(
            '#' + BUTTON_ID
        )?.remove();

        return true;
    }

    function ensureDrawer() {
        if (
            !document.body ||
            !isMapModalOpen() ||
            !state.atlasEnabled
        ) {
            return null;
        }

        injectStyles();

        const modalBody =
            q('#modal-corpo');

        const modalBox =
            q(
                '#modal .modal-caixa'
            );

        if (
            !modalBody ||
            !modalBox
        ) {
            return null;
        }

        modalBox.setAttribute(
            'data-mha-atlas-active',
            '1'
        );

        let drawer =
            q('#' + DRAWER_ID);

        if (drawer) {
            if (
                drawer.parentElement !==
                    modalBody
            ) {
                modalBody.appendChild(
                    drawer
                );
            }

            return drawer;
        }

        drawer =
            document.createElement(
                'aside'
            );

        drawer.id =
            DRAWER_ID;

        drawer.hidden =
            true;

        drawer.innerHTML = `
            <div class="mha-head">
                <strong>${tr('title')}</strong>
                <button type="button" data-mha-close>×</button>
            </div>

            <div class="mha-filters">
                <label class="mha-field mha-search">
                    <span>${tr('filter.search')}</span>
                    <input data-mha-filter="search" type="search" placeholder="${escapeHtml(tr('filter.searchPlaceholder'))}">
                </label>

                <label class="mha-field">
                    <span>${tr('filter.region')}</span>
                    <select data-mha-filter="region"></select>
                </label>

                <div class="mha-field">
                    <span>${tr('filter.level')}</span>
                    <div class="mha-level-inputs">
                        <input data-mha-filter="minLevel" type="number" min="1" step="1" inputmode="numeric" placeholder="${escapeHtml(tr('filter.min'))}" aria-label="${escapeHtml(tr('filter.minLevelAria'))}">
                        <span>–</span>
                        <input data-mha-filter="maxLevel" type="number" min="1" step="1" inputmode="numeric" placeholder="${escapeHtml(tr('filter.max'))}" aria-label="${escapeHtml(tr('filter.maxLevelAria'))}">
                    </div>
                </div>

                <label class="mha-field">
                    <span>${tr('filter.availability')}</span>
                    <select data-mha-filter="availability">
                        <option value="all">${tr('filter.all')}</option>
                        <option value="unlocked">${tr('filter.unlocked')}</option>
                        <option value="locked">${tr('filter.locked')}</option>
                    </select>
                </label>

                <label class="mha-field">
                    <span>${tr('filter.type')}</span>
                    <select data-mha-filter="type"></select>
                </label>

                <label class="mha-field">
                    <span>${tr('filter.weakTo')}</span>
                    <select data-mha-filter="weakness"></select>
                </label>

                <label class="mha-field">
                    <span>${tr('filter.collection')}</span>
                    <select data-mha-filter="captured">
                        <option value="all">${tr('filter.caughtAndUncaught')}</option>
                        <option value="uncaught">${tr('filter.uncaughtOnly')}</option>
                        <option value="captured">${tr('filter.caughtOnly')}</option>
                    </select>
                </label>

                <label class="mha-field">
                    <span>${tr('filter.sort')}</span>
                    <select data-mha-filter="sort">
                        <option value="xp">${tr('sort.xp')}</option>
                        <option value="npc_desc">${tr('sort.npcDesc')}</option>
                        <option value="npc_asc">${tr('sort.npcAsc')}</option>
                        <option value="player_market_desc">${tr('sort.playerDesc')}</option>
                        <option value="player_market_asc">${tr('sort.playerAsc')}</option>
                        <option value="matchup">${tr('sort.matchup')}</option>
                        <option value="spawn_desc">${tr('sort.spawnDesc')}</option>
                        <option value="level_asc">${tr('sort.levelAsc')}</option>
                        <option value="level_desc">${tr('sort.levelDesc')}</option>
                        <option value="pokedex">${tr('sort.pokedex')}</option>
                        <option value="name">${tr('sort.name')}</option>
                    </select>
                </label>

                <div class="mha-filter-actions">
                    <span class="mha-filter-state"></span>
                    <button type="button" data-mha-clear>${tr('filter.clear')}</button>
                </div>
            </div>

            <div class="mha-xp-summary"></div>

            <div class="mha-meta">
                <span class="mha-status"></span>
                <span class="mha-count"></span>
            </div>

            <div class="mha-body"></div>

            <div class="mha-credit">
                <span>${tr('footer.by')}</span>

                <a
                    class="mha-credit-icon"
                    href="https://play.google.com/store/apps/developer?id=MOTHblank"
                    target="_blank"
                    rel="noopener noreferrer"
                    title="${escapeHtml(tr('footer.play'))}"
                    aria-label="${escapeHtml(tr('footer.play'))}"
                >
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path d="M3.7 2.7c-.45.48-.7 1.2-.7 2.1v14.4c0 .9.25 1.62.7 2.1l.08.08L12.9 12 3.78 2.62l-.08.08Zm10.42 10.52-2.05-2.1L4.7 3.55c.17-.03.36-.01.57.1l11.7 6.65-2.85 2.92Zm-9.42 7.23 7.38-7.57 2.04-2.1 2.9 2.95-11.75 6.62c-.2.11-.4.13-.57.1Zm13.72-7.43-1.98-1.12 1.98-2.02c.95.54 1.48 1.08 1.48 1.57 0 .5-.53 1.03-1.48 1.57Z"/>
                    </svg>
                </a>

                <a
                    class="mha-credit-icon"
                    href="https://x.com/MOTHblank"
                    target="_blank"
                    rel="noopener noreferrer"
                    title="${escapeHtml(tr('footer.x'))}"
                    aria-label="${escapeHtml(tr('footer.x'))}"
                >
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path d="M18.9 2H22l-6.77 7.74L23.2 22h-6.24l-4.89-6.39L6.48 22H3.36l7.25-8.29L2.96 2H9.36l4.42 5.84L18.9 2Zm-1.1 17.84h1.72L8.42 4.05H6.57L17.8 19.84Z"/>
                    </svg>
                </a>

                <a
                    class="mha-credit-icon"
                    href="https://wa.me/+5537999933376"
                    target="_blank"
                    rel="noopener noreferrer"
                    title="${escapeHtml(tr('footer.whatsapp'))}"
                    aria-label="${escapeHtml(tr('footer.whatsapp'))}"
                >
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path d="M12 2a9.8 9.8 0 0 0-8.43 14.8L2 22l5.35-1.52A9.95 9.95 0 1 0 12 2Zm0 17.94a8 8 0 0 1-4.08-1.12l-.29-.17-3.18.9.86-3.1-.19-.3A7.9 7.9 0 1 1 12 19.94Zm4.35-5.9c-.24-.12-1.4-.69-1.62-.77-.22-.08-.38-.12-.54.12-.16.24-.62.77-.76.93-.14.16-.28.18-.52.06-.24-.12-1-.37-1.91-1.18-.7-.63-1.18-1.41-1.32-1.65-.14-.24-.02-.37.1-.49.11-.11.24-.28.36-.42.12-.14.16-.24.24-.4.08-.16.04-.3-.02-.42-.06-.12-.54-1.3-.74-1.78-.2-.47-.39-.4-.54-.41h-.46c-.16 0-.42.06-.64.3-.22.24-.84.82-.84 2s.86 2.32.98 2.48c.12.16 1.69 2.58 4.1 3.62.57.25 1.02.4 1.37.51.58.18 1.1.16 1.51.1.46-.07 1.4-.57 1.6-1.13.2-.55.2-1.03.14-1.13-.06-.1-.22-.16-.46-.28Z"/>
                    </svg>
                </a>

                <span class="mha-credit-sep">·</span>

                <a
                    href="https://github.com/MOTHblank/pokeidle-huntatlas"
                    target="_blank"
                    rel="noopener noreferrer"
                >${tr('footer.source')}</a>
            </div>
        `;

        modalBody.appendChild(
            drawer
        );

        q(
            '[data-mha-close]',
            drawer
        ).addEventListener(
            'click',
            () => {
                state.drawerOpen =
                    false;

                queueRender();
            }
        );

        q(
            '[data-mha-clear]',
            drawer
        ).addEventListener(
            'click',
            () => {
                const sort =
                    state.filters.sort;

                state.filters = {
                    ...defaultFilters(),
                    sort
                };
                state.resultLimit =
                    120;
                saveFilters();
                renderDrawer();

                q(
                    '[data-mha-filter="search"]',
                    drawer
                )?.focus();
            }
        );

        if (
            !state.keyboardBound
        ) {
            state.keyboardBound =
                true;

            document.addEventListener(
                'keydown',
                event => {
                    if (
                        !isMapModalOpen() ||
                        event.key !== '/' ||
                        [
                            'INPUT',
                            'SELECT',
                            'TEXTAREA'
                        ].includes(
                            document.activeElement
                                ?.tagName
                        )
                    ) {
                        return;
                    }

                    event.preventDefault();

                    q(
                        '[data-mha-filter="search"]',
                        q(
                            '#' +
                            DRAWER_ID
                        )
                    )?.focus();
                }
            );
        }

        for (
            const control of
            qa(
                '[data-mha-filter]',
                drawer
            )
        ) {
            const key =
                control.dataset
                    .mhaFilter;

            const eventName =
                (
                    key === 'search' ||
                    key === 'minLevel' ||
                    key === 'maxLevel'
                )
                    ? 'input'
                    : 'change';

            control.addEventListener(
                eventName,
                () => {
                    state.filters[
                        key
                    ] =
                        control.value;

                    state.resultLimit =
                        120;

                    saveFilters();
                    renderDrawer();
                }
            );
        }

        return drawer;
    }

    function selectOptions(
        values,
        selected,
        allValue,
        allLabel,
        labelForValue =
            value => value
    ) {
        return [
            `<option value="${escapeHtml(allValue)}">${escapeHtml(allLabel)}</option>`,
            ...values.map(
                value =>
                    `<option value="${escapeHtml(value)}" ${value === selected ? 'selected' : ''}>${escapeHtml(labelForValue(value))}</option>`
            )
        ].join('');
    }

    function renderFilterControls(
        drawer
    ) {
        const search =
            q(
                '[data-mha-filter="search"]',
                drawer
            );

        if (
            search &&
            search.value !==
                state.filters.search
        ) {
            search.value =
                state.filters.search;
        }

        const region =
            q(
                '[data-mha-filter="region"]',
                drawer
            );

        const regionHtml =
            selectOptions(
                regionOptions(),
                state.filters.region,
                'all',
                tr(
                    'filter.allRegions'
                )
            );

        if (
            region.innerHTML !==
            regionHtml
        ) {
            region.innerHTML =
                regionHtml;
        }

        region.value =
            state.filters.region;

        for (
            const key of [
                'minLevel',
                'maxLevel'
            ]
        ) {
            const control =
                q(
                    `[data-mha-filter="${key}"]`,
                    drawer
                );

            if (
                control &&
                control.value !==
                    String(
                        state.filters[key] ??
                        ''
                    )
            ) {
                control.value =
                    state.filters[key] ??
                    '';
            }
        }

        const type =
            q(
                '[data-mha-filter="type"]',
                drawer
            );

        const typeHtml =
            selectOptions(
                STANDARD_TYPES,
                state.filters.type,
                'all',
                state.typeStatus ===
                    'loading'
                    ? tr(
                        'filter.typeLoading'
                    )
                    : tr(
                        'filter.allTypes'
                    ),
                typeLabel
            );

        if (
            type.innerHTML !==
            typeHtml
        ) {
            type.innerHTML =
                typeHtml;
        }

        type.value =
            state.filters.type;

        const weakness =
            q(
                '[data-mha-filter="weakness"]',
                drawer
            );

        const weakHtml =
            selectOptions(
                STANDARD_TYPES,
                state.filters.weakness,
                'all',
                state.typeStatus ===
                    'loading'
                    ? tr(
                        'filter.weaknessLoading'
                    )
                    : tr(
                        'filter.anyWeakness'
                    ),
                typeLabel
            );

        if (
            weakness.innerHTML !==
            weakHtml
        ) {
            weakness.innerHTML =
                weakHtml;
        }

        weakness.value =
            state.filters.weakness;

        for (
            const key of [
                'availability',
                'captured',
                'sort'
            ]
        ) {
            const control =
                q(
                    `[data-mha-filter="${key}"]`,
                    drawer
                );

            if (control) {
                control.value =
                    state.filters[key];
            }
        }

        const activeCount =
            activeFilterCount();

        const filterState =
            q(
                '.mha-filter-state',
                drawer
            );

        if (filterState) {
            filterState.textContent =
                activeCount
                    ? tr(
                        activeCount === 1
                            ? 'filter.changedOne'
                            : 'filter.changedMany',
                        {
                            count:
                                activeCount
                        }
                    )
                    : tr(
                        'filter.defaults'
                    );
        }

        const clearButton =
            q(
                '[data-mha-clear]',
                drawer
            );

        if (clearButton) {
            clearButton.disabled =
                activeCount === 0;
        }
    }

    function huntSpawnPercent(
        item
    ) {
        const total =
            Number(
                item.hunt
                    .totalSpawns ||
                0
            );

        if (
            !total ||
            !item.points
        ) {
            return null;
        }

        return (
            item.points /
            total *
            100
        );
    }

    function speciesMarkup(
        species
    ) {
        const capturedMarkup =
            species.captured
                ? `<span class="mha-caught">✓ ${escapeHtml(
                    tr(
                        'capture.caught'
                    )
                )}${species.captureCount > 1 ? ' ×' + species.captureCount : ''}</span>`
                : `<span class="mha-uncaught">○ ${escapeHtml(
                    tr(
                        'capture.notCaught'
                    )
                )}</span>`;

        const typeMarkup =
            species.types.length
                ? `<span class="mha-types">${species.types.map(type => `<span class="mha-type">${escapeHtml(typeLabel(type))}</span>`).join('')}</span>`
                : `<span class="mha-types"><span class="mha-type">${escapeHtml(
                    tr(
                        'capture.typeUnknown'
                    )
                )}</span></span>`;

        const matchup =
            speciesMatchup(
                species
            );

        const leadName =
            matchup?.lead?.nome ||
            tr(
                'common.lead'
            );

        const npcStats =
            npcSellStatsForSpecies(
                species
            );

        const npcRange =
            npcStats &&
            npcStats.min !==
                npcStats.max
                ? tr(
                    'price.range',
                    {
                        min:
                            localizedNumber(
                                Math.round(
                                    npcStats.min
                                )
                            ),
                        max:
                            localizedNumber(
                                Math.round(
                                    npcStats.max
                                )
                            )
                    }
                )
                : '';

        const npcMarkup =
            npcStats?.average > 0
                ? `
                    <span
                        class="mha-npc-value"
                        title="${escapeHtml(
                            tr(
                                'price.npcTip',
                                {
                                    plural:
                                        npcStats.count === 1
                                            ? ''
                                            : 's',
                                    range:
                                        npcRange
                                }
                            )
                        )}"
                    >${escapeHtml(
                        tr(
                            'price.npc',
                            {
                                value:
                                    formatRate(
                                        npcStats.average
                                    )
                            }
                        )
                    )}</span>
                `
                : '';

        const market =
            species.market;

        const marketMarkup =
            market?.count
                ? `
                    <span
                        class="mha-market"
                        title="${escapeHtml(
                            tr(
                                'price.playerTip',
                                {
                                    count:
                                        market.count,
                                    sample:
                                        tr(
                                            market.count === 1
                                                ? 'price.sample'
                                                : 'price.samples'
                                        ),
                                    min:
                                        localizedNumber(
                                            Math.round(
                                                market.min
                                            )
                                        ),
                                    max:
                                        localizedNumber(
                                            Math.round(
                                                market.max
                                            )
                                        )
                                }
                            )
                        )}"
                    >${escapeHtml(
                        tr(
                            'price.players',
                            {
                                value:
                                    formatRate(
                                        market.average
                                    )
                            }
                        )
                    )}</span>
                `
                : '';

        const matchupMarkup =
            matchup
                ? `
                    <span class="mha-matchups">
                        ${matchupBadgeMarkup(
                            'deal',
                            matchup.offense,
                            leadName,
                            species.name
                        )}
                        ${matchupBadgeMarkup(
                            'take',
                            matchup.defense,
                            leadName,
                            species.name
                        )}
                    </span>
                `
                : '';

        const huntsMarkup =
            species.hunts
                .map(
                    (item, index) => {
                        const hunt =
                            item.hunt;

                        const unlocked =
                            isUnlocked(
                                hunt
                            );

                        const current =
                            hunt.slug ===
                            state.currentHuntSlug;

                        const percent =
                            huntSpawnPercent(
                                item
                            );

                        const xpInfo =
                            huntXpEstimate(
                                hunt
                            );

                        const xpLabel =
                            xpInfo?.value
                                ? tr(
                                    xpInfo.observed
                                        ? 'hunt.measuredXp'
                                        : 'hunt.modeledXp',
                                    {
                                        value:
                                            formatRate(
                                                xpInfo.value
                                            )
                                    }
                                )
                                : null;

                        const sub =
                            [
                                String(
                                    hunt.area ||
                                    ''
                                ).toUpperCase(),
                                tr(
                                    'hunt.level',
                                    {
                                        level:
                                            Number(
                                                hunt.nivel ||
                                                0
                                            )
                                    }
                                ),
                                percent !== null
                                    ? tr(
                                        'hunt.weight',
                                        {
                                            value:
                                                percent.toFixed(
                                                    percent >=
                                                        10
                                                        ? 0
                                                        : 1
                                                )
                                        }
                                    )
                                    : null,
                                tr(
                                    unlocked
                                        ? 'hunt.unlocked'
                                        : 'hunt.locked'
                                ),
                                xpLabel
                            ]
                                .filter(Boolean)
                                .join(' · ');

                        const label =
                            tr(
                                current
                                    ? 'hunt.here'
                                    : unlocked
                                        ? 'hunt.go'
                                        : 'hunt.locked'
                            );

                        return `
                            <div class="mha-hunt ${current ? 'current' : ''} ${!unlocked ? 'locked' : ''}">
                                <div class="mha-hunt-main">
                                    <div class="mha-hunt-title">${escapeHtml(hunt.nome || hunt.slug)}</div>
                                    <div class="mha-hunt-sub">${escapeHtml(sub)}</div>
                                </div>

                                <button
                                    type="button"
                                    class="mha-go ${current ? 'current' : ''}"
                                    data-mha-go="${escapeHtml(hunt.slug)}"
                                    ${!unlocked || current ? 'disabled' : ''}
                                    title="${escapeHtml(
                                        !unlocked
                                            ? tr(
                                                'hunt.requiresLevel',
                                                {
                                                    level:
                                                        Number(
                                                            hunt.nivel ||
                                                            0
                                                        )
                                                }
                                            )
                                            : current
                                                ? tr(
                                                    'hunt.current'
                                                )
                                                : tr(
                                                    'hunt.travel'
                                                )
                                    )}"
                                >${label}</button>
                            </div>
                        `;
                    }
                )
                .join('');

        return `
            <section class="mha-species">
                <div class="mha-species-head">
                    <span class="mha-species-name">${escapeHtml(species.name)}</span>
                    ${npcMarkup}
                    ${marketMarkup}
                    ${matchupMarkup}
                    ${typeMarkup}
                    ${capturedMarkup}
                </div>

                <div class="mha-hunts">
                    ${huntsMarkup}
                </div>
            </section>
        `;
    }

    function renderXpSummary(
        drawer,
        results
    ) {
        const host =
            q(
                '.mha-xp-summary',
                drawer
            );

        if (!host) {
            return;
        }

        const lead =
            leadPokemon();

        if (!lead) {
            host.innerHTML =
                `<div class="mha-xp-note">${escapeHtml(
                    tr(
                        'xp.waitingLead'
                    )
                )}</div>`;

            return;
        }

        const ranked =
            rankedUnlockedHunts(
                results
            );

        const usable =
            ranked.filter(
                row =>
                    Number(
                        row.xp?.value ||
                        0
                    ) > 0
            );

        const best =
            usable[0] || null;

        const current =
            currentHunt();

        const currentXp =
            current
                ? huntXpEstimate(
                    current
                )
                : null;

        const profile =
            combatProfile();

        const measuredHunts =
            recordsForLead()
                .filter(
                    record =>
                        observedRates(
                            record
                        )
                )
                .length;

        const leadText =
            tr(
                'xp.lead',
                {
                    name:
                        lead.nome ||
                        'Pokémon',
                    level:
                        Number(
                            lead.level ||
                            0
                        ),
                    power:
                        Math.round(
                            pokemonPower(
                                lead
                            )
                        )
                }
            );

        const profileDetails =
            profile
                ? [
                    profile.warmStart
                        ? tr(
                            'xp.warmStart'
                        )
                        : null,
                    profile.kills
                        ? tr(
                            'xp.kills',
                            {
                                count:
                                    profile.kills
                            }
                        )
                        : null,
                    profile.attacks
                        ? tr(
                            'xp.attacks',
                            {
                                count:
                                    profile.attacks
                            }
                        )
                        : null,
                    profile.cadenceMs
                        ? tr(
                            'xp.cadence',
                            {
                                seconds:
                                    (
                                        profile.cadenceMs /
                                        1000
                                    ).toFixed(2),
                                detail:
                                    profile.cadenceMeasured
                                        ? ''
                                        : profile.cadenceFromThroughput
                                            ? tr(
                                                'xp.throughput'
                                            )
                                            : tr(
                                                'xp.floor'
                                            )
                            }
                        )
                        : null,
                    profile.movementMs
                        ? tr(
                            'xp.travel',
                            {
                                seconds:
                                    (
                                        profile.movementMs /
                                        1000
                                    ).toFixed(2)
                            }
                        )
                        : null,
                    profile.hpSamples
                        ? tr(
                            'xp.hpSamples',
                            {
                                count:
                                    profile.hpSamples
                            }
                        )
                        : null
                ]
                    .filter(Boolean)
                    .join(' · ')
                : '';

        const calibrationText =
            profile?.attacks
                ? (
                    measuredHunts
                        ? tr(
                            measuredHunts === 1
                                ? 'xp.measuredHunt'
                                : 'xp.measuredHunts',
                            {
                                count:
                                    measuredHunts
                            }
                        )
                        : tr(
                            'xp.calibrating'
                        )
                ) +
                  (
                      profileDetails
                          ? ' · ' +
                            profileDetails
                          : ''
                  )
                : tr(
                    'xp.learning'
                );
        const bestRows =
            usable
                .slice(0, 5)
                .map(
                    (row, index) => {
                        const here =
                            row.hunt.slug ===
                            state.currentHuntSlug;

                        return `
                            <div class="mha-best-row">
                                <span class="mha-best-rank">${index + 1}</span>
                                <span class="mha-best-name">${escapeHtml(row.hunt.nome || row.hunt.slug)} · ${escapeHtml(
                                    tr(
                                        'hunt.level',
                                        {
                                            level:
                                                Number(
                                                    row.hunt.nivel ||
                                                    0
                                                )
                                        }
                                    )
                                )}</span>
                                <span class="mha-xp-value" title="${escapeHtml([
                                    row.xp.killsH
                                        ? tr(
                                            'xp.killsPerHour',
                                            {
                                                count:
                                                    Math.round(
                                                        row.xp.killsH
                                                    )
                                            }
                                        )
                                        : null,
                                    row.xp.pokemonValue
                                        ? tr(
                                            'xp.pokemonPerHour',
                                            {
                                                value:
                                                    formatRate(
                                                        row.xp.pokemonValue
                                                    )
                                            }
                                        )
                                        : null
                                ].filter(Boolean).join(' · '))}">${escapeHtml(
                                    tr(
                                        row.xp.observed
                                            ? 'xp.measuredShort'
                                            : 'xp.modelShort',
                                        {
                                            value:
                                                formatRate(
                                                    row.xp.value
                                                )
                                        }
                                    )
                                )}</span>
                                <button
                                    type="button"
                                    class="mha-go ${here ? 'current' : ''}"
                                    data-mha-best-go="${escapeHtml(row.hunt.slug)}"
                                    ${here ? 'disabled' : ''}
                                >${tr(
                                    here
                                        ? 'hunt.here'
                                        : 'hunt.go'
                                )}</button>
                            </div>
                        `;
                    }
                )
                .join('');

        const bestHere =
            best?.hunt?.slug ===
            state.currentHuntSlug;

        host.innerHTML = `
            <div class="mha-xp-primary">
                <div class="mha-xp-primary-main">
                    <strong>${best
                        ? escapeHtml(
                            best.hunt.nome ||
                            best.hunt.slug
                        ) +
                          ' · ' +
                          escapeHtml(
                              tr(
                                  'hunt.level',
                                  {
                                      level:
                                          Number(
                                              best.hunt.nivel ||
                                              0
                                          )
                                  }
                              )
                          )
                        : escapeHtml(
                            tr(
                                'xp.best'
                            )
                        )}</strong>
                    <span>${best
                        ? escapeHtml(
                            tr(
                                best.xp.observed
                                    ? 'xp.measuredTrainer'
                                    : 'xp.modeledTrainer',
                                {
                                    value:
                                        formatRate(
                                            best.xp.value
                                        )
                                }
                            )
                        )
                        : escapeHtml(
                            tr(
                                'xp.noEstimate'
                            )
                        )}</span>
                </div>

                ${best ? `
                    <button
                        type="button"
                        class="mha-go ${bestHere ? 'current' : ''}"
                        data-mha-best-go="${escapeHtml(best.hunt.slug)}"
                        ${bestHere ? 'disabled' : ''}
                    >${tr(
                        bestHere
                            ? 'hunt.here'
                            : 'hunt.go'
                    )}</button>
                ` : ''}

                <button type="button" class="mha-xp-toggle" data-mha-xp-toggle>
                    ${tr(
                        state.xpDetailsOpen
                            ? 'xp.hideDetails'
                            : 'xp.details'
                    )}
                </button>
            </div>

            <div class="mha-xp-note">
                ${escapeHtml(
                    tr(
                        'xp.recommendationNote',
                        {
                            lead:
                                leadText
                        }
                    )
                )}
            </div>

            <div class="mha-xp-details" ${state.xpDetailsOpen ? '' : 'hidden'}>
                <div class="mha-xp-note">
                    ${escapeHtml(calibrationText)}
                    ${currentXp?.value
                        ? escapeHtml(
                            tr(
                                currentXp.observed
                                    ? 'xp.currentMeasured'
                                    : 'xp.currentModeled',
                                {
                                    value:
                                        formatRate(
                                            currentXp.value
                                        )
                                }
                            )
                        )
                        : ''}
                    ${escapeHtml(
                        tr(
                            'xp.modelNote'
                        )
                    )}
                </div>

                <div class="mha-best-list">
                    ${bestRows}
                </div>
            </div>
        `;

        const detailsToggle =
            q(
                '[data-mha-xp-toggle]',
                host
            );

        detailsToggle?.addEventListener(
            'click',
            () => {
                state.xpDetailsOpen =
                    !state.xpDetailsOpen;

                renderXpSummary(
                    drawer,
                    results
                );
            }
        );

        for (
            const button of
            qa(
                '[data-mha-best-go]',
                host
            )
        ) {
            button.addEventListener(
                'click',
                () => {
                    const hunt =
                        state.hunts.find(
                            item =>
                                item.slug ===
                                button.dataset
                                    .mhaBestGo
                        );

                    if (hunt) {
                        navigateToHunt(
                            hunt
                        );
                    }
                }
            );
        }
    }

    function renderDrawer() {
        const locale =
            currentLocale();

        if (
            state.lastLocale !==
                locale
        ) {
            state.lastLocale =
                locale;

            q(
                '#' + DRAWER_ID
            )?.remove();

            ensureMapToggle();
        }

        const drawer =
            ensureDrawer();

        if (!drawer) {
            return;
        }

        drawer.hidden =
            false;

        renderFilterControls(
            drawer
        );

        const results =
            filteredSpecies();

        renderXpSummary(
            drawer,
            results
        );

        const body =
            q(
                '.mha-body',
                drawer
            );

        const limit =
            Math.max(
                120,
                Number(
                    state.resultLimit ||
                    120
                )
            );

        const visibleResults =
            results.slice(
                0,
                limit
            );

        const hasMore =
            results.length >
            visibleResults.length;

        const html =
            visibleResults.length
                ? visibleResults
                    .map(
                        speciesMarkup
                    )
                    .join('') +
                  (
                      hasMore
                          ? `
                              <button type="button" class="mha-show-more" data-mha-show-more>
                                  ${escapeHtml(
                                      tr(
                                          'results.showMore',
                                          {
                                              count:
                                                  results.length -
                                                  visibleResults.length
                                          }
                                      )
                                  )}
                              </button>
                          `
                          : ''
                  )
                : `
                    <div class="mha-empty">
                        ${escapeHtml(
                            tr(
                                state.hunts.length
                                    ? 'results.noMatch'
                                    : 'results.waiting'
                            )
                        )}
                    </div>
                `;

        if (
            body.innerHTML !==
            html
        ) {
            body.innerHTML =
                html;

            for (
                const button of
                qa(
                    '[data-mha-go]',
                    body
                )
            ) {
                button.addEventListener(
                    'click',
                    () => {
                        const slug =
                            button.dataset
                                .mhaGo;

                        const hunt =
                            state.hunts.find(
                                item =>
                                    item.slug ===
                                    slug
                            );

                        if (hunt) {
                            navigateToHunt(
                                hunt
                            );
                        }
                    }
                );
            }

            q(
                '[data-mha-show-more]',
                body
            )?.addEventListener(
                'click',
                () => {
                    state.resultLimit =
                        limit + 120;

                    renderDrawer();
                }
            );
        }

        const status =
            q(
                '.mha-status',
                drawer
            );

        const typeSuffix =
            state.typeStatus ===
                'loading'
                ? tr(
                    'status.loadingTypes'
                )
                : state.typeStatus ===
                    'error'
                    ? tr(
                        'status.typesUnavailable'
                    )
                    : '';

        const marketSuffix =
            state.marketStatus ===
                'loading'
                ? tr(
                    'status.marketSampling'
                )
                : '';

        const statusText =
            localizedStatus() +
            typeSuffix +
            marketSuffix;

        if (
            status.textContent !==
            statusText
        ) {
            status.textContent =
                statusText;
        }

        const count =
            q(
                '.mha-count',
                drawer
            );

        const huntCount =
            new Set(
                results.flatMap(
                    species =>
                        species.hunts.map(
                            item =>
                                item.hunt.slug
                        )
                )
            ).size;

        const countText =
            hasMore
                ? tr(
                    'results.countPartial',
                    {
                        shown:
                            visibleResults.length,
                        pokemon:
                            results.length,
                        hunts:
                            huntCount
                    }
                )
                : tr(
                    'results.count',
                    {
                        pokemon:
                            results.length,
                        hunts:
                            huntCount
                    }
                );

        if (
            count.textContent !==
            countText
        ) {
            count.textContent =
                countText;
        }
    }

    function queueRender() {
        if (
            state.renderQueued
        ) {
            return;
        }

        state.renderQueued =
            true;

        queueMicrotask(
            () => {
                state.renderQueued =
                    false;

                ensureButton();

                if (
                    isMapModalOpen()
                ) {
                    ensureMapToggle();

                    if (
                        state.atlasEnabled
                    ) {
                        state.drawerOpen =
                            true;
                        renderDrawer();
                    } else {
                        state.drawerOpen =
                            false;
                        restoreNativeMapView();
                    }
                } else {
                    cleanupMapIntegration();
                }
            }
        );
    }

    function bootstrap() {
        injectStyles();
        ensureButton();
        installMapObserver();
        scheduleMapSync();

        setInterval(
            () => {
                ensureButton();

                if (
                    isMapModalOpen()
                ) {
                    ensureMapToggle();

                    if (
                        state.atlasEnabled
                    ) {
                        state.drawerOpen =
                            true;
                        renderDrawer();
                    } else {
                        state.drawerOpen =
                            false;
                        restoreNativeMapView();
                    }
                } else {
                    cleanupMapIntegration();
                }
            },
            3000
        );

        console.info(
            '[PokéIdle Hunt Atlas] v1.6.1 loaded'
        );
    }

    if (
        document.readyState ===
        'loading'
    ) {
        document.addEventListener(
            'DOMContentLoaded',
            bootstrap,
            {
                once: true
            }
        );
    } else {
        bootstrap();
    }
})();
