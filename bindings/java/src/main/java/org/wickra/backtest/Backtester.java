package org.wickra.backtest;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SymbolLookup;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandle;

/**
 * Java binding for the wickra-backtest engine, calling the C ABI through the
 * Foreign Function and Memory API (FFM, stable since Java 22).
 *
 * <p>The native library {@code wickra_backtest} is loaded from
 * {@code java.library.path}. Build it first with
 * {@code cargo build -p wickra-backtest-c} and point the path at the Cargo
 * target directory.
 *
 * <p>Results are byte-identical to the Rust, Python, Node.js, WASM and C#
 * bindings: the same engine kernel runs behind every reach.
 */
public final class Backtester {

    // Package-private rather than private: StreamingBacktest resolves its own
    // symbols through the same lookup, so the library is loaded once and both
    // reaches share one linker instead of duplicating the static init.
    static final Linker LINKER = Linker.nativeLinker();
    private static final SymbolLookup LOOKUP;
    private static final MethodHandle RUN;
    private static final MethodHandle RUN_JSON;
    static final MethodHandle FREE;
    private static final MethodHandle VERSION;

    static {
        System.loadLibrary("wickra_backtest");
        LOOKUP = SymbolLookup.loaderLookup();
        RUN = LINKER.downcallHandle(
                symbol("wickra_backtest_run"),
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS,
                        ValueLayout.JAVA_LONG, ValueLayout.ADDRESS, ValueLayout.JAVA_DOUBLE,
                        ValueLayout.ADDRESS));
        RUN_JSON = LINKER.downcallHandle(
                symbol("wickra_backtest_run_json"),
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS, ValueLayout.ADDRESS));
        FREE = LINKER.downcallHandle(
                symbol("wickra_backtest_free_string"),
                FunctionDescriptor.ofVoid(ValueLayout.ADDRESS));
        VERSION = LINKER.downcallHandle(
                symbol("wickra_backtest_version"),
                FunctionDescriptor.of(ValueLayout.ADDRESS));
    }

    private Backtester() {
    }

    static MemorySegment symbol(String name) {
        return LOOKUP.find(name).orElseThrow(
                () -> new UnsatisfiedLinkError("wickra-backtest: missing symbol " + name));
    }

    /** The native library version (do not free; owned by the library). */
    public static String version() {
        try {
            MemorySegment ptr = (MemorySegment) VERSION.invokeExact();
            return ptr.reinterpret(Long.MAX_VALUE).getString(0);
        } catch (Throwable t) {
            throw new RuntimeException("wickra_backtest_version failed", t);
        }
    }

    /**
     * Run a strategy spec over OHLCV data and return the report as JSON.
     *
     * @param open    open prices
     * @param high    high prices
     * @param low     low prices
     * @param close   close prices
     * @param volume  volumes, or {@code null} for all-zero
     * @param time    bar timestamps, or {@code null} for {@code 0..n}
     * @param spec    the strategy spec as JSON
     * @param capital starting capital
     * @return the backtest report as a JSON string
     * @throws IllegalStateException if the inputs or spec are rejected
     */
    public static String run(double[] open, double[] high, double[] low,
                             double[] close, double[] volume, long[] time,
                             String spec, double capital) {
        int n = open.length;
        if (high.length != n || low.length != n || close.length != n) {
            throw new IllegalArgumentException("OHLC arrays must have equal length");
        }
        double[] vol = volume != null ? volume : new double[n];
        long[] ts = time != null ? time : defaultTime(n);
        if (vol.length != n || ts.length != n) {
            throw new IllegalArgumentException("volume and time length must match OHLC");
        }
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment pOpen = arena.allocateFrom(ValueLayout.JAVA_DOUBLE, open);
            MemorySegment pHigh = arena.allocateFrom(ValueLayout.JAVA_DOUBLE, high);
            MemorySegment pLow = arena.allocateFrom(ValueLayout.JAVA_DOUBLE, low);
            MemorySegment pClose = arena.allocateFrom(ValueLayout.JAVA_DOUBLE, close);
            MemorySegment pVol = arena.allocateFrom(ValueLayout.JAVA_DOUBLE, vol);
            MemorySegment pTime = arena.allocateFrom(ValueLayout.JAVA_LONG, ts);
            MemorySegment pSpec = arena.allocateFrom(spec);
            MemorySegment pOut = arena.allocate(ValueLayout.ADDRESS);

            int code = (int) RUN.invokeExact(
                    pOpen, pHigh, pLow, pClose, pVol, pTime,
                    (long) n, pSpec, capital, pOut);

            MemorySegment outPtr = pOut.get(ValueLayout.ADDRESS, 0);
            if (MemorySegment.NULL.equals(outPtr)) {
                throw new IllegalStateException(
                        "wickra_backtest_run returned code " + code + " with no message");
            }
            String json = outPtr.reinterpret(Long.MAX_VALUE).getString(0);
            FREE.invokeExact(outPtr);
            if (code != 0) {
                throw new IllegalStateException(
                        "wickra_backtest_run failed (code " + code + "): " + json);
            }
            return json;
        } catch (RuntimeException e) {
            throw e;
        } catch (Throwable t) {
            throw new RuntimeException("wickra_backtest_run failed", t);
        }
    }

    /**
     * Run a backtest from a single request bundle: a JSON document carrying the
     * candles, the spec, the starting capital and any optional feeds.
     *
     * @param requestJson the request bundle as JSON
     * @return the backtest report as a JSON string
     * @throws IllegalStateException if the request is rejected
     */
    public static String runJson(String requestJson) {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment pReq = arena.allocateFrom(requestJson);
            MemorySegment pOut = arena.allocate(ValueLayout.ADDRESS);

            int code = (int) RUN_JSON.invokeExact(pReq, pOut);

            MemorySegment outPtr = pOut.get(ValueLayout.ADDRESS, 0);
            if (MemorySegment.NULL.equals(outPtr)) {
                throw new IllegalStateException(
                        "wickra_backtest_run_json returned code " + code + " with no message");
            }
            String json = outPtr.reinterpret(Long.MAX_VALUE).getString(0);
            FREE.invokeExact(outPtr);
            if (code != 0) {
                throw new IllegalStateException(
                        "wickra_backtest_run_json failed (code " + code + "): " + json);
            }
            return json;
        } catch (RuntimeException e) {
            throw e;
        } catch (Throwable t) {
            throw new RuntimeException("wickra_backtest_run_json failed", t);
        }
    }

    /** Run with default volume (zero), default timestamps and the given capital. */
    public static String run(double[] open, double[] high, double[] low,
                             double[] close, String spec, double capital) {
        return run(open, high, low, close, null, null, spec, capital);
    }

    /** Run with default volume, timestamps and 10,000 starting capital. */
    public static String run(double[] open, double[] high, double[] low,
                             double[] close, String spec) {
        return run(open, high, low, close, null, null, spec, 10_000.0);
    }

    private static long[] defaultTime(int n) {
        long[] t = new long[n];
        for (int i = 0; i < n; i++) {
            t[i] = i;
        }
        return t;
    }
}
