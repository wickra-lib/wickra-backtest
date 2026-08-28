// Optional C++ convenience layer over the wickra-backtest C ABI
// (`wickra_backtest.h`). Header-only, and hand-written: cbindgen generates the
// `.h` beside this file, not this file.
//
// C++ already reaches every export through the C header, which is `extern "C"`.
// What it does not get for free is the ownership: the ABI hands out two
// resources, and both must be released exactly once.
//
//   - a `WickraBacktestStream *` from `wickra_backtest_stream_new`, released by
//     `wickra_backtest_stream_free` -- or CONSUMED by
//     `wickra_backtest_stream_finish_json`, after which freeing it again is a
//     double free;
//   - a `char *` written to an out-parameter by every JSON-returning call and by
//     every call that failed, released by `wickra_backtest_free_string`.
//
// A streaming run touches both on every bar, so the C example that drives one
// spends more lines on frees than on the backtest. `Stream` and `String` below
// hold each resource in a move-only owner that releases at scope exit:
//
//     #include "wickra_backtest.hpp"
//
//     wickra::backtest::Stream stream;
//     wickra::backtest::String err;
//     if (wickra_backtest_stream_new(spec, 1000.0, stream.out(), err.out())
//             != WICKRA_BT_OK) {
//         std::fprintf(stderr, "%s\n", err.c_str());   // err frees itself
//         return 1;                                    // so does stream
//     }
//     wickra_backtest_stream_step(stream.get(), o, h, l, c, v, t, err.out());
//
//     wickra::backtest::String report;
//     // finish consumes the handle, so hand over ownership with release():
//     wickra_backtest_stream_finish_json(stream.release(), report.out());
//
// `release()` is the whole reason `Stream` is not the same class as `String`:
// it is how a consuming call is expressed without the destructor then freeing a
// handle the ABI has already taken.
//
// This layer adds no runtime cost beyond the C calls themselves, throws
// nothing, and allocates nothing of its own.

#ifndef WICKRA_BACKTEST_HPP
#define WICKRA_BACKTEST_HPP

#include "wickra_backtest.h"

#include <utility>

namespace wickra {
namespace backtest {

/// Move-only owner of a string the ABI allocated, released with
/// `wickra_backtest_free_string`.
class String {
public:
    String() noexcept : ptr_(nullptr) {}

    /// Adopts an already-obtained string. Ownership passes to this object.
    explicit String(char *ptr) noexcept : ptr_(ptr) {}

    ~String() { wickra_backtest_free_string(ptr_); }

    String(const String &) = delete;
    String &operator=(const String &) = delete;

    String(String &&other) noexcept : ptr_(std::exchange(other.ptr_, nullptr)) {}

    String &operator=(String &&other) noexcept {
        if (this != &other) {
            wickra_backtest_free_string(ptr_);
            ptr_ = std::exchange(other.ptr_, nullptr);
        }
        return *this;
    }

    /// The out-parameter to pass to a `char **` argument. Any string held is
    /// released first, so one `String` can be reused across a bar loop.
    char **out() noexcept {
        wickra_backtest_free_string(ptr_);
        ptr_ = nullptr;
        return &ptr_;
    }

    /// The owned string, or `nullptr` if the call left it unset.
    char *get() const noexcept { return ptr_; }

    /// The owned string, or `"(null)"` -- for printing a message the ABI may or
    /// may not have written.
    const char *c_str() const noexcept { return ptr_ != nullptr ? ptr_ : "(null)"; }

    /// Gives up ownership; the caller must free the result.
    char *release() noexcept { return std::exchange(ptr_, nullptr); }

    /// True when a string is held.
    explicit operator bool() const noexcept { return ptr_ != nullptr; }

private:
    char *ptr_;
};

/// Move-only owner of a streaming backtest handle, released with
/// `wickra_backtest_stream_free`.
class Stream {
public:
    Stream() noexcept : ptr_(nullptr) {}

    /// Adopts an already-obtained handle. Ownership passes to this object.
    explicit Stream(WickraBacktestStream *ptr) noexcept : ptr_(ptr) {}

    ~Stream() { wickra_backtest_stream_free(ptr_); }

    Stream(const Stream &) = delete;
    Stream &operator=(const Stream &) = delete;

    Stream(Stream &&other) noexcept : ptr_(std::exchange(other.ptr_, nullptr)) {}

    Stream &operator=(Stream &&other) noexcept {
        if (this != &other) {
            wickra_backtest_stream_free(ptr_);
            ptr_ = std::exchange(other.ptr_, nullptr);
        }
        return *this;
    }

    /// The out-parameter to pass to `wickra_backtest_stream_new`. Any handle
    /// held is released first.
    WickraBacktestStream **out() noexcept {
        wickra_backtest_stream_free(ptr_);
        ptr_ = nullptr;
        return &ptr_;
    }

    /// The owned handle, for the `wickra_backtest_stream_*` calls that borrow it.
    WickraBacktestStream *get() const noexcept { return ptr_; }

    /// Gives up ownership. Pass this to `wickra_backtest_stream_finish_json`,
    /// which frees the handle itself whatever the outcome.
    WickraBacktestStream *release() noexcept { return std::exchange(ptr_, nullptr); }

    /// True when a handle is held.
    explicit operator bool() const noexcept { return ptr_ != nullptr; }

private:
    WickraBacktestStream *ptr_;
};

}  // namespace backtest
}  // namespace wickra

#endif  // WICKRA_BACKTEST_HPP
