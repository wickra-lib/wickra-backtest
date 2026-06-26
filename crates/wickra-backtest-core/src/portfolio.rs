//! Cash/position accounting for one signed position (long `qty > 0` or short
//! `qty < 0`).
//!
//! Fills are charged a taker fee on notional and slippage is applied to the fill
//! price by the engine before it calls [`Portfolio::enter`] / [`Portfolio::exit`].
//! The signed-quantity convention makes the same `enter`/`exit` cash maths work
//! for both sides: `PnL` is always `qty * (exit − entry) − fees`.

use serde::Serialize;

/// A completed round-trip trade.
#[derive(Debug, Clone, Serialize)]
pub struct Trade {
    /// Entry bar time.
    pub entry_time: i64,
    /// Exit bar time.
    pub exit_time: i64,
    /// Entry fill price.
    pub entry_price: f64,
    /// Exit fill price.
    pub exit_price: f64,
    /// Position quantity.
    pub qty: f64,
    /// Realised `PnL` net of fees.
    pub pnl: f64,
    /// Return on the entry notional, in percent.
    pub return_pct: f64,
    /// Why the position was closed (`signal` / `stop_loss` / `take_profit` / `end`).
    pub reason: String,
}

/// Long-only portfolio: cash plus at most one open position.
#[derive(Debug, Clone)]
pub struct Portfolio {
    /// Free cash.
    pub cash: f64,
    /// Open quantity (`0.0` when flat).
    pub qty: f64,
    /// Fill price of the open position.
    pub entry_price: f64,
    /// Entry time of the open position.
    pub entry_time: i64,
    /// Fees paid on entry of the open position (carried so exit can net them).
    entry_fee: f64,
    /// Completed trades.
    pub trades: Vec<Trade>,
    /// Total fees paid across the run.
    pub fees_paid: f64,
}

impl Portfolio {
    /// Create a portfolio with `cash` of starting capital.
    pub fn new(cash: f64) -> Self {
        Self {
            cash,
            qty: 0.0,
            entry_price: 0.0,
            entry_time: 0,
            entry_fee: 0.0,
            trades: Vec::new(),
            fees_paid: 0.0,
        }
    }

    /// Whether a position is open (long `qty > 0` or short `qty < 0`).
    pub fn in_position(&self) -> bool {
        self.qty.abs() > f64::EPSILON
    }

    /// `true` for a long position, `false` for a short.
    pub fn is_long(&self) -> bool {
        self.qty > 0.0
    }

    /// Mark-to-market equity at `mark` price.
    pub fn equity(&self, mark: f64) -> f64 {
        self.cash + self.qty * mark
    }

    /// Apply a funding `payment` to cash (positive = paid out, e.g. a long
    /// paying positive funding; negative = received). Also recorded in
    /// `fees_paid` so the report's total cost reflects funding.
    pub fn apply_funding(&mut self, payment: f64) {
        self.cash -= payment;
        self.fees_paid += payment;
    }

    /// Open a long of `qty` at `price`, paying `fee`.
    pub fn enter(&mut self, qty: f64, price: f64, time: i64, fee: f64) {
        self.cash -= qty * price + fee;
        self.qty = qty;
        self.entry_price = price;
        self.entry_time = time;
        self.entry_fee = fee;
        self.fees_paid += fee;
    }

    /// Close the open position at `price`, paying `fee`, recording a [`Trade`].
    pub fn exit(&mut self, price: f64, time: i64, fee: f64, reason: &str) {
        let qty = self.qty;
        self.cash += qty * price - fee;
        self.fees_paid += fee;
        let notional = qty.abs() * self.entry_price;
        let pnl = qty * (price - self.entry_price) - self.entry_fee - fee;
        let return_pct = if notional.abs() < f64::EPSILON {
            0.0
        } else {
            pnl / notional * 100.0
        };
        self.trades.push(Trade {
            entry_time: self.entry_time,
            exit_time: time,
            entry_price: self.entry_price,
            exit_price: price,
            qty,
            pnl,
            return_pct,
            reason: reason.to_string(),
        });
        self.qty = 0.0;
        self.entry_fee = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_pnl_nets_fees() {
        let mut pf = Portfolio::new(1000.0);
        pf.enter(10.0, 10.0, 1, 1.0); // buy 10 @ 10, fee 1 -> cash 1000-100-1=899
        assert!(pf.in_position());
        assert!((pf.cash - 899.0).abs() < 1e-9);
        pf.exit(12.0, 2, 1.0, "signal"); // sell 10 @ 12, fee 1 -> cash 899+120-1=1018
        assert!(!pf.in_position());
        assert!((pf.cash - 1018.0).abs() < 1e-9);
        let t = &pf.trades[0];
        // pnl = 10*(12-10) - 1 - 1 = 18
        assert!((t.pnl - 18.0).abs() < 1e-9);
        assert!((pf.fees_paid - 2.0).abs() < 1e-9);
    }

    #[test]
    fn short_round_trip_profits_when_price_falls() {
        let mut pf = Portfolio::new(1000.0);
        pf.enter(-10.0, 10.0, 1, 0.0); // short 10 @ 10 -> receive 100 -> cash 1100
        assert!(pf.in_position());
        assert!(!pf.is_long());
        assert!((pf.cash - 1100.0).abs() < 1e-9);
        pf.exit(8.0, 2, 0.0, "signal"); // buy back 10 @ 8 -> pay 80 -> cash 1020
        assert!((pf.cash - 1020.0).abs() < 1e-9);
        let t = &pf.trades[0];
        // pnl = -10*(8-10) = 20
        assert!((t.pnl - 20.0).abs() < 1e-9);
        assert!((t.return_pct - 20.0).abs() < 1e-9); // 20 / (10*10) * 100
    }
}
