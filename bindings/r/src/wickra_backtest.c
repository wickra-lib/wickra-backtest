#include <R.h>
#include <Rinternals.h>
#include <R_ext/Rdynload.h>
#include <stdint.h>

#include "wickra_backtest.h"

/* Return the native library version as a length-one character vector. */
SEXP wkbt_version(void) {
    return mkString(wickra_backtest_version());
}

/* Run a strategy spec over OHLCV data. Returns list(code, json). */
SEXP wkbt_run(SEXP open, SEXP high, SEXP low, SEXP close, SEXP volume,
              SEXP time, SEXP spec, SEXP capital) {
    int n = LENGTH(open);

    /* R has no native int64; timestamps arrive as doubles. */
    int64_t *t = (int64_t *) R_alloc((size_t) n, sizeof(int64_t));
    double *td = REAL(time);
    for (int i = 0; i < n; i++) {
        t[i] = (int64_t) td[i];
    }

    const char *cspec = CHAR(STRING_ELT(spec, 0));
    char *out = NULL;
    int code = wickra_backtest_run(
        REAL(open), REAL(high), REAL(low), REAL(close), REAL(volume),
        t, (uintptr_t) n, cspec, REAL(capital)[0], &out);

    SEXP json = PROTECT(mkString(out != NULL ? out : ""));
    if (out != NULL) {
        wickra_backtest_free_string(out);
    }
    SEXP ans = PROTECT(allocVector(VECSXP, 2));
    SET_VECTOR_ELT(ans, 0, ScalarInteger(code));
    SET_VECTOR_ELT(ans, 1, json);
    UNPROTECT(2);
    return ans;
}

static const R_CallMethodDef CallEntries[] = {
    {"wkbt_run", (DL_FUNC) &wkbt_run, 8},
    {"wkbt_version", (DL_FUNC) &wkbt_version, 0},
    {NULL, NULL, 0}
};

void R_init_wickrabacktest(DllInfo *dll) {
    R_registerRoutines(dll, NULL, CallEntries, NULL, NULL);
    R_useDynamicSymbols(dll, FALSE);
}
