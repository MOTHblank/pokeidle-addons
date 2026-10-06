using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

// Native userscript injector used when Tampermonkey is unavailable (or as a
// fallback alongside it). It parses each addons/*.user.js header block, and
// registers a document-start bootstrap per pane that decodes and evaluates
// every script whose @match/@include patterns cover the page URL.
//
// Scripts are evaluated inside the MAIN world wrapped in an IIFE, so they
// observe and hook the real page (the addons feature-detect `unsafeWindow`
// and fall back to `window`). A minimal GM_* compatibility surface is defined
// first: storage over localStorage, menu/no-op APIs, and a fetch-based
// GM_xmlhttpRequest shim.
internal sealed class UserscriptLoader
{
    private static readonly Regex MetadataBlock = new(
        @"//\s*==UserScript==\s*(?<body>.*?)//\s*==/UserScript==",
        RegexOptions.Singleline | RegexOptions.Compiled);

    private static readonly Regex MetadataLine = new(
        @"^\s*//\s*@(?<key>[A-Za-z][A-Za-z0-9_-]*)\s+(?<value>.+?)\s*$",
        RegexOptions.Multiline | RegexOptions.Compiled);

    private readonly List<Userscript> _scripts = [];

    public UserscriptLoader(string addonsFolder)
    {
        Folder = Path.GetFullPath(addonsFolder);

        Load();
    }

    public string Folder { get; }

    public IReadOnlyList<Userscript> Scripts => _scripts;

    // Registers (or re-registers) the injection bootstrap on a pane's view.
    // Safe to call repeatedly; AddScriptToExecuteOnDocumentCreatedAsync
    // persists for the lifetime of the CoreWebView2 instance.
    public async Task AttachAsync(CoreWebView2 view)
    {
        var bootstrap = BuildBootstrap();

        await view.AddScriptToExecuteOnDocumentCreatedAsync(bootstrap);
    }

    private void Load()
    {
        _scripts.Clear();

        if (!Directory.Exists(Folder))
            return;

        var files = Directory
            .EnumerateFiles(Folder, "*.user.js", SearchOption.TopDirectoryOnly)
            .OrderBy(Path.GetFileName, StringComparer.OrdinalIgnoreCase);

        foreach (var file in files)
        {
            try
            {
                _scripts.Add(Parse(file));
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine(
                    $"[IdleShell] userscript {Path.GetFileName(file)} " +
                    $"skipped: {ex.Message}");
            }
        }
    }

    private static Userscript Parse(string path)
    {
        var source = File.ReadAllText(path);

        var body = MetadataBlock.Match(source) is { Success: true } m
            ? m.Groups["body"].Value
            : throw new InvalidDataException(
                "no ==UserScript== metadata block");

        var name = Path.GetFileNameWithoutExtension(path);
        var runAt = "document-end";
        var matches = new List<string>();
        var includes = new List<string>();
        var grants = new List<string>();

        foreach (Match line in MetadataLine.Matches(body))
        {
            var key = line.Groups["key"].Value.ToLowerInvariant();
            var value = line.Groups["value"].Value.Trim();

            switch (key)
            {
                case "name":
                    if (value.Length > 0) name = value;
                    break;
                case "match":
                    matches.Add(value);
                    break;
                case "include":
                    includes.Add(value);
                    break;
                case "grant":
                    grants.Add(value);
                    break;
                case "run-at":
                    runAt = value.ToLowerInvariant();
                    break;
            }
        }

        if (matches.Count == 0 && includes.Count == 0)
        {
            throw new InvalidDataException(
                "no @match or @include metadata");
        }

        return new Userscript(name, source, matches, runAt, path)
        {
            Includes = includes,
            Grants = grants
        };
    }

    private string BuildBootstrap()
    {
        var payload = _scripts.Select(script => new InjectedScript(
            script.Name,
            script.Matches,
            script.Includes,
            Convert.ToBase64String(
                Encoding.UTF8.GetBytes(script.Source))));

        var json = JsonSerializer.Serialize(payload);

        // The generated JS must not contain a literal </script> sequence.
        json = json.Replace("</", "<\\/");

        return $$"""
            (() => {
              'use strict';
              const SCRIPTS = {{json}};
              const NS = '__idleshell_native_' + Math.random().toString(36).slice(2);

              try { Object.defineProperty(window, NS, { value: window }); } catch (e) {}

              function idleshellMatch(pattern, url) {
                try {
                  const prefix = pattern.split('*')[0];
                  const u = new URL(url);
                  const candidate = u.origin + u.pathname;
                  if (!candidate.startsWith(prefix)) return false;
                  if (pattern.endsWith('*')) return true;
                  const last = pattern[pattern.length - 1];
                  if ('/?=&#'.includes(last)) return true;
                  return candidate === pattern || u.href === pattern;
                } catch (e) { return false; }
              }

              function idleshellGM() {
                const storeMem = new Map();
                const raw = (k) => 'userscript.' + k;
                let ls = null;
                try { ls = window.localStorage; ls.getItem('__probe__'); }
                catch (e) { ls = null; }
                const getItem = (k) => {
                  try { return ls ? ls.getItem(raw(k)) : (storeMem.has(k) ? storeMem.get(k) : null); }
                  catch (e) { return storeMem.has(k) ? storeMem.get(k) : null; }
                };
                const setItem = (k, v) => {
                  v = String(v);
                  try { if (ls) ls.setItem(raw(k), v); else storeMem.set(k, v); }
                  catch (e) { storeMem.set(k, v); }
                };
                const delItem = (k) => {
                  try { if (ls) ls.removeItem(raw(k)); else storeMem.delete(k); }
                  catch (e) { storeMem.delete(k); }
                };
                const noop = () => {};
                const api = {
                  GM_getValue: (k, d) => {
                    const v = getItem(k);
                    if (v === null) return d;
                    try { return JSON.parse(v); } catch (e) { return v; }
                  },
                  GM_setValue: (k, v) => setItem(k, JSON.stringify(v)),
                  GM_deleteValue: (k) => delItem(k),
                  GM_listValues: () => {
                    try {
                      if (!ls) return [...storeMem.keys()];
                      const out = [];
                      for (let i = 0; i < ls.length; i++) {
                        const key = ls.key(i);
                        if (key && key.startsWith('userscript.')) out.push(key.slice(11));
                      }
                      return out;
                    } catch (e) { return [...storeMem.keys()]; }
                  },
                  GM_getResourceText: () => null,
                  GM_getResourceURL: () => null,
                  GM_addStyle: (css) => {
                    try {
                      const s = document.createElement('style');
                      s.textContent = css;
                      (document.head || document.documentElement).appendChild(s);
                      return s;
                    } catch (e) { return null; }
                  },
                  GM_registerMenuCommand: noop,
                  GM_unregisterMenuCommand: noop,
                  GM_setClipboard: (text) => {
                    try { navigator.clipboard.writeText(String(text)); } catch (e) {}
                  },
                  GM_openInTab: (url) => {
                    try { window.open(url, '_blank'); } catch (e) {}
                  },
                  GM_notification: noop,
                  unsafeWindow: window
                };
                api.GM_xmlhttpRequest = (details) => {
                  details = details || {};
                  const done = (fn, arg) => { try { if (typeof fn === 'function') fn(arg); } catch (e) {} };
                  const headers = {};
                  try {
                    (String(details.headers || '') ? [] : Object.entries(details.headers || {}))
                      .forEach(([k, v]) => { headers[k] = String(v); });
                  } catch (e) {}
                  const controller = typeof AbortController !== 'undefined' ? new AbortController() : null;
                  const request = {
                    abort: () => { try { controller && controller.abort(); } catch (e) {} }
                  };
                  done(details.onabort, request);
                  fetch(details.url, {
                    method: details.method || (details.data !== undefined ? 'POST' : 'GET'),
                    headers,
                    body: details.data !== undefined ? details.data : undefined,
                    redirect: 'follow',
                    credentials: details.anonymous ? 'omit' : 'same-origin',
                    signal: controller ? controller.signal : undefined
                  }).then(async (res) => {
                    const text = await res.text();
                    const xml = (() => {
                      try {
                        return /xml/i.test(res.headers.get('content-type') || '') ||
                               /^\s*<\?xml/.test(text)
                          ? new DOMParser().parseFromString(text, 'text/xml') : null;
                      } catch (e) { return null; }
                    })();
                    const resp = {
                      readyState: 4, status: res.status, statusText: res.statusText,
                      responseHeaders: (() => {
                        let out = '';
                        try { res.headers.forEach((v, k) => { out += k + ': ' + v + '\r\n'; }); } catch (e) {}
                        return out;
                      })(),
                      responseText: text, response: text, responseXML: xml,
                      finalUrl: res.url, context: details.context
                    };
                    if (res.ok) done(details.onload, resp);
                    else done(details.onerror, { ...resp, status: res.status });
                    done(details.onloadend, resp);
                  }).catch((err) => {
                    done(details.onerror, {
                      readyState: 4, status: 0, statusText: String(err && err.name || 'error'),
                      responseText: '', response: null, responseXML: null,
                      finalUrl: details.url, context: details.context, error: err
                    });
                    done(details.onloadend, null);
                  });
                  return request;
                };
                return api;
              }

              function idleshellInstall() {
                const gm = idleshellGM();
                for (const key of Object.keys(gm)) {
                  try {
                    if (window[key] === undefined) {
                      Object.defineProperty(window, key,
                        { value: gm[key], configurable: true, writable: true });
                    }
                  } catch (e) {}
                }
                for (const entry of SCRIPTS) {
                  let hit = entry.matches.some((p) => idleshellMatch(p, location.href));
                  if (!hit) hit = entry.includes.some((p) => {
                    try { return location.href.includes(new RegExp(p.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).source.replace(/\\\*/g, '.*')); }
                    catch (e) { return location.href.includes(p); }
                  });
                  if (!hit) continue;
                  try {
                    const src = decodeURIComponent(escape(atob(entry.code)));
                    const fn = new Function('"use strict";\n' + src +
                      '\n//# sourceURL=idleshell://' + encodeURIComponent(entry.name) + '.user.js');
                    fn.call(window);
                  } catch (err) {
                    try {
                      console.error('[IdleShell] userscript failed: ' + entry.name, err);
                    } catch (e) {}
                  }
                }
              }

              try { idleshellInstall(); } catch (e) {
                try { console.error('[IdleShell] userscript bootstrap failed', e); } catch (x) {}
              }
            })();
            """;
    }

    private sealed record InjectedScript(
        string Name,
        IReadOnlyList<string> Matches,
        IReadOnlyList<string> Includes,
        string Code);
}
