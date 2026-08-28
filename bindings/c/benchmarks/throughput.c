/* Throughput benchmark for the wickra-backtest C ABI.
 *
 * Measures what crossing the boundary costs. Every reach in this repository
 * runs the same Rust engine, so a difference between two bindings is not a
 * difference in the backtester -- it is the price of that language's FFI.
 *
 * This one is the floor. C calls the exported functions directly, with no
 * marshalling layer of its own, so whatever it reports is the engine plus the
 * bare cost of a function call. Every other binding here pays that and its own
 * overhead on top, which is what makes this the useful baseline to read the
 * others against.
 *
 * The strategy is examples/ema-cross.json: two EMAs, a crossover, fractional
 * sizing, taker costs, slippage and a trailing stop.
 *
 * Build and run (after `cargo build -p wickra-backtest-c --release`):
 *
 *   cmake -S bindings/c/benchmarks -B bindings/c/benchmarks/build
 *   cmake --build bindings/c/benchmarks/build --config Release
 *   ./bindings/c/benchmarks/build/throughput            # 200000 bars
 *   ./bindings/c/benchmarks/build/throughput 1000000
 */
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include "wickra_backtest.h"

static const char *const SPEC =
    "{\"symbol\":\"BTCUSDT\",\"timeframe\":\"1h\","
    "\"indicators\":{\"ema_fast\":{\"type\":\"Ema\",\"params\":[5]},"
    "\"ema_slow\":{\"type\":\"Ema\",\"params\":[15]}},"
    "\"entry\":{\"cross_above\":[\"ema_fast\",\"ema_slow\"]},"
    "\"exit\":{\"cross_below\":[\"ema_fast\",\"ema_slow\"]},"
    "\"sizing\":{\"type\":\"fixed_fraction\",\"fraction\":0.95},"
    "\"costs\":{\"taker_bps\":5,\"slippage\":{\"type\":\"fixed_bps\",\"bps\":2}},"
    "\"risk\":{\"trailing_stop_pct\":5.0}}";

static const double CAPITAL = 10000.0;
static const int REPS = 3;

struct series {
    double *open, *high, *low, *close, *volume;
    int64_t *time;
    size_t n;
};

/* The deterministic synthetic OHLCV every binding's harness builds. No RNG, so
 * two runs are comparable and so are two languages. */
static int series_init(struct series *s, size_t n) {
    s->n = n;
    s->open = malloc(n * sizeof *s->open);
    s->high = malloc(n * sizeof *s->high);
    s->low = malloc(n * sizeof *s->low);
    s->close = malloc(n * sizeof *s->close);
    s->volume = malloc(n * sizeof *s->volume);
    s->time = malloc(n * sizeof *s->time);
    if (!s->open || !s->high || !s->low || !s->close || !s->volume || !s->time) {
        return 0;
    }
    for (size_t i = 0; i < n; i++) {
        double mid = 100.0 + sin((double)i * 0.001) * 20.0 + (double)i * 1e-4;
        s->close[i] = mid + sin((double)i * 0.05) * 2.0;
        s->open[i] = i ? s->close[i - 1] : s->close[i];
        s->high[i] = (s->open[i] > s->close[i] ? s->open[i] : s->close[i]) + 1.5;
        s->low[i] = (s->open[i] < s->close[i] ? s->open[i] : s->close[i]) - 1.5;
        s->volume[i] = 1000.0 + (double)(i % 97) * 13.0;
        s->time[i] = (int64_t)i;
    }
    return 1;
}

static void series_free(struct series *s) {
    free(s->open); free(s->high); free(s->low);
    free(s->close); free(s->volume); free(s->time);
}

static double now_seconds(void) {
    struct timespec ts;
    timespec_get(&ts, TIME_UTC);
    return (double)ts.tv_sec + (double)ts.tv_nsec / 1e9;
}

static int compare_double(const void *a, const void *b) {
    double x = *(const double *)a, y = *(const double *)b;
    return (x > y) - (x < y);
}

/* One streamed pass: new, step every bar, finish. Returns 0 on failure. */
static int run_streaming(const struct series *s) {
    WickraBacktestStream *bt = NULL;
    char *err = NULL;
    if (wickra_backtest_stream_new(SPEC, CAPITAL, &bt, &err) != WICKRA_BT_OK) {
        fprintf(stderr, "could not start the run: %s\n", err ? err : "(null)");
        wickra_backtest_free_string(err);
        return 0;
    }
    for (size_t i = 0; i < s->n; i++) {
        if (wickra_backtest_stream_step(bt, s->open[i], s->high[i], s->low[i],
                                        s->close[i], s->volume[i], s->time[i],
                                        &err) != WICKRA_BT_OK) {
            fprintf(stderr, "bar %zu rejected: %s\n", i, err ? err : "(null)");
            wickra_backtest_free_string(err);
            wickra_backtest_stream_free(bt);
            return 0;
        }
    }
    char *report = NULL;
    /* finish consumes the handle whatever the outcome. */
    int code = wickra_backtest_stream_finish_json(bt, &report);
    if (code != WICKRA_BT_OK) {
        fprintf(stderr, "could not finish the run: %s\n", report ? report : "(null)");
        wickra_backtest_free_string(report);
        return 0;
    }
    wickra_backtest_free_string(report);
    return 1;
}

/* One batch pass over the whole series. Returns 0 on failure. */
static int run_batch(const struct series *s) {
    char *out = NULL;
    int code = wickra_backtest_run(s->open, s->high, s->low, s->close, s->volume,
                                   s->time, s->n, SPEC, CAPITAL, &out);
    if (code != WICKRA_BT_OK) {
        fprintf(stderr, "batch run failed: %s\n", out ? out : "(null)");
        wickra_backtest_free_string(out);
        return 0;
    }
    wickra_backtest_free_string(out);
    return 1;
}

/* Median seconds over REPS runs, after one warmup. Returns -1 on failure. */
static double median_seconds(int (*pass)(const struct series *), const struct series *s) {
    if (!pass(s)) {
        return -1.0;
    }
    double samples[16];
    for (int r = 0; r < REPS; r++) {
        double start = now_seconds();
        if (!pass(s)) {
            return -1.0;
        }
        samples[r] = now_seconds() - start;
    }
    qsort(samples, (size_t)REPS, sizeof samples[0], compare_double);
    return samples[REPS / 2];
}

static void print_row(const char *name, double seconds, size_t bars) {
    printf("%-14s%16.0f%12.0f\n", name, (double)bars / seconds,
           seconds / (double)bars * 1e9);
}

int main(int argc, char **argv) {
    long bars = 200000;
    if (argc > 1) {
        bars = strtol(argv[1], NULL, 10);
        if (bars < 1000) {
            fprintf(stderr, "usage: throughput [bars >= 1000]\n");
            return 1;
        }
    }

    struct series s;
    if (!series_init(&s, (size_t)bars)) {
        fprintf(stderr, "out of memory for %ld bars\n", bars);
        series_free(&s);
        return 1;
    }

    double streaming = median_seconds(run_streaming, &s);
    double batch = streaming < 0 ? -1.0 : median_seconds(run_batch, &s);
    if (streaming < 0 || batch < 0) {
        series_free(&s);
        return 1;
    }

    printf("wickra-backtest C throughput - %ld bars (median of %d runs)\n\n", bars, REPS);
    printf("%-14s%16s%12s\n", "path", "bars/sec", "ns/bar");
    printf("------------------------------------------\n");
    print_row("streaming", streaming, s.n);
    print_row("batch", batch, s.n);
    printf("\nThis is the floor: C calls the exports directly, so the numbers are the\n"
           "engine plus a function call. Every other binding pays this plus its own\n"
           "marshalling, which is what makes this the baseline to read them against.\n"
           "Machine-dependent - compare bindings on one machine, not across machines.\n");

    series_free(&s);
    return 0;
}
