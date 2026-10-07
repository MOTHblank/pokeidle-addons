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


    // False after Close(); guards host-script registration on dead views.
    public bool IsAttached { get; private set; } = true;

    private Pane(
        PaneSpec spec,
        CoreWebView2Controller controller,
        CoreWebView2Environment environment,
        ViolentmonkeyManager? userscripts)
    {
        Spec = spec;
        Controller = controller;
        _environment = environment;
        _userscripts = userscripts;
    }

    public static async Task<Pane> CreateAsync(
        CoreWebView2Environment env, IntPtr hwnd, PaneSpec spec,
        ViolentmonkeyManager? userscripts = null,
        bool navigate = true)
    {
        var options = env.CreateCoreWebView2ControllerOptions();
        options.ProfileName = spec.Profile;
        options.IsInPrivateModeEnabled = false;

        var controller = await env.CreateCoreWebView2ControllerAsync(hwnd, options);
        var pane = new Pane(spec, controller, env, userscripts);
        await pane.ConfigureAsync();

        if (navigate)
            pane.View.Navigate(spec.Url);

        return pane;
    }

    private readonly ViolentmonkeyManager? _userscripts;
    private readonly CoreWebView2Environment _environment;

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

    private static string LiveStreamScannerScript() => """
        (() => {
          'use strict';

          const BUTTON_ID = 'idleshell-scan-live-streams';
          const INVENTORY_ID = 'btn-bolsa';

          const LIVE_VALUE_RE =
            /^(?:1|true|yes|on|live|online|ao[_ -]?vivo|en[_ -]?vivo)$/i;
          const LIVE_TEXT_RE =
            /^(?:live|online|ao vivo|ao-vivo|en vivo|en-vivo|watch now|assistir agora|ver ao vivo|assistir)$/i;
          const NEGATIVE_RE =
            /^(?:offline|off-line|ended|encerrad[oa]|not live|nao ao vivo)$/i;

          const EXCLUDED_TWITCH = new Set([
            'directory', 'downloads', 'jobs', 'p', 'search',
            'settings', 'subscriptions', 'wallet', 'videos', 'video',
            'popout', 'embed'
          ]);

          const EXCLUDED_KICK = new Set([
            'categories', 'browse', 'directory', 'following', 'search',
            'settings', 'auth', 'login', 'register', 'signup',
            'video', 'videos', 'popout'
          ]);

          let observer = null;
          let installTimer = 0;
          let scanning = false;

          const text = value =>
            String(value == null ? '' : value)
              .normalize('NFD')
              .replace(/[\u0300-\u036f]/g, '')
              .replace(/\s+/g, ' ')
              .trim()
              .toLowerCase();

          const channelUrl = raw => {
            try {
              const url = new URL(String(raw || ''), location.href);
              if (!/^https?:$/i.test(url.protocol)) return null;

              const host = url.hostname.toLowerCase().replace(/^www\./, '');
              if (host !== 'twitch.tv' && host !== 'kick.com') return null;

              const parts = url.pathname
                .split('/')
                .map(p => p.trim())
                .filter(Boolean);

              if (parts.length !== 1) return null;

              const channel = parts[0];
              const excluded =
                host === 'twitch.tv' ? EXCLUDED_TWITCH : EXCLUDED_KICK;

              if (!channel || excluded.has(channel.toLowerCase())) return null;

              return 'https://' + host + '/' + encodeURIComponent(channel);
            } catch (_) {
              return null;
            }
          };

          const liveValue = value => {
            const valueText = text(value);
            if (!valueText) return null;
            if (NEGATIVE_RE.test(valueText)) return false;
            if (LIVE_VALUE_RE.test(valueText)) return true;
            return null;
          };

          const markerOn = node => {
            if (!node || node.nodeType !== 1) return null;

            for (const name of [
              'data-live', 'data-is-live', 'data-online',
              'data-stream-live', 'data-streaming', 'data-status',
              'data-state'
            ]) {
              const result = liveValue(node.getAttribute(name));
              if (result !== null) return result;
            }

            for (const name of ['aria-label', 'title']) {
              const value = text(node.getAttribute(name));
              if (!value) continue;
              if (NEGATIVE_RE.test(value)) return false;
              if (LIVE_TEXT_RE.test(value)) return true;
            }

            for (const cls of Array.from(node.classList || []).map(text)) {
              if (/^(?:live|is-live|live-now|live-stream|stream-live|online|is-online|ao-vivo|aovivo|en-vivo|envivo)$/.test(cls))
                return true;
              if (/(?:offline|is-offline|ended|encerrad[oa])/.test(cls))
                return false;
            }

            const badges = node.querySelectorAll(
              'b,strong,small,span,i,[role="status"],[class*="badge"],[class*="status"],[class*="live"],[class*="online"]'
            );

            for (const badge of Array.from(badges).slice(0, 100)) {
              const value = text(badge.textContent);
              if (!value || value.length > 40) continue;
              if (NEGATIVE_RE.test(value)) return false;
              if (LIVE_TEXT_RE.test(value) || /^(?:\d+\s+)?(?:live|online|ao vivo|en vivo)(?:\s+\d+)?$/i.test(value))
                return true;
            }

            return null;
          };

          const isLive = anchor => {
            let node = anchor;
            for (let depth = 0; node && depth <= 8; depth++, node = node.parentElement) {
              const result = markerOn(node);
              if (result !== null) return result;
            }
            return false;
          };

          const collect = () => {
            const channels = new Map();

            for (const anchor of document.querySelectorAll('a[href]')) {
              const url = channelUrl(anchor.href || anchor.getAttribute('href'));
              if (!url || !isLive(anchor)) continue;
              channels.set(url, true);
            }

            return Array.from(channels.keys());
          };

          const send = url => {
            try {
              if (typeof window.__idleshell_openLink === 'function' &&
                  window.__idleshell_openLink(url, 'manual-live-chat-scan'))
                return true;
            } catch (_) {}

            try {
              window.chrome?.webview?.postMessage(JSON.stringify({
                type: 'link',
                url,
                source: 'manual-live-chat-scan'
              }));
              return true;
            } catch (_) {
              return false;
            }
          };

          const scan = button => {
            if (scanning) return;
            scanning = true;

            const label = button.querySelector('span');
            const original = label?.textContent || 'Open Live Streams';

            try {
              button.disabled = true;
              if (label) label.textContent = 'Scanning…';

              const urls = collect();
              let sent = 0;

              for (const url of urls) {
                if (send(url)) sent++;
              }

              if (label)
                label.textContent =
                  sent > 0 ? 'Opened ' + sent + ' live' : 'No live streams';
              console.info(
                '[IdleShell] manual live stream scan:',
                urls.length,
                'live channel(s),',
                sent,
                'routed'
              );
            } catch (error) {
              console.error('[IdleShell] manual live stream scan failed:', error);
              if (label) label.textContent = 'Scan failed';
            } finally {
              window.setTimeout(() => {
                button.disabled = false;
                if (label) label.textContent = original;
                scanning = false;
              }, 1600);
            }
          };

          const install = () => {
            installTimer = 0;

            const inventory = document.getElementById(INVENTORY_ID);
            if (!inventory || document.getElementById(BUTTON_ID)) return;

            const button = document.createElement('button');
            button.type = 'button';
            button.id = BUTTON_ID;
            button.className = inventory.className || 'btn-inventario';
            button.title = 'Open all live Twitch/KICK streams';
            button.setAttribute('aria-label', 'Open all live Twitch and KICK streams');

            const label = document.createElement('span');
            label.textContent = 'Open Live Streams';
            button.appendChild(label);

            button.addEventListener('click', () => scan(button));
            inventory.insertAdjacentElement('afterend', button);
          };

          const scheduleInstall = () => {
            if (installTimer) return;
            installTimer = window.setTimeout(install, 100);
          };

          const start = () => {
            install();

            if (!document.documentElement || observer) return;

            observer = new MutationObserver(scheduleInstall);
            observer.observe(document.documentElement, {
              childList: true,
              subtree: true
            });
          };

          if (document.readyState === 'loading')
            document.addEventListener('DOMContentLoaded', start, { once: true });
          else
            start();

          console.info('[IdleShell] native live stream scanner ready');
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
        {
            await View.AddScriptToExecuteOnDocumentCreatedAsync(StreamLinkInterceptorScript());
            await View.AddScriptToExecuteOnDocumentCreatedAsync(LiveStreamScannerScript());
        }

        if (_userscripts is not null)
        {
            await _userscripts.InstallForProfileAsync(
                View.Profile, _environment);
        }

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
        // cannot control. The game pane forwards supported stream URLs to the host.
        View.NewWindowRequested += (_, e) =>
        {
            e.Handled = true;
            PopupRequested?.Invoke(this, e.Uri);
        };

    }

    // Low-memory target must be set on a *visible* webview (setting it while
    // suspended/hidden is ignored per docs), so Show() applies Normal and
    // Hide()/Park() apply Low. Never mix with TrySuspend.
    public Task AttachUserscriptAsync(ViolentmonkeyManager manager) =>
        manager.InstallForProfileAsync(
            View.Profile, _environment);

    public void Show(Rectangle bounds)
    {
        Controller.Bounds = bounds;
        Controller.IsVisible = true;
        try { View.MemoryUsageTargetLevel = CoreWebView2MemoryUsageTargetLevel.Normal; }
        catch { }
    }

    public void Hide()
    {
        Controller.IsVisible = false;
        try { View.MemoryUsageTargetLevel = CoreWebView2MemoryUsageTargetLevel.Low; }
        catch { }
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
        return Spec;
    }

    public void Close() => Controller.Close();
}
