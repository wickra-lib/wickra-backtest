package org.wickra.backtest.examples;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.List;
import org.json.JSONObject;
import org.wickra.backtest.Backtester;
import org.wickra.backtest.StreamingBacktest;

/**
 * Run the shared EMA-cross strategy from Java, both ways.
 *
 * <pre>
 *   cargo build -p wickra-backtest-c
 *   mvn -f bindings/java install
 *   mvn -f examples/java compile exec:exec
 * </pre>
 *
 * <p>Reads the same {@code examples/sample.csv} and {@code examples/ema-cross.json}
 * every other language example uses, runs the whole series at once, then feeds
 * the same bars one at a time and checks that the two agree. That equality is the
 * point of the library: a live loop is the streaming path with a socket in place
 * of the file, so a backtest is not a separate model of the strategy.
 */
public final class Backtest {

    private static final double CAPITAL = 10_000.0;

    private Backtest() {
    }

    public static void main(String[] args) throws IOException {
        Path dir = Paths.get("..").toAbsolutePath().normalize();
        List<String> rows = Files.readAllLines(dir.resolve("sample.csv"));
        String spec = Files.readString(dir.resolve("ema-cross.json"));

        // The CSV columns are time,open,high,low,close,volume.
        int n = rows.size() - 1;
        long[] time = new long[n];
        double[] open = new double[n];
        double[] high = new double[n];
        double[] low = new double[n];
        double[] close = new double[n];
        double[] volume = new double[n];
        for (int i = 0; i < n; i++) {
            String[] f = rows.get(i + 1).split(",");
            time[i] = (long) Double.parseDouble(f[0]);
            open[i] = Double.parseDouble(f[1]);
            high[i] = Double.parseDouble(f[2]);
            low[i] = Double.parseDouble(f[3]);
            close[i] = Double.parseDouble(f[4]);
            volume[i] = Double.parseDouble(f[5]);
        }

        String batch = Backtester.run(open, high, low, close, volume, time, spec, CAPITAL);

        // The same run, driven bar by bar. Replace the loop with reads from a
        // socket and this is a live strategy; nothing else about it changes.
        String streamed;
        try (StreamingBacktest live = new StreamingBacktest(spec, CAPITAL)) {
            for (int i = 0; i < n; i++) {
                live.step(open[i], high[i], low[i], close[i], volume[i], time[i]);
            }
            streamed = live.finishJson();
        }

        JSONObject report = new JSONObject(streamed);
        JSONObject metrics = report.getJSONObject("metrics");
        System.out.printf("bars            %d%n", n);
        System.out.printf("trades          %d%n", metrics.getInt("num_trades"));
        System.out.printf("pnl             %.2f%n", metrics.getDouble("pnl"));
        System.out.printf("return %%        %.2f%n", metrics.getDouble("return_pct"));
        System.out.printf("max drawdown    %.4f%n", metrics.getDouble("max_drawdown"));

        if (!streamed.equals(batch)) {
            System.err.println("streaming and batch disagree -- that should be impossible");
            System.exit(1);
        }
        System.out.println();
        System.out.println("streaming reproduces the batch report exactly");
    }
}
