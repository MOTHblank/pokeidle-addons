'use strict';

// Execute with: node --test tests/moth-watch-automation.test.cjs
// These tests exercise the actual userscript functions without a live game.
// They do not submit orders to the server.
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const script = fs.readFileSync(path.join(__dirname, '..', 'addons', 'market-bot.user.js'), 'utf8');

function section(start, end) {
    const a = script.indexOf(start);
    const b = script.indexOf(end, a + start.length);
    assert.ok(a >= 0 && b > a, `missing section: ${start}`);
    return script.slice(a, b);
}

const sanitizers = section('    function sanitizeAutoSellRules(', '    function loadConfig(');
const { sanitizeAutoSellRules, sanitizeFixedItemRules } =
    new Function(sanitizers + '\nreturn { sanitizeAutoSellRules, sanitizeFixedItemRules };')();

test('Moth Watch purchase and sale automations are off for new profiles', () => {
    const match = script.match(/const DEFAULTS = Object.freeze\((\{[\s\S]*?\})\);/);
    assert.ok(match, 'DEFAULTS must be present');
    const defaults = new Function('return ' + match[1])();
    for (const key of [
        'autoBuy', 'autoBuyItems', 'autoBuyPokemon', 'autoBuyDiamonds',
        'autoSellEnabled', 'fixedRulesEnabled', 'buyCoins', 'buyGems'
    ]) {
        assert.equal(defaults[key], false, key);
    }
});

test('Old saved rules retain their prices and stock reserve but lose quantity toggles', () => {
    const fixed = sanitizeFixedItemRules([{
        id: 'old-fixed', itemId: 106, currency: 'gold',
        buyEnabled: true, buyMax: 30000, buyQtyLimit: 1, buyAll: false,
        sellEnabled: true, sellPrice: 45000, maxSellQty: 1, sellAll: false,
        keepQty: 2
    }])[0];
    assert.equal(fixed.buyMax, 30000);
    assert.equal(fixed.sellPrice, 45000);
    assert.equal(fixed.keepQty, 2);
    assert.equal(fixed.buyEnabled, true);
    assert.equal(fixed.sellEnabled, true);
    for (const removed of ['buyAll', 'buyQtyLimit', 'sellAll', 'maxSellQty']) {
        assert.equal(removed in fixed, false, removed);
    }
    const sell = sanitizeAutoSellRules([{
        itemId: 106, currency: 'orb', mode: 'patient', enabled: true,
        keepQty: 4, maxSellQty: 1, sellAll: false
    }])[0];
    assert.equal(sell.keepQty, 4);
    assert.equal(sell.mode, 'patient');
    assert.equal('sellAll' in sell, false);
});

test('Buy quantity uses full affordable listing, respecting balances, reserves and per-order cap', () => {
    const fn = new Function('balanceFor', 'reserveFor', 'maxSpendFor',
        section('    function purchaseQuantity(', '    function tryBuy(') +
        '\nreturn purchaseQuantity;');
    const purchase = fn(() => 100000, () => 10000, () => 30000);
    assert.equal(purchase({ kind: 'item', currency: 'gold', qty: 20, price: 5000 }), 6);
    assert.equal(purchase({ kind: 'item', currency: 'gold', qty: 3, price: 5000 }), 3);
    assert.equal(purchase({ kind: 'item', currency: 'gold', qty: 5, price: 50000 }), 0);
    const noExtraCap = fn(() => 100000, () => 10000, () => 0);
    assert.equal(noExtraCap({ kind: 'item', currency: 'gold', qty: 100, price: 5000 }), 18);
});

test('Sell quantity uses all owned items except the amount kept in inventory', () => {
    const fn = new Function('ownedQuantity',
        section('    function automationSellQuantity(', '    function runMarketAutomation(') +
        '\nreturn automationSellQuantity;');
    const quantity = fn(() => 10);
    assert.equal(quantity({ itemId: 106, keepQty: 0 }), 10);
    assert.equal(quantity({ itemId: 106, keepQty: 2 }), 8);
    assert.equal(quantity({ itemId: 106, keepQty: 20 }), 0);
});

test('Rules UI has prices and stock reserve but no obsolete quantity fields', () => {
    const helperCode = sanitizers +
        section('    function ruleNumber(', '    function renderWatch(');
    const stubs = `
        const tr = value => value;
        const escapeHtml = value => String(value);
        const config = {
            fixedItemRules: [{
                id: 'example', itemId: 106, currency: 'gold',
                buyEnabled: true, buyMax: 30000,
                sellEnabled: true, sellPrice: 45000, keepQty: 3
            }],
            autoSellRules: [], fixedRulesEnabled: false, autoSellEnabled: false
        };
        const catalogName = () => 'Cocoon Stone';
        const automationRuleStatus = () => 'Inactive';
        const autoSellTargetsHtml = () => '';
        const automationItemOptions = () => '<option value="106">Cocoon Stone</option>';
        const configField = () => '<label>OFF</label>';
    `;
    const render = new Function(stubs + helperCode +
        '\nreturn { fixedRulesHtml, autoSellRulesHtml };')();
    const html = render.fixedRulesHtml() + render.autoSellRulesHtml();
    for (const id of [
        'moth-fixed-buy', 'moth-fixed-sell', 'moth-fixed-keep',
        'moth-fixed-item', 'moth-fixed-currency', 'moth-auto-sell-keep',
        'moth-add-fixed-rule', 'moth-add-auto-sell'
    ]) assert.ok(html.includes(id), id);
    for (const id of [
        'moth-fixed-buy-limit', 'moth-fixed-sell-limit',
        'moth-fixed-buy-all', 'moth-fixed-sell-all',
        'moth-auto-sell-max', 'moth-auto-sell-all'
    ]) assert.ok(!html.includes(id), id);
    assert.ok(html.includes('Cocoon Stone'));
    assert.ok(html.includes('Buy every available unit'));
});


test('Shiny Pokémon only use Gems; ordinary Pokémon remain eligible for both currencies', () => {
    const policyCode = section('    function pokemonCurrencyAllowed(', '    function median(');
    const policy = new Function('speciesIdOf', 'levelBand', policyCode +
        '\nreturn { pokemonCurrencyAllowed, pokemonBucketKeys };')(
        ficha => ficha.speciesId, () => 10
    );
    const shiny = { speciesId: 25, shiny: true, potencia: 3, level: 20 };
    const regular = { speciesId: 25, shiny: false, potencia: 3, level: 20 };

    assert.equal(policy.pokemonCurrencyAllowed(shiny, 'gold'), false);
    assert.equal(policy.pokemonCurrencyAllowed(shiny, 'orb'), true);
    assert.equal(policy.pokemonCurrencyAllowed(regular, 'gold'), true);
    assert.equal(policy.pokemonCurrencyAllowed(regular, 'orb'), true);
    assert.deepEqual(policy.pokemonBucketKeys(shiny, 'gold'), []);
    assert.ok(policy.pokemonBucketKeys(shiny, 'orb').every(k => k.startsWith('orb|')));
    assert.ok(policy.pokemonBucketKeys(regular, 'gold').every(k => k.startsWith('gold|')));

    const sampleHistory = new Function('pokemonCurrencyAllowed', 'pokemonBucketKeys',
        section('    function addPokemonHistorySample(', '    function finalizePokemonHistory(') +
        '\nreturn addPokemonHistorySample;')(
        policy.pokemonCurrencyAllowed, policy.pokemonBucketKeys
    );
    const working = new Map();
    sampleHistory({ tipo: 'pokemon', ficha: shiny, moeda: 'gold', preco: 2000 }, working);
    assert.equal(working.size, 0, 'Old Shiny Coin sales must be ignored');
    sampleHistory({ tipo: 'pokemon', ficha: shiny, moeda: 'orb', preco: 100 }, working);
    assert.ok(working.size > 0, 'Shiny Gem sales remain valid');
    sampleHistory({ tipo: 'pokemon', ficha: regular, moeda: 'gold', preco: 3000 }, working);
    assert.ok([...working.keys()].some(k => k.startsWith('gold|')));
});

test('Shiny Coin purchases are blocked by both auto-buy and purchase quantity guards', () => {
    const policyCode = section('    function pokemonCurrencyAllowed(', '    function pokemonBucketKeys(');
    const policy = new Function(policyCode + '\nreturn pokemonCurrencyAllowed;')();
    const config = {
        autoBuy: true, autoBuyPercent: 40, autoBuyPokemon: true,
        minPokemonSamples: 3
    };
    const logic = section('    function candidateCanAutoBuy(', '    function tryBuy(');
    const buy = new Function('config', 'pokemonCurrencyAllowed',
        'currencyAllowed', 'serverNow', 'balanceFor', 'reserveFor', 'maxSpendFor',
        logic + '\nreturn { candidateCanAutoBuy, purchaseQuantity };')(
        config, policy, () => true, () => Date.now(),
        () => 10000, () => 0, () => 0
    );
    const candidate = {
        kind: 'pokemon', currency: 'gold', ficha: { shiny: true },
        price: 500, unitPrice: 500, qty: 1, ratio: 0.2, samples: 10
    };
    assert.equal(buy.candidateCanAutoBuy(candidate), false);
    assert.equal(buy.purchaseQuantity(candidate), 0);
    candidate.currency = 'orb';
    assert.equal(buy.candidateCanAutoBuy(candidate), true);
    assert.equal(buy.purchaseQuantity(candidate), 1);
    candidate.currency = 'gold';
    candidate.ficha = { shiny: false };
    assert.equal(buy.candidateCanAutoBuy(candidate), true);
    assert.equal(buy.purchaseQuantity(candidate), 1);
});

test('Controller manual purchase validates supplied Shiny metadata', () => {
    const buyCode = section('    function buyFromController(', '    function configureFromController(');
    assert.match(buyCode, /const requestFicha = request\.ficha \|\| request\.pokemon \|\| null;/);
    assert.match(buyCode, /currency === 'gold'/);
    assert.match(buyCode, /shiny Pokémon listings require Gems/);
    assert.match(buyCode, /ok: tryBuy\(candidate, true, requestedQty\)/);
    // The native Controller may not provide Pokémon metadata. In that case
    // only the game's own server-side rules can reliably reject an invalid order.
});

test('Userscript metadata and monitor version stay aligned', () => {
    const version = script.match(/\/\/ @version\s+([\d.]+)/)?.[1];
    const marker = script.match(/data-moth-addon-version-market-bot', '([\d.]+)'/)?.[1];
    assert.ok(version);
    assert.equal(marker, version);
    assert.doesNotThrow(() => new Function(script));
});
