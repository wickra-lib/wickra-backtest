# Run the shared EMA-cross strategy from R, both ways.
#
#   Rscript examples/r/backtest.R
#
# Reads the same examples/sample.csv and examples/ema-cross.json every other
# language example uses, runs the whole series at once, then feeds the same bars
# one at a time and checks that the two agree. That equality is the point of the
# library: a live loop is the streaming path with a socket in place of the file,
# so a backtest is not a separate model of the strategy.
#
# Requires the installed binding: R CMD INSTALL bindings/r
library(wickrabacktest)

root <- file.path("examples")
capital <- 10000

# The CSV columns are time,open,high,low,close,volume.
bars <- utils::read.csv(file.path(root, "sample.csv"))

# The spec is passed as text, not re-serialised: round-tripping it through an R
# JSON parser would unbox the length-one parameter arrays the engine expects.
spec_path <- file.path(root, "ema-cross.json")
spec <- readChar(spec_path, file.info(spec_path)$size)

batch <- backtest_run(
  bars$open, bars$high, bars$low, bars$close,
  volume = bars$volume, time = bars$time,
  spec = spec, capital = capital
)

# The same run, driven bar by bar. Replace the loop with reads from a socket and
# this is a live strategy; nothing else about it changes.
live <- backtest_stream_new(spec, capital = capital)
for (i in seq_len(nrow(bars))) {
  backtest_stream_step(live, bars$open[i], bars$high[i], bars$low[i],
                       bars$close[i], bars$volume[i], bars$time[i])
}
streamed <- backtest_stream_finish_json(live)

# Pull a few numbers out without adding a JSON dependency to the example.
number <- function(json, key) {
  hit <- regmatches(json, regexpr(paste0('"', key, '":-?[0-9.eE+]+'), json))
  as.numeric(sub(paste0('"', key, '":'), "", hit))
}
cat(sprintf("bars            %d\n", nrow(bars)))
cat(sprintf("trades          %d\n", number(streamed, "num_trades")))
cat(sprintf("pnl             %.2f\n", number(streamed, "pnl")))
cat(sprintf("return %%        %.2f\n", number(streamed, "return_pct")))
cat(sprintf("max drawdown    %.4f\n", number(streamed, "max_drawdown")))

if (!identical(streamed, batch)) {
  stop("streaming and batch disagree -- that should be impossible")
}
cat("\nstreaming reproduces the batch report exactly\n")
