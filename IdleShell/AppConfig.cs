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

    // Stream environment: default Chromium throttling, autoplay allowed.
    public const string StreamBrowserArguments = GameBrowserArguments;
        "--autoplay-policy=no-user-gesture-required";

    private static string Root =>
        Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Moth", "IdleShell");

    public static string GameUserDataFolder => Path.Combine(Root, "PokeIdle");
    public static string StreamUserDataFolder => Path.Combine(Root, "Streams");
    public static string SessionFile => Path.Combine(Root, "session.json");

    // Put unpacked extensions (e.g. Tampermonkey) in subfolders here.
    public static string ExtensionsFolder =>
        Path.Combine(AppContext.BaseDirectory, "extensions");
}
