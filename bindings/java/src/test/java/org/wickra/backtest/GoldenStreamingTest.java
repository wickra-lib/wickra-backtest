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

// Driving each shared case one bar at a time must reproduce the same canonical
// report (golden/expected/) the batch entry point produces. GoldenParityTest
// pins the batch side; this pins that streaming did not drift away from it.
class GoldenStreamingTest {

    @Test
    void streamingGoldenParity() throws IOException {
        Path golden = Paths.get("..", "..", "golden");
        int n = 0;
        try (Stream<Path> files = Files.list(golden.resolve("cases"))) {
            for (Path p : (Iterable<Path>) files
                    .filter(x -> x.toString().endsWith(".json"))::iterator) {
                JSONObject c = new JSONObject(Files.readString(p));
                String name = c.getString("name");
                JSONArray open = c.getJSONArray("open");
                JSONArray high = c.getJSONArray("high");
                JSONArray low = c.getJSONArray("low");
                JSONArray close = c.getJSONArray("close");
                JSONArray volume = c.getJSONArray("volume");
                JSONArray time = c.getJSONArray("time");

                String got;
                try (StreamingBacktest bt = new StreamingBacktest(
                        c.getJSONObject("spec").toString(), c.getDouble("capital"))) {
                    for (int i = 0; i < close.length(); i++) {
                        bt.step(open.getDouble(i), high.getDouble(i), low.getDouble(i),
                                close.getDouble(i), volume.getDouble(i), time.getLong(i));
                    }
                    got = bt.finishJson();
                }

                String want = Files.readString(
                        golden.resolve("expected").resolve(name + ".json")).trim();
                assertEquals(want, got, "streaming mismatch for " + name);
                n++;
            }
        }
        assertTrue(n > 0, "no golden cases found");
    }
}
