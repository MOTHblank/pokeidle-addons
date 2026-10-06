using System.Text;
using System.Text.RegularExpressions;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

internal sealed class MainForm : Form
{
    private readonly Panel _toolbar;
    // Tab strip listing every stream account (up to 10); the selected tab's pane
    // is foregrounded in the stream area below it. Panes are parented to the form,
    // not the TabPage, so the strip only acts as a selector.
    private readonly TabControl _streamTabs = new()
    {
        Dock = DockStyle.Top,
        Appearance = TabAppearance.FlatButtons,
        ItemSize = new Size(150, 26),
        SizeMode = TabSizeMode.Fixed,
        Height = 30
    };
    private readonly Label _status = new()
        { AutoSize = true, Padding = new Padding(3, 4, 3, 0), Anchor = AnchorStyles.Top | AnchorStyles.Right };
    private readonly ComboBox _addonsPicker = new() { DropDownStyle = ComboBoxStyle.DropDownList, Width = 190 };

    private CoreWebView2Environment? _gameEnv;
    private CoreWebView2Environment? _streamEnv;
    private readonly AccountManager _accounts = AccountManager.Load();
    private readonly List<Pane> _games = [];
    private readonly List<StreamSlot> _slots = [];
    private readonly Dictionary<string, Pane> _extraPanes =
        new(StringComparer.OrdinalIgnoreCase); // OAuth popups etc., keyed by profile id
    private UserscriptLoader? _userscripts;
    private readonly System.Windows.Forms.Timer _statsTimer = new() { Interval = 5000 };
    private readonly System.Windows.Forms.Timer _probeTimer = new() { Interval = 30000 };
    private readonly CheckBox _probeToggle = new()
        { Text = "Probe", AutoSize = true, Checked = true, Padding = new Padding(3, 6, 3, 0) };
    private StreamMode _inactiveStreamMode = StreamMode.Background;
    private bool _probing;
    private bool _suppressTabEvent;
    private bool _suppressAddonPickerEvent;
    private Button _modeButton = null!;   // assigned in BuildToolbar, called from the ctor
    private Button _allBackgroundButton = null!;
    private ToolStripMenuItem _visibleStreamsItem = null!;
    private int _activeTabIndex = -1;     // -1 = "All background"
    private static readonly Regex StreamUrlRegex =
        new($@"^https?://{AppConfig.StreamHostPattern}/", RegexOptions.IgnoreCase | RegexOptions.Compiled);

    // One tab per stream account: a lazily-created pane plus its last routed URL.
    private sealed class StreamSlot(Account account)
    {
        public Account Account { get; set; } = account;
        public Pane? Pane;
        public string? Url;
        public TabPage Tab { get; } = new(account.DisplayLabel);
    }

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
        // The tab strip keeps its full width in the layout even when all streams
        // are backgrounded, so panes must follow every dock-size change.
        _streamTabs.SizeChanged += (_, _) => LayoutPanes();
        FormClosing += (_, _) => SaveSession();
        FormClosed += (_, _) => { _statsTimer.Stop(); _probeTimer.Stop(); foreach (var p in AllPanes()) p.Close(); };
        Shown += async (_, _) => await InitializeAsync();

        _statsTimer.Tick += (_, _) => UpdateStatus();
        _probeTimer.Tick += async (_, _) => await ProbeTickAsync();
    }

    private IEnumerable<Pane> AllPanes() =>
        _games.Concat(_slots.Select(s => s.Pane)).Concat(_extraPanes.Values).OfType<Pane>();
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

    private void BuildToolbar()
    {
        var menu = new MenuStrip { Dock = DockStyle.Left, GripStyle = ToolStripGripStyle.Hidden };
        var viewMenu = new ToolStripMenuItem("View");
        _visibleStreamsItem = new ToolStripMenuItem($"Visible streams: {_accounts.VisibleStreamCount}");
        for (var n = 1; n <= AccountManager.MaxStreamAccounts; n++)
        {
            var count = n;
            _visibleStreamsItem.DropDownItems.Add(new ToolStripMenuItem($"{count}", null,
                (_, _) => SetVisibleStreamCount(count))
            { ShowShortcutKeys = false });
        }
        viewMenu.DropDownItems.Add(_visibleStreamsItem);
        menu.Items.Add(viewMenu);

        _modeButton = Button("Hidden panes: Background", (_, _) => ToggleStreamMode());
        _allBackgroundButton = Button("Hide all streams", (_, _) => SetAllStreamsBackground());

        var items = new Control[]
        {
            menu,
            Button("Reload games", (_, _) => ReloadGames()),
            Button("DevTools A", (_, _) => _games.ElementAtOrDefault(0)?.View.OpenDevToolsWindow()),
            Button("DevTools B", (_, _) => _games.ElementAtOrDefault(1)?.View.OpenDevToolsWindow()),
            Button("Accounts…", (_, _) => OpenAccountsDialog()),
            _allBackgroundButton,
            Button("+ Stream", async (_, _) => await AddStreamManualAsync()),
            Button("Mute/Unmute", (_, _) => MuteActiveStream()),
            _addonsPicker,
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

        // The picker doubles as a "Reload addons" action: selecting the first
        // reloads every game pane. Other entries just list loaded scripts.
        _addonsPicker.SelectedIndexChanged += async (_, _) =>
        {
            if (_suppressAddonPickerEvent || _addonsPicker.SelectedIndex != 0) return;
            await ReloadAddonsAsync();
        };

        _probeToggle.CheckedChanged += (_, _) =>
        {
            if (_probeToggle.Checked) _probeTimer.Start();
            else _probeTimer.Stop();
        };

        _streamTabs.SelectedIndexChanged += (_, _) =>
        {
            if (_suppressTabEvent) return;
            _activeTabIndex = _streamTabs.SelectedIndex;
            LayoutPanes();
            SaveSession();
        };

        UpdateModeButtonText();
        UpdateVisibleStreamsText();
    }

    private void ToggleStreamMode()
    {
        _inactiveStreamMode = _inactiveStreamMode == StreamMode.Background
            ? StreamMode.Parked : StreamMode.Background;
        foreach (var slot in _slots)
            if (slot.Pane is { } p) p.Mode = _inactiveStreamMode;
        SaveSession();
        UpdateModeButtonText();
        LayoutPanes();
    }

    private void UpdateModeButtonText() =>
        _modeButton.Text = $"Hidden panes: {(_inactiveStreamMode == StreamMode.Background ? "Background" : "Parked")}";

    private void UpdateVisibleStreamsText() =>
        _visibleStreamsItem.Text = $"Visible streams: {_accounts.VisibleStreamCount}";

    private void SetVisibleStreamCount(int n)
    {
        _accounts.SetVisibleStreamCount(n);
        UpdateVisibleStreamsText();
        LayoutPanes();
        SaveSession();
    }

    private static Button Button(string text, EventHandler handler)
    {
        var b = new Button { Text = text, AutoSize = true, Height = 28 };
        b.Click += handler;
        return b;
    }

    private void ReloadGames()
    {
        foreach (var g in _games) g.View.Reload();
    }

    private async Task InitializeAsync()
    {
        try
        {
            Directory.CreateDirectory(AppConfig.GameUserDataFolder);
            Directory.CreateDirectory(AppConfig.StreamUserDataFolder);

            var addonsFolder = ResolveAddonsFolder();
            _userscripts = new UserscriptLoader(addonsFolder);
            Log($"native userscript loader: {_userscripts.Scripts.Count} script(s) parsed from {addonsFolder}");

            RefreshAddonsPicker();

            try
            {
                var version = CoreWebView2Environment.GetAvailableBrowserVersionString();
                Log($"WebView2 Runtime: {version}");
            }
            catch (WebView2RuntimeNotFoundException ex)
            {
                throw new InvalidOperationException(
                    "Microsoft Edge WebView2 Runtime is not installed. " +
                    "Install the Evergreen WebView2 Runtime and start Idle Shell again.", ex);
            }

            _gameEnv = await CoreWebView2Environment.CreateAsync(
                null, AppConfig.GameUserDataFolder,
                new CoreWebView2EnvironmentOptions
                {
                    AdditionalBrowserArguments = AppConfig.GameBrowserArguments,
                    AreBrowserExtensionsEnabled = true
                });

            // All stream accounts share ONE user data folder so their profiles
            // share Chromium's process pool (lower RAM per extra account). The
            // cookies/logins stay separated by profile name (Stream1..Stream10).
            _streamEnv = await CoreWebView2Environment.CreateAsync(
                null, AppConfig.StreamUserDataFolder,
                new CoreWebView2EnvironmentOptions
                {
                    AdditionalBrowserArguments = AppConfig.StreamBrowserArguments,
                    AreBrowserExtensionsEnabled = true
                });

            var session = SessionStore.Load();
            foreach (var spec in session.Where(s => s.Kind == PaneKind.Game))
                await AddGamePaneAsync(spec);

            // One tab per stream account. Panes are created lazily — on first
            // routed link, manual open, restore, or login — so 10 configured
            // accounts don't mean 10 renderers until they actually play video.
            foreach (var account in _accounts.StreamAccounts)
            {
                var slot = new StreamSlot(account);
                _slots.Add(slot);
                _streamTabs.TabPages.Add(slot.Tab);
            }

            // Restore previously-open stream panes (URLs from the last session).
            foreach (var spec in session.Where(s => s.Kind == PaneKind.Stream))
            {
                var slot = SlotForProfile(spec.Profile);
                if (slot is null || string.IsNullOrWhiteSpace(spec.Url)) continue;
                if (IsStreamUrl(spec.Url))
                {
                    await EnsureStreamPaneAsync(slot, spec.Url);
                }
                else
                {
                    // Popup/login pane that lived in a stream profile — reopen it.
                    await AddExtraPaneAsync(new PaneSpec(spec.Title, spec.Url, spec.Profile,
                        PaneKind.Stream, spec.Mode));
                }
            }

            _accounts.Changed += OnAccountsChanged;

            // Default: first visible stream tab in foreground.
            var restoredActive = session.FirstOrDefault(s => s.Kind == PaneKind.ActiveStreamMarker)?.Profile;
            var startIndex = 0;
            if (restoredActive is not null)
            {
                var idx = _slots.FindIndex(s =>
                    string.Equals(s.Account.Id, restoredActive, StringComparison.OrdinalIgnoreCase));
                if (idx >= 0) startIndex = idx;
            }
            if (_slots.Count > 0) SelectTab(startIndex);
            else _activeTabIndex = -1;

            // Add the tab strip after the toolbar so docking puts it directly
            // below the toolbar; panes stay children of the form itself.
            Controls.Add(_streamTabs);
            LayoutPanes();
            _statsTimer.Start();
            if (_probeToggle.Checked) _probeTimer.Start();
            UpdateStatus();
            Log($"startup: {_games.Count} game pane(s), {_slots.Count} stream account(s) " +
                $"({EnabledStreamSlots().Count()} enabled), visible streams: {_accounts.VisibleStreamCount}; " +
                $"accounts file: {AppConfig.AccountsFile}");
        }
        catch (Exception ex)
        {
            _status.Text = "Startup failed";
            MessageBox.Show(this, ex.ToString(), "Idle Shell startup failure",
                MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
    }

    // --- Accounts --------------------------------------------------------------

    private void OpenAccountsDialog()
    {
        using var dlg = new AccountsDialog(
            _accounts,
            acc => PaneForAccount(acc) is not null,
            acc => _ = ShowAccountForLoginAsync(acc));
        dlg.ShowDialog(this);
        LayoutPanes();
    }

    private Pane? PaneForAccount(Account acc) =>
        _slots.FirstOrDefault(s => string.Equals(s.Account.Id, acc.Id, StringComparison.OrdinalIgnoreCase))?.Pane
        ?? (_extraPanes.TryGetValue(acc.Id, out var p) ? p : null);

    private void OnAccountsChanged()
    {
        // Keep slots and tabs in sync with the registry.
        foreach (var slot in _slots)
        {
            var current = _accounts.Find(slot.Account.Id);
            if (current is not null) slot.Account = current;
        }

        var removed = _slots.Where(s => _accounts.Find(s.Account.Id) is null).ToArray();
        foreach (var slot in removed)
        {
            CloseSlot(slot);
            _slots.Remove(slot);
        }

        foreach (var account in _accounts.StreamAccounts)
        {
            if (_slots.All(s => !string.Equals(s.Account.Id, account.Id, StringComparison.OrdinalIgnoreCase)))
            {
                var slot = new StreamSlot(account);
                _slots.Add(slot);
                _streamTabs.TabPages.Add(slot.Tab);
            }
        }

        RebuildTabTitles();
        UpdateVisibleStreamsText();
        LayoutPanes();
        SaveSession();
    }

    private void RebuildTabTitles()
    {
        _suppressTabEvent = true;
        try
        {
            for (var i = 0; i < _slots.Count && i < _streamTabs.TabPages.Count; i++)
                _streamTabs.TabPages[i].Text = _slots[i].Account.DisplayLabel;
        }
        finally { _suppressTabEvent = false; }
    }

    private async Task ShowAccountForLoginAsync(Account acc)
    {
        var slot = SlotForAccount(acc);
        if (slot is not null)
        {
            await EnsureStreamPaneAsync(slot, slot.Url ?? AccountManager.LoginUrl(acc.Service));
            SelectTab(_slots.IndexOf(slot));
            return;
        }
        // Game account: reload/focus is enough — sign-in happens on pokeidle.io.
        var game = _games.FirstOrDefault(g =>
            string.Equals(g.Spec.Profile, acc.Id, StringComparison.OrdinalIgnoreCase));
        game?.View.Navigate(AccountManager.LoginUrl(acc.Service));
    }

    private StreamSlot? SlotForAccount(Account acc) =>
        _slots.FirstOrDefault(s => string.Equals(s.Account.Id, acc.Id, StringComparison.OrdinalIgnoreCase));

    private StreamSlot? SlotForProfile(string profile) =>
        _slots.FirstOrDefault(s => string.Equals(s.Account.Id, profile, StringComparison.OrdinalIgnoreCase));

    // --- Panes -----------------------------------------------------------------

    private async Task AddGamePaneAsync(PaneSpec spec)
    {
        var env = spec.Kind == PaneKind.Game ? _gameEnv! : _streamEnv!;

        var pane = await Pane.CreateAsync(env, Handle, spec, _userscripts);

        pane.MessageReceived += OnPaneMessage;
        pane.PopupRequested += OnPopupRequested;
        // Navigation backstop: a game page that bypasses the userscript and
        // navigates to twitch/kick gets bounced into background stream panes.
        pane.View.NavigationStarting += (_, e) => GamePaneNavigating(pane, _, e);
        _games.Add(pane);
    }

    private async Task EnsureStreamPaneAsync(StreamSlot slot, string url)
    {
        if (slot.Pane is not null)
        {
            slot.Url = url;
            if (slot.Pane.View.Source != url) slot.Pane.View.Navigate(url);
            return;
        }
        var spec = new PaneSpec(slot.Account.DisplayLabel, url, slot.Account.Id,
            PaneKind.Stream, _inactiveStreamMode);
        var pane = await Pane.CreateAsync(_streamEnv!, Handle, spec);
        pane.MessageReceived += OnPaneMessage;
        pane.PopupRequested += OnPopupRequested;
        slot.Pane = pane;
        slot.Url = url;
        LayoutPanes();
    }

    // Extra pane inside an existing stream profile (OAuth popups, login detours).
    private async Task AddExtraPaneAsync(PaneSpec spec)
    {
        if (_extraPanes.TryGetValue(spec.Profile, out var old))
        {
            DetachAndClose(old);
            _extraPanes.Remove(spec.Profile);
        }
        var pane = await Pane.CreateAsync(_streamEnv!, Handle, spec);
        pane.MessageReceived += OnPaneMessage;
        pane.PopupRequested += OnPopupRequested;
        _extraPanes[spec.Profile] = pane;
        LayoutPanes();
    }

    private void DetachAndClose(Pane pane)
    {
        pane.MessageReceived -= OnPaneMessage;
        pane.PopupRequested -= OnPopupRequested;
        pane.Close();
    }

    private void CloseSlot(StreamSlot slot)
    {
        if (slot.Pane is { } p) DetachAndClose(p);
        if (_extraPanes.TryGetValue(slot.Account.Id, out var extra))
        {
            DetachAndClose(extra);
            _extraPanes.Remove(slot.Account.Id);
        }
        _streamTabs.TabPages.Remove(slot.Tab);
    }

    private void RefreshAddonsPicker()
    {
        _suppressAddonPickerEvent = true;
        try
        {
            _addonsPicker.Items.Clear();
            _addonsPicker.Items.Add("Reload addons (all games)");
            foreach (var script in _userscripts?.Scripts ?? [])
                _addonsPicker.Items.Add(script.Name);
            if (_addonsPicker.Items.Count > 0)
                _addonsPicker.SelectedIndex = 0;
        }
        finally
        {
            _suppressAddonPickerEvent = false;
        }
    }

    private async Task ReloadAddonsAsync()
    {
        try
        {
            var folder = _userscripts?.Folder ?? ResolveAddonsFolder();
            _userscripts = new UserscriptLoader(folder);
            RefreshAddonsPicker();

            foreach (var game in _games.ToList())
            {
                await game.AttachUserscriptAsync(_userscripts);
                game.View.Reload();
            }

            Log($"addons reloaded: {_userscripts.Scripts.Count} script(s)");
        }
        catch (Exception ex)
        {
            Log($"addon reload FAILED: {ex}");
            MessageBox.Show(this, ex.ToString(), "Addon reload failure",
                MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
    }

    // --- Stream link routing -------------------------------------------------
    // A pokeidle page (or any pane) reported a twitch.tv/kick.com link. Fan it
    // out to one pane per enabled account of the matching service (Twitch/Kick),
    // so each link plays simultaneously on every login — up to 10 at once.
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
            var targets = _accounts.StreamAccountsForUrl(url);
            if (targets.Count == 0)
            {
                Log($"no enabled stream accounts for {url} (via {via}) — ignored");
                return;
            }
                var created = 0;
            foreach (var account in targets)
            {
                var slot = SlotForAccount(account);
                if (slot is null) continue;
                if (slot.Pane is null) created++;
                await EnsureStreamPaneAsync(slot, url);
            }
            Log($"Routed {url} to {targets.Count} stream account(s) via {via}" +
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
            await AddExtraPaneAsync(new PaneSpec(title, url, opener.Spec.Profile,
                PaneKind.Stream, _inactiveStreamMode));
            var slot = SlotForProfile(opener.Spec.Profile);
            if (slot is not null) SelectTab(_slots.IndexOf(slot));
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
            Directory.CreateDirectory(Path.GetDirectoryName(AppConfig.LogFile)!);
            File.AppendAllText(AppConfig.LogFile, $"{DateTime.Now:yyyy-MM-dd HH:mm:ss} {message}{Environment.NewLine}");
        }
        catch { }
        Console.Error.WriteLine($"[IdleShell] {message}");
    }

    // --- Toolbar buttons ---------------------------------------------------------

    private void SelectTab(int index)
    {
        if (_streamTabs.TabPages.Count == 0) { _activeTabIndex = -1; return; }
        _activeTabIndex = Math.Clamp(index, 0, _streamTabs.TabPages.Count - 1);
        _suppressTabEvent = true;
        _streamTabs.SelectedIndex = _activeTabIndex;
        _suppressTabEvent = false;
        LayoutPanes();
    }

    // "Hide all streams": no stream shown; games take full width. Hidden panes
    // stop compositing but keep running timers/addons (Background mode).
    private void SetAllStreamsBackground()
    {
        _activeTabIndex = -1;
        _suppressTabEvent = true;
        _streamTabs.SelectedIndex = -1;
        _suppressTabEvent = false;
        LayoutPanes();
        SaveSession();
    }

    private void MuteActiveStream()
    {
        var pane = ActiveStreamPane() ?? ForegroundCandidates().FirstOrDefault();
        if (pane is null) return;
        pane.View.IsMuted = !pane.View.IsMuted;
        // Unmuting one audible stream while several are visible: mute the others.
        if (!pane.View.IsMuted)
            foreach (var other in ForegroundCandidates())
                if (other != pane) other.View.IsMuted = true;
    }

    // Manually open a stream URL on the currently selected account only.
    private async Task AddStreamManualAsync()
    {
        var url = Prompt("Stream URL", "https://www.twitch.tv/");
        if (string.IsNullOrWhiteSpace(url)) return;
        if (!url.Contains("://")) url = "https://" + url;
        if (!IsStreamUrl(url))
        {
            MessageBox.Show(this, "Only twitch.tv / kick.com URLs can be opened as stream panes.",
                "Add stream", MessageBoxButtons.OK, MessageBoxIcon.Information);
            return;
        }
        var slot = _activeTabIndex >= 0 && _activeTabIndex < _slots.Count
            ? _slots[_activeTabIndex]
            : _slots.FirstOrDefault(s => s.Account.Enabled);
        if (slot is null)
        {
            MessageBox.Show(this, "No stream accounts configured — add one under Accounts…",
                "Add stream", MessageBoxButtons.OK, MessageBoxIcon.Information);
            return;
        }
        await EnsureStreamPaneAsync(slot, url);
        SaveSession();
    }

    private Pane? ActiveStreamPane()
    {
        if (_activeTabIndex < 0 || _activeTabIndex >= _slots.Count) return null;
        var slot = _slots[_activeTabIndex];
        return slot.Pane ?? (_extraPanes.TryGetValue(slot.Account.Id, out var p) ? p : null);
    }

    private IEnumerable<StreamSlot> EnabledStreamSlots() =>
        _slots.Where(s => s.Account.Enabled);

    // --- Layout --------------------------------------------------------------------
    // Games occupy the left column; the right side hosts a grid of visible stream
    // panes (one tab foregrounded at a time, up to VisibleStreamCount cells). With
    // 10 accounts open, only the visible cells render; the rest run hidden.
    private void LayoutPanes()
    {
        if (_gameEnv is null) return; // init not finished yet

        var top = AppConfig.ToolbarHeight + (_streamTabs.TabPages.Count > 0 ? _streamTabs.Height : 0);
        // The tab strip is docked Top and keeps its full width in the layout even
        // when all streams are backgrounded; hand its reserved band back to the
        // panes by collapsing it to zero height (it grows again on resize).
        if (_streamTabs.TabPages.Count > 0) _streamTabs.SetBounds(0, AppConfig.ToolbarHeight, ClientSize.Width, 0);
        var height = Math.Max(0, ClientSize.Height - top);
        var width = ClientSize.Width;

        var candidates = ForegroundCandidates().ToList();
        var streamWidth = candidates.Count == 0 ? 0 : (candidates.Count > 1 ? width * 2 / 5 : width / 3);
        var gamesWidth = width - streamWidth;
        var each = _games.Count > 0 ? gamesWidth / _games.Count : 0;

        for (var i = 0; i < _games.Count; i++)
            _games[i].Show(new Rectangle(i * each, top, each, height));

        GridLayout(candidates, new Rectangle(gamesWidth, top, streamWidth, height));

        foreach (var slot in _slots)
        {
            if (slot.Pane is null || candidates.Contains(slot.Pane)) continue;
            if (slot.Pane.Mode == StreamMode.Parked) slot.Pane.Park();
            else slot.Pane.Hide(); // Background: no compositing, timers kept alive by flags.
        }
        foreach (var (profile, pane) in _extraPanes)
        {
            if (candidates.Contains(pane)) continue;
            if (pane.Mode == StreamMode.Parked) pane.Park();
            else pane.Hide();
        }
    }

    // Panes that should be rendered right now: the active tab (+ its extra pane)
    // plus any additional visible-account panes up to VisibleStreamCount.
    private List<Pane> ForegroundCandidates()
    {
        var result = new List<Pane>();
        if (_activeTabIndex < 0) return result;

        var visibleAccounts = EnabledStreamSlots().Take(_accounts.VisibleStreamCount).ToList();
        var activeSlot = _activeTabIndex < _slots.Count ? _slots[_activeTabIndex] : null;

        // Always show the selected tab first, even if its account sits beyond the
        // visible-count window.
        if (activeSlot?.Pane is { } ap) result.Add(ap);
        if (activeSlot is not null && _extraPanes.TryGetValue(activeSlot.Account.Id, out var ep))
            result.Add(ep);

        foreach (var slot in visibleAccounts)
        {
            if (slot.Pane is null || slot == activeSlot) continue;
            if (result.Count >= Math.Max(1, _accounts.VisibleStreamCount)) break;
            result.Add(slot.Pane);
        }
        return result;
    }

    // Arrange N panes in a WxH grid inside bounds: columns = ceil(sqrt(N)), rows
    // spread to fill. One pane = full cell (same as before).
    private static void GridLayout(List<Pane> panes, Rectangle bounds)
    {
        if (panes.Count == 0 || bounds.Width <= 0 || bounds.Height <= 0) return;
        var cols = (int)Math.Ceiling(Math.Sqrt(panes.Count));
        var rows = (int)Math.Ceiling(panes.Count / (double)cols);
        var cw = bounds.Width / cols;
        var ch = bounds.Height / rows;
        for (var i = 0; i < panes.Count; i++)
        {
            var r = i / cols;
            var c = i % cols;
            // Last column absorbs rounding remainder.
            var w = c == cols - 1 ? bounds.Right - bounds.X - c * cw : cw;
            var h = r == rows - 1 ? bounds.Bottom - bounds.Y - r * ch : ch;
            panes[i].Show(new Rectangle(bounds.X + c * cw, bounds.Y + r * ch, w, h));
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
            var open = _slots.Count(s => s.Pane is not null);
            var fg = ForegroundCandidates().Count;
            var enabled = EnabledStreamSlots().Count();
            _status.Text = $"Game procs: {infos.Count} · Stream procs: {streamInfos?.Count ?? 0}" +
                           $" · Streams: {fg} fg / {Math.Max(0, open - fg)} bg" +
                           $" · Accounts: {enabled}/{_slots.Count} routing" +
                           $" · Visible: {_accounts.VisibleStreamCount}" +
                           $"({_userscripts?.Scripts.Count ?? 0} scripts)";
        }
        catch
        {
            // Keep the last known status; show the addon state if the base
            // text was never set (startup failure path).
            if (_status.Text.Length == 0)
        }
    }

    private void SaveSession()
    {
        var specs = new List<PaneSpec>();
        specs.AddRange(_games.Select(p => p.Snapshot()));
        foreach (var slot in _slots)
            if (slot.Pane is { } p) specs.Add(p.Snapshot());
        specs.AddRange(_extraPanes.Values.Select(p => p.Snapshot()));
        // Remember which tab was foregrounded (-1 = all background).
        specs.Add(new PaneSpec("active", "", _activeTabIndex >= 0 && _activeTabIndex < _slots.Count
            ? _slots[_activeTabIndex].Account.Id : "-", PaneKind.ActiveStreamMarker));
        SessionStore.Save(specs);
    }

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
        var pane = await Pane.CreateAsync(env, Handle, spec, _userscripts);
