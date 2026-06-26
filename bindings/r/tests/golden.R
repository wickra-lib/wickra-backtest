# Golden parity for the R binding: assert its output against the shared golden
# reports (golden/expected/). The R binding returns the engine JSON verbatim, so
# the match is byte-for-byte. Run from the repo root:
#   Rscript bindings/r/tests/golden.R
library(wickrabacktest)
library(jsonlite)

# Extract the raw JSON text of the "spec" object from a case file. We do not
# round-trip the spec through jsonlite (that would unbox length-1 param arrays
# and turn `{}` into `[]`); the numeric arrays simplify cleanly, the spec does
# not, so we brace-match its original text instead.
extract_spec <- function(raw) {
  i <- regexpr('"spec"', raw)[1]
  s <- substr(raw, i, nchar(raw))
  b <- regexpr("\\{", s)[1]
  s <- substring(s, b)
  depth <- 0L
  end <- 0L
  chars <- strsplit(s, "")[[1]]
  for (k in seq_along(chars)) {
    if (chars[k] == "{") {
      depth <- depth + 1L
    } else if (chars[k] == "}") {
      depth <- depth - 1L
      if (depth == 0L) {
        end <- k
        break
      }
    }
  }
  substr(s, 1, end)
}

golden <- "golden"
cases <- list.files(file.path(golden, "cases"), pattern = "\\.json$", full.names = TRUE)
stopifnot(length(cases) > 0)

for (path in cases) {
  raw <- readChar(path, file.info(path)$size)
  case <- jsonlite::fromJSON(raw, simplifyVector = TRUE)
  spec_json <- extract_spec(raw)
  got <- backtest_run(
    case$open, case$high, case$low, case$close,
    volume = case$volume, time = case$time,
    spec = spec_json, capital = case$capital
  )
  exp_path <- file.path(golden, "expected", paste0(case$name, ".json"))
  want <- trimws(readChar(exp_path, file.info(exp_path)$size))
  if (!identical(got, want)) {
    cat("MISMATCH", case$name, "\n got: ", got, "\nwant: ", want, "\n")
    quit(status = 1)
  }
}
cat("R golden parity: all", length(cases), "cases match\n")
