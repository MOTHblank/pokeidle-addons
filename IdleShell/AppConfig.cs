namespace Moth.PokeIdle.IdleShell;

internal static class AppConfig
{
    public const string GameUrl = "https://pokeidle.io/app";
    public const int ToolbarHeight = 40;

    // Game environment: never throttle, never treat covered windows as hidden.
    public const string GameBrowserArguments =
        "--disable-background-timer-throttling " +
        "--disable-renderer-backgrounding " +
        "--disable-backgrounding-occluded-windows " +
        "--disable-features=CalculateNativeWinOcclusion " +
        "--autoplay-policy=no-user-gesture-required";

    // Stream environment: keeps the unthrottled flags (hidden panes must not have
    // their timers clipped) plus autoplay allowed.
    public const string StreamBrowserArguments = GameBrowserArguments;

    // Probe CSV: logs visibilityState + timer drift per pane, one row per tick.
    public static string ProbeCsvFile => Path.Combine(Root, "probe.csv");

    // Hosts treated as stream links (twitch.tv / kick.com). The link-router
    // userscript and the shell's NewWindowRequested/navigation backstops all
    // match against this pattern — keep the two in sync.
    public const string StreamHostPattern = @"(?:www\.|m\.)?(?:twitch\.tv|kick\.com)";

    // Stream capacity is managed by AccountManager: up to 10 Twitch stream
    // slots + 10 Kick stream slots per game. A login may back the full
    // per-service capacity; accounts.json is authoritative.

    // Account registry managed by the Accounts dialog / AccountManager.
    public static string AccountsFile => Path.Combine(Root, "accounts.json");

    // Optional legacy mirror of enabled stream profiles. Imported on first run,
    // then kept in sync by AccountManager.
    public static string StreamAccountsFile => Path.Combine(Root, "stream-accounts.txt");

    // Event log (link routing, popups, startup).
    public static string LogFile => Path.Combine(Root, "idleshell.log");

    public static string Root =>
        Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Moth", "IdleShell");

    public static string DataRoot => Root;

    public static string GameUserDataFolder => Path.Combine(Root, "PokeIdle");
    public static string StreamUserDataFolder => Path.Combine(Root, "Streams");
    public static string SessionFile => Path.Combine(Root, "session.json");
}
