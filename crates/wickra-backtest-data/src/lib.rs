//! # wickra-backtest-data
//!
//! Data loaders that turn CSV / Parquet / JSONL market history into the bar
//! stream the [`wickra-backtest-core`] engine consumes.
//!
//! Status: **scaffold** (handoff-20, Phase 0). Loaders land in Phase 9.

#![forbid(unsafe_code)]

/// The crate version, surfaced for diagnostics.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_reported() {
        assert!(!super::version().is_empty());
    }
}
