//! C ABI for `wickra-backtest` — the hub for Go, C#, Java, R and C/C++.
//!
//! `wickra_backtest_run(...)` takes parallel OHLCV arrays and a JSON strategy
//! spec and writes the `BacktestReport` as a newly-allocated JSON string to
//! `*out_json` (free it with `wickra_backtest_free_string`). It returns `0` on
//! success and a non-zero code on error (the error message is written to
//! `*out_json`). No panic crosses the boundary.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int};
use std::panic::{catch_unwind, AssertUnwindSafe};

use wickra_backtest_core::{
    run_json, run_with_capital, Candle, StepRequest, StrategySpec, StreamingBacktest,
};

/// Success.
pub const WICKRA_BT_OK: c_int = 0;
/// The inputs or spec were invalid (message in `*out_json`).
pub const WICKRA_BT_ERROR: c_int = 1;
/// A null argument was passed, or an internal panic was caught.
pub const WICKRA_BT_PANIC: c_int = 2;

/// # Safety
/// The six array pointers must each point to `n` elements; `spec_json` must be a
/// valid NUL-terminated string; `out_json` must be a valid pointer to a
/// `char *`. On success `*out_json` receives a string to free with
/// [`wickra_backtest_free_string`].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn wickra_backtest_run(
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    volume: *const c_double,
    time: *const i64,
    n: usize,
    spec_json: *const c_char,
    capital: c_double,
    out_json: *mut *mut c_char,
) -> c_int {
    if out_json.is_null() {
        return WICKRA_BT_PANIC;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        run_ffi(open, high, low, close, volume, time, n, spec_json, capital)
    }));
    let (code, payload) = match outcome {
        Ok(Ok(json)) => (WICKRA_BT_OK, json),
        Ok(Err(msg)) => (WICKRA_BT_ERROR, msg),
        Err(_) => (WICKRA_BT_PANIC, "panic in wickra_backtest_run".to_string()),
    };
    let Ok(cs) = CString::new(payload) else {
        *out_json = std::ptr::null_mut();
        return WICKRA_BT_PANIC;
    };
    *out_json = cs.into_raw();
    code
}

#[allow(clippy::too_many_arguments)]
unsafe fn run_ffi(
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    volume: *const c_double,
    time: *const i64,
    n: usize,
    spec_json: *const c_char,
    capital: c_double,
) -> Result<String, String> {
    if open.is_null()
        || high.is_null()
        || low.is_null()
        || close.is_null()
        || volume.is_null()
        || time.is_null()
        || spec_json.is_null()
    {
        return Err("null argument".to_string());
    }
    let o = std::slice::from_raw_parts(open, n);
    let h = std::slice::from_raw_parts(high, n);
    let l = std::slice::from_raw_parts(low, n);
    let c = std::slice::from_raw_parts(close, n);
    let v = std::slice::from_raw_parts(volume, n);
    let t = std::slice::from_raw_parts(time, n);
    let spec_str = CStr::from_ptr(spec_json)
        .to_str()
        .map_err(|e| e.to_string())?;

    let candles: Vec<Candle> = (0..n)
        .map(|i| Candle {
            time: t[i],
            open: o[i],
            high: h[i],
            low: l[i],
            close: c[i],
            volume: v[i],
        })
        .collect();

    let spec = StrategySpec::parse(spec_str).map_err(|e| e.to_string())?;
    let report = run_with_capital(&spec, &candles, capital).map_err(|e| e.to_string())?;
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

/// Run a backtest from a single JSON request (see `RunRequest`: a document with
/// `spec`, `candles`, optional `capital` and optional `reference` / `derivs` /
/// `books` / `trades` / `sections` feeds), writing the report JSON to
/// `*out_json` (free it with [`wickra_backtest_free_string`]).
///
/// # Safety
/// `request_json` must be a valid NUL-terminated string; `out_json` must be a
/// valid pointer to a `char *`.
#[no_mangle]
pub unsafe extern "C" fn wickra_backtest_run_json(
    request_json: *const c_char,
    out_json: *mut *mut c_char,
) -> c_int {
    if out_json.is_null() {
        return WICKRA_BT_PANIC;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| run_json_ffi(request_json)));
    let (code, payload) = match outcome {
        Ok(Ok(json)) => (WICKRA_BT_OK, json),
        Ok(Err(msg)) => (WICKRA_BT_ERROR, msg),
        Err(_) => (
            WICKRA_BT_PANIC,
            "panic in wickra_backtest_run_json".to_string(),
        ),
    };
    let Ok(cs) = CString::new(payload) else {
        *out_json = std::ptr::null_mut();
        return WICKRA_BT_PANIC;
    };
    *out_json = cs.into_raw();
    code
}

unsafe fn run_json_ffi(request_json: *const c_char) -> Result<String, String> {
    if request_json.is_null() {
        return Err("null argument".to_string());
    }
    let req = CStr::from_ptr(request_json)
        .to_str()
        .map_err(|e| e.to_string())?;
    run_json(req).map_err(|e| e.to_string())
}

/// Free a string returned by [`wickra_backtest_run`].
///
/// # Safety
/// `s` must be a pointer returned by [`wickra_backtest_run`] (and not already
/// freed), or null.
#[no_mangle]
pub unsafe extern "C" fn wickra_backtest_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// The library version as a static NUL-terminated string (do not free).
#[no_mangle]
pub extern "C" fn wickra_backtest_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0")
        .as_ptr()
        .cast::<c_char>()
}

// --- streaming handle -------------------------------------------------------
//
// `wickra_backtest_run` and `_run_json` answer a whole series at once. The
// streaming handle drives the same engine one bar at a time, which is what makes
// "backtest and live are one code path" more than a slogan: a live loop feeds
// `_stream_step` from a socket instead of from an array, and every value it
// produces comes from the code path a backtest already exercised.
//
// The handle is opaque and owns its spec (`StreamingBacktest::new_owned`),
// because a borrowing handle cannot outlive the call that created it and this one
// has to. Every entry point below is wrapped in `catch_unwind` like the two
// above: unwinding out of `extern "C"` is undefined behaviour, so a panic is
// converted to `WICKRA_BT_PANIC` at the boundary.

/// An in-progress streaming backtest. Opaque to C; create it with
/// [`wickra_backtest_stream_new`] and release it with
/// [`wickra_backtest_stream_free`], or by calling
/// [`wickra_backtest_stream_finish_json`], which consumes it.
#[derive(Debug)]
pub struct WickraBacktestStream {
    inner: StreamingBacktest<'static>,
}

/// Write `payload` to `*out` as a newly allocated C string. Returns `false` (and
/// stores null) if the payload contains an interior NUL and cannot be a C string.
unsafe fn write_out(out: *mut *mut c_char, payload: String) -> bool {
    let Ok(cs) = CString::new(payload) else {
        *out = std::ptr::null_mut();
        return false;
    };
    *out = cs.into_raw();
    true
}

/// Start a streaming backtest from a JSON strategy spec.
///
/// On success writes the handle to `*out_handle` and returns [`WICKRA_BT_OK`].
/// On failure writes the message to `*out_err` and leaves `*out_handle` null.
///
/// # Safety
/// `spec_json` must be a valid NUL-terminated string; `out_handle` and `out_err`
/// must be valid pointers. Any string written to `*out_err` is freed with
/// [`wickra_backtest_free_string`].
#[no_mangle]
pub unsafe extern "C" fn wickra_backtest_stream_new(
    spec_json: *const c_char,
    capital: c_double,
    out_handle: *mut *mut WickraBacktestStream,
    out_err: *mut *mut c_char,
) -> c_int {
    if out_handle.is_null() || out_err.is_null() {
        return WICKRA_BT_PANIC;
    }
    *out_handle = std::ptr::null_mut();
    *out_err = std::ptr::null_mut();

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        if spec_json.is_null() {
            return Err("null argument".to_string());
        }
        let spec_str = CStr::from_ptr(spec_json)
            .to_str()
            .map_err(|e| e.to_string())?;
        let spec = StrategySpec::parse(spec_str).map_err(|e| e.to_string())?;
        StreamingBacktest::new_owned(spec, capital).map_err(|e| e.to_string())
    }));

    match outcome {
        Ok(Ok(bt)) => {
            *out_handle = Box::into_raw(Box::new(WickraBacktestStream { inner: bt }));
            WICKRA_BT_OK
        }
        Ok(Err(msg)) => {
            write_out(out_err, msg);
            WICKRA_BT_ERROR
        }
        Err(_) => {
            write_out(out_err, "panic in wickra_backtest_stream_new".to_string());
            WICKRA_BT_PANIC
        }
    }
}

/// Advance the backtest by one candle.
///
/// # Safety
/// `handle` must come from [`wickra_backtest_stream_new`] and must not have been
/// freed; `out_err` must be a valid pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn wickra_backtest_stream_step(
    handle: *mut WickraBacktestStream,
    open: c_double,
    high: c_double,
    low: c_double,
    close: c_double,
    volume: c_double,
    time: i64,
    out_err: *mut *mut c_char,
) -> c_int {
    if out_err.is_null() {
        return WICKRA_BT_PANIC;
    }
    *out_err = std::ptr::null_mut();
    if handle.is_null() {
        write_out(out_err, "null handle".to_string());
        return WICKRA_BT_ERROR;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let candle = Candle {
            time,
            open,
            high,
            low,
            close,
            volume,
        };
        (*handle).inner.step(&candle).map_err(|e| e.to_string())
    }));
    match outcome {
        Ok(Ok(())) => WICKRA_BT_OK,
        Ok(Err(msg)) => {
            write_out(out_err, msg);
            WICKRA_BT_ERROR
        }
        Err(_) => {
            write_out(out_err, "panic in wickra_backtest_stream_step".to_string());
            WICKRA_BT_PANIC
        }
    }
}

/// Advance the backtest by one bar described as a JSON document.
///
/// `step_json` is a [`StepRequest`]: `{"candle": {...}, "feeds": {...}}`, where
/// `feeds` is optional and carries this bar's `reference`, `deriv`, `orderbook`,
/// `trades` and `cross_section`. Use this rather than
/// [`wickra_backtest_stream_step`] whenever the strategy reads a side feed —
/// the scalar form can only supply OHLCV.
///
/// # Safety
/// `handle` must come from [`wickra_backtest_stream_new`] and must not have been
/// freed; `step_json` must be a valid NUL-terminated string; `out_err` must be a
/// valid pointer.
#[no_mangle]
pub unsafe extern "C" fn wickra_backtest_stream_step_json(
    handle: *mut WickraBacktestStream,
    step_json: *const c_char,
    out_err: *mut *mut c_char,
) -> c_int {
    if out_err.is_null() {
        return WICKRA_BT_PANIC;
    }
    *out_err = std::ptr::null_mut();
    if handle.is_null() {
        write_out(out_err, "null handle".to_string());
        return WICKRA_BT_ERROR;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        if step_json.is_null() {
            return Err("null argument".to_string());
        }
        let text = CStr::from_ptr(step_json)
            .to_str()
            .map_err(|e| e.to_string())?;
        let step: StepRequest = serde_json::from_str(text).map_err(|e| e.to_string())?;
        (*handle)
            .inner
            .step_with_feeds(&step.candle, &step.feeds.as_feeds())
            .map_err(|e| e.to_string())
    }));
    match outcome {
        Ok(Ok(())) => WICKRA_BT_OK,
        Ok(Err(msg)) => {
            write_out(out_err, msg);
            WICKRA_BT_ERROR
        }
        Err(_) => {
            write_out(
                out_err,
                "panic in wickra_backtest_stream_step_json".to_string(),
            );
            WICKRA_BT_PANIC
        }
    }
}

/// The number of closed trades so far.
///
/// # Safety
/// `handle` must come from [`wickra_backtest_stream_new`] and must not have been
/// freed; `out_count` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn wickra_backtest_stream_num_trades(
    handle: *const WickraBacktestStream,
    out_count: *mut usize,
) -> c_int {
    if out_count.is_null() || handle.is_null() {
        return WICKRA_BT_PANIC;
    }
    match catch_unwind(AssertUnwindSafe(|| (*handle).inner.num_trades())) {
        Ok(n) => {
            *out_count = n;
            WICKRA_BT_OK
        }
        Err(_) => WICKRA_BT_PANIC,
    }
}

/// The most recent equity point as JSON, or the JSON literal `null` while no bar
/// has been fed yet.
///
/// # Safety
/// `handle` must come from [`wickra_backtest_stream_new`] and must not have been
/// freed; `out_json` must be a valid pointer. Free the string with
/// [`wickra_backtest_free_string`].
#[no_mangle]
pub unsafe extern "C" fn wickra_backtest_stream_latest_equity_json(
    handle: *const WickraBacktestStream,
    out_json: *mut *mut c_char,
) -> c_int {
    stream_json(handle, out_json, "latest_equity", |bt| {
        serde_json::to_string(&bt.latest_equity())
    })
}

/// The whole equity curve so far, as a JSON array.
///
/// # Safety
/// `handle` must come from [`wickra_backtest_stream_new`] and must not have been
/// freed; `out_json` must be a valid pointer. Free the string with
/// [`wickra_backtest_free_string`].
#[no_mangle]
pub unsafe extern "C" fn wickra_backtest_stream_equity_json(
    handle: *const WickraBacktestStream,
    out_json: *mut *mut c_char,
) -> c_int {
    stream_json(handle, out_json, "equity", |bt| {
        serde_json::to_string(bt.equity())
    })
}

/// Shared body for the two read-only JSON accessors above.
unsafe fn stream_json(
    handle: *const WickraBacktestStream,
    out_json: *mut *mut c_char,
    what: &str,
    read: impl Fn(&StreamingBacktest<'static>) -> serde_json::Result<String>,
) -> c_int {
    if out_json.is_null() {
        return WICKRA_BT_PANIC;
    }
    *out_json = std::ptr::null_mut();
    if handle.is_null() {
        write_out(out_json, "null handle".to_string());
        return WICKRA_BT_ERROR;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        read(&(*handle).inner).map_err(|e| e.to_string())
    }));
    match outcome {
        Ok(Ok(json)) => {
            if write_out(out_json, json) {
                WICKRA_BT_OK
            } else {
                WICKRA_BT_PANIC
            }
        }
        Ok(Err(msg)) => {
            write_out(out_json, msg);
            WICKRA_BT_ERROR
        }
        Err(_) => {
            write_out(
                out_json,
                format!("panic in wickra_backtest_stream_{what}_json"),
            );
            WICKRA_BT_PANIC
        }
    }
}

/// Close any open position, produce the report JSON, and consume the handle.
///
/// The handle is freed whatever the outcome: it must not be used again, and must
/// not be passed to [`wickra_backtest_stream_free`].
///
/// # Safety
/// `handle` must come from [`wickra_backtest_stream_new`] and must not have been
/// freed; `out_json` must be a valid pointer. Free the string with
/// [`wickra_backtest_free_string`].
#[no_mangle]
pub unsafe extern "C" fn wickra_backtest_stream_finish_json(
    handle: *mut WickraBacktestStream,
    out_json: *mut *mut c_char,
) -> c_int {
    if out_json.is_null() {
        return WICKRA_BT_PANIC;
    }
    *out_json = std::ptr::null_mut();
    if handle.is_null() {
        write_out(out_json, "null handle".to_string());
        return WICKRA_BT_ERROR;
    }
    let boxed = Box::from_raw(handle);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        serde_json::to_string(&boxed.inner.finish()).map_err(|e| e.to_string())
    }));
    match outcome {
        Ok(Ok(json)) => {
            if write_out(out_json, json) {
                WICKRA_BT_OK
            } else {
                WICKRA_BT_PANIC
            }
        }
        Ok(Err(msg)) => {
            write_out(out_json, msg);
            WICKRA_BT_ERROR
        }
        Err(_) => {
            write_out(
                out_json,
                "panic in wickra_backtest_stream_finish_json".to_string(),
            );
            WICKRA_BT_PANIC
        }
    }
}

/// Release a handle without producing a report.
///
/// # Safety
/// `handle` must come from [`wickra_backtest_stream_new`] and must not have been
/// freed or consumed by [`wickra_backtest_stream_finish_json`]. Null is a no-op.
#[no_mangle]
pub unsafe extern "C" fn wickra_backtest_stream_free(handle: *mut WickraBacktestStream) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = r#"{"symbol":"x","timeframe":"1h","indicators":{},
        "entry":{"gt":[{"price":"close"},100]},
        "exit":{"lt":[{"price":"close"},100]},
        "sizing":{"type":"fixed_qty","qty":1}}"#;

    #[test]
    fn ffi_round_trip_matches_engine() {
        let open = [100.0, 102.0, 104.0, 98.0];
        let high = [101.0, 103.0, 104.0, 98.0];
        let low = [100.0, 102.0, 99.0, 97.0];
        let close = [101.0, 103.0, 99.0, 97.0];
        let volume = [0.0, 0.0, 0.0, 0.0];
        let time = [0i64, 1, 2, 3];
        let spec = CString::new(SPEC).unwrap();
        let mut out: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_run(
                open.as_ptr(),
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                volume.as_ptr(),
                time.as_ptr(),
                4,
                spec.as_ptr(),
                1000.0,
                std::ptr::addr_of_mut!(out),
            )
        };
        assert_eq!(code, WICKRA_BT_OK);
        let json = unsafe { CStr::from_ptr(out).to_str().unwrap().to_string() };
        unsafe { wickra_backtest_free_string(out) };
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["metrics"]["num_trades"], 1);
        assert!((v["trades"][0]["pnl"].as_f64().unwrap() + 4.0).abs() < 1e-9);
        assert!(
            (v["equity"].as_array().unwrap().last().unwrap()["equity"]
                .as_f64()
                .unwrap()
                - 996.0)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn invalid_spec_returns_error() {
        let spec = CString::new(r#"{"bad":true}"#).unwrap();
        let xs = [1.0];
        let ts = [0i64];
        let mut out: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_run(
                xs.as_ptr(),
                xs.as_ptr(),
                xs.as_ptr(),
                xs.as_ptr(),
                xs.as_ptr(),
                ts.as_ptr(),
                1,
                spec.as_ptr(),
                1000.0,
                std::ptr::addr_of_mut!(out),
            )
        };
        assert_eq!(code, WICKRA_BT_ERROR);
        unsafe { wickra_backtest_free_string(out) };
    }

    #[test]
    fn version_is_nul_terminated() {
        let p = wickra_backtest_version();
        let s = unsafe { CStr::from_ptr(p).to_str().unwrap() };
        assert!(!s.is_empty());
    }

    #[test]
    fn run_json_round_trip() {
        let request = CString::new(
            r#"{"capital":1000,
                "spec":{"symbol":"x","timeframe":"1h","indicators":{},
                    "entry":{"gt":[{"price":"close"},100]},
                    "exit":{"lt":[{"price":"close"},100]},
                    "sizing":{"type":"fixed_qty","qty":1}},
                "candles":[
                    {"time":0,"open":100,"high":101,"low":100,"close":101},
                    {"time":1,"open":102,"high":103,"low":102,"close":103},
                    {"time":2,"open":104,"high":104,"low":99,"close":99},
                    {"time":3,"open":98,"high":98,"low":97,"close":97}]}"#,
        )
        .unwrap();
        let mut out: *mut c_char = std::ptr::null_mut();
        let code =
            unsafe { wickra_backtest_run_json(request.as_ptr(), std::ptr::addr_of_mut!(out)) };
        assert_eq!(code, WICKRA_BT_OK);
        let json = unsafe { CStr::from_ptr(out).to_str().unwrap().to_string() };
        unsafe { wickra_backtest_free_string(out) };
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["metrics"]["num_trades"], 1);
        assert!((v["trades"][0]["entry_price"].as_f64().unwrap() - 102.0).abs() < 1e-9);
    }

    /// Drive the C ABI over the shared golden corpus and assert the report is
    /// byte-for-byte identical to the canonical expected reports. This pins the
    /// C / C++ language reach (the same `extern "C"` entry point `example.c`
    /// calls) to the same contract as every other binding.
    #[test]
    fn golden_parity_through_ffi() {
        use std::fs;
        use std::path::PathBuf;

        let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../golden");
        let mut cases: Vec<PathBuf> = fs::read_dir(golden.join("cases"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        cases.sort();
        assert!(!cases.is_empty(), "no golden cases found");

        let col = |v: &serde_json::Value, k: &str| -> Vec<f64> {
            v[k].as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect()
        };

        for path in cases {
            let v: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            let name = v["name"].as_str().unwrap();
            let capital = v["capital"].as_f64().unwrap();
            let spec = CString::new(v["spec"].to_string()).unwrap();
            let (open, high, low, close, volume) = (
                col(&v, "open"),
                col(&v, "high"),
                col(&v, "low"),
                col(&v, "close"),
                col(&v, "volume"),
            );
            let time: Vec<i64> = v["time"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_i64().unwrap())
                .collect();

            let mut out: *mut c_char = std::ptr::null_mut();
            let code = unsafe {
                wickra_backtest_run(
                    open.as_ptr(),
                    high.as_ptr(),
                    low.as_ptr(),
                    close.as_ptr(),
                    volume.as_ptr(),
                    time.as_ptr(),
                    open.len(),
                    spec.as_ptr(),
                    capital,
                    std::ptr::addr_of_mut!(out),
                )
            };
            assert_eq!(code, WICKRA_BT_OK, "case {name} failed");
            let got = unsafe { CStr::from_ptr(out).to_str().unwrap().to_string() };
            unsafe { wickra_backtest_free_string(out) };

            let want =
                fs::read_to_string(golden.join("expected").join(format!("{name}.json"))).unwrap();
            assert_eq!(got, want.trim_end(), "golden mismatch for {name}");
        }
    }

    /// Drive the C ABI's `run_json` over the shared feed request corpus and
    /// assert the report is byte-for-byte identical to the canonical expected
    /// reports — pinning the C / C++ microstructure feed paths to the same
    /// contract as every other binding.
    #[test]
    fn feed_golden_parity_through_ffi() {
        use std::fs;
        use std::path::PathBuf;

        let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../golden");
        let mut requests: Vec<PathBuf> = fs::read_dir(golden.join("requests"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        requests.sort();
        assert!(!requests.is_empty(), "no golden requests found");

        for path in requests {
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();
            let request = CString::new(fs::read_to_string(&path).unwrap()).unwrap();
            let mut out: *mut c_char = std::ptr::null_mut();
            let code =
                unsafe { wickra_backtest_run_json(request.as_ptr(), std::ptr::addr_of_mut!(out)) };
            assert_eq!(code, WICKRA_BT_OK, "request {name} failed");
            let got = unsafe { CStr::from_ptr(out).to_str().unwrap().to_string() };
            unsafe { wickra_backtest_free_string(out) };

            let want =
                fs::read_to_string(golden.join("expected_json").join(format!("{name}.json")))
                    .unwrap();
            assert_eq!(got, want.trim_end(), "feed golden mismatch for {name}");
        }
    }

    /// Feed the four bars of `ffi_round_trip_matches_engine` through the handle.
    /// Returns the report JSON; the handle is consumed by `finish`.
    fn stream_the_fixture(capital: c_double) -> serde_json::Value {
        let bars: [(f64, f64, f64, f64, i64); 4] = [
            (100.0, 101.0, 100.0, 101.0, 0),
            (102.0, 103.0, 102.0, 103.0, 1),
            (104.0, 104.0, 99.0, 99.0, 2),
            (98.0, 98.0, 97.0, 97.0, 3),
        ];
        let spec = CString::new(SPEC).unwrap();
        let mut handle: *mut WickraBacktestStream = std::ptr::null_mut();
        let mut err: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_stream_new(
                spec.as_ptr(),
                capital,
                std::ptr::addr_of_mut!(handle),
                std::ptr::addr_of_mut!(err),
            )
        };
        assert_eq!(code, WICKRA_BT_OK);
        assert!(!handle.is_null());
        assert!(err.is_null());

        for (open, high, low, close, time) in bars {
            let code = unsafe {
                wickra_backtest_stream_step(
                    handle,
                    open,
                    high,
                    low,
                    close,
                    0.0,
                    time,
                    std::ptr::addr_of_mut!(err),
                )
            };
            assert_eq!(code, WICKRA_BT_OK);
            assert!(err.is_null());
        }

        let mut out: *mut c_char = std::ptr::null_mut();
        let code =
            unsafe { wickra_backtest_stream_finish_json(handle, std::ptr::addr_of_mut!(out)) };
        assert_eq!(code, WICKRA_BT_OK);
        let json = unsafe { CStr::from_ptr(out).to_str().unwrap().to_string() };
        unsafe { wickra_backtest_free_string(out) };
        serde_json::from_str(&json).unwrap()
    }

    /// The claim the README makes -- one code path for backtest and live -- is only
    /// true if the handle reproduces the batch report bar for bar. Same spec, same
    /// candles, same capital: the two JSON documents must be identical.
    #[test]
    fn streaming_reproduces_the_batch_report() {
        let open = [100.0, 102.0, 104.0, 98.0];
        let high = [101.0, 103.0, 104.0, 98.0];
        let low = [100.0, 102.0, 99.0, 97.0];
        let close = [101.0, 103.0, 99.0, 97.0];
        let volume = [0.0, 0.0, 0.0, 0.0];
        let time = [0i64, 1, 2, 3];
        let spec = CString::new(SPEC).unwrap();
        let mut out: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_run(
                open.as_ptr(),
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                volume.as_ptr(),
                time.as_ptr(),
                4,
                spec.as_ptr(),
                1000.0,
                std::ptr::addr_of_mut!(out),
            )
        };
        assert_eq!(code, WICKRA_BT_OK);
        let batch_json = unsafe { CStr::from_ptr(out).to_str().unwrap().to_string() };
        unsafe { wickra_backtest_free_string(out) };
        let batch: serde_json::Value = serde_json::from_str(&batch_json).unwrap();

        assert_eq!(stream_the_fixture(1000.0), batch);
    }

    /// The read-only accessors must track the run as it advances, and
    /// `stream_free` must release a handle that was never finished.
    #[test]
    fn accessors_track_the_run_and_free_releases_an_unfinished_handle() {
        let spec = CString::new(SPEC).unwrap();
        let mut handle: *mut WickraBacktestStream = std::ptr::null_mut();
        let mut err: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_stream_new(
                spec.as_ptr(),
                1000.0,
                std::ptr::addr_of_mut!(handle),
                std::ptr::addr_of_mut!(err),
            )
        };
        assert_eq!(code, WICKRA_BT_OK);

        // Before the first bar there is no equity point at all.
        let mut out: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_stream_latest_equity_json(handle, std::ptr::addr_of_mut!(out))
        };
        assert_eq!(code, WICKRA_BT_OK);
        let latest = unsafe { CStr::from_ptr(out).to_str().unwrap().to_string() };
        unsafe { wickra_backtest_free_string(out) };
        assert_eq!(latest, "null");

        for (open, high, low, close, time) in [
            (100.0, 101.0, 100.0, 101.0, 0i64),
            (102.0, 103.0, 102.0, 103.0, 1),
            (104.0, 104.0, 99.0, 99.0, 2),
        ] {
            let code = unsafe {
                wickra_backtest_stream_step(
                    handle,
                    open,
                    high,
                    low,
                    close,
                    0.0,
                    time,
                    std::ptr::addr_of_mut!(err),
                )
            };
            assert_eq!(code, WICKRA_BT_OK);
        }

        // Three bars in, the curve has three points.
        let mut out: *mut c_char = std::ptr::null_mut();
        let code =
            unsafe { wickra_backtest_stream_equity_json(handle, std::ptr::addr_of_mut!(out)) };
        assert_eq!(code, WICKRA_BT_OK);
        let curve: serde_json::Value =
            serde_json::from_str(unsafe { CStr::from_ptr(out).to_str().unwrap() }).unwrap();
        unsafe { wickra_backtest_free_string(out) };
        assert_eq!(curve.as_array().unwrap().len(), 3);

        let mut out: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_stream_latest_equity_json(handle, std::ptr::addr_of_mut!(out))
        };
        assert_eq!(code, WICKRA_BT_OK);
        let latest: serde_json::Value =
            serde_json::from_str(unsafe { CStr::from_ptr(out).to_str().unwrap() }).unwrap();
        unsafe { wickra_backtest_free_string(out) };
        assert_eq!(latest, curve.as_array().unwrap()[2]);

        // Bar 2 closed below 100, which is the exit *signal* -- the fill happens at
        // the next bar's open, so no trade has closed yet.
        let mut trades: usize = 999;
        let code =
            unsafe { wickra_backtest_stream_num_trades(handle, std::ptr::addr_of_mut!(trades)) };
        assert_eq!(code, WICKRA_BT_OK);
        assert_eq!(trades, 0);

        // Feeding the bar the signal points at closes it.
        let code = unsafe {
            wickra_backtest_stream_step(
                handle,
                98.0,
                98.0,
                97.0,
                97.0,
                0.0,
                3,
                std::ptr::addr_of_mut!(err),
            )
        };
        assert_eq!(code, WICKRA_BT_OK);
        let code =
            unsafe { wickra_backtest_stream_num_trades(handle, std::ptr::addr_of_mut!(trades)) };
        assert_eq!(code, WICKRA_BT_OK);
        assert_eq!(trades, 1);

        // Dropped without a report: the handle is released here, not leaked.
        unsafe { wickra_backtest_stream_free(handle) };
        unsafe { wickra_backtest_stream_free(std::ptr::null_mut()) };
    }

    #[test]
    fn stream_new_reports_an_invalid_spec_and_leaves_no_handle() {
        let spec = CString::new(r#"{"bad":true}"#).unwrap();
        let mut handle: *mut WickraBacktestStream = std::ptr::null_mut();
        let mut err: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_stream_new(
                spec.as_ptr(),
                1000.0,
                std::ptr::addr_of_mut!(handle),
                std::ptr::addr_of_mut!(err),
            )
        };
        assert_eq!(code, WICKRA_BT_ERROR);
        assert!(handle.is_null());
        assert!(!err.is_null());
        let msg = unsafe { CStr::from_ptr(err).to_str().unwrap().to_string() };
        unsafe { wickra_backtest_free_string(err) };
        assert!(!msg.is_empty());
    }

    /// A null handle is a caller mistake, not a crash: every entry point that can
    /// carry a message reports one instead of dereferencing it.
    #[test]
    fn a_null_handle_is_reported_rather_than_dereferenced() {
        let mut err: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_stream_step(
                std::ptr::null_mut(),
                1.0,
                1.0,
                1.0,
                1.0,
                0.0,
                0,
                std::ptr::addr_of_mut!(err),
            )
        };
        assert_eq!(code, WICKRA_BT_ERROR);
        unsafe { wickra_backtest_free_string(err) };

        let mut out: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_stream_equity_json(std::ptr::null(), std::ptr::addr_of_mut!(out))
        };
        assert_eq!(code, WICKRA_BT_ERROR);
        unsafe { wickra_backtest_free_string(out) };

        let mut out: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_stream_finish_json(std::ptr::null_mut(), std::ptr::addr_of_mut!(out))
        };
        assert_eq!(code, WICKRA_BT_ERROR);
        unsafe { wickra_backtest_free_string(out) };

        let mut n: usize = 0;
        let code = unsafe {
            wickra_backtest_stream_num_trades(std::ptr::null(), std::ptr::addr_of_mut!(n))
        };
        assert_eq!(code, WICKRA_BT_PANIC);
    }

    /// The document form must be a drop-in for the scalar form: same bars in,
    /// same report out. It exists to carry side feeds, not to behave differently.
    #[test]
    fn step_json_matches_the_scalar_step() {
        let steps = [
            r#"{"candle":{"time":0,"open":100.0,"high":101.0,"low":100.0,"close":101.0,"volume":0.0}}"#,
            r#"{"candle":{"time":1,"open":102.0,"high":103.0,"low":102.0,"close":103.0,"volume":0.0}}"#,
            r#"{"candle":{"time":2,"open":104.0,"high":104.0,"low":99.0,"close":99.0,"volume":0.0},"feeds":{}}"#,
            r#"{"candle":{"time":3,"open":98.0,"high":98.0,"low":97.0,"close":97.0,"volume":0.0},"feeds":{"reference":50.0}}"#,
        ];
        let spec = CString::new(SPEC).unwrap();
        let mut handle: *mut WickraBacktestStream = std::ptr::null_mut();
        let mut err: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_stream_new(
                spec.as_ptr(),
                1000.0,
                std::ptr::addr_of_mut!(handle),
                std::ptr::addr_of_mut!(err),
            )
        };
        assert_eq!(code, WICKRA_BT_OK);

        for step in steps {
            let doc = CString::new(step).unwrap();
            let code = unsafe {
                wickra_backtest_stream_step_json(handle, doc.as_ptr(), std::ptr::addr_of_mut!(err))
            };
            assert_eq!(code, WICKRA_BT_OK);
            assert!(err.is_null());
        }

        let mut out: *mut c_char = std::ptr::null_mut();
        let code =
            unsafe { wickra_backtest_stream_finish_json(handle, std::ptr::addr_of_mut!(out)) };
        assert_eq!(code, WICKRA_BT_OK);
        let json = unsafe { CStr::from_ptr(out).to_str().unwrap().to_string() };
        unsafe { wickra_backtest_free_string(out) };
        let via_json: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(via_json, stream_the_fixture(1000.0));
    }

    #[test]
    fn step_json_reports_a_malformed_document() {
        let spec = CString::new(SPEC).unwrap();
        let mut handle: *mut WickraBacktestStream = std::ptr::null_mut();
        let mut err: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            wickra_backtest_stream_new(
                spec.as_ptr(),
                1000.0,
                std::ptr::addr_of_mut!(handle),
                std::ptr::addr_of_mut!(err),
            )
        };
        assert_eq!(code, WICKRA_BT_OK);

        let doc = CString::new(r#"{"candle":{"open":1.0}}"#).unwrap();
        let code = unsafe {
            wickra_backtest_stream_step_json(handle, doc.as_ptr(), std::ptr::addr_of_mut!(err))
        };
        assert_eq!(code, WICKRA_BT_ERROR);
        assert!(!err.is_null());
        unsafe { wickra_backtest_free_string(err) };
        unsafe { wickra_backtest_stream_free(handle) };
    }
}
