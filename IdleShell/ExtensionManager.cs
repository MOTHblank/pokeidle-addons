using System.Text.Json;
using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

internal sealed class ExtensionManager
{
    public const string TampermonkeyExtensionId =
        "dhdgffkkebhmkfjojejmpbldmpobfkfo";

    public const string TampermonkeyPackageVersion =
        "5.6.6242";

    public const string TampermonkeyPackageUrl =
        "https://data.tampermonkey.net/tampermonkey_5_6_6242.crx";

    private readonly string _tampermonkeyFolder;

    public ExtensionManager(string tampermonkeyFolder)
    {
        _tampermonkeyFolder = Path.GetFullPath(tampermonkeyFolder);
    }

    public async Task<CoreWebView2BrowserExtension> EnsureTampermonkeyAsync(
        CoreWebView2Profile profile)
    {
        var installed = await profile.GetBrowserExtensionsAsync();

        var existing = installed.FirstOrDefault(
            extension => string.Equals(
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
            added = await profile.AddBrowserExtensionAsync(
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

        if (!string.Equals(
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
        var manifestPath = Path.Combine(
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
            using var document = JsonDocument.Parse(
                File.ReadAllText(manifestPath));

            var root = document.RootElement;

            if (!root.TryGetProperty("name", out _) ||
                !root.TryGetProperty("version", out _))
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
}

internal sealed class TampermonkeySetupException : Exception
{
    public TampermonkeySetupException(string message)
        : base(message)
    {
    }

    public TampermonkeySetupException(
        string message,
        Exception innerException)
        : base(message, innerException)
    {
    }
}
