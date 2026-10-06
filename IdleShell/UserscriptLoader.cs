using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

/// <summary>
/// Reads the repository's *.user.js files and packages them as ordinary
/// WebExtension content scripts. WebView2/Chromium then owns matching,
/// timing, frame targeting and JavaScript execution instead of this class
/// injecting/evaluating user code with AddScriptToExecuteOnDocumentCreatedAsync.
/// </summary>
internal sealed class UserscriptLoader
{
    private const string ExtensionName = "IdleShell Userscript Engine";

    private static readonly Regex MetadataBlock = new(
        @"//\s*==UserScript==\s*(?<body>.*?)//\s*==/UserScript==",
        RegexOptions.Singleline | RegexOptions.Compiled);

    private static readonly Regex MetadataLine = new(
        @"^\s*//\s*@(?<key>[A-Za-z][A-Za-z0-9_-]*)\s+(?<value>.+?)\s*$",
        RegexOptions.Multiline | RegexOptions.Compiled);

    private readonly List<Userscript> _scripts = [];
    private readonly HashSet<string> _installedProfiles =
        new(StringComparer.OrdinalIgnoreCase);
    private readonly SemaphoreSlim _installGate = new(1, 1);

    public UserscriptLoader(string addonsFolder)
    {
        Folder = Path.GetFullPath(addonsFolder);
        Load();
        ExtensionFolder = PrepareExtensionBundle();
    }

    public string Folder { get; }
    public string ExtensionFolder { get; }
    public IReadOnlyList<Userscript> Scripts => _scripts;

    /// <summary>
    /// Installs the current unpacked extension in a WebView2 profile before
    /// navigation. WebView2 requires AreBrowserExtensionsEnabled=true.
    /// </summary>
    public async Task InstallAsync(CoreWebView2Profile profile)
    {
        var profileKey = profile.ProfilePath;
        if (_installedProfiles.Contains(profileKey))
            return;

        await _installGate.WaitAsync();
        try
        {
            if (_installedProfiles.Contains(profileKey))
                return;

            // Each UserscriptLoader instance represents one immutable bundle.
            // Replace any older IdleShell Userscript Engine in this profile so
            // addon changes can never leave stale script code installed.
            var installed = await profile.GetBrowserExtensionsAsync();
            foreach (var extension in installed.Where(e =>
                         string.Equals(e.Name, ExtensionName, StringComparison.Ordinal)))
            {
                try { await extension.RemoveAsync(); }
                catch (Exception ex)
                {
                    Console.Error.WriteLine(
                        $"[IdleShell] userscript extension remove failed in {profile.ProfileName}: {ex.Message}");
                }
            }

            var extension = await profile.AddBrowserExtensionAsync(ExtensionFolder);
            if (!extension.IsEnabled)
                await extension.EnableAsync(true);

            _installedProfiles.Add(profileKey);
            Console.Error.WriteLine(
                $"[IdleShell] userscript extension installed in profile {profile.ProfileName}: " +
                $"{extension.Id}, {_scripts.Count} script(s)");
        }
        finally
        {
            _installGate.Release();
        }
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
        var namespaceName = "http://tampermonkey.net/";
        var version = "0.0.0";
        var description = "";
        var author = "";
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
                    namespaceName = value;
                    break;
                case "version":
                    version = value;
                    break;
                case "description":
                    description = value;
                    break;
                case "author":
                    author = value;
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
                    if (parts.Length == 2)
                        resources[parts[0]] = parts[1];
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
            namespaceName,
            version,
            description,
            author,
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

    private string PrepareExtensionBundle()
    {
        var sourceFolder = Path.Combine(
            AppContext.BaseDirectory, "UserscriptExtension");
        var staticFiles = new[] { "bridge.js", "background.js" };

        foreach (var file in staticFiles)
        {
            if (!File.Exists(Path.Combine(sourceFolder, file)))
                throw new FileNotFoundException(
                    $"Userscript extension support file is missing: {file}",
                    Path.Combine(sourceFolder, file));
        }

        var fingerprint = BuildFingerprint(sourceFolder);
        var root = Path.Combine(AppConfig.UserscriptExtensionDataFolder, fingerprint);
        Directory.CreateDirectory(root);
        Directory.CreateDirectory(Path.Combine(root, "scripts"));

        foreach (var file in staticFiles)
            File.Copy(Path.Combine(sourceFolder, file), Path.Combine(root, file), true);

        var contentScripts = new List<Dictionary<string, object?>>();

        for (var i = 0; i < _scripts.Count; i++)
        {
            var script = _scripts[i];
            var fileName = $"script-{i:D2}-{ShortHash(script.Source + script.Path)}.js";
            var outputPath = Path.Combine(root, "scripts", fileName);
            File.WriteAllText(outputPath, BuildWrapper(script), new UTF8Encoding(false));

            var entry = new Dictionary<string, object?>
            {
                ["matches"] = script.Matches.Count > 0
                    ? (object)script.Matches
                    : new[] { "<all_urls>" },
                ["js"] = new[] { $"scripts/{fileName}" },
                ["run_at"] = script.RunAt,
                ["all_frames"] = !script.NoFrames,
                ["world"] = script.InjectInto == "content" ? "ISOLATED" : "MAIN"
            };

            if (script.Includes.Count > 0)
                entry["include_globs"] = script.Includes;
            if (script.Excludes.Count > 0)
                entry["exclude_globs"] = script.Excludes;
            if (script.ExcludeMatches.Count > 0)
                entry["exclude_matches"] = script.ExcludeMatches;

            contentScripts.Add(entry);
        }

        var manifest = new Dictionary<string, object?>
        {
            ["manifest_version"] = 3,
            ["name"] = ExtensionName,
            ["version"] = "1.0.0",
            ["description"] =
                "Browser-native execution engine for the IdleShell repository's Tampermonkey-compatible userscripts.",
            ["background"] = new Dictionary<string, object?>
            {
                ["service_worker"] = "background.js"
            },
            ["host_permissions"] = new[] { "<all_urls>" },
            ["content_scripts"] = contentScripts
                .Prepend(new Dictionary<string, object?>
                {
                    ["matches"] = new[] { "<all_urls>" },
                    ["js"] = ["bridge.js"],
                    ["run_at"] = "document_start",
                    ["all_frames"] = true
                })
                .ToList()
        };

        var options = new JsonSerializerOptions
        {
            WriteIndented = true,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
        };
        File.WriteAllText(
            Path.Combine(root, "manifest.json"),
            JsonSerializer.Serialize(manifest, options),
            new UTF8Encoding(false));

        File.WriteAllText(
            Path.Combine(root, "bundle.txt"),
            $"IdleShell userscript bundle {fingerprint}{Environment.NewLine}" +
            $"Scripts: {_scripts.Count}{Environment.NewLine}" +
            string.Join(
                Environment.NewLine,
                _scripts.Select(s =>
                    $"{s.Name} | {s.Version} | {s.RunAt} | {s.InjectInto} | {s.Path}")),
            new UTF8Encoding(false));

        return root;
    }

    private string BuildWrapper(Userscript script)
    {
        var marker = JsonSerializer.Serialize("__idleshell_gm_bridge_v1__");
        var name = JsonSerializer.Serialize(script.Name);
        var ns = JsonSerializer.Serialize(script.Namespace);
        var version = JsonSerializer.Serialize(script.Version);
        var description = JsonSerializer.Serialize(script.Description);
        var author = JsonSerializer.Serialize(script.Author);
        var sourceFile = JsonSerializer.Serialize(Path.GetFileName(script.Path));
        var resources = JsonSerializer.Serialize(script.Resources);
        var grants = JsonSerializer.Serialize(script.Grants);
        var runAt = JsonSerializer.Serialize(script.RunAt);

        return $$"""
(() => {
  'use strict';

  const __idleshellBridgeMarker = {{marker}};
  const __idleshellScriptMeta = Object.freeze({
    name: {{name}},
    namespace: {{ns}},
    version: {{version}},
    description: {{description}},
    author: {{author}},
    sourceFile: {{sourceFile}},
    runAt: {{runAt}},
    grants: {{grants}},
    resources: {{resources}}
  });

  const __idleshellStoragePrefix =
    '__idleshell_gm_v2__' +
    __idleshellScriptMeta.namespace + ':' +
    __idleshellScriptMeta.name + ':';

  const __idleshellRead = (key) => {
    try {
      const raw = window.localStorage.getItem(
        __idleshellStoragePrefix + String(key));
      return raw === null ? undefined : JSON.parse(raw);
    } catch (_) {
      return undefined;
    }
  };

  const __idleshellWrite = (key, value) => {
    try {
      window.localStorage.setItem(
        __idleshellStoragePrefix + String(key),
        JSON.stringify(value));
    } catch (_) {}
  };

  const __idleshellDelete = (key) => {
    try {
      window.localStorage.removeItem(
        __idleshellStoragePrefix + String(key));
    } catch (_) {}
  };

  const GM_getValue = (key, fallback) => {
    const value = __idleshellRead(key);
    return value === undefined ? fallback : value;
  };

  const GM_getValues = (keys) => {
    if (Array.isArray(keys)) {
      return Object.fromEntries(
        keys.map(key => [String(key), GM_getValue(key, undefined)]));
    }

    const source = keys && typeof keys === 'object' ? keys : {};
    return Object.fromEntries(
      Object.entries(source).map(([key, fallback]) =>
        [key, GM_getValue(key, fallback)]));
  };

  const GM_setValue = (key, value) => {
    const oldValue = GM_getValue(key, undefined);
    __idleshellWrite(key, value);
    try {
      for (const event of ['idleshell-gm-value-changed']) {
        window.dispatchEvent(new CustomEvent(event, {
          detail: { key: String(key), oldValue, newValue: value, remote: false }
        }));
      }
    } catch (_) {}
  };

  const GM_setValues = (values) => {
    for (const [key, value] of Object.entries(values || {}))
      GM_setValue(key, value);
  };

  const GM_deleteValue = (key) => {
    const oldValue = GM_getValue(key, undefined);
    __idleshellDelete(key);
    try {
      window.dispatchEvent(new CustomEvent('idleshell-gm-value-changed', {
        detail: { key: String(key), oldValue, newValue: undefined, remote: false }
      }));
    } catch (_) {}
  };

  const GM_deleteValues = (keys) => {
    for (const key of keys || []) GM_deleteValue(key);
  };

  const GM_listValues = () => {
    const names = [];
    try {
      const prefix = __idleshellStoragePrefix;
      for (let i = 0; i < window.localStorage.length; i++) {
        const key = window.localStorage.key(i);
        if (key && key.startsWith(prefix))
          names.push(key.slice(prefix.length));
      }
    } catch (_) {}
    return names;
  };

  const __idleshellListeners = new Map();

  const GM_addValueChangeListener = (key, callback) => {
    const id = Math.random().toString(36).slice(2);
    __idleshellListeners.set(id, { key: String(key), callback });
    return id;
  };

  const GM_removeValueChangeListener = (id) => {
    __idleshellListeners.delete(String(id));
  };

  window.addEventListener('storage', (event) => {
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
  });

  window.addEventListener('idleshell-gm-value-changed', (event) => {
    const detail = event.detail;
    if (!detail) return;

    for (const listener of __idleshellListeners.values()) {
      if (listener.key === detail.key) {
        try {
          listener.callback(
            detail.key,
            detail.oldValue,
            detail.newValue,
            Boolean(detail.remote));
        } catch (_) {}
      }
    }
  });

  const GM_addStyle = (css) => {
    const style = document.createElement('style');
    style.textContent = String(css);
    (document.head || document.documentElement).appendChild(style);
    return style;
  };

  const GM_addElement = (...args) => {
    let parent = document.body || document.documentElement;
    let tag;
    let attrs;
    let text;

    if (args[0] && args[0].nodeType === 1) {
      parent = args[0];
      tag = args[1];
      attrs = args[2];
      text = args[3];
    } else {
      tag = args[0];
      attrs = args[1];
      text = args[2];
    }

    try {
      const element = document.createElement(String(tag));
      for (const [key, value] of Object.entries(attrs || {}))
        element.setAttribute(key, String(value));
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
        const result = document.execCommand('copy');
        area.remove();
        return result;
      } catch (_) {
        return false;
      }
    }
  };

  const __idleshellXhr = (details) => {
    const id = Math.random().toString(36).slice(2) + Date.now().toString(36);
    const callbacks = details || {};
    let finished = false;
    let timeoutId = null;

    const cleanup = () => {
      window.removeEventListener('message', onMessage, true);
      if (timeoutId !== null) clearTimeout(timeoutId);
    };

    const done = (kind, payload) => {
      if (finished) return;
      finished = true;
      cleanup();

      const fn = callbacks[kind];
      if (typeof fn !== 'function') return;

      try { fn(payload); }
      catch (error) {
        console.error('[IdleShell] GM_xmlhttpRequest callback failed:', error);
      }
    };

    const responseObject = (result) => ({
      readyState: 4,
      status: Number(result.status || 0),
      statusText: String(result.statusText || ''),
      responseHeaders: String(result.responseHeaders || ''),
      responseText: String(result.responseText || ''),
      response: String(result.responseText || ''),
      finalUrl: String(result.finalUrl || callbacks.url || '')
    });

    const onMessage = (event) => {
      if (event.source !== window) return;
      const message = event.data;
      if (!message || message.__idleshell !== __idleshellBridgeMarker ||
          message.kind !== 'xhr-response' || message.id !== id)
        return;

      const result = message.response || {};
      if (result.ok) {
        const response = responseObject(result);
        done('onload', response);
        return;
      }

      done('onerror', {
        readyState: 4,
        status: 0,
        statusText: '',
        responseHeaders: '',
        responseText: '',
        response: '',
        finalUrl: String(callbacks.url || ''),
        error: String(result.error || 'GM_xmlhttpRequest failed')
      });
    };

    window.addEventListener('message', onMessage, true);

    if (Number(callbacks.timeout) > 0) {
      timeoutId = setTimeout(() => {
        done('ontimeout', {
          readyState: 4,
          status: 0,
          statusText: '',
          responseHeaders: '',
          responseText: '',
          response: '',
          finalUrl: String(callbacks.url || '')
        });
      }, Number(callbacks.timeout));
    }

    try {
      window.postMessage({
        __idleshell: __idleshellBridgeMarker,
        kind: 'xhr',
        id,
        details: {
          method: String(callbacks.method || 'GET').toUpperCase(),
          url: String(callbacks.url || ''),
          headers: callbacks.headers && typeof callbacks.headers === 'object'
            ? callbacks.headers : {},
          data: typeof callbacks.data === 'string' ? callbacks.data : undefined,
          withCredentials: Boolean(callbacks.withCredentials),
          anonymous: Boolean(callbacks.anonymous),
          timeout: Number(callbacks.timeout || 0)
        }
      }, '*');
    } catch (error) {
      done('onerror', {
        readyState: 4,
        status: 0,
        statusText: '',
        responseHeaders: '',
        responseText: '',
        response: '',
        finalUrl: String(callbacks.url || ''),
        error: String(error?.message || error)
      });
    }

    return {
      abort: () => {
        done('onabort', {
          readyState: 4,
          status: 0,
          statusText: '',
          responseHeaders: '',
          responseText: '',
          response: '',
          finalUrl: String(callbacks.url || '')
        });
      },
      get readyState() { return finished ? 4 : 1; }
    };
  };

  const GM_xmlhttpRequest = (details) => __idleshellXhr(details);
  const GM_openInTab = (url, options) => {
    const opened = { closed: false, onclose: null };

    try {
      if (typeof window.__idleshell_openLink === 'function' &&
          window.__idleshell_openLink(String(url), 'GM_openInTab')) {
        opened.close = () => {
          opened.closed = true;
          try { opened.onclose?.(); } catch (_) {}
        };
        return opened;
      }
    } catch (_) {}

    try {
      const target = window.open(String(url), '_blank');
      opened.close = () => {
        try { target?.close(); } catch (_) {}
        opened.closed = true;
        try { opened.onclose?.(); } catch (_) {}
      };
    } catch (_) {
      opened.closed = true;
    }

    return opened;
  };

  const GM_registerMenuCommand = (_name, _callback, _accessKey) => null;
  const GM_unregisterMenuCommand = (_id) => {};
  const GM_notification = (...args) => {
    console.info('[IdleShell] GM_notification', ...args);
    return null;
  };

  const GM_getResourceURL = (name) =>
    __idleshellScriptMeta.resources[String(name)] ?? null;
  const GM_getResourceText = (_name) => null;

  const GM_download = (options) => {
    const source = typeof options === 'string' ? { url: options } : (options || {});
    try {
      const anchor = document.createElement('a');
      anchor.href = String(source.url || '');
      anchor.download = String(source.name || '');
      anchor.target = '_blank';
      anchor.rel = 'noreferrer';
      document.documentElement.appendChild(anchor);
      anchor.click();
      anchor.remove();
    } catch (error) {
      console.error('[IdleShell] GM_download failed:', error);
    }
  };

  const GM_info = Object.freeze({
    scriptHandler: 'IdleShell WebExtension',
    version: '1.0',
    script: Object.freeze({
      name: __idleshellScriptMeta.name,
      namespace: __idleshellScriptMeta.namespace,
      version: __idleshellScriptMeta.version,
      description: __idleshellScriptMeta.description,
      author: __idleshellScriptMeta.author
    })
  });

  const GM = Object.freeze({
    getValue: GM_getValue,
    getValues: GM_getValues,
    setValue: GM_setValue,
    setValues: GM_setValues,
    deleteValue: GM_deleteValue,
    deleteValues: GM_deleteValues,
    listValues: GM_listValues,
    addValueChangeListener: GM_addValueChangeListener,
    removeValueChangeListener: GM_removeValueChangeListener,
    addStyle: GM_addStyle,
    addElement: GM_addElement,
    setClipboard: GM_setClipboard,
    xmlhttpRequest: GM_xmlhttpRequest,
    xmlHttpRequest: GM_xmlhttpRequest,
    openInTab: GM_openInTab,
    registerMenuCommand: GM_registerMenuCommand,
    unregisterMenuCommand: GM_unregisterMenuCommand,
    notification: GM_notification,
    getResourceURL: GM_getResourceURL,
    getResourceText: GM_getResourceText,
    download: GM_download
  });

  const unsafeWindow = window;

  console.info(
    '[IdleShell] userscript starting:',
    __idleshellScriptMeta.name,
    __idleshellScriptMeta.version
  );

  // The browser extension engine controls matching/run-at/frame targeting.
  // The original userscript body is executed directly in this wrapper,
  // without eval/new Function.
{{script.Source}}
})();
""";
    }

    private string BuildFingerprint(string sourceFolder)
    {
        using var sha = SHA256.Create();
        var input = new StringBuilder();

        foreach (var script in _scripts)
        {
            input.Append(script.Path);
            input.Append('\n');
            input.Append(script.Source);
            input.Append('\n');
        }

        foreach (var file in new[] { "manifest-support.js", "background.js" })
            input.Append(File.ReadAllText(Path.Combine(sourceFolder, file)));

        var bytes = sha.ComputeHash(Encoding.UTF8.GetBytes(input.ToString()));
        return Convert.ToHexString(bytes)[..16].ToLowerInvariant();
    }

    private static string ShortHash(string value)
    {
        using var sha = SHA256.Create();
        return Convert.ToHexString(
            sha.ComputeHash(Encoding.UTF8.GetBytes(value)))[..12].ToLowerInvariant();
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
