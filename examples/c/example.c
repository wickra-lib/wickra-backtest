/* Minimal C example for the wickra-backtest C ABI.
 *
 * Build (after `cargo build -p wickra-backtest-c`), e.g. with gcc:
 *   gcc example.c -I ../../bindings/c/include \
 *       -L ../../target/debug -lwickra_backtest -o example
 * then run with the shared library on the loader path.
 */
#include <stdint.h>
#include <stdio.h>
#include "wickra_backtest.h"

int main(void) {
    double open[]   = {100, 102, 104, 98};
    double high[]   = {101, 103, 104, 98};
    double low[]    = {100, 102, 99, 97};
    double close[]  = {101, 103, 99, 97};
    double volume[] = {0, 0, 0, 0};
    int64_t time[]  = {0, 1, 2, 3};

    const char *spec =
        "{\"symbol\":\"x\",\"timeframe\":\"1h\",\"indicators\":{},"
        "\"entry\":{\"gt\":[{\"price\":\"close\"},100]},"
        "\"exit\":{\"lt\":[{\"price\":\"close\"},100]},"
        "\"sizing\":{\"type\":\"fixed_qty\",\"qty\":1}}";

    char *out = NULL;
    int code = wickra_backtest_run(open, high, low, close, volume, time, 4,
                                   spec, 1000.0, &out);
    if (code != WICKRA_BT_OK) {
        fprintf(stderr, "error (%d): %s\n", code, out ? out : "(null)");
        wickra_backtest_free_string(out);
        return 1;
    }
    printf("%s\n", out);
    wickra_backtest_free_string(out);
    return 0;
}
