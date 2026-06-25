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
