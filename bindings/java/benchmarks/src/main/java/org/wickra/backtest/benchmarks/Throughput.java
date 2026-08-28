package org.wickra.backtest.benchmarks;

import java.util.Arrays;
import java.util.Locale;
import org.wickra.backtest.Backtester;
import org.wickra.backtest.StreamingBacktest;

/**
 * Throughput benchmark for the wickra-backtest Java binding.
 *
 * <p>Measures what crossing the boundary costs. Every reach in this repository
 * runs the same Rust engine, so a difference between two bindings is not a
 * difference in the backtester -- it is the price of that language's FFI, paid
 * once per bar on the streaming path and once per run on the batch path. That
 * is the number worth knowing before choosing where to drive a live loop from.
 *
 * <p>The strategy is examples/ema-cross.json: two EMAs, a crossover, fractional
 * sizing, taker costs, slippage and a trailing stop. A realistic bar rather than
 * an empty one, so the figure includes the engine work a real strategy does.
 *
 * <p>Build the C ABI in release and install the binding, then:
 *
 * <pre>
 *   cargo build -p wickra-backtest-c --release
 *   mvn -f bindings/java install -DskipTests
 *   mvn -f bindings/java/benchmarks compile exec:exec
 *   mvn -f bindings/java/benchmarks compile exec:exec -Dbars=1000000
 * </pre>
 */
public final class Throughput {

    private static final String SPEC =
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

    private static final double CAPITAL = 10_000.0;
    private static final int REPS = 3;

    private Throughput() {
    }

    public static void main(String[] args) {
        int bars = 200_000;
        if (args.length > 0 && !args[0].isBlank()) {
            bars = Integer.parseInt(args[0].trim());
            if (bars < 1000) {
                throw new IllegalArgumentException("bars must be at least 1000");
            }
        }

        // The deterministic synthetic OHLCV every binding's harness builds. No
        // RNG, so two runs are comparable and so are two languages.
        double[] open = new double[bars];
        double[] high = new double[bars];
        double[] low = new double[bars];
        double[] close = new double[bars];
        double[] volume = new double[bars];
        long[] time = new long[bars];
        for (int i = 0; i < bars; i++) {
            double mid = 100.0 + Math.sin(i * 0.001) * 20.0 + i * 1e-4;
            close[i] = mid + Math.sin(i * 0.05) * 2.0;
            open[i] = i > 0 ? close[i - 1] : close[i];
            high[i] = Math.max(open[i], close[i]) + 1.5;
            low[i] = Math.min(open[i], close[i]) - 1.5;
            volume[i] = 1000.0 + (i % 97) * 13;
            time[i] = i;
        }

        final int n = bars;
        double streaming = medianSeconds(() -> {
            try (StreamingBacktest live = new StreamingBacktest(SPEC, CAPITAL)) {
                for (int i = 0; i < n; i++) {
                    live.step(open[i], high[i], low[i], close[i], volume[i], time[i]);
                }
                live.finishJson();
            }
        });
        double batch = medianSeconds(
                () -> Backtester.run(open, high, low, close, volume, time, SPEC, CAPITAL));

        // Locale.ROOT so the separators are the same wherever this is run; the
        // default locale would make two machines' reports differ in formatting.
        System.out.printf(Locale.ROOT,
                "wickra-backtest Java throughput — %,d bars (median of %d runs)%n%n", bars, REPS);
        System.out.printf(Locale.ROOT, "%-14s%16s%12s%n", "path", "bars/sec", "ns/bar");
        System.out.println("-".repeat(42));
        printRow("streaming", streaming, bars);
        printRow("batch", batch, bars);
        System.out.println("""

                Streaming crosses the boundary once per bar, with scalars; batch crosses it
                once per run and hands over six arrays. The FFM API charges per downcall, so
                which of the two wins is a property of the language, not of the engine.
                Machine-dependent — compare bindings on one machine, not across machines.""");
    }

    private static void printRow(String name, double seconds, int bars) {
        System.out.printf(Locale.ROOT, "%-14s%,16.0f%,12.0f%n",
                name, bars / seconds, seconds / bars * 1e9);
    }

    /**
     * Median wall-clock seconds over {@value #REPS} runs, after one warmup pass
     * that also pays the JIT so it is not charged to the first measurement.
     */
    private static double medianSeconds(Runnable pass) {
        pass.run();
        double[] samples = new double[REPS];
        for (int r = 0; r < REPS; r++) {
            long start = System.nanoTime();
            pass.run();
            samples[r] = (System.nanoTime() - start) / 1e9;
        }
        Arrays.sort(samples);
        return samples[REPS / 2];
    }
}
