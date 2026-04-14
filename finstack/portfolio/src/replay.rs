//! Historical scenario replay for portfolios.
//!
//! Replays a static portfolio through a sequence of dated market snapshots,
//! producing configurable P&L and attribution output at each step.
//!
//! This module is only available when the `scenarios` feature is enabled.

use crate::attribution::PortfolioAttribution;
use crate::error::{Error, Result};
use crate::valuation::PortfolioValuation;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use serde::{Deserialize, Serialize};

/// What to compute at each replay step.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReplayMode {
    /// Just portfolio PV at each date.
    PvOnly,
    /// PV + daily/cumulative P&L.
    PvAndPnl,
    /// PV + P&L + per-position factor decomposition.
    FullAttribution,
}

/// Configuration for a replay run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayConfig {
    /// What to compute at each step.
    pub mode: ReplayMode,
    /// Attribution method (only used in `FullAttribution` mode).
    #[serde(default)]
    pub attribution_method: finstack_valuations::attribution::AttributionMethod,
    /// Valuation options forwarded to `value_portfolio`.
    #[serde(default)]
    pub valuation_options: crate::valuation::PortfolioValuationOptions,
}

/// A dated sequence of market snapshots.
///
/// Invariants enforced by [`ReplayTimeline::new`]:
/// - Non-empty
/// - Sorted by date ascending
/// - No duplicate dates
pub struct ReplayTimeline {
    snapshots: Vec<(Date, MarketContext)>,
}

impl ReplayTimeline {
    /// Create a new timeline from a vector of `(date, market)` pairs.
    ///
    /// Returns an error if the vector is empty, not sorted by date, or
    /// contains duplicate dates.
    pub fn new(snapshots: Vec<(Date, MarketContext)>) -> Result<Self> {
        if snapshots.is_empty() {
            return Err(Error::InvalidInput(
                "ReplayTimeline must be non-empty".into(),
            ));
        }
        for window in snapshots.windows(2) {
            let (d0, _) = &window[0];
            let (d1, _) = &window[1];
            if d1 <= d0 {
                return Err(Error::InvalidInput(format!(
                    "ReplayTimeline dates must be strictly ascending, found {d0} >= {d1}"
                )));
            }
        }
        Ok(Self { snapshots })
    }

    /// Number of snapshots.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Whether the timeline is empty (always false after construction).
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// First and last dates in the timeline.
    pub fn date_range(&self) -> (Date, Date) {
        // Indexing is safe: new() enforces non-empty.
        (
            self.snapshots[0].0,
            self.snapshots[self.snapshots.len() - 1].0,
        )
    }

    /// Iterate over `(date, market)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = &(Date, MarketContext)> {
        self.snapshots.iter()
    }
}

/// Output for a single replay step.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayStep {
    /// Valuation date.
    pub date: Date,
    /// Full portfolio valuation at this date.
    pub valuation: PortfolioValuation,
    /// Daily P&L (this step minus prior step). `None` at step 0.
    pub daily_pnl: Option<Money>,
    /// Cumulative P&L (this step minus step 0). `None` at step 0.
    pub cumulative_pnl: Option<Money>,
    /// Factor attribution between prior step and this step. `None` at step 0
    /// and in non-attribution modes.
    pub attribution: Option<PortfolioAttribution>,
}

/// Aggregate statistics across the full replay.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplaySummary {
    /// First date in the timeline.
    pub start_date: Date,
    /// Last date in the timeline.
    pub end_date: Date,
    /// Number of steps (including step 0).
    pub num_steps: usize,
    /// Portfolio value at step 0.
    pub start_value: Money,
    /// Portfolio value at the last step.
    pub end_value: Money,
    /// Total P&L (end value minus start value).
    pub total_pnl: Money,
    /// Maximum drawdown from peak to trough.
    pub max_drawdown: Money,
    /// Maximum drawdown as a percentage of peak value.
    pub max_drawdown_pct: f64,
    /// Date of the peak before the maximum drawdown.
    pub max_drawdown_peak_date: Date,
    /// Date of the trough of the maximum drawdown.
    pub max_drawdown_trough_date: Date,
}

/// Full output of a replay run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayResult {
    /// Per-step output.
    pub steps: Vec<ReplayStep>,
    /// Aggregate statistics.
    pub summary: ReplaySummary,
}
