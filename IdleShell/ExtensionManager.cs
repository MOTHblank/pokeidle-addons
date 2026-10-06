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

    private TampermonkeyProvisioning? _provisioning;
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

    public void PrepareTampermonkeyProvisioning()
    {
        ThrowIfDisposed();

        _provisioning?.Dispose();

        _provisioning =
            new TampermonkeyProvisioning(
                _addonsFolder,
                Path.Combine(
                    AppConfig.TampermonkeyProvisioningFolder,
                    "tm.json"));

        _provisioning.Prepare();
        _policy.Apply(_provisioning);
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
        _provisioning?.Dispose();
    }
}
