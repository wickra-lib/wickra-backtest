package org.wickra.backtest;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.json.JSONArray;
import org.json.JSONObject;
import org.junit.jupiter.api.Test;

class BacktesterTests {

    private static final String PRICE_SPEC =
            "{\"symbol\":\"x\",\"timeframe\":\"1h\",\"indicators\":{},"
            + "\"entry\":{\"gt\":[{\"price\":\"close\"},100]},"
            + "\"exit\":{\"lt\":[{\"price\":\"close\"},100]},"
            + "\"sizing\":{\"type\":\"fixed_qty\",\"qty\":1}}";

    @Test
    void versionIsNonEmpty() {
        assertFalse(Backtester.version().isEmpty());
    }

    @Test
    void handComputedRoundTripMatchesEngine() {
        double[] open = {100.0, 102.0, 104.0, 98.0};
        double[] high = {101.0, 103.0, 104.0, 98.0};
        double[] low = {100.0, 102.0, 99.0, 97.0};
        double[] close = {101.0, 103.0, 99.0, 97.0};
        long[] time = {0, 1, 2, 3};

        String json = Backtester.run(open, high, low, close, null, time, PRICE_SPEC, 1000.0);
        JSONObject root = new JSONObject(json);

        assertEquals(1, root.getJSONObject("metrics").getInt("num_trades"));
        JSONObject trade = root.getJSONArray("trades").getJSONObject(0);
        assertEquals(102.0, trade.getDouble("entry_price"), 1e-9);
        assertEquals(98.0, trade.getDouble("exit_price"), 1e-9);
        assertEquals(-4.0, trade.getDouble("pnl"), 1e-9);

        JSONArray equity = root.getJSONArray("equity");
        JSONObject last = equity.getJSONObject(equity.length() - 1);
        assertEquals(996.0, last.getDouble("equity"), 1e-9);
    }

    @Test
    void invalidSpecThrows() {
        double[] one = {1.0};
        assertThrows(IllegalStateException.class, () ->
                Backtester.run(one, one, one, one, "{\"bad\":true}"));
    }
}
