// ==UserScript==
// @name         Twitch Low Resource Mode
// @namespace    moth.pokeidle
// @version      1.0.0
// @description  Keeps Twitch streams at the lowest available quality and trims nonessential page rendering to reduce bandwidth, decode and UI overhead.
// @match        https://www.twitch.tv/*
// @match        https://www.twitch.tv/*/*
// @match        https://player.twitch.tv/*
// @match        https://m.twitch.tv/*
// @grant        none
// @run-at       document-idle
// @noframes
// ==/UserScript==

(() => {
    'use strict';

    const TARGET_QUALITY_RE = /^160p(?:\D|$)/i;
    const QUALITY_OPTION_SELECTOR =
        '[data-a-target="player-settings-menu"] input[type="radio"],' +
        '[data-a-target="player-settings-submenu-quality-option"]';

    let qualityBusy = false;
    let lastQualityAttempt = 0;
    let lastUrl = location.href;

    const sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));

    const textOf = (el) => {
        try {
            return String(el?.closest('label')?.textContent ||
                el?.parentElement?.textContent ||
                el?.textContent || '').trim();
        } catch (_) {
            return '';
        }
    };

    const isLowestOption = (el) => {
        const text = textOf(el);
        return TARGET_QUALITY_RE.test(text);
    };

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

        if (document.querySelector('[data-a-target="player-settings-menu"]')) {
            try {
                document.querySelector('[data-a-target="player-settings-button"]')?.click();
            } catch (_) {}
        }
    };

    const forceLowestQuality = async () => {
        if (qualityBusy) return;
        if (Date.now() - lastQualityAttempt < 2500) return;

        const gear = document.querySelector('[data-a-target="player-settings-button"]');
        if (!gear) return;

        const video = document.querySelector('video');
        if (!video) return;

        lastQualityAttempt = Date.now();
        qualityBusy = true;

        try {
            // Open the native Twitch quality menu.
            gear.click();
            await sleep(250);

            let qualityButton =
                document.querySelector('[data-a-target="player-settings-menu-item-quality"]');

            if (!qualityButton) {
                qualityButton = [...document.querySelectorAll(
                    '[data-a-target="player-settings-menu"] button'
                )].find(el => /quality/i.test(el.textContent || ''));
            }

            if (!qualityButton) {
                await closeQualityMenu();
                return;
            }

            qualityButton.click();
            await sleep(250);

            const options = [...document.querySelectorAll(QUALITY_OPTION_SELECTOR)];
            if (!options.length) {
                await closeQualityMenu();
                return;
            }

            // Prefer Twitch's explicit 160p option. Otherwise choose the
            // numerically smallest advertised resolution.
            let target = options.find(isLowestOption);

            if (!target) {
                const scored = options
                    .map((el, index) => {
                        const match = textOf(el).match(/(\d{3,4})p/i);
                        return { el, index, height: match ? Number(match[1]) : Number.MAX_SAFE_INTEGER };
                    })
                    .filter(x => Number.isFinite(x.height))
                    .sort((a, b) => a.height - b.height || a.index - b.index);

                target = scored[0]?.el;
            }

            if (!target) {
                await closeQualityMenu();
                return;
            }

            const label = target.closest('label') || target.parentElement || target;
            label.click();
            await sleep(250);

            console.info('[IdleShell] Twitch low-resource quality:', textOf(target));
        } catch (error) {
            console.warn('[IdleShell] Twitch low-resource quality failed:', error);
        } finally {
            await closeQualityMenu();
            qualityBusy = false;
        }
    };

    const addResourceSavingCss = () => {
        if (document.getElementById('idleshell-twitch-low-resource-css')) return;

        const style = document.createElement('style');
        style.id = 'idleshell-twitch-low-resource-css';
        style.textContent = [
            // Chat is not needed for drop farming and can be a sizeable DOM/render cost.
            '[data-a-target="chat-room-component-layout"] { display:none !important; }',
            // Hide the channel recommendations rail when Twitch exposes this container.
            '[data-a-target="side-nav"] { display:none !important; }',
            // Reduce visual work in areas outside the player without touching video controls.
            '[data-a-target="recommendations-container"],' +
            '[data-a-target="home-recommendations"] { content-visibility:auto !important; }'
         ].join('\n');

        (document.head || document.documentElement)?.appendChild(style);
    };

    const tick = () => {
        addResourceSavingCss();

        if (location.href !== lastUrl) {
            lastUrl = location.href;
            lastQualityAttempt = 0;
        }

        void forceLowestQuality();
    };

    addResourceSavingCss();
    void forceLowestQuality();

    // The 5-second watchdog is deliberate: observing Twitch's entire document
    // creates a callback for a very large number of unrelated DOM mutations.
    // Re-checking on a fixed interval is materially cheaper and still repairs
    // the player after SPA navigation, ads, and player recreation.
    setInterval(tick, 5000);

    console.info('[IdleShell] Twitch low-resource addon active');
})();
