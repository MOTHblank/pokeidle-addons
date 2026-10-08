// ==UserScript==
// @name         Moth Controller Bridge
// @namespace    moth.pokeidle
// @version      1.3.3
// @description  Lightweight protocol bridge for the native Moth controller.
// @match        https://pokeidle.io/app*
// @updateURL    https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/controller-bridge.user.js
// @downloadURL  https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/controller-bridge.user.js
// @grant        unsafeWindow
// @run-at       document-start
// @noframes
// ==/UserScript==

(() => {
    'use strict';

    const page = typeof unsafeWindow !== 'undefined' ? unsafeWindow : window;
    const existingBridge = page.__mothControllerBridgeV1;
    if (
        existingBridge &&
        Number(existingBridge.version) >= 5 &&
        typeof existingBridge.snapshot === 'function' &&
        typeof existingBridge.gameSnapshot === 'function' &&
        typeof existingBridge.socket === 'function'
    ) {
        return;
    }

    let socket = null;
    let gameSocket = null;
    let state = null;
    let statePokemon = new Map();
    let hunts = [];
    let catalog = [];
    let serverTypeChart = null;
    let huntAmplification = 1.5;
    const market = [];
    const events = [];
    const MAX_MARKET = 80;
    const MAX_EVENTS = 250;

    let lastStreamBonus = '';
    let lastStreamBonusAt = 0;
    let lastMessageAt = 0;
    let lastBattleAt = 0;

    const copy = value => {
        try { return JSON.parse(JSON.stringify(value)); } catch { return null; }
    };

    /*
     * The current upstream client advertises delta: 1 in hello and then
     * receives partial state frames. Mirror shared/estado-delta.mjs here so
     * the bridge always exposes the same complete state that the game sees.
     */
    function merge(partial, full = false) {
        if (!partial || typeof partial !== 'object') return state;

        const {
            cheio,
            pokemons,
            pkMud,
            pkFora,
            dexMud,
            dexFora,
            ...rest
        } = partial;

        if (full || cheio || !state) {
            state = { ...rest };
            statePokemon = new Map(
                (Array.isArray(pokemons) ? pokemons : []).map(p => [p?.id, p])
            );
            return buildState();
        }

        Object.assign(state, rest);

        if (Array.isArray(pokemons)) {
            statePokemon = new Map(pokemons.map(p => [p?.id, p]));
        }

        for (const pokemon of pkMud || []) {
            if (pokemon?.id != null) statePokemon.set(pokemon.id, pokemon);
        }

        for (const id of pkFora || []) {
            statePokemon.delete(id);
        }

        if (dexMud || dexFora) {
            state.pokedex = { ...(state.pokedex || {}), ...(dexMud || {}) };
            for (const id of dexFora || []) delete state.pokedex[id];
        }

        return buildState();
    }

    function buildState() {
        return state
            ? { ...state, pokemons: [...statePokemon.values()] }
            : null;
    }
    function incoming(data, sourceSocket = null) {
        if (typeof data !== 'string') return;

        let message;
        try { message = JSON.parse(data); } catch { return; }

        if (message.t !== 'pong') {
            lastMessageAt = Date.now();
        }

        /*
         * PokéIdle can create more than one WebSocket over the lifetime of a
         * page. The authoritative game socket is the one that emits welcome
         * and battle/market protocol messages. Do not let an unrelated socket
         * replace it, or controller commands (hunt.select / market.comprar)
         * can silently go to the wrong connection.
         */
        if (message.t === 'welcome' && sourceSocket) {
            socket = sourceSocket;
            gameSocket = sourceSocket;
        }

        if (message.t === 'welcome') {
            if (Array.isArray(message.hunts)) {
                hunts = copy(message.hunts) || [];
            } else if (Array.isArray(message.estado?.hunts)) {
                hunts = copy(message.estado.hunts) || [];
            } else {
                hunts = [];
            }

            if (
                message.tabelaTipos &&
                typeof message.tabelaTipos === 'object'
            ) {
                serverTypeChart = copy(message.tabelaTipos);
            }

            if (Number.isFinite(Number(message.ampliacaoHunt))) {
                huntAmplification = Number(message.ampliacaoHunt);
            }

            catalog = Array.isArray(message.mercado?.catalogo)
                ? message.mercado.catalogo.slice()
                : [];
            merge(message.estado, true);
            return;
        }

        if (Array.isArray(message.hunts)) {
            hunts = copy(message.hunts) || hunts;
        }

        if (message.estado?.hunts && Array.isArray(message.estado.hunts)) {
            hunts = copy(message.estado.hunts) || hunts;
        }

        if (message.tabelaTipos && typeof message.tabelaTipos === 'object') {
            serverTypeChart = copy(message.tabelaTipos);
        }

        if (Number.isFinite(Number(message.ampliacaoHunt))) {
            huntAmplification = Number(message.ampliacaoHunt);
        }

        if (message.t === 'estado') {
            merge(message.estado);
            return;
        }

        if (message.t === 'batalha') {
            lastBattleAt = Date.now();
            if (sourceSocket && (message.ev || []).some(event => event?.k === 'hunt')) {
                socket = sourceSocket;
                gameSocket = sourceSocket;
            }

            let activeHunt = state?.huntSlug || '';
            for (const event of message.ev || []) {
                if (event?.k === 'hunt' && typeof event.slug === 'string' && event.slug) {
                    activeHunt = event.slug;
                    if (!state) state = {};
                    state.huntSlug = activeHunt;
                }

                events.push({
                    at: Date.now(),
                    hunt: activeHunt,
                    event: copy(event)
                });
            }

            if (events.length > MAX_EVENTS)
                events.splice(0, events.length - MAX_EVENTS);
            return;
        }

        if (message.t === 'market') {
            if (sourceSocket) {
                socket = sourceSocket;
                gameSocket = sourceSocket;
            }
            market.push({ at: Date.now(), message: copy(message) });
            if (market.length > MAX_MARKET)
                market.splice(0, market.length - MAX_MARKET);
        }
    }

    function attach(ws) {
        if (!socket) socket = ws;

        ws.addEventListener('close', () => {
            if (gameSocket === ws) gameSocket = null;
            if (socket === ws) socket = gameSocket || null;
        });

        ws.addEventListener('message', event => {
            if (typeof Blob !== 'undefined' && event.data instanceof Blob) {
                event.data.text().then(text => incoming(text, ws)).catch(() => {});
            } else {
                incoming(event.data, ws);
            }
        });
    }

    function installHook() {
        /*
         * PokéIdle assigns its real message handler with ws.onmessage.
         * We need to observe that first assignment, but changing both
         * onmessage and onclose accessors permanently is unnecessarily risky
         * during document-start and can interfere with native WebSocket setup.
         *
         * Temporarily shadow only onmessage. As soon as the game installs its
         * handler, attach our passive listeners and immediately restore the
         * exact native descriptor. From that point onward the browser/game
         * owns WebSocket completely again.
         */
        const NativeWebSocket = page.WebSocket;
        const proto = NativeWebSocket && NativeWebSocket.prototype;
        if (!proto) return false;

        let owner = proto;
        let descriptor = null;

        while (owner && !descriptor) {
            descriptor =
                Object.getOwnPropertyDescriptor(
                    owner,
                    'onmessage'
                ) || null;
            if (!descriptor) {
                owner = Object.getPrototypeOf(owner);
            }
        }

        if (
            !descriptor ||
            typeof descriptor.set !== 'function' ||
            descriptor.configurable === false
        ) {
            return false;
        }

        const attached = new WeakSet();
        let restored = false;

        const restore = () => {
            if (restored) return;
            restored = true;

            try {
                if (owner === proto) {
                    Object.defineProperty(
                        proto,
                        'onmessage',
                        descriptor
                    );
                } else {
                    delete proto.onmessage;
                }
            } catch {
                /*
                 * Restoration failure must never propagate into the game's
                 * event-handler setter. The native setter was already called.
                 */
            }
        };

        const observe = ws => {
            if (!ws || attached.has(ws)) return;
            attached.add(ws);

            try {
                attach(ws);
            } catch (error) {
                try {
                    console.warn(
                        '[Moth Controller Bridge] socket observation failed:',
                        error
                    );
                } catch {}
            }
        };

        const nativeGet = descriptor.get;
        const nativeSet = descriptor.set;

        try {
            Object.defineProperty(
                proto,
                'onmessage',
                {
                    configurable: true,
                    enumerable: descriptor.enumerable,
                    get: nativeGet
                        ? function() {
                            return Reflect.apply(
                                nativeGet,
                                this,
                                []
                            );
                        }
                        : undefined,
                    set: function(value) {
                        const result =
                            Reflect.apply(
                                nativeSet,
                                this,
                                [value]
                            );

                        if (
                            typeof value ===
                            'function'
                        ) {
                            observe(this);
                            restore();
                        }

                        return result;
                    }
                }
            );

            /*
             * If the game has already assigned onmessage before this userscript
             * gets here, there is nothing more to intercept. The polling
             * fallback below can still use any socket adopted by another addon,
             * while the page continues normally.
             */
            return true;
        } catch {
            restore();
            return false;
        }
    }

    function bridgeSocket() {
        return gameSocket || socket || null;
    }

    function send(payload) {
        const mothWatch = page.__mothMarketWatchControllerV1;

        if (
            mothWatch &&
            typeof mothWatch.buy === 'function' &&
            payload &&
            payload.t === 'market.comprar'
        ) {
            try {
                return mothWatch.buy(payload);
            } catch (error) {
                return { ok: false, error: String(error) };
            }
        }

        if (
            mothWatch &&
            typeof mothWatch.configure === 'function' &&
            payload &&
            payload.t === 'mothWatch.configure'
        ) {
            try {
                return mothWatch.configure(payload.patch || {});
            } catch (error) {
                return { ok: false, error: String(error) };
            }
        }

        if (
            mothWatch &&
            typeof mothWatch.scan === 'function' &&
            payload &&
            payload.t === 'mothWatch.scan'
        ) {
            try {
                return mothWatch.scan();
            } catch (error) {
                return { ok: false, error: String(error) };
            }
        }

        if (
            mothWatch &&
            typeof mothWatch.refreshBaseline === 'function' &&
            payload &&
            payload.t === 'mothWatch.refreshBaseline'
        ) {
            try {
                return mothWatch.refreshBaseline();
            } catch (error) {
                return { ok: false, error: String(error) };
            }
        }

        const target = gameSocket || socket;
        if (!target || target.readyState !== page.WebSocket.OPEN) {
            return { ok: false, error: 'game websocket is not open' };
        }

        try {
            target.send(JSON.stringify(payload));
            return { ok: true };
        } catch (error) {
            return { ok: false, error: String(error) };
        }
    }

    function pickNumber(value, names, depth = 0, seen = new Set()) {
        if (!value || typeof value !== 'object' || depth > 4 || seen.has(value)) return null;
        seen.add(value);

        for (const [key, raw] of Object.entries(value)) {
            const lower = key.toLowerCase();
            if (names.has(lower) && (typeof raw === 'number' || typeof raw === 'string')) {
                const n = Number(raw);
                if (Number.isFinite(n)) return n;
            }
            if (raw && typeof raw === 'object') {
                const nested = pickNumber(raw, names, depth + 1, seen);
                if (nested != null) return nested;
            }
        }
        return null;
    }

    function activePokemon() {
        const id = state?.activeId;
        return id == null
            ? null
            : (state?.pokemons || []).find(p => Number(p?.id) === Number(id)) || null;
    }

    function bodyTextBonus() {
        // Upstream renders the bonus in these explicit HUD nodes. Avoid a
        // full document.body.innerText traversal on every controller poll.
        const sources = [
            document.querySelector('#tr-ativos')?.innerText || '',
            document.querySelector('#evento-texto')?.innerText || '',
            document.querySelector('#evento-faixa')?.innerText || ''
        ];

        const lines = sources
            .join('\n')
            .split(/\n+/)
            .map(v => v.replace(/\s+/g, ' ').trim())
            .filter(Boolean);

        const current = lines.find(line =>
            /\+\s*15\s*%/i.test(line) &&
            /\b(?:XP|EXP|experi)/i.test(line)
        ) || '';

        if (current) {
            lastStreamBonus = current;
            lastStreamBonusAt = Date.now();
        }

        return current;
    }

    function fallen() {
        return document.querySelectorAll('#caidos-lista .caido').length;
    }

    function balls() {
        return [...document.querySelectorAll('#caidos-bolas button.caidos-bola')]
            .map(button => {
                const raw = button.title || button.getAttribute('aria-label') || '';
                const match = raw.match(/(?:você\s+tem|voce\s+tem|you\s+have|tienes)\s+([\d.,]+)/i);
                if (!match) return null;
                return {
                    name: (raw.split('—')[0] || 'Ball').replace(/\s+/g, ' ').trim(),
                    count: Number(match[1].replace(/\D/g, '')) || 0
                };
            })
            .filter(Boolean);
    }

    function snapshot() {
        const pokemon = activePokemon();
        const scanner = document.querySelector('#moth-scan-live-streams');

        const text = selector => {
            const el = document.querySelector(selector);
            return el ? (el.textContent || '').replace(/\s+/g, ' ').trim() : '';
        };

        const currentStreamBonus = bodyTextBonus();

        const twitch = state?.twitch || {};
        const twitchPct = Number(
            twitch.pctAtual ??
            twitch.pct ??
            0
        );

        const twitchWatching = Array.isArray(twitch.assistindoEm)
            ? twitch.assistindoEm.filter(Boolean)
            : [];

        const kickScanner = (() => {
            try {
                const api = page.__mothKickScannerV1;
                return api && typeof api.snapshot === 'function'
                    ? copy(api.snapshot())
                    : null;
            } catch {
                return null;
            }
        })();

        if (
            twitchWatching.length &&
            twitchPct > 0
        ) {
            lastStreamBonus =
                '+' +
                twitchPct +
                '% XP · watching ' +
                twitchWatching.join(', ');
            lastStreamBonusAt = Date.now();
        }

        const playerXp = text('#tr-xp-txt');

        const bonusLines = [
            text('#tr-ativos'),
            text('#evento-texto'),
            text('#evento-faixa')
        ]
            .filter(Boolean)
            .filter(line =>
                /\+\s*\d+\s*%/.test(line) &&
                /\b(?:XP|EXP|experi)/i.test(line)
            )
            .slice(0, 8);

        const numberFrom = selector => {
            const match = text(selector).match(/\d[\d.,]*/);
            return match ? Number(match[0].replace(/\D/g, '')) || 0 : 0;
        };

        return {
            connected: !!gameSocket && gameSocket.readyState === page.WebSocket.OPEN,
            lastMessageAt,
            lastBattleAt,
            huntChangeCooldownMs: lastBattleAt
                ? Math.max(0, 2600 - (Date.now() - lastBattleAt))
                : 0,
            state: {
                level: Number.isFinite(Number(state?.level)) ? Number(state.level) : null,
                xp: Number(state?.xp) || 0,
                xpNivel: Number(state?.xpNivel) || 0,
                xpProximo: Number(state?.xpProximo) || 0,
                huntSlug: state?.huntSlug || '',
                activeId: state?.activeId ?? null,
                gold: Number(state?.gold) || 0,
                orbs: Number(state?.orbs) || 0,
                balls: copy(state?.balls) || {},
                items: copy(state?.items) || {},
                activePokemon: copy(pokemon),
                activePokemonXp: pickNumber(
                    pokemon,
                    new Set(['xp','exp','experiencia','experience','xpAtual','expAtual'])
                ),
                activePokemonXpNivel: Number(pokemon?.xpNivel) || 0,
                activePokemonXpProximo: Number(pokemon?.xpProximo) || 0,
                fallen: fallen(),
                playerXp,
                visibleStreamBonus: currentStreamBonus,
                lastStreamBonus,
                lastStreamBonusAt,
                twitch: copy(state?.twitch),
                kick: copy(state?.kick),
                loja: copy(state?.loja),
                guild: copy(state?.guild),
                passe: copy(state?.passe),
                evento: copy(state?.evento),
                guildBonusPct: Number(state?.guildBonusPct) || 0,
                guildBonusRank: Number(state?.guildBonusRank) || 0,
                trainerXp: Number(state?.xp) || 0,
                trainerXpNivel: Number(state?.xpNivel) || 0,
                trainerXpProximo: Number(state?.xpProximo) || 0,
                xpBonuses: [...new Set(bonusLines)],
                serverNow: Number(state?.servidorAgora) || 0,
                mothWatch: (() => {
                    try {
                        const api = page.__mothMarketWatchControllerV1;
                        return api && typeof api.snapshot === 'function'
                            ? copy(api.snapshot())
                            : null;
                    } catch {
                        return null;
                    }
                })(),
                huntAtlas: (() => {
                    try {
                        const api = page.__mothHuntAtlasControllerV1;
                        return api && typeof api.snapshot === 'function'
                            ? copy(api.snapshot())
                            : null;
                    } catch {
                        return null;
                    }
                })(),
                domBalls: balls(),
                autoCatchOn: text('#moth-ac-toggle').toUpperCase() === 'ON',
                autoCatchCaptures: numberFrom('#moth-ac-captures'),
                autoCatchBallsUsed: numberFrom('#moth-ac-balls-used'),
                autoCatchRate: text('#moth-ac-rate'),
                autoCatchRestock: text('#moth-ac-restock-status'),
                streamScanStatus: scanner?.dataset?.mothScanStatus || '',
                streamScanLive: Number(scanner?.dataset?.mothScanLive || 0) || 0,
                streamScanOpened: Number(scanner?.dataset?.mothScanOpened || 0) || 0
            },
            hunts: copy(hunts) || [],
            stream: {
                twitch: copy(state?.twitch),
                kick: copy(state?.kick),
                kickScanner,
                bonusCurrent: currentStreamBonus,
                bonusLast: lastStreamBonus
            },
            catalog: copy(catalog) || [],
            market: copy(market) || [],
            battleEvents: copy(events) || []
        };
    }

    function gameSnapshot() {
        return {
            connected: !!gameSocket && gameSocket.readyState === page.WebSocket.OPEN,
            lastMessageAt,
            lastBattleAt,
            state: {
                level: Number.isFinite(Number(state?.level)) ? Number(state.level) : null,
                xp: Number(state?.xp) || 0,
                xpNivel: Number(state?.xpNivel) || 0,
                xpProximo: Number(state?.xpProximo) || 0,
                huntSlug: state?.huntSlug || '',
                activeId: state?.activeId ?? null,
                noCentro: state?.noCentro ?? null,
                casa: copy(state?.casa),
                boss: copy(state?.boss),
                evento: copy(state?.evento),
                pokemons: copy(state?.pokemons) || [],
                pokedex: copy(state?.pokedex) || {}
            },
            hunts: copy(hunts) || [],
            serverTypeChart: copy(serverTypeChart),
            huntAmplification
        };
    }

    page.__mothControllerBridgeV1 = {
        version: 5,
        send,
        snapshot,
        gameSnapshot,
        socket: bridgeSocket
    };

    page.__mothControllerHeadless = true;
    installHook();
})();
