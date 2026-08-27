/* Streaming example for the wickra-backtest C ABI.
 *
 * `wickra_backtest_run` answers a whole series at once. The streaming handle
 * drives the same engine one bar at a time, which is what makes "backtest and
 * live are one code path" more than a slogan: replace the loop below with reads
 * from a socket and nothing else changes.
 *
 * This example is also the cross-language proof of that claim from a foreign C
 * consumer: it runs the same bars both ways and fails if the two reports differ.
 *
 * Build (after `cargo build -p wickra-backtest-c`), e.g. with gcc:
 *   gcc streaming.c -I ../../bindings/c/include \
 *       -L ../../target/debug -lwickra_backtest -o streaming
 * then run with the shared library on the loader path. On Windows link the DLL
 * by name instead: -l:wickra_backtest.dll. The same source compiles as C++
 * (g++ -x c++ ...) -- the header is extern "C".
 */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "wickra_backtest.h"

static const char *const SPEC =
    "{\"symbol\":\"x\",\"timeframe\":\"1h\",\"indicators\":{},"
    "\"entry\":{\"gt\":[{\"price\":\"close\"},100]},"
    "\"exit\":{\"lt\":[{\"price\":\"close\"},100]},"
    "\"sizing\":{\"type\":\"fixed_qty\",\"qty\":1}}";

#define BARS 4
static const double OPEN[BARS]   = {100, 102, 104, 98};
static const double HIGH[BARS]   = {101, 103, 104, 98};
static const double LOW[BARS]    = {100, 102, 99, 97};
static const double CLOSE[BARS]  = {101, 103, 99, 97};
static const double VOLUME[BARS] = {0, 0, 0, 0};
static const int64_t TIME[BARS]  = {0, 1, 2, 3};

/* Report the whole series at once, for comparison with the streamed run. */
static char *run_batch(void) {
    char *out = NULL;
    int code = wickra_backtest_run(OPEN, HIGH, LOW, CLOSE, VOLUME, TIME, BARS,
                                   SPEC, 1000.0, &out);
    if (code != WICKRA_BT_OK) {
        fprintf(stderr, "batch run failed (%d): %s\n", code, out ? out : "(null)");
        wickra_backtest_free_string(out);
        return NULL;
    }
    return out;
}

int main(void) {
    WickraBacktestStream *bt = NULL;
    char *err = NULL;
    if (wickra_backtest_stream_new(SPEC, 1000.0, &bt, &err) != WICKRA_BT_OK) {
        fprintf(stderr, "could not start the run: %s\n", err ? err : "(null)");
        wickra_backtest_free_string(err);
        return 1;
    }

    for (int i = 0; i < BARS; i++) {
        if (wickra_backtest_stream_step(bt, OPEN[i], HIGH[i], LOW[i], CLOSE[i],
                                        VOLUME[i], TIME[i], &err) != WICKRA_BT_OK) {
            fprintf(stderr, "bar %d rejected: %s\n", i, err ? err : "(null)");
            wickra_backtest_free_string(err);
            wickra_backtest_stream_free(bt);
            return 1;
        }

        /* Everything a live loop wants is readable between bars. */
        size_t trades = 0;
        char *equity = NULL;
        if (wickra_backtest_stream_num_trades(bt, &trades) == WICKRA_BT_OK
                && wickra_backtest_stream_latest_equity_json(bt, &equity) == WICKRA_BT_OK) {
            printf("bar %d: %zu closed trades, equity %s\n", i, trades, equity);
        }
        wickra_backtest_free_string(equity);
    }

    /* finish consumes the handle: it must not be freed again below. */
    char *streamed = NULL;
    int code = wickra_backtest_stream_finish_json(bt, &streamed);
    if (code != WICKRA_BT_OK) {
        fprintf(stderr, "could not finish the run (%d): %s\n",
                code, streamed ? streamed : "(null)");
        wickra_backtest_free_string(streamed);
        return 1;
    }

    char *batch = run_batch();
    if (batch == NULL) {
        wickra_backtest_free_string(streamed);
        return 1;
    }

    /* The claim worth checking from outside Rust: same bars, same report. */
    int differ = strcmp(streamed, batch) != 0;
    if (differ) {
        fprintf(stderr, "streamed and batch reports differ:\n  %s\n  %s\n",
                streamed, batch);
    } else {
        printf("%s\n", streamed);
        printf("streaming matches the batch report exactly\n");
    }

    wickra_backtest_free_string(streamed);
    wickra_backtest_free_string(batch);
    return differ ? 1 : 0;
}
