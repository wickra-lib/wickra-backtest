// Package wickrabacktest is a Go binding for the wickra-backtest engine. It
// calls the stable C ABI through cgo, so the results are byte-identical to the
// Rust, Python, Node.js, WASM, C# and Java bindings: one engine kernel behind
// every language.
//
// The native library wickra_backtest must be built first
// (cargo build -p wickra-backtest-c) and reachable at run time (on PATH on
// Windows, or LD_LIBRARY_PATH / DYLD_LIBRARY_PATH on Linux / macOS).
package wickrabacktest

/*
#cgo CFLAGS: -I${SRCDIR}/../c/include
#cgo windows LDFLAGS: -L${SRCDIR}/../../target/debug -l:wickra_backtest.dll
#cgo !windows LDFLAGS: -L${SRCDIR}/../../target/debug -lwickra_backtest
#include <stdlib.h>
#include "wickra_backtest.h"
*/
import "C"

import (
	"fmt"
	"unsafe"
)

// Version returns the native library version.
func Version() string {
	return C.GoString(C.wickra_backtest_version())
}

// Run executes a strategy spec over OHLCV data and returns the report as JSON.
//
// volume may be nil (treated as all-zero) and time may be nil (treated as
// 0..n). The returned error wraps the engine's message for an invalid spec or
// mismatched inputs; no panic ever crosses the FFI boundary.
func Run(open, high, low, close, volume []float64, time []int64, spec string, capital float64) (string, error) {
	n := len(open)
	if len(high) != n || len(low) != n || len(close) != n {
		return "", fmt.Errorf("OHLC slices must have equal length")
	}
	if n == 0 {
		return "", fmt.Errorf("no candles")
	}
	if volume == nil {
		volume = make([]float64, n)
	}
	if time == nil {
		time = make([]int64, n)
		for i := range time {
			time[i] = int64(i)
		}
	}
	if len(volume) != n || len(time) != n {
		return "", fmt.Errorf("volume and time length must match OHLC")
	}

	cspec := C.CString(spec)
	defer C.free(unsafe.Pointer(cspec))

	var out *C.char
	code := C.wickra_backtest_run(
		(*C.double)(unsafe.Pointer(&open[0])),
		(*C.double)(unsafe.Pointer(&high[0])),
		(*C.double)(unsafe.Pointer(&low[0])),
		(*C.double)(unsafe.Pointer(&close[0])),
		(*C.double)(unsafe.Pointer(&volume[0])),
		(*C.int64_t)(unsafe.Pointer(&time[0])),
		C.uintptr_t(n),
		cspec,
		C.double(capital),
		&out,
	)

	if out == nil {
		return "", fmt.Errorf("wickra_backtest_run returned code %d with no message", int(code))
	}
	json := C.GoString(out)
	C.wickra_backtest_free_string(out)
	if code != 0 {
		return "", fmt.Errorf("wickra_backtest_run failed (code %d): %s", int(code), json)
	}
	return json, nil
}

// RunSimple runs with zero volume, 0..n timestamps and the given capital.
func RunSimple(open, high, low, close []float64, spec string, capital float64) (string, error) {
	return Run(open, high, low, close, nil, nil, spec, capital)
}
