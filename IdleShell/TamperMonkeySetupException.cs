namespace Moth.PokeIdle.IdleShell;

internal sealed class TampermonkeySetupException : Exception
{
    public TampermonkeySetupException(string message)
        : base(message)
    {
    }

    public TampermonkeySetupException(string message, Exception inner)
        : base(message, inner)
    {
    }
}
