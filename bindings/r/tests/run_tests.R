# Self-contained test runner (no testthat dependency): exits non-zero on failure.
library(wickrabacktest)

stopifnot(nchar(backtest_version()) > 0)

open  <- c(100, 102, 104, 98)
high  <- c(101, 103, 104, 98)
low   <- c(100, 102, 99, 97)
close <- c(101, 103, 99, 97)
spec <- paste0(
  '{"symbol":"x","timeframe":"1h","indicators":{},',
  '"entry":{"gt":[{"price":"close"},100]},',
  '"exit":{"lt":[{"price":"close"},100]},',
  '"sizing":{"type":"fixed_qty","qty":1}}'
)

json <- backtest_run(open, high, low, close, time = c(0, 1, 2, 3),
                     spec = spec, capital = 1000)

# Hand-computed round trip, byte-identical to the other bindings.
stopifnot(grepl('"num_trades":1', json, fixed = TRUE))
stopifnot(grepl('"entry_price":102.0', json, fixed = TRUE))
stopifnot(grepl('"exit_price":98.0', json, fixed = TRUE))
stopifnot(grepl('"pnl":-4.0', json, fixed = TRUE))
stopifnot(grepl('"equity":996.0', json, fixed = TRUE))

# An invalid spec must raise an error.
err <- tryCatch({
  backtest_run(1, 1, 1, 1, spec = '{"bad":true}')
  "no-error"
}, error = function(e) "error")
stopifnot(identical(err, "error"))

# The unified request bundle matches the array-based run by construction.
request <- paste0(
  '{"capital":1000,"spec":', spec, ',"candles":[',
  '{"time":0,"open":100,"high":101,"low":100,"close":101},',
  '{"time":1,"open":102,"high":103,"low":102,"close":103},',
  '{"time":2,"open":104,"high":104,"low":99,"close":99},',
  '{"time":3,"open":98,"high":98,"low":97,"close":97}]}'
)
json2 <- backtest_run_json(request)
stopifnot(grepl('"num_trades":1', json2, fixed = TRUE))
stopifnot(grepl('"entry_price":102.0', json2, fixed = TRUE))

cat("R binding: all checks passed\n")
