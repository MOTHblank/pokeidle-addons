using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

// Lightweight userscript host for this repository's *.user.js addons.
// Supports the metadata and GM APIs currently used by the project.
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

    public async Task AttachAsync(CoreWebView2 view)
    {
        if (_scripts.Count == 0) return;
        await view.AddScriptToExecuteOnDocumentCreatedAsync(BuildBootstrap());
    }

    private void Load()
    {
        _scripts.Clear();
        if (!Directory.Exists(Folder)) return;

        foreach (var file in Directory.EnumerateFiles(Folder, "*.user.js", SearchOption.TopDirectoryOnly)
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
        var body = MetadataBlock.Match(source) is { Success: true } m
            ? m.Groups["body"].Value
            : throw new InvalidDataException("no ==UserScript== metadata block");

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
                case "name": if (value.Length > 0) name = value; break;
                case "match": matches.Add(value); break;
                case "include": includes.Add(value); break;
                case "grant": grants.Add(value); break;
                case "run-at": runAt = value.ToLowerInvariant(); break;
            }
        }

        if (matches.Count == 0 && includes.Count == 0)
            throw new InvalidDataException("no @match or @include metadata");

        return new Userscript(name, source, matches, NormalizeRunAt(runAt), path)
        {
            Includes = includes,
            Grants = grants
        };
    }

    private string BuildBootstrap()
    {
        var payload = JsonSerializer.Serialize(_scripts.Select(script => new InjectedScript(
            script.Name,
            script.Matches,
            script.Includes,
            script.RunAt,
            script.Grants,
            Convert.ToBase64String(Encoding.UTF8.GetBytes(script.Source)))));

        payload = payload.Replace("</", "<\/");
        return $$"""
            (() => {
              'use strict';
              const SCRIPTS = {{payload}};

              const escapeRegex = (value) => String(value)
                .replaceAll("\\", "\\\\")
                .replaceAll(".", "\\.")
                .replaceAll("+", "\\+")
                .replaceAll("?", "\\?")
                .replaceAll("^", "\\^")
                .replaceAll("|", "\\|")
                .replaceAll("(", "\\(")
                .replaceAll(")", "\\)")
                .replaceAll("[", "\\[")
                .replaceAll("]", "\\]");

              const wildcardRegex = (pattern) =>
                new RegExp("^" + escapeRegex(pattern).split("*").join(".*") + "$", "i");

              const matchPattern = (pattern, url) => {
                try {
                  const p = String(pattern).trim();
                  const u = new URL(url);
                  const parts = p.match(/^([^:]+):\/\/([^/]+)(\/.*)?$/);
                  if (!parts) return false;
                  const protocol = parts[1] === "*" ? "[a-z]+" : escapeRegex(parts[1]);
                  const host = parts[2].split("*").map(escapeRegex).join("[^/]*");
                  const path = (parts[3] || "").split("*").map(escapeRegex).join(".*");
                  return new RegExp("^" + protocol + "://" + host + path + "$", "i").test(u.href);
                } catch (_) {
                  return false;
                }
              };

              const includePattern = (pattern, url) => {
                try { return wildcardRegex(pattern).test(url); }
                catch (_) { return false; }
              };

              const installApi = (entry) => {
                const memory = new Map();
                const prefix = "userscript:" + entry.name + ":";
                let ls = null;
                try { ls = window.localStorage; ls.getItem("__idleshell_probe__"); } catch (_) {}

                const read = (key) => {
                  try {
                    return ls ? ls.getItem(prefix + key) :
                      (memory.has(key) ? memory.get(key) : null);
                  } catch (_) {
                    return memory.get(key) ?? null;
                  }
                };

                const write = (key, value) => {
                  const text = String(value);
                  try {
                    if (ls) ls.setItem(prefix + key, text);
                    else memory.set(key, text);
                  } catch (_) {
                    memory.set(key, text);
                  }
                };

                const remove = (key) => {
                  try {
                    if (ls) ls.removeItem(prefix + key);
                    else memory.delete(key);
                  } catch (_) {
                    memory.delete(key);
                  }
                };

                const api = {
                  GM_getValue: (key, fallback) => {
                    const value = read(String(key));
                    if (value === null) return fallback;
                    try { return JSON.parse(value); } catch (_) { return value; }
                  },
                  GM_setValue: (key, value) => write(String(key), JSON.stringify(value)),
                  GM_deleteValue: (key) => remove(String(key)),
                  GM_listValues: () => {
                    try {
                      if (!ls) return [...memory.keys()];
                      const out = [];
                      for (let i = 0; i < ls.length; i++) {
                        const key = ls.key(i);
                        if (key && key.startsWith(prefix)) out.push(key.slice(prefix.length));
                      }
                      return out;
                    } catch (_) {
                      return [...memory.keys()];
                    }
                  },
                  GM_addStyle: (css) => {
                    try {
                      const style = document.createElement("style");
                      style.textContent = String(css);
                      (document.head || document.documentElement).appendChild(style);
                      return style;
                    } catch (_) {
                      return null;
                    }
                  },
                  GM_registerMenuCommand: () => null,
                  GM_unregisterMenuCommand: () => null,
                  GM_notification: () => null,
                  GM_setClipboard: (value) => {
                    try { return navigator.clipboard?.writeText(String(value)); }
                    catch (_) { return null; }
                  },
                  GM_openInTab: (url) => {
                    try { return window.open(String(url), "_blank"); }
                    catch (_) { return null; }
                  },
                  GM_getResourceText: () => null,
                  GM_getResourceURL: () => null,
                  unsafeWindow: window
                };

                api.GM_xmlhttpRequest = (details = {}) => {
                  let settled = false;
                  const controller = typeof AbortController !== "undefined"
                    ? new AbortController()
                    : null;
                  const finish = (callback, value) => {
                    try {
                      if (typeof callback === "function") callback(value);
                    } catch (_) {}
                  };

                  const headers = {};
                  try {
                    if (details.headers && typeof details.headers === "object") {
                      for (const [key, value] of Object.entries(details.headers))
                        headers[key] = String(value);
                    }
                  } catch (_) {}

                  const request = {
                    abort() {
                      if (settled) return;
                      settled = true;
                      try { controller?.abort(); } catch (_) {}
                      finish(details.onabort, request);
                      finish(details.onloadend, request);
                    }
                  };

                  const timeoutId = details.timeout > 0
                    ? setTimeout(() => request.abort(), Number(details.timeout))
                    : null;

                  fetch(String(details.url), {
                    method: String(details.method ||
                      (details.data !== undefined ? "POST" : "GET")).toUpperCase(),
                    headers,
                    body: details.data !== undefined ? details.data : undefined,
                    redirect: "follow",
                    credentials: "omit",
                    mode: "cors",
                    signal: controller?.signal
                  }).then(async (response) => {
                    if (settled) return;

                    const text = await response.text();
                    let parsed = text;
                    if (details.responseType === "json") {
                      try { parsed = JSON.parse(text); } catch (_) {}
                    }

                    const responseHeaders = [];
                    try {
                      response.headers.forEach((value, key) =>
                        responseHeaders.push(key + ": " + value));
                    } catch (_) {}

                    const result = {
                      readyState: 4,
                      status: response.status,
                      statusText: response.statusText,
                      responseHeaders: responseHeaders.join("\r\n"),
                      responseText: text,
                      response: parsed,
                      responseXML: null,
                      finalUrl: response.url,
                      context: details.context
                    };

                    settled = true;
                    if (timeoutId) clearTimeout(timeoutId);
                    if (response.ok) finish(details.onload, result);
                    else finish(details.onerror, result);
                    finish(details.onloadend, result);
                  }).catch((error) => {
                    if (settled) return;

                    settled = true;
                    if (timeoutId) clearTimeout(timeoutId);
                    if (error?.name === "AbortError")
                      finish(details.onabort, request);
                    else
                      finish(details.onerror, {
                        readyState: 4,
                        status: 0,
                        statusText: String(error?.name || "error"),
                        responseText: "",
                        response: null,
                        responseXML: null,
                        finalUrl: String(details.url || ""),
                        context: details.context,
                        error
                      });
                    finish(details.onloadend, request);
                  });

                  return request;
                };

                for (const [key, value] of Object.entries(api)) {
                  try {
                    if (window[key] === undefined) {
                      Object.defineProperty(window, key, {
                        value,
                        configurable: true,
                        writable: true
                      });
                    }
                  } catch (_) {}
                }
              };

              const execute = (entry) => {
                const hitMatch = entry.matches.some(p => matchPattern(p, location.href));
                const hitInclude = !hitMatch && entry.includes.some(p => includePattern(p, location.href));
                if (!hitMatch && !hitInclude) return;

                const run = () => {
                  try {
                    installApi(entry);
                    const source = decodeURIComponent(escape(atob(entry.code)));
                    const fn = new Function('"use strict";\n' + source +
                      '\n//# sourceURL=idleshell://' +
                      encodeURIComponent(entry.name) + '.user.js');
                    fn.call(window);
                  } catch (error) {
                    try {
                      console.error("[IdleShell] userscript failed: " + entry.name, error);
                    } catch (_) {}
                  }
                };

                switch (entry.runAt) {
                  case "document-start":
                    run();
                    break;
                  case "document-idle":
                    if (document.readyState === "complete")
                      setTimeout(run, 0);
                    else
                      window.addEventListener("load", () => setTimeout(run, 0), { once: true });
                    break;
                  default:
                    if (document.readyState === "loading")
                      document.addEventListener("DOMContentLoaded", run, { once: true });
                    else
                      queueMicrotask(run);
                    break;
                }
              };

              for (const entry of SCRIPTS) execute(entry);
            })();
            """;
    }

    private static string NormalizeRunAt(string value) =>
        value is "document-start" or "document-end" or "document-idle"
            ? value
            : "document-end";

    private sealed record InjectedScript(
        string Name,
        IReadOnlyList<string> Matches,
        IReadOnlyList<string> Includes,
        string RunAt,
        IReadOnlyList<string> Grants,
        string Code);
}
