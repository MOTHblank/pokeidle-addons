// ==UserScript==
// @name         Moth Controller Bridge
// @namespace    moth.pokeidle
// @version      1.0.0
// @description  Lightweight protocol bridge for the native Moth controller.
// @match        https://pokeidle.io/app*
// @grant        unsafeWindow
// @run-at       document-start
// @noframes
// ==/UserScript==

(() => {
    'use strict';

    const page = typeof unsafeWindow !== 'undefined' ? unsafeWindow : window;
    if (page.__mothControllerBridgeV1) return;

    let socket = null;
    let state = null;
    let hunts = [];
    let catalog = [];
    const market = [];
    const events = [];
    const MAX_MARKET = 80;
    const MAX_EVENTS = 250;

    const copy = value => {
        try { return JSON.parse(JSON.stringify(value)); } catch { return null; }
    };

    function merge(partial, full = false) {
        if (!partial || typeof partial !== 'object') return;

        const { cheio, pokemons, pkMud, pkFora, dexMud, dexFora, ...rest } = partial;

        if (full || cheio === true || !state) {
            state = { ...rest, pokemons: Array.isArray(pokemons) ? pokemons.slice() : [] };
            return;
        }

        Object.assign(state, rest);

        if (Array.isArray(pokemons)) {
            state.pokemons = pokemons.slice();
        } else {
            const byId = new Map(
                (state.pokemons || []).map(p => [String(p?.id), p])
            );

            for (const pokemon of pkMud || []) {
                if (pokemon?.id != null) byId.set(String(pokemon.id), pokemon);
            }

            for (const id of pkFora || []) byId.delete(String(id));
            state.pokemons = [...byId.values()];
        }

        if (dexMud || dexFora) {
            state.pokedex = { ...(state.pokedex || {}), ...(dexMud || {}) };
            for (const id of dexFora || []) delete state.pokedex[id];
        }
    }

    function incoming(data) {
        if (typeof data !== 'string') return;

        let message;
        try { message = JSON.parse(data); } catch { return; }

        if (message.t === 'welcome') {
            hunts = Array.isArray(message.hunts) ? message.hunts.slice() : [];
            catalog = Array.isArray(message.mercado?.catalogo)
                ? message.mercado.catalogo.slice()
                : [];
            merge(message.estado, true);
            return;
        }

        if (message.t === 'estado') {
            merge(message.estado);
            return;
        }

        if (message.t === 'batalha') {
            for (const event of message.ev || []) {
                events.push({
                    at: Date.now(),
                    hunt: state?.huntSlug || '',
                    event: copy(event)
                });
            }
            if (events.length > MAX_EVENTS)
                events.splice(0, events.length - MAX_EVENTS);
            return;
        }

        if (message.t === 'market') {
            market.push({ at: Date.now(), message: copy(message) });
            if (market.length > MAX_MARKET)
                market.splice(0, market.length - MAX_MARKET);
        }
    }

    function attach(ws) {
        socket = ws;

        ws.addEventListener('close', () => {
            if (socket === ws) socket = null;
        });

        ws.addEventListener('message', event => {
            if (typeof Blob !== 'undefined' && event.data instanceof Blob) {
                event.data.text().then(incoming).catch(() => {});
            } else {
                incoming(event.data);
            }
        });
    }

    function installHook() {
        const NativeWebSocket = page.WebSocket;
        if (typeof NativeWebSocket !== 'function') return false;

        const Wrapped = new Proxy(NativeWebSocket, {
            construct(target, args) {
                const ws = Reflect.construct(target, args, target);
                attach(ws);
                return ws;
            }
        });

        try {
            page.WebSocket = Wrapped;
            return page.WebSocket === Wrapped;
        } catch {
            return false;
        }
    }

    function send(payload) {
        if (!socket || socket.readyState !== page.WebSocket.OPEN) {
            return { ok: false, error: 'game websocket is not open' };
        }

        try {
            socket.send(JSON.stringify(payload));
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
        const lines = (document.body?.innerText || '')
            .split(/\n+/)
            .map(v => v.replace(/\s+/g, ' ').trim())
            .filter(Boolean);

        return lines.find(line =>
            /\+\s*15\s*%/i.test(line) &&
            /\b(?:XP|EXP|experi)/i.test(line)
        ) || '';
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

        const bonusLines = (document.body?.innerText || '')
            .split(/\n+/)
            .map(v => v.replace(/\s+/g, ' ').trim())
            .filter(Boolean)
            .filter(line =>
                /\+\s*\d+\s*%/.test(line) &&
                /\b(?:XP|EXP|experi)/i.test(line)
            )
            .slice(0, 8);

        const text = selector => {
            const el = document.querySelector(selector);
            return el ? (el.textContent || '').replace(/\s+/g, ' ').trim() : '';
        };

        const numberFrom = selector => {
            const match = text(selector).match(/\d[\d.,]*/);
            return match ? Number(match[0].replace(/\D/g, '')) || 0 : 0;
        };

        return {
            connected: !!socket && socket.readyState === page.WebSocket.OPEN,
            state: {
                level: Number.isFinite(Number(state?.level)) ? Number(state.level) : null,
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
                fallen: fallen(),
                visibleStreamBonus: bodyTextBonus(),
                xpBonuses: [...new Set(bonusLines)],
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
            hunts: hunts.map(h => ({
                slug: String(h?.slug || ''),
                name: String(h?.nome || h?.name || h?.slug || ''),
                level: Number(h?.nivel ?? h?.level) || 0,
                species: Array.isArray(h?.especies)
                    ? h.especies.map(s => ({
                        id: Number(s?.pokeId ?? s?.speciesId) || 0,
                        name: String(s?.nome || s?.name || '')
                    }))
                    : []
            })),
            catalog: copy(catalog) || [],
            market: copy(market) || [],
            battleEvents: copy(events) || []
        };
    }

    page.__mothControllerBridgeV1 = {
        version: 1,
        send,
        snapshot
    };

    page.__mothControllerHeadless = true;
    installHook();
})();
