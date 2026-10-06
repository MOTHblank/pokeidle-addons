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
        CoreWebView2Environment environment,
        IntPtr hostWindowHandle)
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
                await SyncScriptsAsync(profile, environment, hostWindowHandle);
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
        CoreWebView2Environment environment,
        IntPtr hostWindowHandle)
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

        var controller = await environment.CreateCoreWebView2ControllerAsync(
            hostWindowHandle, controllerOptions);

        try
        {
            controller.IsVisible = false;
            var view = controller.CoreWebView2;

            await NavigateAndWaitAsync(
                view,
                $"chrome-extension://{vm.Id}/options/index.html#settings");

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

        CoreWebView2NavigationCompletedEventHandler? handler = null;
        handler = (_, e) =>
        {
            if (e.IsSuccess)
                tcs.TrySetResult(true);
            else
                tcs.TrySetException(new InvalidOperationException(
                    $"Violentmonkey options navigation failed: HTTP {e.HttpStatusCode}."));
        };

        view.NavigationCompleted += handler;
        try
        {
            view.Navigate(url);
            await tcs.Task.WaitAsync(TimeSpan.FromSeconds(30));
        }
        finally
        {
            view.NavigationCompleted -= handler;
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
            Path.Combine(AppContext.BaseDirectory, "violentmonkey"),
            Path.Combine(AppContext.BaseDirectory, "vendor", "violentmonkey"),
            Path.Combine(AppContext.BaseDirectory, "..", "..", "vendor", "violentmonkey"),
            Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "vendor", "violentmonkey"),
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
