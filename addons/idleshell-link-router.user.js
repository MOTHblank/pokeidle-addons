// ==UserScript==
// @name         IdleShell Link Router (pokeidle.io)
// @namespace    moth.pokeidle
// @version      1.0.0
// @description  Intercepts twitch.tv / kick.com links inside the PokéIdle game page and hands them to the IdleShell host, which opens them in background stream panes on every Twitch/Kick account profile instead of a popup window.
// @match        https://pokeidle.io/*
// @match        https://www.pokeidle.io/*
// @grant        unsafeWindow
// @run-at       document-start
// ==/UserScript==

/*
 * This script runs inside the GAME pane (profile AccountA / AccountB).
 * The shell injects a small bootstrap into every isolated world via
 * AddScriptToExecuteOnDocumentCreatedAsync, which defines:
 *
 *     window.__idleshell_openLink   — routes a URL to the shell host
 *     window.__idleshell_hostInfo   — { title, profile } of this pane
 *
 * If those are missing (plain browser), everything below is inert.
 *
 * Strategy: never let a stream URL reach window.open() for real — WebView2
 * would raise NewWindowRequested and spawn an uncontrollable OS popup.
 * Instead we route the URL to the shell over window.chrome.webview.postMessage,
 * and the shell opens (or reuses) hidden Background-mode stream panes, one per
 * configured Twitch/Kick login profile.
 */

(() => {
    'use strict';

    const HOST = typeof unsafeWindow !== 'undefined' ? unsafeWindow : window;

    // Same hosts the shell considers "stream" URLs (StreamHostPattern in AppConfig.cs).
    const STREAM_URL_RE =
        /^https?:\/\/(?:www\.|m\.)?(?:twitch\.tv|kick\.com)\/[^\s"'<>]*/i;

    const post = (payload) => {
        try {
            if (HOST.chrome && HOST.chrome.webview &&
                typeof HOST.chrome.webview.postMessage === 'function') {
                HOST.chrome.webview.postMessage(JSON.stringify(payload));
                return true;
            }
        } catch (_) { /* not hosted by IdleShell */ }
        return false;
    };

    const info = () => {
        try {
            return HOST.__idleshell_hostInfo || { title: location.host, profile: 'unknown' };
        } catch (_) {
            return { title: location.host, profile: 'unknown' };
        }
    };

    const route = (rawUrl, source) => {
        if (typeof rawUrl !== 'string' || !STREAM_URL_RE.test(rawUrl)) return false;
        const url = new URL(rawUrl, location.href).href;
        const h = info();
        // Only claim the event when the host actually received it. If the
        // WebView2 bridge is unavailable (for example, when this userscript
        // executes in a context that cannot see chrome.webview), return false
        // so the browser/native WebView2 fallback can handle the popup or
        // navigation normally.
        return post({
            type: 'link',
            url,
            source,
            pane: h.title,
            profile: h.profile
        });
    };

    // 1. window.open(...) calls from the game or other userscripts.
    const realOpen = HOST.open ? HOST.open.bind(HOST) : null;
    const patchedOpen = function (url, ...rest) {
        if (route(url, 'window.open')) {
            // The host owns the stream URL only after post() succeeds.
            return { closed: false, close() {}, focus() {}, blur() {}, location: { href: String(url) } };
        }
        return realOpen ? realOpen(url, ...rest) : undefined;
    };
    try { HOST.open = patchedOpen; } catch (_) {}
    try { window.open = patchedOpen; } catch (_) {}

    // 2. Anchor clicks (target=_blank or plain) anywhere in the page.
    //    Capture phase + stopPropagation so the game's own handlers don't
    //    double-handle the same click. We only suppress the click if the host
    //    accepted the route; otherwise native WebView2 popup/navigation
    //    handling remains available as the fallback.
    const clickHandler = (e) => {
        const a = e.target && e.target.closest ? e.target.closest('a[href]') : null;
        if (!a) return;
        if (route(a.href, 'anchor-click')) {
            e.preventDefault();
            e.stopPropagation();
        }
    };
    const bindClicks = () => document.addEventListener('click', clickHandler, true);
    if (document.documentElement) bindClicks();
    else document.addEventListener('DOMContentLoaded', bindClicks, true);

    // 3. Direct assignments like `location.href = 'https://twitch.tv/...'`
    //    cannot be intercepted without breaking navigation generally, but the
    //    shell also watches each game pane's Source property as a backstop
    //    (see MainForm.GamePaneNavigated).
})();
