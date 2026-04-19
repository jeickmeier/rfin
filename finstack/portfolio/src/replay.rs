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

use crate::portfolio::Portfolio;
use crate::valuation::{value_portfolio, value_portfolio_serial};
use finstack_core::config::FinstackConfig;

/// Portfolios below this position count benefit more from parallelizing the
/// outer replay loop (one Rayon task per timeline snapshot) than from the
/// inner per-position parallelism inside [`value_portfolio`]. Above the
/// threshold the inner parallelism is left in place to avoid oversubscribing
/// the Rayon thread pool.
const REPLAY_OUTER_PARALLEL_POSITION_THRESHOLD: usize = 256;

/// Minimum number of replay snapshots at which outer-loop parallelism is
/// considered; for very short timelines the serial path is clearer and the
/// work per date is amortized by the benchmark loop anyway.
const REPLAY_OUTER_PARALLEL_SNAPSHOT_THRESHOLD: usize = 8;

/// Replay a portfolio through a sequence of dated market snapshots.
///
/// For each date in the timeline the portfolio is re-valued using the
/// corresponding [`MarketContext`].  Depending on [`ReplayMode`]:
///
/// * **`PvOnly`** -- only portfolio PV is recorded at each step.
/// * **`PvAndPnl`** -- daily and cumulative P&L are computed as well.
/// * **`FullAttribution`** -- P&L plus per-position factor decomposition.
///
/// Returns a [`ReplayResult`] containing the per-step detail and an
/// aggregate [`ReplaySummary`].
pub fn replay_portfolio(
    portfolio: &Portfolio,
    timeline: &ReplayTimeline,
    config: &ReplayConfig,
    finstack_config: &FinstackConfig,
) -> Result<ReplayResult> {
    let compute_pnl = matches!(
        config.mode,
        ReplayMode::PvAndPnl | ReplayMode::FullAttribution
    );
    let compute_attribution = matches!(config.mode, ReplayMode::FullAttribution);

    // Decide where to spend Rayon parallelism: small portfolios benefit from
    // parallelizing the outer (per-date) loop and running each valuation
    // serially; large portfolios already saturate the thread pool via
    // per-position parallelism inside `value_portfolio`, so the outer loop
    // runs serially to avoid nested dispatch overhead.
    let use_outer_parallel = portfolio.positions.len() < REPLAY_OUTER_PARALLEL_POSITION_THRESHOLD
        && timeline.snapshots.len() >= REPLAY_OUTER_PARALLEL_SNAPSHOT_THRESHOLD;

    // Phase A: value the portfolio at every snapshot date.
    let valuations: Vec<PortfolioValuation> = if use_outer_parallel {
        use rayon::prelude::*;
        timeline
            .snapshots
            .par_iter()
            .map(|(_date, market)| {
                value_portfolio_serial(
                    portfolio,
                    market,
                    finstack_config,
                    &config.valuation_options,
                )
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        timeline
            .snapshots
            .iter()
            .map(|(_date, market)| {
                value_portfolio(
                    portfolio,
                    market,
                    finstack_config,
                    &config.valuation_options,
                )
            })
            .collect::<Result<Vec<_>>>()?
    };

    // Phase B: assemble ReplayStep entries with P&L and (optionally)
    // attribution. Runs serially — the per-step work is cheap (subtractions
    // and one attribution call) and serial ordering keeps tracing output
    // deterministic. Attribution itself already fans out over positions via
    // `attribute_portfolio_pnl`.
    let mut steps = Vec::with_capacity(timeline.len());

    let (first_date, _) = &timeline.snapshots[0];
    let mut valuations_iter = valuations.into_iter();
    let val_0 = valuations_iter.next().ok_or_else(|| {
        Error::InvalidInput("ReplayTimeline must be non-empty (unreachable)".into())
    })?;
    steps.push(ReplayStep {
        date: *first_date,
        valuation: val_0,
        daily_pnl: None,
        cumulative_pnl: None,
        attribution: None,
    });

    for ((date, market), val_i) in timeline.snapshots.iter().skip(1).zip(valuations_iter) {
        let prev_step = &steps[steps.len() - 1];

        let daily_pnl = if compute_pnl {
            Some(
                val_i
                    .total_base_ccy
                    .checked_sub(prev_step.valuation.total_base_ccy)?,
            )
        } else {
            None
        };

        let cumulative_pnl = if compute_pnl {
            Some(
                val_i
                    .total_base_ccy
                    .checked_sub(steps[0].valuation.total_base_ccy)?,
            )
        } else {
            None
        };

        let attribution = if compute_attribution {
            let attr = crate::attribution::attribute_portfolio_pnl(
                portfolio,
                &timeline.snapshots[steps.len() - 1].1,
                market,
                prev_step.date,
                *date,
                finstack_config,
                config.attribution_method.clone(),
            )?;
            Some(attr)
        } else {
            None
        };

        steps.push(ReplayStep {
            date: *date,
            valuation: val_i,
            daily_pnl,
            cumulative_pnl,
            attribution,
        });
    }

    let summary = compute_summary(&steps);
    Ok(ReplayResult { steps, summary })
}

fn compute_summary(steps: &[ReplayStep]) -> ReplaySummary {
    let start_value = steps[0].valuation.total_base_ccy;
    let end_value = steps[steps.len() - 1].valuation.total_base_ccy;
    let total_pnl = Money::new(
        end_value.amount() - start_value.amount(),
        start_value.currency(),
    );

    // Max drawdown via high-water mark
    let mut peak_value = start_value.amount();
    let mut peak_date = steps[0].date;
    let mut max_dd = 0.0_f64;
    let mut max_dd_peak_date = steps[0].date;
    let mut max_dd_trough_date = steps[0].date;

    for step in steps {
        let val = step.valuation.total_base_ccy.amount();
        if val > peak_value {
            peak_value = val;
            peak_date = step.date;
        }
        let dd = peak_value - val;
        if dd > max_dd {
            max_dd = dd;
            max_dd_peak_date = peak_date;
            max_dd_trough_date = step.date;
        }
    }

    let max_drawdown_pct = if peak_value.abs() > f64::EPSILON {
        max_dd / peak_value
    } else {
        0.0
    };

    ReplaySummary {
        start_date: steps[0].date,
        end_date: steps[steps.len() - 1].date,
        num_steps: steps.len(),
        start_value,
        end_value,
        total_pnl,
        max_drawdown: Money::new(max_dd, start_value.currency()),
        max_drawdown_pct,
        max_drawdown_peak_date: max_dd_peak_date,
        max_drawdown_trough_date: max_dd_trough_date,
    }
}
