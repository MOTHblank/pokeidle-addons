using System.Security.Cryptography;

namespace Moth.PokeIdle.IdleShell;

// Minimal reader for Google Chrome Extension (CRX2 / CRX3) packages.
// Tampermonkey ships as a signed CRX whose payload is an ordinary zip of the
// unpacked extension. WebView2's AddBrowserExtensionAsync only accepts an
// already-unpacked folder, so the shell extracts the payload itself.
internal static class CrxPackage
{
    private const uint CrxMagic = 0x32_72_43; // "Cr24" little-endian

    public static bool IsCrxFile(string path)
    {
        try
        {
            using var stream = File.OpenRead(path);
            return ReadMagicAndVersion(stream, out _) == 0;
        }
        catch
        {
            return false;
        }
    }

    // Extracts the embedded zip payload of a CRX file into targetFolder.
    // The previous contents of targetFolder are replaced. Throws
    // TampermonkeySetupException when the package is not a readable CRX or
    // does not contain a manifest.json at its root.
    public static void ExtractTo(string crxPath, string targetFolder)
    {
        byte[] zipPayload;

        try
        {
            using var stream = File.OpenRead(crxPath);
            zipPayload = ReadPayload(stream, crxPath);
        }
        catch (TampermonkeySetupException)
        {
            throw;
        }
        catch (Exception ex)
        {
            throw new TampermonkeySetupException(
                $"Could not read the CRX package '{crxPath}'.", ex);
        }

        var parent = Path.GetDirectoryName(
            Path.GetFullPath(targetFolder))!;
        Directory.CreateDirectory(parent);

        var staging = targetFolder + ".extracting-" +
            Guid.NewGuid().ToString("N");

        try
        {
            ExtractZip(zipPayload, staging, crxPath);

            if (!File.Exists(Path.Combine(staging, "manifest.json")))
            {
                throw new TampermonkeySetupException(
                    $"The CRX package '{crxPath}' does not contain a " +
                    "manifest.json at the archive root and cannot be used " +
                    "as an unpacked extension.");
            }

            if (Directory.Exists(targetFolder))
            {
                Directory.Delete(targetFolder, recursive: true);
            }

            Directory.Move(staging, targetFolder);
        }
        catch
        {
            try
            {
                if (Directory.Exists(staging))
                {
                    Directory.Delete(staging, recursive: true);
                }
            }
            catch
            {
                // Best-effort cleanup of the staging directory.
            }

            throw;
        }
    }

    // Returns 0 on success (magic verified, version written to out), or a
    // non-zero sentinel when the stream does not start with a CRX header.
    private static int ReadMagicAndVersion(
        Stream stream,
        out uint version)
    {
        version = 0;

        Span<byte> magic = stackalloc byte[4];

        if (stream.ReadExactly(magic) ||
            !BitConverter.TryReadUInt32LittleEndian(magic, out var tag) ||
            tag != CrxMagic)
        {
            return 1;
        }

        Span<byte> four = stackalloc byte[4];

        if (stream.ReadExactly(four) ||
            !BitConverter.TryReadUInt32LittleEndian(four, out version))
        {
            return 1;
        }

        return 0;
    }

    private static byte[] ReadPayload(Stream stream, string path)
    {
        if (ReadMagicAndVersion(stream, out var version) != 0)
        {
            throw new TampermonkeySetupException(
                $"'{path}' is not a CRX package (missing Cr24 header).");
        }

        long zipOffset = version switch
        {
            // CRX2: 16-byte header + publicKeyLength + signatureLength.
            2 => ReadLength(stream, path) + ReadLength(stream, path),
            // CRX3: 12-byte header + protobuf header length.
            3 => ReadLength(stream, path),
            _ => throw new TampermonkeySetupException(
                $"'{path}' uses unsupported CRX version {version}.")
        };

        zipOffset += version == 2 ? 16 : 12;

        if (zipOffset >= stream.Length)
        {
            throw new TampermonkeySetupException(
                $"The CRX payload offset in '{path}' is outside the file.");
        }

        stream.Seek(zipOffset, SeekOrigin.Begin);

        using var buffer = new MemoryStream(
            (int)(stream.Length - zipOffset));

        stream.CopyTo(buffer);

        return buffer.ToArray();
    }

    private static long ReadLength(Stream stream, string path)
    {
        Span<byte> four = stackalloc byte[4];

        if (stream.ReadExactly(four) ||
            !BitConverter.TryReadUInt32LittleEndian(four, out var value))
        {
            throw new TampermonkeySetupException(
                $"The CRX header in '{path}' is truncated.");
        }

        return value;
    }

    private static void ExtractZip(
        byte[] zipPayload,
        string destination,
        string sourcePath)
    {
        Directory.CreateDirectory(destination);

        using var archive = new System.IO.Compression.ZipArchive(
            new MemoryStream(zipPayload),
            System.IO.Compression.ZipArchiveMode.Read);

        foreach (var entry in archive.Entries)
        {
            // Skip bare directory entries and anything that escapes the
            // destination (zip-slip protection).
            if (entry.FullName.EndsWith('/') ||
                entry.FullName.EndsWith('\\'))
            {
                continue;
            }

            var targetPath = Path.GetFullPath(
                Path.Combine(destination, entry.FullName));

            if (!targetPath.StartsWith(
                    Path.GetFullPath(destination) +
                    Path.DirectorySeparatorChar,
                    StringComparison.Ordinal))
            {
                throw new TampermonkeySetupException(
                    $"The CRX package '{sourcePath}' contains an unsafe " +
                    $"entry name: {entry.FullName}");
            }

            Directory.CreateDirectory(
                Path.GetDirectoryName(targetPath)!);

            using var input = entry.Open();
            using var output = File.Create(targetPath);

            input.CopyTo(output);
        }
    }
}
