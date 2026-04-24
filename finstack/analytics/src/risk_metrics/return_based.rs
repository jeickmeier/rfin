//! Return-based risk metrics: mean, volatility, Sharpe, Sortino, CAGR, and more.
//!
//! All functions operate on `&[f64]` return slices and return scalar `f64`.
//! Annualization uses the caller-supplied factor (typically from
//! `PeriodKind::annualization_factor()`).

use std::str::FromStr;

use crate::math::stats::{mean, variance};
use crate::math::summation::kahan_sum;
use rand::{rngs::SmallRng, Rng, SeedableRng};

use super::tail_risk::cornish_fisher_var;

/// True when annualization is requested but `ann_factor` is not a positive finite
/// periods-per-year count (e.g. zero, negative, NaN, or infinity).
///
/// Shared analytics-wide guard; re-exported via [`crate::risk_metrics`] and used
/// from benchmark-relative metrics to avoid redefining the same check.
#[inline]
pub(crate) fn invalid_annualization_factor(annualize: bool, ann_factor: f64) -> bool {
    annualize && (!ann_factor.is_finite() || ann_factor <= 0.0)
}

/// Day-count convention for CAGR annualization over explicit calendar dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum AnnualizationConvention {
    /// Actual calendar days divided by 365.0.
    Act365Fixed,
    /// Actual calendar days divided by 365.25 (default).
    #[default]
    Act365_25,
    /// Actual/Actual using the actual number of days in each calendar year.
    ActAct,
}

impl FromStr for AnnualizationConvention {
    type Err = String;

    /// Parse an annualization convention label (case-insensitive).
    ///
    /// Canonical forms: `"act365_25"`, `"act365_fixed"`, `"act_act"`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "act365_25" => Ok(AnnualizationConvention::Act365_25),
            "act365_fixed" => Ok(AnnualizationConvention::Act365Fixed),
            "act_act" => Ok(AnnualizationConvention::ActAct),
            other => Err(format!(
                "unknown CAGR convention {other:?}; expected one of act365_25, act365_fixed, act_act"
            )),
        }
    }
}

/// Basis used to annualize CAGR from either explicit dates or a periods-per-year factor.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CagrBasis {
    /// Annualize across an explicit calendar range using the chosen convention.
    Dates {
        /// Inclusive start date of the return span.
        start: crate::dates::Date,
        /// Inclusive end date of the return span.
        end: crate::dates::Date,
        /// Day-count convention used to convert the span to a year fraction.
        convention: AnnualizationConvention,
    },
    /// Annualize from a periods-per-year factor such as 252 (daily) or 12 (monthly).
    Factor(f64),
}

impl CagrBasis {
    /// Build a date-based CAGR basis using the default Act/365.25 convention.
    #[must_use]
    pub fn dates(start: crate::dates::Date, end: crate::dates::Date) -> Self {
        Self::Dates {
            start,
            end,
            convention: AnnualizationConvention::default(),
        }
    }

    /// Build a date-based CAGR basis with an explicit day-count convention.
    #[must_use]
    pub fn dates_with_convention(
        start: crate::dates::Date,
        end: crate::dates::Date,
        convention: AnnualizationConvention,
    ) -> Self {
        Self::Dates {
            start,
            end,
            convention,
        }
    }

    /// Build a factor-based CAGR basis from periods per year.
    #[must_use]
    pub fn factor(ann_factor: f64) -> Self {
        Self::Factor(ann_factor)
    }
}

/// Compound annual growth rate from a return series using the supplied basis.
///
/// Computes:
///
/// ```text
/// CAGR = (Π(1 + r_i))^(1/years) - 1
/// ```
///
/// where `years` comes either from an explicit date range or from a
/// periods-per-year factor, depending on `basis`.
///
/// # Arguments
///
/// * `returns`    - Slice of simple period returns.
/// * `basis`      - How to annualize the compounded return.
///
/// # Returns
///
/// Annualized growth rate as a decimal. Returns [`f64::NAN`] for empty input,
/// `0.0` for non-positive date spans, and [`f64::NAN`] when a factor basis uses
/// an invalid annualization factor.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::{cagr, CagrBasis};
/// use finstack_core::dates::{Date, Month};
///
/// let start = Date::from_calendar_date(2024, Month::January, 1).unwrap();
/// let end   = Date::from_calendar_date(2025, Month::January, 1).unwrap();
/// // Single 10% return over one year → CAGR ≈ 10%.
/// let c = cagr(&[0.10], CagrBasis::dates(start, end));
/// assert!((c - 0.10).abs() < 0.01);
/// ```
#[must_use]
pub fn cagr(returns: &[f64], basis: CagrBasis) -> f64 {
    if returns.is_empty() {
        return f64::NAN;
    }

    match basis {
        CagrBasis::Dates {
            start,
            end,
            convention,
        } => cagr_from_dates(returns, start, end, convention),
        CagrBasis::Factor(ann_factor) => cagr_from_factor(returns, ann_factor),
    }
}
fn cagr_from_dates(
    returns: &[f64],
    start: crate::dates::Date,
    end: crate::dates::Date,
    convention: AnnualizationConvention,
) -> f64 {
    let total = 1.0 + crate::returns::comp_total(returns);
    let years = annualized_years(start, end, convention);
    if years <= 0.0 {
        return 0.0;
    }
    total.powf(1.0 / years) - 1.0
}

fn cagr_from_factor(returns: &[f64], ann_factor: f64) -> f64 {
    if !ann_factor.is_finite() || ann_factor <= 0.0 {
        return f64::NAN;
    }
    let total = 1.0 + crate::returns::comp_total(returns);
    let years = returns.len() as f64 / ann_factor;
    if years > 0.0 {
        total.powf(1.0 / years) - 1.0
    } else {
        0.0
    }
}

fn annualized_years(
    start: crate::dates::Date,
    end: crate::dates::Date,
    convention: AnnualizationConvention,
) -> f64 {
    let days = (end - start).whole_days() as f64;
    if days <= 0.0 {
        return 0.0;
    }

    match convention {
        AnnualizationConvention::Act365Fixed => days / 365.0,
        AnnualizationConvention::Act365_25 => days / 365.25,
        AnnualizationConvention::ActAct => actual_actual_years(start, end),
    }
}

fn actual_actual_years(start: crate::dates::Date, end: crate::dates::Date) -> f64 {
    use crate::dates::Month;

    let mut current = start;
    let mut years = 0.0;

    while current < end {
        let next_year_start =
            match crate::dates::Date::from_calendar_date(current.year() + 1, Month::January, 1) {
                Ok(date) => date,
                Err(_) => return years,
            };
        let segment_end = next_year_start.min(end);
        let segment_days = (segment_end - current).whole_days() as f64;
        years += segment_days
            / if is_leap_year(current.year()) {
                366.0
            } else {
                365.0
            };
        current = segment_end;
    }

    years
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Mean return, optionally annualized.
///
/// Computes the **arithmetic** mean of `returns`. When `annualize` is `true`,
/// that mean is scaled by `ann_factor` (e.g., 252 for daily data):
///
/// ```text
/// μ_ann = μ_period × ann_factor
/// ```
///
/// This is **simple** annualization of the average **per-period** return, not a
/// compounded (geometric) annual return. For growth over time that compounds
/// period returns, use [`cagr`]. Volatility in this
/// module uses the usual root-time rule (`σ_ann = σ_period × √ann_factor`); mean
/// return uses **linear** scaling instead.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `annualize`  - Whether to multiply the mean by `ann_factor`.
/// * `ann_factor` - Number of periods per year (e.g., 252 daily, 12 monthly).
///
/// # Returns
///
/// Arithmetic mean return, annualized if requested. Returns `0.0` for an
/// empty slice. When `annualize` is `true`, returns [`f64::NAN`] if `ann_factor`
/// is not finite or is `<= 0`.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::mean_return;
///
/// let r = [0.01, 0.02, 0.03];
/// let m = mean_return(&r, false, 252.0);
/// assert!((m - 0.02).abs() < 1e-12);
///
/// let m_ann = mean_return(&r, true, 252.0);
/// assert!((m_ann - 0.02 * 252.0).abs() < 1e-10);
/// ```
#[must_use]
pub fn mean_return(returns: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    if invalid_annualization_factor(annualize, ann_factor) {
        return f64::NAN;
    }
    let m = mean(returns);
    if annualize {
        m * ann_factor
    } else {
        m
    }
}
/// Volatility (standard deviation of returns), optionally annualized.
///
/// Uses **sample** standard deviation (n-1 denominator), consistent with
/// Bloomberg, QuantLib, and the `OnlineStats::variance()` convention.
/// Annualizes by multiplying by `sqrt(ann_factor)` following the
/// square-root-of-time rule.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `annualize`  - Whether to scale by `sqrt(ann_factor)`.
/// * `ann_factor` - Number of periods per year (e.g., 252 daily, 12 monthly).
///
/// # Returns
///
/// Sample standard deviation of `returns` (n-1 denominator), annualized if requested.
/// Returns `0.0` for an empty slice. When `annualize` is `true`, returns
/// [`f64::NAN`] if `ann_factor` is not finite or is `<= 0`.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::volatility;
///
/// let r = [0.01, -0.01, 0.02, -0.02];
/// let vol = volatility(&r, false, 252.0);
/// assert!(vol > 0.0);
///
/// let vol_ann = volatility(&r, true, 252.0);
/// assert!((vol_ann - vol * 252.0_f64.sqrt()).abs() < 1e-12);
/// ```
#[must_use]
pub fn volatility(returns: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    if invalid_annualization_factor(annualize, ann_factor) {
        return f64::NAN;
    }
    let v = variance(returns).sqrt();
    if annualize {
        v * ann_factor.sqrt()
    } else {
        v
    }
}
/// Sharpe ratio = (annualized return − risk-free rate) / annualized volatility.
///
/// Measures risk-adjusted return relative to total (upside + downside)
/// volatility. A higher value indicates better risk-adjusted performance.
///
/// # Arguments
///
/// * `ann_return`     - Annualized portfolio return.
/// * `ann_vol`        - Annualized portfolio volatility.
/// * `risk_free_rate` - Annualized risk-free rate (e.g., `0.02` for 2%).
///
/// # Returns
///
/// The Sharpe ratio. When `ann_vol` is zero: returns `f64::INFINITY` if
/// excess return is positive, `f64::NEG_INFINITY` if negative, and `0.0`
/// if both are zero (matching [`sortino`] convention).
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::sharpe;
///
/// assert!((sharpe(0.10, 0.15, 0.0) - 0.6667).abs() < 0.001);
/// // Zero volatility with positive excess → +∞.
/// assert_eq!(sharpe(0.10, 0.0, 0.0), f64::INFINITY);
/// ```
///
/// # References
///
/// - Sharpe (1966): see docs/REFERENCES.md#sharpe1966
#[must_use]
pub fn sharpe(ann_return: f64, ann_vol: f64, risk_free_rate: f64) -> f64 {
    if ann_vol == 0.0 {
        let excess = ann_return - risk_free_rate;
        return if excess > 0.0 {
            f64::INFINITY
        } else if excess < 0.0 {
            f64::NEG_INFINITY
        } else {
            0.0
        };
    }
    (ann_return - risk_free_rate) / ann_vol
}

/// Downside deviation: semi-standard deviation below a minimum acceptable return.
///
/// Computes the root-mean-square of returns falling below `mar`, using
/// the full series length as the denominator (population convention),
/// consistent with Sortino & van der Meer (1991):
///
/// ```text
/// DD = sqrt( (1/n) × Σ min(r_i − MAR, 0)² )
/// ```
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `mar`        - Minimum acceptable return (threshold). Use `0.0` for
///   the standard Sortino definition.
/// * `annualize`  - Whether to scale by `sqrt(ann_factor)`.
/// * `ann_factor` - Number of periods per year.
///
/// # Returns
///
/// The downside deviation (non-negative). Returns `0.0` for an empty
/// slice or when no returns fall below `mar`. When `annualize` is `true`,
/// returns [`f64::NAN`] if `ann_factor` is not finite or is `<= 0`.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::downside_deviation;
///
/// let r = [0.01, -0.02, 0.03, -0.01, 0.005];
/// let dd = downside_deviation(&r, 0.0, false, 252.0);
/// assert!(dd > 0.0);
///
/// // All returns above MAR → zero downside deviation.
/// let dd_pos = downside_deviation(&[0.01, 0.02, 0.03], 0.0, false, 252.0);
/// assert_eq!(dd_pos, 0.0);
/// ```
///
/// # References
///
/// - Sortino & van der Meer (1991): see docs/REFERENCES.md#sortinoVanDerMeer1991
#[must_use]
pub fn downside_deviation(returns: &[f64], mar: f64, annualize: bool, ann_factor: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    if invalid_annualization_factor(annualize, ann_factor) {
        return f64::NAN;
    }
    let downside_sq = kahan_sum(returns.iter().filter(|&&r| r < mar).map(|&r| {
        let d = r - mar;
        d * d
    }));
    let dd = (downside_sq / returns.len() as f64).sqrt();
    if annualize {
        dd * ann_factor.sqrt()
    } else {
        dd
    }
}
/// Sortino ratio: penalises only downside volatility.
///
/// Unlike the Sharpe ratio, the Sortino ratio uses the **downside deviation**
/// (semi-standard deviation of negative returns) as the risk denominator,
/// leaving upside volatility unrewarded:
///
/// ```text
/// Sortino = (annualized mean return) / (annualized downside deviation)
/// ```
///
/// Downside deviation is computed over the full return series (denominator
/// is `n`, not the number of negative observations), consistent with the
/// Sortino & van der Meer (1991) definition.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
/// * `annualize` - Whether to annualize both numerator and denominator.
/// * `ann_factor` - Number of periods per year.
/// * `mar` - Minimum acceptable return per period in decimal form.
///
/// # Returns
///
/// The Sortino ratio. Returns `±∞` when the mean is nonzero but there
/// are no negative returns (zero downside risk), and `0.0` when the
/// mean is zero or the downside deviation is zero. When `annualize` is
/// `true`, returns [`f64::NAN`] if `ann_factor` is not finite or is `<= 0`.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::sortino;
///
/// let r = [0.01, 0.02, 0.03, -0.005, 0.01];
/// let s = sortino(&r, false, 252.0, 0.0);
/// assert!(s > 0.0);
/// ```
///
/// # References
///
/// - Sortino & van der Meer (1991): see docs/REFERENCES.md#sortinoVanDerMeer1991
#[must_use]
pub fn sortino(returns: &[f64], annualize: bool, ann_factor: f64, mar: f64) -> f64 {
    if invalid_annualization_factor(annualize, ann_factor) {
        return f64::NAN;
    }
    let excess_mean = mean(returns) - mar;
    let dd = downside_deviation(returns, mar, false, ann_factor);
    if dd == 0.0 {
        return if excess_mean > 0.0 {
            f64::INFINITY
        } else if excess_mean < 0.0 {
            f64::NEG_INFINITY
        } else {
            0.0
        };
    }
    if annualize {
        (excess_mean * ann_factor) / (dd * ann_factor.sqrt())
    } else {
        excess_mean / dd
    }
}
/// Explicit ruin event definition for simulated portfolio paths.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RuinDefinition {
    /// Ruin occurs once path wealth falls to or below a fraction of starting wealth.
    WealthFloor {
        /// Wealth-floor fraction of starting wealth (e.g. `0.5` = 50%).
        floor_fraction: f64,
    },
    /// Ruin occurs if terminal wealth ends at or below a target fraction.
    TerminalFloor {
        /// Terminal-wealth fraction of starting wealth triggering ruin.
        floor_fraction: f64,
    },
    /// Ruin occurs once drawdown from the running peak reaches the threshold.
    DrawdownBreach {
        /// Maximum tolerated drawdown as a positive fraction (e.g. `0.25` = 25%).
        max_drawdown: f64,
    },
}

/// Simulation controls for ruin-probability estimation.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RuinModel {
    /// Number of simulated periods per path.
    pub horizon_periods: usize,
    /// Number of bootstrap paths.
    pub n_paths: usize,
    /// Circular block-bootstrap length. Use `1` for IID resampling.
    pub block_size: usize,
    /// Deterministic RNG seed for reproducibility.
    pub seed: u64,
    /// Confidence level for the reported Wilson-score confidence interval.
    pub confidence_level: f64,
}

impl Default for RuinModel {
    fn default() -> Self {
        Self {
            horizon_periods: 252,
            n_paths: 10_000,
            block_size: 5,
            seed: 42,
            confidence_level: 0.95,
        }
    }
}

/// Ruin-probability estimate with uncertainty bounds.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RuinEstimate {
    /// Estimated probability of ruin.
    pub probability: f64,
    /// Binomial standard error of the estimated probability.
    pub std_err: f64,
    /// Lower confidence bound.
    pub ci_lower: f64,
    /// Upper confidence bound.
    pub ci_upper: f64,
}

impl RuinEstimate {
    fn from_probability(probability: f64, n_paths: usize, confidence_level: f64) -> Self {
        if !probability.is_finite() {
            return Self {
                probability: f64::NAN,
                std_err: f64::NAN,
                ci_lower: f64::NAN,
                ci_upper: f64::NAN,
            };
        }
        let std_err = if n_paths == 0 {
            0.0
        } else {
            (probability * (1.0 - probability) / n_paths as f64).sqrt()
        };
        let cl = if (0.0..1.0).contains(&confidence_level) {
            confidence_level
        } else {
            0.95
        };
        let z = crate::math::special_functions::standard_normal_inv_cdf(0.5 + cl / 2.0);
        if n_paths == 0 {
            return Self {
                probability,
                std_err,
                ci_lower: probability,
                ci_upper: probability,
            };
        }
        let n = n_paths as f64;
        let z2 = z * z;
        let denom = 1.0 + z2 / n;
        let center = (probability + z2 / (2.0 * n)) / denom;
        let half_width =
            z * ((probability * (1.0 - probability) + z2 / (4.0 * n)) / n).sqrt() / denom;
        Self {
            probability,
            std_err,
            ci_lower: (center - half_width).max(0.0),
            ci_upper: (center + half_width).min(1.0),
        }
    }
}

fn valid_ruin_definition(definition: RuinDefinition) -> bool {
    match definition {
        RuinDefinition::WealthFloor { floor_fraction }
        | RuinDefinition::TerminalFloor { floor_fraction } => {
            floor_fraction.is_finite() && (0.0..=1.0).contains(&floor_fraction)
        }
        RuinDefinition::DrawdownBreach { max_drawdown } => {
            max_drawdown.is_finite() && (0.0..=1.0).contains(&max_drawdown)
        }
    }
}

/// Estimate ruin probability from an empirical return distribution via bootstrap simulation.
///
/// The estimator simulates wealth paths by circular block-bootstrap resampling
/// the historical return series. Using `block_size > 1` preserves short-run
/// serial dependence better than IID resampling, while `block_size = 1`
/// reduces to period-by-period sampling with replacement.
///
/// Returns are interpreted as simple per-period decimal returns, so `0.01`
/// means +1% and `-0.25` means -25%. Wealth starts at `1.0` on every path.
/// The ruin condition is controlled by [`RuinDefinition`]:
///
/// - [`RuinDefinition::WealthFloor`] triggers once path wealth falls to or
///   below `floor_fraction` of starting wealth.
/// - [`RuinDefinition::TerminalFloor`] checks the same threshold only at the
///   terminal horizon.
/// - [`RuinDefinition::DrawdownBreach`] triggers once peak-to-trough drawdown
///   reaches `max_drawdown`, expressed as a fraction in `[0, 1]`.
///
/// The confidence interval in [`RuinEstimate`] is a Wilson-score interval
/// around the simulated binomial ruin frequency.
///
/// # Arguments
///
/// * `returns` - Historical simple-return sample in decimal form.
/// * `definition` - Operational definition of ruin for each simulated path.
/// * `model` - Simulation controls including horizon length, number of paths,
///   bootstrap block size, deterministic RNG seed, and confidence level for
///   the reported interval.
///
/// # Returns
///
/// A [`RuinEstimate`] with ruin probability, binomial standard error, and
/// lower/upper confidence bounds.
///
/// Returns a zero-probability estimate when `returns` is empty, or when
/// `model.horizon_periods == 0` or `model.n_paths == 0`.
///
/// Returns `NaN` fields when `model.block_size == 0` or any input return is
/// non-finite.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::{estimate_ruin, RuinDefinition, RuinModel};
///
/// let returns = [0.01, -0.02, 0.015, -0.01, 0.005];
/// let model = RuinModel {
///     horizon_periods: 30,
///     n_paths: 5_000,
///     block_size: 3,
///     seed: 7,
///     confidence_level: 0.95,
/// };
///
/// let estimate = estimate_ruin(
///     &returns,
///     RuinDefinition::WealthFloor {
///         floor_fraction: 0.8,
///     },
///     &model,
/// );
///
/// assert!(estimate.probability.is_finite());
/// assert!((0.0..=1.0).contains(&estimate.probability));
/// assert!(estimate.ci_lower <= estimate.ci_upper);
/// ```
///
/// # References
///
/// - Press et al.: see docs/REFERENCES.md#press-numerical-recipes
#[tracing::instrument(level = "debug", skip(returns), fields(n_returns = returns.len(), horizon = model.horizon_periods, n_paths = model.n_paths, seed = model.seed))]
pub fn estimate_ruin(
    returns: &[f64],
    definition: RuinDefinition,
    model: &RuinModel,
) -> RuinEstimate {
    if returns.is_empty() || model.horizon_periods == 0 || model.n_paths == 0 {
        return RuinEstimate::from_probability(0.0, model.n_paths, model.confidence_level);
    }
    if model.block_size == 0
        || returns.iter().any(|r| !r.is_finite() || *r < -1.0)
        || !valid_ruin_definition(definition)
    {
        return RuinEstimate::from_probability(f64::NAN, model.n_paths, model.confidence_level);
    }

    let mut rng = SmallRng::seed_from_u64(model.seed);
    let block_size = model.block_size.min(returns.len());
    let mut ruin_count = 0usize;

    for _ in 0..model.n_paths {
        let mut wealth = 1.0_f64;
        let mut peak = 1.0_f64;
        let mut steps_done = 0usize;
        let mut ruined = false;

        while steps_done < model.horizon_periods && !ruined {
            let start = rng.random_range(0..returns.len());
            let block_len = block_size.min(model.horizon_periods - steps_done);

            for offset in 0..block_len {
                let r = returns[(start + offset) % returns.len()];
                wealth *= 1.0 + r;
                peak = peak.max(wealth);
                steps_done += 1;

                ruined = match definition {
                    RuinDefinition::WealthFloor { floor_fraction } => wealth <= floor_fraction,
                    RuinDefinition::DrawdownBreach { max_drawdown } => {
                        peak > 0.0 && 1.0 - wealth / peak >= max_drawdown
                    }
                    RuinDefinition::TerminalFloor { .. } => false,
                };
                if ruined {
                    break;
                }
            }
        }

        if !ruined {
            ruined = match definition {
                RuinDefinition::TerminalFloor { floor_fraction } => wealth <= floor_fraction,
                RuinDefinition::WealthFloor { .. } | RuinDefinition::DrawdownBreach { .. } => false,
            };
        }

        ruin_count += usize::from(ruined);
    }

    RuinEstimate::from_probability(
        ruin_count as f64 / model.n_paths as f64,
        model.n_paths,
        model.confidence_level,
    )
}
/// Geometric mean return per period.
///
/// The compound-average return: the constant per-period return that
/// would produce the same terminal wealth as the actual series.
///
/// ```text
/// geo_mean = (Π(1 + r_i))^(1/n) − 1
/// ```
///
/// Computed in log-space with Kahan summation for numerical stability.
/// Returns [`f64::NEG_INFINITY`] if any return is `<= -1.0`, which
/// represents a full wipeout (or worse) and avoids the upward bias that
/// a positive clamp would introduce near total loss.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
///
/// # Returns
///
/// The geometric mean return. Returns [`f64::NAN`] for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::geometric_mean;
///
/// // +10% then −10%: geometric mean < 0 (volatility drag).
/// let gm = geometric_mean(&[0.10, -0.10]);
/// assert!(gm < 0.0);
///
/// // Constant 5% → geometric mean = 5%.
/// let gm5 = geometric_mean(&[0.05, 0.05, 0.05]);
/// assert!((gm5 - 0.05).abs() < 1e-12);
/// ```
#[must_use]
pub fn geometric_mean(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return f64::NAN;
    }
    if returns.iter().any(|&r| r <= -1.0) {
        return f64::NEG_INFINITY;
    }
    let n = returns.len() as f64;
    let log_sum = kahan_sum(returns.iter().map(|&r| (1.0 + r).ln()));
    (log_sum / n).exp() - 1.0
}

/// Omega ratio: probability-weighted gain-to-loss ratio above a threshold.
///
/// ```text
/// Ω(L) = Σ max(r_i − L, 0) / Σ max(L − r_i, 0)
/// ```
///
/// Unlike the Sharpe ratio (which uses only mean and variance), the Omega
/// ratio incorporates the full return distribution.
///
/// # Arguments
///
/// * `returns`   - Slice of period simple returns.
/// * `threshold` - Return threshold (typically `0.0`).
///
/// # Returns
///
/// The Omega ratio. Returns `f64::INFINITY` if gains exist but no losses,
/// `1.0` if all returns equal the threshold (neutral outcome per
/// Keating-Shadwick), and [`f64::NAN`] for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::omega_ratio;
///
/// let r = [0.05, -0.02, 0.03, -0.01, 0.04];
/// let omega = omega_ratio(&r, 0.0);
/// assert!(omega > 1.0);
/// ```
///
/// # References
///
/// - Keating & Shadwick (2002): see docs/REFERENCES.md#keatingShadwick2002
#[must_use]
pub fn omega_ratio(returns: &[f64], threshold: f64) -> f64 {
    if returns.is_empty() {
        return f64::NAN;
    }
    let mut gains = 0.0_f64;
    let mut losses = 0.0_f64;
    for &r in returns {
        if r > threshold {
            gains += r - threshold;
        } else {
            losses += threshold - r;
        }
    }
    if losses == 0.0 {
        return if gains > 0.0 { f64::INFINITY } else { 1.0 };
    }
    gains / losses
}

/// Gain-to-pain ratio: total return divided by total losses.
///
/// ```text
/// GtP = Σ r_i / Σ |r_i|   for r_i < 0
/// ```
///
/// Popular among CTA and systematic macro managers as a simple
/// measure of return efficiency relative to the pain of drawdowns.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
///
/// # Returns
///
/// The gain-to-pain ratio. Returns `f64::INFINITY` when total return is
/// positive but there are no losses, and [`f64::NAN`] for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::gain_to_pain;
///
/// let r = [0.05, -0.02, 0.03, -0.01, 0.04];
/// let gtp = gain_to_pain(&r);
/// assert!(gtp > 0.0);
/// ```
///
/// # References
///
/// - Schwager (2012): see docs/REFERENCES.md#schwager2012
#[must_use]
pub fn gain_to_pain(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return f64::NAN;
    }
    let total: f64 = kahan_sum(returns.iter().copied());
    let abs_losses: f64 = kahan_sum(returns.iter().filter(|&&r| r < 0.0).map(|&r| r.abs()));
    if abs_losses == 0.0 {
        return if total > 0.0 { f64::INFINITY } else { 0.0 };
    }
    total / abs_losses
}

/// Modified Sharpe ratio: excess return divided by Cornish-Fisher VaR.
///
/// Replaces the standard deviation in the Sharpe denominator with the
/// Cornish-Fisher adjusted VaR, accounting for skewness and kurtosis:
///
/// ```text
/// Modified Sharpe = (R_p − R_f) / |CF-VaR|
/// ```
///
/// # Arguments
///
/// * `returns`        - Slice of period simple returns.
/// * `risk_free_rate` - Annualized risk-free rate.
/// * `confidence`     - VaR confidence level (e.g., `0.95`).
/// * `ann_factor`     - Number of periods per year.
///
/// # Returns
///
/// The Modified Sharpe ratio. Returns `0.0` for empty slices and
/// [`f64::NAN`] when the Cornish-Fisher VaR is unexpectedly non-negative.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::modified_sharpe;
///
/// let r = [-0.06, -0.03, -0.02, 0.01, 0.02, 0.025, 0.03, 0.04];
/// let ms = modified_sharpe(&r, 0.02, 0.95, 252.0);
/// assert!(ms.is_finite());
/// ```
///
/// # References
///
/// - Gregoriou & Gueyie (2003): see docs/REFERENCES.md#gregoriou2003
#[must_use]
pub fn modified_sharpe(
    returns: &[f64],
    risk_free_rate: f64,
    confidence: f64,
    ann_factor: f64,
) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let excess_return = mean_return(returns, true, ann_factor) - risk_free_rate;
    let cf_var = cornish_fisher_var(returns, confidence, Some(ann_factor));
    if cf_var >= 0.0 {
        return f64::NAN;
    }
    excess_return / cf_var.abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::Month;
    use crate::math::stats::{mean, variance};

    fn jan1(year: i32) -> crate::dates::Date {
        crate::dates::Date::from_calendar_date(year, Month::January, 1).expect("valid date")
    }

    #[test]
    fn annualization_convention_parses_canonical_forms() {
        assert_eq!(
            "act365_25".parse::<AnnualizationConvention>().unwrap(),
            AnnualizationConvention::Act365_25
        );
        assert_eq!(
            "act365_fixed".parse::<AnnualizationConvention>().unwrap(),
            AnnualizationConvention::Act365Fixed
        );
        assert_eq!(
            "  act_act  ".parse::<AnnualizationConvention>().unwrap(),
            AnnualizationConvention::ActAct
        );
        assert!("nope".parse::<AnnualizationConvention>().is_err());
    }

    #[test]
    fn cagr_basic() {
        let r = [0.10];
        let c = cagr(&r, CagrBasis::dates(jan1(2024), jan1(2025)));
        assert!((c - 0.10).abs() < 0.01);
    }

    #[test]
    fn cagr_with_act_365_fixed() {
        let r = [0.10];
        let c = cagr(
            &r,
            CagrBasis::dates_with_convention(
                jan1(2024),
                jan1(2025),
                AnnualizationConvention::Act365Fixed,
            ),
        );
        assert!((c - 0.09971358593414137).abs() < 1.0e-12);
    }

    #[test]
    fn cagr_default_convention_is_act_365_25() {
        let r = [0.10];
        let c_default = cagr(&r, CagrBasis::dates(jan1(2024), jan1(2025)));
        let c_fixed = cagr(
            &r,
            CagrBasis::dates_with_convention(
                jan1(2024),
                jan1(2025),
                AnnualizationConvention::Act365Fixed,
            ),
        );
        assert!(c_default > c_fixed);
        assert!((c_default - 0.09978518245839707).abs() < 1.0e-12);
    }

    #[test]
    fn cagr_act_act_matches_full_leap_year() {
        let r = [0.10];
        let c = cagr(
            &r,
            CagrBasis::dates_with_convention(
                jan1(2024),
                jan1(2025),
                AnnualizationConvention::ActAct,
            ),
        );
        assert!((c - 0.10).abs() < 1.0e-12);
    }

    #[test]
    fn mean_return_volatility_nan_when_annualized_with_invalid_factor() {
        let r = [0.01_f64, 0.02];
        assert!(mean_return(&r, true, 0.0).is_nan());
        assert!(mean_return(&r, true, -1.0).is_nan());
        assert!(mean_return(&r, true, f64::NAN).is_nan());
        assert!(volatility(&r, true, 0.0).is_nan());
        assert!(volatility(&r, true, f64::INFINITY).is_nan());
    }

    #[test]
    fn downside_deviation_and_sortino_nan_when_annualized_with_invalid_factor() {
        let r = [0.01_f64, -0.02, 0.03];
        assert!(downside_deviation(&r, 0.0, true, 0.0).is_nan());
        assert!(sortino(&r, true, 0.0, 0.0).is_nan());
    }

    #[test]
    fn cagr_factor_basis_rejects_bad_ann_factor() {
        assert!(cagr(&[0.01, 0.02], CagrBasis::factor(0.0)).is_nan());
        assert!(cagr(&[0.01, 0.02], CagrBasis::factor(-1.0)).is_nan());
        assert!(cagr(&[0.01, 0.02], CagrBasis::factor(f64::NAN)).is_nan());
    }

    #[test]
    fn cagr_factor_basis_accepts_single_period() {
        assert!((cagr(&[0.10], CagrBasis::factor(1.0)) - 0.10).abs() < 1.0e-12);
    }

    #[test]
    fn mean_return_annualized_scales_linearly_not_compounded() {
        let r = [0.01, 0.02, 0.03];
        let m_ann = mean_return(&r, true, 252.0);
        let mean_p = mean(&r);
        assert!((m_ann - mean_p * 252.0).abs() < 1e-10);
        let cagr_ann = cagr(&r, CagrBasis::factor(252.0));
        assert!(
            cagr_ann.is_finite() && (m_ann - cagr_ann).abs() > 1e-6,
            "arithmetic annualized mean should differ from compounded cagr"
        );
    }

    #[test]
    fn sharpe_basic() {
        assert!((sharpe(0.10, 0.15, 0.0) - 0.6666).abs() < 0.01);
        assert_eq!(sharpe(0.10, 0.0, 0.0), f64::INFINITY);
        assert_eq!(sharpe(-0.05, 0.0, 0.0), f64::NEG_INFINITY);
        assert_eq!(sharpe(0.02, 0.0, 0.02), 0.0);
    }

    #[test]
    fn sharpe_with_risk_free_rate() {
        assert!((sharpe(0.10, 0.15, 0.02) - 0.5333).abs() < 0.01);
    }

    #[test]
    fn sortino_positive_returns() {
        let r = [0.01, 0.02, 0.03, -0.005, 0.01];
        let s = sortino(&r, false, 252.0, 0.0);
        assert!(s > 0.0);
    }

    #[test]
    fn downside_deviation_hand_calc() {
        let r = [0.01, -0.02, 0.03, -0.01, 0.005];
        let dd = downside_deviation(&r, 0.0, false, 252.0);
        assert!((dd - 0.01).abs() < 1e-14);
    }

    #[test]
    fn downside_deviation_annualized() {
        let r = [0.01, -0.02, 0.03, -0.01, 0.005];
        let dd_raw = downside_deviation(&r, 0.0, false, 252.0);
        let dd_ann = downside_deviation(&r, 0.0, true, 252.0);
        assert!((dd_ann - dd_raw * 252.0_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn downside_deviation_all_positive() {
        let dd = downside_deviation(&[0.01, 0.02, 0.03], 0.0, false, 252.0);
        assert_eq!(dd, 0.0);
    }

    #[test]
    fn downside_deviation_empty() {
        assert_eq!(downside_deviation(&[], 0.0, false, 252.0), 0.0);
    }

    #[test]
    fn downside_deviation_with_mar() {
        let r = [0.01, 0.02, 0.03, 0.005];
        let dd = downside_deviation(&r, 0.02, false, 252.0);
        let expected = (0.000325_f64 / 4.0).sqrt();
        assert!((dd - expected).abs() < 1e-14);
    }

    #[test]
    fn sortino_consistent_with_downside_deviation() {
        let r = [0.01, 0.02, 0.03, -0.005, 0.01];
        let m = mean(&r);
        let dd = downside_deviation(&r, 0.0, false, 252.0);
        let s = sortino(&r, false, 252.0, 0.0);
        assert!((s - m / dd).abs() < 1e-12);
    }

    #[test]
    fn sortino_respects_mar_in_numerator_and_denominator() {
        let r = [0.01, 0.02, 0.03, 0.04];
        let mar = 0.02;
        let expected = (mean(&r) - mar) / downside_deviation(&r, mar, false, 252.0);
        let actual = sortino(&r, false, 252.0, mar);
        assert!((actual - expected).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean_constant() {
        let gm = geometric_mean(&[0.05, 0.05, 0.05]);
        assert!((gm - 0.05).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean_volatility_drag_exact() {
        let gm = geometric_mean(&[0.10, -0.10]);
        let expected = 0.99_f64.sqrt() - 1.0;
        assert!((gm - expected).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean_empty() {
        assert!(geometric_mean(&[]).is_nan());
    }

    #[test]
    fn geometric_mean_total_wipeout_is_negative_infinity() {
        assert_eq!(geometric_mean(&[0.10, -1.0]), f64::NEG_INFINITY);
        assert_eq!(geometric_mean(&[-1.5]), f64::NEG_INFINITY);
    }

    #[test]
    fn geometric_mean_less_than_arithmetic() {
        let r = [0.05, 0.10, -0.03, 0.08];
        let gm = geometric_mean(&r);
        let am = mean(&r);
        assert!(gm < am);
    }

    #[test]
    fn omega_ratio_hand_calc() {
        let r = [0.05, -0.02, 0.03, -0.01, 0.04];
        let omega = omega_ratio(&r, 0.0);
        assert!((omega - 4.0).abs() < 1e-12);
    }

    #[test]
    fn omega_ratio_no_losses() {
        assert_eq!(omega_ratio(&[0.01, 0.02, 0.03], 0.0), f64::INFINITY);
    }

    #[test]
    fn omega_ratio_empty() {
        assert!(omega_ratio(&[], 0.0).is_nan());
    }

    #[test]
    fn gain_to_pain_hand_calc() {
        let r = [0.05, -0.02, 0.03, -0.01, 0.04];
        let gtp = gain_to_pain(&r);
        assert!((gtp - 3.0).abs() < 1e-12);
    }

    #[test]
    fn gain_to_pain_no_losses() {
        assert_eq!(gain_to_pain(&[0.01, 0.02]), f64::INFINITY);
    }

    #[test]
    fn gain_to_pain_empty() {
        assert!(gain_to_pain(&[]).is_nan());
    }

    #[test]
    fn ruin_model_reports_zero_probability_for_strictly_positive_paths() {
        let returns = [0.01; 12];
        let model = RuinModel {
            horizon_periods: 24,
            n_paths: 256,
            block_size: 4,
            seed: 7,
            confidence_level: 0.95,
        };
        let estimate = estimate_ruin(
            &returns,
            RuinDefinition::DrawdownBreach { max_drawdown: 0.20 },
            &model,
        );
        assert_eq!(estimate.probability, 0.0);
        assert_eq!(estimate.ci_lower, 0.0);
        assert!(estimate.ci_upper > 0.0);
    }

    #[test]
    fn ruin_model_reports_certain_ruin_when_every_path_breaches_floor() {
        let returns = [-0.50; 4];
        let model = RuinModel {
            horizon_periods: 1,
            n_paths: 128,
            block_size: 1,
            seed: 11,
            confidence_level: 0.95,
        };
        let estimate = estimate_ruin(
            &returns,
            RuinDefinition::WealthFloor {
                floor_fraction: 0.75,
            },
            &model,
        );
        assert_eq!(estimate.probability, 1.0);
    }

    #[test]
    fn ruin_model_is_reproducible_for_fixed_seed() {
        let returns = [0.03, -0.02, 0.01, -0.04, 0.02, -0.01];
        let model = RuinModel {
            horizon_periods: 18,
            n_paths: 512,
            block_size: 3,
            seed: 42,
            confidence_level: 0.95,
        };
        let first = estimate_ruin(
            &returns,
            RuinDefinition::TerminalFloor {
                floor_fraction: 0.85,
            },
            &model,
        );
        let second = estimate_ruin(
            &returns,
            RuinDefinition::TerminalFloor {
                floor_fraction: 0.85,
            },
            &model,
        );
        assert_eq!(first, second);
    }

    #[test]
    fn stricter_ruin_barrier_produces_higher_probability() {
        let returns = [0.04, -0.05, 0.02, -0.03, 0.01, -0.02, 0.03, -0.04];
        let model = RuinModel {
            horizon_periods: 24,
            n_paths: 1024,
            block_size: 2,
            seed: 99,
            confidence_level: 0.95,
        };
        let strict = estimate_ruin(
            &returns,
            RuinDefinition::DrawdownBreach { max_drawdown: 0.10 },
            &model,
        );
        let loose = estimate_ruin(
            &returns,
            RuinDefinition::DrawdownBreach { max_drawdown: 0.25 },
            &model,
        );
        assert!(strict.probability >= loose.probability);
    }

    #[test]
    fn modified_sharpe_is_finite_when_cf_var_is_a_loss() {
        let r = [-0.06, -0.03, -0.02, 0.01, 0.02, 0.025, 0.03, 0.04];
        let ms = modified_sharpe(&r, 0.02, 0.95, 252.0);
        assert!(ms.is_finite());
    }

    #[test]
    fn modified_sharpe_empty() {
        assert_eq!(modified_sharpe(&[], 0.02, 0.95, 252.0), 0.0);
    }

    #[test]
    fn modified_sharpe_positive_cf_var_returns_nan() {
        let r = [0.03; 12];
        let ms = modified_sharpe(&r, 0.0, 0.95, 12.0);
        assert!(ms.is_nan());
    }

    #[test]
    fn cagr_empty_is_nan() {
        assert!(cagr(&[], CagrBasis::factor(252.0)).is_nan());
    }

    #[test]
    fn parametric_var_scales_mean_and_vol_by_horizon() {
        let returns = [0.01, -0.02, 0.03, -0.01, 0.02, -0.005];
        let ann_factor = 12.0;
        let m = mean(&returns);
        let vol = variance(&returns).sqrt();
        let z = crate::math::special_functions::standard_normal_inv_cdf(0.05);
        let expected = m * ann_factor + z * vol * ann_factor.sqrt();
        let actual = crate::risk_metrics::parametric_var(&returns, 0.95, Some(ann_factor));
        assert!((actual - expected).abs() < 1e-14, "{actual} vs {expected}");
    }
}
