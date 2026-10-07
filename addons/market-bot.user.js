// ==UserScript==
// @name         PokéIdle Moth Watch
// @namespace    moth.pokeidle
// @version      0.1.12
// @description  Community Market watchlist and configurable underprice sniper using completed-sale references.
// @match        https://pokeidle.io/app*
// @grant        unsafeWindow
// @updateURL    https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/market-bot.user.js
// @downloadURL  https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/market-bot.user.js
// @run-at       document-start
// ==/UserScript==

(() => {
    'use strict';

    const page = typeof unsafeWindow !== 'undefined' ? unsafeWindow : window;

    const CONFIG_KEY = 'moth-pokeidle-market-bot-config-v1';
    const BASELINE_KEY = 'moth-pokeidle-market-item-baseline-v1';
    const POKEMON_REFERENCE_KEY = 'moth-pokeidle-market-pokemon-reference-v1';
    const BACKGROUND_MIN_GAP_MS = 3000;
    const RATE_LIMIT_BACKOFF_MS = 15000;
    const ITEM_MARKET_CAP_TTL_MS = 10 * 60 * 1000;

    /*
     * Source-ready price table.
     *
     * This intentionally starts empty. PokéIdle does not ship player-market
     * averages in its static client; the authoritative averages arrive only
     * on an authenticated market.itens response. On the first logged-in run,
     * Moth Watch freezes those server-calculated 7-day weighted averages into
     * BASELINE_KEY. "Export baseline JSON" emits entries in exactly this
     * shape so a verified snapshot can later be pasted here without guessing.
     *
     * Shape:
     *   itemId: {
     *     name: 'Water Stone',
     *     gold: { average: 30000, units: 27 } | null,
     *     orb:  { average: 42, units: 8 } | null
     *   }
     */
    const HARD_CODED_ITEM_AVERAGES = Object.freeze({});

    const DEFAULTS = Object.freeze({
        enabled: true,
        scanSeconds: 5,
        watchPercent: 70,
        autoBuy: false,
        autoBuyPercent: 40,
        scanItems: true,
        scanPokemon: true,
        autoBuyItems: true,
        autoBuyPokemon: true,
        buyCoins: true,
        buyGems: true,
        buyWholeItemBatch: true,
        goldReserve: 0,
        gemReserve: 0,
        maxCoinsPerBuy: 0,
        maxGemsPerBuy: 0,
        minItemUnits: 5,
        minPokemonSamples: 3,
        pokemonHistoryPages: 12,
        pokemonHistoryRefreshMinutes: 60,
        viewCurrency: 'all',
        viewKind: 'all',
        viewSort: 'discount'
    });

    const state = {
        socket: null,
        hookInstalled: false,
        nick: '',
        gold: 0,
        orbs: 0,
        catalog: new Map(),
        itemSummary: {},
        liveItemAverages: {},
        itemMarketCaps: new Map(),
        candidates: new Map(),
        detailQueue: [],
        detailQueued: new Set(),
        lastDetailRequest: new Map(),
        pendingBuy: null,
        attemptedListings: new Map(),
        buyLog: [],
        lastScanAt: 0,
        scanTimer: null,
        watchOpen: false,
        renderQueued: false,
        baseline: loadBaseline(),
        forceBaselineNext: false,
        pokemonReference: loadPokemonReference(),
        historyFetch: null,
        pokemonScan: null,
        protocolMessages: 0,
        itemStatus: 'waiting',
        pokemonStatus: 'waiting',
        serverClockOffset: 0,
        lastBotSendAt: 0,
        rateLimitedUntil: 0,
        itemSummaryRetryTimer: null,
        detailPumpTimer: null,
        itemScanStats: {
            goldChecked: 0,
            orbChecked: 0,
            goldSuspicious: 0,
            orbSuspicious: 0
        },
        releaseTimers: new Map()
    };

    let config = loadConfig();

    function q(selector, root) {
        return (root || document).querySelector(selector);
    }

    function qa(selector, root) {
        return Array.from((root || document).querySelectorAll(selector));
    }

    function normalize(value) {
        return String(value == null ? '' : value)
            .normalize('NFD')
            .replace(/[\u0300-\u036f]/g, '')
            .toLowerCase()
            .replace(/\s+/g, ' ')
            .trim();
    }

    function escapeHtml(value) {
        return String(value == null ? '' : value)
            .replaceAll('&', '&amp;')
            .replaceAll('<', '&lt;')
            .replaceAll('>', '&gt;')
            .replaceAll('"', '&quot;')
            .replaceAll("'", '&#039;');
    }

    function num(value) {
        return Number(value || 0).toLocaleString();
    }

    function pct(value) {
        if (!Number.isFinite(value)) return '—';
        return (value * 100).toFixed(value < 0.1 ? 1 : 0) + '%';
    }

    function money(value, currency) {
        const src = currency === 'orb'
            ? '/img/moeda-gema.png'
            : '/assets/site/assets/ui/moeda-ouro.png';
        return '<span class="moth-mw-money"><img src="' + src + '" alt="">' + num(value) + '</span>';
    }

    function loadConfig() {
        try {
            const saved = JSON.parse(localStorage.getItem(CONFIG_KEY) || '{}');
            return Object.assign({}, DEFAULTS, saved || {});
        } catch {
            return Object.assign({}, DEFAULTS);
        }
    }

    function saveConfig() {
        localStorage.setItem(CONFIG_KEY, JSON.stringify(config));
    }

    function emptyBaseline() {
        return { version: 1, capturedAt: 0, source: 'none', items: {} };
    }

    function loadBaseline() {
        let saved = null;
        try {
            saved = JSON.parse(localStorage.getItem(BASELINE_KEY) || 'null');
        } catch {}

        if (saved && saved.items && typeof saved.items === 'object') {
            return saved;
        }

        const sourceEntries = Object.entries(HARD_CODED_ITEM_AVERAGES);
        if (sourceEntries.length) {
            return {
                version: 1,
                capturedAt: 0,
                source: 'hardcoded',
                items: Object.fromEntries(sourceEntries)
            };
        }

        return emptyBaseline();
    }

    function saveBaseline() {
        try {
            localStorage.setItem(BASELINE_KEY, JSON.stringify(state.baseline));
        } catch {}
    }

    function loadPokemonReference() {
        try {
            const raw = JSON.parse(localStorage.getItem(POKEMON_REFERENCE_KEY) || 'null');
            if (raw && raw.values && typeof raw.values === 'object') return raw;
        } catch {}
        return { version: 1, capturedAt: 0, values: {} };
    }

    function savePokemonReference() {
        try {
            localStorage.setItem(POKEMON_REFERENCE_KEY, JSON.stringify(state.pokemonReference));
        } catch {}
    }

    function backgroundDelayMs() {
        const now = Date.now();
        return Math.max(
            0,
            Number(state.rateLimitedUntil || 0) - now,
            Number(state.lastBotSendAt || 0) + BACKGROUND_MIN_GAP_MS - now
        );
    }

    function send(payload) {
        const socket = state.socket;
        if (!socket || socket.readyState !== 1) return false;
        try {
            socket.send(JSON.stringify(payload));
            state.lastBotSendAt = Date.now();
            return true;
        } catch {
            return false;
        }
    }

    function sendBackground(payload) {
        if (backgroundDelayMs() > 0) return false;
        return send(payload);
    }

    function isRateLimitError(message) {
        const text = normalize([
            message && message.chave,
            message && message.msg,
            message && message.message
        ].filter(Boolean).join(' '));

        return (
            text.includes('muitas mensagens') ||
            text.includes('too many messages') ||
            text.includes('rate limit')
        );
    }

    function enterRateLimitBackoff() {
        state.rateLimitedUntil = Math.max(
            Number(state.rateLimitedUntil || 0),
            Date.now() + RATE_LIMIT_BACKOFF_MS
        );
        state.itemStatus = 'rate-limit backoff';
        state.pokemonStatus = 'rate-limit backoff';
        addLog('Server rate limit · background scans paused for 15s');
        queueRender();
    }

    function serverNow() {
        return Date.now() - Number(state.serverClockOffset || 0);
    }

    function mergePlayer(snapshot, full) {
        if (!snapshot || typeof snapshot !== 'object') return;

        const previousGold = state.gold;
        const previousOrbs = state.orbs;

        if (Number(snapshot.servidorAgora) > 0) {
            state.serverClockOffset = Date.now() - Number(snapshot.servidorAgora);
        }

        if (full || Object.prototype.hasOwnProperty.call(snapshot, 'nick')) {
            if (snapshot.nick) state.nick = String(snapshot.nick);
        }
        if (full || Object.prototype.hasOwnProperty.call(snapshot, 'gold')) {
            if (Number.isFinite(Number(snapshot.gold))) state.gold = Number(snapshot.gold);
        }
        if (full || Object.prototype.hasOwnProperty.call(snapshot, 'orbs')) {
            if (Number.isFinite(Number(snapshot.orbs))) state.orbs = Number(snapshot.orbs);
        }

        if (state.pendingBuy && Date.now() - state.pendingBuy.sentAt > 4500) {
            finishPendingBuy('timeout');
        }

        if (state.watchOpen && (previousGold !== state.gold || previousOrbs !== state.orbs)) {
            queueRender();
        }
    }

    function catalogName(itemId) {
        const item = state.catalog.get(Number(itemId));
        return item && (item.nome || item.name)
            ? String(item.nome || item.name)
            : 'Item #' + itemId;
    }

    function freezeItemBaseline(medias, force) {
        if (!medias || typeof medias !== 'object') return false;
        if (!force && state.baseline && state.baseline.capturedAt) return false;

        const ids = new Set([
            ...Array.from(state.catalog.keys()).map(String),
            ...Object.keys(medias)
        ]);

        const items = {};
        for (const rawId of ids) {
            const id = Number(rawId);
            if (!Number.isFinite(id) || id <= 0) continue;
            const m = medias[id] || medias[String(id)] || null;
            const gold = m && m.gold && Number(m.gold.media) > 0
                ? { average: Number(m.gold.media), units: Number(m.gold.unidades || 0) }
                : null;
            const orb = m && m.orb && Number(m.orb.media) > 0
                ? { average: Number(m.orb.media), units: Number(m.orb.unidades || 0) }
                : null;

            items[id] = {
                name: catalogName(id),
                gold: gold,
                orb: orb
            };
        }

        state.baseline = {
            version: 1,
            capturedAt: Date.now(),
            source: 'server-7d-weighted-average',
            items: items
        };
        state.forceBaselineNext = false;
        saveBaseline();
        addLog('Baseline frozen: ' + Object.keys(items).length + ' market items');
        queueRender();
        return true;
    }

    function supplementItemBaseline(medias) {
        if (!state.baseline?.capturedAt || !medias || typeof medias !== 'object') return false;

        let added = 0;
        const items = state.baseline.items || (state.baseline.items = {});

        for (const [rawId, m] of Object.entries(medias)) {
            const id = Number(rawId);
            if (!Number.isFinite(id) || id <= 0 || !m) continue;

            const row = items[id] || items[String(id)] || {
                name: catalogName(id),
                gold: null,
                orb: null
            };

            for (const currency of ['gold', 'orb']) {
                if (row[currency] && Number(row[currency].average) > 0) continue;
                const src = m[currency];
                if (!src || !(Number(src.media) > 0)) continue;
                row[currency] = {
                    average: Number(src.media),
                    units: Number(src.unidades || 0)
                };
                added++;
            }

            items[id] = row;
        }

        if (!added) return false;

        saveBaseline();
        addLog('Baseline expanded with ' + added + ' missing Coin/Gem reference' + (added === 1 ? '' : 's'));
        return true;
    }

    function itemReference(itemId, currency) {
        /*
         * The server's 7-day weighted mean is useful, but a single extreme
         * trade can distort a mean. Once we have at least three current
         * listings for this item/currency, cap that historical reference at
         * the median active unit price. This is deliberately conservative:
         * Moth Watch may miss a deal, but it will not call a normal 70k
         * market a 99% discount because one historical mean says 6M.
         */
        let historical = null;

        const liveRow = itemMediaFor(itemId);
        const live = liveRow ? liveRow[currency] : null;
        if (
            live &&
            Number.isFinite(Number(live.media)) &&
            Number(live.media) > 0
        ) {
            historical = {
                average: Number(live.media),
                units: Number(live.unidades || 0),
                name: catalogName(itemId),
                source: 'live-7d'
            };
        }

        if (!historical) {
            const row = state.baseline && state.baseline.items
                ? state.baseline.items[itemId] || state.baseline.items[String(itemId)]
                : null;
            const ref = row ? row[currency] : null;
            if (ref && Number.isFinite(Number(ref.average)) && Number(ref.average) > 0) {
                historical = {
                    average: Number(ref.average),
                    units: Number(ref.units || 0),
                    name: row.name || catalogName(itemId),
                    source: 'frozen-fallback'
                };
            }
        }

        if (!historical) return null;

        const capKey = String(itemId) + ':' + currency;
        const cap = state.itemMarketCaps.get(capKey);
        if (
            cap &&
            Date.now() - Number(cap.updatedAt || 0) <= ITEM_MARKET_CAP_TTL_MS &&
            Number(cap.count) >= 3 &&
            Number.isFinite(Number(cap.median)) &&
            Number(cap.median) > 0 &&
            Number(cap.median) < historical.average
        ) {
            return {
                average: Number(cap.median),
                units: historical.units,
                name: historical.name,
                source: 'active-median',
                activeListings: Number(cap.count),
                serverAverage: historical.average
            };
        }

        return historical;
    }

    function queueItemDetail(itemId, currency) {
        const key = String(itemId) + ':' + currency;
        const now = Date.now();
        if (state.detailQueued.has(key)) return;
        if (now - Number(state.lastDetailRequest.get(key) || 0) < Math.max(1000, config.scanSeconds * 800)) return;
        state.detailQueued.add(key);
        state.detailQueue.push({ itemId: Number(itemId), currency: currency, key: key });
        scheduleItemDetailPump(0);
    }

    function scheduleItemDetailPump(delay) {
        if (state.detailPumpTimer) return;
        state.detailPumpTimer = setTimeout(() => {
            state.detailPumpTimer = null;
            pumpItemDetails();
        }, Math.max(0, Number(delay) || 0));
    }

    function pumpItemDetails() {
        if (!state.detailQueue.length) return;
        if (!state.socket || state.socket.readyState !== 1) return;

        const delay = backgroundDelayMs();
        if (delay > 0) {
            scheduleItemDetailPump(delay + 25);
            return;
        }

        const job = state.detailQueue[0];
        if (!sendBackground({ t: 'market.item', itemId: job.itemId, moeda: job.currency })) {
            scheduleItemDetailPump(BACKGROUND_MIN_GAP_MS);
            return;
        }

        state.detailQueue.shift();
        state.detailQueued.delete(job.key);
        state.lastDetailRequest.set(job.key, Date.now());

        if (state.detailQueue.length) {
            scheduleItemDetailPump(BACKGROUND_MIN_GAP_MS);
        }
    }

    function discoveryRatio() {
        const watch = Math.max(0.01, Number(config.watchPercent) / 100);
        const auto = config.autoBuy
            ? Math.max(0.01, Number(config.autoBuyPercent) / 100)
            : 0;
        return Math.max(watch, auto);
    }

    function scanSuspiciousItemSummaries() {
        const summary = state.itemSummary || {};
        const scanRatio = discoveryRatio();
        const stats = {
            goldChecked: 0,
            orbChecked: 0,
            goldSuspicious: 0,
            orbSuspicious: 0
        };
        const ids = new Set([
            ...Object.keys(state.liveItemAverages || {}),
            ...Object.keys(state.baseline.items || {})
        ]);

        for (const rawId of ids) {
            const id = Number(rawId);
            if (!Number.isFinite(id) || id <= 0) continue;

            const r = summary[id] || summary[rawId];
            if (!r || !r.anuncios) continue;

            for (const currency of ['gold', 'orb']) {
                const ref = itemReference(id, currency);
                if (!ref || !(Number(ref.average) > 0)) continue;

                const checkedKey = currency === 'orb' ? 'orbChecked' : 'goldChecked';
                const suspiciousKey = currency === 'orb' ? 'orbSuspicious' : 'goldSuspicious';
                stats[checkedKey]++;

                const min = currency === 'orb' ? Number(r.minOrb) : Number(r.minGold);
                if (!Number.isFinite(min) || min <= 0) continue;
                if (min <= Number(ref.average) * scanRatio) {
                    stats[suspiciousKey]++;
                    queueItemDetail(id, currency);
                }
            }
        }

        state.itemScanStats = stats;
    }

    function removeCandidateGroup(prefix) {
        const now = serverNow();
        for (const [key, candidate] of Array.from(state.candidates.entries())) {
            if (!key.startsWith(prefix)) continue;
            /*
             * A recent-list page can move a new listing off page 0 long
             * before its market retention expires. Keep it locally until
             * unlock; the purchase request itself is still validated by the
             * server if the seller cancels or somebody else wins it.
             */
            if (candidate.retainedUntil && candidate.retainedUntil > now) continue;
            state.candidates.delete(key);
        }
    }

    function retentionUntil(listing) {
        const t = Number(listing && listing.compravelEm);
        return Number.isFinite(t) && t > serverNow() ? t : 0;
    }

    function scheduleCandidateRelease(candidate) {
        if (!candidate || !candidate.retainedUntil) return;
        if (candidate.retainedUntil <= serverNow()) return;

        const existing = state.releaseTimers.get(candidate.listingId);
        if (existing && existing.at === candidate.retainedUntil) return;
        if (existing) clearTimeout(existing.timer);

        const delay = Math.max(
            0,
            Math.min(
                2_147_000_000,
                candidate.retainedUntil - serverNow() + 500
            )
        );

        const timer = setTimeout(() => {
            state.releaseTimers.delete(candidate.listingId);
            const current = state.candidates.get(candidate.key);
            if (!current) return;

            /*
             * Adding stock to a live listing restarts retention upstream.
             * A fresher scan may therefore have moved the timestamp after
             * this timer was created. Never erase that newer hold.
             */
            if (current.retainedUntil > serverNow()) {
                scheduleCandidateRelease(current);
                queueRender();
                return;
            }

            current.retainedUntil = 0;
            maybeAutoBuy(current);
            queueRender();
        }, delay);

        state.releaseTimers.set(candidate.listingId, {
            timer: timer,
            at: candidate.retainedUntil
        });
    }

    function isOwnListing(listing) {
        return !!state.nick &&
            normalize(listing && listing.vendedor) === normalize(state.nick);
    }

    function addCandidate(candidate) {
        state.candidates.set(candidate.key, candidate);
        scheduleCandidateRelease(candidate);
        maybeAutoBuy(candidate);
        queueRender();
    }

    function handleItemListings(message) {
        const itemId = Number(message.itemId);
        const currency = message.moeda === 'orb' ? 'orb' : 'gold';
        const lines = Array.isArray(message.linhas) ? message.linhas : [];

        const activePrices = lines
            .map(listing => Number(listing && listing.preco))
            .filter(price => Number.isFinite(price) && price > 0);

        const capKey = String(itemId) + ':' + currency;
        if (activePrices.length >= 3) {
            state.itemMarketCaps.set(capKey, {
                median: median(activePrices),
                count: activePrices.length,
                updatedAt: Date.now()
            });
        } else {
            state.itemMarketCaps.delete(capKey);
        }

        const ref = itemReference(itemId, currency);
        if (!ref) return;

        const group = 'item:' + itemId + ':' + currency + ':';
        removeCandidateGroup(group);

        const scanRatio = discoveryRatio();
        for (const listing of lines) {
            if (!listing || isOwnListing(listing)) continue;
            const price = Number(listing.preco);
            const qty = Math.max(1, Number(listing.qtd || 1));
            if (!Number.isFinite(price) || price <= 0) continue;
            const ratio = price / ref.average;
            if (ratio > scanRatio) continue;

            addCandidate({
                key: group + listing.id,
                kind: 'item',
                listingId: Number(listing.id),
                itemId: itemId,
                name: ref.name || catalogName(itemId),
                currency: currency,
                price: price,
                average: ref.average,
                samples: ref.units,
                referenceSource: ref.source,
                activeReferenceListings: Number(ref.activeListings || 0),
                serverAverage: Number(ref.serverAverage || 0),
                ratio: ratio,
                qty: qty,
                seller: String(listing.vendedor || '—'),
                retainedUntil: retentionUntil(listing),
                firstSeenAt: Date.now(),
                seenAt: Date.now()
            });
        }
    }

    function itemMediaFor(id) {
        return state.liveItemAverages[id] || state.liveItemAverages[String(id)] || null;
    }

    function handleItemSummary(message) {
        state.itemSummary = message.resumo || {};
        state.liveItemAverages = message.medias || {};
        state.itemStatus = 'live';

        if (!state.baseline.capturedAt || state.forceBaselineNext) {
            freezeItemBaseline(state.liveItemAverages, true);
        } else {
            supplementItemBaseline(state.liveItemAverages);
        }

        scanSuspiciousItemSummaries();
        queueRender();
    }

    function levelBand(level) {
        const n = Number(level || 0);
        if (n < 100) return 'lt100';
        if (n < 500) return '100-499';
        if (n < 1000) return '500-999';
        if (n < 5000) return '1000-4999';
        if (n < 10000) return '5000-9999';
        if (n < 25000) return '10000-24999';
        return '25000+';
    }

    function speciesIdOf(ficha) {
        const id = Number(ficha && (ficha.speciesId != null ? ficha.speciesId : ficha.pokeId));
        return Number.isFinite(id) && id > 0 ? id : null;
    }

    function pokemonBucketKeys(ficha, currency) {
        const speciesId = speciesIdOf(ficha);
        if (!speciesId) return [];
        const shiny = ficha && ficha.shiny ? 1 : 0;
        const power = Math.max(1, Number(ficha && ficha.potencia || 1));
        const band = levelBand(ficha && ficha.level);
        const root = currency + '|s:' + speciesId + '|sh:' + shiny;

        return [
            root + '|p:' + power + '|lv:' + band,
            root + '|p:' + power
        ];
    }

    function median(values) {
        if (!values.length) return 0;
        const sorted = values.slice().sort((a, b) => a - b);
        const mid = Math.floor(sorted.length / 2);
        return sorted.length % 2
            ? sorted[mid]
            : (sorted[mid - 1] + sorted[mid]) / 2;
    }

    function percentile(values, p) {
        if (!values.length) return 0;
        const sorted = values.slice().sort((a, b) => a - b);
        const index = Math.min(sorted.length - 1, Math.max(0, Math.floor((sorted.length - 1) * p)));
        return sorted[index];
    }

    function addPokemonHistorySample(line, working) {
        if (!line || line.tipo !== 'pokemon' || !line.ficha) return;
        const currency = line.moeda === 'orb' ? 'orb' : 'gold';
        const price = Number(line.bruto != null ? line.bruto : line.preco);
        if (!Number.isFinite(price) || price <= 0) return;

        for (const key of pokemonBucketKeys(line.ficha, currency)) {
            let values = working.get(key);
            if (!values) {
                values = [];
                working.set(key, values);
            }
            if (values.length < 200) values.push(price);
        }
    }

    function finalizePokemonHistory(working) {
        const values = {};
        for (const [key, samples] of working) {
            if (!samples.length) continue;
            const sum = samples.reduce((a, b) => a + b, 0);
            values[key] = {
                count: samples.length,
                average: sum / samples.length,
                median: median(samples),
                lowQuartile: percentile(samples, 0.25),
                min: Math.min.apply(null, samples),
                max: Math.max.apply(null, samples)
            };
        }

        state.pokemonReference = {
            version: 1,
            capturedAt: Date.now(),
            values: values
        };
        savePokemonReference();
        state.pokemonStatus = 'history ready';
        addLog('Pokémon reference refreshed: ' + Object.keys(values).length + ' buckets');
        queueRender();
    }

    function pokemonReferenceFor(ficha, currency) {
        const minSamples = Math.max(1, Number(config.minPokemonSamples || 1));
        const values = state.pokemonReference.values || {};

        for (const key of pokemonBucketKeys(ficha, currency)) {
            const row = values[key];
            if (row && Number(row.count) >= minSamples && Number(row.median) > 0) {
                return {
                    value: Number(row.median),
                    average: Number(row.average || row.median),
                    samples: Number(row.count),
                    key: key
                };
            }
        }
        return null;
    }

    function requestHistoryPage() {
        const fetch = state.historyFetch;
        if (!fetch) return;
        if (q('#cm-hist-corpo')) {
            setTimeout(requestHistoryPage, 1500);
            return;
        }

        const delay = backgroundDelayMs();
        if (delay > 0) {
            setTimeout(requestHistoryPage, delay + 25);
            return;
        }

        const currency = fetch.currencies[fetch.currencyIndex];
        if (!sendBackground({
            t: 'market.historicoGlobal',
            pagina: fetch.page,
            tipo: 'pokemon',
            moeda: currency
        })) {
            setTimeout(requestHistoryPage, BACKGROUND_MIN_GAP_MS);
            return;
        }
        fetch.waitingSince = Date.now();
    }

    function ensurePokemonHistory(force) {
        if (!config.scanPokemon) return;
        if (state.historyFetch) return;

        const age = Date.now() - Number(state.pokemonReference.capturedAt || 0);
        const maxAge = Math.max(5, Number(config.pokemonHistoryRefreshMinutes || 60)) * 60 * 1000;
        if (!force && state.pokemonReference.capturedAt && age < maxAge) {
            state.pokemonStatus = 'history cached';
            return;
        }

        state.historyFetch = {
            currencies: ['gold', 'orb'],
            currencyIndex: 0,
            page: 0,
            working: new Map(),
            waitingSince: 0
        };
        state.pokemonStatus = 'loading history';
        requestHistoryPage();
        queueRender();
    }

    function handlePokemonHistory(message) {
        const fetch = state.historyFetch;
        if (!fetch || message.aba !== 'historicoGlobal') return false;

        const expectedCurrency = fetch.currencies[fetch.currencyIndex];
        const lines = Array.isArray(message.linhas) ? message.linhas : [];
        const foreign = lines.some(line =>
            line && line.tipo && line.tipo !== 'pokemon' ||
            line && line.moeda && line.moeda !== expectedCurrency
        );
        if (foreign) return false;

        for (const line of lines) addPokemonHistorySample(line, fetch.working);

        const maxPages = Math.max(1, Math.min(50, Number(config.pokemonHistoryPages || 12)));
        if (message.temMais && fetch.page + 1 < maxPages) {
            fetch.page++;
            setTimeout(requestHistoryPage, 100);
            return true;
        }

        if (fetch.currencyIndex + 1 < fetch.currencies.length) {
            fetch.currencyIndex++;
            fetch.page = 0;
            setTimeout(requestHistoryPage, 100);
            return true;
        }

        const working = fetch.working;
        state.historyFetch = null;
        finalizePokemonHistory(working);
        return true;
    }

    function recentPokemonRequest(currency) {
        return {
            t: 'market.listar',
            tipo: 'pokemon',
            moeda: currency,
            busca: '',
            ordem: 'recentes',
            criterios: [],
            elemento: '',
            categoria: '',
            soShiny: false,
            soP5: false,
            soTmElemental: false,
            soTmAoe: false,
            semOutland: false,
            nivelMin: '',
            nivelMax: '',
            potenciaMin: '',
            potenciaMax: '',
            ivMin: '',
            ivMax: '',
            qualidadeMin: '',
            qualidadeMax: '',
            notaMin: '',
            notaMax: '',
            pagina: 0,
            especieId: 0
        };
    }

    function startPokemonScan() {
        if (!config.scanPokemon || state.pokemonScan) return;

        /*
         * Bot requests share the game's WebSocket. A background market.listar
         * response is also seen by PokéIdle's own handler and can redraw the
         * normal Pokémon market. Moth Watch may scan while its own panel hides
         * that view, but otherwise leave interactive browsing alone.
         */
        if (q('.cm-topo') && !state.watchOpen) {
            state.pokemonStatus = 'paused while browsing';
            return;
        }

        if (!state.pokemonReference.capturedAt) {
            ensurePokemonHistory(false);
            return;
        }
        state.pokemonScan = {
            queue: ['gold', 'orb'],
            waiting: null,
            sentAt: 0
        };
        pumpPokemonScan();
    }

    function pumpPokemonScan() {
        const scan = state.pokemonScan;
        if (!scan || scan.waiting) return;
        const currency = scan.queue[0];
        if (!currency) {
            state.pokemonScan = null;
            state.pokemonStatus = 'watching';
            queueRender();
            return;
        }

        const delay = backgroundDelayMs();
        if (delay > 0) {
            setTimeout(pumpPokemonScan, delay + 25);
            return;
        }

        if (!sendBackground(recentPokemonRequest(currency))) {
            setTimeout(pumpPokemonScan, BACKGROUND_MIN_GAP_MS);
            return;
        }

        scan.queue.shift();
        scan.waiting = currency;
        scan.sentAt = Date.now();
        state.pokemonStatus = 'scanning ' + (currency === 'orb' ? 'gems' : 'coins');
        setTimeout(() => {
            if (state.pokemonScan === scan && scan.waiting === currency && Date.now() - scan.sentAt >= 2500) {
                scan.waiting = null;
                pumpPokemonScan();
            }
        }, 2600);
    }

    function handlePokemonListings(message) {
        const scan = state.pokemonScan;
        if (!scan || !scan.waiting || message.aba !== 'vitrine') return false;
        if (Number(message.pagina || 0) !== 0) return false;

        const currency = scan.waiting;
        const lines = Array.isArray(message.linhas) ? message.linhas : [];
        const looksLikeOurResponse = !lines.length || lines.every(a =>
            (!a.tipo || a.tipo === 'pokemon') &&
            (!a.moeda || a.moeda === currency)
        );
        if (!looksLikeOurResponse) return false;

        const group = 'pokemon:' + currency + ':';
        removeCandidateGroup(group);
        const scanRatio = discoveryRatio();

        for (const listing of lines) {
            if (!listing || !listing.ficha || isOwnListing(listing)) continue;
            const price = Number(listing.preco) * Math.max(1, Number(listing.qtd || 1));
            if (!Number.isFinite(price) || price <= 0) continue;

            const ref = pokemonReferenceFor(listing.ficha, currency);
            if (!ref || !(ref.value > 0)) continue;

            const ratio = price / ref.value;
            if (ratio > scanRatio) continue;

            const species = String(
                listing.ficha.nome ||
                listing.ficha.name ||
                ('Pokémon #' + (speciesIdOf(listing.ficha) || '?'))
            );
            const shiny = listing.ficha.shiny ? 'Shiny ' : '';
            const power = Math.max(1, Number(listing.ficha.potencia || 1));

            addCandidate({
                key: group + listing.id,
                kind: 'pokemon',
                listingId: Number(listing.id),
                pokemonId: Number(listing.pokemonId || listing.ficha.id || 0),
                name: shiny + species + ' · P' + power + ' · Lv ' + Number(listing.ficha.level || 0),
                currency: currency,
                price: price,
                unitPrice: Number(listing.preco),
                average: ref.value,
                arithmeticAverage: ref.average,
                samples: ref.samples,
                ratio: ratio,
                qty: Math.max(1, Number(listing.qtd || 1)),
                seller: String(listing.vendedor || '—'),
                retainedUntil: retentionUntil(listing),
                firstSeenAt: Date.now(),
                seenAt: Date.now()
            });
        }

        scan.waiting = null;
        setTimeout(pumpPokemonScan, 100);
        queueRender();
        return true;
    }

    function balanceFor(currency) {
        return currency === 'orb' ? state.orbs : state.gold;
    }

    function reserveFor(currency) {
        return Math.max(0, Number(currency === 'orb' ? config.gemReserve : config.goldReserve) || 0);
    }

    function maxSpendFor(currency) {
        return Math.max(0, Number(currency === 'orb' ? config.maxGemsPerBuy : config.maxCoinsPerBuy) || 0);
    }

    function currencyAllowed(currency) {
        return currency === 'orb' ? !!config.buyGems : !!config.buyCoins;
    }

    function autoThresholdFor(candidate) {
        return Math.max(0.01, Number(config.autoBuyPercent) / 100);
    }

    function candidateCanAutoBuy(candidate) {
        if (!config.autoBuy || !candidate) return false;
        if (!currencyAllowed(candidate.currency)) return false;
        if (candidate.retainedUntil && candidate.retainedUntil > serverNow()) return false;
        if (candidate.ratio > autoThresholdFor(candidate)) return false;
        if (candidate.kind === 'item') {
            if (!config.autoBuyItems) return false;
            if (Number(candidate.samples || 0) < Math.max(1, Number(config.minItemUnits || 1))) return false;
        }
        if (candidate.kind === 'pokemon') {
            if (!config.autoBuyPokemon) return false;
            if (Number(candidate.samples || 0) < Math.max(1, Number(config.minPokemonSamples || 1))) return false;
        }
        return true;
    }

    function purchaseQuantity(candidate) {
        const balance = balanceFor(candidate.currency);
        const available = Math.max(0, balance - reserveFor(candidate.currency));
        const cap = maxSpendFor(candidate.currency);
        const budget = cap > 0 ? Math.min(available, cap) : available;

        const unitPrice = candidate.kind === 'pokemon'
            ? Number(candidate.unitPrice || candidate.price)
            : Number(candidate.price);

        if (!(unitPrice > 0) || budget < unitPrice) return 0;

        if (candidate.kind === 'pokemon') return 1;
        if (!config.buyWholeItemBatch) return 1;

        return Math.max(0, Math.min(
            Math.max(1, Number(candidate.qty || 1)),
            Math.floor(budget / unitPrice)
        ));
    }

    function tryBuy(candidate, manual) {
        if (!candidate || state.pendingBuy) return false;
        if (!manual && !candidateCanAutoBuy(candidate)) return false;
        if (candidate.retainedUntil && candidate.retainedUntil > serverNow()) return false;

        const now = Date.now();
        const blockedUntil = Number(state.attemptedListings.get(candidate.listingId) || 0);
        if (blockedUntil > now) return false;

        const qty = purchaseQuantity(candidate);
        if (qty < 1) {
            if (manual) addLog('Skipped: not enough spendable balance for ' + candidate.name);
            return false;
        }

        const unitPrice = candidate.kind === 'pokemon'
            ? Number(candidate.unitPrice || candidate.price)
            : Number(candidate.price);
        const spend = unitPrice * qty;
        const balanceBefore = balanceFor(candidate.currency);

        state.pendingBuy = {
            listingId: candidate.listingId,
            candidateKey: candidate.key,
            currency: candidate.currency,
            unitPrice: unitPrice,
            qty: qty,
            spend: spend,
            balanceBefore: balanceBefore,
            sentAt: now,
            manual: !!manual
        };
        state.attemptedListings.set(candidate.listingId, now + 15000);

        const sent = send({
            t: 'market.comprar',
            id: candidate.listingId,
            qtd: qty,
            preco: unitPrice,
            moeda: candidate.currency
        });

        if (!sent) {
            state.pendingBuy = null;
            return false;
        }

        addLog(
            (manual ? 'BUY' : 'AUTO BUY') + ' · ' +
            candidate.name + ' · ' + qty + ' × ' + num(unitPrice) + ' ' +
            (candidate.currency === 'orb' ? 'Gems' : 'Coins') +
            ' · ' + Math.round((1 - candidate.ratio) * 100) + '% under reference'
        );
        queueRender();

        setTimeout(() => {
            if (state.pendingBuy && state.pendingBuy.listingId === candidate.listingId) {
                finishPendingBuy('settle-timeout');
            }
        }, 5000);

        return true;
    }

    function maybeAutoBuy(candidate) {
        if (!candidateCanAutoBuy(candidate)) return;
        tryBuy(candidate, false);
    }

    function finishPendingBuy(reason) {
        const pending = state.pendingBuy;
        if (!pending) return;

        const confirmed = reason === 'server-confirmed';

        state.pendingBuy = null;

        if (confirmed) {
            /*
             * Do not hit the same partially-filled listing again just because
             * it survives our purchase. This also makes max-per-buy a real
             * cap instead of something the next scan silently circumvents.
             */
            state.attemptedListings.set(
                pending.listingId,
                Date.now() + 30 * 60 * 1000
            );
            state.candidates.delete(pending.candidateKey);
            addLog('Purchase confirmed · listing #' + pending.listingId);
        } else if (reason === 'refused') {
            state.attemptedListings.set(
                pending.listingId,
                Date.now() + 5000
            );
        }

        requestItemSummary();
        setTimeout(startPokemonScan, 250);
        queueRender();
    }

    function addLog(text) {
        state.buyLog.unshift({
            at: Date.now(),
            text: String(text)
        });
        if (state.buyLog.length > 12) state.buyLog.length = 12;
    }

    function requestItemSummary() {
        if (!config.enabled || !config.scanItems) return false;

        const delay = backgroundDelayMs();
        if (delay > 0) {
            state.itemStatus = 'queued';
            if (!state.itemSummaryRetryTimer) {
                state.itemSummaryRetryTimer = setTimeout(() => {
                    state.itemSummaryRetryTimer = null;
                    requestItemSummary();
                }, delay + 25);
            }
            return true;
        }

        state.itemStatus = 'scanning';
        return sendBackground({ t: 'market.itens' });
    }

    function runScan(force) {
        if (!config.enabled) return;
        if (!state.socket || state.socket.readyState !== 1) return;

        const now = Date.now();
        const interval = Math.max(1, Number(config.scanSeconds || 5)) * 1000;
        if (!force && now - state.lastScanAt < interval) return;
        state.lastScanAt = now;

        if (config.scanItems) requestItemSummary();
        if (config.scanPokemon) {
            ensurePokemonHistory(false);
            startPokemonScan();
        }

        pruneCandidates();
        queueRender();
    }

    function pruneCandidates() {
        const cutoff = Date.now() - Math.max(30000, Number(config.scanSeconds || 5) * 6000);
        const nowServer = serverNow();
        for (const [key, candidate] of state.candidates) {
            if (candidate.retainedUntil && candidate.retainedUntil > nowServer) continue;
            if (candidate.seenAt < cutoff) state.candidates.delete(key);
        }
        for (const [id, until] of state.attemptedListings) {
            if (until < Date.now()) state.attemptedListings.delete(id);
        }
    }

    function decodeMessage(data) {
        if (typeof data === 'string') {
            try { return JSON.parse(data); } catch { return null; }
        }
        if (data instanceof ArrayBuffer) {
            try { return JSON.parse(new TextDecoder().decode(data)); } catch { return null; }
        }
        return null;
    }

    function handleProtocolObject(message) {
        if (!message || typeof message !== 'object') return;
        state.protocolMessages++;

        if (message.t === 'welcome') {
            mergePlayer(message.estado, true);
            const catalog = message.mercado && Array.isArray(message.mercado.catalogo)
                ? message.mercado.catalogo
                : [];
            state.catalog.clear();
            for (const item of catalog) {
                const id = Number(item && item.id);
                if (Number.isFinite(id) && id > 0) state.catalog.set(id, item);
            }

            setTimeout(() => runScan(true), 650);
            setTimeout(() => ensurePokemonHistory(false), 1200);
            ensureUi();
            queueRender();
            return;
        }

        if (message.t === 'estado') {
            mergePlayer(message.estado, false);
            return;
        }

        if (message.t === 'erro') {
            if (isRateLimitError(message)) {
                enterRateLimitBackoff();
                return;
            }
            if (state.pendingBuy) {
                addLog('Purchase refused: ' + String(message.chave || message.msg || 'server error'));
                finishPendingBuy('refused');
            }
            return;
        }

        /*
         * marketComprado is the buyer-side authoritative acknowledgement.
         * It is stronger than guessing from a balance delta, and upstream
         * itself uses this event to refresh the Community Market after a
         * partial or complete purchase.
         */
        if (message.t === 'batalha') {
            for (const event of message.ev || []) {
                if (event && event.k === 'marketComprado' && state.pendingBuy) {
                    finishPendingBuy('server-confirmed');
                    break;
                }
            }
            return;
        }

        if (message.t !== 'market') return;

        if (message.aba === 'itens') {
            handleItemSummary(message);
            return;
        }
        if (message.aba === 'item') {
            handleItemListings(message);
            return;
        }
        if (message.aba === 'historicoGlobal' && handlePokemonHistory(message)) {
            return;
        }
        handlePokemonListings(message);
    }

    function handleProtocolData(data) {
        if (typeof Blob !== 'undefined' && data instanceof Blob) {
            data.text().then(text => {
                const message = decodeMessage(text);
                if (message) handleProtocolObject(message);
            }).catch(() => {});
            return;
        }
        const message = decodeMessage(data);
        if (message) handleProtocolObject(message);
    }

    function attachSocket(socket) {
        state.socket = socket;

        socket.addEventListener('message', event => {
            handleProtocolData(event.data);
        });

        socket.addEventListener('close', () => {
            if (state.socket === socket) {
                state.socket = null;
                state.itemStatus = 'disconnected';
                state.pokemonStatus = 'disconnected';
                state.pendingBuy = null;
                state.pokemonScan = null;
                state.historyFetch = null;
                if (state.itemSummaryRetryTimer) clearTimeout(state.itemSummaryRetryTimer);
                if (state.detailPumpTimer) clearTimeout(state.detailPumpTimer);
                state.itemSummaryRetryTimer = null;
                state.detailPumpTimer = null;
                for (const entry of state.releaseTimers.values()) clearTimeout(entry.timer);
                state.releaseTimers.clear();
                queueRender();
            }
        });
    }

    function installSocketHook() {
        if (state.hookInstalled) return true;
        const NativeWebSocket = page.WebSocket;
        if (typeof NativeWebSocket !== 'function') return false;

        if (page.__mothMarketWatchWebSocket) {
            state.hookInstalled = true;
            return true;
        }

        try {
            const WrappedWebSocket = new Proxy(NativeWebSocket, {
                construct(target, args) {
                    const socket = Reflect.construct(target, args, target);
                    attachSocket(socket);
                    return socket;
                }
            });

            for (const key of ['CONNECTING', 'OPEN', 'CLOSING', 'CLOSED']) {
                try {
                    Object.defineProperty(
                        WrappedWebSocket,
                        key,
                        { value: NativeWebSocket[key] }
                    );
                } catch {}
            }

            page.WebSocket = WrappedWebSocket;
            page.__mothMarketWatchWebSocket = WrappedWebSocket;
            state.hookInstalled = true;
            return true;
        } catch (error) {
            /*
             * A hostile/non-writable page WebSocket must not kill the market UI.
             * The watch panel remains usable for manual inspection and can still
             * render cached/reference data.
             */
            state.hookInstalled = false;
            state.itemStatus = 'socket hook unavailable';
            state.pokemonStatus = 'socket hook unavailable';
            try { console.warn('[Moth Watch] WebSocket hook unavailable', error); } catch {}
            return false;
        }
    }

    function injectStyle() {
        if (q('#moth-market-watch-style')) return;
        const style = document.createElement('style');
        style.id = 'moth-market-watch-style';
        style.textContent = [
            '#moth-market-watch-tab{white-space:nowrap}',
            '.cm-corpo.moth-watch-active>:not(#moth-market-watch-panel){display:none!important}',
            '#moth-market-watch-panel{display:none;width:100%;height:100%;min-height:420px;overflow:auto;padding:10px;color:var(--sobre-mad,#eee)}',
            '.cm-corpo.moth-watch-active>#moth-market-watch-panel{display:block!important}',
            '.moth-mw-head{display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:10px}',
            '.moth-mw-head h3{margin:0;font-size:15px}',
            '.moth-mw-head .moth-mw-status{font-size:10px;color:var(--sobre-mad-dim,#aaa)}',
            '.moth-mw-spacer{flex:1}',
            '.moth-mw-btn{border:0;border-radius:5px;padding:5px 8px;background:var(--mad,#5a413b);color:var(--sobre-mad,#fff);box-shadow:inset 0 0 0 2px var(--mad-linha,#3a2824);cursor:pointer;font-weight:700;font-size:10px}',
            '.moth-mw-btn:hover{filter:brightness(1.12)}',
            '.moth-mw-btn.primary{background:var(--rx,#84325f);color:#fff}',
            '.moth-mw-btn:disabled{opacity:.45;cursor:default}',
            '.moth-mw-config{display:grid;grid-template-columns:repeat(auto-fit,minmax(190px,1fr));gap:6px;margin-bottom:10px;padding:8px;border-radius:7px;background:var(--vao,#21191d);box-shadow:inset 0 0 0 2px var(--mad-linha,#3a2824)}',
            '.moth-mw-config label{display:flex;align-items:center;justify-content:space-between;gap:8px;font-size:10px;color:var(--sobre-mad-dim,#bbb)}',
            '.moth-mw-config input[type=number]{width:76px;padding:3px 5px;border:1px solid var(--mad-linha,#555);border-radius:4px;background:var(--vao2,#171115);color:var(--sobre-mad,#fff)}',
            '.moth-mw-config input[type=checkbox]{flex:none}',
            '.moth-mw-baseline{display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin:-2px 0 10px;font-size:9px;color:var(--sobre-mad-dim,#aaa)}',
            '.moth-mw-filters{display:flex;align-items:center;gap:10px;flex-wrap:wrap;margin:0 0 10px;padding:7px 8px;border-radius:7px;background:var(--vao,#21191d);box-shadow:inset 0 0 0 2px var(--mad-linha,#3a2824)}',
            '.moth-mw-filter-group{display:flex;align-items:center;gap:4px;flex-wrap:wrap}',
            '.moth-mw-filter-label{font-size:9px;font-weight:700;color:var(--sobre-mad-dim,#aaa);margin-right:2px}',
            '.moth-mw-filter{display:inline-flex;align-items:center;gap:4px;min-height:28px}',
            '.moth-mw-filter img{width:16px;height:16px;object-fit:contain}',
            '.moth-mw-filter.on{filter:brightness(1.18);box-shadow:inset 0 0 0 2px var(--rx,#84325f)}',
            '.moth-mw-sort{display:flex;align-items:center;gap:5px;margin-left:auto}',
            '.moth-mw-sort select{min-height:28px;padding:3px 7px;border:1px solid var(--mad-linha,#555);border-radius:5px;background:var(--vao2,#171115);color:var(--sobre-mad,#fff);font-size:10px}',
            '.moth-mw-list{display:grid;gap:5px}',
            '.moth-mw-row{display:grid;grid-template-columns:minmax(170px,1.4fr) 120px 110px 90px minmax(110px,.8fr) auto;gap:7px;align-items:center;padding:7px;border-radius:6px;background:var(--vao,#21191d);box-shadow:inset 0 0 0 2px var(--mad-linha,#3a2824)}',
            '.moth-mw-row.deep{box-shadow:inset 0 0 0 2px var(--rx,#84325f),inset 0 0 0 4px var(--rx-esc,#5d2345)}',
            '.moth-mw-name{min-width:0}.moth-mw-name b,.moth-mw-name span{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}',
            '.moth-mw-name b{font-size:11px}.moth-mw-name span{font-size:9px;color:var(--sobre-mad-dim,#aaa)}',
            '.moth-mw-kind{display:inline-block;margin-right:5px;padding:1px 4px;border-radius:3px;background:var(--mad,#5a413b);font-size:8px;text-transform:uppercase}',
            '.moth-mw-discount{font-weight:800;color:var(--seta,#efc66b)}',
            '.moth-mw-money{display:inline-flex;align-items:center;gap:3px;font-weight:700}.moth-mw-money img{width:15px;height:15px;object-fit:contain}',
            '.moth-mw-meta{font-size:9px;color:var(--sobre-mad-dim,#aaa)}',
            '.moth-mw-empty{padding:22px;text-align:center;color:var(--sobre-mad-dim,#aaa);background:var(--vao,#21191d);border-radius:6px}',
            '.moth-mw-log{margin-top:10px;padding-top:8px;border-top:1px solid var(--mad-linha,#444);font-size:9px;color:var(--sobre-mad-dim,#aaa)}',
            '.moth-mw-log div{margin:2px 0}',
            '@media(max-width:850px){.moth-mw-row{grid-template-columns:minmax(150px,1fr) 105px 80px auto}.moth-mw-row .moth-mw-ref,.moth-mw-row .moth-mw-seller{display:none}}'
        ].join('\n');
        (document.head || document.documentElement).appendChild(style);
    }

    function baselineStats() {
        const rows = Object.values(state.baseline.items || {});
        let gold = 0;
        let orb = 0;
        for (const row of rows) {
            if (row && row.gold && Number(row.gold.average) > 0) gold++;
            if (row && row.orb && Number(row.orb.average) > 0) orb++;
        }
        return { items: rows.length, gold: gold, orb: orb };
    }

    function exportBaselineText() {
        const rows = state.baseline.items || {};
        const sorted = Object.keys(rows)
            .map(Number)
            .filter(Number.isFinite)
            .sort((a, b) => a - b);
        const lines = ['const HARD_CODED_ITEM_AVERAGES = Object.freeze({'];
        for (const id of sorted) {
            const row = rows[id] || rows[String(id)] || {};
            const compact = {
                name: row.name || catalogName(id),
                gold: row.gold && Number(row.gold.average) > 0
                    ? { average: Math.round(Number(row.gold.average)), units: Number(row.gold.units || 0) }
                    : null,
                orb: row.orb && Number(row.orb.average) > 0
                    ? { average: Math.round(Number(row.orb.average)), units: Number(row.orb.units || 0) }
                    : null
            };
            lines.push('    ' + id + ': ' + JSON.stringify(compact) + ',');
        }
        lines.push('});');
        return lines.join('\n');
    }

    async function copyText(text) {
        try {
            await navigator.clipboard.writeText(text);
            return true;
        } catch {
            try {
                const area = document.createElement('textarea');
                area.value = text;
                area.style.position = 'fixed';
                area.style.opacity = '0';
                document.body.appendChild(area);
                area.select();
                const ok = document.execCommand('copy');
                area.remove();
                return ok;
            } catch {
                return false;
            }
        }
    }

    function configField(label, key, type, suffix) {
        const value = config[key];
        if (type === 'checkbox') {
            return '<label><span>' + escapeHtml(label) + '</span><input type="checkbox" data-moth-cfg="' +
                escapeHtml(key) + '"' + (value ? ' checked' : '') + '></label>';
        }
        return '<label><span>' + escapeHtml(label) + (suffix ? ' <small>' + escapeHtml(suffix) + '</small>' : '') +
            '</span><input type="number" data-moth-cfg="' + escapeHtml(key) + '" value="' +
            escapeHtml(value) + '" min="0" step="1"></label>';
    }

    function renderCandidate(candidate) {
        const discount = Math.max(0, 1 - candidate.ratio);
        const retainedSeconds = candidate.retainedUntil > serverNow()
            ? Math.ceil((candidate.retainedUntil - serverNow()) / 1000)
            : 0;
        const canBuy = !state.pendingBuy && !retainedSeconds && purchaseQuantity(candidate) > 0;
        const auto = candidateCanAutoBuy(candidate);
        const total = candidate.kind === 'item'
            ? candidate.price * candidate.qty
            : candidate.price;
        const referenceLabel = candidate.kind === 'item'
            ? candidate.referenceSource === 'active-median'
                ? 'current market median · ' + num(candidate.activeReferenceListings) + ' listings' +
                    (candidate.serverAverage > candidate.average
                        ? ' · server 7d avg ' + num(Math.round(candidate.serverAverage)) + ' ignored'
                        : '')
                : (candidate.referenceSource === 'live-7d' ? 'live 7d avg / unit' : 'frozen fallback / unit') +
                    ' · ' + num(candidate.samples) + ' unit' + (candidate.samples === 1 ? '' : 's') + ' sold'
            : 'sales median · ' + num(candidate.samples) + ' sale' + (candidate.samples === 1 ? '' : 's');

        return '<div class="moth-mw-row' + (candidate.ratio <= autoThresholdFor(candidate) ? ' deep' : '') + '">' +
            '<div class="moth-mw-name"><b><span class="moth-mw-kind">' + escapeHtml(candidate.kind) + '</span>' +
            escapeHtml(candidate.name) + '</b><span>' +
            (candidate.kind === 'item' ? num(candidate.qty) + ' units · total ' + num(total) : 'completed-sales reference') +
            '</span></div>' +
            '<div><div>' + money(candidate.price, candidate.currency) + '</div><div class="moth-mw-meta">listed' +
            (candidate.kind === 'item' && candidate.qty > 1 ? ' / unit' : '') + '</div></div>' +
            '<div class="moth-mw-ref"><div>' + money(Math.round(candidate.average), candidate.currency) +
            '</div><div class="moth-mw-meta">' + escapeHtml(referenceLabel) + '</div></div>' +
            '<div><div class="moth-mw-discount">−' + pct(discount) + '</div><div class="moth-mw-meta">' +
            (auto ? 'auto-buy range' : 'watch range') + '</div></div>' +
            '<div class="moth-mw-seller"><b>Seller: ' + escapeHtml(candidate.seller) + '</b><div class="moth-mw-meta">' +
            (retainedSeconds ? 'retained · ' + retainedSeconds + 's' : 'available') + '</div></div>' +
            '<button type="button" class="moth-mw-btn primary" data-moth-buy="' + escapeHtml(candidate.key) + '"' +
            (canBuy ? '' : ' disabled') + '>' + (retainedSeconds ? 'Wait' : 'Buy') + '</button>' +
            '</div>';
    }

    function candidateVisible(candidate) {
        if (!candidate) return false;

        const currencyFilter = ['gold', 'orb'].includes(config.viewCurrency)
            ? config.viewCurrency
            : 'all';
        const kindFilter = ['item', 'pokemon'].includes(config.viewKind)
            ? config.viewKind
            : 'all';

        if (currencyFilter !== 'all' && candidate.currency !== currencyFilter) return false;
        if (kindFilter !== 'all' && candidate.kind !== kindFilter) return false;
        return true;
    }

    function candidateSort(a, b) {
        const mode = [
            'discount',
            'newest',
            'price-asc',
            'price-desc',
            'reference-desc',
            'quantity-desc',
            'name'
        ].includes(config.viewSort)
            ? config.viewSort
            : 'discount';

        /*
         * Raw Coin and Gem amounts are not comparable. For value-based sorts
         * with both currencies visible, keep currencies grouped first.
         */
        const currencyGroup = a.currency === b.currency
            ? 0
            : a.currency === 'gold' ? -1 : 1;

        if (mode === 'newest') {
            return Number(b.firstSeenAt || b.seenAt || 0) - Number(a.firstSeenAt || a.seenAt || 0) ||
                a.name.localeCompare(b.name);
        }
        if (mode === 'price-asc') {
            return currencyGroup || Number(a.price || 0) - Number(b.price || 0) ||
                a.name.localeCompare(b.name);
        }
        if (mode === 'price-desc') {
            return currencyGroup || Number(b.price || 0) - Number(a.price || 0) ||
                a.name.localeCompare(b.name);
        }
        if (mode === 'reference-desc') {
            return currencyGroup || Number(b.average || 0) - Number(a.average || 0) ||
                a.name.localeCompare(b.name);
        }
        if (mode === 'quantity-desc') {
            return Number(b.qty || 0) - Number(a.qty || 0) ||
                a.ratio - b.ratio ||
                a.name.localeCompare(b.name);
        }
        if (mode === 'name') {
            return a.name.localeCompare(b.name) ||
                a.ratio - b.ratio;
        }

        return a.ratio - b.ratio ||
            Number(b.qty || 0) - Number(a.qty || 0) ||
            a.name.localeCompare(b.name);
    }

    function sortSelect() {
        const options = [
            ['discount', 'Biggest discount'],
            ['newest', 'Newest detected'],
            ['price-asc', 'Price: low → high'],
            ['price-desc', 'Price: high → low'],
            ['reference-desc', 'Reference: high → low'],
            ['quantity-desc', 'Quantity: high → low'],
            ['name', 'Name: A → Z']
        ];

        return '<label class="moth-mw-sort"><span class="moth-mw-filter-label">Sort</span>' +
            '<select id="moth-mw-sort">' +
            options.map(([value, label]) =>
                '<option value="' + value + '"' + (config.viewSort === value ? ' selected' : '') + '>' +
                escapeHtml(label) + '</option>'
            ).join('') +
            '</select></label>';
    }

    function filterButton(label, group, value, active, icon) {
        const nativeClass = group === 'currency' ? 'cm-moeda' : 'cm-filtro';
        return '<button type="button" class="' + nativeClass + ' moth-mw-filter' + (active ? ' on' : '') + '"' +
            ' data-moth-filter="' + escapeHtml(group) + '" data-moth-value="' + escapeHtml(value) + '"' +
            ' aria-pressed="' + (active ? 'true' : 'false') + '">' +
            (icon ? '<img src="' + escapeHtml(icon) + '" alt="">' : '') +
            escapeHtml(label) +
            '</button>';
    }

    function renderWatch() {
        const panel = q('#moth-market-watch-panel');
        if (!panel || !state.watchOpen) return;
        if (panel.contains(document.activeElement) && document.activeElement.matches('input')) return;

        const baseline = baselineStats();
        const currencyFilter = ['gold', 'orb'].includes(config.viewCurrency) ? config.viewCurrency : 'all';
        const kindFilter = ['item', 'pokemon'].includes(config.viewKind) ? config.viewKind : 'all';
        const candidates = Array.from(state.candidates.values())
            .filter(c =>
                c.ratio <= Math.max(0.01, Number(config.watchPercent) / 100) &&
                candidateVisible(c)
            )
            .sort(candidateSort);

        const captured = state.baseline.capturedAt
            ? new Date(state.baseline.capturedAt).toLocaleString()
            : 'not frozen yet';

        panel.innerHTML =
            '<div class="moth-mw-head">' +
            '<h3>Moth Watch</h3>' +
            '<span class="moth-mw-status">Items: ' + escapeHtml(state.itemStatus) +
            ' · Pokémon: ' + escapeHtml(state.pokemonStatus) +
            ' · ' + num(state.protocolMessages) + ' protocol messages</span>' +
            '<span class="moth-mw-spacer"></span>' +
            '<button type="button" class="moth-mw-btn" id="moth-mw-scan">Scan now</button>' +
            '</div>' +
            '<div class="moth-mw-config">' +
            configField('Watch prices ≤ baseline', 'watchPercent', 'number', '%') +
            configField('Auto-buy prices ≤ baseline', 'autoBuyPercent', 'number', '%') +
            configField('Scan every', 'scanSeconds', 'number', 'seconds') +
            configField('Auto-buy', 'autoBuy', 'checkbox') +
            configField('Watch items', 'scanItems', 'checkbox') +
            configField('Watch Pokémon', 'scanPokemon', 'checkbox') +
            configField('Auto-buy items', 'autoBuyItems', 'checkbox') +
            configField('Auto-buy Pokémon', 'autoBuyPokemon', 'checkbox') +
            configField('Buy Coin listings', 'buyCoins', 'checkbox') +
            configField('Buy Gem listings', 'buyGems', 'checkbox') +
            configField('Take full item batch', 'buyWholeItemBatch', 'checkbox') +
            configField('Keep Coins', 'goldReserve', 'number', '') +
            configField('Keep Gems', 'gemReserve', 'number', '') +
            configField('Max Coins / buy', 'maxCoinsPerBuy', 'number', '0 = unlimited') +
            configField('Max Gems / buy', 'maxGemsPerBuy', 'number', '0 = unlimited') +
            configField('Min item units in average', 'minItemUnits', 'number', '') +
            configField('Min Pokémon sale samples', 'minPokemonSamples', 'number', '') +
            configField('Pokémon history pages', 'pokemonHistoryPages', 'number', '') +
            '</div>' +
            '<div class="moth-mw-baseline">' +
            '<b>Item references:</b> live server 7d averages · frozen fallback ' + escapeHtml(captured) +
            ' · ' + num(baseline.items) + ' items · ' + num(baseline.gold) + ' Coin references · ' +
            num(baseline.orb) + ' Gem references' +
            ' · current irregular: ' + num(state.itemScanStats.goldSuspicious) + '/' +
            num(state.itemScanStats.goldChecked) + ' Coin · ' +
            num(state.itemScanStats.orbSuspicious) + '/' +
            num(state.itemScanStats.orbChecked) + ' Gem' +
            '<span class="moth-mw-spacer"></span>' +
            '<button type="button" class="moth-mw-btn" id="moth-mw-refresh-baseline">Freeze current 7d averages</button>' +
            '<button type="button" class="moth-mw-btn" id="moth-mw-export-baseline">Export baseline JSON</button>' +
            '</div>' +
            '<div class="moth-mw-filters">' +
            '<div class="moth-mw-filter-group"><span class="moth-mw-filter-label">Currency</span>' +
            filterButton('All', 'currency', 'all', currencyFilter === 'all', '') +
            filterButton('Coins', 'currency', 'gold', currencyFilter === 'gold', '/assets/site/assets/ui/moeda-ouro.png') +
            filterButton('Gems', 'currency', 'orb', currencyFilter === 'orb', '/img/moeda-gema.png') +
            '</div>' +
            '<div class="moth-mw-filter-group"><span class="moth-mw-filter-label">Type</span>' +
            filterButton('All', 'kind', 'all', kindFilter === 'all', '') +
            filterButton('Items', 'kind', 'item', kindFilter === 'item', '/assets/site/assets/ui/menu-bolsa.png') +
            filterButton('Pokémon', 'kind', 'pokemon', kindFilter === 'pokemon', '/assets/site/assets/ui/ball-poke.png') +
            '</div>' +
            sortSelect() +
            '</div>' +
            '<div class="moth-mw-list">' +
            (candidates.length
                ? candidates.map(renderCandidate).join('')
                : '<div class="moth-mw-empty">No listings currently below the watch threshold.</div>') +
            '</div>' +
            '<div class="moth-mw-log"><b>Recent bot activity</b>' +
            (state.buyLog.length
                ? state.buyLog.map(row => '<div>' + new Date(row.at).toLocaleTimeString() + ' · ' + escapeHtml(row.text) + '</div>').join('')
                : '<div>No purchases attempted this session.</div>') +
            '</div>';

        for (const input of qa('[data-moth-cfg]', panel)) {
            input.addEventListener('change', () => {
                const key = input.dataset.mothCfg;
                if (!(key in DEFAULTS)) return;
                config[key] = input.type === 'checkbox'
                    ? input.checked
                    : Math.max(0, Number(input.value) || 0);
                saveConfig();
                if (key === 'pokemonHistoryPages' || key === 'minPokemonSamples') {
                    ensurePokemonHistory(true);
                }
                runScan(true);
                queueRender();
            });
        }

        for (const button of qa('[data-moth-filter]', panel)) {
            button.addEventListener('click', () => {
                const group = button.dataset.mothFilter;
                const value = button.dataset.mothValue;

                if (group === 'currency' && ['all', 'gold', 'orb'].includes(value)) {
                    config.viewCurrency = value;
                } else if (group === 'kind' && ['all', 'item', 'pokemon'].includes(value)) {
                    config.viewKind = value;
                } else {
                    return;
                }

                saveConfig();
                queueRender();
            });
        }

        const sort = q('#moth-mw-sort', panel);
        if (sort) {
            sort.addEventListener('change', () => {
                config.viewSort = sort.value;
                saveConfig();
                queueRender();
            });
        }

        q('#moth-mw-scan', panel).addEventListener('click', () => runScan(true));

        q('#moth-mw-refresh-baseline', panel).addEventListener('click', () => {
            state.forceBaselineNext = true;
            state.baseline = emptyBaseline();
            saveBaseline();
            requestItemSummary();
            queueRender();
        });

        q('#moth-mw-export-baseline', panel).addEventListener('click', async () => {
            const ok = await copyText(exportBaselineText());
            addLog(ok ? 'Baseline source copied to clipboard' : 'Could not copy baseline');
            queueRender();
        });

        for (const button of qa('[data-moth-buy]', panel)) {
            button.addEventListener('click', () => {
                const candidate = state.candidates.get(button.dataset.mothBuy);
                if (candidate) tryBuy(candidate, true);
            });
        }
    }

    function queueRender() {
        if (state.renderQueued) return;
        state.renderQueued = true;
        requestAnimationFrame(() => {
            state.renderQueued = false;
            ensureUi();
            renderWatch();
        });
    }

    function openWatch() {
        ensureUi();
        const body = q('.cm-corpo');
        if (!body) return;
        state.watchOpen = true;
        body.classList.add('moth-watch-active');
        const button = q('#moth-market-watch-tab');
        if (button) button.classList.add('on');
        runScan(true);
        renderWatch();
    }

    function closeWatch() {
        state.watchOpen = false;
        const body = q('.cm-corpo');
        if (body) body.classList.remove('moth-watch-active');
        const button = q('#moth-market-watch-tab');
        if (button) button.classList.remove('on');
    }

    function ensureUi() {
        injectStyle();

        const top = q('.cm-topo');
        const body = q('.cm-corpo');
        if (!top || !body) return false;

        let button = q('#moth-market-watch-tab');
        if (!button) {
            button = document.createElement('button');
            button.id = 'moth-market-watch-tab';
            button.type = 'button';
            const shortcutHost = q('.cm-atalhos', top);
            if (shortcutHost) {
                button.className = 'cm-acao cm-atalho';
                button.innerHTML = '<i aria-hidden="true">M</i><span>Moth Watch</span>';
            } else {
                button.className = 'cm-acao';
                button.textContent = 'Moth Watch';
            }
            button.title = 'Underpriced listings and market sniper settings';
            button.addEventListener('click', event => {
                event.preventDefault();
                event.stopPropagation();
                if (state.watchOpen) closeWatch();
                else openWatch();
            });

            (q('.cm-atalhos', top) || top).appendChild(button);
        }

        let panel = q('#moth-market-watch-panel');
        if (!panel) {
            panel = document.createElement('section');
            panel.id = 'moth-market-watch-panel';
            body.appendChild(panel);
        }

        if (!top.dataset.mothWatchBound) {
            top.dataset.mothWatchBound = '1';
            top.addEventListener('click', event => {
                if (event.target.closest('#moth-market-watch-tab')) return;
                if (state.watchOpen) closeWatch();
            }, true);
        }

        if (state.watchOpen) {
            body.classList.add('moth-watch-active');
            button.classList.add('on');
        }

        return true;
    }

    function bootstrap() {
        try { installSocketHook(); } catch {}
        try { injectStyle(); } catch {}

        state.scanTimer = setInterval(() => {
            ensureUi();
            runScan(false);
            if (state.pendingBuy && Date.now() - state.pendingBuy.sentAt > 5000) {
                finishPendingBuy('watchdog');
            }
            if (state.watchOpen) queueRender();
        }, 1000);

        const observer = new MutationObserver(() => {
            if (q('.cm-topo')) ensureUi();
        });

        observer.observe(
            document.documentElement || document,
            { childList: true, subtree: true }
        );

        ensureUi();
        console.info('[Moth Watch] v0.1.11 loaded');
    }

    bootstrap();
})();
