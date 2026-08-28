# Throughput benchmark for the wickra-backtest R binding.
#
# Measures what crossing the boundary costs. Every reach in this repository runs
# the same Rust engine, so a difference between two bindings is not a difference
# in the backtester -- it is the price of that language's FFI, paid once per bar
# on the streaming path and once per run on the batch path.
#
# R is the boundary where that difference is largest and most worth knowing: the
# streaming loop is an interpreted `for` over .Call, while the batch path hands
# over six numeric vectors and returns once.
#
# The strategy is examples/ema-cross.json: two EMAs, a crossover, fractional
# sizing, taker costs, slippage and a trailing stop.
#
# Run after installing the binding (R CMD INSTALL bindings/r):
#
#   Rscript bindings/r/benchmarks/throughput.R                 # 200k bars
#   Rscript bindings/r/benchmarks/throughput.R --bars 1000000

library(wickrabacktest)

spec <- paste0(
  '{"symbol":"BTCUSDT","timeframe":"1h",',
  '"indicators":{"ema_fast":{"type":"Ema","params":[5]},',
  '"ema_slow":{"type":"Ema","params":[15]}},',
  '"entry":{"cross_above":["ema_fast","ema_slow"]},',
  '"exit":{"cross_below":["ema_fast","ema_slow"]},',
  '"sizing":{"type":"fixed_fraction","fraction":0.95},',
  '"costs":{"taker_bps":5,"slippage":{"type":"fixed_bps","bps":2}},',
  '"risk":{"trailing_stop_pct":5.0}}'
)
capital <- 10000

parse_bars <- function() {
  args <- commandArgs(trailingOnly = TRUE)
  hit <- match("--bars", args)
  if (is.na(hit) || length(args) < hit + 1L) {
    return(200000L)
  }
  n <- suppressWarnings(as.integer(args[hit + 1L]))
  if (is.na(n) || n < 1000L) {
    stop("--bars must be an integer >= 1000")
  }
  n
}

bars <- parse_bars()

# Deterministic synthetic OHLCV, from the same formula every binding's harness
# uses. Vectorised, so building it does not dominate the measurement.
i <- seq_len(bars) - 1L
mid <- 100 + sin(i * 0.001) * 20 + i * 1e-4
close <- mid + sin(i * 0.05) * 2
open <- c(close[1], close[-bars])
high <- pmax(open, close) + 1.5
low <- pmin(open, close) - 1.5
volume <- 1000 + (i %% 97) * 13
time <- as.numeric(i)

# Seconds for one run, measured by repeating until the clock has something to
# say. R's timers are coarse on some platforms -- a fast batch run over a small
# series measured as exactly zero and reported an infinite rate -- so the run is
# repeated until the batch takes at least a fifth of a second, and the total is
# divided by the count.
median_seconds <- function(fn, reps = 3L, floor_seconds = 0.2) {
  fn() # warmup
  samples <- vapply(seq_len(reps), function(...) {
    count <- 1L
    repeat {
      start <- Sys.time()
      for (unused in seq_len(count)) fn()
      elapsed <- as.numeric(Sys.time() - start, units = "secs")
      if (elapsed >= floor_seconds) {
        return(elapsed / count)
      }
      count <- count * 2L
    }
  }, numeric(1))
  stats::median(samples)
}

streaming <- function() {
  bt <- backtest_stream_new(spec, capital = capital)
  for (k in seq_len(bars)) {
    backtest_stream_step(bt, open[k], high[k], low[k], close[k], volume[k], time[k])
  }
  invisible(backtest_stream_finish_json(bt))
}

batch <- function() {
  invisible(backtest_run(open, high, low, close,
                         volume = volume, time = time,
                         spec = spec, capital = capital))
}

streaming_s <- median_seconds(streaming)
batch_s <- median_seconds(batch)

cat(sprintf("wickra-backtest R throughput — %s bars (median of 3 runs)\n\n",
            format(bars, big.mark = ",")))
cat(sprintf("%-14s%16s%12s\n", "path", "bars/sec", "ns/bar"))
cat(strrep("-", 42), "\n", sep = "")
for (row in list(list("streaming", streaming_s), list("batch", batch_s))) {
  cat(sprintf("%-14s%16s%12s\n", row[[1]],
              format(round(bars / row[[2]]), big.mark = ","),
              format(round(row[[2]] / bars * 1e9), big.mark = ",")))
}
cat(paste0(
  "\nStreaming crosses the boundary once per bar, from an interpreted loop.\n",
  "Batch crosses it once per run and hands over six numeric vectors, so the\n",
  "gap here is R's per-call overhead rather than anything about the engine.\n",
  "Machine-dependent — compare bindings on one machine, not across machines.\n"
))
