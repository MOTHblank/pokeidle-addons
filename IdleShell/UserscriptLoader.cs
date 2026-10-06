using System.Text.Json;
using System.Text.RegularExpressions;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

internal sealed class UserscriptLoader
{
    private static readonly Regex MetadataLine = new(
        @"^\s*//\s*@(?<key>[A-Za-z][A-Za-z0-9_-]*)\s+(?<value>.+?)\s*$",
        RegexOptions.Compiled);

    private readonly string _addonsFolder;

    public UserscriptLoader(string addonsFolder) => _addonsFolder = addonsFolder;

    public IReadOnlyList<Userscript> Load()
    {
        if (!Directory.Exists(_addonsFolder))
            return [];

        return Directory.EnumerateFiles(_addonsFolder, "*.user.js", SearchOption.TopDirectoryOnly)
            .OrderBy(Path.GetFileName, StringComparer.OrdinalIgnoreCase)
            .Select(Parse)
            .Where(script => script.Matches.Count > 0)
            .ToArray();
    }

    public async Task<int> InjectAsync(
        CoreWebView2 webView,
        CancellationToken cancellationToken = default)
    {
        var scripts = Load();
        foreach (var script in scripts)
        {
            cancellationToken.ThrowIfCancellationRequested();
            await webView.AddScriptToExecuteOnDocumentCreatedAsync(BuildInjection(script));
        }

        return scripts.Count;
    }

    private static Userscript Parse(string path)
    {
        var source = File.ReadAllText(path);
        var matches = new List<string>();
        var name = Path.GetFileNameWithoutExtension(path);
        var runAt = "document-start";

        foreach (var line in source.Split('\n'))
        {
            var match = MetadataLine.Match(line.TrimEnd('\r'));
            if (!match.Success)
                continue;

            switch (match.Groups["key"].Value)
            {
                case "name":
                    name = match.Groups["value"].Value.Trim();
                    break;
                case "match":
                    matches.Add(match.Groups["value"].Value.Trim());
                    break;
                case "run-at":
                    runAt = match.Groups["value"].Value.Trim();
                    break;
            }
        }

        return new Userscript(name, source, matches, runAt, path);
    }

    private static string BuildInjection(Userscript script)
    {
        var source = script.Source
            .Replace("</script", "<\\/script", StringComparison.OrdinalIgnoreCase);

        var name = JsonSerializer.Serialize(script.Name);
        var sourceLiteral = JsonSerializer.Serialize(source);
        var matches = "[" + string.Join(",", script.Matches.Select(JsonSerializer.Serialize)) + "]";

        var run = script.RunAt switch
        {
            "document-idle" => @"
                const run = () => window.__mothRunUserscript(source, name);
                if (document.readyState === 'loading') {
                    window.addEventListener('DOMContentLoaded', run, { once: true });
                } else {
                    setTimeout(run, 0);
                }",
            "document-end" => @"
                const run = () => window.__mothRunUserscript(source, name);
                if (document.readyState === 'loading') {
                    window.addEventListener('DOMContentLoaded', run, { once: true });
                } else {
                    run();
                }",
            _ => "window.__mothRunUserscript(source, name);"
        };

        return $$"""
        (() => {
            const name = {{name}};
            const matches = {{matches}};
            const source = {{sourceLiteral}};

            const urlMatches = (pattern, url) => {
                if (!pattern) return false;
                const escaped = pattern
                    .replace(/[\^$+?.()|{}[\]]/g, '\\$&')
                    .replace(/\*/g, '.*');
                return new RegExp('^' + escaped + '$', 'i').test(url);
            };

            if (!matches.some(pattern => urlMatches(pattern, location.href))) {
                return;
            }

            window.unsafeWindow ??= window;

            window.GM_xmlhttpRequest ??= (details) => {
                const method = details?.method ?? 'GET';
                const url = details?.url;
                const headers = details?.headers ?? {};
                const body = details?.data;

                fetch(url, {
                    method,
                    headers,
                    body,
                    credentials: 'include'
                }).then(async response => {
                    const text = await response.text();
                    details?.onload?.({
                        status: response.status,
                        statusText: response.statusText,
                        responseText: text,
                        response: text,
                        finalUrl: response.url,
                        readyState: 4
                    });
                }).catch(error => details?.onerror?.(error));
            };

            if (!window.__mothRunUserscript) {
                window.__mothRunUserscript = (code, scriptName) => {
                    try {
                        (0, eval)(code);
                    } catch (error) {
                        console.error('[IdleShell] userscript failed:', scriptName, error);
                    }
                };
            }

            {{run}}
        })();
        """;
    }
}
