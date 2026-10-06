namespace Moth.PokeIdle.IdleShell;

internal sealed record Userscript(
    string Name,
    string Namespace,
    string Source,
    IReadOnlyList<string> Matches,
    IReadOnlyList<string> Includes,
    IReadOnlyList<string> Excludes,
    IReadOnlyList<string> ExcludeMatches,
    IReadOnlyList<string> Requires,
    IReadOnlyDictionary<string, string> Resources,
    IReadOnlyList<string> Connects,
    string RunAt,
    string InjectInto,
    bool NoFrames,
    IReadOnlyList<string> Grants,
    string Path);
