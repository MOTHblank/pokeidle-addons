using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

internal sealed class MainForm : Form
{
    private readonly Panel _toolbar;
    private readonly ComboBox _streamPicker = new() { DropDownStyle = ComboBoxStyle.DropDownList, Width = 220 };
    private readonly Label _status = new() { AutoSize = true, Padding = new Padding(3, 4, 3, 0) };

    private CoreWebView2Environment? _gameEnv;
    private CoreWebView2Environment? _streamEnv;
    private readonly List<Pane> _games = [];
    private readonly List<Pane> _streams = [];
    private Pane? _activeStream;
    private readonly System.Windows.Forms.Timer _statsTimer = new() { Interval = 5000 };

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
        FormClosed += (_, _) => { _statsTimer.Stop(); foreach (var p in AllPanes()) p.Close(); };
        Shown += async (_, _) => await InitializeAsync();

        _statsTimer.Tick += (_, _) => UpdateStatus();
    }

    private IEnumerable<Pane> AllPanes() => _games.Concat(_streams);

    private void BuildToolbar()
    {
        var items = new Control[]
        {
            Button("Reload A", (_, _) => _games.ElementAtOrDefault(0)?.View.Reload()),
            Button("Reload B", (_, _) => _games.ElementAtOrDefault(1)?.View.Reload()),
            Button("DevTools A", (_, _) => _games.ElementAtOrDefault(0)?.View.OpenDevToolsWindow()),
            Button("DevTools B", (_, _) => _games.ElementAtOrDefault(1)?.View.OpenDevToolsWindow()),
            _streamPicker,
            Button("+ Stream", async (_, _) => await AddStreamAsync()),
            Button("Close stream", (_, _) => CloseActiveStream()),
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
    }

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

            _gameEnv = await CoreWebView2Environment.CreateAsync(
                null, AppConfig.GameUserDataFolder,
                new CoreWebView2EnvironmentOptions
                {
                    AdditionalBrowserArguments = AppConfig.GameBrowserArguments,
                    AreBrowserExtensionsEnabled = true
                });

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
            UpdateStatus();
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
        var pane = await Pane.CreateAsync(env, Handle, spec);

        if (spec.Kind == PaneKind.Game)
        {
            _games.Add(pane);
        }
        else
        {
            _streams.Add(pane);
            _streamPicker.Items.Add(spec.Title);
            _streamPicker.SelectedIndex = _streams.Count - 1; // triggers layout
        }
    }

    private async Task AddStreamAsync()
    {
        var url = Prompt("Stream URL", "https://www.twitch.tv/");
        if (string.IsNullOrWhiteSpace(url)) return;
        if (!url.Contains("://")) url = "https://" + url;

        var title = Uri.TryCreate(url, UriKind.Absolute, out var u)
            ? u.Host.Replace("www.", "") + u.AbsolutePath
            : url;

        await AddPaneAsync(new PaneSpec(title, url, "Streams", PaneKind.Stream));
    }

    private void CloseActiveStream()
    {
        if (_activeStream is null) return;
        var index = _streams.IndexOf(_activeStream);
        _activeStream.Close();
        _streams.RemoveAt(index);
        _streamPicker.Items.RemoveAt(index);
        _activeStream = null;
        if (_streams.Count > 0) _streamPicker.SelectedIndex = Math.Min(index, _streams.Count - 1);
        else LayoutPanes();
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
            if (s == _activeStream) s.Show(new Rectangle(gamesWidth, top, streamWidth, height));
            else s.Park();
        }
    }

    private void UpdateStatus()
    {
        try
        {
            if (_gameEnv is null) return;
            var infos = _gameEnv.GetProcessInfos();
            var streamInfos = _streamEnv?.GetProcessInfos();
            _status.Text = $"Game procs: {infos.Count} · Stream procs: {streamInfos?.Count ?? 0}";
        }
        catch { }
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
