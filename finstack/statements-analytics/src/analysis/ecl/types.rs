//! Core types for ECL / IFRS 9 / CECL computation.
//!
//! This module defines the fundamental data structures used throughout the ECL
//! framework:
//!
//! - [`Stage`] -- IFRS 9 impairment stage classification
//! - [`Exposure`] -- a single credit exposure at a reporting date
//! - [`QualitativeFlags`] -- qualitative SICR triggers
//! - [`PdTermStructure`] -- trait abstracting PD curve sources
//! - [`RawPdCurve`] -- user-supplied PD term structure with linear interpolation

use finstack_core::{Error, InputError, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Stage
// ---------------------------------------------------------------------------

/// IFRS 9 impairment stage for a credit exposure.
///
/// Under IFRS 9, financial instruments are classified into three stages that
/// determine the ECL measurement horizon:
///
/// - **Stage 1**: 12-month ECL (no significant increase in credit risk)
/// - **Stage 2**: Lifetime ECL (significant increase in credit risk detected)
/// - **Stage 3**: Lifetime ECL (credit-impaired, objective evidence of default)
///
/// # References
///
/// IFRS 9 Financial Instruments, Section 5.5 -- Impairment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stage {
    /// Performing -- 12-month ECL. No significant increase in credit risk
    /// since initial recognition.
    Stage1,
    /// Underperforming -- lifetime ECL. Significant increase in credit risk
    /// (SICR) detected but not yet credit-impaired.
    Stage2,
    /// Non-performing -- lifetime ECL. Credit-impaired (objective evidence of
    /// default: DPD > 90, restructuring, etc.).
    Stage3,
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stage::Stage1 => write!(f, "Stage 1"),
            Stage::Stage2 => write!(f, "Stage 2"),
            Stage::Stage3 => write!(f, "Stage 3"),
        }
    }
}

// ---------------------------------------------------------------------------
// Qualitative flags
// ---------------------------------------------------------------------------

/// Qualitative triggers for SICR detection (IFRS 9 B5.5.17).
///
/// These flags represent non-quantitative indicators that may trigger a
/// Stage 2 classification independently of PD movements or DPD backstops.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QualitativeFlags {
    /// On internal watchlist.
    pub watchlist: bool,
    /// Forbearance or concession granted.
    pub forbearance: bool,
    /// Significant adverse change in business or financial conditions.
    pub adverse_conditions: bool,
    /// Custom user-defined flags (e.g., sector-specific triggers).
    pub custom: Vec<String>,
}

impl QualitativeFlags {
    /// Returns `true` if any qualitative flag is active.
    pub fn any_active(&self) -> bool {
        self.watchlist
            || self.forbearance
            || self.adverse_conditions
            || !self.custom.is_empty()
    }

    /// Returns the names of all active flags.
    pub fn active_flags(&self) -> Vec<String> {
        let mut flags = Vec::new();
        if self.watchlist {
            flags.push("watchlist".to_string());
        }
        if self.forbearance {
            flags.push("forbearance".to_string());
        }
        if self.adverse_conditions {
            flags.push("adverse_conditions".to_string());
        }
        for c in &self.custom {
            flags.push(c.clone());
        }
        flags
    }
}

// ---------------------------------------------------------------------------
// Exposure
// ---------------------------------------------------------------------------

/// A single credit exposure for ECL computation.
///
/// Represents one instrument or facility at a reporting date. All monetary
/// amounts are in the exposure's base currency (currency conversion is the
/// caller's responsibility).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exposure {
    /// Unique identifier for the exposure (instrument ID, facility ID, etc.).
    pub id: String,

    /// Segment keys for portfolio aggregation (product type, geography, rating
    /// bucket, etc.). Empty vec means "unclassified".
    pub segments: Vec<String>,

    /// Outstanding balance (drawn amount) at the reporting date.
    pub ead: f64,

    /// Effective interest rate (annualized, decimal). Used as the IFRS 9
    /// discount rate. Example: 0.05 = 5%.
    pub eir: f64,

    /// Remaining maturity in years from reporting date. For revolving
    /// facilities, use behavioural maturity.
    pub remaining_maturity_years: f64,

    /// Loss given default (decimal, 0..1). Can be point-in-time or
    /// downturn LGD depending on methodology.
    pub lgd: f64,

    /// Current days past due (DPD). Used for backstop staging triggers.
    pub days_past_due: u32,

    /// Current rating label (must match the `PdTermStructure` scale).
    /// `None` if the exposure is unrated.
    pub current_rating: Option<String>,

    /// Rating at origination (initial recognition). Used for SICR delta PD
    /// comparison. `None` disables the PD delta trigger.
    pub origination_rating: Option<String>,

    /// Qualitative flags that can trigger Stage 2 classification.
    pub qualitative_flags: QualitativeFlags,

    /// Number of consecutive performing periods since last Stage 2/3
    /// classification. Used for curing logic.
    pub consecutive_performing_periods: u32,

    /// Previous reporting period's stage (for migration tracking).
    pub previous_stage: Option<Stage>,
}

// ---------------------------------------------------------------------------
// PD Term Structure trait
// ---------------------------------------------------------------------------

/// Abstraction over PD term structure sources.
///
/// Implementations extract marginal default probabilities for time buckets.
/// The library provides [`RawPdCurve`] for user-supplied PD vectors; external
/// implementations can wrap `HazardCurve` or `TransitionMatrix`.
///
/// # Contract
///
/// - `cumulative_pd` must return values in \[0, 1\].
/// - `cumulative_pd` must be monotonically non-decreasing in `t`.
/// - `marginal_pd` must return values in \[0, 1\].
pub trait PdTermStructure: Send + Sync {
    /// Cumulative probability of default by time `t` (in years) for the
    /// given rating state. Returns a value in \[0, 1\].
    fn cumulative_pd(&self, rating: &str, t: f64) -> Result<f64>;

    /// Marginal (forward) PD for the interval (t1, t2\], conditional on
    /// survival to t1. Default implementation derives from cumulative PD.
    fn marginal_pd(&self, rating: &str, t1: f64, t2: f64) -> Result<f64> {
        let s1 = 1.0 - self.cumulative_pd(rating, t1)?;
        let s2 = 1.0 - self.cumulative_pd(rating, t2)?;
        if s1 <= 0.0 {
            return Ok(1.0);
        }
        Ok(1.0 - s2 / s1)
    }
}

// ---------------------------------------------------------------------------
// RawPdCurve
// ---------------------------------------------------------------------------

/// Raw user-supplied PD term structure with linear interpolation.
///
/// Use this when you have a discrete set of cumulative PD observations
/// (e.g., from internal rating model output) rather than a parametric
/// hazard curve or transition matrix.
///
/// # Interpolation
///
/// - Knots must be sorted by time and monotonically increasing in PD.
/// - For `t` before the first knot, cumulative PD is 0.
/// - For `t` after the last knot, cumulative PD is flat-extrapolated.
/// - Between knots, linear interpolation is applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPdCurve {
    /// Rating label this curve applies to.
    pub rating: String,
    /// (time_years, cumulative_pd) knots, sorted by time, monotonically increasing.
    pub knots: Vec<(f64, f64)>,
}

impl RawPdCurve {
    /// Create a new `RawPdCurve`, validating that knots are sorted and monotonic.
    pub fn new(rating: impl Into<String>, knots: Vec<(f64, f64)>) -> Result<Self> {
        if knots.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }
        for window in knots.windows(2) {
            if window[1].0 <= window[0].0 {
                return Err(InputError::NonMonotonicKnots.into());
            }
        }
        Ok(Self {
            rating: rating.into(),
            knots,
        })
    }
}

impl PdTermStructure for RawPdCurve {
    fn cumulative_pd(&self, rating: &str, t: f64) -> Result<f64> {
        if rating != self.rating {
            return Err(Error::Validation(format!(
                "RawPdCurve is for rating '{}', got '{}'",
                self.rating, rating
            )));
        }
        Ok(interp_linear(&self.knots, t))
    }
}

/// Linear interpolation with flat extrapolation on a sorted knot vector.
pub(crate) fn interp_linear(knots: &[(f64, f64)], t: f64) -> f64 {
    if knots.is_empty() {
        return 0.0;
    }
    if t <= knots[0].0 {
        return knots[0].1;
    }
    if t >= knots[knots.len() - 1].0 {
        return knots[knots.len() - 1].1;
    }
    // Binary search for the right interval
    let idx = knots.partition_point(|k| k.0 < t);
    if idx == 0 {
        return knots[0].1;
    }
    let (t0, y0) = knots[idx - 1];
    let (t1, y1) = knots[idx];
    let w = (t - t0) / (t1 - t0);
    y0 + w * (y1 - y0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_display() {
        assert_eq!(format!("{}", Stage::Stage1), "Stage 1");
        assert_eq!(format!("{}", Stage::Stage2), "Stage 2");
        assert_eq!(format!("{}", Stage::Stage3), "Stage 3");
    }

    #[test]
    fn test_qualitative_flags_none_active() {
        let flags = QualitativeFlags::default();
        assert!(!flags.any_active());
        assert!(flags.active_flags().is_empty());
    }

    #[test]
    fn test_qualitative_flags_some_active() {
        let flags = QualitativeFlags {
            watchlist: true,
            forbearance: false,
            adverse_conditions: false,
            custom: vec!["sector_stress".to_string()],
        };
        assert!(flags.any_active());
        let active = flags.active_flags();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"watchlist".to_string()));
        assert!(active.contains(&"sector_stress".to_string()));
    }

    #[test]
    fn test_raw_pd_curve_validates_knots() {
        // Too few points
        assert!(RawPdCurve::new("BBB", vec![(1.0, 0.02)]).is_err());
        // Non-monotonic
        assert!(RawPdCurve::new("BBB", vec![(2.0, 0.02), (1.0, 0.04)]).is_err());
        // Valid
        assert!(RawPdCurve::new("BBB", vec![(1.0, 0.02), (2.0, 0.04)]).is_ok());
    }

    #[test]
    fn test_interp_linear() {
        let knots = vec![(1.0, 0.02), (2.0, 0.05), (5.0, 0.12)];
        // Before first knot -> first value
        assert!((interp_linear(&knots, 0.5) - 0.02).abs() < 1e-10);
        // At first knot
        assert!((interp_linear(&knots, 1.0) - 0.02).abs() < 1e-10);
        // Midpoint of first segment
        assert!((interp_linear(&knots, 1.5) - 0.035).abs() < 1e-10);
        // At second knot
        assert!((interp_linear(&knots, 2.0) - 0.05).abs() < 1e-10);
        // After last knot -> flat extrapolation
        assert!((interp_linear(&knots, 10.0) - 0.12).abs() < 1e-10);
    }

    #[test]
    fn test_raw_pd_curve_cumulative_pd() {
        let curve = RawPdCurve::new(
            "BBB",
            vec![(1.0, 0.02), (2.0, 0.05), (5.0, 0.12)],
        )
        .ok();
        let curve = curve.as_ref().unwrap(); // ok in test
        assert!((curve.cumulative_pd("BBB", 1.5).unwrap() - 0.035).abs() < 1e-10);
        // Wrong rating
        assert!(curve.cumulative_pd("AA", 1.0).is_err());
    }

    #[test]
    fn test_raw_pd_curve_marginal_pd() {
        let curve = RawPdCurve::new(
            "BBB",
            vec![(0.0, 0.0), (1.0, 0.02), (2.0, 0.05)],
        )
        .ok();
        let curve = curve.as_ref().unwrap();
        // Marginal PD from 0 to 1: cumulative goes from 0 to 0.02
        // survival(0) = 1.0, survival(1) = 0.98
        // marginal = 1 - 0.98/1.0 = 0.02
        let mpd = curve.marginal_pd("BBB", 0.0, 1.0).unwrap();
        assert!((mpd - 0.02).abs() < 1e-10);

        // Marginal PD from 1 to 2: survival(1) = 0.98, survival(2) = 0.95
        // marginal = 1 - 0.95/0.98 = 0.030612...
        let mpd = curve.marginal_pd("BBB", 1.0, 2.0).unwrap();
        let expected = 1.0 - 0.95 / 0.98;
        assert!((mpd - expected).abs() < 1e-10);
    }
}
