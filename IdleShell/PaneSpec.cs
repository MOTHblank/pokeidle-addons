using System.Text.Json;
using System.Text.Json.Serialization;

namespace Moth.PokeIdle.IdleShell;

internal enum PaneKind { Game, Stream, ActiveStreamMarker }

// How an inactive stream pane behaves. Background = IsVisible=false (no compositing,
// timers kept alive via browser flags). Parked = off-screen but still rendering
// (fallback if platforms pause when hidden).
internal enum StreamMode { Background, Parked }

// Profile names must be alphanumeric (WebView2 restriction).
internal sealed record PaneSpec(
    string Title, string Url, string Profile, PaneKind Kind,
    StreamMode Mode = StreamMode.Background);

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
                if (list is { Count: > 0 } && list.Count(p => p.Kind == PaneKind.Game) == 2)
                    return list;
            }
        }
        catch { /* fall through to defaults */ }

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
            File.WriteAllText(AppConfig.SessionFile, JsonSerializer.Serialize(specs, Options));
        }
        catch { }
    }
}
