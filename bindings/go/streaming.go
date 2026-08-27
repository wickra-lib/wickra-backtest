package wickrabacktest

/*
#include <stdlib.h>
#include "wickra_backtest.h"
*/
import "C"

import (
	"fmt"
	"unsafe"
)

// StreamingBacktest is a backtest driven one bar at a time.
//
// Run needs the whole series up front. This drives the same engine bar by bar,
// so a live loop and a backtest are the same code path: feed it from a socket
// instead of from a slice and every value it reports was produced the way the
// backtest produced it.
//
// The value owns a native handle, so Close must be called -- normally with
// defer. Unlike the C# binding there is no finalizer backstop: Go's own
// contract for a resource-holding value is an explicit Close, and a finalizer
// that may or may not run is not a substitute for one. FinishJSON also releases
// the handle, and Close after it is a no-op.
type StreamingBacktest struct {
	handle *C.WickraBacktestStream
	bars   int64
}

// NewStreamingBacktest starts a streaming backtest of spec (a JSON string).
// The returned error wraps the engine's message for an invalid spec.
func NewStreamingBacktest(spec string, capital float64) (*StreamingBacktest, error) {
	cspec := C.CString(spec)
	defer C.free(unsafe.Pointer(cspec))

	var handle *C.WickraBacktestStream
	var cerr *C.char
	code := C.wickra_backtest_stream_new(cspec, C.double(capital), &handle, &cerr)
	if code != 0 {
		msg := C.GoString(cerr)
		C.wickra_backtest_free_string(cerr)
		return nil, fmt.Errorf("wickra_backtest_stream_new failed (code %d): %s", int(code), msg)
	}
	return &StreamingBacktest{handle: handle}, nil
}

// IsFinished reports whether the run has been finished or closed.
func (b *StreamingBacktest) IsFinished() bool {
	return b.handle == nil
}

// live returns the handle, or an error naming the mistake if the run is over.
func (b *StreamingBacktest) live() (*C.WickraBacktestStream, error) {
	if b.handle == nil {
		return nil, fmt.Errorf("this backtest is finished")
	}
	return b.handle, nil
}

// Step advances the backtest by one OHLCV bar.
func (b *StreamingBacktest) Step(open, high, low, close, volume float64, time int64) error {
	handle, err := b.live()
	if err != nil {
		return err
	}
	var cerr *C.char
	code := C.wickra_backtest_stream_step(
		handle,
		C.double(open), C.double(high), C.double(low), C.double(close),
		C.double(volume), C.int64_t(time), &cerr,
	)
	if code != 0 {
		msg := C.GoString(cerr)
		C.wickra_backtest_free_string(cerr)
		return fmt.Errorf("wickra_backtest_stream_step failed (code %d): %s", int(code), msg)
	}
	b.bars++
	return nil
}

// StepSimple advances by one bar with zero volume and the bar index as its
// timestamp, mirroring RunSimple's defaults.
func (b *StreamingBacktest) StepSimple(open, high, low, close float64) error {
	return b.Step(open, high, low, close, 0, b.bars)
}

// StepJSON advances by one bar described as a request document:
// {"candle": {...}, "feeds": {...}}, where feeds optionally carries this bar's
// reference, derivatives, order-book, trade or cross-section input. This is the
// only form that can drive a strategy reading a side feed.
func (b *StreamingBacktest) StepJSON(stepJSON string) error {
	handle, err := b.live()
	if err != nil {
		return err
	}
	cstep := C.CString(stepJSON)
	defer C.free(unsafe.Pointer(cstep))

	var cerr *C.char
	code := C.wickra_backtest_stream_step_json(handle, cstep, &cerr)
	if code != 0 {
		msg := C.GoString(cerr)
		C.wickra_backtest_free_string(cerr)
		return fmt.Errorf("wickra_backtest_stream_step_json failed (code %d): %s", int(code), msg)
	}
	b.bars++
	return nil
}

// EquityJSON returns the equity curve so far as a JSON array.
func (b *StreamingBacktest) EquityJSON() (string, error) {
	handle, err := b.live()
	if err != nil {
		return "", err
	}
	var out *C.char
	code := C.wickra_backtest_stream_equity_json(handle, &out)
	return payload("wickra_backtest_stream_equity_json", code, out)
}

// LatestEquityJSON returns the most recent equity point as JSON, or the JSON
// literal null before the first bar.
func (b *StreamingBacktest) LatestEquityJSON() (string, error) {
	handle, err := b.live()
	if err != nil {
		return "", err
	}
	var out *C.char
	code := C.wickra_backtest_stream_latest_equity_json(handle, &out)
	return payload("wickra_backtest_stream_latest_equity_json", code, out)
}

// NumTrades returns the number of closed trades so far.
func (b *StreamingBacktest) NumTrades() (int, error) {
	handle, err := b.live()
	if err != nil {
		return 0, err
	}
	var count C.uintptr_t
	code := C.wickra_backtest_stream_num_trades(handle, &count)
	if code != 0 {
		return 0, fmt.Errorf("wickra_backtest_stream_num_trades failed (code %d)", int(code))
	}
	return int(count), nil
}

// FinishJSON closes any open position and returns the report as JSON. It ends
// the run: the handle is released and further use returns an error.
func (b *StreamingBacktest) FinishJSON() (string, error) {
	handle, err := b.live()
	if err != nil {
		return "", err
	}
	b.handle = nil
	var out *C.char
	code := C.wickra_backtest_stream_finish_json(handle, &out)
	return payload("wickra_backtest_stream_finish_json", code, out)
}

// Close releases the run without producing a report. It is idempotent, so it is
// safe to defer alongside FinishJSON.
func (b *StreamingBacktest) Close() {
	if b.handle != nil {
		C.wickra_backtest_stream_free(b.handle)
		b.handle = nil
	}
}

// payload turns a (code, out) pair from the C ABI into a Go string or error,
// freeing the native string either way.
func payload(fn string, code C.int, out *C.char) (string, error) {
	if out == nil {
		return "", fmt.Errorf("%s returned code %d with no message", fn, int(code))
	}
	json := C.GoString(out)
	C.wickra_backtest_free_string(out)
	if code != 0 {
		return "", fmt.Errorf("%s failed (code %d): %s", fn, int(code), json)
	}
	return json, nil
}
