//! Error types for the backtest engine.
//!
//! All fallible paths return [`BacktestError`]; bindings map it to a stable
//! integer code so no panic ever crosses an FFI boundary.

use thiserror::Error;

/// An error raised while parsing a strategy spec or running a backtest.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BacktestError {
    /// The strategy spec could not be parsed or was structurally invalid.
    #[error("invalid strategy spec: {0}")]
    InvalidSpec(String),

    /// An indicator referenced in the spec is unknown to the registry.
    #[error("unknown indicator type: {0}")]
    UnknownIndicator(String),

    /// An indicator was constructed with the wrong number or value of parameters.
    #[error("invalid parameters for indicator {indicator}: {reason}")]
    InvalidParams {
        /// The indicator type name.
        indicator: String,
        /// Why the parameters were rejected.
        reason: String,
    },

    /// A rule referenced an indicator name that is not declared in `indicators`.
    #[error("rule references undeclared indicator: {0}")]
    UndeclaredRef(String),

    /// The input data was empty or malformed.
    #[error("invalid input data: {0}")]
    InvalidData(String),
}

/// Convenience result alias for the engine.
pub type Result<T> = core::result::Result<T, BacktestError>;
