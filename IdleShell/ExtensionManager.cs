using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

internal sealed class ExtensionManager : IDisposable
{
    public const string TampermonkeyExtensionId =
        "dhdgffkkebhmkfjojejmpbldmpobfkfo";

    public const string TampermonkeyPackageUrl =
        "https://www.tampermonkey.net/crx/tampermonkey_stable.crx";

    private readonly string _tampermonkeyFolder;
    private readonly string _addonsFolder;
    private readonly TampermonkeyPolicy _policy;

    private HttpProvisioningServer? _server;
    private bool _disposed;

    public ExtensionManager(
        string tampermonkeyFolder,
        string addonsFolder)
    {
        _tampermonkeyFolder =
            Path.GetFullPath(tampermonkeyFolder);

        _addonsFolder =
            Path.GetFullPath(addonsFolder);

        _policy =
            new TampermonkeyPolicy();
    }

    // Unpacks the Tampermonkey CRX (if needed), builds the jsonImport
    // provisioning document from addons/*.user.js, serves it over loopback,
    // and writes the third-party extension policy. Must run BEFORE any
    // WebView2 environment is created so the browser picks up the policy at
    // startup. Returns a human-readable summary for the log.
    public string PrepareTampermonkeyProvisioning()
    {
        ThrowIfDisposed();

        EnsureUnpackedTampermonkey();

        var outputPath = Path.Combine(
            AppConfig.TampermonkeyProvisioningFolder,
            "tm.json");

        var provisioning = new TampermonkeyProvisioning(
            _addonsFolder,
            outputPath);

        _server?.Dispose();
        _server = provisioning.OwnedServer;

        provisioning.Prepare();
        _policy.Apply(provisioning);

        return $"{provisioning.ScriptCount} userscript(s) provisioned " +
            $"from {provisioning.AddonsFolder}; " +
            $"jsonImport url={provisioning.Url} hash={provisioning.Hash}";
    }

    // The setup script extracts the CRX to %LOCALAPPDATA%; when it has not
    // been run, fall back to unpacking a *.crx found next to the shell so
    // Tampermonkey can still be installed automatically.
    private void EnsureUnpackedTampermonkey()
    {
        if (File.Exists(Path.Combine(_tampermonkeyFolder, "manifest.json")))
            return;

        var candidates = new List<string>();

        foreach (var dir in new[]
        {
            AppConfig.ExtensionsFolder,
            Path.Combine(AppContext.BaseDirectory, "extensions"),
            _addonsFolder.Length > 0
                ? Path.Combine(
                    Path.GetDirectoryName(_addonsFolder) ?? "",
                    "IdleShell", "extensions")
                : ""
        })
        {
            if (string.IsNullOrEmpty(dir) || !Directory.Exists(dir))
                continue;

            candidates.AddRange(Directory.EnumerateFiles(dir, "*.crx"));
        }

        foreach (var crx in candidates.Distinct(StringComparer.OrdinalIgnoreCase))
        {
            try
            {
                CrxPackage.ExtractTo(crx, _tampermonkeyFolder);

                Console.Error.WriteLine(
                    $"[IdleShell] Unpacked {Path.GetFileName(crx)} into " +
                    $"{_tampermonkeyFolder}");

                return;
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine(
                    $"[IdleShell] Could not unpack {crx}: {ex.Message}");
            }
        }
    }

    public async Task<CoreWebView2BrowserExtension>
        EnsureTampermonkeyAsync(
            CoreWebView2Profile profile)
    {
        ThrowIfDisposed();

        var installed =
            await profile
                .GetBrowserExtensionsAsync();

        var existing =
            installed.FirstOrDefault(
                extension =>
                    string.Equals(
                        extension.Id,
                        TampermonkeyExtensionId,
                        StringComparison.OrdinalIgnoreCase));

        if (existing is not null)
        {
            if (!existing.IsEnabled)
                await existing.EnableAsync(true);

            return existing;
        }

        ValidateExtensionFolder();

        CoreWebView2BrowserExtension added;

        try
        {
            added =
                await profile
                    .AddBrowserExtensionAsync(
                        _tampermonkeyFolder);
        }
        catch (Exception ex)
        {
            throw new TampermonkeySetupException(
                "WebView2 could not install Tampermonkey. " +
                "Verify that browser extensions are enabled and that the " +
                "Tampermonkey package is installed correctly.",
                ex);
        }

        if (
            !string.Equals(
                added.Id,
                TampermonkeyExtensionId,
                StringComparison.OrdinalIgnoreCase))
        {
            try
            {
                await added.RemoveAsync();
            }
            catch
            {
                // Keep the original validation failure as the useful error.
            }

            throw new TampermonkeySetupException(
                "The extension installed from the configured Tampermonkey " +
                "folder does not have the expected Tampermonkey extension ID.");
        }

        return added;
    }

    private void ValidateExtensionFolder()
    {
        var manifestPath =
            Path.Combine(
                _tampermonkeyFolder,
                "manifest.json");

        if (!File.Exists(manifestPath))
        {
            throw new TampermonkeySetupException(
                "Tampermonkey is not installed for Idle Shell.\n\n" +
                $"Expected: {_tampermonkeyFolder}\n\n" +
                "Run IdleShell\\setup-tampermonkey.ps1 once, then start " +
                "Idle Shell again.");
        }

        try
        {
            using var document =
                System.Text.Json.JsonDocument.Parse(
                    File.ReadAllText(
                        manifestPath));

            var root =
                document.RootElement;

            if (
                !root.TryGetProperty(
                    "name",
                    out _) ||
                !root.TryGetProperty(
                    "version",
                    out _))
            {
                throw new InvalidDataException(
                    "The Tampermonkey manifest is missing name or version.");
            }
        }
        catch (TampermonkeySetupException)
        {
            throw;
        }
        catch (Exception ex)
        {
            throw new TampermonkeySetupException(
                "The Tampermonkey manifest is invalid.",
                ex);
        }
    }

    private void ThrowIfDisposed()
    {
        if (_disposed)
        {
            throw new ObjectDisposedException(
                nameof(ExtensionManager));
        }
    }

    public void Dispose()
    {
        if (_disposed)
            return;

        _disposed = true;
        _server?.Dispose();
    }
}
