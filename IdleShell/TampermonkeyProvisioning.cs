using System.Net;
using System.Net.Sockets;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Moth.PokeIdle.IdleShell;

internal sealed class TampermonkeyProvisioning : IDisposable
{
    private static readonly Regex MetadataLine = new(
        @"^\s*//\s*@(?<key>[A-Za-z][A-Za-z0-9_-]*)\s+(?<value>.+?)\s*$",
        RegexOptions.Compiled);

    private readonly string _addonsFolder;
    private readonly string _outputPath;
    private readonly HttpProvisioningServer _server;

    private bool _disposed;

    public TampermonkeyProvisioning(
        string addonsFolder,
        string outputPath)
    {
        _addonsFolder = Path.GetFullPath(addonsFolder);
        _outputPath = Path.GetFullPath(outputPath);
        _server = new HttpProvisioningServer(_outputPath);
    }

    public string AddonsFolder => _addonsFolder;

    public HttpProvisioningServer OwnedServer => _server;

    public string JsonPath => _outputPath;

    public string Url => _server.Url
        ?? throw new InvalidOperationException(
            "Tampermonkey provisioning server is not started.");

    public string Hash { get; private set; } = string.Empty;

    public int ScriptCount { get; private set; }

    public void Prepare()
    {
        ThrowIfDisposed();

        var payload = BuildPayload();

        var json = JsonSerializer.Serialize(
                payload,
                new JsonSerializerOptions
                {
                    WriteIndented = true
                }) +
            Environment.NewLine;

        Directory.CreateDirectory(
            Path.GetDirectoryName(_outputPath)!);

        File.WriteAllText(
            _outputPath,
            json,
            new UTF8Encoding(
                encoderShouldEmitUTF8Identifier: false));

        Hash = "1:" +
            Convert.ToHexString(
                SHA256.HashData(
                    Encoding.UTF8.GetBytes(json)))
            .ToLowerInvariant();

        _server.Start();
    }

    private ProvisioningDocument BuildPayload()
    {
        ScriptCount = 0;

        if (!Directory.Exists(_addonsFolder))
        {
            return new ProvisioningDocument(
                "1",
                []);
        }

        var files = Directory
            .EnumerateFiles(
                _addonsFolder,
                "*.user.js",
                SearchOption.TopDirectoryOnly)
            .OrderBy(
                Path.GetFileName,
                StringComparer.OrdinalIgnoreCase)
            .ToArray();

        // One bad header must not prevent the remaining addons from being
        // provisioned ("no scripts load at all" failure mode).
        var scripts = files
            .Select(LoadScript)
            .OfType<ParsedUserscript>()
            .Select(
                (script, index) => new ProvisioningScript(
                    script.Name,
                    true,
                    index + 1,
                    CreateStableGuid(
                        script.Path).ToString(),
                    Convert.ToBase64String(
                        Encoding.UTF8.GetBytes(
                            script.Source))))
            .ToArray();

        ScriptCount = scripts.Length;

        return new ProvisioningDocument(
            "1",
            scripts);
    }

    private static ParsedUserscript? LoadScript(string path)
    {
        try
        {
            return ParseScript(path);
        }
        catch (TampermonkeySetupException ex)
        {
            Console.Error.WriteLine(
                $"[IdleShell] Userscript skipped during provisioning: " +
                $"{ex.Message}");

            return null;
        }
    }

    private static ParsedUserscript ParseScript(
        string path)
    {
        var source =
            File.ReadAllText(path);

        var name =
            Path.GetFileNameWithoutExtension(path);

        var hasMatch = false;

        foreach (var line in source.Split('\n'))
        {
            var match =
                MetadataLine.Match(
                    line.TrimEnd('\r'));

            if (!match.Success)
                continue;

            switch (match.Groups["key"].Value)
            {
                case "name":
                    name =
                        match.Groups["value"].Value.Trim();
                    break;

                case "match":
                case "include":
                    hasMatch = true;
                    break;
            }
        }

        if (!hasMatch)
        {
            throw new TampermonkeySetupException(
                $"Userscript '{Path.GetFileName(path)}' has no @match " +
                "or @include metadata.");
        }

        return new ParsedUserscript(
            name,
            source,
            path);
    }

    private static Guid CreateStableGuid(
        string path)
    {
        var normalized =
            Path.GetFullPath(path)
                .Replace('\\', '/')
                .ToLowerInvariant();

        var hash =
            SHA256.HashData(
                Encoding.UTF8.GetBytes(
                    "moth.pokeidle.userscript/" +
                    normalized));

        Span<byte> bytes =
            stackalloc byte[16];

        hash.AsSpan(
                0,
                16)
            .CopyTo(bytes);

        // Mark the stable value as UUID version 5-shaped.
        bytes[6] =
            (byte)(
                (bytes[6] & 0x0f) |
                0x50);

        bytes[8] =
            (byte)(
                (bytes[8] & 0x3f) |
                0x80);

        return new Guid(bytes);
    }

    private void ThrowIfDisposed()
    {
        if (_disposed)
        {
            throw new ObjectDisposedException(
                nameof(TampermonkeyProvisioning));
        }
    }

    public void Dispose()
    {
        if (_disposed)
            return;

        _disposed = true;
        _server.Dispose();
    }

    private sealed record ParsedUserscript(
        string Name,
        string Source,
        string Path);

    private sealed record ProvisioningDocument(
        string Version,
        IReadOnlyList<ProvisioningScript> Scripts);

    private sealed record ProvisioningScript(
        string Name,
        bool Enabled,
        int Position,
        string Uuid,
        string Source);
}

internal sealed class HttpProvisioningServer : IDisposable
{
    private readonly string _filePath;
    private readonly TcpListener _listener;
    private CancellationTokenSource? _cts;
    private Task? _serverTask;
    private bool _disposed;

    public HttpProvisioningServer(
        string filePath)
    {
        _filePath =
            Path.GetFullPath(filePath);

        _listener =
            new TcpListener(
                IPAddress.Loopback,
                0);
    }

    public string? Url { get; private set; }

    public void Start()
    {
        if (_serverTask is not null)
            return;

        _listener.Start();

        var endpoint =
            (IPEndPoint)_listener.LocalEndpoint;

        Url =
            $"http://127.0.0.1:{endpoint.Port}/tm.json";

        _cts =
            new CancellationTokenSource();

        _serverTask =
            Task.Run(
                () => RunAsync(
                    _cts.Token));
    }

    private async Task RunAsync(
        CancellationToken cancellationToken)
    {
        while (
            !cancellationToken
                .IsCancellationRequested)
        {
            TcpClient? client = null;

            try
            {
                client =
                    await _listener
                        .AcceptTcpClientAsync(
                            cancellationToken);

                _ = Task.Run(
                    () => HandleClientAsync(
                        client,
                        cancellationToken),
                    cancellationToken);
            }
            catch (OperationCanceledException)
            {
                break;
            }
            catch (ObjectDisposedException)
            {
                break;
            }
            catch (SocketException)
            {
                if (
                    !cancellationToken
                        .IsCancellationRequested)
                {
                    continue;
                }

                break;
            }
        }
    }

    private async Task HandleClientAsync(
        TcpClient client,
        CancellationToken cancellationToken)
    {
        using (client);

        try
        {
            using var stream =
                client.GetStream();

            var buffer =
                new byte[4096];

            var bytesRead =
                await stream.ReadAsync(
                    buffer,
                    cancellationToken);

            var request =
                Encoding.ASCII.GetString(
                    buffer,
                    0,
                    bytesRead);

            var path =
                ParseRequestPath(request);

            if (
                !string.Equals(
                    path,
                    "/tm.json",
                    StringComparison.Ordinal))
            {
                await WriteResponseAsync(
                    stream,
                    404,
                    "text/plain; charset=utf-8",
                    "Not Found",
                    cancellationToken);

                return;
            }

            var body =
                await File.ReadAllBytesAsync(
                    _filePath,
                    cancellationToken);

            await WriteResponseAsync(
                stream,
                200,
                "application/json; charset=utf-8",
                body,
                cancellationToken);
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine(
                "[IdleShell] Tampermonkey provisioning " +
                $"request failed: {ex}");
        }
    }

    private static string ParseRequestPath(
        string request)
    {
        var firstLineEnd =
            request.IndexOf(
                "\r\n",
                StringComparison.Ordinal);

        if (firstLineEnd < 0)
            return string.Empty;

        var firstLine =
            request[..firstLineEnd];

        var parts =
            firstLine.Split(
                ' ',
                StringSplitOptions.RemoveEmptyEntries);

        if (
            parts.Length < 2 ||
            !string.Equals(
                parts[0],
                "GET",
                StringComparison.Ordinal))
        {
            return string.Empty;
        }

        var rawPath =
            parts[1];

        var queryIndex =
            rawPath.IndexOf('?');

        if (queryIndex >= 0)
            rawPath = rawPath[..queryIndex];

        return Uri.UnescapeDataString(
            rawPath);
    }

    private static async Task WriteResponseAsync(
        NetworkStream stream,
        int statusCode,
        string contentType,
        string text,
        CancellationToken cancellationToken)
    {
        await WriteResponseAsync(
            stream,
            statusCode,
            contentType,
            Encoding.UTF8.GetBytes(text),
            cancellationToken);
    }

    private static async Task WriteResponseAsync(
        NetworkStream stream,
        int statusCode,
        string contentType,
        byte[] body,
        CancellationToken cancellationToken)
    {
        var reason =
            statusCode == 200
                ? "OK"
                : "Not Found";

        var headers =
            $"HTTP/1.1 {statusCode} {reason}\r\n" +
            $"Content-Type: {contentType}\r\n" +
            $"Content-Length: {body.Length}\r\n" +
            "Cache-Control: no-store\r\n" +
            "Connection: close\r\n" +
            "\r\n";

        var headerBytes =
            Encoding.ASCII.GetBytes(headers);

        await stream.WriteAsync(
            headerBytes,
            cancellationToken);

        await stream.WriteAsync(
            body,
            cancellationToken);
    }

    public void Dispose()
    {
        if (_disposed)
            return;

        _disposed = true;

        try
        {
            _cts?.Cancel();
        }
        catch
        {
        }

        try
        {
            _listener.Stop();
        }
        catch
        {
        }

        _cts?.Dispose();
    }
}
