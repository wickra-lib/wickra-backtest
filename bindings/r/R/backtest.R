#' wickrabacktest: the Wickra backtest engine in R
#'
#' A streaming-native, event-driven backtester. A strategy is a JSON spec rather
#' than code, so the same spec produces the same report here as from every other
#' Wickra language binding, and a backtest and a live loop are one code path.
#'
#' Two ways in: [backtest_run()] and [backtest_run_json()] answer a whole series
#' at once, while [backtest_stream_new()] and its companions drive the same
#' engine one bar at a time.
#'
#' @keywords internal
#' @useDynLib wickrabacktest, .registration = TRUE
"_PACKAGE"

#' The native wickra-backtest library version.
#'
#' @return A length-one character vector.
#' @export
backtest_version <- function() {
  .Call("wkbt_version", PACKAGE = "wickrabacktest")
}

#' Run a strategy spec over OHLCV data.
#'
#' Calls the wickra-backtest engine through its C ABI and returns the report as
#' a JSON string, byte-identical to the other language bindings.
#'
#' @param open,high,low,close Numeric price vectors of equal length.
#' @param volume Optional numeric volume vector (defaults to zeros).
#' @param time Optional numeric timestamp vector (defaults to 0..n-1).
#' @param spec The strategy spec as a JSON string.
#' @param capital Starting capital (default 10000).
#' @return The backtest report as a JSON string.
#' @export
backtest_run <- function(open, high, low, close, volume = NULL, time = NULL,
                         spec, capital = 10000) {
  n <- length(open)
  if (length(high) != n || length(low) != n || length(close) != n) {
    stop("OHLC vectors must have equal length")
  }
  if (is.null(volume)) volume <- numeric(n)
  if (is.null(time)) time <- seq_len(n) - 1
  if (length(volume) != n || length(time) != n) {
    stop("volume and time length must match OHLC")
  }
  res <- .Call("wkbt_run",
               as.double(open), as.double(high), as.double(low),
               as.double(close), as.double(volume), as.double(time),
               as.character(spec)[1], as.double(capital)[1],
               PACKAGE = "wickrabacktest")
  code <- res[[1]]
  json <- res[[2]]
  if (code != 0) {
    stop(sprintf("wickra_backtest_run failed (code %d): %s", code, json))
  }
  json
}

#' Run a backtest from a single request bundle.
#'
#' The request is a JSON document carrying the candles, the spec, the starting
#' capital and any optional feeds. Returns the report as a JSON string,
#' byte-identical to the other language bindings.
#'
#' @param request The request bundle as a JSON string.
#' @return The backtest report as a JSON string.
#' @export
backtest_run_json <- function(request) {
  res <- .Call("wkbt_run_json", as.character(request)[1],
               PACKAGE = "wickrabacktest")
  code <- res[[1]]
  json <- res[[2]]
  if (code != 0) {
    stop(sprintf("wickra_backtest_run_json failed (code %d): %s", code, json))
  }
  json
}

# --- streaming ---------------------------------------------------------------
#
# backtest_run() needs the whole series up front. These drive the same engine one
# bar at a time, so a live loop and a backtest are the same code path: feed them
# from a socket instead of from a vector and every value they report was produced
# the way the backtest produced it.
#
# The handle is an environment rather than a bare externalptr so the bar counter
# can advance in place, which is what lets `time` default to the bar index the
# way backtest_run() defaults it to 0..n-1. The native pointer inside carries a
# finalizer, so a run dropped without backtest_stream_finish_json() or
# backtest_stream_free() still releases its Rust-side memory.

wkbt_check <- function(res, what) {
  if (res[[1]] != 0) {
    stop(sprintf("%s failed (code %d): %s", what, res[[1]], res[[2]]))
  }
  res[[2]]
}

wkbt_ptr <- function(bt) {
  if (!inherits(bt, "wickra_stream")) {
    stop("expected a streaming backtest from backtest_stream_new()")
  }
  bt$ptr
}

#' Start a streaming backtest.
#'
#' @param spec The strategy spec as a JSON string.
#' @param capital Starting capital (default 10000).
#' @return A `wickra_stream` handle.
#' @export
backtest_stream_new <- function(spec, capital = 10000) {
  res <- .Call("wkbt_stream_new", as.character(spec)[1], as.double(capital)[1],
               PACKAGE = "wickrabacktest")
  ptr <- wkbt_check(res, "wickra_backtest_stream_new")
  bt <- new.env(parent = emptyenv())
  bt$ptr <- ptr
  bt$bars <- 0
  class(bt) <- "wickra_stream"
  bt
}

#' Advance a streaming backtest by one OHLCV bar.
#'
#' @param bt A `wickra_stream` handle.
#' @param open,high,low,close Numeric bar prices.
#' @param volume Bar volume (default 0).
#' @param time Bar timestamp; defaults to the number of bars fed so far.
#' @return The handle, invisibly.
#' @export
backtest_stream_step <- function(bt, open, high, low, close, volume = 0,
                                 time = NULL) {
  ptr <- wkbt_ptr(bt)
  if (is.null(time)) time <- bt$bars
  res <- .Call("wkbt_stream_step", ptr,
               as.double(open)[1], as.double(high)[1], as.double(low)[1],
               as.double(close)[1], as.double(volume)[1], as.double(time)[1],
               PACKAGE = "wickrabacktest")
  wkbt_check(res, "wickra_backtest_stream_step")
  bt$bars <- bt$bars + 1
  invisible(bt)
}

#' Advance a streaming backtest by one bar given as a request document.
#'
#' The document is `{"candle": {...}, "feeds": {...}}`, where `feeds` optionally
#' carries this bar's reference, derivatives, order-book, trade or cross-section
#' input. This is the only form that can drive a strategy reading a side feed.
#'
#' @param bt A `wickra_stream` handle.
#' @param step The step document as a JSON string.
#' @return The handle, invisibly.
#' @export
backtest_stream_step_json <- function(bt, step) {
  ptr <- wkbt_ptr(bt)
  res <- .Call("wkbt_stream_step_json", ptr, as.character(step)[1],
               PACKAGE = "wickrabacktest")
  wkbt_check(res, "wickra_backtest_stream_step_json")
  bt$bars <- bt$bars + 1
  invisible(bt)
}

#' The equity curve of a streaming backtest so far, as a JSON array.
#'
#' @param bt A `wickra_stream` handle.
#' @return A length-one character vector.
#' @export
backtest_stream_equity_json <- function(bt) {
  res <- .Call("wkbt_stream_equity_json", wkbt_ptr(bt), PACKAGE = "wickrabacktest")
  wkbt_check(res, "wickra_backtest_stream_equity_json")
}

#' The most recent equity point of a streaming backtest.
#'
#' @param bt A `wickra_stream` handle.
#' @return A length-one character vector; the JSON literal `null` before the
#'   first bar.
#' @export
backtest_stream_latest_equity_json <- function(bt) {
  res <- .Call("wkbt_stream_latest_equity_json", wkbt_ptr(bt),
               PACKAGE = "wickrabacktest")
  wkbt_check(res, "wickra_backtest_stream_latest_equity_json")
}

#' The number of closed trades in a streaming backtest so far.
#'
#' @param bt A `wickra_stream` handle.
#' @return A length-one numeric vector.
#' @export
backtest_stream_num_trades <- function(bt) {
  res <- .Call("wkbt_stream_num_trades", wkbt_ptr(bt), PACKAGE = "wickrabacktest")
  wkbt_check(res, "wickra_backtest_stream_num_trades")
}

#' Close any open position and return the streaming backtest report.
#'
#' Ends the run: the handle is released, and further use raises an error.
#'
#' @param bt A `wickra_stream` handle.
#' @return The backtest report as a JSON string.
#' @export
backtest_stream_finish_json <- function(bt) {
  res <- .Call("wkbt_stream_finish_json", wkbt_ptr(bt), PACKAGE = "wickrabacktest")
  wkbt_check(res, "wickra_backtest_stream_finish_json")
}

#' Release a streaming backtest without producing a report.
#'
#' Idempotent, so it is safe to call alongside `backtest_stream_finish_json()`.
#'
#' @param bt A `wickra_stream` handle.
#' @return `NULL`, invisibly.
#' @export
backtest_stream_free <- function(bt) {
  .Call("wkbt_stream_free", wkbt_ptr(bt), PACKAGE = "wickrabacktest")
  invisible(NULL)
}
