// ==UserScript==
// @name         Twitch Low Resource Mode
// @namespace    moth.pokeidle
// @version      2.1.0
// @description  Keeps Twitch stream tabs at the lowest practical resource usage. Twitch chat pop-outs stay untouched.
// @match        https://www.twitch.tv/*
// @match        https://www.twitch.tv/*/*
// @match        https://player.twitch.tv/*
// @match        https://m.twitch.tv/*
// @grant        none
// @updateURL    https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/twitch-low-resource.user.js
// @downloadURL  https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/twitch-low-resource.user.js
// @run-at       document-idle
// @noframes
// ==/UserScript==

(() => {
    'use strict';

    const sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));

    const isTwitch = /(^|\.)twitch\.tv$/i.test(location.hostname);

    // Twitch popout chat is already the minimum viable Twitch surface.
    const isTwitchChatPopout =
        isTwitch && /\/popout\/[^/]+\/chat(?:[/?]|$)/i.test(location.pathname);

    if (isTwitchChatPopout) {
        console.info('[Moth] Twitch chat-only tab left untouched');
        return;
    }

    if (isTwitch) {
        runTwitch();
        return;
    }

    function textOf(el) {
        try {
            return String(
                el?.closest('label')?.textContent ||
                el?.getAttribute?.('aria-label') ||
                el?.parentElement?.textContent ||
                el?.textContent ||
                ''
            ).replace(/\s+/g, ' ').trim();
        } catch (_) {
            return '';
        }
    }

    function numericQuality(text) {
        const match = String(text || '').match(/(\d{3,4})p/i);
        return match ? Number(match[1]) : Number.MAX_SAFE_INTEGER;
    }

    function simulateClick(el) {
        if (!el) return;

        try {
            el.focus();
        } catch (_) {}

        for (const type of [
            'pointerover',
            'pointerenter',
            'pointerdown',
            'mousedown',
            'pointerup',
            'mouseup',
            'click'
        ]) {
            try {
                el.dispatchEvent(new PointerEvent(type, {
                    bubbles: true,
                    cancelable: true,
                    composed: true,
                    pointerId: 1,
                    pointerType: 'mouse',
                    isPrimary: true
                }));
            } catch (_) {
                try {
                    el.click();
                } catch (_) {}
            }
        }
    }

    function findMainVideo() {
        const videos = [...document.querySelectorAll('video')]
            .filter(video => {
                const rect = video.getBoundingClientRect();
                return rect.width > 0 && rect.height > 0;
            })
            .sort((a, b) => {
                const ar = a.getBoundingClientRect();
                const br = b.getBoundingClientRect();
                return (br.width * br.height) - (ar.width * ar.height);
            });

        return videos[0] || null;
    }

    function runTwitch() {
        const TARGET_QUALITY_RE = /^160p(?:\D|$)/i;
        const QUALITY_OPTION_SELECTOR =
            '[data-a-target="player-settings-menu"] input[type="radio"],' +
            '[data-a-target="player-settings-submenu-quality-option"]';

        let qualityBusy = false;
        let lastQualityAttempt = 0;
        let qualityConfiguredAt = 0;
        let lastUrl = location.href;

        const isLowestOption = (el) => TARGET_QUALITY_RE.test(textOf(el));

        const closeQualityMenu = async () => {
            try {
                document.dispatchEvent(new KeyboardEvent('keydown', {
                    key: 'Escape',
                    code: 'Escape',
                    bubbles: true,
                    cancelable: true
                }));
            } catch (_) {}
            await sleep(100);

            const menu = document.querySelector('[data-a-target="player-settings-menu"]');
            if (menu) {
                try {
                    document.querySelector('[data-a-target="player-settings-button"]')?.click();
                } catch (_) {}
            }
        };

        const forceLowestQuality = async () => {
            if (qualityBusy) return;
            if (Date.now() - qualityConfiguredAt < 60000) return;
            if (Date.now() - lastQualityAttempt < 15000) return;

            const gear =
                document.querySelector('[data-a-target="player-settings-button"]');

            const video = document.querySelector('video');

            if (!gear || !video) return;

            lastQualityAttempt = Date.now();
            qualityBusy = true;

            try {
                gear.click();
                await sleep(250);

                let qualityButton =
                    document.querySelector(
                        '[data-a-target="player-settings-menu-item-quality"]'
                    );

                if (!qualityButton) {
                    qualityButton = [...document.querySelectorAll(
                        '[data-a-target="player-settings-menu"] button'
                    )].find(el => /quality/i.test(el.textContent || ''));
                }

                if (!qualityButton) return;

                qualityButton.click();
                await sleep(250);

                const options = [...document.querySelectorAll(QUALITY_OPTION_SELECTOR)];

                if (!options.length) return;

                let target = options.find(isLowestOption);

                if (!target) {
                    const scored = options
                        .map((el, index) => ({
                            el,
                            index,
                            height: numericQuality(textOf(el))
                        }))
                        .filter(item => Number.isFinite(item.height))
                        .sort(
                            (a, b) =>
                                a.height - b.height ||
                                a.index - b.index
                        );

                    target = scored[0]?.el;
                }

                if (!target) return;

                (target.closest('label') || target.parentElement || target).click();
                await sleep(250);

                qualityConfiguredAt = Date.now();
                console.info('[Moth] Twitch low-resource quality:', textOf(target));
            } catch (error) {
                console.warn('[Moth] Twitch low-resource quality failed:', error);
            } finally {
                await closeQualityMenu();
                qualityBusy = false;
            }
        };

        const addTwitchCss = () => {
            if (document.getElementById('moth-twitch-low-resource-css')) return;

            const style = document.createElement('style');
            style.id = 'moth-twitch-low-resource-css';
            style.textContent = [
                '[data-a-target="chat-room-component-layout"] { display:none !important; }',
                '[data-a-target="side-nav"] { display:none !important; }',
                '[data-a-target="recommendations-container"],',
                '[data-a-target="home-recommendations"] { content-visibility:auto !important; }'
            ].join('\n');

            (document.head || document.documentElement)?.appendChild(style);
        };

        const tick = () => {
            addTwitchCss();

            if (location.href !== lastUrl) {
                lastUrl = location.href;
                lastQualityAttempt = 0;
                qualityConfiguredAt = 0;
            }

            void forceLowestQuality();
        };

        addTwitchCss();
        void forceLowestQuality();
        setInterval(tick, 15000);

        console.info('[Moth] Twitch low-resource mode active');
    }
})();
