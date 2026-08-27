module github.com/wickra-lib/wickra-backtest-examples

go 1.23

require github.com/wickra-lib/wickra-backtest-go v0.0.0

// The example builds against the binding in this repository, not a published
// version of it, so the two always move together.
replace github.com/wickra-lib/wickra-backtest-go => ../../bindings/go
