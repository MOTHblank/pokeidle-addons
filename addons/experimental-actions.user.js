// ==UserScript==
// @name         PokéIdle Experimental Actions
// @namespace    moth.pokeidle
// @version      1.3.0
// @description  Reversible, gameplay-useful protocol probes for PokéIdle actions that may work outside their normal UI context.
// @match        https://pokeidle.io/*
// @match        https://www.pokeidle.io/*
// @grant        unsafeWindow
// @run-at       document-start
// ==/UserScript==

(() => {
    'use strict';

    const ROOT_ID = 'moth-experimental-actions';
    const BUTTON_ID = 'moth-experimental-actions-button';
    const STYLE_ID = 'moth-experimental-actions-style';
    const SCRAPE_BUTTON_ID = 'moth-upstream-scraper-button';

    const RESPONSE_WINDOW_MS = 3200;
    const CLICK_COOLDOWN_MS = 700;
    const MAX_HISTORY = 30;

    const page =
        typeof unsafeWindow !== 'undefined'
            ? unsafeWindow
            : window;

    const q = (selector, root = document) =>
        root.querySelector(selector);

    const qa = (selector, root = document) =>
        [...root.querySelectorAll(selector)];

    const runtime = {
        socket: null,
        hookInstalled: false,
        socketsSeen: 0,
        messagesSeen: 0,

        gameState: null,
        welcome: null,
        catalogoBolas: [],
        bossesJogaveis: [],

        activeProbe: null,
        probeTimer: null,
        lastActionAt: 0,
        history: []
    };

    function escapeHtml(value) {
        return String(value == null ? '' : value)
            .replaceAll('&', '&amp;')
            .replaceAll('<', '&lt;')
            .replaceAll('>', '&gt;')
            .replaceAll('"', '&quot;')
            .replaceAll("'", '&#039;');
    }

    function clone(value) {
        if (value == null) return value;

        try {
            return structuredClone(value);
        } catch {
            try {
                return JSON.parse(JSON.stringify(value));
            } catch {
                return value;
            }
        }
    }

    function isPlainObject(value) {
        return Boolean(
            value &&
            typeof value === 'object' &&
            !Array.isArray(value)
        );
    }

    function mergeDelta(base, delta) {
        if (!isPlainObject(delta)) {
            return clone(delta);
        }

        const out =
            isPlainObject(base)
                ? { ...base }
                : {};

        for (const [key, value] of Object.entries(delta)) {
            if (isPlainObject(value)) {
                out[key] =
                    mergeDelta(
                        out[key],
                        value
                    );
            } else {
                out[key] =
                    clone(value);
            }
        }

        return out;
    }

    function decodeProtocolData(data) {
        if (typeof data === 'string') {
            try {
                return JSON.parse(data);
            } catch {
                return null;
            }
        }

        if (data instanceof ArrayBuffer) {
            try {
                return JSON.parse(
                    new TextDecoder().decode(data)
                );
            } catch {
                return null;
            }
        }

        return null;
    }

    function socketReady() {
        return Boolean(
            runtime.socket &&
            runtime.socket.readyState ===
                page.WebSocket.OPEN
        );
    }

    function sceneLabel(state = runtime.gameState) {
        if (!state) return 'unknown';

        if (state.mistico) {
            return 'mystic arena';
        }

        if (state.boss && state.boss.arena) {
            return 'boss arena';
        }

        if (state.casa && state.casa.dentro != null) {
            return 'house #' + state.casa.dentro;
        }

        if (state.noCentro) {
            return 'PokéCenter';
        }

        if (state.huntSlug) {
            return 'hunt ' + state.huntSlug;
        }

        return 'no scene';
    }

    function summary(state = runtime.gameState) {
        if (!state) {
            return {
                scene: 'unknown',
                level: null,
                noCentro: false,
                huntSlug: null,
                casaDentro: null,
                bossArena: false,
                mistico: false,
                activeId: null,
                gold: null,
                diamonds: null,
                outlandTier: null,
                team: 0,
                teamSlots: '',
                lootLocks: '',
                collection: ''
            };
        }

        const pokemons =
            Array.isArray(state.pokemons)
                ? state.pokemons
                : [];

        const team =
            pokemons
                .filter(
                    pokemon =>
                        pokemon.slot != null
                )
                .sort(
                    (a, b) =>
                        Number(a.slot) -
                        Number(b.slot)
                );

        const automation =
            isPlainObject(state.automation)
                ? state.automation
                : {};

        const normalizedIds =
            values =>
                Array.isArray(values)
                    ? values
                        .map(Number)
                        .filter(Number.isFinite)
                        .sort((a, b) => a - b)
                        .join(',')
                    : '';

        return {
            scene:
                sceneLabel(state),
            level:
                state.level != null
                    ? Number(state.level)
                    : null,
            noCentro:
                Boolean(state.noCentro),
            huntSlug:
                state.huntSlug != null
                    ? state.huntSlug
                    : null,
            casaDentro:
                state.casa &&
                state.casa.dentro != null
                    ? state.casa.dentro
                    : null,
            bossArena:
                Boolean(
                    state.boss &&
                    state.boss.arena
                ),
            mistico:
                Boolean(state.mistico),
            activeId:
                state.activeId != null
                    ? state.activeId
                    : null,
            gold:
                state.gold != null
                    ? Number(state.gold)
                    : null,
            diamonds:
                state.diamonds != null
                    ? Number(state.diamonds)
                    : null,
            outlandTier:
                state.outlandTier != null
                    ? Number(state.outlandTier)
                    : null,
            team:
                team.length,
            teamSlots:
                team
                    .map(
                        pokemon =>
                            String(pokemon.id) +
                            ':' +
                            String(pokemon.slot)
                    )
                    .join(','),
            lootLocks:
                normalizedIds(
                    automation.lootTravado
                ),
            collection:
                normalizedIds(
                    automation.pokemonTravado
                )
        };
    }

    function diffSummary(before, after) {
        const changes = [];

        for (const key of Object.keys(after || {})) {
            const a =
                before
                    ? before[key]
                    : undefined;

            const b =
                after[key];

            if (
                JSON.stringify(a) !==
                JSON.stringify(b)
            ) {
                changes.push(
                    key +
                    ': ' +
                    String(a) +
                    ' → ' +
                    String(b)
                );
            }
        }

        return changes;
    }


    function relevantKeysForProbe(
        probe
    ) {
        const map = {
            'switch-hunt-roundtrip': [
                'scene',
                'huntSlug',
                'noCentro'
            ],
            'team-add-roundtrip': [
                'team',
                'teamSlots',
                'activeId'
            ],
            'team-park-roundtrip': [
                'team',
                'teamSlots',
                'activeId'
            ],
            'loot-lock-roundtrip': [
                'lootLocks'
            ],
            'collection-roundtrip': [
                'collection'
            ],
            'outland-tier-roundtrip': [
                'outlandTier',
                'huntSlug',
                'scene'
            ]
        };

        return map[
            probe &&
            probe.id
        ] || [];
    }

    function diffRelevantSummary(
        probe,
        before,
        after
    ) {
        const allowed =
            new Set(
                relevantKeysForProbe(
                    probe
                )
            );

        if (!allowed.size) {
            return [];
        }

        return diffSummary(
            before,
            after
        ).filter(
            change =>
                allowed.has(
                    change.split(':')[0]
                )
        );
    }

    function compactPayload(payload) {
        try {
            return JSON.stringify(payload);
        } catch {
            return String(payload);
        }
    }

    function addProbeEvent(text, kind) {
        const probe =
            runtime.activeProbe;

        if (!probe) return;

        const now =
            Date.now();

        probe.events.push({
            at:
                now -
                probe.startedAt,
            kind:
                kind || 'message',
            text:
                String(text)
        });

        if (probe.events.length > 40) {
            probe.events.splice(
                0,
                probe.events.length - 40
            );
        }

        updateUI();
    }

    function summarizeBattleEvent(event) {
        if (!event || !event.k) return null;

        if (event.k === 'aviso') {
            return {
                kind: 'notice',
                text:
                    'aviso: ' +
                    String(
                        event.msg ||
                        event.texto ||
                        event.mensagem ||
                        '(no text)'
                    )
            };
        }

        if (event.k === 'joy') {
            return {
                kind: 'accepted',
                text:
                    'joy: ' +
                    String(
                        event.msg ||
                        event.texto ||
                        event.mensagem ||
                        'server responded'
                    )
            };
        }

        if (
            event.k === 'afiliadoRecolhido'
        ) {
            return {
                kind: 'accepted',
                text:
                    'affiliate claimed: ' +
                    String(
                        Number(event.diamantes || 0)
                    ) +
                    ' diamonds · ' +
                    String(
                        Number(event.gemas || 0)
                    ) +
                    ' gems'
            };
        }

        if (
            event.k === 'erro' ||
            event.k === 'error'
        ) {
            return {
                kind: 'error',
                text:
                    event.k +
                    ': ' +
                    String(
                        event.msg ||
                        event.texto ||
                        '(no text)'
                    )
            };
        }

        return null;
    }

    function inspectIncoming(data) {
        runtime.messagesSeen++;

        const message =
            decodeProtocolData(data);

        if (!message) return;

        if (message.t === 'welcome') {
            runtime.welcome =
                clone(message);

            runtime.gameState =
                clone(message.estado || null);

            runtime.catalogoBolas =
                Array.isArray(message.catalogoBolas)
                    ? clone(message.catalogoBolas)
                    : [];

            runtime.bossesJogaveis =
                Array.isArray(message.bossesJogaveis)
                    ? clone(message.bossesJogaveis)
                    : [];

            updateUI();
        } else if (message.t === 'estado') {
            runtime.gameState =
                mergeDelta(
                    runtime.gameState,
                    message.estado || {}
                );
        }

        const probe =
            runtime.activeProbe;

        if (!probe) {
            updateUI();
            return;
        }

        if (
            Array.isArray(probe.expect) &&
            probe.expect.includes(
                message.t
            )
        ) {
            addProbeEvent(
                'expected response: ' +
                String(message.t),
                'accepted'
            );
        }

        const rejection =
            message.recusa ||
            message.erro ||
            message.error ||
            null;

        if (message.t === 'erro') {
            addProbeEvent(
                'error: ' +
                String(
                    message.msg ||
                    message.texto ||
                    message.mensagem ||
                    rejection ||
                    '(no text)'
                ),
                'error'
            );
        } else if (rejection) {
            addProbeEvent(
                String(message.t || 'message') +
                ' rejection: ' +
                (
                    typeof rejection === 'string'
                        ? rejection
                        : compactPayload(
                            rejection
                        )
                ),
                'notice'
            );
        }

        if (message.t === 'batalha' && Array.isArray(message.ev)) {
            for (const event of message.ev) {
                const summarized =
                    summarizeBattleEvent(event);

                if (summarized) {
                    addProbeEvent(
                        summarized.text,
                        summarized.kind
                    );
                }
            }
        } else if (message.t === 'campo.init') {
            addProbeEvent(
                'campo.init received',
                'state'
            );
        } else if (
            message.t !== 'campo' &&
            message.t !== 'pong' &&
            message.t !== 'estado'
        ) {
            addProbeEvent(
                'message: ' +
                String(message.t || 'unknown'),
                'message'
            );
        }

        if (message.t === 'estado') {
            const nowSummary =
                summary();

            const previous =
                probe.lastSummary ||
                probe.before;

            const changes =
                diffRelevantSummary(
                    probe,
                    previous,
                    nowSummary
                );

            for (const change of changes) {
                addProbeEvent(
                    'state ' + change,
                    'state'
                );
            }

            probe.lastSummary =
                nowSummary;
        }

        updateUI();
    }

    function attachSocket(socket) {
        runtime.socketsSeen++;
        runtime.socket =
            socket;

        socket.addEventListener(
            'close',
            () => {
                if (
                    runtime.socket ===
                    socket
                ) {
                    runtime.socket =
                        null;

                    updateUI();
                }
            }
        );

        socket.addEventListener(
            'message',
            event => {
                if (
                    typeof Blob !== 'undefined' &&
                    event.data instanceof Blob
                ) {
                    event.data
                        .text()
                        .then(inspectIncoming)
                        .catch(() => {});

                    return;
                }

                inspectIncoming(
                    event.data
                );
            }
        );
    }

    function installProtocolHook() {
        if (runtime.hookInstalled) {
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
            page.__mothExperimentalActionsHookV1
        ) {
            runtime.hookInstalled =
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

            page.__mothExperimentalActionsHookV1 =
                true;

            runtime.hookInstalled =
                page.WebSocket ===
                WrappedWebSocket;
        } catch (error) {
            console.warn(
                '[PokéIdle Experimental Actions] WebSocket hook failed',
                error
            );

            runtime.hookInstalled =
                false;
        }

        return runtime.hookInstalled;
    }

    installProtocolHook();

    function send(payload) {
        if (!socketReady()) {
            throw new Error(
                'No live game socket. Reload PokéIdle after enabling the userscript.'
            );
        }

        runtime.socket.send(
            JSON.stringify(payload)
        );
    }

    function teamPokemon() {
        const pokemons =
            Array.isArray(
                runtime.gameState &&
                runtime.gameState.pokemons
            )
                ? runtime.gameState.pokemons
                : [];

        return pokemons
            .filter(
                pokemon =>
                    pokemon.slot != null
            )
            .sort(
                (a, b) =>
                    Number(a.slot) -
                    Number(b.slot)
            );
    }

    function allPokemon() {
        return Array.isArray(
            runtime.gameState &&
            runtime.gameState.pokemons
        )
            ? runtime.gameState.pokemons
            : [];
    }

    function normalHuntActive() {
        const state =
            runtime.gameState;

        return Boolean(
            state &&
            state.huntSlug &&
            !state.noCentro &&
            !(
                state.casa &&
                state.casa.dentro != null
            ) &&
            !(
                state.boss &&
                state.boss.arena
            ) &&
            !state.mistico
        );
    }

    function huntCatalog() {
        return Array.isArray(
            runtime.welcome &&
            runtime.welcome.hunts
        )
            ? runtime.welcome.hunts
            : [];
    }

    function currentHuntDefinition() {
        const slug =
            runtime.gameState &&
            runtime.gameState.huntSlug;

        return (
            huntCatalog().find(
                hunt =>
                    hunt &&
                    hunt.slug === slug
            ) ||
            null
        );
    }

    function alternativeUnlockedHunt() {
        if (!normalHuntActive()) {
            return null;
        }

        const state =
            runtime.gameState;

        const current =
            currentHuntDefinition();

        const level =
            Number(state.level || 0);

        const candidates =
            huntCatalog()
                .filter(
                    hunt =>
                        hunt &&
                        hunt.slug &&
                        hunt.slug !== state.huntSlug &&
                        Number(hunt.nivel || 0) <= level
                )
                .sort(
                    (a, b) => {
                        const sameRegionA =
                            current &&
                            a.regiao === current.regiao
                                ? 0
                                : 1;

                        const sameRegionB =
                            current &&
                            b.regiao === current.regiao
                                ? 0
                                : 1;

                        if (
                            sameRegionA !==
                            sameRegionB
                        ) {
                            return (
                                sameRegionA -
                                sameRegionB
                            );
                        }

                        return (
                            Math.abs(
                                Number(a.nivel || 0) -
                                Number(current && current.nivel || 0)
                            ) -
                            Math.abs(
                                Number(b.nivel || 0) -
                                Number(current && current.nivel || 0)
                            )
                        );
                    }
                );

        return candidates[0] || null;
    }

    function collectionIds() {
        const values =
            runtime.gameState &&
            runtime.gameState.automation &&
            runtime.gameState.automation.pokemonTravado;

        return new Set(
            Array.isArray(values)
                ? values.map(Number)
                : []
        );
    }

    function houseXpSharePokemonIds() {
        const ids =
            new Set();

        const houses =
            runtime.gameState &&
            runtime.gameState.casa &&
            Array.isArray(
                runtime.gameState.casa.lista
            )
                ? runtime.gameState.casa.lista
                : [];

        for (const house of houses) {
            for (
                const pokemonId of
                Array.isArray(house && house.postos)
                    ? house.postos
                    : []
            ) {
                if (pokemonId != null) {
                    ids.add(
                        Number(pokemonId)
                    );
                }
            }
        }

        return ids;
    }

    function freeTeamSlot() {
        const used =
            new Set(
                teamPokemon().map(
                    pokemon =>
                        Number(pokemon.slot)
                )
            );

        const max =
            Math.max(
                1,
                Number(
                    runtime.welcome &&
                    runtime.welcome.maxEquipe ||
                    5
                )
            );

        for (
            let slot = 0;
            slot < max;
            slot++
        ) {
            if (!used.has(slot)) {
                return slot;
            }
        }

        return null;
    }

    function depotCandidate() {
        const collection =
            collectionIds();

        return (
            allPokemon().find(
                pokemon =>
                    pokemon &&
                    pokemon.id != null &&
                    pokemon.slot == null &&
                    !collection.has(
                        Number(pokemon.id)
                    )
            ) ||
            null
        );
    }

    function parkableTeamPokemon() {
        const houseIds =
            houseXpSharePokemonIds();

        const activeId =
            runtime.gameState &&
            runtime.gameState.activeId;

        return (
            teamPokemon().find(
                pokemon =>
                    Number(pokemon.id) !==
                        Number(activeId) &&
                    !houseIds.has(
                        Number(pokemon.id)
                    )
            ) ||
            null
        );
    }

    function unlockedLootItem() {
        const items =
            runtime.gameState &&
            runtime.gameState.items;

        if (
            !items ||
            typeof items !== 'object'
        ) {
            return null;
        }

        const locked =
            new Set(
                (
                    runtime.gameState.automation &&
                    Array.isArray(
                        runtime.gameState.automation.lootTravado
                    )
                        ? runtime.gameState.automation.lootTravado
                        : []
                ).map(Number)
            );

        const row =
            Object.entries(items)
                .map(
                    ([id, count]) => ({
                        id: Number(id),
                        count: Number(count) || 0
                    })
                )
                .filter(
                    item =>
                        Number.isFinite(item.id) &&
                        item.count > 0 &&
                        !locked.has(item.id)
                )
                .sort(
                    (a, b) =>
                        a.id - b.id
                )[0];

        return row || null;
    }

    function collectionCandidate() {
        return depotCandidate();
    }

    function alternativeOutlandTier() {
        if (!normalHuntActive()) {
            return null;
        }

        const hunt =
            currentHuntDefinition();

        if (
            !hunt ||
            !(
                hunt.regiao === 'outland' ||
                hunt.area === 'outland'
            )
        ) {
            return null;
        }

        const tiers =
            Array.isArray(
                runtime.welcome &&
                runtime.welcome.outlandTiers
            )
                ? runtime.welcome.outlandTiers
                : [];

        const level =
            Number(
                runtime.gameState.level ||
                0
            );

        const current =
            Number(
                runtime.gameState.outlandTier ||
                1
            );

        return (
            tiers.find(
                tier =>
                    tier &&
                    Number(tier.tier) !== current &&
                    Number(tier.nivel || 0) <= level
            ) ||
            null
        );
    }

    function actionAvailability(action) {
        if (!socketReady()) {
            return {
                ok: false,
                reason:
                    'socket not observed; reload page'
            };
        }

        if (!runtime.gameState) {
            return {
                ok: false,
                reason:
                    'waiting for game state'
            };
        }

        try {
            const result =
                action.available
                    ? action.available()
                    : true;

            if (result === true) {
                return {
                    ok: true,
                    reason: ''
                };
            }

            if (result === false) {
                return {
                    ok: false,
                    reason:
                        'not applicable now'
                };
            }

            return (
                result || {
                    ok: false,
                    reason:
                        'not applicable now'
                }
            );
        } catch (error) {
            return {
                ok: false,
                reason:
                    String(
                        error &&
                        error.message ||
                        error
                    )
            };
        }
    }

    const actions = [
        {
            id: 'switch-hunt-roundtrip',
            group: 'Highest value · easy',
            label: 'Switch hunt directly, then return',
            risk: 'medium',
            description:
                'While hunting, jump directly to another unlocked area with hunt.select, then automatically restore the original hunt. If accepted, this can become a fast hunt-switch addon feature.',
            expectsRoundTrip: true,
            successOnStateChange: true,
            responseWindowMs: 4200,
            available:
                () => {
                    const target =
                        alternativeUnlockedHunt();

                    return {
                        ok:
                            Boolean(target),
                        reason:
                            normalHuntActive()
                                ? 'needs another unlocked hunt'
                                : 'requires a normal active hunt'
                    };
                },
            steps:
                () => {
                    const original =
                        runtime.gameState.huntSlug;

                    const target =
                        alternativeUnlockedHunt();

                    return [
                        {
                            delay: 0,
                            payload: {
                                t: 'hunt.select',
                                slug:
                                    target.slug
                            }
                        },
                        {
                            delay: 1500,
                            payload: {
                                t: 'hunt.select',
                                slug:
                                    original
                            }
                        }
                    ];
                }
        },
        {
            id: 'team-add-roundtrip',
            group: 'Highest value · easy',
            label: 'Bring Depot Pokémon into hunt team',
            risk: 'medium',
            description:
                'During a normal hunt, put one Depot Pokémon into a free team slot, then send it back to Depot. Tests live team editing without visiting the Center.',
            expectsRoundTrip: true,
            successOnStateChange: true,
            responseWindowMs: 3800,
            available:
                () => ({
                    ok:
                        Boolean(
                            normalHuntActive() &&
                            depotCandidate() &&
                            freeTeamSlot() != null
                        ),
                    reason:
                        !normalHuntActive()
                            ? 'requires a normal active hunt'
                            : freeTeamSlot() == null
                                ? 'team is full'
                                : 'needs a Depot Pokémon outside the Collection'
                }),
            steps:
                () => {
                    const pokemon =
                        depotCandidate();

                    const slot =
                        freeTeamSlot();

                    return [
                        {
                            delay: 0,
                            payload: {
                                t: 'team.move',
                                pokemonId:
                                    pokemon.id,
                                slot
                            }
                        },
                        {
                            delay: 1100,
                            payload: {
                                t: 'team.move',
                                pokemonId:
                                    pokemon.id,
                                slot: null
                            }
                        }
                    ];
                }
        },
        {
            id: 'team-park-roundtrip',
            group: 'Highest value · easy',
            label: 'Park hunt teammate in Depot, then restore',
            risk: 'medium',
            description:
                'Temporarily move a non-active teammate to Depot during a normal hunt, then restore the same slot. Avoids Pokémon currently assigned to House XP Share.',
            expectsRoundTrip: true,
            successOnStateChange: true,
            responseWindowMs: 3800,
            available:
                () => ({
                    ok:
                        Boolean(
                            normalHuntActive() &&
                            parkableTeamPokemon()
                        ),
                    reason:
                        normalHuntActive()
                            ? 'needs a non-active teammate not assigned to House XP Share'
                            : 'requires a normal active hunt'
                }),
            steps:
                () => {
                    const pokemon =
                        parkableTeamPokemon();

                    const slot =
                        pokemon.slot;

                    return [
                        {
                            delay: 0,
                            payload: {
                                t: 'team.move',
                                pokemonId:
                                    pokemon.id,
                                slot: null
                            }
                        },
                        {
                            delay: 1100,
                            payload: {
                                t: 'team.move',
                                pokemonId:
                                    pokemon.id,
                                slot
                            }
                        }
                    ];
                }
        },
        {
            id: 'loot-lock-roundtrip',
            group: 'Useful shortcuts · easy',
            label: 'Lock an owned item remotely, then unlock',
            risk: 'low',
            description:
                'Toggle shop.lockLoot for one currently-unlocked owned item, then restore it. Useful if addon controls should protect drops without opening Market inventory.',
            expectsRoundTrip: true,
            successOnStateChange: true,
            responseWindowMs: 3400,
            available:
                () => ({
                    ok:
                        Boolean(
                            unlockedLootItem()
                        ),
                    reason:
                        'needs at least one owned item that is not already locked'
                }),
            steps:
                () => {
                    const item =
                        unlockedLootItem();

                    return [
                        {
                            delay: 0,
                            payload: {
                                t: 'shop.lockLoot',
                                id:
                                    item.id
                            }
                        },
                        {
                            delay: 1000,
                            payload: {
                                t: 'shop.lockLoot',
                                id:
                                    item.id
                            }
                        }
                    ];
                }
        },
        {
            id: 'collection-roundtrip',
            group: 'Useful shortcuts · slower',
            label: 'Protect Depot Pokémon, then restore',
            risk: 'medium',
            description:
                'Move one Depot Pokémon into the Collection/protected list and back. Waits through the Collection cooldown, so it is slower but tests a useful remote safety control.',
            expectsRoundTrip: true,
            successOnStateChange: true,
            responseWindowMs: 8500,
            available:
                () => ({
                    ok:
                        Boolean(
                            collectionCandidate()
                        ),
                    reason:
                        'needs a Depot Pokémon outside the Collection'
                }),
            steps:
                () => {
                    const pokemon =
                        collectionCandidate();

                    return [
                        {
                            delay: 0,
                            payload: {
                                t: 'colecao.mover',
                                pokemonId:
                                    pokemon.id,
                                para: 'colecao'
                            }
                        },
                        {
                            delay: 4200,
                            payload: {
                                t: 'colecao.mover',
                                pokemonId:
                                    pokemon.id,
                                para: 'depot'
                            }
                        }
                    ];
                }
        },
        {
            id: 'outland-tier-roundtrip',
            group: 'Useful shortcuts · situational',
            label: 'Change Outland tier mid-hunt, then restore',
            risk: 'medium',
            description:
                'While actively hunting in Outland, switch to another tier unlocked by your trainer level, then restore the original tier.',
            expectsRoundTrip: true,
            successOnStateChange: true,
            responseWindowMs: 4200,
            available:
                () => ({
                    ok:
                        Boolean(
                            alternativeOutlandTier()
                        ),
                    reason:
                        'requires an Outland hunt and at least two unlocked tiers'
                }),
            steps:
                () => {
                    const original =
                        Number(
                            runtime.gameState.outlandTier ||
                            1
                        );

                    const target =
                        alternativeOutlandTier();

                    return [
                        {
                            delay: 0,
                            payload: {
                                t: 'outland.tier',
                                tier:
                                    Number(target.tier)
                            }
                        },
                        {
                            delay: 1400,
                            payload: {
                                t: 'outland.tier',
                                tier:
                                    original
                            }
                        }
                    ];
                }
        }
    ];

    function finishProbe(reason) {
        const probe =
            runtime.activeProbe;

        if (!probe) return;

        if (runtime.probeTimer) {
            clearTimeout(
                runtime.probeTimer
            );

            runtime.probeTimer =
                null;
        }

        probe.finishedAt =
            Date.now();

        probe.after =
            summary();

        probe.changes =
            diffRelevantSummary(
                probe,
                probe.before,
                probe.after
            );

        const accepted =
            probe.events.some(
                event =>
                    event.kind ===
                    'accepted'
            );

        const error =
            probe.events.some(
                event =>
                    event.kind ===
                    'error'
            );

        const notice =
            probe.events.some(
                event =>
                    event.kind ===
                    'notice'
            );

        const stateChanged =
            probe.events.some(
                event =>
                    event.kind ===
                    'state'
            );

        if (
            probe.expectsRoundTrip &&
            stateChanged
        ) {
            probe.result =
                probe.changes.length
                    ? 'state changed; restore may have failed'
                    : 'state changed and restored';
        } else if (
            probe.expectsRejection
        ) {
            if (
                error ||
                notice
            ) {
                probe.result =
                    'rejected as expected';
            } else if (
                accepted ||
                probe.changes.length
            ) {
                probe.result =
                    'unexpected acceptance / state change';
            } else {
                probe.result =
                    reason ||
                    'no explicit rejection';
            }
        } else if (error) {
            probe.result =
                'server error';
        } else if (notice) {
            probe.result =
                'server notice / likely rejection';
        } else if (
            accepted ||
            (
                probe.successOnStateChange &&
                stateChanged
            )
        ) {
            probe.result =
                accepted
                    ? 'expected response received'
                    : 'relevant state changed';
        } else if (
            probe.changes.length
        ) {
            probe.result =
                'relevant state changed';
        } else {
            probe.result =
                reason ||
                'no relevant response';
        }

        runtime.history.unshift(
            probe
        );

        if (
            runtime.history.length >
            MAX_HISTORY
        ) {
            runtime.history.splice(
                MAX_HISTORY
            );
        }

        runtime.activeProbe =
            null;

        updateUI();
    }

    function runAction(action) {
        const availability =
            actionAvailability(
                action
            );

        if (!availability.ok) {
            return;
        }

        if (
            runtime.activeProbe
        ) {
            finishProbe(
                'interrupted by next probe'
            );
        }

        const now =
            Date.now();

        if (
            now -
                runtime.lastActionAt <
            CLICK_COOLDOWN_MS
        ) {
            return;
        }

        runtime.lastActionAt =
            now;

        let steps;

        try {
            steps =
                action.steps();
        } catch (error) {
            runtime.history.unshift({
                label:
                    action.label,
                result:
                    'build failed: ' +
                    String(
                        error &&
                        error.message ||
                        error
                    ),
                startedAt:
                    now,
                events: [],
                changes: []
            });

            updateUI();
            return;
        }

        runtime.activeProbe = {
            id:
                action.id,
            label:
                action.label,
            risk:
                action.risk,
            startedAt:
                now,
            before:
                summary(),
            lastSummary:
                summary(),
            outgoing: [],
            events: [],
            changes: [],
            result:
                'waiting',
            expect:
                Array.isArray(
                    action.expect
                )
                    ? [...action.expect]
                    : [],
            expectsRejection:
                Boolean(
                    action.expectsRejection
                ),
            expectsRoundTrip:
                Boolean(
                    action.expectsRoundTrip
                ),
            successOnStateChange:
                Boolean(
                    action.successOnStateChange
                )
        };

        try {
            for (const step of steps) {
                const delay =
                    Math.max(
                        0,
                        Number(
                            step.delay ||
                            0
                        )
                    );

                setTimeout(
                    () => {
                        if (
                            !runtime.activeProbe ||
                            runtime.activeProbe.id !==
                                action.id
                        ) {
                            return;
                        }

                        try {
                            send(
                                step.payload
                            );

                            runtime.activeProbe
                                .outgoing
                                .push({
                                    at:
                                        Date.now() -
                                        runtime.activeProbe
                                            .startedAt,
                                    payload:
                                        clone(
                                            step.payload
                                        )
                                });

                            updateUI();
                        } catch (error) {
                            addProbeEvent(
                                'send failed: ' +
                                String(
                                    error &&
                                    error.message ||
                                    error
                                ),
                                'error'
                            );
                        }
                    },
                    delay
                );
            }
        } catch (error) {
            addProbeEvent(
                'probe failed: ' +
                String(
                    error &&
                    error.message ||
                    error
                ),
                'error'
            );
        }

        runtime.probeTimer =
            setTimeout(
                () =>
                    finishProbe(
                        'no explicit response in window'
                    ),
                Math.max(
                    RESPONSE_WINDOW_MS,
                    Number(
                        action.responseWindowMs ||
                        0
                    )
                )
            );

        updateUI();
    }

    function injectStyle() {
        if (q('#' + STYLE_ID)) {
            return;
        }

        const style =
            document.createElement(
                'style'
            );

        style.id =
            STYLE_ID;

        style.textContent = [
            '#' + BUTTON_ID + '{',
            'display:inline-flex!important;',
            'align-items:center!important;',
            'justify-content:center!important;',
            'width:auto!important;',
            'height:30px!important;',
            'padding:5px 8px!important;',
            'border:1px solid rgba(255,255,255,.17)!important;',
            'border-radius:6px!important;',
            'background:rgba(74,38,48,.9)!important;',
            'color:#eee!important;',
            'font:700 10px/1 system-ui,sans-serif!important;',
            'cursor:pointer!important;',
            '}',
            '#' + ROOT_ID + '{',
            'position:fixed!important;',
            'right:405px!important;',
            'bottom:12px!important;',
            'z-index:2147483646!important;',
            'width:min(440px,calc(100vw - 24px))!important;',
            'max-height:82vh!important;',
            'overflow:auto!important;',
            'padding:10px!important;',
            'border:1px solid rgba(255,255,255,.18)!important;',
            'border-radius:9px!important;',
            'background:#171316!important;',
            'color:#eee7ea!important;',
            'box-shadow:0 10px 35px rgba(0,0,0,.5)!important;',
            'font:11px/1.35 system-ui,sans-serif!important;',
            '}',
            '#' + ROOT_ID + '[hidden]{display:none!important;}',
            '#' + ROOT_ID + ' *{box-sizing:border-box!important;}',
            '.mea-head{display:flex;align-items:center;gap:7px;margin-bottom:7px;}',
            '.mea-head strong{flex:1;font-size:12px;}',
            '.mea-head button,.mea-clear,.mea-copy{border:1px solid rgba(255,255,255,.14);border-radius:5px;background:rgba(255,255,255,.06);color:#eee;padding:5px 7px;font:inherit;cursor:pointer;}',
            '.mea-state{padding:7px;border-radius:6px;background:rgba(255,255,255,.045);color:#cfc4c9;margin-bottom:8px;}',
            '.mea-state b{color:#fff;}',
            '.mea-group{margin-top:9px;}',
            '.mea-group-title{margin-bottom:4px;color:#a99fa3;font-size:9px;text-transform:uppercase;letter-spacing:.08em;}',
            '.mea-action{display:grid;grid-template-columns:1fr auto;gap:6px;align-items:start;padding:6px 0;border-top:1px solid rgba(255,255,255,.06);}',
            '.mea-action:first-of-type{border-top:0;}',
            '.mea-action b{display:block;font-size:10px;margin-bottom:1px;}',
            '.mea-action small{display:block;color:#92878c;font-size:9px;}',
            '.mea-action button{min-width:58px;border:1px solid rgba(255,255,255,.15);border-radius:5px;padding:5px 7px;background:#273a2e;color:#f2fff5;font:700 9px/1 system-ui,sans-serif;cursor:pointer;}',
            '.mea-action button[data-risk="medium"]{background:#594619;}',
            '.mea-action button[data-risk="high"]{background:#602d33;}',
            '.mea-action button:disabled{opacity:.38;cursor:default;}',
            '.mea-live{margin-top:8px;padding:7px;border-radius:6px;background:rgba(255,255,255,.045);}',
            '.mea-live strong{display:block;margin-bottom:4px;}',
            '.mea-payload{font:9px/1.25 ui-monospace,SFMono-Regular,Consolas,monospace;color:#b9aeb3;overflow-wrap:anywhere;}',
            '.mea-log-head{display:flex;align-items:center;gap:6px;margin-top:10px;margin-bottom:4px;}',
            '.mea-log-head b{flex:1;}',
            '.mea-history{display:grid;gap:5px;}',
            '.mea-entry{padding:6px;border-radius:5px;background:rgba(255,255,255,.04);}',
            '.mea-entry-top{display:flex;justify-content:space-between;gap:8px;}',
            '.mea-entry-top span{color:#a89da2;font-size:9px;}',
            '.mea-entry-result{margin-top:2px;font-weight:700;}',
            '.mea-entry-detail{margin-top:3px;color:#968b90;font-size:9px;overflow-wrap:anywhere;}',
            '@media(max-width:900px){#' + ROOT_ID + '{right:12px!important;}}'
        ].join('');

        (
            document.head ||
            document.documentElement
        ).appendChild(
            style
        );
    }

    function ensureButton() {
        injectStyle();

        let button =
            q('#' + BUTTON_ID);

        if (!button) {
            button =
                document.createElement(
                    'button'
                );

            button.type =
                'button';

            button.id =
                BUTTON_ID;

            button.textContent =
                'Experiments';

            button.title =
                'PokéIdle experimental one-shot protocol actions';

            button.addEventListener(
                'click',
                () => {
                    const panel =
                        ensurePanel();

                    panel.hidden =
                        !panel.hidden;

                    updateUI();
                }
            );
        }

        const scrape =
            q(
                '#' +
                SCRAPE_BUTTON_ID
            );

        const host =
            scrape &&
            scrape.parentElement
                ? scrape.parentElement
                : q('.menu-topo') ||
                  document.body;

        if (!host) return;

        if (scrape) {
            if (
                button.parentElement !==
                    host ||
                button.previousElementSibling !==
                    scrape
            ) {
                scrape.insertAdjacentElement(
                    'afterend',
                    button
                );
            }
        } else if (
            button.parentElement !==
            host
        ) {
            host.appendChild(
                button
            );
        }
    }

    function groupedActions() {
        const groups =
            new Map();

        for (const action of actions) {
            if (!groups.has(action.group)) {
                groups.set(
                    action.group,
                    []
                );
            }

            groups.get(
                action.group
            ).push(
                action
            );
        }

        return groups;
    }

    function panelMarkup() {
        let html = '';

        html +=
            '<div class="mea-head">' +
            '<strong>Experimental Actions</strong>' +
            '<button type="button" data-mea-close>×</button>' +
            '</div>';

        html +=
            '<div class="mea-state" data-mea-state></div>' +
            '<div class="mea-state">Prioritized for practical use: reversible hunt/team/protection shortcuts first. Tests auto-restore their starting state and avoid scarce-resource, evolution/refinement, premium, trade, destructive Pokémon, and held Exp. Share race paths.</div>';

        for (
            const [
                group,
                groupActions
            ] of
            groupedActions()
        ) {
            html +=
                '<section class="mea-group">' +
                '<div class="mea-group-title">' +
                escapeHtml(group) +
                '</div>';

            for (const action of groupActions) {
                html +=
                    '<div class="mea-action" data-mea-action="' +
                    escapeHtml(action.id) +
                    '">' +
                    '<div><b>' +
                    escapeHtml(action.label) +
                    '</b><small>' +
                    escapeHtml(action.description) +
                    '</small><small data-mea-reason></small></div>' +
                    '<button type="button" data-mea-run="' +
                    escapeHtml(action.id) +
                    '" data-risk="' +
                    escapeHtml(action.risk) +
                    '">Test</button>' +
                    '</div>';
            }

            html +=
                '</section>';
        }

        html +=
            '<div class="mea-live" data-mea-live hidden></div>';

        html +=
            '<div class="mea-log-head">' +
            '<b>Probe history</b>' +
            '<button type="button" class="mea-copy" data-mea-copy>Copy last</button>' +
            '<button type="button" class="mea-clear" data-mea-clear>Clear</button>' +
            '</div>' +
            '<div class="mea-history" data-mea-history></div>';

        return html;
    }

    function ensurePanel() {
        injectStyle();

        let root =
            q('#' + ROOT_ID);

        if (root) {
            return root;
        }

        root =
            document.createElement(
                'section'
            );

        root.id =
            ROOT_ID;

        root.hidden =
            true;

        root.innerHTML =
            panelMarkup();

        document.body.appendChild(
            root
        );

        q(
            '[data-mea-close]',
            root
        ).addEventListener(
            'click',
            () => {
                root.hidden =
                    true;
            }
        );

        for (
            const button of
            qa(
                '[data-mea-run]',
                root
            )
        ) {
            button.addEventListener(
                'click',
                () => {
                    const action =
                        actions.find(
                            item =>
                                item.id ===
                                button.dataset
                                    .meaRun
                        );

                    if (action) {
                        runAction(
                            action
                        );
                    }
                }
            );
        }

        q(
            '[data-mea-clear]',
            root
        ).addEventListener(
            'click',
            () => {
                runtime.history = [];
                updateUI();
            }
        );

        q(
            '[data-mea-copy]',
            root
        ).addEventListener(
            'click',
            async () => {
                const item =
                    runtime.history[0] ||
                    runtime.activeProbe;

                if (!item) return;

                const text =
                    JSON.stringify(
                        item,
                        null,
                        2
                    );

                try {
                    await navigator.clipboard.writeText(
                        text
                    );
                } catch {}
            }
        );

        return root;
    }

    function renderHistory(root) {
        const host =
            q(
                '[data-mea-history]',
                root
            );

        if (!host) return;

        host.innerHTML =
            runtime.history
                .slice(0, 12)
                .map(
                    item => {
                        const outgoing =
                            (item.outgoing || [])
                                .map(
                                    row =>
                                        compactPayload(
                                            row.payload
                                        )
                                )
                                .join(' · ');

                        const events =
                            (item.events || [])
                                .map(
                                    row =>
                                        row.text
                                )
                                .join(' · ');

                        const changes =
                            (item.changes || [])
                                .join(' · ');

                        return (
                            '<div class="mea-entry">' +
                            '<div class="mea-entry-top"><b>' +
                            escapeHtml(item.label || item.id || 'Probe') +
                            '</b><span>' +
                            escapeHtml(
                                new Date(
                                    item.startedAt ||
                                    Date.now()
                                ).toLocaleTimeString()
                            ) +
                            '</span></div>' +
                            '<div class="mea-entry-result">' +
                            escapeHtml(item.result || '') +
                            '</div>' +
                            (
                                outgoing
                                    ? '<div class="mea-entry-detail">out: ' +
                                      escapeHtml(outgoing) +
                                      '</div>'
                                    : ''
                            ) +
                            (
                                events
                                    ? '<div class="mea-entry-detail">in: ' +
                                      escapeHtml(events) +
                                      '</div>'
                                    : ''
                            ) +
                            (
                                changes
                                    ? '<div class="mea-entry-detail">Δ ' +
                                      escapeHtml(changes) +
                                      '</div>'
                                    : ''
                            ) +
                            '</div>'
                        );
                    }
                )
                .join('');
    }

    function updateUI() {
        ensureButton();

        const root =
            q('#' + ROOT_ID);

        if (!root) return;

        const stateHost =
            q(
                '[data-mea-state]',
                root
            );

        const current =
            summary();

        stateHost.innerHTML =
            '<b>' +
            escapeHtml(
                socketReady()
                    ? 'Protocol live'
                    : runtime.hookInstalled
                        ? 'Waiting for socket'
                        : 'Protocol hook failed'
            ) +
            '</b> · ' +
            escapeHtml(
                current.scene
            ) +
            ' · active ' +
            escapeHtml(
                current.activeId == null
                    ? '—'
                    : current.activeId
            ) +
            ' · gold ' +
            escapeHtml(
                current.gold == null
                    ? '—'
                    : current.gold
            ) +
            ' · 💎 ' +
            escapeHtml(
                current.diamonds == null
                    ? '—'
                    : current.diamonds
            );

        for (const action of actions) {
            const row =
                q(
                    '[data-mea-action="' +
                    action.id +
                    '"]',
                    root
                );

            if (!row) continue;

            const availability =
                actionAvailability(
                    action
                );

            const button =
                q(
                    '[data-mea-run]',
                    row
                );

            const reason =
                q(
                    '[data-mea-reason]',
                    row
                );

            button.disabled =
                !availability.ok ||
                Boolean(
                    runtime.activeProbe
                );

            reason.textContent =
                availability.ok
                    ? ''
                    : availability.reason ||
                      'not available';
        }

        const live =
            q(
                '[data-mea-live]',
                root
            );

        if (
            runtime.activeProbe
        ) {
            const probe =
                runtime.activeProbe;

            live.hidden =
                false;

            const outgoing =
                probe.outgoing
                    .map(
                        row =>
                            compactPayload(
                                row.payload
                            )
                    )
                    .join(' · ');

            const incoming =
                probe.events
                    .map(
                        row =>
                            row.text
                    )
                    .join(' · ');

            live.innerHTML =
                '<strong>Testing: ' +
                escapeHtml(
                    probe.label
                ) +
                '</strong>' +
                '<div class="mea-payload">out: ' +
                escapeHtml(
                    outgoing ||
                    'scheduled'
                ) +
                '</div>' +
                '<div class="mea-payload">in: ' +
                escapeHtml(
                    incoming ||
                    'waiting…'
                ) +
                '</div>';
        } else {
            live.hidden =
                true;

            live.innerHTML =
                '';
        }

        renderHistory(
            root
        );
    }

    function bootstrap() {
        installProtocolHook();
        ensureButton();
        ensurePanel();
        updateUI();

        setInterval(
            () => {
                ensureButton();
                updateUI();
            },
            1000
        );

        console.info(
            '[PokéIdle Experimental Actions] v1.1.0 loaded'
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
