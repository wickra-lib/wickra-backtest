// C++ example for the wickra-backtest C ABI, through the optional RAII wrapper
// (`wickra_backtest.hpp`).
//
// This is the same streaming run as `streaming.c` -- same spec, same bars, same
// comparison against the batch entry point -- written with `wickra::backtest::
// Stream` and `wickra::backtest::String` instead of hand-placed frees. It is
// therefore two proofs at once: that the header compiles and links as C++, and
// that the wrapper's ownership rules are the ABI's, including the one that
// hand-written code gets wrong -- `finish` consumes the handle, so the
// destructor must not free it afterwards.
//
// Build (after `cargo build -p wickra-backtest-c`), e.g. with g++:
//   g++ -std=c++17 cpp_smoke.cpp -I ../../bindings/c/include \
//       -L ../../target/debug -lwickra_backtest -o cpp_smoke
// then run with the shared library on the loader path.

#include "wickra_backtest.hpp"

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <utility>

namespace bt = wickra::backtest;

namespace {

const char *const SPEC =
    "{\"symbol\":\"x\",\"timeframe\":\"1h\",\"indicators\":{},"
    "\"entry\":{\"gt\":[{\"price\":\"close\"},100]},"
    "\"exit\":{\"lt\":[{\"price\":\"close\"},100]},"
    "\"sizing\":{\"type\":\"fixed_qty\",\"qty\":1}}";

constexpr int BARS = 4;
const double OPEN[BARS] = {100, 102, 104, 98};
const double HIGH[BARS] = {101, 103, 104, 98};
const double LOW[BARS] = {100, 102, 99, 97};
const double CLOSE[BARS] = {101, 103, 99, 97};
const double VOLUME[BARS] = {0, 0, 0, 0};
const std::int64_t TIME[BARS] = {0, 1, 2, 3};

// The whole series at once, for comparison with the streamed run. On failure the
// same out-parameter carries the message instead of the report, which is why one
// `String` covers both.
bool run_batch(bt::String &out) {
    int code = wickra_backtest_run(OPEN, HIGH, LOW, CLOSE, VOLUME, TIME, BARS,
                                   SPEC, 1000.0, out.out());
    if (code != WICKRA_BT_OK) {
        std::fprintf(stderr, "FAIL: batch run (%d): %s\n", code, out.c_str());
        return false;
    }
    return true;
}

// Ownership is what this wrapper exists for, so check it rather than assume it:
// a moved-from or released owner must be empty, or the destructor frees a
// resource that has already gone.
bool ownership_transfers_are_empty() {
    bt::Stream stream;
    bt::String err;
    if (wickra_backtest_stream_new(SPEC, 1000.0, stream.out(), err.out()) != WICKRA_BT_OK) {
        std::fprintf(stderr, "FAIL: could not start a run: %s\n", err.c_str());
        return false;
    }

    bt::Stream moved(std::move(stream));
    if (static_cast<bool>(stream) || !static_cast<bool>(moved)) {
        std::fputs("FAIL: moving a Stream left the source non-empty\n", stderr);
        return false;
    }

    WickraBacktestStream *raw = moved.release();
    if (static_cast<bool>(moved) || raw == nullptr) {
        std::fputs("FAIL: releasing a Stream left the source non-empty\n", stderr);
        return false;
    }
    wickra_backtest_stream_free(raw);

    bt::String held;
    if (!run_batch(held) || !held) {
        return false;
    }
    bt::String moved_string(std::move(held));
    if (static_cast<bool>(held) || !static_cast<bool>(moved_string)) {
        std::fputs("FAIL: moving a String left the source non-empty\n", stderr);
        return false;
    }
    return true;
}

}  // namespace

int main() {
    if (!ownership_transfers_are_empty()) {
        return 1;
    }

    bt::Stream stream;
    bt::String err;
    if (wickra_backtest_stream_new(SPEC, 1000.0, stream.out(), err.out()) != WICKRA_BT_OK) {
        std::fprintf(stderr, "FAIL: could not start the run: %s\n", err.c_str());
        return 1;
    }

    // One `String` for the whole loop: `out()` releases the previous bar's
    // equity before the next call writes into it.
    bt::String equity;
    for (int i = 0; i < BARS; i++) {
        if (wickra_backtest_stream_step(stream.get(), OPEN[i], HIGH[i], LOW[i], CLOSE[i],
                                        VOLUME[i], TIME[i], err.out()) != WICKRA_BT_OK) {
            std::fprintf(stderr, "FAIL: bar %d rejected: %s\n", i, err.c_str());
            return 1;
        }

        std::size_t trades = 0;
        if (wickra_backtest_stream_num_trades(stream.get(), &trades) == WICKRA_BT_OK
                && wickra_backtest_stream_latest_equity_json(stream.get(), equity.out())
                       == WICKRA_BT_OK) {
            std::printf("bar %d: %zu closed trades, equity %s\n", i, trades, equity.c_str());
        }
    }

    // finish consumes the handle: release() hands ownership over, so ~Stream
    // does not free what the ABI has already taken.
    bt::String streamed;
    int code = wickra_backtest_stream_finish_json(stream.release(), streamed.out());
    if (code != WICKRA_BT_OK) {
        std::fprintf(stderr, "FAIL: could not finish the run (%d): %s\n", code, streamed.c_str());
        return 1;
    }

    bt::String batch;
    if (!run_batch(batch)) {
        return 1;
    }

    // The claim worth checking from a C++ consumer too: same bars, same report.
    if (std::strcmp(streamed.c_str(), batch.c_str()) != 0) {
        std::fprintf(stderr, "FAIL: streamed and batch reports differ:\n  %s\n  %s\n",
                     streamed.c_str(), batch.c_str());
        return 1;
    }

    std::printf("%s\n", streamed.c_str());
    std::puts("OK: C++ RAII wrapper matches the batch report exactly");
    return 0;
}
