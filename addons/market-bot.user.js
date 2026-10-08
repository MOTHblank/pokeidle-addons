// ==UserScript==
// @name         PokéIdle Moth Watch
// @namespace    moth.pokeidle
// @version      0.1.13
// @description  Community Market watchlist and configurable underprice sniper using completed-sale references.
// @match        https://pokeidle.io/app*
// @grant        unsafeWindow
// @updateURL    https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/market-bot.user.js
// @downloadURL  https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/market-bot.user.js
// @run-at       document-start
// ==/UserScript==

(() => {
    'use strict';

    if (new URLSearchParams(location.search).has('moth-controller')) return;

