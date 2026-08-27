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

/* Run a backtest from a single JSON request bundle. Returns list(code, json). */
SEXP wkbt_run_json(SEXP request) {
    const char *creq = CHAR(STRING_ELT(request, 0));
    char *out = NULL;
    int code = wickra_backtest_run_json(creq, &out);

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

/* --- streaming handle ---------------------------------------------------- */

/*
 * The engine handle is held in an externalptr. R owns the lifetime, so a handle
 * dropped without backtest_stream_finish_json() or _free() would leak Rust-side
 * memory that the R garbage collector knows nothing about -- hence a registered
 * finalizer, which also runs at session exit.
 *
 * Clearing the pointer on finish/free is what makes the finalizer safe: it sees
 * NULL and does nothing, so releasing twice is not a double free.
 */

static void wkbt_stream_finalizer(SEXP ptr) {
    WickraBacktestStream *handle = (WickraBacktestStream *) R_ExternalPtrAddr(ptr);
    if (handle != NULL) {
        wickra_backtest_stream_free(handle);
        R_ClearExternalPtr(ptr);
    }
}

/* list(code, payload): payload is the message on failure, else the value. */
static SEXP wkbt_pair(int code, SEXP payload) {
    SEXP ans = PROTECT(allocVector(VECSXP, 2));
    SET_VECTOR_ELT(ans, 0, ScalarInteger(code));
    SET_VECTOR_ELT(ans, 1, payload);
    UNPROTECT(1);
    return ans;
}

/* Copy a C string into R and free it; NULL reads as the empty string. */
static SEXP wkbt_take(char *s) {
    SEXP out = PROTECT(mkString(s != NULL ? s : ""));
    if (s != NULL) {
        wickra_backtest_free_string(s);
    }
    UNPROTECT(1);
    return out;
}

/* Start a streaming backtest. Returns list(code, externalptr | message). */
SEXP wkbt_stream_new(SEXP spec, SEXP capital) {
    const char *cspec = CHAR(STRING_ELT(spec, 0));
    WickraBacktestStream *handle = NULL;
    char *err = NULL;
    int code = wickra_backtest_stream_new(cspec, REAL(capital)[0], &handle, &err);
    if (code != 0) {
        return wkbt_pair(code, wkbt_take(err));
    }
    SEXP ptr = PROTECT(R_MakeExternalPtr(handle, R_NilValue, R_NilValue));
    R_RegisterCFinalizerEx(ptr, wkbt_stream_finalizer, TRUE);
    SEXP ans = PROTECT(wkbt_pair(code, ptr));
    UNPROTECT(2);
    return ans;
}

/* Advance by one OHLCV bar. Returns list(code, message). */
SEXP wkbt_stream_step(SEXP ptr, SEXP open, SEXP high, SEXP low, SEXP close,
                      SEXP volume, SEXP time) {
    WickraBacktestStream *handle = (WickraBacktestStream *) R_ExternalPtrAddr(ptr);
    if (handle == NULL) {
        return wkbt_pair(WICKRA_BT_ERROR, mkString("this backtest is finished"));
    }
    char *err = NULL;
    int code = wickra_backtest_stream_step(
        handle, REAL(open)[0], REAL(high)[0], REAL(low)[0], REAL(close)[0],
        REAL(volume)[0], (int64_t) REAL(time)[0], &err);
    return wkbt_pair(code, wkbt_take(err));
}

/* Advance by one bar given as a request document. Returns list(code, message). */
SEXP wkbt_stream_step_json(SEXP ptr, SEXP step) {
    WickraBacktestStream *handle = (WickraBacktestStream *) R_ExternalPtrAddr(ptr);
    if (handle == NULL) {
        return wkbt_pair(WICKRA_BT_ERROR, mkString("this backtest is finished"));
    }
    char *err = NULL;
    int code = wickra_backtest_stream_step_json(handle, CHAR(STRING_ELT(step, 0)), &err);
    return wkbt_pair(code, wkbt_take(err));
}

/* The equity curve so far. Returns list(code, json). */
SEXP wkbt_stream_equity_json(SEXP ptr) {
    WickraBacktestStream *handle = (WickraBacktestStream *) R_ExternalPtrAddr(ptr);
    if (handle == NULL) {
        return wkbt_pair(WICKRA_BT_ERROR, mkString("this backtest is finished"));
    }
    char *out = NULL;
    int code = wickra_backtest_stream_equity_json(handle, &out);
    return wkbt_pair(code, wkbt_take(out));
}

/* The most recent equity point. Returns list(code, json). */
SEXP wkbt_stream_latest_equity_json(SEXP ptr) {
    WickraBacktestStream *handle = (WickraBacktestStream *) R_ExternalPtrAddr(ptr);
    if (handle == NULL) {
        return wkbt_pair(WICKRA_BT_ERROR, mkString("this backtest is finished"));
    }
    char *out = NULL;
    int code = wickra_backtest_stream_latest_equity_json(handle, &out);
    return wkbt_pair(code, wkbt_take(out));
}

/* The number of closed trades so far. Returns list(code, count). */
SEXP wkbt_stream_num_trades(SEXP ptr) {
    WickraBacktestStream *handle = (WickraBacktestStream *) R_ExternalPtrAddr(ptr);
    if (handle == NULL) {
        return wkbt_pair(WICKRA_BT_ERROR, mkString("this backtest is finished"));
    }
    uintptr_t count = 0;
    int code = wickra_backtest_stream_num_trades(handle, &count);
    /* R has no native 64-bit integer, so the count travels as a double. */
    return wkbt_pair(code, ScalarReal((double) count));
}

/*
 * Close any open position and return the report. Ends the run: the pointer is
 * cleared first, so the finalizer will not free a handle finish already consumed.
 */
SEXP wkbt_stream_finish_json(SEXP ptr) {
    WickraBacktestStream *handle = (WickraBacktestStream *) R_ExternalPtrAddr(ptr);
    if (handle == NULL) {
        return wkbt_pair(WICKRA_BT_ERROR, mkString("this backtest is finished"));
    }
    R_ClearExternalPtr(ptr);
    char *out = NULL;
    int code = wickra_backtest_stream_finish_json(handle, &out);
    return wkbt_pair(code, wkbt_take(out));
}

/* Release the run without producing a report. Idempotent. */
SEXP wkbt_stream_free(SEXP ptr) {
    WickraBacktestStream *handle = (WickraBacktestStream *) R_ExternalPtrAddr(ptr);
    if (handle != NULL) {
        wickra_backtest_stream_free(handle);
        R_ClearExternalPtr(ptr);
    }
    return R_NilValue;
}

static const R_CallMethodDef CallEntries[] = {
    {"wkbt_run", (DL_FUNC) &wkbt_run, 8},
    {"wkbt_run_json", (DL_FUNC) &wkbt_run_json, 1},
    {"wkbt_version", (DL_FUNC) &wkbt_version, 0},
    {"wkbt_stream_new", (DL_FUNC) &wkbt_stream_new, 2},
    {"wkbt_stream_step", (DL_FUNC) &wkbt_stream_step, 7},
    {"wkbt_stream_step_json", (DL_FUNC) &wkbt_stream_step_json, 2},
    {"wkbt_stream_equity_json", (DL_FUNC) &wkbt_stream_equity_json, 1},
    {"wkbt_stream_latest_equity_json", (DL_FUNC) &wkbt_stream_latest_equity_json, 1},
    {"wkbt_stream_num_trades", (DL_FUNC) &wkbt_stream_num_trades, 1},
    {"wkbt_stream_finish_json", (DL_FUNC) &wkbt_stream_finish_json, 1},
    {"wkbt_stream_free", (DL_FUNC) &wkbt_stream_free, 1},
    {NULL, NULL, 0}
};

void R_init_wickrabacktest(DllInfo *dll) {
    R_registerRoutines(dll, NULL, CallEntries, NULL, NULL);
    R_useDynamicSymbols(dll, FALSE);
}
