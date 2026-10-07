internal sealed class StreamPresenceManager : IAsyncDisposable
{
    private const string TwitchClientId = "kimdg7e24uc9nkkn2h3t43j1uwrcd2";
    private static readonly Uri TwitchIrcUri =
        new("wss://irc-ws.chat.twitch.tv:443");

    private readonly CoreWebView2Environment _environment;
    private readonly IntPtr _ownerHwnd;
    private readonly Action<string> _log;
    private readonly HttpClient _http = new();
    private readonly Dictionary<string, TwitchChatClient> _twitch =
        new(StringComparer.OrdinalIgnoreCase);
    private readonly Dictionary<string, KickChatHost> _kick =
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
        _channels.Count(p => p.Value.Count > 0);

    public int ConnectedTwitchAccounts =>
        _twitch.Count;

    public int ConnectedKickAccounts =>
        _kick.Count;

    public bool IsAccountConnected(string accountId) =>
        _channels.TryGetValue(accountId, out var set) && set.Count > 0;

    public bool IsChannelJoined(Account account, string url)
    {
        var channel = ChannelSlug(url);
        return channel is not null &&
               _channels.TryGetValue(account.Id, out var set) &&
               set.Contains(channel);
    }

    public async Task JoinAsync(Account account, string url)
    {
        if (!account.IsStream)
            throw new ArgumentException("Only Twitch/Kick accounts can join stream chats.", nameof(account));

        var channel = ChannelSlug(url);
        if (channel is null)
            throw new ArgumentException("The URL does not contain a valid Twitch/Kick channel.", nameof(url));

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

        try
        {
            switch (account.Service)
            {
                case AccountService.Twitch:
                    var twitch = await GetTwitchClientAsync(account, interactive: false);
                    if (twitch is null)
                        throw new AuthenticationRequiredException(
                            $"Twitch account '{account.Label}' needs to be logged in again.");

                    await twitch.JoinAsync(channel);
                    break;

                case AccountService.Kick:
                    var kick = await GetKickHostAsync(account, channel);
                    await kick.JoinAsync(channel);
                    break;

                default:
                    throw new ArgumentOutOfRangeException();
            }

            refs[channel] = 1;
            _log($"stream presence joined {account.Service} {account.DisplayLabel}: {channel}");
        }
        catch
        {
            if (refs.Count == 0)
                _channels.Remove(account.Id);
            throw;
        }
    }

    public async Task LoginAsync(Account account)
    {
        if (!account.IsStream)
        {
            _log($"ignored login request for non-stream account {account.Id}");
            return;
        }

        if (account.Service == AccountService.Twitch)
        {
            var cookies = await BrowserLoginDialog.ShowAsync(
                _environment,
                account.Id,
                "https://www.twitch.tv/",
                "Twitch login",
                _ownerHwnd,
                "https://www.twitch.tv/");

            var token = cookies
                .FirstOrDefault(c =>
                    string.Equals(c.Name, "auth-token", StringComparison.OrdinalIgnoreCase) &&
                    !string.IsNullOrWhiteSpace(c.Value))
                ?.Value;

            if (string.IsNullOrWhiteSpace(token))
                throw new InvalidOperationException(
                    "Twitch login completed, but no Twitch auth-token cookie was found. " +
                    "Stay on twitch.tv after signing in and press Done.");

            TwitchCredentialStore.Save(account.Id, token);
            _log($"Twitch credential refreshed for {account.DisplayLabel}");

            if (_twitch.Remove(account.Id, out var old))
                await old.DisposeAsync();

            if (_channels.TryGetValue(account.Id, out var channels) && channels.Count > 0)
            {
                var client = await GetTwitchClientAsync(account, interactive: true);
                if (client is not null)
                {
                    foreach (var channel in channels.Keys.ToArray())
                        await client.JoinAsync(channel);
                }
            }

            return;
        }

        if (account.Service == AccountService.Kick)
        {
            await BrowserLoginDialog.ShowAsync(
                _environment,
                account.Id,
                "https://kick.com/",
                "Kick login",
                _ownerHwnd,
                "https://kick.com/");

            if (_kick.TryGetValue(account.Id, out var host))
                await host.ReloadAsync();

            _log($"Kick login flow completed for {account.DisplayLabel}");
        }
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
            _log($"external stream open failed: {ex.Message}");
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

        switch (account.Service)
        {
            case AccountService.Twitch:
                if (_twitch.TryGetValue(account.Id, out var twitch))
                    await twitch.LeaveAsync(channel);
                break;

            case AccountService.Kick:
                if (_kick.TryGetValue(account.Id, out var kick))
                    await kick.LeaveAsync(channel);
                break;
        }

        if (refs.Count == 0)
        {
            _channels.Remove(account.Id);

            if (_twitch.Remove(account.Id, out var twitch))
                await twitch.DisposeAsync();

            if (_kick.Remove(account.Id, out var kick))
                await kick.DisposeAsync();
        }

        _log($"stream presence left {account.Service} {account.DisplayLabel}: {channel}");
    }

    public async Task ResetAsync()
    {
        foreach (var twitch in _twitch.Values.ToArray())
            await twitch.DisposeAsync();
        foreach (var kick in _kick.Values.ToArray())
            await kick.DisposeAsync();

        _twitch.Clear();
        _kick.Clear();
        _channels.Clear();
    }

    public async ValueTask DisposeAsync()
    {
        await ResetAsync();
        _http.Dispose();
    }

    private async Task<TwitchChatClient?> GetTwitchClientAsync(
        Account account,
        bool interactive)
    {
        if (_twitch.TryGetValue(account.Id, out var existing))
            return existing;

        var token = TwitchCredentialStore.Load(account.Id);

        if (string.IsNullOrWhiteSpace(token))
        {
            token = await TryReadLegacyTwitchTokenAsync(account.Id);
            if (!string.IsNullOrWhiteSpace(token))
                TwitchCredentialStore.Save(account.Id, token);
        }

        if (string.IsNullOrWhiteSpace(token) && interactive)
        {
            var cookies = await BrowserLoginDialog.ShowAsync(
                _environment,
                account.Id,
                "https://www.twitch.tv/",
                "Twitch login",
                _ownerHwnd,
                "https://www.twitch.tv/");

            token = cookies
                .FirstOrDefault(c =>
                    string.Equals(c.Name, "auth-token", StringComparison.OrdinalIgnoreCase) &&
                    !string.IsNullOrWhiteSpace(c.Value))
                ?.Value;

            if (!string.IsNullOrWhiteSpace(token))
                TwitchCredentialStore.Save(account.Id, token);
        }

        if (string.IsNullOrWhiteSpace(token))
            return null;

        var login = await GetTwitchLoginAsync(token);
        if (string.IsNullOrWhiteSpace(login))
        {
            TwitchCredentialStore.Delete(account.Id);
            return null;
        }

        var client = new TwitchChatClient(
            TwitchIrcUri,
            login,
            token,
            _log);

        client.AuthenticationFailed += () =>
        {
            TwitchCredentialStore.Delete(account.Id);
            _log($"Twitch authentication expired for account {account.DisplayLabel}");
        };

        await client.StartAsync();
        _twitch[account.Id] = client;
        return client;
    }

    private async Task<string?> GetTwitchLoginAsync(string token)
    {
        try
        {
            using var request = new HttpRequestMessage(
                HttpMethod.Get,
                "https://api.twitch.tv/helix/users");

            request.Headers.TryAddWithoutValidation("Client-Id", TwitchClientId);
            request.Headers.TryAddWithoutValidation(
                "Authorization",
                $"Bearer {token.Trim().TrimStart()}");

            using var response = await _http.SendAsync(request);
            if (!response.IsSuccessStatusCode)
                return null;

            using var doc = JsonDocument.Parse(await response.Content.ReadAsStringAsync());
            return doc.RootElement
                .GetProperty("data")[0]
                .GetProperty("login")
                .GetString();
        }
        catch (Exception ex)
        {
            _log($"Twitch user lookup failed: {ex.Message}");
            return null;
        }
    }

    private async Task<string?> TryReadLegacyTwitchTokenAsync(string profileId)
    {
        CoreWebView2Controller? controller = null;
        try
        {
            var options = _environment.CreateCoreWebView2ControllerOptions();
            options.ProfileName = profileId;
            options.IsInPrivateModeEnabled = false;

            controller = await _environment.CreateCoreWebView2ControllerAsync(
                new IntPtr(-3),
                options);

            controller.IsVisible = false;
            controller.CoreWebView2.Settings.AreDevToolsEnabled = false;

            var loaded = new TaskCompletionSource<bool>(
                TaskCreationOptions.RunContinuationsAsynchronously);

            controller.CoreWebView2.NavigationCompleted += (_, e) =>
            {
                if (e.IsSuccess)
                    loaded.TrySetResult(true);
                else
                    loaded.TrySetResult(false);
            };

            controller.CoreWebView2.Navigate("https://www.twitch.tv/");
            _ = await loaded.Task.WaitAsync(TimeSpan.FromSeconds(20));

            var cookies = await controller.CoreWebView2.CookieManager
                .GetCookiesAsync("https://www.twitch.tv/");

            return cookies
                .FirstOrDefault(c =>
                    string.Equals(c.Name, "auth-token", StringComparison.OrdinalIgnoreCase) &&
                    !string.IsNullOrWhiteSpace(c.Value))
                ?.Value;
        }
        catch (Exception ex)
        {
            _log($"legacy Twitch credential probe skipped: {ex.Message}");
            return null;
        }
        finally
        {
            try { controller?.Close(); } catch { }
        }
    }

    private async Task<KickChatHost> GetKickHostAsync(Account account, string initialChannel)
    {
        if (_kick.TryGetValue(account.Id, out var existing))
            return existing;

        var host = new KickChatHost(
            _environment,
            account.Id,
            _ownerHwnd,
            _log);

        await host.JoinAsync(initialChannel);
        _kick[account.Id] = host;
        return host;
    }

    private static string? ChannelSlug(string url)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri))
            return null;

        var service = AccountManager.ServiceForUrl(url);
        if (service is not (AccountService.Twitch or AccountService.Kick))
            return null;

        var segment = uri.AbsolutePath
            .Split('/', StringSplitOptions.RemoveEmptyEntries)
            .FirstOrDefault();

        if (string.IsNullOrWhiteSpace(segment))
            return null;

        if (service == AccountService.Twitch)
        {
            var excluded = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
            {
                "directory", "downloads", "jobs", "p", "search", "settings",
                "subscriptions", "wallet", "videos", "video"
            };
            if (excluded.Contains(segment))
                return null;
        }
        else
        {
            var excluded = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
            {
                "categories", "browse", "directory", "following", "search",
                "settings", "auth", "login", "register", "signup",
                "video", "videos"
            };
            if (excluded.Contains(segment))
                return null;
        }

        return Uri.UnescapeDataString(segment);
    }

}

internal sealed class TwitchChatClient : IAsyncDisposable
{
    private readonly Uri _uri;
    private readonly string _login;
    private readonly string _token;
    private readonly Action<string> _log;
    private readonly SemaphoreSlim _sendGate = new(1, 1);
    private readonly HashSet<string> _channels =
        new(StringComparer.OrdinalIgnoreCase);
    private readonly CancellationTokenSource _stop = new();

    private ClientWebSocket? _socket;
    private Task? _receiveTask;
    private volatile bool _authenticationFailed;

    public TwitchChatClient(
        Uri uri,
        string login,
        string token,
        Action<string> log)
    {
        _uri = uri;
        _login = login;
        _token = token.Trim();
        _log = log;
    }

    public event Action? AuthenticationFailed;

    public bool IsAuthenticationFailed => _authenticationFailed;

    public async Task StartAsync()
    {
        if (_receiveTask is not null)
            return;

        await ConnectAsync();

        _receiveTask = Task.Run(ReceiveLoopAsync);
    }

    public async Task JoinAsync(string channel)
    {
        if (_receiveTask is null || _socket?.State != WebSocketState.Open)
            await StartAsync();

        if (_authenticationFailed)
            throw new AuthenticationRequiredException(
                $"Twitch account '{_login}' is no longer authenticated.");

        if (_channels.Add(channel))
            await SendRawAsync($"JOIN #{channel}");
    }

    public async Task LeaveAsync(string channel)
    {
        if (!_channels.Remove(channel) ||
            _socket?.State != WebSocketState.Open)
            return;

        await SendRawAsync($"PART #{channel}");
    }

    private async Task ConnectAsync()
    {
        var socket = new ClientWebSocket();
        await socket.ConnectAsync(_uri, _stop.Token);
        _socket?.Dispose();
        _socket = socket;

        await SendRawAsync($"PASS oauth:{_token}");
        await SendRawAsync($"NICK {_login}");
        await SendRawAsync(
            "CAP REQ :twitch.tv/membership twitch.tv/tags twitch.tv/commands");

        _log($"Twitch chat connected as {_login}; {_channels.Count} channel(s) pending/active");

        foreach (var channel in _channels.ToArray())
            await SendRawAsync($"JOIN #{channel}");
    }

    private async Task SendRawAsync(string line)
    {
        var socket = _socket;
        if (socket is null || socket.State != WebSocketState.Open)
            throw new InvalidOperationException("Twitch chat connection is not open.");

        var bytes = Encoding.UTF8.GetBytes(line + "\r\n");
        await _sendGate.WaitAsync(_stop.Token);
        try
        {
            await socket.SendAsync(
                bytes,
                WebSocketMessageType.Text,
                true,
                _stop.Token);
        }
        finally
        {
            _sendGate.Release();
        }
    }

    private async Task ReceiveLoopAsync()
    {
        var buffer = new byte[16 * 1024];
        var reconnectDelay = TimeSpan.FromSeconds(2);

        while (!_stop.IsCancellationRequested)
        {
            try
            {
                var socket = _socket;
                if (socket is null)
                    throw new InvalidOperationException("Twitch chat socket disappeared.");

                using var ms = new MemoryStream();
                WebSocketReceiveResult result;

                do
                {
                    result = await socket.ReceiveAsync(buffer, _stop.Token);

                    if (result.MessageType == WebSocketMessageType.Close)
                    {
                        await TryCloseSocketAsync(socket);
                        break;
                    }

                    if (result.MessageType != WebSocketMessageType.Text)
                        continue;

                    ms.Write(buffer, 0, result.Count);
                }
                while (!result.EndOfMessage && result.MessageType != WebSocketMessageType.Close);

                if (result.MessageType == WebSocketMessageType.Close)
                    throw new WebSocketException("Twitch requested connection close.");

                var message = Encoding.UTF8.GetString(
                    ms.GetBuffer(),
                    0,
                    checked((int)ms.Length));

                foreach (var line in message.Split(
                             new[] { "\r\n", "\n" },
                             StringSplitOptions.RemoveEmptyEntries))
                {
                    if (line.StartsWith("PING", StringComparison.OrdinalIgnoreCase))
                    {
                        await SendRawAsync("PONG :tmi.twitch.tv");
                        continue;
                    }

                    if (line.Contains("Login authentication failed", StringComparison.OrdinalIgnoreCase) ||
                        line.Contains("Improperly formatted auth", StringComparison.OrdinalIgnoreCase) ||
                        (line.StartsWith(":tmi.twitch.tv NOTICE", StringComparison.OrdinalIgnoreCase) &&
                         line.Contains("authentication", StringComparison.OrdinalIgnoreCase)))
                    {
                        _authenticationFailed = true;
                        AuthenticationFailed?.Invoke();
                        await TryCloseSocketAsync(socket);
                        return;
                    }
                }

                reconnectDelay = TimeSpan.FromSeconds(2);
            }
            catch (OperationCanceledException) when (_stop.IsCancellationRequested)
            {
                return;
            }
            catch (Exception ex)
            {
                if (_stop.IsCancellationRequested || _authenticationFailed)
                    return;

                _log($"Twitch chat disconnected for {_login}: {ex.Message}");
            }

            if (_stop.IsCancellationRequested || _authenticationFailed)
                return;

            try
            {
                await Task.Delay(reconnectDelay, _stop.Token);
            }
            catch (OperationCanceledException)
            {
                return;
            }

            reconnectDelay = TimeSpan.FromSeconds(
                Math.Min(30, reconnectDelay.TotalSeconds * 2));

            try
            {
                await ConnectAsync();
            }
            catch (Exception ex)
            {
                _log($"Twitch chat reconnect failed for {_login}: {ex.Message}");
            }
        }
    }

    private async Task TryCloseSocketAsync(ClientWebSocket socket)
    {
        try
        {
            if (socket.State is WebSocketState.Open or WebSocketState.CloseReceived)
            {
                await socket.CloseAsync(
                    WebSocketCloseStatus.NormalClosure,
                    "IdleShell reconnecting",
                    CancellationToken.None);
            }
        }
        catch { }
        finally
        {
            if (ReferenceEquals(_socket, socket))
                _socket = null;
            socket.Dispose();
        }
    }

    public async ValueTask DisposeAsync()
    {
        _stop.Cancel();

        var socket = _socket;
        if (socket is not null)
            await TryCloseSocketAsync(socket);

        if (_receiveTask is not null)
        {
            try { await _receiveTask; }
            catch { }
        }

        _sendGate.Dispose();
        _stop.Dispose();
    }
}

internal sealed class KickChatHost : IAsyncDisposable
{
    private readonly CoreWebView2Environment _environment;
    private readonly string _profileId;
    private readonly IntPtr _ownerHwnd;
    private readonly Action<string> _log;
    private readonly HashSet<string> _channels =
        new(StringComparer.OrdinalIgnoreCase);

    private CoreWebView2Controller? _controller;
    private CoreWebView2? _view;
    private string? _primaryChannel;
    private TaskCompletionSource<bool>? _pageReady;

    private const string BootstrapScript = """
(() => {
  'use strict';

  const state = globalThis.__idleshellKickChat ??= {
    frames: Object.create(null)
  };

  const add = (slug) => {
    slug = String(slug || '').trim();
    if (!slug || state.frames[slug]) return false;

    const frame = document.createElement('iframe');
    frame.src = 'https://kick.com/popout/' + encodeURIComponent(slug) + '/chat';
    frame.setAttribute('aria-hidden', 'true');
    frame.style.cssText =
      'position:absolute;left:-10000px;top:-10000px;width:320px;height:480px;' +
      'border:0;opacity:0.01;pointer-events:none;';

    frame.addEventListener('load', () => {
      try {
        window.chrome?.webview?.postMessage(JSON.stringify({
          type: 'kick-chat-frame',
          slug,
          state: 'connected'
        }));
      } catch (_) {}
    });

    frame.addEventListener('error', () => {
      try {
        window.chrome?.webview?.postMessage(JSON.stringify({
          type: 'kick-chat-frame',
          slug,
          state: 'error'
        }));
      } catch (_) {}
    });

    document.body.appendChild(frame);
    state.frames[slug] = frame;
    return true;
  };

  const remove = (slug) => {
    slug = String(slug || '').trim();
    const frame = state.frames[slug];
    if (!frame) return false;
    frame.remove();
    delete state.frames[slug];
    return true;
  };

  globalThis.__idleshellKickAdd = add;
  globalThis.__idleshellKickRemove = remove;
  globalThis.__idleshellKickClear = () => {
    for (const slug of Object.keys(state.frames))
      state.frames[slug]?.remove();
    state.frames = Object.create(null);
  };

  document.documentElement.style.background = 'transparent';
  document.body.style.background = 'transparent';
  document.body.style.overflow = 'hidden';
  document.body.style.margin = '0';
})();
""";

    public KickChatHost(
        CoreWebView2Environment environment,
        string profileId,
        IntPtr ownerHwnd,
        Action<string> log)
    {
        _environment = environment;
        _profileId = profileId;
        _ownerHwnd = ownerHwnd;
        _log = log;
    }

    public async Task JoinAsync(string slug)
    {
        if (_channels.Contains(slug))
            return;

        if (_controller is null)
        {
            _channels.Add(slug);
            await StartAsync(slug);
            return;
        }

        _channels.Add(slug);
        await AddFrameAsync(slug);
    }

    public async Task LeaveAsync(string slug)
    {
        if (!_channels.Remove(slug))
            return;

        if (_controller is null)
            return;

        if (string.Equals(slug, _primaryChannel, StringComparison.OrdinalIgnoreCase))
        {
            var next = _channels.FirstOrDefault();
            if (next is null)
            {
                _primaryChannel = null;
                return;
            }

            _primaryChannel = next;
            _pageReady = new TaskCompletionSource<bool>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _view!.Navigate(ChatUrl(next));
            await _pageReady.Task.WaitAsync(TimeSpan.FromSeconds(30));

            foreach (var channel in _channels.Where(x =>
                         !string.Equals(x, _primaryChannel, StringComparison.OrdinalIgnoreCase)))
                await AddFrameAsync(channel);

            return;
        }

        var js = $"globalThis.__idleshellKickRemove?.({JsonSerializer.Serialize(slug)});";
        await _view!.ExecuteScriptAsync(js);
    }

    public async Task ReloadAsync()
    {
        if (_view is null || _primaryChannel is null)
            return;

        _pageReady = new TaskCompletionSource<bool>(
            TaskCreationOptions.RunContinuationsAsynchronously);

        _view.Reload();
        await _pageReady.Task.WaitAsync(TimeSpan.FromSeconds(30));

        foreach (var channel in _channels.Where(x =>
                     !string.Equals(x, _primaryChannel, StringComparison.OrdinalIgnoreCase)))
            await AddFrameAsync(channel);
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

        _controller.IsVisible = false;
        _controller.Bounds = new System.Drawing.Rectangle(-10000, -10000, 1, 1);

        _view = _controller.CoreWebView2;
        _view.Settings.AreDevToolsEnabled = false;
        _view.Settings.IsStatusBarEnabled = false;
        _view.Settings.IsZoomControlEnabled = false;
        _view.WebMessageReceived += OnWebMessage;
        _view.ProcessFailed += (_, e) =>
            _log($"Kick chat WebView process failed for profile {_profileId}: {e.ProcessFailedKind}");

        _pageReady = new TaskCompletionSource<bool>(
            TaskCreationOptions.RunContinuationsAsynchronously);

        _view.NavigationCompleted += OnNavigationCompleted;
        _view.Navigate(ChatUrl(primaryChannel));
        await _pageReady.Task.WaitAsync(TimeSpan.FromSeconds(30));

        try
        {
            _view.MemoryUsageTargetLevel =
                CoreWebView2MemoryUsageTargetLevel.Low;
        }
        catch { }
    }

    private async Task AddFrameAsync(string slug)
    {
        if (_view is null)
            return;

        var js = $"globalThis.__idleshellKickAdd?.({JsonSerializer.Serialize(slug)});";
        await _view.ExecuteScriptAsync(js);
    }

    private static string ChatUrl(string slug) =>
        "https://kick.com/popout/" + Uri.EscapeDataString(slug) + "/chat";

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
                    $"Kick chat host navigation failed: HTTP {e.HttpStatusCode}."));
            return;
        }

        _ = InitializeHostAsync();
    }

    private async Task InitializeHostAsync()
    {
        try
        {
            await _view!.ExecuteScriptAsync(BootstrapScript);
            _pageReady?.TrySetResult(true);
        }
        catch (Exception ex)
        {
            _pageReady?.TrySetException(ex);
        }
    }

    private void OnWebMessage(object? sender, CoreWebView2WebMessageReceivedEventArgs e)
    {
        try
        {
            using var doc = JsonDocument.Parse(e.WebMessageAsJson);
            if (doc.RootElement.TryGetProperty("type", out var type) &&
                type.GetString() == "kick-chat-frame" &&
                doc.RootElement.TryGetProperty("slug", out var slug))
            {
                var state = doc.RootElement.TryGetProperty("state", out var stateValue)
                    ? stateValue.GetString()
                    : "?";
                _log($"Kick chat frame {_profileId}/{slug.GetString()}: {state}");
            }
        }
        catch { }
    }

    public async ValueTask DisposeAsync()
    {
        try
        {
            if (_view is not null)
                _view.WebMessageReceived -= OnWebMessage;
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
        _channels.Count(p => p.Value.Count > 0);

    public int ConnectedTwitchAccounts =>
        _hosts.Values.Count(h => h.Service == AccountService.Twitch);

    public int ConnectedKickAccounts =>
        _hosts.Values.Count(h => h.Service == AccountService.Kick);

    public bool IsAccountConnected(string accountId) =>
        _channels.TryGetValue(accountId, out var set) && set.Count > 0;

    public bool IsChannelJoined(Account account, string url)
    {
        var channel = ChannelSlug(url);
        return channel is not null &&
               _channels.TryGetValue(account.Id, out var set) &&
               set.ContainsKey(channel);
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

        try
        {
            if (!_hosts.TryGetValue(account.Id, out var host))
            {
                host = new ChatPresenceHost(
                    _environment,
                    account.Id,
                    account.Service,
                    _ownerHwnd,
                    _log);

                _hosts[account.Id] = host;
            }

            await host.JoinAsync(channel);
            refs[channel] = 1;

            _log(
                $"stream chat joined {account.Service} {account.DisplayLabel}: {channel}");
        }
        catch
        {
            if (refs.Count == 0)
                _channels.Remove(account.Id);

            if (_channels.Count == 0 || !_channels.ContainsKey(account.Id))
            {
                if (_hosts.Remove(account.Id, out var host))
                    await host.DisposeAsync();
            }

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

        _log($"{account.Service} login flow completed for {account.DisplayLabel}");
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
            _log($"external stream open failed: {ex.Message}");
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
            $"stream chat left {account.Service} {account.DisplayLabel}: {channel}");
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

        var segment = uri.AbsolutePath
            .Split('/', StringSplitOptions.RemoveEmptyEntries)
            .FirstOrDefault();

        if (string.IsNullOrWhiteSpace(segment))
            return null;

        if (service == AccountService.Twitch)
        {
            var excluded = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
            {
                "directory", "downloads", "jobs", "p", "search", "settings",
                "subscriptions", "wallet", "videos", "video", "popout", "embed"
            };
            if (excluded.Contains(segment))
                return null;
        }
        else
        {
            var excluded = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
            {
                "categories", "browse", "directory", "following", "search",
                "settings", "auth", "login", "register", "signup",
                "video", "videos", "popout"
            };
            if (excluded.Contains(segment))
                return null;
        }

        return Uri.UnescapeDataString(segment);
    }
}

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
            if (string.Equals(channel, _primaryChannel, StringComparison.OrdinalIgnoreCase))
                return;

            await AddFrameAsync(channel);
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

        if (string.Equals(channel, _primaryChannel, StringComparison.OrdinalIgnoreCase))
        {
            var next = _channels.FirstOrDefault();
            if (next is null)
            {
                _primaryChannel = null;
                return;
            }

            _primaryChannel = next;
            await NavigatePrimaryAsync(next);

            foreach (var remaining in _channels.Where(x =>
                         !string.Equals(x, _primaryChannel, StringComparison.OrdinalIgnoreCase)))
                await AddFrameAsync(remaining);

            return;
        }

        await RemoveFrameAsync(channel);
    }

    public async Task ReloadAsync()
    {
        if (_view is null || _primaryChannel is null)
            return;

        await NavigatePrimaryAsync(_primaryChannel);

        foreach (var channel in _channels.Where(x =>
                     !string.Equals(x, _primaryChannel, StringComparison.OrdinalIgnoreCase)))
            await AddFrameAsync(channel);
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

        _controller.IsVisible = false;
        _controller.Bounds = new System.Drawing.Rectangle(-10000, -10000, 1, 1);

        _view = _controller.CoreWebView2;
        _view.Settings.AreDevToolsEnabled = false;
        _view.Settings.IsStatusBarEnabled = false;
        _view.Settings.IsZoomControlEnabled = false;

        _view.ProcessFailed += (_, e) =>
            _log(
                $"{_service} chat WebView process failed for profile " +
                $"{_profileId}: {e.ProcessFailedKind}");

        _pageReady = NewReadySource();
        _view.NavigationCompleted += OnNavigationCompleted;

        _view.Navigate(ChatUrl(_primaryChannel));
        await _pageReady.Task.WaitAsync(TimeSpan.FromSeconds(30));

        try
        {
            _view.MemoryUsageTargetLevel =
                CoreWebView2MemoryUsageTargetLevel.Low;
        }
        catch { }

        _log(
            $"{_service} chat presence host started for {_profileId}; " +
            $"primary channel {_primaryChannel}");
    }

    private async Task NavigatePrimaryAsync(string channel)
    {
        if (_view is null)
            return;

        _primaryChannel = channel;
        _pageReady = NewReadySource();
        _view.Navigate(ChatUrl(channel));
        await _pageReady.Task.WaitAsync(TimeSpan.FromSeconds(30));
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
  return true;
})()
""";

        await _view.ExecuteScriptAsync(js);
    }

    private async Task RemoveFrameAsync(string channel)
    {
        if (_view is null)
            return;

        var js = $"globalThis.__idleshellRemoveChatFrame?.({JsonSerializer.Serialize(channel)});";
        await _view.ExecuteScriptAsync(js);
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
                    $"{_service} chat navigation failed: HTTP {e.HttpStatusCode}."));
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
            Text = "Sign in normally, then press Done. Idle Shell only reads the session cookie needed for chat presence.",
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
