// Throughput benchmark for the wickra-backtest C# binding.
//
// Measures what crossing the boundary costs. Every reach in this repository
// runs the same Rust engine, so a difference between two bindings is not a
// difference in the backtester -- it is the price of that language's FFI, paid
// once per bar on the streaming path and once per run on the batch path.
//
// The strategy is examples/ema-cross.json: two EMAs, a crossover, fractional
// sizing, taker costs, slippage and a trailing stop. A realistic bar rather
// than an empty one, so the figure includes the engine work a real strategy
// does.
//
// Build the C ABI in release first -- the test project copies the debug build
// because tests do not care, but a benchmark against an unoptimised engine
// measures the wrong thing:
//
//   cargo build -p wickra-backtest-c --release
//   dotnet run --project bindings/csharp/benchmarks -c Release              # 200k bars
//   dotnet run --project bindings/csharp/benchmarks -c Release -- --bars 1000000

using System;
using System.Diagnostics;
using System.Globalization;
using Wickra.Backtest;

namespace Wickra.Backtest.Benchmarks;

internal static class Program
{
    private const string Spec =
        """
        {"symbol":"BTCUSDT","timeframe":"1h",
         "indicators":{"ema_fast":{"type":"Ema","params":[5]},
                       "ema_slow":{"type":"Ema","params":[15]}},
         "entry":{"cross_above":["ema_fast","ema_slow"]},
         "exit":{"cross_below":["ema_fast","ema_slow"]},
         "sizing":{"type":"fixed_fraction","fraction":0.95},
         "costs":{"taker_bps":5,"slippage":{"type":"fixed_bps","bps":2}},
         "risk":{"trailing_stop_pct":5.0}}
        """;

    private const double Capital = 10_000.0;
    private const int Reps = 3;

    private static int Main(string[] args)
    {
        int bars = 200_000;
        int flag = Array.IndexOf(args, "--bars");
        if (flag >= 0 && flag + 1 < args.Length)
        {
            if (!int.TryParse(args[flag + 1], out bars) || bars < 1000)
            {
                Console.Error.WriteLine("--bars must be an integer >= 1000");
                return 1;
            }
        }

        // The deterministic synthetic OHLCV every binding's harness builds. No
        // RNG, so two runs are comparable and so are two languages.
        var open = new double[bars];
        var high = new double[bars];
        var low = new double[bars];
        var close = new double[bars];
        var volume = new double[bars];
        var time = new long[bars];
        for (int i = 0; i < bars; i++)
        {
            double mid = 100.0 + Math.Sin(i * 0.001) * 20.0 + i * 1e-4;
            close[i] = mid + Math.Sin(i * 0.05) * 2.0;
            open[i] = i > 0 ? close[i - 1] : close[i];
            high[i] = Math.Max(open[i], close[i]) + 1.5;
            low[i] = Math.Min(open[i], close[i]) - 1.5;
            volume[i] = 1000.0 + (i % 97) * 13;
            time[i] = i;
        }

        double streaming = MedianSeconds(() =>
        {
            using var live = new StreamingBacktest(Spec, Capital);
            for (int i = 0; i < bars; i++)
            {
                live.Step(open[i], high[i], low[i], close[i], volume[i], time[i]);
            }
            live.FinishJson();
        });

        double batch = MedianSeconds(() =>
            Backtester.Run(open, high, low, close, volume, time, Spec, Capital));

        var culture = CultureInfo.InvariantCulture;
        Console.WriteLine($"wickra-backtest C# throughput — {bars.ToString("N0", culture)} bars (median of {Reps} runs)\n");
        Console.WriteLine($"{"path",-14}{"bars/sec",16}{"ns/bar",12}");
        Console.WriteLine(new string('-', 42));
        foreach (var (name, seconds) in new[] { ("streaming", streaming), ("batch", batch) })
        {
            string rate = (bars / seconds).ToString("N0", culture);
            string perBar = (seconds / bars * 1e9).ToString("N0", culture);
            Console.WriteLine($"{name,-14}{rate,16}{perBar,12}");
        }

        Console.WriteLine(
            "\nStreaming crosses the boundary once per bar, with scalars; batch crosses it\n" +
            "once per run and marshals six arrays. P/Invoke charges per call, so which of\n" +
            "the two wins is a property of the language, not of the engine behind both.\n" +
            "Machine-dependent — compare bindings on one machine, not across machines.");
        return 0;
    }

    /// Median wall-clock seconds over `Reps` runs, after one warmup pass that
    /// also pays the JIT so it is not charged to the first measurement.
    private static double MedianSeconds(Action run)
    {
        run();
        var samples = new double[Reps];
        for (int r = 0; r < Reps; r++)
        {
            var watch = Stopwatch.StartNew();
            run();
            watch.Stop();
            samples[r] = watch.Elapsed.TotalSeconds;
        }
        Array.Sort(samples);
        return samples[Reps / 2];
    }
}
