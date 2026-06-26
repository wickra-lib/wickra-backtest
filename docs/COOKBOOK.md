# Cookbook

Complete, runnable strategies. Each lives under
[`examples/strategies/`](../examples/strategies/) and is checked by the
`example_specs` test, so these never drift out of sync with the engine. Run any
of them with:

```bash
wkbt run --data candles.csv --spec examples/strategies/rsi-mean-reversion.json
```

or from any binding by reading the file and passing it as the spec (or, with a
candle stream and feeds, as one `run_json` request bundle).

For the full grammar see the [strategy spec reference](STRATEGY_SPEC.md).

## RSI mean reversion

Buy oversold, sell back to the mean, with a 3% stop.
[`rsi-mean-reversion.json`](../examples/strategies/rsi-mean-reversion.json):

```json
{
  "symbol": "BTCUSDT", "timeframe": "1h",
  "indicators": { "rsi": { "type": "Rsi", "params": [14] } },
  "entry": { "lt": ["rsi", 30] },
  "exit":  { "gt": ["rsi", 50] },
  "sizing": { "type": "fixed_fraction", "fraction": 0.5 },
  "costs": { "taker_bps": 5, "slippage": { "type": "fixed_bps", "bps": 2 } },
  "risk": { "stop_loss_pct": 3.0 }
}
```

## MACD trend, long and short

Go long on a bullish MACD cross, flip short on the bearish cross.
[`macd-trend.json`](../examples/strategies/macd-trend.json):

```json
{
  "symbol": "BTCUSDT", "timeframe": "4h",
  "indicators": { "macd": { "type": "Macd", "params": [12, 26, 9] } },
  "entry":       { "cross_above": ["macd.macd", "macd.signal"] },
  "exit":        { "cross_below": ["macd.macd", "macd.signal"] },
  "short_entry": { "cross_below": ["macd.macd", "macd.signal"] },
  "short_exit":  { "cross_above": ["macd.macd", "macd.signal"] },
  "sizing": { "type": "fixed_fraction", "fraction": 0.95 },
  "costs": { "maker_bps": 2, "taker_bps": 5 }
}
```

## Bollinger breakout with a volatility target

Enter on a close above the upper band, exit back at the middle band, size to a
2% per-bar volatility target and trail the stop.
[`bollinger-breakout.json`](../examples/strategies/bollinger-breakout.json):

```json
{
  "symbol": "ETHUSDT", "timeframe": "1h",
  "indicators": { "bb": { "type": "Bollinger", "params": [20, 2] } },
  "entry": { "gt": [{ "price": "close" }, "bb.upper"] },
  "exit":  { "lt": [{ "price": "close" }, "bb.middle"] },
  "sizing": { "type": "vol_target", "target_vol": 0.02, "lookback": 20 },
  "risk": { "trailing_stop_pct": 4.0, "max_leverage": 3.0 }
}
```

## Donchian channel breakout, risk-sized

Classic turtle-style breakout: buy new highs, exit on new lows, size each trade
to risk 1% of equity against the stop.
[`donchian-breakout.json`](../examples/strategies/donchian-breakout.json):

```json
{
  "symbol": "BTCUSDT", "timeframe": "1d",
  "indicators": { "dc": { "type": "Donchian", "params": [20] } },
  "entry": { "ge": [{ "price": "high" }, "dc.upper"] },
  "exit":  { "le": [{ "price": "low" }, "dc.lower"] },
  "sizing": { "type": "risk_per_trade", "risk_pct": 1.0 },
  "risk": { "stop_loss_pct": 5.0 }
}
```

## Funding carry (perpetuals)

A microstructure strategy: hold when perpetual funding is negative (you get
paid to hold), and charge funding to the position each bar. Needs a derivatives
feed, supplied as `derivs` in a `run_json` request.
[`funding-carry.json`](../examples/strategies/funding-carry.json):

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

## Order-book imbalance

The differentiator: trade on live order-flow. Enter when top-of-book pressure is
strongly bid-heavy, with spread-based slippage. Needs an order-book feed,
supplied as `books` in a `run_json` request.
[`orderbook-imbalance.json`](../examples/strategies/orderbook-imbalance.json):

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

Feed bundles (`derivs`, `books`, `trades`, `sections`, `reference`) are passed
alongside the candles in a single `run_json` request document — see the
[strategy spec reference](STRATEGY_SPEC.md) and the feed golden requests under
[`golden/requests/`](../golden/requests/) for the exact shapes.
