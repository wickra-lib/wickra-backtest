using System;
using System.Runtime.InteropServices;

namespace Wickra.Backtest;

/// <summary>
/// A backtest driven one bar at a time.
/// </summary>
/// <remarks>
/// <para>
/// <see cref="Backtester.Run"/> needs the whole series up front. This drives the
/// same engine bar by bar, so a live loop and a backtest are the same code path:
/// feed it from a socket instead of from an array and every value it reports was
/// produced the way the backtest produced it.
/// </para>
/// <para>
/// The instance owns a native handle, so dispose it -- with <c>using</c>, by
/// calling <see cref="Dispose"/>, or by calling <see cref="FinishJson"/>, which
/// ends the run and releases the handle. The finalizer is a backstop for a run
/// that is dropped without any of those.
/// </para>
/// </remarks>
public sealed class StreamingBacktest : IDisposable
{
    private const string Lib = "wickra_backtest";

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern int wickra_backtest_stream_new(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string specJson, double capital,
        out IntPtr outHandle, out IntPtr outErr);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern int wickra_backtest_stream_step(
        IntPtr handle, double open, double high, double low, double close,
        double volume, long time, out IntPtr outErr);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern int wickra_backtest_stream_step_json(
        IntPtr handle, [MarshalAs(UnmanagedType.LPUTF8Str)] string stepJson, out IntPtr outErr);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern int wickra_backtest_stream_equity_json(IntPtr handle, out IntPtr outJson);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern int wickra_backtest_stream_latest_equity_json(IntPtr handle, out IntPtr outJson);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern int wickra_backtest_stream_num_trades(IntPtr handle, out nuint outCount);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern int wickra_backtest_stream_finish_json(IntPtr handle, out IntPtr outJson);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern void wickra_backtest_stream_free(IntPtr handle);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    private static extern void wickra_backtest_free_string(IntPtr s);

    private IntPtr handle;
    private long bars;

    /// <summary>
    /// Start a streaming backtest of <paramref name="spec"/> (a JSON string).
    /// </summary>
    /// <exception cref="InvalidOperationException">The spec is invalid.</exception>
    public StreamingBacktest(string spec, double capital = 10_000.0)
    {
        int code = wickra_backtest_stream_new(spec, capital, out IntPtr created, out IntPtr err);
        if (code != 0)
        {
            string message = Marshal.PtrToStringUTF8(err) ?? string.Empty;
            wickra_backtest_free_string(err);
            throw new InvalidOperationException($"backtest error ({code}): {message}");
        }
        handle = created;
    }

    /// <summary>Whether the run has been finished or disposed.</summary>
    public bool IsFinished => handle == IntPtr.Zero;

    /// <summary>The number of closed trades so far.</summary>
    public long NumTrades
    {
        get
        {
            IntPtr live = Live();
            int code = wickra_backtest_stream_num_trades(live, out nuint count);
            if (code != 0)
            {
                throw new InvalidOperationException($"backtest error ({code})");
            }
            return (long)count;
        }
    }

    /// <summary>
    /// Advance by one bar. <paramref name="time"/> defaults to the number of bars
    /// fed so far, matching <see cref="Backtester.Run"/>'s default.
    /// </summary>
    public void Step(
        double open, double high, double low, double close,
        double volume = 0.0, long? time = null)
    {
        IntPtr live = Live();
        int code = wickra_backtest_stream_step(
            live, open, high, low, close, volume, time ?? bars, out IntPtr err);
        Check(code, err);
        bars++;
    }

    /// <summary>
    /// Advance by one bar described as a request document:
    /// <c>{"candle": {...}, "feeds": {...}}</c>, where <c>feeds</c> optionally
    /// carries this bar's reference, derivatives, order-book, trade or
    /// cross-section input. This is the only form that can drive a strategy
    /// reading a side feed.
    /// </summary>
    public void StepJson(string stepJson)
    {
        IntPtr live = Live();
        int code = wickra_backtest_stream_step_json(live, stepJson, out IntPtr err);
        Check(code, err);
        bars++;
    }

    /// <summary>The equity curve so far, as a JSON array.</summary>
    public string EquityJson()
    {
        IntPtr live = Live();
        return Payload(wickra_backtest_stream_equity_json(live, out IntPtr json), json);
    }

    /// <summary>
    /// The most recent equity point as JSON, or the JSON literal <c>null</c>
    /// before the first bar.
    /// </summary>
    public string LatestEquityJson()
    {
        IntPtr live = Live();
        return Payload(wickra_backtest_stream_latest_equity_json(live, out IntPtr json), json);
    }

    /// <summary>
    /// Close any open position and return the report as a JSON string. Ends the
    /// run: the handle is released and further use throws.
    /// </summary>
    public string FinishJson()
    {
        IntPtr live = Live();
        handle = IntPtr.Zero;
        GC.SuppressFinalize(this);
        return Payload(wickra_backtest_stream_finish_json(live, out IntPtr json), json);
    }

    /// <summary>
    /// Release the run without producing a report. Idempotent.
    /// </summary>
    public void Dispose()
    {
        if (handle != IntPtr.Zero)
        {
            wickra_backtest_stream_free(handle);
            handle = IntPtr.Zero;
        }
        GC.SuppressFinalize(this);
    }

    /// <summary>
    /// Backstop for a run dropped without <see cref="Dispose"/> or
    /// <see cref="FinishJson"/>: the handle owns Rust-side memory, which the GC
    /// cannot reclaim on its own.
    /// </summary>
    ~StreamingBacktest()
    {
        if (handle != IntPtr.Zero)
        {
            wickra_backtest_stream_free(handle);
            handle = IntPtr.Zero;
        }
    }

    private IntPtr Live()
    {
        if (handle == IntPtr.Zero)
        {
            throw new ObjectDisposedException(nameof(StreamingBacktest), "this backtest is finished");
        }
        return handle;
    }

    private static void Check(int code, IntPtr err)
    {
        if (code == 0)
        {
            return;
        }
        string message = Marshal.PtrToStringUTF8(err) ?? string.Empty;
        wickra_backtest_free_string(err);
        throw new InvalidOperationException($"backtest error ({code}): {message}");
    }

    private static string Payload(int code, IntPtr json)
    {
        string payload = Marshal.PtrToStringUTF8(json) ?? string.Empty;
        wickra_backtest_free_string(json);
        if (code != 0)
        {
            throw new InvalidOperationException($"backtest error ({code}): {payload}");
        }
        return payload;
    }
}
