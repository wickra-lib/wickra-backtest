using System;
using System.Runtime.InteropServices;

namespace Wickra.Backtest;

/// <summary>
/// C# bindings for the wickra-backtest engine over its C ABI. A strategy is a
/// JSON spec, so results are byte-identical to the Rust, Python, Node and WASM
/// bindings.
/// </summary>
public static class Backtester
{
    private const string Lib = "wickra_backtest";

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern int wickra_backtest_run(
        double[] open, double[] high, double[] low, double[] close, double[] volume,
        long[] time, nuint n, [MarshalAs(UnmanagedType.LPUTF8Str)] string specJson,
        double capital, out IntPtr outJson);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern int wickra_backtest_run_json(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string requestJson, out IntPtr outJson);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern void wickra_backtest_free_string(IntPtr s);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr wickra_backtest_version();

    /// <summary>The native library version.</summary>
    public static string Version() => Marshal.PtrToStringUTF8(wickra_backtest_version()) ?? string.Empty;

    /// <summary>
    /// Run a backtest of <paramref name="spec"/> (a JSON string) over the OHLCV
    /// arrays and return the report as a JSON string.
    /// </summary>
    public static string Run(
        double[] open, double[] high, double[] low, double[] close,
        double[]? volume = null, long[]? time = null, string spec = "{}", double capital = 10_000.0)
    {
        int n = open.Length;
        volume ??= new double[n];
        if (time is null)
        {
            time = new long[n];
            for (int i = 0; i < n; i++)
            {
                time[i] = i;
            }
        }

        int code = wickra_backtest_run(open, high, low, close, volume, time, (nuint)n, spec, capital, out IntPtr ptr);
        string payload = Marshal.PtrToStringUTF8(ptr) ?? string.Empty;
        wickra_backtest_free_string(ptr);
        if (code != 0)
        {
            throw new InvalidOperationException($"backtest error ({code}): {payload}");
        }
        return payload;
    }

    /// <summary>
    /// Run a backtest from a single request bundle: a JSON document carrying the
    /// candles, the spec, the starting capital and any optional feeds. Returns
    /// the report as a JSON string.
    /// </summary>
    public static string RunJson(string requestJson)
    {
        int code = wickra_backtest_run_json(requestJson, out IntPtr ptr);
        string payload = Marshal.PtrToStringUTF8(ptr) ?? string.Empty;
        wickra_backtest_free_string(ptr);
        if (code != 0)
        {
            throw new InvalidOperationException($"backtest error ({code}): {payload}");
        }
        return payload;
    }
}
