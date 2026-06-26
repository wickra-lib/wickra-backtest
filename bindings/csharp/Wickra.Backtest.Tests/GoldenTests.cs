using System.IO;
using System.Runtime.CompilerServices;
using System.Text.Json;
using Wickra.Backtest;
using Xunit;

// The C# binding asserts its output against the shared golden reports
// (golden/expected/), pinning cross-language equality. It returns the engine
// JSON verbatim, so the match is byte-for-byte.
public class GoldenTests
{
    private static string GoldenDir([CallerFilePath] string thisFile = "")
        => Path.GetFullPath(Path.Combine(Path.GetDirectoryName(thisFile)!, "..", "..", "..", "golden"));

    private static double[] Doubles(JsonElement root, string key)
    {
        var arr = root.GetProperty(key);
        var r = new double[arr.GetArrayLength()];
        for (int i = 0; i < r.Length; i++)
        {
            r[i] = arr[i].GetDouble();
        }
        return r;
    }

    private static long[] Longs(JsonElement root, string key)
    {
        var arr = root.GetProperty(key);
        var r = new long[arr.GetArrayLength()];
        for (int i = 0; i < r.Length; i++)
        {
            r[i] = arr[i].GetInt64();
        }
        return r;
    }

    [Fact]
    public void GoldenParity()
    {
        string golden = GoldenDir();
        var cases = Directory.GetFiles(Path.Combine(golden, "cases"), "*.json");
        Assert.NotEmpty(cases);
        foreach (var path in cases)
        {
            using var doc = JsonDocument.Parse(File.ReadAllText(path));
            var root = doc.RootElement;
            string name = root.GetProperty("name").GetString()!;
            double capital = root.GetProperty("capital").GetDouble();
            string spec = root.GetProperty("spec").GetRawText();

            string got = Backtester.Run(
                Doubles(root, "open"), Doubles(root, "high"), Doubles(root, "low"),
                Doubles(root, "close"), Doubles(root, "volume"), Longs(root, "time"),
                spec, capital);

            string want = File.ReadAllText(Path.Combine(golden, "expected", name + ".json")).Trim();
            Assert.Equal(want, got);
        }
    }

    // Feed golden parity: each request bundle (golden/requests/) drives a
    // microstructure feed path through run_json, asserted byte-for-byte against
    // the shared expected reports (golden/expected_json/).
    [Fact]
    public void FeedGoldenParity()
    {
        string golden = GoldenDir();
        var requests = Directory.GetFiles(Path.Combine(golden, "requests"), "*.json");
        Assert.NotEmpty(requests);
        foreach (var path in requests)
        {
            string name = Path.GetFileNameWithoutExtension(path);
            string request = File.ReadAllText(path);
            string got = Backtester.RunJson(request);
            string want = File.ReadAllText(Path.Combine(golden, "expected_json", name + ".json")).Trim();
            Assert.Equal(want, got);
        }
    }
}
