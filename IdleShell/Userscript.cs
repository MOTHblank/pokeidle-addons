namespace Moth.PokeIdle.IdleShell;

internal sealed record Userscript(
    string Name,
    string Source,
    IReadOnlyList<string> Matches,
    string RunAt,
    string Path);
