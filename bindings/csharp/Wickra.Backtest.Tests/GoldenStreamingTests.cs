using System.IO;
using System.Runtime.CompilerServices;
using System.Text.Json;
using Wickra.Backtest;
using Xunit;

// Driving each shared case one bar at a time must reproduce the same canonical
// report (golden/expected/) the batch entry point produces. GoldenTests pins the
// batch side; this pins that streaming did not drift away from it.
public class GoldenStreamingTests
{
    private static string GoldenDir([CallerFilePath] string thisFile = "")
        => Path.GetFullPath(Path.Combine(Path.GetDirectoryName(thisFile)!, "..", "..", "..", "golden"));

    [Fact]
    public void StreamingGoldenParity()
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

            var open = root.GetProperty("open");
            var high = root.GetProperty("high");
            var low = root.GetProperty("low");
            var close = root.GetProperty("close");
            var volume = root.GetProperty("volume");
            var time = root.GetProperty("time");

            string got;
            using (var bt = new StreamingBacktest(spec, capital))
            {
                for (int i = 0; i < close.GetArrayLength(); i++)
                {
                    bt.Step(
                        open[i].GetDouble(), high[i].GetDouble(), low[i].GetDouble(),
                        close[i].GetDouble(), volume[i].GetDouble(), time[i].GetInt64());
                }
                got = bt.FinishJson();
            }

            string want = File.ReadAllText(Path.Combine(golden, "expected", name + ".json")).Trim();
            Assert.Equal(want, got);
        }
    }
}
