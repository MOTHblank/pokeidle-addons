using System.Text;
using System.Text.RegularExpressions;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

internal sealed class MainForm : Form
{
    private readonly Panel _toolbar;
    private readonly ComboBox _streamPicker = new() { DropDownStyle = ComboBoxStyle.DropDownList, Width = 220 };
    private readonly ComboBox _addonsPicker = new() { DropDownStyle = ComboBoxStyle.DropDownList, Width = 190 };
    private readonly Label _status = new() { AutoSize = true, Padding = new Padding(3, 4, 3, 0) };

    private CoreWebView2Environment? _gameEnv;
    private CoreWebView2Environment? _streamEnv;
    private readonly List<Pane> _games = [];
    private readonly List<Pane> _streams = [];
    private Pane? _activeStream;
    private ExtensionManager? _extensions;
    private UserscriptLoader? _userscripts;
    private string _tampermonkeyState = "not ready";
    private bool _nativeFallback;
    private readonly System.Windows.Forms.Timer _statsTimer = new() { Interval = 5000 };
    private readonly System.Windows.Forms.Timer _probeTimer = new() { Interval = 30000 };
    private readonly CheckBox _probeToggle = new()
        { Text = "Probe", AutoSize = true, Checked = true, Padding = new Padding(3, 6, 3, 0) };
    private StreamMode _inactiveStreamMode = StreamMode.Background;
    private bool _probing;
    private Button _modeButton = null!; // assigned in BuildToolbar, called from the ctor
    private static readonly Regex StreamUrlRegex =
        new($@"^https?://{AppConfig.StreamHostPattern}/", RegexOptions.IgnoreCase | RegexOptions.Compiled);

    public MainForm()
    {
        Text = "PokéIdle Idle Shell";
        StartPosition = FormStartPosition.CenterScreen;
        MinimumSize = new Size(960, 600);
        ClientSize = new Size(1440, 850);

        _toolbar = new Panel
        {
            Dock = DockStyle.Top,
            Height = AppConfig.ToolbarHeight,
            Padding = new Padding(8, 5, 8, 5)
        };
        BuildToolbar();
        Controls.Add(_toolbar);

        Resize += (_, _) => LayoutPanes();
        FormClosing += (_, _) => SaveSession();
        FormClosed += (_, _) => { _statsTimer.Stop(); _probeTimer.Stop(); foreach (var p in AllPanes()) p.Close(); };
        Shown += async (_, _) => await InitializeAsync();

        _statsTimer.Tick += (_, _) => UpdateStatus();
        _probeTimer.Tick += async (_, _) => await ProbeTickAsync();
    }

    // Addons folder resolution order: repo checkout next to the exe's parent
    // folders, then the build-output copy shipped beside the exe.
    internal static string ResolveAddonsFolder()
    {
        var candidates = new[]
        {
            Path.Combine(AppContext.BaseDirectory, "addons"),
            Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "addons")),
            Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "addons")),
            Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "addons"))
        };

        foreach (var dir in candidates)
        {
            try
            {
                if (Directory.Exists(dir) &&
                    Directory.EnumerateFiles(dir, "*.user.js").Any())
                {
                    return Path.GetFullPath(dir);
                }
            }
            catch { }
        }

        return Path.GetFullPath(candidates[0]);
    }

    private IEnumerable<Pane> AllPanes() => _games.Concat(_streams);

    private void BuildToolbar()
    {
        _modeButton = Button("Hidden panes: Background", (_, _) => ToggleStreamMode());

        var items = new Control[]
        {
            Button("Reload A", (_, _) => _games.ElementAtOrDefault(0)?.View.Reload()),
            Button("Reload B", (_, _) => _games.ElementAtOrDefault(1)?.View.Reload()),
            Button("DevTools A", (_, _) => _games.ElementAtOrDefault(0)?.View.OpenDevToolsWindow()),
            Button("DevTools B", (_, _) => _games.ElementAtOrDefault(1)?.View.OpenDevToolsWindow()),
            _addonsPicker,
            _streamPicker,
            Button("+ Stream", async (_, _) => await AddStreamAsync()),
            Button("Close stream", (_, _) => CloseActiveStream()),
            Button("Foreground", (_, _) => SetAllStreamsForeground()),
            Button("Background", (_, _) => SetAllStreamsBackground()),
            Button("Mute/Unmute", (_, _) => UnmuteActiveStream()),
            _modeButton,
            _probeToggle,
            _status
        };

        var x = 8;
        foreach (var c in items)
        {
            c.Location = new Point(x, 6);
            _toolbar.Controls.Add(c);
            x += c.Width + 6;
        }

        _streamPicker.SelectedIndexChanged += (_, _) =>
        {
            _activeStream = _streamPicker.SelectedIndex >= 0 ? _streams[_streamPicker.SelectedIndex] : null;
            LayoutPanes();
        };

        // The picker doubles as a "Reload addons" action: selecting the first
        // entry re-provisions Tampermonkey + refreshes the native loader and
        // reloads every game pane. Other entries just list loaded scripts.
        _addonsPicker.SelectedIndexChanged += async (_, _) =>
        {
            if (_addonsPicker.SelectedIndex != 0) return;
            await ReloadAddonsAsync();
        };

        _probeToggle.CheckedChanged += (_, _) =>
        {
            if (_probeToggle.Checked) _probeTimer.Start();
            else _probeTimer.Stop();
        };

        UpdateModeButtonText();
    }

    private void ToggleStreamMode()
    {
        _inactiveStreamMode = _inactiveStreamMode == StreamMode.Background
            ? StreamMode.Parked : StreamMode.Background;
        foreach (var s in _streams) s.Mode = _inactiveStreamMode;
        SaveSession();
        UpdateModeButtonText();
        LayoutPanes();
    }

    private void UpdateModeButtonText() =>
        _modeButton.Text = $"Hidden panes: {(_inactiveStreamMode == StreamMode.Background ? "Background" : "Parked")}";

    private static Button Button(string text, EventHandler handler)
    {
        var b = new Button { Text = text, AutoSize = true, Height = 28 };
        b.Click += handler;
        return b;
    }

    private async Task InitializeAsync()
    {
        try
        {
            Directory.CreateDirectory(AppConfig.GameUserDataFolder);
            Directory.CreateDirectory(AppConfig.StreamUserDataFolder);

            // --- Addon plumbing (must happen BEFORE the WebView2 environments
            // exist: the jsonImport policy is read at browser startup and the
            // loopback provisioning server must be listening before Tampermonkey's
            // background page boots). Previously nothing ever called into
            // ExtensionManager, so Tampermonkey was never installed and no
            // userscript was ever injected — "scripts not loaded at all".
            var addonsFolder = ResolveAddonsFolder();

            _extensions = new ExtensionManager(
                Path.Combine(AppConfig.Root, "Extensions", "Tampermonkey"),
                addonsFolder);

            try
            {
                _tampermonkeyState = _extensions.PrepareTampermonkeyProvisioning();
                Log($"tampermonkey provisioning: {_tampermonkeyState}");
            }
            catch (Exception ex)
            {
                _tampermonkeyState = "provisioning failed";
                Log($"tampermonkey provisioning FAILED: {ex.Message}");
            }

            // Native injector fallback so addons still run when Tampermonkey
            // cannot be installed (missing package, policy not honored, etc.).
            _userscripts = new UserscriptLoader(addonsFolder);
            Log($"native userscript loader: {_userscripts.Scripts.Count} script(s) parsed from {addonsFolder}");

            RefreshAddonsPicker();

            _gameEnv = await CoreWebView2Environment.CreateAsync(
                null, AppConfig.GameUserDataFolder,
                new CoreWebView2EnvironmentOptions
                {
                    AdditionalBrowserArguments = AppConfig.GameBrowserArguments,
                    AreBrowserExtensionsEnabled = true
                });

            // All stream accounts share ONE user data folder so their profiles
            // share Chromium's process pool (lower RAM per extra account). The
            // cookies/logins stay separated by profile name (Stream1, Stream2...).
            _streamEnv = await CoreWebView2Environment.CreateAsync(
                null, AppConfig.StreamUserDataFolder,
                new CoreWebView2EnvironmentOptions
                {
                    AdditionalBrowserArguments = AppConfig.StreamBrowserArguments,
                    AreBrowserExtensionsEnabled = true
                });

            foreach (var spec in SessionStore.Load())
                await AddPaneAsync(spec);

            LayoutPanes();
            _statsTimer.Start();
            if (_probeToggle.Checked) _probeTimer.Start();
            UpdateStatus();
            Log($"startup: {_games.Count} game pane(s), {_streams.Count} stream pane(s); " +
                $"stream accounts: {string.Join(',', AppConfig.LoadStreamProfiles())}");
        }
        catch (Exception ex)
        {
            _status.Text = "Startup failed";
            MessageBox.Show(this, ex.ToString(), "Idle Shell startup failure",
                MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
    }

    private async Task AddPaneAsync(PaneSpec spec)
    {
        var env = spec.Kind == PaneKind.Game ? _gameEnv! : _streamEnv!;

        // Game profiles get Tampermonkey (which owns the provisioned addons).
        var installTampermonkey =
            spec.Kind == PaneKind.Game && _extensions is not null;

        // Native injection fallback: only when Tampermonkey is not in play for
        // this pane, so scripts never run twice.
        var nativeScripts =
            spec.Kind == PaneKind.Game && (!installTampermonkey || _nativeFallback)
                ? _userscripts
                : null;

        var pane = await Pane.CreateAsync(env, Handle, spec,
            installTampermonkey ? _extensions : null, nativeScripts);

        if (spec.Kind == PaneKind.Game && installTampermonkey && !pane.TampermonkeyReady)
        {
            // Tampermonkey could not be installed/started in this profile —
            // enable the native injector on every game pane from now on and
            // reload this one with it attached.
            Log($"Tampermonkey unavailable ({pane.TampermonkeyError}); " +
                "falling back to the native userscript injector");

            if (!_nativeFallback)
            {
                _nativeFallback = true;
                foreach (var g in _games) await EnableNativeFallbackAsync(g);
                await EnableNativeFallbackAsync(pane);
            }
            else
            {
                await EnableNativeFallbackAsync(pane);
            }
        }

        pane.MessageReceived += OnPaneMessage;
        pane.PopupRequested += OnPopupRequested;

        if (spec.Kind == PaneKind.Game)
        {
            // Navigation backstop: a game page that bypasses the userscript and
            // navigates to twitch/kick gets bounced into background stream panes.
            pane.View.NavigationStarting += (_, e) => GamePaneNavigating(pane, _, e);
            _games.Add(pane);
        }
        else
        {
            _streams.Add(pane);
            _streamPicker.Items.Add(spec.Title);
            _streamPicker.SelectedIndex = _streams.Count - 1; // triggers layout
        }
    }

    private static async Task EnableNativeFallbackAsync(Pane pane)
    {
        // The bootstrap runs at document creation, so a reload is required for
        // it to take effect on the already-loaded page.
        await pane.AttachUserscriptFallback();
        pane.View.Reload();
    }

    private void RefreshAddonsPicker()
    {
        _addonsPicker.Items.Clear();
        _addonsPicker.Items.Add("Reload addons (all games)");
        foreach (var script in _userscripts?.Scripts ?? [])
            _addonsPicker.Items.Add(script.Name);
        if (_addonsPicker.Items.Count > 0) _addonsPicker.SelectedIndex = 0;
    }

    private async Task ReloadAddonsAsync()
    {
        if (_extensions is not null)
        {
            try
            {
                _tampermonkeyState = _extensions.PrepareTampermonkeyProvisioning();
                Log($"addons reloaded: {_tampermonkeyState}");
            }
            catch (Exception ex)
            {
                _tampermonkeyState = "provisioning failed";
                Log($"addon reload FAILED: {ex.Message}");
            }
        }

        _userscripts = new UserscriptLoader(
            _userscripts?.Folder ?? ResolveAddonsFolder());
        RefreshAddonsPicker();

        foreach (var game in _games.ToList())
        {
            if (_nativeFallback) await game.AttachUserscriptFallback();
            game.View.Reload();
        }
    }

    // --- Stream link routing -------------------------------------------------
    // A pokeidle page (or any pane) reported a twitch.tv/kick.com link. Fan it
    // out to one hidden Background-mode stream pane per configured Twitch/Kick
    // account profile ("each link loaded on two distinct twitch accounts").
    private void OnPaneMessage(Pane pane, HostMessage msg)
    {
        if (msg.Type != "link" || !IsStreamUrl(msg.Url)) return;
        _ = RouteStreamLinkAsync(msg.Url, $"userscript ({msg.Source})");
    }

    // Backstops for pages that bypass the userscript: popups from game panes,
    // and game panes that somehow navigated straight onto a stream host.
    private void OnPopupRequested(Pane pane, string url)
    {
        if (pane.Spec.Kind == PaneKind.Game && IsStreamUrl(url))
        {
            _ = RouteStreamLinkAsync(url, "popup backstop");
            return;
        }
        // Stream-pane popups (Twitch OAuth "Log in with Twitch" etc.): open them
        // as a visible pane in the SAME profile so window.opener keeps working.
        _ = OpenStreamPopupAsync(pane, url);
    }

    // Backstop for game panes that somehow navigate straight onto a stream host
    // (e.g. location.href assignment the userscript can't intercept). Cancels
    // the navigation and bounces the URL into the background stream panes.
    private void GamePaneNavigating(Pane pane, object? sender, CoreWebView2NavigationStartingEventArgs e)
    {
        if (!IsStreamUrl(e.Uri)) return;
        e.Cancel = true; // never leave the game page
        _ = RouteStreamLinkAsync(e.Uri, "navigation backstop");
    }

    public static bool IsStreamUrl(string? url) =>
        url is not null && StreamUrlRegex.IsMatch(url);

    private async Task RouteStreamLinkAsync(string url, string via)
    {
        try
        {
            var channel = ChannelLabel(url);
            var created = 0;
            foreach (var profile in AppConfig.LoadStreamProfiles())
            {
                var existing = _streams.FirstOrDefault(s => s.Spec.Profile == profile);
                if (existing is not null)
                {
                    existing.View.Navigate(url);
                    continue;
                }
                await AddPaneAsync(new PaneSpec($"{channel} · {profile}", url, profile,
                    PaneKind.Stream, _inactiveStreamMode));
                created++;
            }
            Log($"Routed {url} to {AppConfig.LoadStreamProfiles().Count} stream account(s) via {via}" +
                (created > 0 ? $" ({created} new pane(s) created)" : ""));
            SaveSession();
        }
        catch (Exception ex)
        {
            Log($"link routing failed: {ex.Message}");
        }
    }

    private async Task OpenStreamPopupAsync(Pane opener, string url)
    {
        try
        {
            var title = Uri.TryCreate(url, UriKind.Absolute, out var u)
                ? u.Host.Replace("www.", "") + u.AbsolutePath : url;
            await AddPaneAsync(new PaneSpec(title, url, opener.Spec.Profile,
                PaneKind.Stream, _inactiveStreamMode));
            _streamPicker.SelectedIndex = _streams.Count - 1; // show it
            Log($"Opened popup in foreground: {url} (profile {opener.Spec.Profile})");
        }
        catch (Exception ex)
        {
            Log($"popup open failed: {ex.Message}");
        }
    }

    private static string ChannelLabel(string url)
    {
        try
        {
            var u = new Uri(url);
            var seg = u.AbsolutePath.Split('/', StringSplitOptions.RemoveEmptyEntries);
            var host = u.Host.Contains("kick", StringComparison.OrdinalIgnoreCase) ? "kick" : "twitch";
            return $"{host}/{(seg.Length > 0 ? seg[0] : "")}";
        }
        catch { return "stream"; }
    }

    private void Log(string message)
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(AppConfig.ProbeCsvFile)!);
            File.AppendAllText(AppConfig.LogFile, $"{DateTime.Now:yyyy-MM-dd HH:mm:ss} {message}{Environment.NewLine}");
        }
        catch { }
        Console.Error.WriteLine($"[IdleShell] {message}");
    }

    // --- Toolbar buttons -------------------------------------------------------

    // "Turn them on": bring every stream pane to Foreground (visible, rendering).
    private void SetAllStreamsForeground()
    {
        if (_streams.Count == 0) return;
        _activeStream ??= _streams[0];
        LayoutPanes();
        SaveSession();
    }

    // "Set them to background": hide every stream pane at once. No stream is
    // shown in the layout, so the games take the full width and each hidden
    // pane stops compositing (Background mode) or parks off-screen (Parked).
    private void SetAllStreamsBackground()
    {
        _activeStream = null;
        _streamPicker.SelectedIndex = -1; // suppresses the SelectedIndexChanged relayout
        LayoutPanes();
        SaveSession();
    }

    private void UnmuteActiveStream()
    {
        if (_activeStream is null) return;
        _activeStream.View.IsMuted = !_activeStream.View.IsMuted;
    }

    private async Task AddStreamAsync()
    {
        var url = Prompt("Stream URL", "https://www.twitch.tv/");
        if (string.IsNullOrWhiteSpace(url)) return;
        if (!url.Contains("://")) url = "https://" + url;

        var title = Uri.TryCreate(url, UriKind.Absolute, out var u)
            ? u.Host.Replace("www.", "") + u.AbsolutePath
            : url;

        await AddPaneAsync(new PaneSpec(title, url, "Streams", PaneKind.Stream, _inactiveStreamMode));
    }

    private void CloseActiveStream()
    {
        if (_activeStream is null) return;
        var index = _streams.IndexOf(_activeStream);
        _activeStream.MessageReceived -= OnPaneMessage;
        _activeStream.PopupRequested -= OnPopupRequested;
        _activeStream.Close();
        _streams.RemoveAt(index);
        _streamPicker.Items.RemoveAt(index);
        _activeStream = null;
        if (_streams.Count > 0) _streamPicker.SelectedIndex = Math.Min(index, _streams.Count - 1);
        else LayoutPanes();
        SaveSession();
    }

    private void LayoutPanes()
    {
        var top = AppConfig.ToolbarHeight;
        var height = Math.Max(0, ClientSize.Height - top);
        var width = ClientSize.Width;

        var streamWidth = _activeStream is null ? 0 : width / 3;
        var gamesWidth = width - streamWidth;
        var each = _games.Count > 0 ? gamesWidth / _games.Count : 0;

        for (var i = 0; i < _games.Count; i++)
            _games[i].Show(new Rectangle(i * each, top, each, height));

        foreach (var s in _streams)
        {
            if (s == _activeStream) { s.Show(new Rectangle(gamesWidth, top, streamWidth, height)); continue; }
            if (s.Mode == StreamMode.Parked) s.Park();
            else s.Hide(); // Background: no compositing, timers kept alive by flags.
        }
    }

    // Probe: every 30s, run the visibility/timer-drift script in each pane and
    // append one CSV row per pane to AppConfig.ProbeCsvFile. This settles whether
    // hidden (IsVisible=false) panes still report "visible" and keep unthrottled
    // timers — i.e. whether streams/addons survive background mode.
    private async Task ProbeTickAsync()
    {
        if (_probing) return; // skip a tick if the previous round is still running
        _probing = true;
        try
        {
            EnsureProbeHeader();
            foreach (var p in AllPanes())
            {
                var raw = await p.ProbeAsync();
                var fields = ParseProbeJson(raw ?? "");
                var state = p.Spec.Kind == PaneKind.Stream && !p.IsForeground
                    ? p.Mode.ToString() : "Foreground";
                var line = string.Join(',',
                    DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss"),
                    Csv(p.Spec.Title), Csv(p.Spec.Kind.ToString()), Csv(state),
                    Csv(fields.vis), Csv(fields.hidden), Csv(fields.focus), Csv(fields.drift));
                File.AppendAllText(AppConfig.ProbeCsvFile, line + Environment.NewLine);
            }
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"[IdleShell] probe: {ex.Message}");
        }
        finally { _probing = false; }
    }

    private static void EnsureProbeHeader()
    {
        Directory.CreateDirectory(Path.GetDirectoryName(AppConfig.ProbeCsvFile)!);
        if (!File.Exists(AppConfig.ProbeCsvFile))
            File.WriteAllText(AppConfig.ProbeCsvFile,
                "timestamp,title,kind,state,visibility,hidden,focus,driftMs" + Environment.NewLine);
    }

    // ExecuteScriptAsync returns a JSON-encoded string containing the page's JSON,
    // i.e. double-encoded. Decode once as a string, then parse the inner object.
    private static (string vis, string hidden, string focus, string drift) ParseProbeJson(string outer)
    {
        try
        {
            using var doc = System.Text.Json.JsonDocument.Parse(outer);
            var inner = doc.RootElement.GetString();
            if (inner is null) return ("?", "?", "?", "?");
            using var page = System.Text.Json.JsonDocument.Parse(inner);
            string Field(string key) =>
                page.RootElement.TryGetProperty(key, out var v) ? v.ToString() : "?";
            return (Field("visibility"), Field("hidden"), Field("focus"), Field("driftMs"));
        }
        catch { return ("error", "error", "error", "error"); }
    }

    private static string Csv(string s) =>
        s.Contains(',') || s.Contains('"') ? "\"" + s.Replace("\"", "\"\"") + "\"" : s;

    private void UpdateStatus()
    {
        try
        {
            if (_gameEnv is null) return;
            var infos = _gameEnv.GetProcessInfos();
            var streamInfos = _streamEnv?.GetProcessInfos();
            var hidden = _streams.Count(s => !s.IsForeground);
            _status.Text = $"Game procs: {infos.Count} · Stream procs: {streamInfos?.Count ?? 0}" +
                           $" · Streams: {_streams.Count - hidden} fg / {hidden} bg" +
                           $" · Addons: {(_nativeFallback ? "native injector" : "Tampermonkey")} " +
                           $"({_userscripts?.Scripts.Count ?? 0} scripts)";
        }
        catch
        {
            // Keep the last known status; show the addon state if the base
            // text was never set (startup failure path).
            if (_status.Text.Length == 0)
                _status.Text = $"Addons: {(_tampermonkeyState)}";
        }
    }

    private void SaveSession() =>
        SessionStore.Save(AllPanes().Select(p => p.Snapshot()));

    private string? Prompt(string title, string initial)
    {
        using var form = new Form
        {
            Text = title, FormBorderStyle = FormBorderStyle.FixedDialog,
            StartPosition = FormStartPosition.CenterParent, ClientSize = new Size(460, 70),
            MaximizeBox = false, MinimizeBox = false
        };
        var box = new TextBox { Left = 10, Top = 10, Width = 440, Text = initial };
        var ok = new Button { Text = "OK", Left = 290, Top = 38, DialogResult = DialogResult.OK };
        var cancel = new Button { Text = "Cancel", Left = 375, Top = 38, DialogResult = DialogResult.Cancel };
        form.Controls.AddRange([box, ok, cancel]);
        form.AcceptButton = ok;
        form.CancelButton = cancel;
        return form.ShowDialog(this) == DialogResult.OK ? box.Text.Trim() : null;
    }
}
