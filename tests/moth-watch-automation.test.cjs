'use strict';

// Run with node --test tests/moth-watch-automation.test.cjs.
// These tests exercise userscript code without a live game or real orders.
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const script = fs.readFileSync(path.join(__dirname, '..', 'addons', 'market-bot.user.js'), 'utf8');

function section(start, end) {
    const a = script.indexOf(start), b = script.indexOf(end, a + start.length);
    assert.ok(a >= 0 && b > a, 'missing section: ' + start);
    return script.slice(a, b);
}
const match = script.match(/const DEFAULTS = Object.freeze\((\{[\s\S]*?\})\);/);
assert.ok(match);
const defaults = new Function('return ' + match[1])();

test('New profiles default to no automated purchases, and old mechanisms cannot submit sales', () => {
    assert.equal(defaults.fixedRulesEnabled, false);
    assert.deepEqual(defaults.fixedItemRules, []);
    for (const old of ['autoBuy', 'autoBuyPercent', 'autoBuyItems', 'autoBuyPokemon', 'autoBuyDiamonds',
        'buyCoins', 'buyGems', 'autoSellEnabled', 'autoSellRules']) {
        assert.equal(Object.hasOwn(defaults, old), false, old);
    }
    assert.doesNotMatch(script, /market\.criar/);
    assert.doesNotMatch(script, /candidateCanAutoBuy|sendItemSale|sanitizeAutoSellRules|maybeAutoBuy/);
});

test('Existing fixed rules retain buying parameters but never restore selling or percentage options', () => {
    const storage = {
        value: JSON.stringify({
            fixedRulesEnabled: true, autoBuy: true, autoBuyPercent: 90,
            autoSellEnabled: true, autoSellRules: [{ itemId: 106, enabled: true }],
            fixedItemRules: [{ id: 'old', itemId: 106, currency: 'gold', buyEnabled: true,
                buyMax: 30000, sellEnabled: true, sellPrice: 45000, keepQty: 2 }]
        }),
        getItem() { return this.value; }, setItem(_, v) { this.value = v; }
    };
    const src = section('    function sanitizeFixedItemRules(', '    function emptyBaseline(');
    const fn = new Function('localStorage', 'DEFAULTS', 'CONFIG_KEY', src +
        '\nreturn { sanitizeFixedItemRules, loadConfig };')(storage, defaults, 'settings');
    const loaded = fn.loadConfig();
    assert.equal(loaded.fixedRulesEnabled, true);
    assert.equal(loaded.fixedItemRules.length, 1);
    assert.equal(loaded.fixedItemRules[0].buyMax, 30000);
    assert.equal(loaded.fixedItemRules[0].buyEnabled, true);
    for (const key of ['sellEnabled', 'sellPrice', 'keepQty']) {
        assert.equal(Object.hasOwn(loaded.fixedItemRules[0], key), false, key);
    }
    for (const key of ['autoBuy', 'autoBuyPercent', 'autoSellEnabled', 'autoSellRules']) {
        assert.equal(Object.hasOwn(loaded, key), false, key);
    }
});

test('Rule matching directly examines fresh item listings, skips attempted and retained offers', () => {
    const now = Date.now();
    const state = {
        itemOfferDetails: new Map([['106:gold', { updatedAt: now, offers: [
            { id: 11, preco: 18000, qtd: 2, vendedor: 'other' },
            { id: 12, preco: 24000, qtd: 3, vendedor: 'other' },
            { id: 13, preco: 5000, qtd: 4, vendedor: 'self' },
            { id: 14, preco: 6000, qtd: 4, vendedor: 'other', compravelEm: now + 60000 },
            { id: 15, preco: 65000, qtd: 4, vendedor: 'other' }
        ] }]]),
        attemptedListings: new Map([[11, now + 60000]])
    };
    const fn = new Function('state', 'isOwnListing', 'retentionUntil', 'serverNow',
        'itemReference', 'catalogName', section('    function fixedRuleCandidate(', '    function runMarketAutomation(') +
        '\nreturn fixedRuleCandidate;')(state, offer => offer.vendedor === 'self',
        offer => Number(offer.compravelEm || 0) > now ? offer.compravelEm : 0,
        () => now, () => null, () => 'Cocoon Stone');
    const rule = { itemId: 106, currency: 'gold', buyMax: 30000 };
    assert.equal(fn(rule).listingId, 12, 'An already attempted cheap offer must not block the next one');
    assert.equal(fn(rule).price, 24000);
    assert.equal(fn(rule).average, 0, 'No reference data should be required');
    state.attemptedListings.delete(11);
    assert.equal(fn(rule).listingId, 11);
    state.itemOfferDetails.get('106:gold').updatedAt = now - 50000;
    assert.equal(fn(rule), null, 'Very old listings require a refresh');
});

test('Purchase quantity honors the whole available offer, balance, reserves and transaction caps', () => {
    const src = section('    function purchaseQuantity(', '    function tryBuy(');
    const create = new Function('balanceFor', 'reserveFor', 'maxSpendFor', 'pokemonCurrencyAllowed',
        src + '\nreturn purchaseQuantity;');
    const fn = create(() => 100000, () => 10000, () => 30000, () => true);
    assert.equal(fn({ kind: 'item', currency: 'gold', price: 5000, qty: 20 }), 6);
    assert.equal(fn({ kind: 'item', currency: 'gold', price: 5000, qty: 3 }), 3);
    assert.equal(fn({ kind: 'item', currency: 'gold', price: 35000, qty: 3 }), 0);
    assert.equal(create(() => 100000, () => 10000, () => 0, () => true)
        ({ kind: 'item', currency: 'gold', price: 5000, qty: 100 }), 18);
});

test('Multiple fixed rules are checked fairly, including after an unaffordable rule', () => {
    const state = { socket: { readyState: 1 }, pendingBuy: null, automationLastRunAt: 0, ruleItemKeys: new Set() };
    const rules = [
        { itemId: 106, currency: 'gold', buyMax: 30000, buyEnabled: true },
        { itemId: 107, currency: 'orb', buyMax: 50, buyEnabled: true }
    ];
    const config = { enabled: true, fixedRulesEnabled: true, fixedItemRules: rules };
    const requested = [], bought = [];
    const fn = new Function('config', 'state', 'gameIsLoading', 'q', 'sanitizeFixedItemRules',
        'automationDetail', 'fixedRuleCandidate', 'purchaseQuantity', 'tryBuy',
        section('    function runMarketAutomation(', '    function runScan(force)') + '\nreturn runMarketAutomation;')(
        config, state, () => false, () => null, v => v,
        id => { requested.push(id); }, rule => ({ listingId: rule.itemId, name: String(rule.itemId) }),
        candidate => candidate.listingId === 106 ? 0 : 1,
        candidate => { bought.push(candidate.listingId); return true; }
    );
    fn();
    assert.deepEqual(requested, [106, 107]);
    assert.deepEqual(bought, [107], 'Later eligible rules must still execute');
    config.fixedRulesEnabled = false;
    state.automationLastRunAt = 0;
    fn();
    assert.deepEqual(bought, [107], 'Master switch off must prevent orders');
});

test('Rule requests get ahead of background scans without displacing manual item inspections', () => {
    const state = {
        detailPending: null, detailQueue: [
            { key: '105:gold' }, { key: '999:gold' }
        ], detailQueued: new Set(), ruleItemKeys: new Set(['105:gold', '106:gold']),
        selectedItemFocus: null, sellRecommendationKey: null,
        lastDetailRequest: new Map()
    };
    const fn = new Function('state', 'config', 'scheduleItemDetailPump',
        section('    function queueItemDetail(', '    function scheduleItemDetailPump(') +
        '\nreturn queueItemDetail;')(state, {scanSeconds:5}, () => {});
    fn(106,'gold');
    assert.deepEqual(state.detailQueue.map(x => x.key), ['105:gold','106:gold','999:gold']);
    state.selectedItemFocus = { key:'107:orb' };
    fn(107,'orb');
    assert.equal(state.detailQueue[0].key, '107:orb');
});

test('Only buying controls are rendered, with a visible explanation of activation prerequisites', () => {
    const helpers = section('    function ruleNumber(', '    function renderWatch(');
    const fixedRulesHtml = new Function('config', 'sanitizeFixedItemRules', 'escapeHtml', 'tr',
        'catalogName', 'fixedRuleStatus', 'ruleCurrencyUnused', 'automationItemOptions', 'configField',
        helpers + '\nreturn fixedRulesHtml;')(
        { fixedItemRules: [{id:'a',itemId:106,currency:'gold',buyEnabled:true,buyMax:30000}] },
        v => v, String, v => v, () => 'Cocoon Stone', () => 'Monitoring',
        null, () => '<option>106</option>', () => '<label>OFF</label>');
    const html = fixedRulesHtml();
    assert.match(html, /moth-fixed-buy/);
    assert.match(html, /data-moth-fixed-field="buyMax"/);
    for(const obsolete of ['moth-fixed-sell','moth-fixed-keep','moth-add-auto-sell','moth-auto-sell',
        'moth-mw-cfg-autoBuyPercent']) assert.equal(html.includes(obsolete), false, obsolete);
    assert.match(script, /No other buying switches are required/);
    assert.match(script, /Rule-based buying is on, but no Buy rules are active/);
});

test('Userscript version and monitor attribute match, with valid syntax and Portuguese guidance', () => {
    const version = script.match(/\/\/ @version\s+([\d.]+)/)?.[1];
    const marker = script.match(/data-moth-addon-version-market-bot', '([\d.]+)'/)?.[1];
    assert.ok(version);
    assert.equal(version, marker);
    assert.doesNotThrow(() => new Function(script));
    const pt = section('    const PT_BR = Object.freeze(', '    function tr(');
    const dict = new Function(pt + '\nreturn PT_BR;')();
    assert.ok(dict['Enable rule-based buying']);
    assert.ok(dict['Rule-based buying is on, but no Buy rules are active. Create or enable a rule first.']);
});
