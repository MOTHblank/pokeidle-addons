using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

// Builds a tiny unpacked MV3 extension from the repository's ordinary *.user.js
// files. This is deliberately not Tampermonkey and never requires a CRX.
//
// The generated content scripts run in the MAIN world at the userscript's
// requested run_at, which is important for addons that patch WebSocket,
// window.open, timers, or other page-owned globals.
internal sealed class UserscriptLoader
{
    private static readonly Regex MetadataBlock = new(
        @"//\s*==UserScript==\s*(?<body>.*?)//\s*==/UserScript==",
        RegexOptions.Singleline | RegexOptions.Compiled);

    private static readonly Regex MetadataLine = new(
        @"^\s*//\s*@(?<key>[A-Za-z][A-Za-z0-9_-]*)\s+(?<value>.+?)\s*$",
        RegexOptions.Multiline | RegexOptions.Compiled);

    private readonly List<Userscript> _scripts = [];
    private readonly HashSet<string> _installedProfiles = new(StringComparer.OrdinalIgnoreCase);

    public UserscriptLoader(string addonsFolder)
    {
        Folder = Path.GetFullPath(addonsFolder);
        Load();
    }

    public string Folder { get; }
    public IReadOnlyList<Userscript> Scripts => _scripts;

    public async Task<CoreWebView2BrowserExtension?> InstallAsync(CoreWebView2Profile profile)
    {
        if (_scripts.Count == 0)
            return null;

        var profileKey = profile.ProfilePath;
        if (_installedProfiles.Contains(profileKey))
            return null;

        var extensions = await profile.GetBrowserExtensionsAsync();
        foreach (var extension in extensions.Where(e =>
                     e.Name.StartsWith(ExtensionNamePrefix, StringComparison.Ordinal)))
        {
            try { await extension.RemoveAsync(); }
            catch { /* stale extension cleanup is best-effort */ }
        }

        var extensionFolder = BuildExtensionFolder();
        var installed = await profile.AddBrowserExtensionAsync(extensionFolder);
        _installedProfiles.Add(profileKey);
        return installed;
    }

    private const string ExtensionNamePrefix = "PokéIdle Idle Shell Userscripts";

    private void Load()
    {
        _scripts.Clear();
        if (!Directory.Exists(Folder))
            return;

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
            throw new InvalidDataException("no @match or @include metadata");

        return new Userscript(name, source, matches, NormalizeRunAt(runAt), path)
        {
            Includes = includes,
            Grants = grants
        };
    }

    private string BuildExtensionFolder()
    {
        var fingerprintInput = string.Join(
            "\n",
            _scripts.Select(s => s.Name + "\n" + s.RunAt + "\n" + s.Source));
        var hash = Convert.ToHexString(
            SHA256.HashData(Encoding.UTF8.GetBytes(fingerprintInput))).ToLowerInvariant();

        var root = Path.Combine(AppConfig.Root, "UserscriptExtension", hash);
        Directory.CreateDirectory(root);

        var contentScripts = new List<ManifestContentScript>();

        for (var i = 0; i < _scripts.Count; i++)
        {
            var script = _scripts[i];
            var fileName = $"script-{i:D3}.js";
            var path = Path.Combine(root, fileName);

            File.WriteAllText(path, BuildScriptWrapper(script), new UTF8Encoding(false));

            contentScripts.Add(new ManifestContentScript
            {
                Matches = script.Matches.Count > 0 ? script.Matches : ["https://*/*"],
                IncludeGlobs = script.Includes.Count > 0 ? script.Includes : null,
                Js = [fileName],
                RunAt = ToManifestRunAt(script.RunAt),
                World = "MAIN",
                AllFrames = false
            });
        }

        var manifest = new
        {
            manifest_version = 3,
            name = "PokéIdle Idle Shell Userscripts",
            version = "1.0.0",
            description = "Locally generated userscript host for PokéIdle Idle Shell.",
            content_scripts = contentScripts
        };

        var manifestJson = JsonSerializer.Serialize(
            manifest,
            new JsonSerializerOptions { WriteIndented = true });

        File.WriteAllText(
            Path.Combine(root, "manifest.json"),
            manifestJson,
            new UTF8Encoding(false));

        return root;
    }

    private static string BuildScriptWrapper(Userscript script)
    {
        var scriptName = JsonSerializer.Serialize(script.Name);
        var source = script.Source;

        // The wrapper is deliberately an outer lexical scope. Each userscript
        // gets its own GM_* bindings even though all scripts share the page MAIN
        // world, so asynchronous callbacks never accidentally use another
        // script's storage namespace.
        return $$"""
            (() => {
              'use strict';

              const __idleshellScriptName = {{scriptName}};
              const __idleshellStoragePrefix =
                'idleshell.userscript.' + __idleshellScriptName + '.';
              const __idleshellMemory = new Map();

              const __idleshellStorage = (() => {
                try {
                  const storage = window.localStorage;
                  storage.getItem('__idleshell_probe__');
                  return storage;
                } catch (_) {
                  return null;
                }
              })();

              const __idleshellRead = (key) => {
                try {
                  return __idleshellStorage
                    ? __idleshellStorage.getItem(__idleshellStoragePrefix + key)
                    : (__idleshellMemory.has(key)
                      ? __idleshellMemory.get(key)
                      : null);
                } catch (_) {
                  return __idleshellMemory.has(key)
                    ? __idleshellMemory.get(key)
                    : null;
                }
              };

              const __idleshellWrite = (key, value) => {
                const text = String(value);
                try {
                  if (__idleshellStorage)
                    __idleshellStorage.setItem(__idleshellStoragePrefix + key, text);
                  else
                    __idleshellMemory.set(key, text);
                } catch (_) {
                  __idleshellMemory.set(key, text);
                }
              };

              const __idleshellDelete = (key) => {
                try {
                  if (__idleshellStorage)
                    __idleshellStorage.removeItem(__idleshellStoragePrefix + key);
                  else
                    __idleshellMemory.delete(key);
                } catch (_) {
                  __idleshellMemory.delete(key);
                }
              };

              const GM_getValue = (key, fallback) => {
                const value = __idleshellRead(String(key));
                if (value === null) return fallback;
                try { return JSON.parse(value); } catch (_) { return value; }
              };

              const GM_setValue = (key, value) =>
                __idleshellWrite(String(key), JSON.stringify(value));

              const GM_deleteValue = (key) =>
                __idleshellDelete(String(key));

              const GM_listValues = () => {
                try {
                  if (!__idleshellStorage)
                    return [...__idleshellMemory.keys()];

                  const out = [];
                  for (let i = 0; i < __idleshellStorage.length; i++) {
                    const key = __idleshellStorage.key(i);
                    if (key && key.startsWith(__idleshellStoragePrefix))
                      out.push(key.slice(__idleshellStoragePrefix.length));
                  }
                  return out;
                } catch (_) {
                  return [...__idleshellMemory.keys()];
                }
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

              const GM_registerMenuCommand = () => null;
              const GM_unregisterMenuCommand = () => null;
              const GM_notification = () => null;

              const GM_setClipboard = (value) => {
                try {
                  return navigator.clipboard?.writeText(String(value));
                } catch (_) {
                  return null;
                }
              };

              const GM_openInTab = (url) => {
                try {
                  return window.open(String(url), '_blank');
                } catch (_) {
                  return null;
                }
              };

              const GM_getResourceText = () => null;
              const GM_getResourceURL = () => null;

              const unsafeWindow = window;

              const GM_xmlhttpRequest = (details = {}) => {
                let settled = false;
                const controller =
                  typeof AbortController !== 'undefined'
                    ? new AbortController()
                    : null;

                const finish = (callback, value) => {
                  try {
                    if (typeof callback === 'function')
                      callback(value);
                  } catch (_) {}
                };

                const headers = {};
                try {
                  if (details.headers &&
                      typeof details.headers === 'object') {
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
                  method: String(
                    details.method ||
                    (details.data !== undefined ? 'POST' : 'GET')
                  ).toUpperCase(),
                  headers,
                  body: details.data !== undefined
                    ? details.data
                    : undefined,
                  redirect: 'follow',
                  credentials: 'omit',
                  mode: 'cors',
                  signal: controller?.signal
                }).then(async (response) => {
                  if (settled) return;

                  const text = await response.text();
                  let parsed = text;

                  if (details.responseType === 'json') {
                    try { parsed = JSON.parse(text); } catch (_) {}
                  }

                  const responseHeaders = [];
                  try {
                    response.headers.forEach((value, key) =>
                      responseHeaders.push(key + ': ' + value));
                  } catch (_) {}

                  const result = {
                    readyState: 4,
                    status: response.status,
                    statusText: response.statusText,
                    responseHeaders: responseHeaders.join('\r\n'),
                    responseText: text,
                    response: parsed,
                    responseXML: null,
                    finalUrl: response.url,
                    context: details.context
                  };

                  settled = true;
                  if (timeoutId) clearTimeout(timeoutId);

                  if (response.ok)
                    finish(details.onload, result);
                  else
                    finish(details.onerror, result);

                  finish(details.onloadend, result);
                }).catch((error) => {
                  if (settled) return;

                  settled = true;
                  if (timeoutId) clearTimeout(timeoutId);

                  if (error?.name === 'AbortError') {
                    finish(details.onabort, request);
                  } else {
                    finish(details.onerror, {
                      readyState: 4,
                      status: 0,
                      statusText: String(error?.name || 'error'),
                      responseText: '',
                      response: null,
                      responseXML: null,
                      finalUrl: String(details.url || ''),
                      context: details.context,
                      error
                    });
                  }

                  finish(details.onloadend, request);
                });

                return request;
              };

              try {
                (function () {
            {{source}}
                })();
              } catch (error) {
                try {
                  console.error(
                    '[IdleShell] userscript failed: ' + __idleshellScriptName,
                    error
                  );
                } catch (_) {}
              }
            })();
            """;
    }

    private static string NormalizeRunAt(string value) =>
        value is "document-start" or "document-end" or "document-idle"
            ? value
            : "document-end";

    private static string ToManifestRunAt(string value) =>
        value switch
        {
            "document-start" => "document_start",
            "document-idle" => "document_idle",
            _ => "document_end"
        };

    private sealed class ManifestContentScript
    {
        [JsonPropertyName("matches")]
        public IReadOnlyList<string> Matches { get; init; } = [];

        [JsonPropertyName("include_globs")]
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public IReadOnlyList<string>? IncludeGlobs { get; init; }

        [JsonPropertyName("js")]
        public IReadOnlyList<string> Js { get; init; } = [];

        [JsonPropertyName("run_at")]
        public string RunAt { get; init; } = "document_end";

        [JsonPropertyName("world")]
        public string World { get; init; } = "MAIN";

        [JsonPropertyName("all_frames")]
        public bool AllFrames { get; init; }
    }
}
