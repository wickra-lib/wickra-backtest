package org.wickra.backtest;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.json.JSONArray;
import org.json.JSONObject;
import org.junit.jupiter.api.Test;

/**
 * The streaming class must be the same engine as {@link Backtester#run}, one bar
 * at a time -- that equivalence is the claim, so it is what these tests pin.
 */
class StreamingBacktestTests {

    private static final String PRICE_SPEC =
            "{\"symbol\":\"x\",\"timeframe\":\"1h\",\"indicators\":{},"
            + "\"entry\":{\"gt\":[{\"price\":\"close\"},100]},"
            + "\"exit\":{\"lt\":[{\"price\":\"close\"},100]},"
            + "\"sizing\":{\"type\":\"fixed_qty\",\"qty\":1}}";

    private static final double[] OPEN = {100.0, 102.0, 104.0, 98.0};
    private static final double[] HIGH = {101.0, 103.0, 104.0, 98.0};
    private static final double[] LOW = {100.0, 102.0, 99.0, 97.0};
    private static final double[] CLOSE = {101.0, 103.0, 99.0, 97.0};

    private static String batchReport() {
        return Backtester.run(OPEN, HIGH, LOW, CLOSE, PRICE_SPEC, 1000.0);
    }

    @Test
    void streamingReproducesTheBatchReport() {
        try (StreamingBacktest bt = new StreamingBacktest(PRICE_SPEC, 1000.0)) {
            for (int i = 0; i < OPEN.length; i++) {
                bt.step(OPEN[i], HIGH[i], LOW[i], CLOSE[i]);
            }
            assertEquals(batchReport(), bt.finishJson());
        }
    }

    @Test
    void stepJsonMatchesTheScalarStep() {
        try (StreamingBacktest bt = new StreamingBacktest(PRICE_SPEC, 1000.0)) {
            for (int i = 0; i < OPEN.length; i++) {
                bt.stepJson("{\"candle\":{\"time\":" + i
                        + ",\"open\":" + OPEN[i] + ",\"high\":" + HIGH[i]
                        + ",\"low\":" + LOW[i] + ",\"close\":" + CLOSE[i]
                        + ",\"volume\":0}}");
            }
            assertEquals(batchReport(), bt.finishJson());
        }
    }

    @Test
    void accessorsTrackTheRun() {
        try (StreamingBacktest bt = new StreamingBacktest(PRICE_SPEC, 1000.0)) {
            assertEquals("null", bt.latestEquityJson());
            assertEquals(0, new JSONArray(bt.equityJson()).length());
            assertEquals(0L, bt.numTrades());
            assertFalse(bt.isFinished());

            for (int i = 0; i < 3; i++) {
                bt.step(OPEN[i], HIGH[i], LOW[i], CLOSE[i]);
            }

            JSONArray curve = new JSONArray(bt.equityJson());
            assertEquals(3, curve.length());
            // Bar 2 closed below 100, which is the exit *signal*; the fill lands
            // on the next bar's open, so nothing has closed yet.
            assertEquals(0L, bt.numTrades());

            bt.step(OPEN[3], HIGH[3], LOW[3], CLOSE[3]);
            assertEquals(1L, bt.numTrades());
        }
    }

    @Test
    void stepDefaultsTheTimestampToTheBarIndex() {
        try (StreamingBacktest bt = new StreamingBacktest(PRICE_SPEC, 1000.0)) {
            for (int i = 0; i < OPEN.length; i++) {
                bt.step(OPEN[i], HIGH[i], LOW[i], CLOSE[i]);
            }
            JSONArray curve = new JSONArray(bt.equityJson());
            assertEquals(OPEN.length, curve.length());
            for (int i = 0; i < curve.length(); i++) {
                assertEquals(i, curve.getJSONObject(i).getLong("time"));
            }
        }
    }

    @Test
    void aFinishedRunRefusesFurtherUse() {
        StreamingBacktest bt = new StreamingBacktest(PRICE_SPEC, 1000.0);
        bt.step(OPEN[0], HIGH[0], LOW[0], CLOSE[0]);
        bt.finishJson();
        assertTrue(bt.isFinished());

        assertThrows(IllegalStateException.class, () -> bt.step(1.0, 1.0, 1.0, 1.0));
        assertThrows(IllegalStateException.class, () -> bt.stepJson("{}"));
        assertThrows(IllegalStateException.class, bt::equityJson);
        assertThrows(IllegalStateException.class, bt::latestEquityJson);
        assertThrows(IllegalStateException.class, bt::numTrades);
        assertThrows(IllegalStateException.class, bt::finishJson);
    }

    @Test
    void closeIsIdempotentAndComposesWithFinish() {
        StreamingBacktest bt = new StreamingBacktest(PRICE_SPEC, 1000.0);
        bt.step(OPEN[0], HIGH[0], LOW[0], CLOSE[0]);
        bt.close();
        bt.close();
        assertTrue(bt.isFinished());

        // finishJson already released the handle, so the try-with-resources
        // close that follows it must be a no-op rather than a double free.
        try (StreamingBacktest other = new StreamingBacktest(PRICE_SPEC, 1000.0)) {
            other.step(OPEN[0], HIGH[0], LOW[0], CLOSE[0]);
            other.finishJson();
        }
    }

    @Test
    void anInvalidSpecThrows() {
        assertThrows(IllegalStateException.class,
                () -> new StreamingBacktest("{\"bad\":true}", 1000.0));
    }

    /**
     * A pairwise indicator is undefined without its reference series, so a spec
     * that reads one proves the per-bar feed actually arrives -- and it must
     * agree with the batch path fed the same reference.
     */
    @Test
    void perBarFeedsReachAReferenceReadingStrategy() {
        // A sine path, not a geometric one: constant growth means constant log
        // returns, which drives the correlation's variance to zero.
        int n = 24;
        double[] closes = new double[n];
        for (int i = 0; i < n; i++) {
            closes[i] = 100.0 + 10.0 * Math.sin(i * 0.5);
        }
        String spec = "{\"symbol\":\"x\",\"timeframe\":\"1h\","
                + "\"indicators\":{\"corr\":{\"type\":\"PearsonCorrelation\",\"params\":[5]}},"
                + "\"entry\":{\"gt\":[\"corr\",0.5]},\"exit\":{\"lt\":[\"corr\",-0.5]},"
                + "\"sizing\":{\"type\":\"fixed_qty\",\"qty\":1}}";

        StringBuilder candles = new StringBuilder();
        StringBuilder reference = new StringBuilder();
        String streamed;
        try (StreamingBacktest bt = new StreamingBacktest(spec, 1000.0)) {
            for (int i = 0; i < n; i++) {
                double c = closes[i];
                double ref = 2.0 * c;
                String candle = "{\"time\":" + i + ",\"open\":" + c + ",\"high\":" + (c + 1.0)
                        + ",\"low\":" + (c - 1.0) + ",\"close\":" + c + ",\"volume\":0}";
                if (i > 0) {
                    candles.append(',');
                    reference.append(',');
                }
                candles.append(candle);
                reference.append("{\"time\":").append(i).append(",\"open\":").append(ref)
                        .append(",\"high\":").append(ref).append(",\"low\":").append(ref)
                        .append(",\"close\":").append(ref).append(",\"volume\":0}");
                bt.stepJson("{\"candle\":" + candle + ",\"feeds\":{\"reference\":" + ref + "}}");
            }
            streamed = bt.finishJson();
        }

        String batch = Backtester.runJson("{\"spec\":" + spec + ",\"capital\":1000,\"candles\":["
                + candles + "],\"reference\":[" + reference + "]}");
        assertEquals(batch, streamed);
        assertEquals(1, trades(streamed));

        // The feed is load-bearing: without it the correlation never resolves.
        String blind;
        try (StreamingBacktest bt = new StreamingBacktest(spec, 1000.0)) {
            for (int i = 0; i < n; i++) {
                double c = closes[i];
                bt.step(c, c + 1.0, c - 1.0, c);
            }
            blind = bt.finishJson();
        }
        assertEquals(0, trades(blind));
        assertNotEquals(streamed, blind);
    }

    private static int trades(String reportJson) {
        return new JSONObject(reportJson).getJSONObject("metrics").getInt("num_trades");
    }
}
