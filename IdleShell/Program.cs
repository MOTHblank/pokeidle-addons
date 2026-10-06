namespace Moth.PokeIdle.IdleShell;

internal static class Program
{
    [STAThread]
    private static void Main()
    {
        ApplicationConfiguration.Initialize();
        Application.SetUnhandledExceptionMode(UnhandledExceptionMode.CatchException);
        Application.ThreadException += (_, e) => ShowFatal("UI thread exception", e.Exception);
        AppDomain.CurrentDomain.UnhandledException += (_, e) =>
            LogFatal(e.ExceptionObject as Exception);

        try
        {
            Application.Run(new MainForm());
        }
        catch (Exception ex)
        {
            ShowFatal("Startup exception", ex);
        }
    }

    private static void ShowFatal(string title, Exception ex)
    {
        LogFatal(ex);
        try
        {
            MessageBox.Show(ex.ToString(), "PokéIdle Idle Shell - " + title,
                MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
        catch { }
    }

    private static void LogFatal(Exception? ex)
    {
        if (ex is null) return;
        try
        {
            Directory.CreateDirectory(AppConfig.Root);
            File.AppendAllText(AppConfig.LogFile,
                $"{DateTime.Now:yyyy-MM-dd HH:mm:ss} FATAL {ex}{Environment.NewLine}");
        }
        catch { }
    }
}
