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

    // False after Close(); guards host-script registration on dead views.
    public bool IsAttached { get; private set; } = true;

    private Pane(
        PaneSpec spec,
        CoreWebView2Controller controller,
        CoreWebView2Environment environment,
        IntPtr hostWindowHandle,
        ViolentmonkeyManager? userscripts)
    {
        Spec = spec;
        Controller = controller;
        _environment = environment;
        _hostWindowHandle = hostWindowHandle;
        _userscripts = userscripts;
        if (spec.Kind == PaneKind.Stream) Mode = spec.Mode;
    }

    public static async Task<Pane> CreateAsync(
        CoreWebView2Environment env, IntPtr hwnd, PaneSpec spec,
        ViolentmonkeyManager? userscripts = null)
    {
        var options = env.CreateCoreWebView2ControllerOptions();
        options.ProfileName = spec.Profile;
        options.IsInPrivateModeEnabled = false;

        var controller = await env.CreateCoreWebView2ControllerAsync(hwnd, options);
        var pane = new Pane(spec, controller, env, hwnd, userscripts);
        await pane.ConfigureAsync();

        pane.View.Navigate(spec.Url);
        return pane;
    }

    private readonly ViolentmonkeyManager? _userscripts;
    private readonly CoreWebView2Environment _environment;
    private readonly IntPtr _hostWindowHandle;

    private static string BootstrapScript(PaneSpec spec)
    {
        var title = JsonSerializer.Serialize(spec.Title);
        var profile = JsonSerializer.Serialize(spec.Profile);
        var kind = JsonSerializer.Serialize(spec.Kind.ToString());

        var script = """
            (() => {
              'use strict';

              const info = Object.freeze({
                title: __TITLE__,
                profile: __PROFILE__,
                kind: __KIND__
              });

              try {
                Object.defineProperty(window, '__idleshell_hostInfo', {
                  value: info,
                  configurable: false,
                  enumerable: false,
                  writable: false
                });
              } catch (_) {
                try { window.__idleshell_hostInfo = info; } catch (_) {}
              }

              try {
                Object.defineProperty(window, '__idleshell_openLink', {
                  value: (url, source = 'host-bridge') => {
                    try {
                      if (window.chrome?.webview?.postMessage) {
                        window.chrome.webview.postMessage(JSON.stringify({
                          type: 'link',
                          url: String(url),
                          source,
                          pane: info.title,
                          profile: info.profile
                        }));
                        return true;
                      }
                    } catch (_) {}
                    return false;
                  },
                  configurable: false,
                  enumerable: false,
                  writable: false
                });
              } catch (_) {}
            })();
            """;

        return script
            .Replace("__TITLE__", title, StringComparison.Ordinal)
            .Replace("__PROFILE__", profile, StringComparison.Ordinal)
            .Replace("__KIND__", kind, StringComparison.Ordinal);
    }

    private static string StreamLinkInterceptorScript() => """
        (() => {
          'use strict';

          const streamUrl = (raw) => {
            try {
              const url = new URL(String(raw || ''), location.href);
              if (!/^https?:$/.test(url.protocol)) return null;
              if (!/^(?:www\\.|m\\.)?(?:twitch\\.tv|kick\\.com)$/i.test(url.hostname))
                return null;
              return url.href;
            } catch (_) {
              return null;
            }
          };

          const post = (url, source) => {
            const normalized = streamUrl(url);
            if (!normalized) return false;

            try {
              if (typeof window.__idleshell_openLink === 'function') {
                if (window.__idleshell_openLink(normalized, source))
                  return true;
              }
            } catch (_) {}

            try {
              window.chrome?.webview?.postMessage(JSON.stringify({
                type: 'link',
                url: normalized,
                source
              }));
              return true;
            } catch (_) {
              return false;
            }
          };

          const findLink = (event) => {
            try {
              const path = typeof event.composedPath === 'function'
                ? event.composedPath()
                : [event.target];

              for (const item of path) {
                if (!item || item.nodeType !== 1) continue;

                const href = item.href;
                if (typeof href === 'string') {
                  const url = streamUrl(href);
                  if (url) return url;
                }

                const raw = item.getAttribute?.('href');
                const url = streamUrl(raw);
                if (url) return url;
              }
            } catch (_) {}

            return null;
          };

          const onClick = (event) => {
            const url = findLink(event);
            if (!url) return;

            if (post(url, 'native-anchor-click')) {
              event.preventDefault();
              event.stopPropagation();
              event.stopImmediatePropagation?.();
            }
          };

          document.addEventListener('click', onClick, true);
          document.addEventListener('auxclick', onClick, true);

          const realOpen = window.open;
          window.open = function(url, ...rest) {
            if (post(url, 'native-window-open')) {
              return {
                closed: false,
                close() {},
                focus() {},
                blur() {}
              };
            }

            return realOpen ? realOpen.call(window, url, ...rest) : undefined;
          };

          console.info('[IdleShell] native stream-link interceptor installed');
        })();
        """;

    private async Task ConfigureAsync()
    {
        var s = View.Settings;
        s.AreDevToolsEnabled = true;
        s.IsStatusBarEnabled = false;
        s.IsZoomControlEnabled = true;

        await View.AddScriptToExecuteOnDocumentCreatedAsync(BootstrapScript(Spec));

        if (Spec.Kind == PaneKind.Game)
            await View.AddScriptToExecuteOnDocumentCreatedAsync(StreamLinkInterceptorScript());

        // Install the real upstream Violentmonkey extension before the first
        // navigation, and synchronize repository scripts through VM's own API.
        if (_userscripts is not null)
            await _userscripts.InstallForProfileAsync(
                View.Profile, _environment, _hostWindowHandle);

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

    }

    // Low-memory target must be set on a *visible* webview (setting it while
    // suspended/hidden is ignored per docs), so Show() applies Normal and
    // Hide()/Park() apply Low. Never mix with TrySuspend.
    public Task AttachUserscriptAsync(ViolentmonkeyManager manager) =>
        manager.InstallForProfileAsync(
            View.Profile, View.Environment, _hostWindowHandle);

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
