using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

internal sealed class MainForm : Form
{
    private readonly Panel _toolbar;
    private readonly ExtensionManager _extensionManager;

    private Label _status = null!;
    private CoreWebView2Environment? _environment;
    private CoreWebView2Controller? _controllerA;
    private CoreWebView2Controller? _controllerB;
    private CoreWebView2? _webViewA;
    private CoreWebView2? _webViewB;

    public MainForm()
    {
        Text = "PokéIdle Idle Shell";
        StartPosition = FormStartPosition.CenterScreen;
        MinimumSize = new Size(960, 600);
        ClientSize = new Size(1440, 850);
        KeyPreview = true;

        _extensionManager =
            new ExtensionManager(
                AppConfig.TampermonkeyExtensionFolder);

        _toolbar = new Panel
        {
            Dock = DockStyle.Top,
            Height = AppConfig.ToolbarHeight,
            Padding = new Padding(8, 5, 8, 5)
        };

        AddToolbarControls();
        Controls.Add(_toolbar);

        Resize += (_, _) => LayoutWebViews();
        FormClosed += (_, _) => DisposeWebViews();
        Shown += async (_, _) => await InitializeAsync();
    }

    private void AddToolbarControls()
    {
        var accountA = CreateLabel("Account A");
        var accountB = CreateLabel("Account B");

        var reloadA = CreateButton(
            "Reload A",
            (_, _) => _webViewA?.Reload());

        var reloadB = CreateButton(
            "Reload B",
            (_, _) => _webViewB?.Reload());

        var devToolsA = CreateButton(
            "DevTools A",
            (_, _) => _webViewA?.OpenDevToolsWindow());

        var devToolsB = CreateButton(
            "DevTools B",
            (_, _) => _webViewB?.OpenDevToolsWindow());

        _status = CreateLabel("Starting…");
        _status.AutoSize = true;

        var x = 8;
        foreach (var control in new Control[]
        {
            accountA, reloadA, devToolsA,
            accountB, reloadB, devToolsB
        })
        {
            control.Location = new Point(x, 7);
            _toolbar.Controls.Add(control);
            x += control.Width + 6;
        }

        _status.Location = new Point(x + 10, 8);
        _toolbar.Controls.Add(_status);
    }

    private static Label CreateLabel(string text) =>
        new()
        {
            Text = text,
            AutoSize = true,
            Padding = new Padding(3, 4, 3, 0)
        };

    private static Button CreateButton(
        string text,
        EventHandler handler)
    {
        var button = new Button
        {
            Text = text,
            AutoSize = true,
            Height = 28
        };

        button.Click += handler;
        return button;
    }

    private async Task InitializeAsync()
    {
        try
        {
            Directory.CreateDirectory(AppConfig.UserDataFolder);

            var options =
                new CoreWebView2EnvironmentOptions
                {
                    AdditionalBrowserArguments =
                        AppConfig.BrowserArguments,
                    AreBrowserExtensionsEnabled = true
                };

            _environment =
                await CoreWebView2Environment.CreateAsync(
                    browserExecutableFolder: null,
                    userDataFolder: AppConfig.UserDataFolder,
                    options: options);

            _controllerA =
                await CreateControllerAsync("AccountA");

            _controllerB =
                await CreateControllerAsync("AccountB");

            _webViewA = _controllerA.CoreWebView2;
            _webViewB = _controllerB.CoreWebView2;

            ConfigureWebView(_webViewA, "A");
            ConfigureWebView(_webViewB, "B");

            var tampermonkeyA =
                await _extensionManager.EnsureTampermonkeyAsync(
                    _webViewA.Profile);

            var tampermonkeyB =
                await _extensionManager.EnsureTampermonkeyAsync(
                    _webViewB.Profile);

            _webViewA.Navigate(AppConfig.GameUrl);
            _webViewB.Navigate(AppConfig.GameUrl);

            _status.Text =
                $"Tampermonkey ready · A {tampermonkeyA.Id[..8]} · B {tampermonkeyB.Id[..8]}";

            LayoutWebViews();
        }
        catch (TampermonkeySetupException ex)
        {
            _status.Text = "Tampermonkey setup required";

            MessageBox.Show(
                this,
                ex.Message,
                "Idle Shell · Tampermonkey",
                MessageBoxButtons.OK,
                MessageBoxIcon.Warning);
        }
        catch (Exception ex)
        {
            _status.Text = "Startup failed";

            MessageBox.Show(
                this,
                ex.ToString(),
                "Idle Shell startup failure",
                MessageBoxButtons.OK,
                MessageBoxIcon.Error);
        }
    }

    private async Task<CoreWebView2Controller> CreateControllerAsync(
        string profileName)
    {
        if (_environment is null)
        {
            throw new InvalidOperationException(
                "WebView2 environment is not initialized.");
        }

        var options =
            _environment.CreateCoreWebView2ControllerOptions();

        options.ProfileName = profileName;
        options.IsInPrivateModeEnabled = false;

        return await _environment
            .CreateCoreWebView2ControllerAsync(
                Handle,
                options);
    }

    private static void ConfigureWebView(
        CoreWebView2 webView,
        string account)
    {
        webView.Settings.AreDefaultContextMenusEnabled = true;
        webView.Settings.AreDevToolsEnabled = true;
        webView.Settings.IsStatusBarEnabled = false;
        webView.Settings.IsZoomControlEnabled = true;

        webView.NewWindowRequested += (_, args) =>
        {
            args.Handled = true;
            webView.Navigate(args.Uri);
        };

        webView.NavigationCompleted += (_, args) =>
        {
            if (!args.IsSuccess)
            {
                Console.Error.WriteLine(
                    $"[IdleShell] Account {account} navigation failed: {args.WebErrorStatus}");
            }
        };
    }

    private void LayoutWebViews()
    {
        var top = AppConfig.ToolbarHeight;
        var height = Math.Max(
            0,
            ClientSize.Height - top);

        var width = Math.Max(
            0,
            ClientSize.Width / 2);

        _controllerA?.Bounds =
            new Rectangle(
                0,
                top,
                width,
                height);

        _controllerB?.Bounds =
            new Rectangle(
                width,
                top,
                ClientSize.Width - width,
                height);
    }

    private void DisposeWebViews()
    {
        _controllerA?.Close();
        _controllerB?.Close();
    }
}
