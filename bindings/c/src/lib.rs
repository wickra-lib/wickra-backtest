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

use wickra_backtest_core::{run_json, run_with_capital, Candle, StrategySpec};

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
}
