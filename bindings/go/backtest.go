// Package wickrabacktest is a Go binding for the wickra-backtest engine. It
// calls the stable C ABI through cgo, so the results are byte-identical to the
// Rust, Python, Node.js, WASM, C# and Java bindings: one engine kernel behind
// every language.
//
// The native library wickra_backtest must be built first
// (cargo build -p wickra-backtest-c) and staged under lib/<goos>_<goarch>/. On
// Linux/macOS the library path is baked in via rpath; on Windows the DLL must be
// discoverable at run time (next to the executable or on PATH).
package wickrabacktest

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo linux,amd64 LDFLAGS: -L${SRCDIR}/lib/linux_amd64 -lwickra_backtest -Wl,-rpath,${SRCDIR}/lib/linux_amd64
#cgo linux,arm64 LDFLAGS: -L${SRCDIR}/lib/linux_arm64 -lwickra_backtest -Wl,-rpath,${SRCDIR}/lib/linux_arm64
#cgo darwin,amd64 LDFLAGS: -L${SRCDIR}/lib/darwin_amd64 -lwickra_backtest -Wl,-rpath,${SRCDIR}/lib/darwin_amd64
#cgo darwin,arm64 LDFLAGS: -L${SRCDIR}/lib/darwin_arm64 -lwickra_backtest -Wl,-rpath,${SRCDIR}/lib/darwin_arm64
#cgo windows,amd64 LDFLAGS: -L${SRCDIR}/lib/windows_amd64 -l:wickra_backtest.dll
#cgo windows,arm64 LDFLAGS: -L${SRCDIR}/lib/windows_arm64 -l:wickra_backtest.dll
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

// RunJSON runs a backtest from a single request bundle: a JSON document
// carrying the candles, the spec, the starting capital and any optional feeds.
// It returns the report as JSON. The returned error wraps the engine's message
// for an invalid request; no panic ever crosses the FFI boundary.
func RunJSON(requestJSON string) (string, error) {
	creq := C.CString(requestJSON)
	defer C.free(unsafe.Pointer(creq))

	var out *C.char
	code := C.wickra_backtest_run_json(creq, &out)

	if out == nil {
		return "", fmt.Errorf("wickra_backtest_run_json returned code %d with no message", int(code))
	}
	json := C.GoString(out)
	C.wickra_backtest_free_string(out)
	if code != 0 {
		return "", fmt.Errorf("wickra_backtest_run_json failed (code %d): %s", int(code), json)
	}
	return json, nil
}
