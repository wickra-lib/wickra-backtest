# Strategy spec reference

A Wickra strategy is **data**, not code: a single JSON document describing the
indicators, the entry/exit logic, sizing, costs, risk and execution. The same
document runs identically in Rust, Python, Node.js, WASM, C, C++, C#, Go, Java
and R, and the backtest matches a live run by construction.

A machine-readable JSON Schema is committed at
[`schema/strategy_spec.schema.json`](../schema/strategy_spec.schema.json) and is
printed by `wkbt schema`. This page is the human-readable companion.

## Top level

```jsonc
{
  "spec_version": 1,            // optional, defaults to 1
  "symbol": "BTCUSDT",          // required — metadata, see below
  "ref_symbol": "ETHUSDT",      // optional — metadata naming the pairwise reference
  "timeframe": "1h",            // required — metadata, free-form (e.g. 1m/5m/1h/1d)
  "indicators": { /* name -> indicator */ },
  "entry":  { /* condition */ },     // required — opens a long
  "exit":   { /* condition */ },     // required — closes a long
  "short_entry": { /* condition */ },// optional — opens a short
  "short_exit":  { /* condition */ },// optional — closes a short
  "sizing": { /* sizing */ },        // required
  "costs":  { /* costs */ },         // optional
  "risk":   { /* risk */ },          // optional
  "execution": { /* execution */ },  // optional
  "warmup": 200                      // optional — overrides the auto warmup
}
```

Any indicator name referenced in a condition must be declared in `indicators`,
or the spec is rejected at parse time.

### `symbol`, `ref_symbol` and `timeframe` are metadata

The engine never resolves any of the three. It does not fetch data: the caller
supplies the candles, and for a pairwise indicator the reference series as well,
through `RunRequest.reference` or `run_with_ref`. These fields record what the
spec was written for, so a stored spec is not ambiguous about which instrument
and bar size it belongs to — and, for `ref_symbol`, about which instrument the
caller is expected to pair it against.

Setting `ref_symbol` therefore does **not** make a pairwise indicator work on its
own. The reference series still has to be passed with the request.

## Indicators

```jsonc
"indicators": {
  "ema_fast": { "type": "Ema", "params": [20] },
  "macd":     { "type": "Macd", "params": [12, 26, 9] },
  "imb":      { "type": "OrderBookImbalanceTop1", "params": [], "feed": "orderbook" }
}
```

| Field    | Meaning |
|----------|---------|
| `type`   | The Wickra indicator name (e.g. `Ema`, `Rsi`, `Macd`, `Atr`). |
| `params` | The constructor parameters, in order. Defaults to `[]`. |
| `feed`   | Optional. The input the indicator consumes: `kline`, `trade`, `orderbook`, `trade_quote`, `derivatives` or `cross_section`. Redundant — the `type` already determines it — so its only effect is to be cross-checked: a spec whose declared feed contradicts its indicator is rejected at parse. Omit it and the indicator decides. |

Multi-output indicators expose their fields as `"name.field"` in operands — for
example `macd.macd`, `macd.signal`, `macd.histogram`, `bb.upper`, `bb.middle`,
`bb.lower`, `dc.upper`, `dc.lower`. A bare `"name"` refers to the indicator's
primary field.

## Operands

An **operand** evaluates to a number each bar. It is one of:

| Form | Meaning |
|------|---------|
| `"ema_fast"` | An indicator value (or `"name.field"` for a named field). |
| `70` | A literal constant. |
| `{ "price": "close" }` | A price field: `open`, `high`, `low`, `close`, `volume`, `hlc3`, `ohlc4`. |
| `{ "prev": ["ema_fast", 1] }` | The operand's value `n` bars ago. |
| `{ "add": [a, b] }` | `a + b`. |
| `{ "sub": [a, b] }` | `a - b`. |
| `{ "mul": [a, b] }` | `a * b`. |
| `{ "div": [a, b] }` | `a / b`. |

Arithmetic operands nest, so `{ "sub": ["ema_fast", "ema_slow"] }` is itself an
operand usable anywhere a number is expected.

## Conditions

A **condition** evaluates to true/false each bar:

| Form | Meaning |
|------|---------|
| `{ "gt": [a, b] }` | `a > b` (also `lt`, `ge`, `le`, `eq`, `ne`). |
| `{ "cross_above": [a, b] }` | `a` crossed above `b` this bar. |
| `{ "cross_below": [a, b] }` | `a` crossed below `b` this bar. |
| `{ "between": [a, lo, hi] }` | `lo <= a <= hi`. |
| `{ "rising": [a, n] }` | `a` is greater than its value `n` bars ago. |
| `{ "falling": [a, n] }` | `a` is less than its value `n` bars ago. |
| `{ "all": [c1, c2, …] }` | All sub-conditions true (AND). |
| `{ "any": [c1, c2, …] }` | Any sub-condition true (OR). |
| `{ "not": c }` | Negation. |
| `{ "in_position": true }` | A position is currently open. |
| `{ "bars_since_entry": { "ge": 5 } }` | Bars since entry satisfies the predicate (`gt`/`lt`/`ge`/`le`/`eq`). |

## Sizing

Tagged by `type`:

| Sizing | Meaning |
|--------|---------|
| `{ "type": "fixed_fraction", "fraction": 0.95 }` | A fraction of current equity. |
| `{ "type": "fixed_qty", "qty": 1.0 }` | A fixed quantity of the base asset. |
| `{ "type": "fixed_cash", "cash": 1000 }` | A fixed cash notional. |
| `{ "type": "vol_target", "target_vol": 0.02, "lookback": 20 }` | Scale notional to a target per-bar return volatility. No position until `lookback` bars exist. |
| `{ "type": "risk_per_trade", "risk_pct": 1.0 }` | Size so a stop-loss hit loses `risk_pct` of equity (needs `risk.stop_loss_pct`). |

## Costs

```jsonc
"costs": {
  "maker_bps": 2,                 // fee on resting limit fills
  "taker_bps": 5,                 // fee on market / stop / close fills
  "slippage": { "type": "fixed_bps", "bps": 2 },
  "funding": false                // charge perpetual funding to open positions
}
```

Slippage models (tagged by `type`):

| Slippage | Meaning |
|----------|---------|
| `{ "type": "fixed_bps", "bps": 2 }` | A fixed number of basis points. |
| `{ "type": "spread" }` | The order book's half-spread (needs an order-book feed). |
| `{ "type": "volume_impact", "coef": 0.1 }` | Linear impact `coef * order_qty / bar_volume`. |

`funding` requires a derivatives feed: each bar an open position is charged
`qty * mark_price * funding_rate` (longs pay when the rate is positive, shorts
receive).

## Risk

All fields are optional:

| Field | Meaning |
|-------|---------|
| `stop_loss_pct` | Stop-loss as a percent move against the position. |
| `take_profit_pct` | Take-profit as a percent move in favour. |
| `trailing_stop_pct` | Trailing stop as a percent retrace from the favourable extreme. |
| `max_leverage` | Maximum leverage (default 1× — no leverage). |
| `max_position_pct` | Maximum position as a percent of equity. |
| `liquidation` | Liquidate a leveraged position intrabar at its bankruptcy price (bites only above 1× leverage). |

Stops and targets are evaluated **intrabar** along the conservative O→H→L→C
path: when both a stop and a target could be hit in the same bar, the stop wins.
Fills are **gap-aware** — if a bar opens beyond the level (a gap), the fill is
the open (the worse price for a stop, the better for a take-profit), not the
unreachable level.

## Execution

```jsonc
"execution": {
  "order_type": "market",         // market | limit | stop | stop_limit
  "fill_timing": "next_open",     // next_open (default) | close
  "limit_offset_pct": -0.5,       // required for limit orders
  "stop_offset_pct": 0.5,         // required for stop orders
  "latency_bars": 0,              // extra bars before a fill
  "partial_fills": false,
  "max_participation": 0.1        // required when partial_fills is set
}
```

The default `fill_timing` is `next_open`: a signal computed on a bar's close
fills on the **next** bar's open, which structurally prevents look-ahead bias.
`close` is an opt-in, deliberately optimistic mode (fills on the very close that
produced the signal — not tradeable live) and is restricted to market orders
with no latency.

For limit/stop orders the offset is a percent of the signal bar's close: a
negative `limit_offset_pct` places a long limit below the market (buy the dip),
a positive `stop_offset_pct` places a long stop above it (breakout). A resting
order is good-till-filled.

## Report

`run` / `run_json` return a `BacktestReport`:

```jsonc
{
  "schema_version": 1,
  "symbol": "BTCUSDT",
  "timeframe": "1h",
  "initial_capital": 10000.0,
  "metrics": {
    "pnl": …, "return_pct": …, "sharpe": …, "sortino": …, "calmar": …,
    "max_drawdown": …, "win_rate": …, "profit_factor": …, "num_trades": …
  },
  "trades": [ { "entry_time": …, "exit_time": …, "entry_price": …, "exit_price": …,
               "qty": …, "pnl": …, "return_pct": …, "reason": "…" } ],
  "equity": [ { "time": …, "equity": … } ],
  "fees_paid": …
}
```

`symbol` and `timeframe` are echoed from the spec, so a stored report says what
it is a report of. The engine reads whatever candles it is given and does not
check them against either, so they are labels rather than guarantees.

A trade's `reason` is one of `signal` (an exit rule fired), `stop_loss`,
`take_profit`, `trailing_stop`, `liquidation` or `end` (the position was still
open when the data ran out). A negative `qty` is a short. `equity` is a per-bar
stream, suitable to write as JSON Lines and tail live.

See the [Cookbook](COOKBOOK.md) for complete, runnable strategies.
