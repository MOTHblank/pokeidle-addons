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

    // Twitch/Kick login profiles. Each intercepted stream link opens one
    // hidden Background-mode pane per profile here, so drops are farmed on
    // every account at once. Add accounts by extending this list; names must
    // be alphanumeric (WebView2 profile restriction).
    public static readonly string[] StreamProfiles = ["Stream1", "Stream2"];

    // Accounts file: one profile name per line, # comments allowed. When it
    // exists it overrides StreamProfiles above.
    public static string StreamAccountsFile => Path.Combine(Root, "stream-accounts.txt");

    // Event log (link routing, popups, startup).
    public static string LogFile => Path.Combine(Root, "idleshell.log");

    public static IReadOnlyList<string> LoadStreamProfiles()
    {
        try
        {
            if (File.Exists(StreamAccountsFile))
            {
                var lines = File.ReadAllLines(StreamAccountsFile)
                    .Select(l => l.Trim())
                    .Where(l => l.Length > 0 && !l.StartsWith('#'))
                    .Where(l => l.All(char.IsLetterOrDigit))
                    .ToArray();
                if (lines.Length > 0) return lines;
            }
        }
        catch { /* fall back to built-in list */ }
        return StreamProfiles;
    }

    private static string Root =>
        Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Moth", "IdleShell");

    public static string GameUserDataFolder => Path.Combine(Root, "PokeIdle");
    public static string StreamUserDataFolder => Path.Combine(Root, "Streams");
    public static string SessionFile => Path.Combine(Root, "session.json");
	public static string TampermonkeyProvisioningFolder =>
		Path.Combine(Root, "TampermonkeyProvisioning");
    // Put unpacked extensions (e.g. Tampermonkey) in subfolders here.
    public static string ExtensionsFolder =>
        Path.Combine(AppContext.BaseDirectory, "extensions");
}
