using Microsoft.Web.WebView2.Core;

namespace Moth.PokeIdle.IdleShell;

internal sealed class Pane
{
    public PaneSpec Spec { get; private set; }
    public CoreWebView2Controller Controller { get; }
    public CoreWebView2 View => Controller.CoreWebView2;

    private Pane(PaneSpec spec, CoreWebView2Controller controller)
    {
        Spec = spec;
        Controller = controller;
    }

    public static async Task<Pane> CreateAsync(
        CoreWebView2Environment env, IntPtr hwnd, PaneSpec spec)
    {
        var options = env.CreateCoreWebView2ControllerOptions();
        options.ProfileName = spec.Profile;
        options.IsInPrivateModeEnabled = false;

        var controller = await env.CreateCoreWebView2ControllerAsync(hwnd, options);
        var pane = new Pane(spec, controller);
        await pane.ConfigureAsync();
        pane.View.Navigate(spec.Url);
        return pane;
    }

    private async Task ConfigureAsync()
    {
        var s = View.Settings;
        s.AreDevToolsEnabled = true;
        s.IsStatusBarEnabled = false;
        s.IsZoomControlEnabled = true;

        // Allow popups (Twitch/Kick/Google OAuth need window.opener).
        View.NewWindowRequested += (_, e) => e.Handled = false;

        if (Spec.Kind == PaneKind.Stream)
        {
            View.IsMuted = true;
            View.MemoryUsageTargetLevel = CoreWebView2MemoryUsageTargetLevel.Low;
        }

        await LoadExtensionsAsync();
    }

    private async Task LoadExtensionsAsync()
    {
        if (!Directory.Exists(AppConfig.ExtensionsFolder)) return;

        foreach (var dir in Directory.EnumerateDirectories(AppConfig.ExtensionsFolder))
        {
            if (!File.Exists(Path.Combine(dir, "manifest.json"))) continue;
            try { await View.Profile.AddBrowserExtensionAsync(dir); }
            catch (Exception ex)
            {
                // Already installed in this profile, or unsupported manifest.
                Console.Error.WriteLine($"[IdleShell] extension {dir}: {ex.Message}");
            }
        }
    }

    public void Show(Rectangle bounds)
    {
        Controller.Bounds = bounds;
        Controller.IsVisible = true;
    }


	public void Park()
	{
		Controller.Bounds = new Rectangle(-10000, -10000, 640, 360);
		Controller.IsVisible = true;
}

    // Keep the latest URL so the session restores where you left off.
    public PaneSpec Snapshot()
    {
        var url = View?.Source;
        if (!string.IsNullOrWhiteSpace(url)) Spec = Spec with { Url = url };
        return Spec;
    }

    public void Close() => Controller.Close();
}
