package org.wickra.backtest;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.stream.Stream;
import org.json.JSONArray;
import org.json.JSONObject;
import org.junit.jupiter.api.Test;

// The Java binding asserts its output against the shared golden reports
// (golden/expected/), pinning cross-language equality. It returns the engine
// JSON verbatim, so the match is byte-for-byte.
class GoldenParityTest {

    private static double[] doubles(JSONObject o, String key) {
        JSONArray a = o.getJSONArray(key);
        double[] r = new double[a.length()];
        for (int i = 0; i < r.length; i++) {
            r[i] = a.getDouble(i);
        }
        return r;
    }

    private static long[] longs(JSONObject o, String key) {
        JSONArray a = o.getJSONArray(key);
        long[] r = new long[a.length()];
        for (int i = 0; i < r.length; i++) {
            r[i] = a.getLong(i);
        }
        return r;
    }

    @Test
    void goldenParity() throws IOException {
        Path golden = Paths.get("..", "..", "golden");
        int n = 0;
        try (Stream<Path> files = Files.list(golden.resolve("cases"))) {
            for (Path p : (Iterable<Path>) files
                    .filter(x -> x.toString().endsWith(".json"))::iterator) {
                JSONObject c = new JSONObject(Files.readString(p));
                String name = c.getString("name");
                String got = Backtester.run(
                        doubles(c, "open"), doubles(c, "high"), doubles(c, "low"),
                        doubles(c, "close"), doubles(c, "volume"), longs(c, "time"),
                        c.getJSONObject("spec").toString(), c.getDouble("capital"));
                String want = Files
                        .readString(golden.resolve("expected").resolve(name + ".json"))
                        .trim();
                assertEquals(want, got, "golden mismatch for " + name);
                n++;
            }
        }
        assertTrue(n > 0, "no golden cases found");
    }
}
