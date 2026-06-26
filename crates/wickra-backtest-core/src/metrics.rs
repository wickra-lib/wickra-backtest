//! Performance metrics computed from the equity curve and the trade log.

use serde::Serialize;

use crate::portfolio::Trade;

/// Summary performance metrics.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Metrics {
    /// Absolute `PnL` (final equity − initial equity).
    pub pnl: f64,
    /// Total return in percent.
    pub return_pct: f64,
    /// Per-bar Sharpe ratio (mean / std of per-bar equity returns).
    pub sharpe: f64,
    /// Per-bar Sortino ratio (mean / downside deviation of per-bar returns).
    pub sortino: f64,
    /// Calmar ratio (total return divided by maximum drawdown).
    pub calmar: f64,
    /// Maximum drawdown in percent (peak-to-trough).
    pub max_drawdown: f64,
    /// Fraction of trades that were profitable, in percent.
    pub win_rate: f64,
    /// Gross profit divided by gross loss.
    pub profit_factor: f64,
    /// Number of completed trades.
    pub num_trades: usize,
}

/// Compute metrics from the starting equity, the per-bar equity series and trades.
pub fn compute(initial: f64, equity: &[f64], trades: &[Trade]) -> Metrics {
    let final_equity = equity.last().copied().unwrap_or(initial);
    let pnl = final_equity - initial;
    let return_pct = if initial.abs() < f64::EPSILON {
        0.0
    } else {
        pnl / initial * 100.0
    };

    let sharpe = sharpe_ratio(equity);
    let sortino = sortino_ratio(equity);
    let max_drawdown = max_drawdown_pct(equity);
    let calmar = if max_drawdown.abs() < f64::EPSILON {
        if return_pct > 0.0 {
            f64::INFINITY
        } else {
            0.0
        }
    } else {
        return_pct / max_drawdown
    };

    let wins = trades.iter().filter(|t| t.pnl > 0.0).count();
    let win_rate = if trades.is_empty() {
        0.0
    } else {
        wins as f64 / trades.len() as f64 * 100.0
    };

    let gross_profit: f64 = trades.iter().filter(|t| t.pnl > 0.0).map(|t| t.pnl).sum();
    let gross_loss: f64 = trades.iter().filter(|t| t.pnl < 0.0).map(|t| -t.pnl).sum();
    let profit_factor = if gross_loss.abs() < f64::EPSILON {
        if gross_profit > 0.0 {
            f64::INFINITY
        } else {
            0.0
        }
    } else {
        gross_profit / gross_loss
    };

    Metrics {
        pnl,
        return_pct,
        sharpe,
        sortino,
        calmar,
        max_drawdown,
        win_rate,
        profit_factor,
        num_trades: trades.len(),
    }
}

fn bar_returns(equity: &[f64]) -> Vec<f64> {
    equity
        .windows(2)
        .map(|w| {
            if w[0].abs() < f64::EPSILON {
                0.0
            } else {
                w[1] / w[0] - 1.0
            }
        })
        .collect()
}

fn sharpe_ratio(equity: &[f64]) -> f64 {
    if equity.len() < 2 {
        return 0.0;
    }
    let returns = bar_returns(equity);
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let var = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    let std = var.sqrt();
    if std.abs() < f64::EPSILON {
        0.0
    } else {
        mean / std
    }
}

fn sortino_ratio(equity: &[f64]) -> f64 {
    if equity.len() < 2 {
        return 0.0;
    }
    let returns = bar_returns(equity);
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    // Downside deviation: root-mean-square of the returns below the zero target.
    let downside = returns.iter().map(|r| r.min(0.0).powi(2)).sum::<f64>() / n;
    let dd = downside.sqrt();
    if dd.abs() < f64::EPSILON {
        0.0
    } else {
        mean / dd
    }
}

fn max_drawdown_pct(equity: &[f64]) -> f64 {
    let mut peak = f64::NEG_INFINITY;
    let mut max_dd = 0.0;
    for &e in equity {
        if e > peak {
            peak = e;
        }
        if peak > 0.0 {
            let dd = (peak - e) / peak * 100.0;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_equity_is_zero_metrics() {
        let m = compute(1000.0, &[1000.0, 1000.0, 1000.0], &[]);
        assert!((m.pnl).abs() < 1e-9);
        assert!((m.sharpe).abs() < 1e-9);
        assert!((m.max_drawdown).abs() < 1e-9);
    }

    #[test]
    fn drawdown_and_return() {
        let m = compute(100.0, &[100.0, 120.0, 90.0, 110.0], &[]);
        assert!((m.return_pct - 10.0).abs() < 1e-9); // 100 -> 110
                                                     // peak 120 -> trough 90 = 25% drawdown
        assert!((m.max_drawdown - 25.0).abs() < 1e-9);
        // Calmar = total return / max drawdown.
        assert!((m.calmar - 10.0 / 25.0).abs() < 1e-9);
    }

    #[test]
    fn sortino_only_penalises_downside() {
        // A rising curve with varying (but never negative) returns has no
        // downside deviation, so Sortino is zero by convention while the
        // varying returns still give a positive Sharpe.
        let up = compute(100.0, &[100.0, 110.0, 115.0], &[]);
        assert!(up.sortino.abs() < 1e-9);
        assert!(up.sharpe > 0.0);

        // With a drawdown the downside deviation is positive and finite.
        let mixed = compute(100.0, &[100.0, 110.0, 99.0, 105.0], &[]);
        assert!(mixed.sortino.is_finite());
    }

    #[test]
    fn calmar_is_infinite_without_drawdown() {
        let m = compute(100.0, &[100.0, 110.0, 120.0], &[]);
        assert!(m.max_drawdown.abs() < 1e-9);
        assert!(m.calmar.is_infinite() && m.calmar > 0.0);
    }
}
