using System.Text.Json;
using System.Text.Json.Serialization;

namespace Moth.PokeIdle.IdleShell;

internal enum PaneKind { Game }

// Profile names must be alphanumeric (WebView2 restriction).
// Group identifies the game workspace owning a game pane.
// Stream presence is not a PaneSpec and is intentionally not persisted.
internal sealed record PaneSpec(
    string Title,
    string Url,
    string Profile,
    PaneKind Kind,
    string Group = "",
    bool Background = false);

internal static class SessionStore
{
    private static readonly JsonSerializerOptions Options = new()
    {
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() }
    };

    public static List<PaneSpec> Load()
    {
        try
        {
            if (File.Exists(AppConfig.SessionFile))
            {
                var list = JsonSerializer.Deserialize<List<PaneSpec>>(
                    File.ReadAllText(AppConfig.SessionFile), Options);

                var games = list?
                    .Where(p => p.Kind == PaneKind.Game)
                    .Take(2)
                    .ToList();

                if (games is { Count: 2 })
                    return games;
            }
        }
        catch
        {
            // Old sessions containing obsolete stream pane records are intentionally
            // discarded after the stream subsystem redesign.
        }

        return
        [
            new("Account A", AppConfig.GameUrl, "AccountA", PaneKind.Game),
            new("Account B", AppConfig.GameUrl, "AccountB", PaneKind.Game)
        ];
    }

    public static void Save(IEnumerable<PaneSpec> specs)
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(AppConfig.SessionFile)!);
            File.WriteAllText(
                AppConfig.SessionFile,
                JsonSerializer.Serialize(specs, Options));
        }
        catch { }
    }
}
