using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

/// <summary>
/// Native userscript runtime for the game WebView2 instances.
///
/// This intentionally uses WebView2's document-created script facility instead
/// of pretending to be a browser-extension userscript manager. The injected
/// wrapper runs in the page context before page JavaScript, which preserves the
/// behavior required by addons that patch window/WebSocket/timers at startup.
///
/// The API surface follows the commonly used Greasemonkey/Tampermonkey/
/// Violentmonkey conventions. Violentmonkey is used as the compatibility
/// reference because it is an open-source WebExtension userscript manager.
/// </summary>
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

    /// <summary>
    /// Replaces the previously registered document-created scripts represented
    /// by existingScriptIds, then registers the current addon set.
    /// </summary>
    public async Task<IReadOnlyList<string>> InstallAsync(
        CoreWebView2 view,
        IEnumerable<string>? existingScriptIds = null)
    {
        if (existingScriptIds is not null)
        {
            foreach (var id in existingScriptIds.ToArray())
            {
                try { view.RemoveScriptToExecuteOnDocumentCreated(id); }
                catch { }
            }
        }

        var ids = new List<string>(_scripts.Count);
        foreach (var script in _scripts)
        {
            try
            {
                var runtime = BuildRuntimeScript(script);
                var id = await view.AddScriptToExecuteOnDocumentCreatedAsync(runtime);
                ids.Add(id);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine(
                    $"[IdleShell] userscript '{script.Name}' registration failed: {ex}");
            }
        }

        Console.Error.WriteLine(
            $"[IdleShell] native userscript runtime registered {ids.Count}/{_scripts.Count} script(s) for {view.Source}");
        return ids;
    }

    private void Load()
    {
        _scripts.Clear();

        if (!Directory.Exists(Folder))
            return;

        foreach (var file in Directory.EnumerateFiles(
                     Folder,
                     "*.user.js",
                     SearchOption.TopDirectoryOnly)
                 .OrderBy(Path.GetFileName, StringComparer.OrdinalIgnoreCase))
        {
            try
            {
                _scripts.Add(Parse(file));
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine(
                    $"[IdleShell] userscript {Path.GetFileName(file)} skipped: {ex.Message}");
            }
        }
    }

    private static Userscript Parse(string path)
    {
        var source = File.ReadAllText(path);
        var match = MetadataBlock.Match(source);
        if (!match.Success)
            throw new InvalidDataException("no ==UserScript== metadata block");

        var name = Path.GetFileNameWithoutExtension(path);
        var ns = "http://tampermonkey.net/";
        var runAt = "document-end";
        var injectInto = "page";
        var matches = new List<string>();
        var includes = new List<string>();
        var excludes = new List<string>();
        var excludeMatches = new List<string>();
        var requires = new List<string>();
        var resources = new Dictionary<string, string>(StringComparer.Ordinal);
        var connects = new List<string>();
        var grants = new List<string>();
        var noFrames = false;

        foreach (Match line in MetadataLine.Matches(match.Groups["body"].Value))
        {
            var key = line.Groups["key"].Value.ToLowerInvariant();
            var value = line.Groups["value"].Value.Trim();

            switch (key)
            {
                case "name":
                    if (value.Length > 0) name = value;
                    break;
                case "namespace":
                    ns = value;
                    break;
                case "match":
                    matches.Add(value);
                    break;
                case "include":
                    includes.Add(value);
                    break;
                case "exclude":
                    excludes.Add(value);
                    break;
                case "exclude-match":
                    excludeMatches.Add(value);
                    break;
                case "require":
                    requires.Add(value);
                    break;
                case "resource":
                {
                    var parts = value.Split((char[]?)null, 2, StringSplitOptions.RemoveEmptyEntries);
                    if (parts.Length == 2) resources[parts[0]] = parts[1];
                    break;
                }
                case "connect":
                    connects.Add(value);
                    break;
                case "grant":
                    grants.Add(value);
                    break;
                case "run-at":
                    runAt = value.ToLowerInvariant();
                    break;
                case "inject-into":
                    injectInto = value.ToLowerInvariant();
                    break;
                case "noframes":
                    noFrames = true;
                    break;
            }
        }

        if (matches.Count == 0 && includes.Count == 0)
            throw new InvalidDataException("no @match or @include metadata");

        return new Userscript(
            name,
            ns,
            source,
            matches,
            includes,
            excludes,
            excludeMatches,
            requires,
            resources,
            connects,
            NormalizeRunAt(runAt),
            NormalizeInjectInto(injectInto),
            noFrames,
            grants,
            path);
    }

    private static string BuildRuntimeScript(Userscript script)
    {
        var name = JsonSerializer.Serialize(script.Name);
        var ns = JsonSerializer.Serialize(script.Namespace);
        var source = JsonSerializer.Serialize(script.Source);
        var matches = JsonSerializer.Serialize(script.Matches);
        var includes = JsonSerializer.Serialize(script.Includes);
        var excludes = JsonSerializer.Serialize(script.Excludes);
        var excludeMatches = JsonSerializer.Serialize(script.ExcludeMatches);
        var grants = JsonSerializer.Serialize(script.Grants);
        var resources = JsonSerializer.Serialize(script.Resources);
        var connects = JsonSerializer.Serialize(script.Connects);
        var path = JsonSerializer.Serialize(script.Path);
        var runAt = JsonSerializer.Serialize(script.RunAt);
        var injectInto = JsonSerializer.Serialize(script.InjectInto);
        var noFrames = script.NoFrames ? "true" : "false";

        return $$"""
        (() => {
          'use strict';

          const __idleshellMeta = Object.freeze({
            name: {{name}},
            namespace: {{ns}},
            matches: {{matches}},
            includes: {{includes}},
            excludes: {{excludes}},
            excludeMatches: {{excludeMatches}},
            grants: {{grants}},
            resources: {{resources}},
            connects: {{connects}},
            runAt: {{runAt}},
            injectInto: {{injectInto}},
            noFrames: {{noFrames}},
            path: {{path}}
          });

          const __idleshellScriptSource = {{source}};

          const __idleshellEscapeRegexChar = (ch) => {
            switch (ch) {
              case '\\':
              case '.':
              case '+':
              case '?':
              case '^':
              case '$':
              case '(':
              case ')':
              case '[':
              case ']':
              case '{':
              case '}':
              case '|':
                return '\\' + ch;
              default:
                return ch;
            }
          };

          // Keep glob parsing dependency-free. This avoids embedding a fragile
          // character-class regex in the bootstrap runtime itself.
          const __idleshellGlobRegex = (pattern) => {
            let out = '^';
            for (const ch of String(pattern)) {
              if (ch === '*') out += '.*';
              else if (ch === '?') out += '.';
              else out += __idleshellEscapeRegexChar(ch);
            }
            return new RegExp(out + '$');
          };

          const __idleshellMatchPattern = (pattern, rawUrl) => {
            try {
              const url = new URL(rawUrl);
              if (pattern === '<all_urls>')
                return /^(https?|file|ftp):$/i.test(url.protocol);

              const m = String(pattern).match(/^([^:]+)://([^/]*)(\/.*)$/);
              if (!m) return false;

              const scheme = m[1].toLowerCase();
              const hostPattern = m[2].toLowerCase();
              const pathPattern = m[3];

              if (scheme !== '*' && scheme !== url.protocol.slice(0, -1).toLowerCase())
                return false;
              if (scheme === '*' && !/^(https?|file|ftp):$/i.test(url.protocol))
                return false;

              const hostname = url.host.toLowerCase();
              if (hostPattern !== '*') {
                if (hostPattern.startsWith('*.')) {
                  const suffix = hostPattern.slice(2);
                  if (hostname !== suffix && !hostname.endsWith('.' + suffix))
                    return false;
                } else if (hostname !== hostPattern) {
                  return false;
                }
              }

              return __idleshellGlobRegex(pathPattern).test(url.pathname + url.search + url.hash);
            } catch (_) {
              return false;
            }
          };

          const __idleshellMatches = (rawUrl) => {
            if (__idleshellMeta.noFrames && window.top !== window)
              return false;

            if (__idleshellMeta.excludeMatches.some(p => __idleshellMatchPattern(p, rawUrl)))
              return false;

            if (__idleshellMeta.excludes.some(p => __idleshellGlobRegex(p).test(rawUrl)))
              return false;

            const matches = __idleshellMeta.matches;
            if (matches.length && matches.some(p => __idleshellMatchPattern(p, rawUrl)))
              return true;

            const includes = __idleshellMeta.includes;
            return includes.length > 0 && includes.some(p => __idleshellGlobRegex(p).test(rawUrl));
          };

          if (!__idleshellMatches(location.href))
            return;

          const __idleshellHostWindow = window;
          const __idleshellMemory = new Map();
          const __idleshellStoragePrefix =
            '__idleshell_gm__' + __idleshellMeta.namespace + ':' + __idleshellMeta.name + ':';
          const __idleshellListeners = new Map();

          const __idleshellStorage = (() => {
            try {
              const s = window.localStorage;
              s.getItem(__idleshellStoragePrefix + '__probe__');
              return s;
            } catch (_) {
              return null;
            }
          })();

          const __idleshellRead = (key) => {
            const name = __idleshellStoragePrefix + String(key);
            try {
              const raw = __idleshellStorage?.getItem(name);
              if (raw !== null && raw !== undefined)
                return JSON.parse(raw);
            } catch (_) {}
            return __idleshellMemory.has(String(key))
              ? __idleshellMemory.get(String(key))
              : undefined;
          };

          const __idleshellWrite = (key, value, remote = false) => {
            const k = String(key);
            const oldValue = __idleshellRead(k);
            const normalized = value;
            try {
              if (__idleshellStorage)
                __idleshellStorage.setItem(
                  __idleshellStoragePrefix + k,
                  JSON.stringify(normalized));
              else
                __idleshellMemory.set(k, normalized);
            } catch (_) {
              __idleshellMemory.set(k, normalized);
            }

            for (const listener of __idleshellListeners.values()) {
              if (listener.key === k) {
                try { listener.callback(k, oldValue, normalized, remote); } catch (_) {}
              }
            }
          };

          const __idleshellDelete = (key, remote = false) => {
            const k = String(key);
            const oldValue = __idleshellRead(k);
            try {
              __idleshellStorage?.removeItem(__idleshellStoragePrefix + k);
            } catch (_) {}
            __idleshellMemory.delete(k);

            for (const listener of __idleshellListeners.values()) {
              if (listener.key === k) {
                try { listener.callback(k, oldValue, undefined, remote); } catch (_) {}
              }
            }
          };

          const __idleshellGetValue = (key, fallback) => {
            const value = __idleshellRead(key);
            return value === undefined ? fallback : value;
          };

          const __idleshellListValues = () => {
            const out = new Set(__idleshellMemory.keys());
            try {
              if (__idleshellStorage) {
                for (let i = 0; i < __idleshellStorage.length; i++) {
                  const key = __idleshellStorage.key(i);
                  if (key && key.startsWith(__idleshellStoragePrefix))
                    out.add(key.slice(__idleshellStoragePrefix.length));
                }
              }
            } catch (_) {}
            return [...out];
          };

          const __idleshellValueListener = (event) => {
            try {
              if (!event.key || !event.key.startsWith(__idleshellStoragePrefix))
                return;
              const key = event.key.slice(__idleshellStoragePrefix.length);
              const newValue = event.newValue === null ? undefined : JSON.parse(event.newValue);
              const oldValue = event.oldValue === null ? undefined : JSON.parse(event.oldValue);
              for (const listener of __idleshellListeners.values()) {
                if (listener.key === key) {
                  try { listener.callback(key, oldValue, newValue, true); } catch (_) {}
                }
              }
            } catch (_) {}
          };
          window.addEventListener('storage', __idleshellValueListener);

          const GM_getValue = (key, fallback) =>
            __idleshellGetValue(key, fallback);
          const GM_getValues = (keys) => {
            if (Array.isArray(keys))
              return Object.fromEntries(keys.map(key => [key, __idleshellGetValue(key, undefined)]));
            const source = keys && typeof keys === 'object' ? keys : {};
            return Object.fromEntries(Object.entries(source).map(([key, fallback]) =>
              [key, __idleshellGetValue(key, fallback)]));
          };
          const GM_setValue = (key, value) => __idleshellWrite(key, value);
          const GM_setValues = (values) => {
            for (const [key, value] of Object.entries(values || {}))
              __idleshellWrite(key, value);
          };
          const GM_deleteValue = (key) => __idleshellDelete(key);
          const GM_deleteValues = (keys) => {
            for (const key of keys || []) __idleshellDelete(key);
          };
          const GM_listValues = () => __idleshellListValues();

          const GM_addValueChangeListener = (key, callback) => {
            const id = Math.random().toString(36).slice(2);
            __idleshellListeners.set(id, { key: String(key), callback });
            return id;
          };
          const GM_removeValueChangeListener = (id) => {
            __idleshellListeners.delete(String(id));
          };

          const GM_addStyle = (css) => {
            try {
              const style = document.createElement('style');
              style.textContent = String(css);
              (document.head || document.documentElement).appendChild(style);
              return style;
            } catch (_) {
              return null;
            }
          };

          const GM_addElement = (...args) => {
            try {
              let parent = document.body || document.documentElement;
              let tag = args[0];
              let attrs = args[1];
              let text = args[2];

              if (args[0] && args[0].nodeType === 1) {
                parent = args[0];
                tag = args[1];
                attrs = args[2];
                text = args[3];
              }

              const element = document.createElement(String(tag));
              for (const [key, value] of Object.entries(attrs || {})) {
                try { element.setAttribute(key, String(value)); } catch (_) {}
              }
              if (text !== undefined) element.textContent = String(text);
              parent.appendChild(element);
              return element;
            } catch (_) {
              return null;
            }
          };

          const GM_setClipboard = async (data) => {
            try {
              await navigator.clipboard.writeText(String(data));
              return true;
            } catch (_) {
              try {
                const area = document.createElement('textarea');
                area.value = String(data);
                area.style.position = 'fixed';
                area.style.opacity = '0';
                document.documentElement.appendChild(area);
                area.select();
                const ok = document.execCommand('copy');
                area.remove();
                return ok;
              } catch (_) {
                return false;
              }
            }
          };

          const GM_openInTab = (urlOrUrl, optionsOrBackground) => {
            const url = String(urlOrUrl);
            const active =
              typeof optionsOrBackground === 'boolean'
                ? !optionsOrBackground
                : Boolean(optionsOrBackground?.active ?? true);

            const opened = { closed: false, onclose: null };

            try {
              // A successful IdleShell bridge owns the navigation. Do not then
              // fall through to window.open(), which would route the same stream
              // twice through the native popup interceptor.
              if (typeof __idleshellHostOpenLink === 'function' &&
                  __idleshellHostOpenLink(url, optionsOrBackground)) {
                opened.close = () => {
                  opened.closed = true;
                  try { opened.onclose?.(); } catch (_) {}
                };
                return opened;
              }
            } catch (_) {}

            const target = active ? '_blank' : '_blank';
            try {
              const real = window.open(url, target);
              opened.close = () => {
                try { real?.close(); } catch (_) {}
                opened.closed = true;
                try { opened.onclose?.(); } catch (_) {}
              };
            } catch (_) {
              opened.closed = true;
            }
            return opened;
          };

          const GM_notification = (details, ondone) => {
            const options = typeof details === 'string'
              ? { text: details }
              : (details || {});
            try {
              if (typeof Notification !== 'undefined' &&
                  Notification.permission !== 'denied') {
                const finish = () => {
                  try { ondone?.(); } catch (_) {}
                };
                const show = () => {
                  const n = new Notification(options.title || __idleshellMeta.name, {
                    body: options.text || '',
                    icon: options.image
                  });
                  n.onclick = options.onclick;
                  n.onclose = finish;
                  return n;
                };
                if (Notification.permission === 'granted') return show();
                Notification.requestPermission().then(permission => {
                  if (permission === 'granted') show();
                  finish();
                }).catch(finish);
              }
            } catch (_) {}
            return null;
          };

          const __idleshellRequest = (details = {}) => {
            let settled = false;
            let timeoutId = null;
            const controller =
              typeof AbortController !== 'undefined' ? new AbortController() : null;

            let resolvePromise;
            let rejectPromise;
            const promise = new Promise((resolve, reject) => {
              resolvePromise = resolve;
              rejectPromise = reject;
            });

            const finish = (callback, response) => {
              try { if (typeof callback === 'function') callback(response); } catch (_) {}
            };

            const request = {
              abort() {
                if (settled) return;
                settled = true;
                try { controller?.abort(); } catch (_) {}
                if (timeoutId) clearTimeout(timeoutId);
                finish(details.onabort, request);
                finish(details.onloadend, request);
                rejectPromise(request);
              }
            };
            Object.defineProperty(request, 'then', { value: promise.then.bind(promise) });
            Object.defineProperty(request, 'catch', { value: promise.catch.bind(promise) });
            Object.defineProperty(request, 'finally', { value: promise.finally.bind(promise) });

            const complete = async () => {
              try {
                const method = String(details.method || (details.data !== undefined ? 'POST' : 'GET')).toUpperCase();
                const headers = {};
                for (const [key, value] of Object.entries(details.headers || {}))
                  headers[key] = String(value);

                const url = new URL(String(details.url || ''), location.href).href;
                const response = await fetch(url, {
                  method,
                  headers,
                  body: details.data !== undefined ? details.data : undefined,
                  redirect: 'follow',
                  credentials: details.anonymous ? 'omit' : 'include',
                  mode: 'cors',
                  signal: controller?.signal
                });

                const responseType = String(details.responseType || 'text').toLowerCase();
                let body = null;
                let responseText;
                if (responseType === 'arraybuffer') {
                  body = await response.arrayBuffer();
                } else if (responseType === 'blob') {
                  body = await response.blob();
                } else if (responseType === 'json') {
                  responseText = await response.text();
                  try { body = JSON.parse(responseText); } catch (_) { body = null; }
                } else {
                  responseText = await response.text();
                  body = responseText;
                }

                const result = {
                  readyState: 4,
                  status: response.status,
                  statusText: response.statusText,
                  responseHeaders: [...response.headers.entries()]
                    .map(([k, v]) => k + ': ' + v)
                    .join('\r\n'),
                  responseText,
                  response: body,
                  responseXML: null,
                  finalUrl: response.url,
                  context: details.context,
                  lengthComputable: true,
                  loaded: typeof body === 'string' ? body.length : 0,
                  total: typeof body === 'string' ? body.length : 0
                };

                if (settled) return;
                settled = true;
                if (timeoutId) clearTimeout(timeoutId);

                if (response.ok) {
                  finish(details.onload, result);
                  resolvePromise(result);
                } else {
                  finish(details.onerror, result);
                  rejectPromise(result);
                }
                finish(details.onloadend, result);
              } catch (error) {
                if (settled) return;
                settled = true;
                if (timeoutId) clearTimeout(timeoutId);

                if (error?.name === 'AbortError') {
                  finish(details.onabort, request);
                  finish(details.ontimeout, request);
                } else {
                  const result = {
                    readyState: 4,
                    status: 0,
                    statusText: String(error?.name || 'error'),
                    responseText: '',
                    response: null,
                    responseXML: null,
                    finalUrl: String(details.url || ''),
                    context: details.context,
                    error
                  };
                  finish(details.onerror, result);
                }

                finish(details.onloadend, request);
                rejectPromise(error);
              }
            };

            if (details.timeout > 0)
              timeoutId = setTimeout(() => request.abort(), Number(details.timeout));

            complete();
            return request;
          };

          const GM_xmlhttpRequest = (details) => __idleshellRequest(details);

          const GM_download = (detailsOrUrl, filename) => {
            const details = typeof detailsOrUrl === 'string'
              ? { url: detailsOrUrl, name: filename }
              : (detailsOrUrl || {});
            try {
              const anchor = document.createElement('a');
              anchor.href = new URL(String(details.url), location.href).href;
              anchor.download = String(details.name || 'download');
              anchor.style.display = 'none';
              document.documentElement.appendChild(anchor);
              anchor.click();
              anchor.remove();
              return { abort() {} };
            } catch (_) {
              return null;
            }
          };

          const __idleshellMenu = new Map();
          let __idleshellMenuId = 0;
          const GM_registerMenuCommand = (caption, callback, options = {}) => {
            const id = String(options.id || (++__idleshellMenuId));
            __idleshellMenu.set(id, { id, caption: String(caption), callback, options });
            return id;
          };
          const GM_unregisterMenuCommand = (id) => {
            __idleshellMenu.delete(String(id));
          };

          const GM_getResourceURL = (name) => {
            const value = __idleshellMeta.resources?.[name];
            return value || null;
          };
          const GM_getResourceText = () => null;

          const GM_info = Object.freeze({
            script: Object.freeze({
              name: __idleshellMeta.name,
              namespace: __idleshellMeta.namespace,
              description: '',
              version: '0.0.0',
              matches: __idleshellMeta.matches,
              includes: __idleshellMeta.includes,
              excludes: __idleshellMeta.excludes,
              grant: __idleshellMeta.grants,
              noframes: __idleshellMeta.noFrames,
              runIn: __idleshellMeta.injectInto
            }),
            scriptMetaStr: '',
            injectInto: __idleshellMeta.injectInto,
            isIncognito: false,
            downloadMode: 'native'
          });

          const unsafeWindow = __idleshellHostWindow;

          const GM = Object.freeze({
            info: GM_info,
            getValue: async (key, fallback) => GM_getValue(key, fallback),
            getValues: async (keys) => GM_getValues(keys),
            setValue: async (key, value) => GM_setValue(key, value),
            setValues: async values => GM_setValues(values),
            deleteValue: async key => GM_deleteValue(key),
            deleteValues: async keys => GM_deleteValues(keys),
            listValues: async () => GM_listValues(),
            addValueChangeListener: async (key, callback) => GM_addValueChangeListener(key, callback),
            removeValueChangeListener: async id => GM_removeValueChangeListener(id),
            addStyle: css => GM_addStyle(css),
            addElement: (...args) => GM_addElement(...args),
            openInTab: (...args) => GM_openInTab(...args),
            registerMenuCommand: (...args) => GM_registerMenuCommand(...args),
            unregisterMenuCommand: (...args) => GM_unregisterMenuCommand(...args),
            notification: (...args) => GM_notification(...args),
            setClipboard: (...args) => GM_setClipboard(...args),
            xmlHttpRequest: (...args) => GM_xmlhttpRequest(...args),
            download: (...args) => GM_download(...args),
            getResourceText: (...args) => GM_getResourceText(...args),
            getResourceUrl: (...args) => GM_getResourceURL(...args)
          });

          const __idleshellOpenLink = __idleshellHostWindow.__idleshell_openLink;
          const __idleshellHostOpenLink = typeof __idleshellOpenLink === 'function'
            ? __idleshellOpenLink
            : null;

          const __idleshellExpose = () => {
            // The userscript's own globals are lexical to this wrapper, like a
            // real userscript sandbox. Page-facing APIs are exposed through
            // unsafeWindow, not by polluting window with GM_* names.
          };

          const __idleshellRun = () => {
            try {
              // Direct eval is intentional: the userscript must see the GM_*
              // bindings and GM metadata defined by this wrapper. Indirect eval
              // would execute in the global scope and make those bindings vanish.
              eval(__idleshellScriptSource);
              console.info('[IdleShell] userscript loaded:', __idleshellMeta.name);
            } catch (error) {
              console.error('[IdleShell] userscript failed:', __idleshellMeta.name, error);
            }
          };

          if (__idleshellMeta.runAt === 'document-start') {
            __idleshellRun();
          } else if (__idleshellMeta.runAt === 'document-idle') {
            const __idleshellIdle = () => {
              const run = () => __idleshellRun();
              if (typeof requestIdleCallback === 'function')
                requestIdleCallback(run, { timeout: 1000 });
              else
                setTimeout(run, 0);
            };
            if (document.readyState === 'complete') __idleshellIdle();
            else window.addEventListener('load', __idleshellIdle, { once: true });
          } else {
            const __idleshellEnd = () => setTimeout(__idleshellRun, 0);
            if (document.readyState === 'loading')
              document.addEventListener('DOMContentLoaded', __idleshellEnd, { once: true });
            else
              __idleshellEnd();
          }
        })();
        """;
    }

    private static string NormalizeRunAt(string value) =>
        value is "document-start" or "document-end" or "document-idle"
            ? value
            : "document-end";

    private static string NormalizeInjectInto(string value) =>
        value is "page" or "content" or "auto"
            ? value
            : "page";
}
