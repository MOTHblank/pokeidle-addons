using System.Text.Json;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

/// <summary>
/// Integrates the real upstream Violentmonkey WebExtension into WebView2.
///
/// IdleShell does not implement a userscript runtime. Violentmonkey owns
/// metadata parsing, matching, execution timing, sandboxes, GM APIs, storage,
/// @require/@resource handling, menus, notifications, and XHR. This class only
/// installs the upstream extension and asks its own background command handler
/// to import the repository's *.user.js files into VM storage.
/// </summary>
internal sealed class ViolentmonkeyManager
{
    public const string Version = "2.49.0";

    private readonly HashSet<string> _installedProfiles =
        new(StringComparer.OrdinalIgnoreCase);
    private readonly HashSet<string> _syncedProfiles =
        new(StringComparer.OrdinalIgnoreCase);
    private readonly SemaphoreSlim _gate = new(1, 1);
    private readonly List<LocalScript> _scripts = [];

    public ViolentmonkeyManager(string addonsFolder)
    {
        Folder = Path.GetFullPath(addonsFolder);
        ExtensionFolder = ResolveExtensionFolder();
        LoadScripts();
    }

    public string Folder { get; }
    public string ExtensionFolder { get; }
    public IReadOnlyList<string> ScriptNames =>
        _scripts.Select(script => script.Name).ToArray();

    public async Task InstallForProfileAsync(
        CoreWebView2Profile profile,
        CoreWebView2Environment environment)
    {
        var key = profile.ProfilePath;
        await _gate.WaitAsync();
        try
        {
            if (_installedProfiles.Contains(key) && _syncedProfiles.Contains(key))
                return;

            if (!_installedProfiles.Contains(key))
            {
                if (!File.Exists(Path.Combine(ExtensionFolder, "manifest.json")))
                {
                    throw new FileNotFoundException(
                        "The real Violentmonkey extension is missing. Run IdleShell\\run.ps1 first.",
                        Path.Combine(ExtensionFolder, "manifest.json"));
                }

                var installed = await profile.GetBrowserExtensionsAsync();
                var extension = installed.FirstOrDefault(e =>
                    e.Name.Contains("Violentmonkey", StringComparison.OrdinalIgnoreCase));

                if (extension is null)
                {
                    extension = await profile.AddBrowserExtensionAsync(ExtensionFolder);
                    Console.Error.WriteLine(
                        $"[IdleShell] real Violentmonkey {Version} installed in profile " +
                        $"{profile.ProfileName}: {extension.Id}");
                }
                else
                {
                    Console.Error.WriteLine(
                        $"[IdleShell] real Violentmonkey {Version} already present in profile " +
                        $"{profile.ProfileName}: {extension.Id}");
                }

                if (!extension.IsEnabled)
                    await extension.EnableAsync(true);

                _installedProfiles.Add(key);
            }

            if (!_syncedProfiles.Contains(key))
            {
                await SyncScriptsAsync(profile, environment);
                _syncedProfiles.Add(key);
            }
        }
        finally
        {
            _gate.Release();
        }
    }

    private async Task SyncScriptsAsync(
        CoreWebView2Profile profile,
        CoreWebView2Environment environment)
    {
        var extensions = await profile.GetBrowserExtensionsAsync();
        var vm = extensions.FirstOrDefault(e =>
            e.Name.Contains("Violentmonkey", StringComparison.OrdinalIgnoreCase));

        if (vm is null)
            throw new InvalidOperationException(
                $"Violentmonkey {Version} was installed but could not be found in profile {profile.ProfileName}.");

        var controllerOptions = environment.CreateCoreWebView2ControllerOptions();
        controllerOptions.ProfileName = profile.ProfileName;
        controllerOptions.IsInPrivateModeEnabled = false;

        // HWND_MESSAGE creates an invisible message-only WebView. This keeps
        // the VM control page completely outside IdleShell's visible layout.
        var messageWindow = new IntPtr(-3);
        var controller = await environment.CreateCoreWebView2ControllerAsync(
            messageWindow, controllerOptions);

        try
        {
            controller.IsVisible = false;
            var view = controller.CoreWebView2;

            await NavigateAndWaitAsync(
                view,
                $"chrome-extension://{vm.Id}/options/index.html#settings");

            await RemoveLegacyRouterAsync(view);

            foreach (var script in _scripts)
                await ImportScriptAsync(view, script);

            Console.Error.WriteLine(
                $"[IdleShell] real Violentmonkey {Version} synchronized " +
                $"{_scripts.Count} repository script(s) into profile {profile.ProfileName}");
        }
        finally
        {
            try { controller.Close(); }
            catch { }
        }
    }

    private static async Task NavigateAndWaitAsync(CoreWebView2 view, string url)
    {
        var tcs = new TaskCompletionSource<bool>(
            TaskCreationOptions.RunContinuationsAsynchronously);

        // Let the compiler infer WebView2's event delegate type. The delegate
        // type itself is not public in the pinned WebView2 package.
        view.NavigationCompleted += (_, e) =>
        {
            if (e.IsSuccess)
                tcs.TrySetResult(true);
            else
                tcs.TrySetException(new InvalidOperationException(
                    $"Violentmonkey options navigation failed: HTTP {e.HttpStatusCode}."));
        };

        view.Navigate(url);
        await tcs.Task.WaitAsync(TimeSpan.FromSeconds(30));
    }

    private static async Task RemoveLegacyRouterAsync(CoreWebView2 view)
    {
        const string legacyName = "IdleShell Link Router (pokeidle.io)";
        const string legacyNamespace = "moth.pokeidle";

        var request = """
(() => {
  const state = { removed: false, id: null };
  return Promise.resolve(
    chrome.runtime.sendMessage({
      cmd: 'GetScript',
      data: {
        meta: {
          name: __LEGACY_NAME__,
          namespace: __LEGACY_NAMESPACE__
        }
      }
    })
  ).then(script => {
    const id = script?.props?.id;
    if (!id) return state;
    state.id = id;
    return Promise.resolve(
      chrome.runtime.sendMessage({
        cmd: 'MarkRemoved',
        data: { id, removed: true }
      })
    ).then(() => {
      state.removed = true;
      return state;
    });
  }).then(value => JSON.stringify(value));
})()
"""
            .Replace("__LEGACY_NAME__", JsonSerializer.Serialize(legacyName), StringComparison.Ordinal)
            .Replace("__LEGACY_NAMESPACE__", JsonSerializer.Serialize(legacyNamespace), StringComparison.Ordinal);

        try
        {
            var outer = await view.ExecuteScriptAsync(request);
            using var outerDoc = JsonDocument.Parse(outer);
            var inner = outerDoc.RootElement.GetString();
            if (string.IsNullOrWhiteSpace(inner))
                return;

            using var result = JsonDocument.Parse(inner);
            if (result.RootElement.TryGetProperty("removed", out var removed) &&
                removed.GetBoolean())
            {
                var id = result.RootElement.TryGetProperty("id", out var idValue)
                    ? idValue.ToString()
                    : "?";
                Console.Error.WriteLine(
                    $"[IdleShell] removed legacy stream-link router script #{id}");
            }
        }
        catch (Exception ex)
        {
            // A fresh profile simply has nothing to remove; do not make startup
            // dependent on this one-time migration.
            Console.Error.WriteLine(
                $"[IdleShell] legacy stream-link router cleanup skipped: {ex.Message}");
        }
    }

    private static async Task ImportScriptAsync(CoreWebView2 view, LocalScript script)
    {
        var id = Guid.NewGuid().ToString("N");
        var code = JsonSerializer.Serialize(script.Source);
        var installUrl = JsonSerializer.Serialize(script.SourceUrl);

        var request = $$"""
(() => {
  const id = "{{id}}";
  const state = window.__idleshellVmImports ??= Object.create(null);

  try {
    const data = {
      code: {{code}},
      url: {{installUrl}},
      config: {
        shouldUpdate: 0
      }
    };

    Promise.resolve(
      chrome.runtime.sendMessage({
        cmd: 'ParseScript',
        data
      })
    ).then(
      result => { state[id] = { done: true, ok: true, result }; },
      error => {
        state[id] = {
          done: true,
          ok: false,
          error: String(error?.message || error)
        };
      }
    );

    state[id] = { done: false };
  } catch (error) {
    state[id] = {
      done: true,
      ok: false,
      error: String(error?.message || error)
    };
  }

  return true;
})()
""";

        await view.ExecuteScriptAsync(request);

        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(30);
        while (DateTime.UtcNow < deadline)
        {
            await Task.Delay(50);

            var probe = await view.ExecuteScriptAsync(
                $$"""
                (() => {
                  const v = window.__idleshellVmImports?.["{{id}}"];
                  return JSON.stringify(v ?? null);
                })()
                """);

            try
            {
                using var outer = JsonDocument.Parse(probe);
                var json = outer.RootElement.GetString();
                if (string.IsNullOrWhiteSpace(json))
                    continue;

                using var state = JsonDocument.Parse(json);
                var root = state.RootElement;
                if (!root.TryGetProperty("done", out var done) || !done.GetBoolean())
                    continue;

                if (root.TryGetProperty("ok", out var ok) && ok.GetBoolean())
                    return;

                var error = root.TryGetProperty("error", out var errorValue)
                    ? errorValue.GetString()
                    : "unknown Violentmonkey installation error";

                throw new InvalidOperationException(
                    $"Violentmonkey failed to import '{script.Name}': {error}");
            }
            catch (JsonException)
            {
                // The extension page may still be changing while the first
                // command response is being materialized.
            }
        }

        throw new TimeoutException(
            $"Timed out importing '{script.Name}' into Violentmonkey.");
    }

    private void LoadScripts()
    {
        _scripts.Clear();

        if (!Directory.Exists(Folder))
            return;

        foreach (var path in Directory.EnumerateFiles(
                     Folder,
                     "*.user.js",
                     SearchOption.TopDirectoryOnly)
                 .OrderBy(Path.GetFileName, StringComparer.OrdinalIgnoreCase))
        {
            try
            {
                _scripts.Add(new LocalScript(
                    Path.GetFileNameWithoutExtension(path),
                    File.ReadAllText(path),
                    BuildSourceUrl(path)));
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine(
                    $"[IdleShell] userscript '{path}' skipped: {ex.Message}");
            }
        }
    }

    private static string BuildSourceUrl(string path)
    {
        var name = Uri.EscapeDataString(Path.GetFileName(path));
        return $"https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/main/addons/{name}";
    }

    private static string ResolveExtensionFolder()
    {
        var candidates = new[]
        {
            // Canonical runtime copy produced by the project build.
            Path.Combine(AppContext.BaseDirectory, "vendor", "violentmonkey"),
            // Source-tree fallback for development when running directly from the
            // repository before the content has been copied to bin/.
            Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "vendor", "violentmonkey")
        };

        foreach (var candidate in candidates)
        {
            try
            {
                if (File.Exists(Path.Combine(candidate, "manifest.json")))
                    return Path.GetFullPath(candidate);
            }
            catch { }
        }

        return Path.GetFullPath(candidates[0]);
    }

    private sealed record LocalScript(
        string Name,
        string Source,
        string SourceUrl);
}
