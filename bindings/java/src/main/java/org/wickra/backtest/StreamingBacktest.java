package org.wickra.backtest;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandle;

/**
 * A backtest driven one bar at a time.
 *
 * <p>{@link Backtester#run} needs the whole series up front. This drives the
 * same engine bar by bar, so a live loop and a backtest are the same code path:
 * feed it from a socket instead of from an array and every value it reports was
 * produced the way the backtest produced it.
 *
 * <p>The instance owns a native handle, so it is {@link AutoCloseable} and
 * belongs in a try-with-resources block. {@link #finishJson()} also releases the
 * handle, and {@link #close()} afterwards is a no-op, so the two compose.
 *
 * <p>Results are byte-identical to the streaming reach of every other binding:
 * one engine kernel, one set of numbers.
 */
public final class StreamingBacktest implements AutoCloseable {

    private static final MethodHandle STREAM_NEW;
    private static final MethodHandle STREAM_STEP;
    private static final MethodHandle STREAM_STEP_JSON;
    private static final MethodHandle STREAM_EQUITY_JSON;
    private static final MethodHandle STREAM_LATEST_EQUITY_JSON;
    private static final MethodHandle STREAM_NUM_TRADES;
    private static final MethodHandle STREAM_FINISH_JSON;
    private static final MethodHandle STREAM_FREE;

    static {
        STREAM_NEW = Backtester.LINKER.downcallHandle(
                Backtester.symbol("wickra_backtest_stream_new"),
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS, ValueLayout.JAVA_DOUBLE,
                        ValueLayout.ADDRESS, ValueLayout.ADDRESS));
        STREAM_STEP = Backtester.LINKER.downcallHandle(
                Backtester.symbol("wickra_backtest_stream_step"),
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS,
                        ValueLayout.JAVA_DOUBLE, ValueLayout.JAVA_DOUBLE,
                        ValueLayout.JAVA_DOUBLE, ValueLayout.JAVA_DOUBLE,
                        ValueLayout.JAVA_DOUBLE, ValueLayout.JAVA_LONG,
                        ValueLayout.ADDRESS));
        STREAM_STEP_JSON = Backtester.LINKER.downcallHandle(
                Backtester.symbol("wickra_backtest_stream_step_json"),
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT,
                        ValueLayout.ADDRESS, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
        STREAM_EQUITY_JSON = Backtester.LINKER.downcallHandle(
                Backtester.symbol("wickra_backtest_stream_equity_json"),
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
        STREAM_LATEST_EQUITY_JSON = Backtester.LINKER.downcallHandle(
                Backtester.symbol("wickra_backtest_stream_latest_equity_json"),
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
        STREAM_NUM_TRADES = Backtester.LINKER.downcallHandle(
                Backtester.symbol("wickra_backtest_stream_num_trades"),
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
        STREAM_FINISH_JSON = Backtester.LINKER.downcallHandle(
                Backtester.symbol("wickra_backtest_stream_finish_json"),
                FunctionDescriptor.of(
                        ValueLayout.JAVA_INT, ValueLayout.ADDRESS, ValueLayout.ADDRESS));
        STREAM_FREE = Backtester.LINKER.downcallHandle(
                Backtester.symbol("wickra_backtest_stream_free"),
                FunctionDescriptor.ofVoid(ValueLayout.ADDRESS));
    }

    private MemorySegment handle;
    private long bars;

    /**
     * Start a streaming backtest.
     *
     * @param spec    the strategy spec as JSON
     * @param capital starting capital
     * @throws IllegalStateException if the spec is rejected
     */
    public StreamingBacktest(String spec, double capital) {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment pSpec = arena.allocateFrom(spec);
            MemorySegment pHandle = arena.allocate(ValueLayout.ADDRESS);
            MemorySegment pErr = arena.allocate(ValueLayout.ADDRESS);

            int code = (int) STREAM_NEW.invokeExact(pSpec, capital, pHandle, pErr);
            if (code != 0) {
                throw new IllegalStateException(
                        "wickra_backtest_stream_new failed (code " + code + "): "
                                + take(pErr.get(ValueLayout.ADDRESS, 0)));
            }
            handle = pHandle.get(ValueLayout.ADDRESS, 0);
        } catch (RuntimeException e) {
            throw e;
        } catch (Throwable t) {
            throw new RuntimeException("wickra_backtest_stream_new failed", t);
        }
    }

    /** Start a streaming backtest with 10,000 starting capital. */
    public StreamingBacktest(String spec) {
        this(spec, 10_000.0);
    }

    /** Whether the run has been finished or closed. */
    public boolean isFinished() {
        return handle == null;
    }

    /**
     * Advance the backtest by one OHLCV bar.
     *
     * @throws IllegalStateException if the run is finished or the bar is rejected
     */
    public void step(double open, double high, double low, double close,
                     double volume, long time) {
        MemorySegment live = live();
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment pErr = arena.allocate(ValueLayout.ADDRESS);
            int code = (int) STREAM_STEP.invokeExact(
                    live, open, high, low, close, volume, time, pErr);
            if (code != 0) {
                throw new IllegalStateException(
                        "wickra_backtest_stream_step failed (code " + code + "): "
                                + take(pErr.get(ValueLayout.ADDRESS, 0)));
            }
            bars++;
        } catch (RuntimeException e) {
            throw e;
        } catch (Throwable t) {
            throw new RuntimeException("wickra_backtest_stream_step failed", t);
        }
    }

    /**
     * Advance by one bar with zero volume and the bar index as its timestamp,
     * mirroring {@link Backtester#run}'s defaults.
     */
    public void step(double open, double high, double low, double close) {
        step(open, high, low, close, 0.0, bars);
    }

    /**
     * Advance by one bar described as a request document:
     * {@code {"candle": {...}, "feeds": {...}}}, where {@code feeds} optionally
     * carries this bar's reference, derivatives, order-book, trade or
     * cross-section input. This is the only form that can drive a strategy
     * reading a side feed.
     *
     * @throws IllegalStateException if the run is finished or the document is rejected
     */
    public void stepJson(String stepJson) {
        MemorySegment live = live();
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment pStep = arena.allocateFrom(stepJson);
            MemorySegment pErr = arena.allocate(ValueLayout.ADDRESS);
            int code = (int) STREAM_STEP_JSON.invokeExact(live, pStep, pErr);
            if (code != 0) {
                throw new IllegalStateException(
                        "wickra_backtest_stream_step_json failed (code " + code + "): "
                                + take(pErr.get(ValueLayout.ADDRESS, 0)));
            }
            bars++;
        } catch (RuntimeException e) {
            throw e;
        } catch (Throwable t) {
            throw new RuntimeException("wickra_backtest_stream_step_json failed", t);
        }
    }

    /** The equity curve so far, as a JSON array. */
    public String equityJson() {
        return read(STREAM_EQUITY_JSON, "wickra_backtest_stream_equity_json");
    }

    /**
     * The most recent equity point as JSON, or the JSON literal {@code null}
     * before the first bar.
     */
    public String latestEquityJson() {
        return read(STREAM_LATEST_EQUITY_JSON, "wickra_backtest_stream_latest_equity_json");
    }

    /** The number of closed trades so far. */
    public long numTrades() {
        MemorySegment live = live();
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment pCount = arena.allocate(ValueLayout.JAVA_LONG);
            int code = (int) STREAM_NUM_TRADES.invokeExact(live, pCount);
            if (code != 0) {
                throw new IllegalStateException(
                        "wickra_backtest_stream_num_trades failed (code " + code + ")");
            }
            return pCount.get(ValueLayout.JAVA_LONG, 0);
        } catch (RuntimeException e) {
            throw e;
        } catch (Throwable t) {
            throw new RuntimeException("wickra_backtest_stream_num_trades failed", t);
        }
    }

    /**
     * Close any open position and return the report as JSON. Ends the run: the
     * handle is released and further use throws.
     *
     * @throws IllegalStateException if the run is already finished
     */
    public String finishJson() {
        MemorySegment live = live();
        handle = null;
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment pOut = arena.allocate(ValueLayout.ADDRESS);
            int code = (int) STREAM_FINISH_JSON.invokeExact(live, pOut);
            MemorySegment outPtr = pOut.get(ValueLayout.ADDRESS, 0);
            if (MemorySegment.NULL.equals(outPtr)) {
                throw new IllegalStateException(
                        "wickra_backtest_stream_finish_json returned code " + code
                                + " with no message");
            }
            String json = take(outPtr);
            if (code != 0) {
                throw new IllegalStateException(
                        "wickra_backtest_stream_finish_json failed (code " + code + "): " + json);
            }
            return json;
        } catch (RuntimeException e) {
            throw e;
        } catch (Throwable t) {
            throw new RuntimeException("wickra_backtest_stream_finish_json failed", t);
        }
    }

    /** Release the run without producing a report. Idempotent. */
    @Override
    public void close() {
        if (handle == null) {
            return;
        }
        MemorySegment live = handle;
        handle = null;
        try {
            STREAM_FREE.invokeExact(live);
        } catch (Throwable t) {
            throw new RuntimeException("wickra_backtest_stream_free failed", t);
        }
    }

    /** The live handle, or an error naming the mistake if the run is over. */
    private MemorySegment live() {
        if (handle == null) {
            throw new IllegalStateException("this backtest is finished");
        }
        return handle;
    }

    /** Invoke a read-only accessor that writes a JSON string to an out pointer. */
    private String read(MethodHandle accessor, String name) {
        MemorySegment live = live();
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment pOut = arena.allocate(ValueLayout.ADDRESS);
            int code = (int) accessor.invokeExact(live, pOut);
            MemorySegment outPtr = pOut.get(ValueLayout.ADDRESS, 0);
            if (MemorySegment.NULL.equals(outPtr)) {
                throw new IllegalStateException(
                        name + " returned code " + code + " with no message");
            }
            String json = take(outPtr);
            if (code != 0) {
                throw new IllegalStateException(name + " failed (code " + code + "): " + json);
            }
            return json;
        } catch (RuntimeException e) {
            throw e;
        } catch (Throwable t) {
            throw new RuntimeException(name + " failed", t);
        }
    }

    /**
     * Copy a native string into Java and free it. A null pointer means the call
     * left no message, which reads as the empty string.
     */
    private static String take(MemorySegment ptr) throws Throwable {
        if (MemorySegment.NULL.equals(ptr)) {
            return "";
        }
        String value = ptr.reinterpret(Long.MAX_VALUE).getString(0);
        Backtester.FREE.invokeExact(ptr);
        return value;
    }
}
