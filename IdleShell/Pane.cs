using System.Text.Json;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

// Payload delivered to the host when a page calls window.__idleshell_post(json).
internal readonly record struct HostMessage(string Type, string Url, string Source, string Pane, string Profile);

internal sealed class Pane
{
    public PaneSpec Spec { get; private set; }
    public CoreWebView2Controller Controller { get; }
    public CoreWebView2 View => Controller.CoreWebView2;

    // Raised for every __idleshell_post message (link routing, probe results).
    public event Action<Pane, HostMessage>? MessageReceived;
    // Raised when page content requests a new window (window.open / target=_blank).
    public event Action<Pane, string>? PopupRequested;

    // Per-pane background behavior for inactive stream panes (default: Background).
    public StreamMode Mode { get; set; } = StreamMode.Background;

    // True while this pane is displayed in the layout; false when hidden/parked.
    public bool IsForeground { get; private set; } = true;

    private Pane(PaneSpec spec, CoreWebView2Controller controller)
    {
        Spec = spec;
        Controller = controller;
        if (spec.Kind == PaneKind.Stream) Mode = spec.Mode;
    }

    public static async Task<Pane> CreateAsync(
        CoreWebView2Environment env, IntPtr hwnd, PaneSpec spec)
    {
        var options = env.CreateCoreWebView2ControllerOptions();
        options.ProfileName = spec.Profile;
        options.IsInPrivateModeEnabled = false;

        var controller = await env.CreateCoreWebView2ControllerAsync(hwnd, options);
        var pane = new Pane(spec, controller);
        await pane.ConfigureAsync();
        pane.View.Navigate(spec.Url);
        return pane;
    }

    // Bootstrap injected into EVERY isolated world (including Tampermonkey's)
    // before any page script runs. It only defines helpers; the link-router
    // userscript does the interception. Because it lands in all worlds, pages
    // can call __idleshell_openLink directly even without a userscript.
    private static string BootstrapScript(PaneSpec spec) => $$"""
        (() => {
          try {
            const post = (payload) => {
              try { window.chrome.webview.postMessage(JSON.stringify(payload)); } catch (e) {}
            };
            Object.defineProperty(window, '__idleshell_hostInfo',
              { value: Object.freeze({ title: {{JsonSerializer.Serialize(spec.Title)}},
                                      profile: {{JsonSerializer.Serialize(spec.Profile)}} }),
                configurable: false, writable: false });
            window.__idleshell_post = post;
            window.__idleshell_openLink = (url) => {
              post({ type: 'link', url: String(url), source: 'bootstrap',
                     pane: window.__idleshell_hostInfo.title,
                     profile: window.__idleshell_hostInfo.profile });
            };
          } catch (e) {}
        })();
        """;

    private async Task ConfigureAsync()
    {
        var s = View.Settings;
        s.AreDevToolsEnabled = true;
        s.IsStatusBarEnabled = false;
        s.IsZoomControlEnabled = true;

        await View.AddScriptToExecuteOnDocumentCreatedAsync(BootstrapScript(Spec));

        View.WebMessageReceived += (_, e) =>
        {
            try
            {
                using var doc = JsonDocument.Parse(e.WebMessageAsJson);
                var root = doc.RootElement;
                string Field(string key) =>
                    root.TryGetProperty(key, out var v) && v.ValueKind == JsonValueKind.String
                        ? v.GetString() ?? "" : "";
                MessageReceived?.Invoke(this, new HostMessage(
                    Field("type"), Field("url"), Field("source"), Field("pane"), Field("profile")));
            }
            catch { /* not one of our JSON payloads */ }
        };

        // Capture popups instead of letting WebView2 spawn an OS window we
        // cannot control (no profile/visibility handling there). The game pane
        // forwards the URL to the link router; stream panes open it visibly.
        View.NewWindowRequested += (_, e) =>
        {
            e.Handled = true;
            PopupRequested?.Invoke(this, e.Uri);
        };

        if (Spec.Kind == PaneKind.Stream)
        {
            View.IsMuted = true;
        }

        await LoadExtensionsAsync();
    }

    // Low-memory target must be set on a *visible* webview (setting it while
    // suspended/hidden is ignored per docs), so Show() applies Normal and
    // Hide()/Park() apply Low. Never mix with TrySuspend.
    public void Show(Rectangle bounds)
    {
        Controller.Bounds = bounds;
        Controller.IsVisible = true;
        IsForeground = true;
        try { View.MemoryUsageTargetLevel = CoreWebView2MemoryUsageTargetLevel.Normal; } catch { }
    }

    // Background mode: IsVisible=false stops compositing (the big cost) while the
    // page keeps running — Chromium throttling is already disabled via browser
    // flags. Do NOT call TrySuspendAsync here: it pauses script timers and
    // animations, which would stop the addons.
    // Note: CoreWebView2Settings.PreferredBackgroundTimerWakeInterval (the
    // "unthrottled hidden timers" API, wake interval 0) is not exposed by the
    // pinned WebView2 package (1.0.4258.31); the --disable-background-timer-throttling
    // flag in AppConfig covers this instead.
    public void Hide()
    {
        Controller.IsVisible = false;
        IsForeground = false;
        try { View.MemoryUsageTargetLevel = CoreWebView2MemoryUsageTargetLevel.Low; } catch { }
    }

    // Legacy off-screen parking: keeps rendering + compositing alive, so it is
    // only useful as a fallback if platforms pause when the pane is hidden.
    public void Park()
    {
        Controller.Bounds = new Rectangle(-10000, -10000, 640, 360);
        Controller.IsVisible = true;
        IsForeground = false;
        try { View.MemoryUsageTargetLevel = CoreWebView2MemoryUsageTargetLevel.Low; } catch { }
    }

    private async Task LoadExtensionsAsync()
    {
        if (!Directory.Exists(AppConfig.ExtensionsFolder)) return;

        foreach (var dir in Directory.EnumerateDirectories(AppConfig.ExtensionsFolder))
        {
            if (!File.Exists(Path.Combine(dir, "manifest.json"))) continue;
            try { await View.Profile.AddBrowserExtensionAsync(dir); }
            catch (Exception ex)
            {
                // Already installed in this profile, or unsupported manifest.
                Console.Error.WriteLine($"[IdleShell] extension {dir}: {ex.Message}");
            }
        }
    }

    // One-shot probe script: reads visibilityState/hasFocus and measures how much
    // a setTimeout(1000) drifted (background timer throttling clamps to >=1000ms).
    private const string ProbeScript = """
        (() => {
          const t0 = Date.now();
          return new Promise(res => setTimeout(() => res(JSON.stringify({
            visibility: document.visibilityState,
            hidden: document.hidden,
            focus: document.hasFocus(),
            driftMs: (Date.now() - t0) - 1000
          })), 1000));
        })()
        """;

    public async Task<string?> ProbeAsync()
    {
        try { return await View.ExecuteScriptAsync(ProbeScript); }
        catch (Exception ex) { return $"\"probe-error: {ex.Message.Replace("\"", "'")}\""; }
    }

    // Keep the latest URL so the session restores where you left off.
    public PaneSpec Snapshot()
    {
        var url = View?.Source;
        if (!string.IsNullOrWhiteSpace(url)) Spec = Spec with { Url = url };
        return Spec with { Mode = Mode };
    }

    public void Close() => Controller.Close();
}
