using System.Text;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

internal sealed class MainForm : Form
{
    private readonly Panel _toolbar;
    private readonly Label _status = new()
        { AutoSize = true, Padding = new Padding(3, 4, 3, 0), Anchor = AnchorStyles.Top | AnchorStyles.Right };
    private readonly ComboBox _addonsPicker = new()
        { DropDownStyle = ComboBoxStyle.DropDownList, Width = 190 };

    private CoreWebView2Environment? _gameEnv;
    private CoreWebView2Environment? _streamEnv;
    private StreamPresenceManager? _streams;
    private readonly AccountManager _accounts = AccountManager.Load();
    private readonly List<Pane> _games = [];
    private readonly List<GameWorkspace> _workspaces = [new(0), new(1)];
    private ViolentmonkeyManager? _userscripts;
    private readonly System.Windows.Forms.Timer _statsTimer = new() { Interval = 5000 };
    private readonly System.Windows.Forms.Timer _probeTimer = new() { Interval = 30000 };
    private readonly CheckBox _probeToggle = new()
        { Text = "Probe", AutoSize = true, Checked = true, Padding = new Padding(3, 6, 3, 0) };
    private bool _probing;
    private bool _suppressTabEvent;
    private bool _suppressAddonPickerEvent;
    private readonly SemaphoreSlim _streamRouteGate = new(1, 1);
    private Button _game1Button = null!;
    private Button _game2Button = null!;
    private Button _foregroundBothButton = null!;
    private int _activeWorkspaceIndex;
    // Each game owns a completely separate stream area. A login profile can be
    // used by both games because each workspace owns its own WebView2 controller.
    private sealed class GameWorkspace(int index)
    {
        public int Index { get; } = index;
        public string GameProfile { get; set; } = "";
        public Pane? GamePane;
        public bool GameForeground { get; set; } = true;
        public DateTime? LastProbeAt { get; set; }
        public bool? LastProbeHealthy { get; set; }
        public string LastProbeSummary { get; set; } = "Checking…";
        public int ActiveTabIndex { get; set; } = -1;
        public bool StreamsExpanded { get; set; }
        public AccountService ActiveStreamService { get; set; } = AccountService.Twitch;

        public Label Header { get; } = new()
        {
            AutoSize = false,
            TextAlign = ContentAlignment.MiddleLeft,
            Font = new Font(SystemFonts.MessageBoxFont ?? Control.DefaultFont, FontStyle.Bold),
            Padding = new Padding(8, 0, 8, 0)
        };

        public Label Health { get; } = new()
        {
            AutoSize = false,
            TextAlign = ContentAlignment.MiddleCenter,
            Padding = new Padding(4, 0, 4, 0),
            Text = "Checking…"
        };

        public Button GameToggle { get; } = new()
        {
            AutoSize = false,
            Text = "Background",
            Height = 26
        };

        public Button StreamHeader { get; } = new()
        {
            AutoSize = false,
            TextAlign = ContentAlignment.MiddleLeft,
            FlatStyle = FlatStyle.System,
            Padding = new Padding(8, 0, 8, 0),
            Height = 30
        };

        public Button StreamOpen { get; } = new()
        {
            Text = "+ Open stream",
            AutoSize = false,
            Height = 30
        };

        public ComboBox StreamServicePicker { get; } = new()
        {
            DropDownStyle = ComboBoxStyle.DropDownList,
            Width = 82,
            Height = 30
        };

        public TabControl StreamTabs { get; } = new()
        {
            Appearance = TabAppearance.FlatButtons,
            ItemSize = new Size(66, 26),
            SizeMode = TabSizeMode.Fixed,
            Multiline = false,
            Height = 30,
            HotTrack = true
        };

        public List<StreamSlot> Slots { get; } = [];
    }

    private sealed class StreamSlot(Account account, int slotNumber)
    {
        public Account Account { get; set; } = account;
        public int SlotNumber { get; } = slotNumber;
        public string? Url;
        public StreamConnectionState State { get; set; }
        public TabPage Tab { get; } = new();
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
        BuildWorkspaceChrome();
        Controls.Add(_toolbar);

        Resize += (_, _) => LayoutPanes();
        FormClosing += (_, _) => SaveSession();
        FormClosed += (_, _) =>
        {
            _statsTimer.Stop();
            _probeTimer.Stop();
            foreach (var p in _games) p.Close();
            _ = _streams?.DisposeAsync();
        };
        Shown += async (_, _) => await InitializeAsync();

        _statsTimer.Tick += (_, _) => UpdateStatus();
        _probeTimer.Tick += async (_, _) => await ProbeTickAsync();
    }

    private IEnumerable<StreamSlot> AllStreamSlots() =>
        _workspaces.SelectMany(w => w.Slots);

    private IEnumerable<Pane> AllPanes() => _games;

    private void BuildWorkspaceChrome()
    {
        foreach (var workspace in _workspaces)
        {
            workspace.Header.Text = $"GAME {workspace.Index + 1}  ·  starting…";
            workspace.StreamHeader.Text = $"Chat · Game {workspace.Index + 1}";

            // WebView2 controllers are child HWNDs of the form. A full-size
            // workspace panel can otherwise cover the WebView surface.
            Controls.Add(workspace.Header);
            Controls.Add(workspace.Health);
            Controls.Add(workspace.GameToggle);
            workspace.StreamServicePicker.Items.AddRange(["Twitch", "Kick"]);
            workspace.StreamServicePicker.SelectedIndex = 0;

            Controls.Add(workspace.StreamHeader);
            Controls.Add(workspace.StreamServicePicker);
            Controls.Add(workspace.StreamOpen);
            Controls.Add(workspace.StreamTabs);

            workspace.GameToggle.Click += (_, _) =>
            {
                SetActiveWorkspace(workspace.Index);
                ToggleGameForeground(workspace);
            };
            workspace.Header.MouseDown += (_, _) => SetActiveWorkspace(workspace.Index);
            workspace.Health.MouseDown += (_, _) => SetActiveWorkspace(workspace.Index);
            workspace.StreamHeader.Click += (_, _) =>
            {
                SetActiveWorkspace(workspace.Index);
                workspace.StreamsExpanded = !workspace.StreamsExpanded;
                if (workspace.StreamsExpanded && workspace.ActiveTabIndex < 0)
                    SelectFirstOpenSlotForService(workspace);
                LayoutPanes();
            };
            workspace.StreamOpen.Click += async (_, _) =>
            {
                SetActiveWorkspace(workspace.Index);
                workspace.StreamsExpanded = true;
                LayoutPanes();
                await AddStreamManualAsync(workspace.Index);
            };
            workspace.StreamServicePicker.SelectedIndexChanged += (_, _) =>
            {
                if (_suppressTabEvent) return;
                workspace.ActiveStreamService = workspace.StreamServicePicker.SelectedIndex == 1
                    ? AccountService.Kick
                    : AccountService.Twitch;
                SelectFirstOpenSlotForService(workspace);
                RefreshStreamTabs(workspace);
                LayoutPanes();
            };
            workspace.StreamTabs.Enter += (_, _) => SetActiveWorkspace(workspace.Index);
            workspace.StreamTabs.SelectedIndexChanged += (_, _) =>
            {
                if (_suppressTabEvent) return;
                SetActiveWorkspace(workspace.Index);
                if (workspace.StreamTabs.SelectedIndex >= 0 &&
                    workspace.StreamTabs.SelectedIndex < workspace.StreamTabs.TabPages.Count &&
                    workspace.StreamTabs.TabPages[workspace.StreamTabs.SelectedIndex].Tag is StreamSlot slot)
                {
                    workspace.ActiveTabIndex = workspace.Slots.IndexOf(slot);
                }
                LayoutPanes();
                SaveSession();
            };
            workspace.StreamTabs.MouseUp += (_, e) =>
            {
                if (e.Button == MouseButtons.Right)
                    ShowStreamContextMenu(workspace, e.Location);
            };

            workspace.StreamsExpanded = false;
        }
    }

    private void SetActiveWorkspace(int index)
    {
        _activeWorkspaceIndex = Math.Clamp(index, 0, _workspaces.Count - 1);
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

    private void BuildToolbar()
    {
        var items = new Control[]
        {
            Button("Reload games", (_, _) => ReloadGames()),
            Button("DevTools A", (_, _) => _games.ElementAtOrDefault(0)?.View.OpenDevToolsWindow()),
            Button("DevTools B", (_, _) => _games.ElementAtOrDefault(1)?.View.OpenDevToolsWindow()),
            _game1Button = Button("Game 1: foreground", (_, _) => ToggleGameForeground(_workspaces[0])),
            _game2Button = Button("Game 2: foreground", (_, _) => ToggleGameForeground(_workspaces[1])),
            _foregroundBothButton = Button("Games: foreground both", (_, _) => SetGamesForeground(true)),
            Button("Accounts…", (_, _) => OpenAccountsDialog()),
            Button("Chat…", (_, _) => OpenStreamsForActiveWorkspace()),
            Button("Collapse streams", (_, _) => SetAllStreamsBackground()),
            _addonsPicker,
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
            _userscripts = new ViolentmonkeyManager(addonsFolder);
            Log($"real Violentmonkey {ViolentmonkeyManager.Version}: {_userscripts.ScriptNames.Count} repository script(s) found in {addonsFolder}");

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

            // The stream environment is not a video environment anymore. It is
            // used only for Kick's single chat host per login and temporary
            // interactive login windows. Twitch chat is native WebSocket/IRC.
            _streamEnv = await CoreWebView2Environment.CreateAsync(
                null, AppConfig.StreamUserDataFolder,
                new CoreWebView2EnvironmentOptions
                {
                    AdditionalBrowserArguments = AppConfig.StreamBrowserArguments,
                    AreBrowserExtensionsEnabled = false
                });

            var session = SessionStore.Load();
            foreach (var spec in session.Where(s => s.Kind == PaneKind.Game))
                await AddGamePaneAsync(spec);

            // Each game column gets up to 10 Twitch + 10 Kick stream slots.
            // Each workspace exposes 10 Twitch + 10 Kick slots. Slot numbers are
            // per service and are UI identities; login profiles are implementation
            // details shown in the tab context menu instead of being tab names.
            _accounts.EnsureStreamAccount(AccountService.Twitch);
            _accounts.EnsureStreamAccount(AccountService.Kick);
            _suppressTabEvent = true;
            try
            {
                foreach (var workspace in _workspaces)
                    BuildStreamSlots(workspace);
            }
            finally
            {
                _suppressTabEvent = false;
            }
            _streams = new StreamPresenceManager(_streamEnv, Handle, Log);
            UpdateWorkspaceHeaders();

            // Stream slots are intentionally fresh each shell session and are not
            // restored; game profiles remain persistent.
            _accounts.Changed += OnAccountsChanged;

            _activeWorkspaceIndex = 0;
            LayoutPanes();
            _statsTimer.Start();
            if (_probeToggle.Checked) _probeTimer.Start();
            UpdateStatus();
            Log($"startup: {_games.Count} game workspace(s), " +
                $"{_accounts.StreamAccounts.Count()} stream login(s) × 2 workspaces " +
                $"({_accounts.EnabledStreamAccounts.Count()} enabled; up to " +
                $"{AccountManager.MaxStreamsPerService} Twitch + {AccountManager.MaxStreamsPerService} Kick streams per game; " +
                $"{AccountManager.MaxStreamSlotsPerAccount} streams per login); " +
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
            IsAccountActive,
            acc => _ = ShowAccountForLoginAsync(acc));
        dlg.ShowDialog(this);
        LayoutPanes();
    }

    private bool IsAccountActive(Account acc)
    {
        if (acc.IsStream)
            return _streams?.IsAccountConnected(acc.Id) == true;

        return _games.Any(g =>
            string.Equals(g.Spec.Profile, acc.Id, StringComparison.OrdinalIgnoreCase));
    }

    private async void OnAccountsChanged()
    {
        try
        {
            if (_streams is not null)
                await _streams.ResetAsync();

            foreach (var workspace in _workspaces)
                BuildStreamSlots(workspace);

            RebuildTabTitles();
            UpdateWorkspaceHeaders();
            LayoutPanes();
            SaveSession();
        }
        catch (Exception ex)
        {
            Log($"stream account refresh failed: {ex}");
        }
    }

    private void BuildStreamSlots(GameWorkspace workspace)
    {
        workspace.Slots.Clear();
        workspace.StreamTabs.TabPages.Clear();
        workspace.ActiveTabIndex = -1;

        var enabledByService = _accounts.EnabledStreamAccounts
            .GroupBy(a => a.Service)
            .ToDictionary(g => g.Key, g => g.ToList());

        foreach (var service in new[] { AccountService.Twitch, AccountService.Kick })
        {
            if (!enabledByService.TryGetValue(service, out var accounts))
                continue;

            // Slot identity is independent from the login profile. With two
            // Twitch logins this produces T1..T10, with logins assigned alternately.
            for (var slotNumber = 1; slotNumber <= AccountManager.MaxStreamsPerService; slotNumber++)
            {
                var account = accounts[(slotNumber - 1) % accounts.Count];
                AddStreamSlot(workspace, account, slotNumber);
            }
        }

        if (workspace.Slots.All(s => s.Account.Service != workspace.ActiveStreamService) &&
            workspace.Slots.Count > 0)
        {
            workspace.ActiveStreamService = workspace.Slots[0].Account.Service;
            RefreshStreamTabs(workspace);
        }
    }

    private void AddStreamSlot(GameWorkspace workspace, Account account, int slotNumber)
    {
        if (workspace.Slots.Count(s =>
                s.Account.Service == account.Service) >= AccountManager.MaxStreamsPerService)
            return;

        var existing = workspace.Slots.FirstOrDefault(s =>
            string.Equals(s.Account.Id, account.Id, StringComparison.OrdinalIgnoreCase) &&
            s.SlotNumber == slotNumber);
        if (existing is not null)
        {
            existing.Account = account;
            existing.Tab.Text = StreamTabText(existing);
            existing.Tab.AccessibleName = $"{account.Service} slot {slotNumber} · {account.DisplayLabel}";
            existing.Tab.Tag = existing;
            RefreshStreamTabs(workspace);
            return;
        }

        var slot = new StreamSlot(account, slotNumber);
        workspace.Slots.Add(slot);
        slot.Tab.Tag = slot;
        slot.Tab.Text = StreamTabText(slot);
        slot.Tab.AccessibleName = $"{account.Service} slot {slotNumber} · {account.DisplayLabel}";
        RefreshStreamTabs(workspace);
    }

    private static string StreamTabText(StreamSlot slot)
    {
        var service = slot.Account.Service == AccountService.Twitch ? "T" : "K";
        var state = slot.Url is null
            ? "○"
            : slot.State switch
            {
                StreamConnectionState.Connected => "●",
                StreamConnectionState.Connecting => "…",
                StreamConnectionState.AuthenticationRequired => "!",
                StreamConnectionState.Error => "×",
                _ => "○"
            };
        return $"{state} {service}{slot.SlotNumber}";
    }

    private void RefreshStreamTabs(GameWorkspace workspace)
    {
        _suppressTabEvent = true;
        try
        {
            var activeSlot = workspace.ActiveTabIndex >= 0 && workspace.ActiveTabIndex < workspace.Slots.Count
                ? workspace.Slots[workspace.ActiveTabIndex]
                : null;
            workspace.StreamTabs.TabPages.Clear();
            foreach (var slot in workspace.Slots.Where(s => s.Account.Service == workspace.ActiveStreamService))
                workspace.StreamTabs.TabPages.Add(slot.Tab);

            var preferred = activeSlot is not null && activeSlot.Account.Service == workspace.ActiveStreamService
                ? workspace.StreamTabs.TabPages.IndexOf(activeSlot.Tab)
                : -1;
            workspace.StreamTabs.SelectedIndex = preferred;
        }
        finally
        {
            _suppressTabEvent = false;
        }
    }

    private void SelectFirstOpenSlotForService(GameWorkspace workspace)
    {
        var index = workspace.Slots.FindIndex(s =>
            s.Account.Service == workspace.ActiveStreamService && s.Url is not null);
        workspace.ActiveTabIndex = index;
    }

    private void RebuildTabTitles()
    {
        _suppressTabEvent = true;
        try
        {
            foreach (var workspace in _workspaces)
                foreach (var slot in workspace.Slots)
                {
                    slot.Tab.Text = StreamTabText(slot);
                    slot.Tab.AccessibleName =
                        $"{slot.Account.Service} slot {slot.SlotNumber} · " +
                        $"{slot.Account.DisplayLabel} · " +
                        $"{slot.Url ?? "not joined"}";
                }
        }
        finally { _suppressTabEvent = false; }
    }

    private void UpdateWorkspaceHeader(GameWorkspace workspace)
    {
        var open = workspace.Slots.Count(s => s.Url is not null);
        var openTwitch = workspace.Slots.Count(s =>
            s.Url is not null && s.Account.Service == AccountService.Twitch);
        var openKick = workspace.Slots.Count(s =>
            s.Url is not null && s.Account.Service == AccountService.Kick);
        var connected = workspace.Slots.Count(s =>
            s.Url is not null && s.State == StreamConnectionState.Connected);
        var state = workspace.GameForeground ? "FOREGROUND" : "BACKGROUND";
        workspace.Header.Text =
            $"GAME {workspace.Index + 1}  ·  {workspace.GamePane?.Spec.Title ?? workspace.GameProfile}";
        workspace.Health.Text = workspace.LastProbeSummary;
        workspace.Health.ForeColor = workspace.LastProbeHealthy switch
        {
            true => System.Drawing.Color.DarkGreen,
            false => System.Drawing.Color.Firebrick,
            _ => SystemColors.ControlText
        };
        workspace.GameToggle.Text = workspace.GameForeground ? "Background" : "Foreground";
        workspace.StreamHeader.Text =
            $"{(workspace.StreamsExpanded ? "▾" : "▸")} Chats · {connected} connected · T {openTwitch}/{AccountManager.MaxStreamsPerService} · K {openKick}/{AccountManager.MaxStreamsPerService}";
        workspace.StreamOpen.Text = "+ Join chat";
        var serviceIndex = workspace.ActiveStreamService == AccountService.Kick ? 1 : 0;
        if (workspace.StreamServicePicker.SelectedIndex != serviceIndex)
        {
            _suppressTabEvent = true;
            workspace.StreamServicePicker.SelectedIndex = serviceIndex;
            _suppressTabEvent = false;
        }

        if (workspace.Index == 0 && _game1Button is not null)
            _game1Button.Text = $"Game 1: {state.ToLowerInvariant()}";
        if (workspace.Index == 1 && _game2Button is not null)
            _game2Button.Text = $"Game 2: {state.ToLowerInvariant()}";
        if (_foregroundBothButton is not null)
            _foregroundBothButton.Text = _workspaces.All(w => w.GameForeground)
                ? "Games: both foreground"
                : "Games: foreground both";
    }

    private void CloseActiveStream()
    {
        var workspace = _workspaces[Math.Clamp(_activeWorkspaceIndex, 0, _workspaces.Count - 1)];
        if (workspace.ActiveTabIndex < 0 || workspace.ActiveTabIndex >= workspace.Slots.Count)
            return;
        _ = CloseStreamAsync(workspace, workspace.Slots[workspace.ActiveTabIndex]);
    }

    private void ShowStreamContextMenu(GameWorkspace workspace, Point location)
    {
        var hit = -1;
        for (var i = 0; i < workspace.StreamTabs.TabPages.Count; i++)
        {
            if (workspace.StreamTabs.GetTabRect(i).Contains(location))
            {
                hit = i;
                break;
            }
        }

        if (hit < 0 || hit >= workspace.StreamTabs.TabPages.Count)
            return;

        if (workspace.StreamTabs.TabPages[hit].Tag is not StreamSlot slot)
            return;

        SetActiveWorkspace(workspace.Index);
        _suppressTabEvent = true;
        workspace.StreamTabs.SelectedIndex = hit;
        _suppressTabEvent = false;
        workspace.ActiveTabIndex = workspace.Slots.IndexOf(slot);

        var menu = new ContextMenuStrip();
        menu.Items.Add(new ToolStripMenuItem
        {
            Text = $"{slot.Account.Service} {slot.SlotNumber} · {slot.Account.DisplayLabel}",
            Enabled = false
        });

        var joinText = slot.Url is null ? "Join chat…" : "Rejoin chat…";
        menu.Items.Add(
            joinText,
            null,
            async (_, _) =>
            {
                var initial = slot.Url ?? AccountManager.LoginUrl(slot.Account.Service);
                await JoinStreamSlotAsync(workspace, slot, initial);
            });

        menu.Items.Add(
            "Log in…",
            null,
            async (_, _) =>
            {
                await ShowAccountForLoginAsync(slot.Account);
            });

        if (slot.Url is not null)
        {
            menu.Items.Add(
                "Open channel in browser",
                null,
                async (_, _) => await _streams!.OpenExternallyAsync(slot.Url));

            menu.Items.Add(
                "Leave chat",
                null,
                async (_, _) => await CloseStreamAsync(workspace, slot));
        }

        menu.Items.Add(new ToolStripSeparator());
        menu.Items.Add(
            $"Close {slot.Account.Service} {slot.SlotNumber}",
            null,
            async (_, _) => await CloseStreamAsync(workspace, slot));
        menu.Items.Add(
            $"Close all joined chats for Game {workspace.Index + 1}",
            null,
            async (_, _) => await CloseAllStreamsAsync(workspace));

        menu.Show(workspace.StreamTabs, location);
    }

    private async Task CloseStreamAsync(GameWorkspace workspace, StreamSlot slot)
    {
        var wasActive = workspace.ActiveTabIndex >= 0 &&
                        workspace.ActiveTabIndex < workspace.Slots.Count &&
                        ReferenceEquals(workspace.Slots[workspace.ActiveTabIndex], slot);

        if (_streams is not null && slot.Url is not null)
            await _streams.LeaveAsync(slot.Account, slot.Url);

        slot.Url = null;
        slot.State = StreamConnectionState.Disconnected;
        slot.Tab.Text = StreamTabText(slot);

        if (wasActive)
        {
            var next = workspace.Slots.FirstOrDefault(s =>
                s.Account.Service == workspace.ActiveStreamService &&
                s.Url is not null &&
                !ReferenceEquals(s, slot))
                ?? workspace.Slots.FirstOrDefault(s =>
                    s.Url is not null && !ReferenceEquals(s, slot));

            if (next is not null)
                SelectTab(workspace, workspace.Slots.IndexOf(next));
            else
            {
                workspace.ActiveTabIndex = -1;
                RefreshStreamTabs(workspace);
                _suppressTabEvent = true;
                workspace.StreamTabs.SelectedIndex = -1;
                _suppressTabEvent = false;
            }
        }

        RebuildTabTitles();
        LayoutPanes();
        SaveSession();
    }

    private async Task CloseAllStreamsAsync(GameWorkspace workspace)
    {
        foreach (var slot in workspace.Slots.ToArray())
        {
            if (_streams is not null && slot.Url is not null)
                await _streams.LeaveAsync(slot.Account, slot.Url);
            slot.Url = null;
            slot.State = StreamConnectionState.Disconnected;
        }

        workspace.ActiveTabIndex = -1;
        _suppressTabEvent = true;
        workspace.StreamTabs.SelectedIndex = -1;
        _suppressTabEvent = false;

        RebuildTabTitles();
        LayoutPanes();
        SaveSession();
    }

    private void ToggleGameForeground(GameWorkspace workspace)
    {
        workspace.GameForeground = !workspace.GameForeground;
        UpdateWorkspaceHeader(workspace);
        LayoutPanes();
        SaveSession();
        Log($"Game {workspace.Index + 1} switched to {(workspace.GameForeground ? "foreground" : "background")}");
    }

    private void SetGamesForeground(bool foreground)
    {
        foreach (var workspace in _workspaces)
            workspace.GameForeground = foreground;

        LayoutPanes();
        SaveSession();
        Log($"games switched to {(foreground ? "foreground" : "background")}");
    }

    private void UpdateWorkspaceHeaders()
    {
        foreach (var workspace in _workspaces)
            UpdateWorkspaceHeader(workspace);
    }

    private async Task ShowAccountForLoginAsync(Account acc)
    {
        if (acc.IsStream)
        {
            try
            {
                if (_streams is null)
                    throw new InvalidOperationException("Stream subsystem is not initialized.");

                await _streams.LoginAsync(acc);
                RebuildTabTitles();
                UpdateWorkspaceHeaders();
            }
            catch (Exception ex)
            {
                Log($"stream login failed for {acc.DisplayLabel}: {ex}");
                MessageBox.Show(
                    this,
                    ex.Message,
                    $"{acc.Service} login",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Error);
            }

            return;
        }

        var game = _games.FirstOrDefault(g =>
            string.Equals(g.Spec.Profile, acc.Id, StringComparison.OrdinalIgnoreCase));
        if (game is not null)
            game.View.Navigate(AccountManager.LoginUrl(acc.Service));
    }

    private IEnumerable<StreamSlot> SlotsForAccount(GameWorkspace workspace, string profile) =>
        workspace.Slots.Where(s =>
            string.Equals(s.Account.Id, profile, StringComparison.OrdinalIgnoreCase));

    private StreamSlot? FirstSlotForAccount(GameWorkspace workspace, string profile) =>
        SlotsForAccount(workspace, profile).OrderBy(s => s.SlotNumber).FirstOrDefault();

    private StreamSlot? FindOpenStreamSlot(GameWorkspace workspace, string url)
    {
        var canonical = CanonicalStreamUrl(url);
        return workspace.Slots.FirstOrDefault(s =>
            s.Url is not null &&
            string.Equals(CanonicalStreamUrl(s.Url), canonical, StringComparison.OrdinalIgnoreCase));
    }

    private StreamSlot? FindFreeStreamSlot(GameWorkspace workspace, AccountService service) =>
        workspace.Slots
            .Where(s => s.Account.Enabled && s.Account.Service == service && s.Url is null)
            .OrderBy(s => s.SlotNumber)
            .ThenBy(s => s.Account.Id, StringComparer.OrdinalIgnoreCase)
            .FirstOrDefault();

    private GameWorkspace? WorkspaceForGroup(string? group) =>
        string.IsNullOrWhiteSpace(group) ? null :
        _workspaces.FirstOrDefault(w =>
            string.Equals(w.GameProfile, group, StringComparison.OrdinalIgnoreCase));

    private GameWorkspace? WorkspaceForPane(Pane pane)
    {
        if (pane.Spec.Kind == PaneKind.Game)
            return WorkspaceForGroup(pane.Spec.Profile);

        return _workspaces.FirstOrDefault(w =>
            w.Slots.Any(s => ReferenceEquals(s.Pane, pane)));
    }

    // --- Panes -----------------------------------------------------------------

    private async Task AddGamePaneAsync(PaneSpec spec)
    {
        var pane = await Pane.CreateAsync(_gameEnv!, Handle, spec, _userscripts);

        pane.MessageReceived += OnPaneMessage;
        pane.PopupRequested += OnPopupRequested;
        pane.View.NavigationStarting += (_, e) => GamePaneNavigating(pane, _, e);
        pane.View.NavigationCompleted += (_, e) =>
            Log($"Game {spec.Profile} navigation {(e.IsSuccess ? "completed" : "FAILED")} " +
                $"({e.HttpStatusCode}): {pane.View.Source}");
        pane.View.SourceChanged += (_, _) =>
            Log($"Game {spec.Profile} source: {pane.View.Source}");
        pane.View.ProcessFailed += (_, e) =>
            Log($"Game {spec.Profile} WebView process FAILED: {e.ProcessFailedKind}");

        _games.Add(pane);

        if (_games.Count <= _workspaces.Count)
        {
            var workspace = _workspaces[_games.Count - 1];
            workspace.GamePane = pane;
            workspace.GameProfile = spec.Profile;
            workspace.GameForeground = !spec.Background;
            UpdateWorkspaceHeader(workspace);
        }
    }

    private async Task JoinStreamSlotAsync(
        GameWorkspace workspace,
        StreamSlot slot,
        string url)
    {
        var canonical = CanonicalStreamUrl(url);
        if (!IsStreamUrl(canonical))
            throw new ArgumentException(
                "Only Twitch or Kick channel URLs can be joined.",
                nameof(url));

        workspace.StreamsExpanded = true;
        workspace.ActiveStreamService = slot.Account.Service;
        slot.State = StreamConnectionState.Connecting;
        slot.Url = canonical;
        slot.Tab.Text = StreamTabText(slot);
        RebuildTabTitles();
        LayoutPanes();

        try
        {
            if (_streams is null)
                throw new InvalidOperationException("Stream subsystem is not initialized.");

            await _streams.JoinAsync(slot.Account, canonical);
            slot.State = StreamConnectionState.Connected;
        }
        catch (AuthenticationRequiredException ex)
        {
            slot.State = StreamConnectionState.AuthenticationRequired;
            slot.Url = null;
            throw new InvalidOperationException(
                ex.Message + " Use Accounts → Log in… and then join the channel again.");
        }
        catch
        {
            slot.State = StreamConnectionState.Error;
            slot.Url = null;
            throw;
        }
        finally
        {
            slot.Tab.Text = StreamTabText(slot);
            RebuildTabTitles();
            UpdateWorkspaceHeaders();
            LayoutPanes();
        }
    }

    private void RefreshAddonsPicker()
    {
        _suppressAddonPickerEvent = true;
        try
        {
            _addonsPicker.Items.Clear();
            _addonsPicker.Items.Add("Reload userscripts (all panes)");
            foreach (var scriptName in _userscripts?.ScriptNames ?? [])
                _addonsPicker.Items.Add(scriptName);
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
            _userscripts = new ViolentmonkeyManager(folder);
            RefreshAddonsPicker();

            foreach (var pane in _games.ToList())
            {
                await pane.AttachUserscriptAsync(_userscripts);
                pane.View.Reload();
            }

            Log($"userscript extension reloaded: {_userscripts.ScriptNames.Count} script(s) across {_games.Count} game pane(s); stream chat presence does not use the extension");
        }
        catch (Exception ex)
        {
            Log($"addon reload FAILED: {ex}");
            MessageBox.Show(this, ex.ToString(), "Addon reload failure",
                MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
    }

    // --- Stream link routing -------------------------------------------------
    // Stream links stay inside the game workspace that generated them.
    private void OnPaneMessage(Pane pane, HostMessage msg)
    {
        if (msg.Type != "link" || !IsStreamUrl(msg.Url)) return;

        var workspace = WorkspaceForPane(pane);
        if (workspace is not null)
            _ = RouteStreamLinkAsync(
                msg.Url, $"userscript ({msg.Source})");
    }

    private void OnPopupRequested(Pane pane, string url)
    {
        if (IsStreamUrl(url))
        {
            var workspace = WorkspaceForPane(pane);
            if (workspace is not null)
                _ = RouteStreamLinkAsync(url, "popup backstop");
            return;
        }

        Log($"ignored non-stream popup from {pane.Spec.Kind}: {url}");
    }

    private void GamePaneNavigating(
        Pane pane, object? sender, CoreWebView2NavigationStartingEventArgs e)
    {
        if (!IsStreamUrl(e.Uri)) return;
        e.Cancel = true;
        _ = RouteStreamLinkAsync(e.Uri, "navigation backstop");
    }

    public static bool IsStreamUrl(string? url) =>
        AccountManager.ServiceForUrl(url) is AccountService.Twitch or AccountService.Kick;

    private async Task RouteStreamLinkAsync(string url, string via)
    {
        await _streamRouteGate.WaitAsync();
        try
        {
            var service = AccountManager.ServiceForUrl(url);
            if (service is null)
            {
                Log($"unsupported stream URL {url} (via {via})");
                return;
            }

            var canonical = CanonicalStreamUrl(url);
            var routed = 0;
            var duplicate = 0;
            var full = 0;

            // One streamer click is shared between Game 1 and Game 2, but inside
            // each game it consumes exactly ONE free slot. This prevents the
            // previous bug where both slots associated with a login received the
            // same channel.
            foreach (var workspace in _workspaces)
            {
                var existing = FindOpenStreamSlot(workspace, canonical);
                if (existing is not null)
                {
                    duplicate++;
                    SelectTab(workspace, workspace.Slots.IndexOf(existing));
                    continue;
                }

                var slot = FindFreeStreamSlot(workspace, service.Value);
                if (slot is null)
                {
                    full++;
                    continue;
                }

                await JoinStreamSlotAsync(workspace, slot, canonical);
                SelectTab(workspace, workspace.Slots.IndexOf(slot));
                routed++;
            }

            Log($"Routed {canonical} as {service} via {via}: " +
                $"{routed} new pane(s), {duplicate} already open, {full} workspace(s) full " +
                $"(capacity {AccountManager.MaxStreamsPerService} {service} streams/game; " +
                $"{AccountManager.MaxStreamSlotsPerAccount}/login)");
            SaveSession();
        }
        catch (Exception ex)
        {
            Log($"link routing failed: {ex}");
            _status.Text = $"Stream route failed: {ex.Message}";
        }
        finally
        {
            _streamRouteGate.Release();
        }
    }

    private static string CanonicalStreamUrl(string url)
    {
        try
        {
            var u = new Uri(url);
            var builder = new UriBuilder(u)
            {
                Host = u.Host.ToLowerInvariant(),
                Query = "",
                Fragment = ""
            };

            var path = u.AbsolutePath.TrimEnd('/');
            builder.Path = path.Length == 0 ? "/" : path;
            return builder.Uri.AbsoluteUri;
        }
        catch
        {
            return url.Trim();
        }
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

    private void SelectTab(GameWorkspace workspace, int index)
    {
        var target = index >= 0 && index < workspace.Slots.Count ? workspace.Slots[index] : null;
        if (target is null)
        {
            workspace.ActiveTabIndex = -1;
            return;
        }

        workspace.StreamsExpanded = true;
        workspace.ActiveStreamService = target.Account.Service;
        RefreshStreamTabs(workspace);
        var tabIndex = workspace.StreamTabs.TabPages.IndexOf(target.Tab);
        workspace.ActiveTabIndex = index;
        if (tabIndex < 0) return;

        SetActiveWorkspace(workspace.Index);
        _suppressTabEvent = true;
        workspace.StreamTabs.SelectedIndex = tabIndex;
        _suppressTabEvent = false;
        LayoutPanes();
    }

    private void OpenStreamsForActiveWorkspace()
    {
        var workspace = _workspaces[Math.Clamp(_activeWorkspaceIndex, 0, _workspaces.Count - 1)];
        workspace.StreamsExpanded = true;
        LayoutPanes();
        _ = AddStreamManualAsync(workspace.Index);
    }

    private void SetAllStreamsBackground()
    {
        foreach (var workspace in _workspaces)
        {
            // Collapse only the lightweight chat/presence dock. The underlying
            // chat sessions intentionally remain active so accounts stay present.
            workspace.StreamsExpanded = false;
            workspace.ActiveTabIndex = -1;
            _suppressTabEvent = true;
            workspace.StreamTabs.SelectedIndex = -1;
            _suppressTabEvent = false;
        }

        LayoutPanes();
        SaveSession();
    }

    private async Task AddStreamManualAsync(int workspaceIndex)
    {
        var workspace = _workspaces[Math.Clamp(workspaceIndex, 0, _workspaces.Count - 1)];
        SetActiveWorkspace(workspace.Index);
        workspace.StreamsExpanded = true;
        LayoutPanes();

        var initial = workspace.ActiveStreamService == AccountService.Kick
            ? "https://kick.com/"
            : "https://www.twitch.tv/";

        var url = Prompt(
            $"Join stream chat · Game {workspace.Index + 1}",
            initial);
        if (string.IsNullOrWhiteSpace(url)) return;

        if (!url.Contains("://", StringComparison.Ordinal))
            url = "https://" + url.Trim();

        if (!IsStreamUrl(url))
        {
            MessageBox.Show(
                this,
                "Only Twitch or Kick channel URLs are supported.",
                "Join stream chat",
                MessageBoxButtons.OK,
                MessageBoxIcon.Information);
            return;
        }

        var service = AccountManager.ServiceForUrl(url);
        if (service is null) return;

        var existing = FindOpenStreamSlot(workspace, url);
        if (existing is not null)
        {
            SelectTab(workspace, workspace.Slots.IndexOf(existing));
            return;
        }

        var slot = FindFreeStreamSlot(workspace, service.Value);
        if (slot is null)
        {
            MessageBox.Show(
                this,
                $"No free {service} slot. This workspace supports {AccountManager.MaxStreamsPerService} {service} chat channels.",
                "Join stream chat",
                MessageBoxButtons.OK,
                MessageBoxIcon.Information);
            return;
        }

        try
        {
            await JoinStreamSlotAsync(workspace, slot, url);
            SelectTab(workspace, workspace.Slots.IndexOf(slot));
            SaveSession();
        }
        catch (Exception ex)
        {
            Log($"manual stream presence join failed: {ex}");
            MessageBox.Show(
                this,
                $"The chat presence could not be established.\n\n{ex.Message}\n\nSee the IdleShell log for details.",
                "Join stream chat",
                MessageBoxButtons.OK,
              // --- Layout --------------------------------------------------------------------
    // Streams are no longer browser panes. The dock is only a compact control
    // strip, so game WebViews retain almost the entire workspace.
    private void LayoutPanes()
    {
        if (_gameEnv is null) return;

        const int pagePadding = 6;
        const int columnGap = 6;
        const int headerHeight = 34;
        const int streamBarHeight = 30;
        const int tabHeight = 30;
        const int topGap = 6;
        const int streamOpenWidth = 92;

        var contentTop = AppConfig.ToolbarHeight + topGap;
        var contentHeight = Math.Max(0, ClientSize.Height - contentTop - pagePadding);
        var contentWidth = Math.Max(0, ClientSize.Width - pagePadding * 2);
        var columnWidth = Math.Max(0, (contentWidth - columnGap) / 2);

        for (var i = 0; i < _workspaces.Count; i++)
        {
            var workspace = _workspaces[i];
            var x = pagePadding + i * (columnWidth + columnGap);
            var frame = new Rectangle(x, contentTop, columnWidth, contentHeight);
            var innerWidth = Math.Max(0, frame.Width - 2);
            var innerHeight = Math.Max(0, frame.Height - 2);

            workspace.Header.Bounds =
                new Rectangle(frame.X + 1, frame.Y, innerWidth, headerHeight);
            workspace.Header.BackColor = SystemColors.ActiveCaption;
            workspace.Header.ForeColor = SystemColors.ActiveCaptionText;

            var gameTop = headerHeight + 6;
            var dockHeight = workspace.StreamsExpanded
                ? streamBarHeight + tabHeight + 2
                : streamBarHeight;
            var streamTop = Math.Max(gameTop + 1, innerHeight - dockHeight);

            workspace.StreamHeader.Bounds =
                new Rectangle(
                    frame.X + 1,
                    frame.Y + streamTop,
                    Math.Max(1, innerWidth - streamOpenWidth - 86),
                    streamBarHeight);
            workspace.StreamHeader.BackColor = SystemColors.ControlLight;

            workspace.StreamServicePicker.Bounds =
                new Rectangle(
                    frame.Right - streamOpenWidth - 1 - 82 - 4,
                    frame.Y + streamTop,
                    82,
                    streamBarHeight);
            workspace.StreamServicePicker.Visible = workspace.StreamsExpanded;

            workspace.StreamOpen.Bounds =
                new Rectangle(
                    frame.Right - streamOpenWidth - 1,
                    frame.Y + streamTop,
                    streamOpenWidth,
                    streamBarHeight);

            workspace.StreamTabs.Bounds =
                new Rectangle(
                    frame.X + 1,
                    frame.Y + streamTop + streamBarHeight,
                    innerWidth,
                    tabHeight);
            workspace.StreamTabs.Visible = workspace.StreamsExpanded;

            var gameBounds = new Rectangle(
                frame.X + 3,
                frame.Y + 1 + gameTop,
                Math.Max(0, frame.Width - 6),
                Math.Max(0, streamTop - gameTop - 2));

            if (workspace.GamePane is { } gamePane)
            {
                if (workspace.GameForeground)
                    gamePane.Show(gameBounds);
                else
                    gamePane.Hide();
            }
        }

        UpdateWorkspaceHeaders();
    }

    // Probe every pane. A hidden game is considered healthy when JavaScript
    // still responds and the 1-second timer remains near real time.
    private async Task ProbeTickAsync()
    {
        if (_probing) return;
        _probing = true;
        try
        {
            EnsureProbeHeader();
            foreach (var p in _games.ToArray())
            {
                var raw = await p.ProbeAsync();
                var fields = ParseProbeJson(raw ?? "");

                var workspace = WorkspaceForGroup(p.Spec.Profile);
                if (workspace is not null)
                {
                    workspace.LastProbeAt = DateTime.Now;
                    workspace.LastProbeHealthy =
                        ProbeHealthy(fields.vis, fields.hidden, fields.drift);

                    var label = workspace.GameForeground
                        ? "Healthy"
                        : "BG healthy";

                    workspace.LastProbeSummary =
                        workspace.LastProbeHealthy == true
                            ? $"● {label} · {fields.drift} ms"
                            : $"● Check · {fields.drift} ms";

                    UpdateWorkspaceHeader(workspace);
                }

                var state = workspace?.GameForeground == true
                    ? "Foreground"
                    : "Background";

                var line = string.Join(',',
                    DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss"),
                    Csv(p.Spec.Title),
                    Csv(p.Spec.Kind.ToString()),
                    Csv(state),
                    Csv(fields.vis),
                    Csv(fields.hidden),
                    Csv(fields.focus),
                    Csv(fields.drift));

                File.AppendAllText(
                    AppConfig.ProbeCsvFile,
                    line + Environment.NewLine);
            }
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"[IdleShell] probe: {ex.Message}");
        }
        finally
        {
            _probing = false;
        }
    }

    private static bool ProbeHealthy(string vis, string hidden, string drift)
    {
        if (vis is "error" or "?" || hidden is "error" or "?" ||
            drift is "error" or "?")
            return false;

        return double.TryParse(drift, out var ms) && Math.Abs(ms) < 1500;
    }

    private string GameHealthSummary() =>
        string.Join(" / ", _workspaces.Select(w =>
            $"G{w.Index + 1} {w.LastProbeSummary}"));

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
            var open = AllStreamSlots().Count(s => s.Url is not null);
            var joined = _streams?.JoinedChannelCount ?? 0;
            var twitchAccounts = _streams?.ConnectedTwitchAccounts ?? 0;
            var kickAccounts = _streams?.ConnectedKickAccounts ?? 0;
            var enabled = _accounts.EnabledStreamAccounts.Count();

            _status.Text =
                $"Games: {_games.Count}/2 · Game procs: {infos.Count}" +
                $" · Chats: {joined} joined / {open} slots" +
                $" · Sessions: T{twitchAccounts} K{kickAccounts}" +
                $" · Routing accounts: {enabled}/{_accounts.StreamAccounts.Count()}" +
                $" · Health: {GameHealthSummary()}" +
                $" · Addons: {_userscripts?.ScriptNames.Count ?? 0} userscripts";
        }
        catch (Exception ex)
        {
            if (_status.Text.Length == 0)
                _status.Text = "Status unavailable";

            Log($"status update failed: {ex.Message}");
        }
    }

    private void SaveSession()
    {
        var specs = new List<PaneSpec>();
        foreach (var workspace in _workspaces)
        {
            if (workspace.GamePane is { } gamePane)
            {
                specs.Add(gamePane.Snapshot() with
                {
                    Background = !workspace.GameForeground,
                    Group = workspace.GameProfile
                });
            }
        }

        // Chat presence sessions and active stream tabs are intentionally not
        // persisted; live detection rebuilds them from the current game state.
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
