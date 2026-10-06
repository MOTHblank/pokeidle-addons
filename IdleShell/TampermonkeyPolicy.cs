using Microsoft.Win32;

namespace Moth.PokeIdle.IdleShell;

internal sealed class TampermonkeyPolicy
{
    private const string PolicyRoot =
        @"Software\Policies\Microsoft\Edge\WebView2";

    private const string JsonImportEntry =
        "1";

    public void Apply(
        TampermonkeyProvisioning provisioning)
    {
        if (string.IsNullOrWhiteSpace(provisioning.Hash))
        {
            throw new TampermonkeySetupException(
                "Tampermonkey provisioning hash is not prepared.");
        }

        using var root =
            Registry.CurrentUser.CreateSubKey(
                $@"{PolicyRoot}\3rdparty\extensions\" +
                $@"{ExtensionManager.TampermonkeyExtensionId}\jsonImport",
                writable: true);

        if (root is null)
        {
            throw new TampermonkeySetupException(
                "Windows denied access to the Idle Shell WebView2 " +
                "Tampermonkey policy location.");
        }

        using var import =
            root.CreateSubKey(
                JsonImportEntry,
                writable: true);

        if (import is null)
        {
            throw new TampermonkeySetupException(
                "Idle Shell could not create the Tampermonkey provisioning " +
                "policy.");
        }

        import.SetValue(
            "hash",
            provisioning.Hash,
            RegistryValueKind.String);

        import.SetValue(
            "url",
            provisioning.Url,
            RegistryValueKind.String);

        import.SetValue(
            "haltOnError",
            0,
            RegistryValueKind.DWord);

        import.SetValue(
            "installAsSystemScripts",
            1,
            RegistryValueKind.DWord);
    }
}
