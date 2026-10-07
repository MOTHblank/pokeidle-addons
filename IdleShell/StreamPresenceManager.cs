using System.Text.Json;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

internal enum StreamConnectionState
{
    Disconnected,
    Connecting,
    Connected,
    AuthenticationRequired,
    Error
}

/// <summary>
/// Owns stream-chat presence independently of stream video.
/// There is at most one chat WebView per login profile, regardless of how many
/// channels that login has joined. Twitch and Kick both use their chat-only
/// popout surfaces; video is never loaded by IdleShell.
/// </summary>
internal sealed class StreamPresenceManager : IAsyncDisposable
{
    private readonly CoreWebView2Environment _environment;
    private readonly IntPtr _ownerHwnd;
    private readonly Action<string> _log;
    private readonly Dictionary<string, ChatPresenceHost> _hosts =
        new(StringComparer.OrdinalIgnoreCase);
    private readonly Dictionary<string, Dictionary<string, int>> _channels =
        new(StringComparer.OrdinalIgnoreCase);

    public StreamPresenceManager(
        CoreWebView2Environment environment,
        IntPtr ownerHwnd,
        Action<string> log)
    {
        _environment = environment;
        _ownerHwnd = ownerHwnd;
        _log = log;
    }

    public int JoinedChannelCount =>
        _channels.Values.Sum(c => c.Count);

    public int ConnectedAccountCount =>
        _channels.Count(pair => pair.Value.Count > 0);

    public int ConnectedTwitchAccounts =>
        _hosts.Values.Count(host => host.Service == AccountService.Twitch);

    public int ConnectedKickAccounts =>
        _hosts.Values.Count(host => host.Service == AccountService.Kick);

    public bool IsAccountConnected(string accountId) =>
        _channels.TryGetValue(accountId, out var channels) &&
        channels.Count > 0;

    public bool IsChannelJoined(Account account, string url)
    {
        var channel = ChannelSlug(url);
        return channel is not null &&
               _channels.TryGetValue(account.Id, out var channels) &&
               channels.ContainsKey(channel);
    }

    public async Task JoinAsync(Account account, string url)
    {
        if (!account.IsStream)
            throw new ArgumentException(
                "Only Twitch/Kick accounts can join stream chats.",
                nameof(account));

        var channel = ChannelSlug(url);
        if (channel is null)
            throw new ArgumentException(
                "The URL does not contain a valid Twitch/Kick channel.",
                nameof(url));

        if (!_channels.TryGetValue(account.Id, out var refs))
        {
            refs = new Dictionary<string, int>(StringComparer.OrdinalIgnoreCase);
            _channels[account.Id] = refs;
        }

        if (refs.TryGetValue(channel, out var current))
        {
            refs[channel] = current + 1;
            return;
        }

        ChatPresenceHost? host = null;
        var createdHost = false;
        try
        {
            if (!_hosts.TryGetValue(account.Id, out host))
            {
                host = new ChatPresenceHost(
                    _environment,
                    account.Id,
                    account.Service,
                    _ownerHwnd,
                    _log);
                _hosts[account.Id] = host;
                createdHost = true;
            }

            await host.JoinAsync(channel);
            refs[channel] = 1;

            _log(
                $"chat presence joined {account.Service} {account.DisplayLabel}: {channel}");
        }
        catch
        {
            if (refs.Count == 0)
                _channels.Remove(account.Id);

            if (createdHost && _hosts.Remove(account.Id, out host))
                await host.DisposeAsync();

            throw;
        }
    }

    public async Task LoginAsync(Account account)
    {
        if (!account.IsStream)
            return;

        await BrowserLoginDialog.ShowAsync(
            _environment,
            account.Id,
            AccountManager.LoginUrl(account.Service),
            $"{account.Service} login",
            _ownerHwnd,
            account.Service == AccountService.Twitch
                ? "https://www.twitch.tv/"
                : "https://kick.com/");

        if (_hosts.TryGetValue(account.Id, out var host))
            await host.ReloadAsync();
        else
            _log(
                $"{account.Service} login completed for {account.DisplayLabel}; " +
                "no active chat host needed a reload.");

        _log(
            $"{account.Service} login flow completed for {account.DisplayLabel}; " +
            "chat presence will use this persistent profile");
    }

    public Task OpenExternallyAsync(string url)
    {
        try
        {
            System.Diagnostics.Process.Start(
                new System.Diagnostics.ProcessStartInfo
                {
                    FileName = url,
                    UseShellExecute = true
                });
        }
        catch (Exception ex)
        {
            _log($"external channel open failed: {ex.Message}");
        }

        return Task.CompletedTask;
    }

    public async Task LeaveAsync(Account account, string url)
    {
        var channel = ChannelSlug(url);
        if (channel is null)
            return;

        if (!_channels.TryGetValue(account.Id, out var refs) ||
            !refs.TryGetValue(channel, out var current))
            return;

        if (current > 1)
        {
            refs[channel] = current - 1;
            return;
        }

        refs.Remove(channel);

        if (_hosts.TryGetValue(account.Id, out var host))
            await host.LeaveAsync(channel);

        if (refs.Count == 0)
        {
            _channels.Remove(account.Id);

            if (_hosts.Remove(account.Id, out host))
                await host.DisposeAsync();
        }

        _log(
            $"chat presence left {account.Service} {account.DisplayLabel}: {channel}");
    }

    public async Task ResetAsync()
    {
        foreach (var host in _hosts.Values.ToArray())
            await host.DisposeAsync();

        _hosts.Clear();
        _channels.Clear();
    }

    public async ValueTask DisposeAsync()
    {
        await ResetAsync();
    }

    private static string? ChannelSlug(string url)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri))
            return null;

        var service = AccountManager.ServiceForUrl(url);
        if (service is not (AccountService.Twitch or AccountService.Kick))
            return null;

        var segments = uri.AbsolutePath
            .Split('/', StringSplitOptions.RemoveEmptyEntries);

        if (segments.Length != 1)
            return null;

        var segment = Uri.UnescapeDataString(segments[0]);
        if (string.IsNullOrWhiteSpace(segment))
            return null;

        var excluded = service == AccountService.Twitch
            ? new HashSet<string>(StringComparer.OrdinalIgnoreCase)
            {
                "directory", "downloads", "jobs", "p", "search", "settings",
                "subscriptions", "wallet", "videos", "video", "popout", "embed"
            }
            : new HashSet<string>(StringComparer.OrdinalIgnoreCase)
            {
                "categories", "browse", "directory", "following", "search",
                "settings", "auth", "login", "register", "signup",
                "video", "videos", "popout"
            };

        return excluded.Contains(segment) ? null : segment;
    }
}

/// <summary>
/// One hidden browser surface per stream login. Its document is a chat-only
/// surface for one primary channel; additional joined channels are lightweight
/// hidden iframes inside the same WebView2 renderer/profile.
/// </summary>
internal sealed class ChatPresenceHost : IAsyncDisposable
{
    private readonly CoreWebView2Environment _environment;
    private readonly string _profileId;
    private readonly AccountService _service;
    private readonly IntPtr _ownerHwnd;
    private readonly Action<string> _log;
    private readonly HashSet<string> _channels =
        new(StringComparer.OrdinalIgnoreCase);

    private CoreWebView2Controller? _controller;
    private CoreWebView2? _view;
    private string? _primaryChannel;
    private TaskCompletionSource<bool>? _pageReady;

    public AccountService Service => _service;

    public ChatPresenceHost(
        CoreWebView2Environment environment,
        string profileId,
        AccountService service,
        IntPtr ownerHwnd,
        Action<string> log)
    {
        _environment = environment;
        _profileId = profileId;
        _service = service;
        _ownerHwnd = ownerHwnd;
        _log = log;
    }

    public async Task JoinAsync(string channel)
    {
        if (_channels.Contains(channel))
            return;

        if (_controller is null)
        {
            _channels.Add(channel);
            try
            {
                await StartAsync(channel);
            }
            catch
            {
                _channels.Remove(channel);
                throw;
            }

            return;
        }

        _channels.Add(channel);
        try
        {
            if (!string.Equals(
                    channel,
                    _primaryChannel,
                    StringComparison.OrdinalIgnoreCase))
            {
                await AddFrameAsync(channel);
            }
        }
        catch
        {
            _channels.Remove(channel);
            throw;
        }
    }

    public async Task LeaveAsync(string channel)
    {
        if (!_channels.Remove(channel))
            return;

        if (_controller is null)
            return;

        if (!string.Equals(
                channel,
                _primaryChannel,
                StringComparison.OrdinalIgnoreCase))
        {
            await RemoveFrameAsync(channel);
            return;
        }

        var next = _channels.FirstOrDefault();
        if (next is null)
        {
            _primaryChannel = null;
            return;
        }

        _primaryChannel = next;
        await NavigatePrimaryAsync(next);

        foreach (var remaining in _channels.Where(x =>
                     !string.Equals(
                         x,
                         _primaryChannel,
                         StringComparison.OrdinalIgnoreCase)))
        {
            await AddFrameAsync(remaining);
        }
    }

    public async Task ReloadAsync()
    {
        if (_view is null || _primaryChannel is null)
            return;

        await NavigatePrimaryAsync(_primaryChannel);

        foreach (var channel in _channels.Where(x =>
                     !string.Equals(
                         x,
                         _primaryChannel,
                         StringComparison.OrdinalIgnoreCase)))
        {
            await AddFrameAsync(channel);
        }
    }

    private async Task StartAsync(string primaryChannel)
    {
        if (_controller is not null)
            return;

        _primaryChannel = primaryChannel;

        var options = _environment.CreateCoreWebView2ControllerOptions();
        options.ProfileName = _profileId;
        options.IsInPrivateModeEnabled = false;

        _controller = await _environment.CreateCoreWebView2ControllerAsync(
            _ownerHwnd,
            options);

        // Keep the controller off-screen while allowing WebView2 to consider it
        // visible long enough for the low-memory target to be accepted.
        _controller.Bounds = new System.Drawing.Rectangle(-10000, -10000, 1, 1);
        _controller.IsVisible = true;

        _view = _controller.CoreWebView2;
        _view.Settings.AreDevToolsEnabled = false;
        _view.Settings.IsStatusBarEnabled = false;
        _view.Settings.IsZoomControlEnabled = false;

        _view.ProcessFailed += (_, e) =>
            _log(
                $"{_service} chat WebView failed for {_profileId}: " +
                $"{e.ProcessFailedKind}");

        _pageReady = NewReadySource();
        _view.NavigationCompleted += OnNavigationCompleted;
        _view.Navigate(ChatUrl(_service, primaryChannel));

        await _pageReady.Task.WaitAsync(TimeSpan.FromSeconds(30));

        if (!await HasAuthenticatedSessionAsync())
            throw new AuthenticationRequiredException(
                $"{_service} account '{_profileId}' is not logged in. " +
                "Use Accounts → Log in… first.");

        try
        {
            _view.MemoryUsageTargetLevel =
                CoreWebView2MemoryUsageTargetLevel.Low;
        }
        catch { }

        _controller.IsVisible = false;

        _log(
            $"{_service} chat presence host started for {_profileId}; " +
            $"primary channel {primaryChannel}");
    }

    private async Task NavigatePrimaryAsync(string channel)
    {
        if (_view is null)
            return;

        _primaryChannel = channel;
        _pageReady = NewReadySource();
        _view.Navigate(ChatUrl(_service, channel));
        await _pageReady.Task.WaitAsync(TimeSpan.FromSeconds(30));

        if (!await HasAuthenticatedSessionAsync())
            throw new AuthenticationRequiredException(
                $"{_service} account '{_profileId}' is not logged in.");
    }

    private async Task AddFrameAsync(string channel)
    {
        if (_view is null)
            return;

        var js = $$"""
(() => {
  const slug = {{JsonSerializer.Serialize(channel)}};
  const map = globalThis.__idleshellChatFrames ??= Object.create(null);

  if (map[slug]) return true;

  const frame = document.createElement('iframe');
  frame.src = {{JsonSerializer.Serialize(ChatUrl(_service, channel))}};
  frame.setAttribute('aria-hidden', 'true');
  frame.style.cssText =
    'position:absolute;left:-10000px;top:-10000px;' +
    'width:1px;height:1px;border:0;opacity:0.01;' +
    'pointer-events:none;';

  frame.addEventListener('load', () => {
    try {
      window.chrome?.webview?.postMessage(JSON.stringify({
        type: 'chat-presence',
        service: {{JsonSerializer.Serialize(_service.ToString())}},
        profile: {{JsonSerializer.Serialize(_profileId)}},
        channel: slug,
        state: 'connected'
      }));
    } catch (_) {}
  });

  frame.addEventListener('error', () => {
    try {
      window.chrome?.webview?.postMessage(JSON.stringify({
        type: 'chat-presence',
        service: {{JsonSerializer.Serialize(_service.ToString())}},
        profile: {{JsonSerializer.Serialize(_profileId)}},
        channel: slug,
        state: 'error'
      }));
    } catch (_) {}
  });

  document.body.appendChild(frame);
  map[slug] = frame;

  globalThis.__idleshellRemoveChatFrame ??= (name) => {
    const existing = map[String(name || '')];
    if (!existing) return false;
    existing.remove();
    delete map[String(name || '')];
    return true;
  };

  return true;
})()
""";

        await _view.ExecuteScriptAsync(js);
    }

    private async Task RemoveFrameAsync(string channel)
    {
        if (_view is null)
            return;

        var js =
            $"globalThis.__idleshellRemoveChatFrame?.(" +
            $"{JsonSerializer.Serialize(channel)});";

        await _view.ExecuteScriptAsync(js);
    }

    private async Task<bool> HasAuthenticatedSessionAsync()
    {
        if (_view is null)
            return false;

        if (_service == AccountService.Twitch)
        {
            try
            {
                var cookies = await _view.CookieManager.GetCookiesAsync(
                    "https://www.twitch.tv/");
                return cookies.Any(cookie =>
                    string.Equals(
                        cookie.Name,
                        "auth-token",
                        StringComparison.OrdinalIgnoreCase) &&
                    !string.IsNullOrWhiteSpace(cookie.Value));
            }
            catch
            {
                return false;
            }
        }

        // KICK's public help documents that logged-out users can still read chat,
        // while authenticated users can participate in it. We therefore look
        // specifically for the chat's logged-out state rather than merely
        // checking whether the page loaded.
        try
        {
            var raw = await _view.ExecuteScriptAsync("""
(() => JSON.stringify({
  text: (document.body?.innerText || '').toLowerCase(),
  composer: !!document.querySelector(
    'textarea, input[placeholder*="message" i], [contenteditable="true"]'
  )
}))()
""");

            using var outer = JsonDocument.Parse(raw);
            var inner = outer.RootElement.GetString();
            if (string.IsNullOrWhiteSpace(inner))
                return false;

            using var doc = JsonDocument.Parse(inner);
            var text = doc.RootElement.GetProperty("text").GetString() ?? "";
            var composer = doc.RootElement.GetProperty("composer").GetBoolean();

            if (text.Contains("log in to kick", StringComparison.OrdinalIgnoreCase) ||
                text.Contains("log in to chat", StringComparison.OrdinalIgnoreCase) ||
                text.Contains("login to chat", StringComparison.OrdinalIgnoreCase) ||
                text.Contains("sign in to chat", StringComparison.OrdinalIgnoreCase) ||
                text.Contains("please log in", StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }

            return composer || !text.Contains(
                "disconnected",
                StringComparison.OrdinalIgnoreCase);
        }
        catch
        {
            return false;
        }
    }

    private static string ChatUrl(AccountService service, string channel) =>
        service switch
        {
            AccountService.Twitch =>
                "https://www.twitch.tv/popout/" +
                Uri.EscapeDataString(channel) + "/chat",
            AccountService.Kick =>
                "https://kick.com/popout/" +
                Uri.EscapeDataString(channel) + "/chat",
            _ => throw new ArgumentOutOfRangeException(nameof(service))
        };

    private static TaskCompletionSource<bool> NewReadySource() =>
        new(TaskCreationOptions.RunContinuationsAsynchronously);

    private void OnNavigationCompleted(
        object? sender,
        CoreWebView2NavigationCompletedEventArgs e)
    {
        if (_pageReady is null)
            return;

        if (!e.IsSuccess)
        {
            _pageReady.TrySetException(
                new InvalidOperationException(
                    $"{_service} chat navigation failed: " +
                    $"HTTP {e.HttpStatusCode}."));
            return;
        }

        _pageReady.TrySetResult(true);
    }

    public async ValueTask DisposeAsync()
    {
        try
        {
            if (_view is not null)
                _view.NavigationCompleted -= OnNavigationCompleted;
        }
        catch { }

        try { _controller?.Close(); } catch { }

        _view = null;
        _controller = null;
        _channels.Clear();
        _primaryChannel = null;

        await Task.CompletedTask;
    }
}

internal sealed class AuthenticationRequiredException(string message)
    : Exception(message);

internal sealed class BrowserLoginDialog : Form
{
    private readonly CoreWebView2Environment _environment;
    private readonly string _profileId;
    private readonly string _url;
    private readonly string _cookieUri;
    private readonly string _title;
    private readonly List<CoreWebView2Cookie> _cookies = [];
    private CoreWebView2Controller? _controller;
    private CoreWebView2? _view;
    private Button _done = null!;
    private Label _hint = null!;

    private BrowserLoginDialog(
        CoreWebView2Environment environment,
        string profileId,
        string url,
        string title,
        string cookieUri)
    {
        _environment = environment;
        _profileId = profileId;
        _url = url;
        _title = title;
        _cookieUri = cookieUri;

        Text = title;
        StartPosition = FormStartPosition.CenterParent;
        ClientSize = new System.Drawing.Size(1100, 760);
        MinimizeBox = false;
        MaximizeBox = true;

        _hint = new Label
        {
            Dock = DockStyle.Top,
            Height = 34,
            Text =
                "Sign in normally, then press Done. Idle Shell keeps the login " +
                "in this account's persistent browser profile.",
            TextAlign = ContentAlignment.MiddleLeft,
            Padding = new Padding(10, 0, 10, 0)
        };

        _done = new Button
        {
            Dock = DockStyle.Bottom,
            Height = 34,
            Text = "Done"
        };
        _done.Click += async (_, _) => await FinishAsync();

        Controls.Add(_hint);
        Controls.Add(_done);

        Shown += async (_, _) => await CreateBrowserAsync();
        FormClosed += (_, _) =>
        {
            try { _controller?.Close(); } catch { }
            _controller = null;
            _view = null;
        };

        Resize += (_, _) =>
        {
            if (_controller is not null)
                _controller.Bounds = BrowserBounds();
        };
    }

    public static async Task<IReadOnlyList<CoreWebView2Cookie>> ShowAsync(
        CoreWebView2Environment environment,
        string profileId,
        string url,
        string title,
        IntPtr ownerHwnd,
        string cookieUri)
    {
        using var dialog = new BrowserLoginDialog(
            environment,
            profileId,
            url,
            title,
            cookieUri);

        if (ownerHwnd != IntPtr.Zero)
            _ = ownerHwnd;

        dialog.ShowDialog();
        return dialog._cookies.ToArray();
    }

    private async Task CreateBrowserAsync()
    {
        try
        {
            var options = _environment.CreateCoreWebView2ControllerOptions();
            options.ProfileName = _profileId;
            options.IsInPrivateModeEnabled = false;

            _controller = await _environment.CreateCoreWebView2ControllerAsync(
                Handle,
                options);

            _view = _controller.CoreWebView2;
            _view.Settings.AreDevToolsEnabled = false;
            _view.Settings.IsStatusBarEnabled = false;
            _view.Settings.IsZoomControlEnabled = true;
            _controller.Bounds = BrowserBounds();
            _controller.IsVisible = true;
            _view.Navigate(_url);
        }
        catch (Exception ex)
        {
            _hint.Text = "Login browser failed: " + ex.Message;
        }
    }

    private Rectangle BrowserBounds() =>
        new(
            0,
            _hint.Height,
            ClientSize.Width,
            Math.Max(1, ClientSize.Height - _hint.Height - _done.Height));

    private async Task FinishAsync()
    {
        if (_view is null)
        {
            DialogResult = DialogResult.Cancel;
            return;
        }

        try
        {
            var cookies = await _view.CookieManager.GetCookiesAsync(_cookieUri);
            _cookies.Clear();
            _cookies.AddRange(cookies);
            DialogResult = DialogResult.OK;
        }
        catch (Exception ex)
        {
            MessageBox.Show(
                this,
                "Could not read the login session: " + ex.Message,
                "Login",
                MessageBoxButtons.OK,
                MessageBoxIcon.Error);
        }
    }
}
