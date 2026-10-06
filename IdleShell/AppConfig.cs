namespace Moth.PokeIdle.IdleShell;

internal static class AppConfig
{
    public const string GameUrl = "https://pokeidle.io/app";
    public const int ToolbarHeight = 40;

    public const string BrowserArguments =
        "--disable-background-timer-throttling " +
        "--disable-renderer-backgrounding " +
        "--disable-backgrounding-occluded-windows";

    public static string UserDataFolder =>
        Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Moth",
            "IdleShell",
            "PokeIdle");

    public static string AddonsFolder =>
        Path.Combine(AppContext.BaseDirectory, "addons");
}
