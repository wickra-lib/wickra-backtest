using System;
using System.Globalization;
using System.IO;
using System.Runtime.CompilerServices;
using System.Text.Json;
using Wickra.Backtest;

// Run the shared EMA-cross strategy from C#, both ways.
//
//   cargo build -p wickra-backtest-c
//   dotnet run --project examples/csharp
//
// Reads the same examples/sample.csv and examples/ema-cross.json every other
// language example uses, runs the whole series at once, then feeds the same bars
// one at a time and checks that the two agree. That equality is the point of the
// library: a live loop is the streaming path with a socket in place of the file,
// so a backtest is not a separate model of the strategy.

internal static class Program
{
    private const double Capital = 10_000.0;

    // Resolved from this source file so the example runs the same whatever the
    // working directory is.
    private static string ExamplesDir([CallerFilePath] string thisFile = "")
        => Path.GetFullPath(Path.Combine(Path.GetDirectoryName(thisFile)!, ".."));

    private static int Main()
    {
        string dir = ExamplesDir();
        string spec = File.ReadAllText(Path.Combine(dir, "ema-cross.json"));
        string[] rows = File.ReadAllLines(Path.Combine(dir, "sample.csv"));

        // The CSV columns are time,open,high,low,close,volume.
        int n = rows.Length - 1;
        var time = new long[n];
        var open = new double[n];
        var high = new double[n];
        var low = new double[n];
        var close = new double[n];
        var volume = new double[n];
        for (int i = 0; i < n; i++)
        {
            string[] f = rows[i + 1].Split(',');
            time[i] = long.Parse(f[0], CultureInfo.InvariantCulture);
            open[i] = double.Parse(f[1], CultureInfo.InvariantCulture);
            high[i] = double.Parse(f[2], CultureInfo.InvariantCulture);
            low[i] = double.Parse(f[3], CultureInfo.InvariantCulture);
            close[i] = double.Parse(f[4], CultureInfo.InvariantCulture);
            volume[i] = double.Parse(f[5], CultureInfo.InvariantCulture);
        }

        string batch = Backtester.Run(open, high, low, close, volume, time, spec, Capital);

        // The same run, driven bar by bar. Replace the loop with reads from a
        // socket and this is a live strategy; nothing else about it changes.
        string streamed;
        using (var live = new StreamingBacktest(spec, Capital))
        {
            for (int i = 0; i < n; i++)
            {
                live.Step(open[i], high[i], low[i], close[i], volume[i], time[i]);
            }
            streamed = live.FinishJson();
        }

        using var report = JsonDocument.Parse(streamed);
        var metrics = report.RootElement.GetProperty("metrics");
        Console.WriteLine($"bars            {n}");
        Console.WriteLine($"trades          {metrics.GetProperty("num_trades").GetInt32()}");
        Console.WriteLine($"pnl             {metrics.GetProperty("pnl").GetDouble():F2}");
        Console.WriteLine($"return %        {metrics.GetProperty("return_pct").GetDouble():F2}");
        Console.WriteLine($"max drawdown    {metrics.GetProperty("max_drawdown").GetDouble():F4}");

        if (streamed != batch)
        {
            Console.Error.WriteLine("streaming and batch disagree -- that should be impossible");
            return 1;
        }
        Console.WriteLine();
        Console.WriteLine("streaming reproduces the batch report exactly");
        return 0;
    }
}
