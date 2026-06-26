# Microstructure backtesting

The differentiator: Wickra-backtest can replay the **order book, trades,
perpetual funding and market breadth** as strategy inputs, not just OHLCV bars.
Off-the-shelf backtesters can't — they only see candles. Because the indicators
are the same `wickra-core` kernels that run live, a microstructure backtest is
value-identical to a live run.

A microstructure strategy is still just a [strategy spec](STRATEGY_SPEC.md): you
declare an indicator with the appropriate `feed`, reference it in your rules, and
supply the matching per-bar feed alongside the candles. The uniform way to pass
feeds is a single [`run_json`](STRATEGY_SPEC.md) request bundle.

## The feed families

| Feed | `run_json` field | Unlocks (examples) |
|------|------------------|--------------------|
| Derivatives ticks | `derivs` | `FundingRate`, `OpenInterestDelta`, `LongShortRatio`, `TakerBuySellRatio`, `PerpetualPremium` |
| Order book | `books` | `OrderBookImbalanceTop1`/`TopN`/`Full`, `Microprice`, `QuotedSpread`, `DepthSlope`, `OrderFlowImbalance` |
| Trades | `trades` | `CumulativeVolumeDelta`, `TradeImbalance`, `SignedVolume`, `Vpin`, `AmihudIlliquidity`, `RollMeasure` |
| Trade + quote | `trades` + `books` | `EffectiveSpread`, `RealizedSpread`, `KylesLambda` |
| Cross-section (breadth) | `sections` | `AdvanceDecline`, `McClellan`, `Trin`, `BreadthThrust`, `NewHighsNewLows` |
| Reference series (pairwise) | `reference` | `PearsonCorrelation`, `Beta`, `SpreadZScore`, `Cointegration` |

Each present feed must have the **same length as `candles`** (one entry per bar).
Absent feeds are simply omitted. The full backtestable catalogue is 495
indicators across every family.

## The request bundle

`run_json` takes one JSON document: the spec, the candles, the capital and any
feeds. The feeds are per-bar arrays parallel to `candles`:

```jsonc
{
  "capital": 10000,
  "spec":    { /* a normal strategy spec */ },
  "candles": [ { "time": 0, "open": …, "high": …, "low": …, "close": …, "volume": … }, … ],
  "derivs":   [ /* one DerivativesTick per bar */ ],
  "books":    [ /* one OrderBook per bar */ ],
  "trades":   [ [ /* the TradePrints in bar 0 */ ], [ /* bar 1 */ ], … ],
  "sections": [ /* one CrossSection per bar */ ],
  "reference":[ /* a parallel candle series for pairwise indicators */ ]
}
```

### Feed shapes

```jsonc
// DerivativesTick (derivs)
{ "funding_rate": 0.01, "mark_price": 100, "index_price": 100, "futures_price": 100,
  "open_interest": 1000, "long_size": 600, "short_size": 400,
  "taker_buy_volume": 50, "taker_sell_volume": 40,
  "long_liquidation": 0, "short_liquidation": 0 }

// OrderBook (books) — price levels, best first
{ "bids": [ { "price": 100.0, "size": 9.0 } ],
  "asks": [ { "price": 101.0, "size": 1.0 } ] }

// TradePrint (each element of a trades[bar] list) — side is "buy" or "sell"
{ "price": 100.0, "size": 5.0, "side": "buy", "timestamp": 0 }

// CrossSection (sections) — one panel of members per bar
{ "members": [ { "change": 1.0, "volume": 100.0, "new_high": false, "new_low": false }, … ] }
```

## Worked examples

These are the validated parity cases under
[`golden/requests/`](../golden/requests/) — each drives one feed path and is
asserted byte-for-byte across all ten language bindings.

### Perpetual funding carry — `derivs`

Hold while funding is favourable and charge it to the position each bar
(`costs.funding`):

```json
{
  "symbol": "BTCUSDT", "timeframe": "1h",
  "indicators": { "fr": { "type": "FundingRate", "params": [] } },
  "entry": { "lt": ["fr", 0.0] },
  "exit":  { "gt": ["fr", 0.0] },
  "sizing": { "type": "fixed_fraction", "fraction": 0.5 },
  "costs": { "taker_bps": 5, "funding": true }
}
```

See [`funding_long.json`](../golden/requests/funding_long.json) for the full
bundle including the `derivs` feed, and the [funding carry
cookbook](COOKBOOK.md#funding-carry-perpetuals) entry.

### Order-book imbalance — `books`

Trade on top-of-book pressure, with spread-based slippage:

```json
{
  "symbol": "BTCUSDT", "timeframe": "1m",
  "indicators": { "imb": { "type": "OrderBookImbalanceTop1", "params": [], "feed": "orderbook" } },
  "entry": { "gt": ["imb", 0.5] },
  "exit":  { "lt": ["imb", 0.0] },
  "sizing": { "type": "fixed_qty", "qty": 0.1 },
  "costs": { "taker_bps": 5, "slippage": { "type": "spread" } }
}
```

The `spread` slippage model reads the live order book, so the fill cost reflects
the actual book at the time. See
[`orderbook_imbalance.json`](../golden/requests/orderbook_imbalance.json).

### Trade-flow (cumulative volume delta) — `trades`

Each bar carries the list of trades that printed in it; trade-flow indicators
replay them:

```json
{
  "symbol": "BTCUSDT", "timeframe": "1m",
  "indicators": { "cvd": { "type": "CumulativeVolumeDelta", "params": [], "feed": "trade" } },
  "entry": { "gt": ["cvd", 0.0] },
  "exit":  { "lt": ["cvd", -1.0] },
  "sizing": { "type": "fixed_qty", "qty": 1 }
}
```

See [`trade_cvd.json`](../golden/requests/trade_cvd.json).

### Market breadth — `sections`

Cross-sectional indicators take a per-bar panel of the universe's members:

```json
{
  "symbol": "BTCUSDT", "timeframe": "1h",
  "indicators": { "ad": { "type": "AdvanceDecline", "params": [] } },
  "entry": { "gt": ["ad", 0.0] },
  "exit":  { "lt": ["ad", -100.0] },
  "sizing": { "type": "fixed_qty", "qty": 1 }
}
```

See [`cross_section_breadth.json`](../golden/requests/cross_section_breadth.json).

## Notes and limitations

- **Each feed is per bar.** Aggregate intra-bar order books / trades into one
  snapshot or list per candle before passing them.
- **`new_high` / `new_low` are the only settable breadth flags.** `wickra-core`'s
  cross-section `Member` is `#[non_exhaustive]`, so breadth indicators that need
  `above_ma` or `on_buy_signal` (`PercentAboveMa`, `BullishPercentIndex`) see
  those flags as `false` and will not signal as intended; the advance-decline,
  new-high/new-low and TRIN-style indicators are fully driven.
- **Pairwise indicators need the `reference` series**, a parallel candle series
  whose close is the second input; without it they yield nothing.
- The native Rust API also exposes `run_with_deriv`, `run_with_orderbook`,
  `run_with_trades`, `run_with_cross_section` and `run_with_ref` for a single
  feed, and `StreamingBacktest::step_with_feeds` for the live path.
