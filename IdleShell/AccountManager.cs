using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;

namespace Moth.PokeIdle.IdleShell;

internal enum AccountService { PokeIdle, Twitch, Kick }

/// <summary>
/// One managed login. Id doubles as the WebView2 profile name (alphanumeric,
/// stable across renames); Label is display-only. Service tells the shell which
/// environment/profile folder the account lives in and which links route to it.
/// </summary>
internal sealed record Account(
    string Id,
    string Label,
    AccountService Service,
    bool Enabled = true)
{
    public bool IsStream => Service is AccountService.Twitch or AccountService.Kick;

    // Display label with the service appended when the label doesn't already
    // mention it, e.g. "main · kick".
    public string DisplayLabel
    {
        get
        {
            var svc = Service.ToString().ToLowerInvariant();
            return Label.Contains(svc, StringComparison.OrdinalIgnoreCase)
                ? Label : $"{Label} · {svc}";
        }
    }
}

/// <summary>
/// Persistent registry of every login the shell manages: PokéIdle game
/// accounts plus up to <see cref="MaxStreamAccounts"/> Twitch/Kick stream
/// accounts. Stored as JSON at AppConfig.AccountsFile; auto-seeded on first
/// run (AccountA/AccountB games, Stream1/Stream2 logins). The legacy flat
/// stream-accounts.txt file is imported once when no JSON store exists yet.
///
/// Adding/removing accounts happens through the Accounts dialog (UI.AccountsDialog)
/// or by editing the JSON file directly while the app is closed.
/// </summary>
internal sealed class AccountManager
{
    public const int MaxStreamAccountsPerService = 10;
    public const int MaxStreamAccounts = MaxStreamAccountsPerService * 2;
    public const int MaxStreamsPerService = 10;
    // A login may back the full service capacity. The old 2-slots-per-login
    // rule created an artificial 4-stream ceiling across the two game panes.
    public const int MaxStreamSlotsPerAccount = MaxStreamsPerService;
    public const int MaxStreamSlots = MaxStreamsPerService * 2;
    public const int DefaultVisibleStreams = 2;

    private static readonly Regex GeneratedIdPattern =
        new(@"^(?:Stream|Account)(\d+)$", RegexOptions.IgnoreCase | RegexOptions.Compiled);
    private static readonly Regex AlnumOnly =
        new(@"^[A-Za-z0-9]+$", RegexOptions.Compiled);

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() }
    };

    private readonly List<Account> _accounts = [];

    public event Action? Changed;

    public IReadOnlyList<Account> All => _accounts;
    public IEnumerable<Account> GameAccounts => _accounts.Where(a => a.Service == AccountService.PokeIdle);
    public IEnumerable<Account> StreamAccounts => _accounts.Where(a => a.IsStream);
    public IEnumerable<Account> EnabledStreamAccounts => _accounts.Where(a => a.IsStream && a.Enabled);

    public int VisibleStreamCount { get; set; } = DefaultVisibleStreams;

    public Account? Find(string id) =>
        _accounts.FirstOrDefault(a => string.Equals(a.Id, id, StringComparison.OrdinalIgnoreCase));

    /// <summary>Enabled stream accounts whose service matches the URL's host.</summary>
    public IReadOnlyList<Account> StreamAccountsForUrl(string url)
    {
        var service = ServiceForUrl(url);
        if (service is null) return [];
        return EnabledStreamAccounts.Where(a => a.Service == service.Value).ToArray();
    }

    public static AccountService? ServiceForUrl(string? url)
    {
        if (url is null || !Uri.TryCreate(url, UriKind.Absolute, out var u))
            return null;

        return u.Host.ToLowerInvariant() switch
        {
            "twitch.tv" or "www.twitch.tv" or "m.twitch.tv" => AccountService.Twitch,
            "kick.com" or "www.kick.com" or "m.kick.com" => AccountService.Kick,
            "pokeidle.io" or "www.pokeidle.io" => AccountService.PokeIdle,
            _ => null
        };
    }

    // Landing pages for logging an account in. Twitch goes straight to its
    // authorize page (the "Log in with Twitch" button on twitch.tv only works
    // from a page that requested scopes); Kick's front page has the login link.
    public static string LoginUrl(AccountService service) => service switch
    {
        AccountService.Twitch =>
            "https://www.twitch.tv/authorize?client_id=kimdg7e24uc9nkkn2h3t43j1uwrcd2" +
            "&redirect_uri=https%3A%2F%2Fwww.twitch.tv%2F&response_type=token" +
            "&scope=user%3Aread%3Aemail&force_verify=true",
        AccountService.Kick => "https://kick.com/",
        AccountService.PokeIdle => AppConfig.GameUrl,
        _ => AppConfig.GameUrl
    };

    public void SetVisibleStreamCount(int n)
    {
        VisibleStreamCount = Math.Clamp(n, 1, MaxStreamSlots);
        Save();
        Changed?.Invoke();
    }

    public Account EnsureStreamAccount(AccountService service)
    {
        if (!service.IsStream())
            throw new ArgumentException("Only Twitch/Kick accounts can be ensured.", nameof(service));

        var existing = StreamAccounts.FirstOrDefault(a => a.Service == service);
        if (existing is not null)
            return existing;

        var label = service == AccountService.Kick ? "Kick 1" : "Twitch 1";
        return Add(label, service);
    }

    public Account Add(string label, AccountService service)
    {
        label = SanitizeLabel(label);
        if (label.Length == 0)
            throw new ArgumentException("Account label must not be empty.");
        if (service.IsStream() &&
            StreamAccounts.Count(a => a.Service == service) >= MaxStreamAccountsPerService)
            throw new InvalidOperationException(
                $"At most {MaxStreamAccountsPerService} {service} login profiles are supported.");

        var account = new Account(NextId(service), label, service);
        _accounts.Add(account);
        Save();
        Changed?.Invoke();
        return account;
    }

    public void Rename(Account account, string label)
    {
        label = SanitizeLabel(label);
        if (label.Length == 0)
            throw new ArgumentException("Account label must not be empty.");
        Replace(account, account with { Label = label });
    }

    public void SetService(Account account, AccountService service)
    {
        // Moving into the stream pool still respects the cap.
        if (service.IsStream() &&
            account.Service != service &&
            StreamAccounts.Count(a => a.Service == service) >= MaxStreamAccountsPerService)
            throw new InvalidOperationException(
                $"At most {MaxStreamAccountsPerService} {service} login profiles are supported.");
        Replace(account, account with { Service = service });
    }

    public void SetEnabled(Account account, bool enabled)
    {
        Replace(account, account with { Enabled = enabled });
    }

    public void Remove(Account account)
    {
        _accounts.Remove(account);
        Save();
        Changed?.Invoke();
    }

    private void Replace(Account old, Account updated)
    {
        var i = _accounts.IndexOf(old);
        if (i >= 0) _accounts[i] = updated;
        Save();
        Changed?.Invoke();
    }

    private string NextId(AccountService service)
    {
        var prefix = service switch
        {
            AccountService.PokeIdle => "Account",
            AccountService.Kick => "StreamKick",
            _ => "Stream"
        };

        var max = 0;
        foreach (var a in _accounts)
        {
            var m = GeneratedIdPattern.Match(a.Id);
            if (!m.Success || m.Groups[1].Value.StartsWith('0'))
                continue;

            var belongs = service switch
            {
                AccountService.PokeIdle =>
                    a.Id.StartsWith("Account", StringComparison.OrdinalIgnoreCase),

                // New Kick profiles are explicitly named StreamKickN. Legacy
                // StreamN profiles remain valid Kick profiles when already
                // stored in accounts.json and are not renumbered.
                AccountService.Kick =>
                    a.Id.StartsWith("StreamKick", StringComparison.OrdinalIgnoreCase),

                AccountService.Twitch =>
                    a.Id.StartsWith("Stream", StringComparison.OrdinalIgnoreCase) &&
                    !a.Id.StartsWith("StreamKick", StringComparison.OrdinalIgnoreCase),

                _ => false
            };

            if (belongs)
                max = Math.Max(max, int.Parse(m.Groups[1].Value));
        }

        var candidate = $"{prefix}{max + 1}";
        while (_accounts.Any(a =>
                   string.Equals(a.Id, candidate, StringComparison.OrdinalIgnoreCase)))
        {
            max++;
            candidate = $"{prefix}{max + 1}";
        }

        return candidate;
    }

    private static string SanitizeLabel(string label) => label.Trim();

    // --- Persistence ----------------------------------------------------------

    public static AccountManager Load()
    {
        var mgr = new AccountManager();
        try
        {
            if (File.Exists(AppConfig.AccountsFile))
            {
                var doc = JsonSerializer.Deserialize<AccountFile>(
                    File.ReadAllText(AppConfig.AccountsFile), JsonOptions);
                if (doc?.Accounts is { Count: > 0 } list)
                {
                    mgr._accounts.AddRange(list.Where(IsValid));
                    mgr.VisibleStreamCount =
                        Math.Clamp(doc.VisibleStreams ?? DefaultVisibleStreams, 1, MaxStreamSlots);
                }
            }
        }
        catch { /* fall through to seeding */ }

        if (mgr._accounts.Count == 0)
            mgr.SeedDefaults();

        // Keep the legacy text file in sync so external tooling / hand edits
        // still show the current stream profile list.
        mgr.WriteLegacyAccountsFile();
        return mgr;
    }

    private void SeedDefaults()
    {
        // Prefer the legacy stream-accounts.txt when present (first-run import).
        var legacy = ReadLegacyProfiles();
        _accounts.Add(new Account("AccountA", "Account A", AccountService.PokeIdle));
        _accounts.Add(new Account("AccountB", "Account B", AccountService.PokeIdle));
        if (legacy.Count > 0)
        {
            foreach (var p in legacy.Take(MaxStreamAccounts))
                _accounts.Add(new Account(p, p, GuessStreamService(p)));
        }
        else
        {
            _accounts.Add(new Account("Stream1", "Stream 1", AccountService.Twitch));
            _accounts.Add(new Account("Stream2", "Stream 2", AccountService.Twitch));
            _accounts.Add(new Account("StreamKick1", "Kick 1", AccountService.Kick));
        }
        Save();
    }

    // Kick logins often carry a "kick"/"kik" hint in the chosen name; default
    // everything else to Twitch (the dominant drop-farming platform).
    private static AccountService GuessStreamService(string profile) =>
        profile.Contains("kick", StringComparison.OrdinalIgnoreCase) ||
        profile.Contains("kik", StringComparison.OrdinalIgnoreCase)
            ? AccountService.Kick
            : AccountService.Twitch;

    private static bool IsValid(Account a) =>
        !string.IsNullOrWhiteSpace(a.Id) && AlnumOnly.IsMatch(a.Id) &&
        !string.IsNullOrWhiteSpace(a.Label);

    private IReadOnlyList<string> ReadLegacyProfiles()
    {
        try
        {
            if (File.Exists(AppConfig.StreamAccountsFile))
                return File.ReadAllLines(AppConfig.StreamAccountsFile)
                    .Select(l => l.Trim())
                    .Where(l => l.Length > 0 && !l.StartsWith('#'))
                    .Where(l => AlnumOnly.IsMatch(l))
                    .Distinct(StringComparer.OrdinalIgnoreCase)
                    .ToArray();
        }
        catch { }
        return [];
    }

    private void WriteLegacyAccountsFile()
    {
        try
        {
            Directory.CreateDirectory(AppConfig.Root);
            File.WriteAllText(AppConfig.StreamAccountsFile, string.Join(Environment.NewLine,
                ["# Managed by Idle Shell's AccountManager (accounts.json).",
                 "# This file mirrors the enabled Twitch/Kick login profiles.",
                 ..StreamAccounts.Select(a => a.Enabled ? a.Id : $"# {a.Id} (disabled)"),
                 ""]));
        }
        catch { }
    }

    public void Save()
    {
        try
        {
            Directory.CreateDirectory(AppConfig.Root);
            var doc = new AccountFile(_accounts, VisibleStreamCount);
            File.WriteAllText(AppConfig.AccountsFile,
                JsonSerializer.Serialize(doc, JsonOptions));
        }
        catch { }
    }

    private sealed record AccountFile(List<Account> Accounts, int? VisibleStreams);
}

file static class AccountServiceExtensions
{
    public static bool IsStream(this AccountService service) =>
        service is AccountService.Twitch or AccountService.Kick;
}
