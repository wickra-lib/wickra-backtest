# The streaming reach must be the same engine as backtest_run(), one bar at a
# time -- that equivalence is the claim, so it is what this file pins.
library(wickrabacktest)

open  <- c(100, 102, 104, 98)
high  <- c(101, 103, 104, 98)
low   <- c(100, 102, 99, 97)
close <- c(101, 103, 99, 97)
price_spec <- paste0(
  '{"symbol":"x","timeframe":"1h","indicators":{},',
  '"entry":{"gt":[{"price":"close"},100]},',
  '"exit":{"lt":[{"price":"close"},100]},',
  '"sizing":{"type":"fixed_qty","qty":1}}'
)
batch <- backtest_run(open, high, low, close, spec = price_spec, capital = 1000)

# Streaming reproduces the batch report, byte for byte.
bt <- backtest_stream_new(price_spec, capital = 1000)
for (i in seq_along(open)) {
  backtest_stream_step(bt, open[i], high[i], low[i], close[i])
}
stopifnot(identical(backtest_stream_finish_json(bt), batch))

# The document form is a drop-in for the scalar one.
bt <- backtest_stream_new(price_spec, capital = 1000)
for (i in seq_along(open)) {
  backtest_stream_step_json(bt, sprintf(
    '{"candle":{"time":%d,"open":%g,"high":%g,"low":%g,"close":%g,"volume":0}}',
    i - 1L, open[i], high[i], low[i], close[i]))
}
stopifnot(identical(backtest_stream_finish_json(bt), batch))

# The accessors track the run.
bt <- backtest_stream_new(price_spec, capital = 1000)
stopifnot(identical(backtest_stream_latest_equity_json(bt), "null"))
stopifnot(identical(backtest_stream_equity_json(bt), "[]"))
stopifnot(backtest_stream_num_trades(bt) == 0)
for (i in 1:3) {
  backtest_stream_step(bt, open[i], high[i], low[i], close[i])
}
curve <- backtest_stream_equity_json(bt)
stopifnot(lengths(regmatches(curve, gregexpr('"time"', curve, fixed = TRUE))) == 3)
# Bar 3 closed below 100, which is the exit *signal*; the fill lands on the next
# bar's open, so nothing has closed yet.
stopifnot(backtest_stream_num_trades(bt) == 0)
backtest_stream_step(bt, open[4], high[4], low[4], close[4])
stopifnot(backtest_stream_num_trades(bt) == 1)

# Timestamps default to the bar index, as backtest_run() defaults them to 0..n-1.
stopifnot(grepl('"time":0', backtest_stream_equity_json(bt), fixed = TRUE))
stopifnot(grepl('"time":3', backtest_stream_equity_json(bt), fixed = TRUE))
backtest_stream_free(bt)

# A finished run refuses further use, from every entry point.
bt <- backtest_stream_new(price_spec, capital = 1000)
backtest_stream_step(bt, open[1], high[1], low[1], close[1])
invisible(backtest_stream_finish_json(bt))
raises <- function(expr) {
  tryCatch({
    force(expr)
    FALSE
  }, error = function(e) TRUE)
}
stopifnot(raises(backtest_stream_step(bt, 1, 1, 1, 1)))
stopifnot(raises(backtest_stream_step_json(bt, "{}")))
stopifnot(raises(backtest_stream_equity_json(bt)))
stopifnot(raises(backtest_stream_latest_equity_json(bt)))
stopifnot(raises(backtest_stream_num_trades(bt)))
stopifnot(raises(backtest_stream_finish_json(bt)))
# Freeing after finish is a no-op, not a double free.
backtest_stream_free(bt)
backtest_stream_free(bt)

# An invalid spec raises rather than returning a broken handle.
stopifnot(raises(backtest_stream_new('{"bad":true}')))

# A value that is not a handle is rejected before it reaches the C layer.
stopifnot(raises(backtest_stream_equity_json(42)))

# Per-bar feeds reach a reference-reading strategy.
#
# A pairwise indicator is undefined without its reference series, so a spec that
# reads one proves the feed actually arrives -- and it must agree with the batch
# path fed the same reference. The path is a sine, not a geometric one: constant
# growth means constant log returns, which drives the correlation's variance to
# zero and makes the indicator report nothing at all.
n <- 24
closes <- 100 + 10 * sin(seq_len(n) * 0.5 - 0.5)
reference <- 2 * closes
pair_spec <- paste0(
  '{"symbol":"x","timeframe":"1h",',
  '"indicators":{"corr":{"type":"PearsonCorrelation","params":[5]}},',
  '"entry":{"gt":["corr",0.5]},"exit":{"lt":["corr",-0.5]},',
  '"sizing":{"type":"fixed_qty","qty":1}}'
)
candle_json <- function(i, o, h, l, cl) {
  sprintf('{"time":%d,"open":%.17g,"high":%.17g,"low":%.17g,"close":%.17g,"volume":0}',
          i - 1L, o, h, l, cl)
}

bt <- backtest_stream_new(pair_spec, capital = 1000)
candles <- character(n)
refs <- character(n)
for (i in seq_len(n)) {
  cl <- closes[i]
  candles[i] <- candle_json(i, cl, cl + 1, cl - 1, cl)
  refs[i] <- candle_json(i, reference[i], reference[i], reference[i], reference[i])
  backtest_stream_step_json(bt, sprintf(
    '{"candle":%s,"feeds":{"reference":%.17g}}', candles[i], reference[i]))
}
streamed <- backtest_stream_finish_json(bt)

batch_fed <- backtest_run_json(sprintf(
  '{"spec":%s,"capital":1000,"candles":[%s],"reference":[%s]}',
  pair_spec, paste(candles, collapse = ","), paste(refs, collapse = ",")))
stopifnot(identical(streamed, batch_fed))
stopifnot(grepl('"num_trades":1', streamed, fixed = TRUE))

# The feed is load-bearing: without it the correlation never resolves, so the
# strategy never fires and the two runs cannot agree.
blind_run <- backtest_stream_new(pair_spec, capital = 1000)
for (i in seq_len(n)) {
  cl <- closes[i]
  backtest_stream_step(blind_run, cl, cl + 1, cl - 1, cl)
}
blind <- backtest_stream_finish_json(blind_run)
stopifnot(grepl('"num_trades":0', blind, fixed = TRUE))
stopifnot(!identical(blind, streamed))

cat("R binding: streaming checks passed\n")
