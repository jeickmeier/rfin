//! Two-clock plumbing for pricers that combine a vol-surface clock with
//! a discount-curve clock.
//!
//! # Background
//!
//! Many Monte Carlo and closed-form pricers in this crate use the
//! pattern:
//!
//! ```text
//! let t_vol = inst.day_count.year_fraction(as_of, expiry)?;
//! let df = disc_curve.df_between_dates(as_of, expiry)?;
//! let r_eff = -df.ln() / t_vol;   // WRONG when day counts differ
//! ```
//!
//! The resulting `r_eff` is then used as the drift in a GBM / Heston /
//! barrier simulation. This is only correct when the instrument's
//! day-count convention (the basis the vol surface was calibrated on)
//! matches the discount curve's own day-count convention. When they
//! differ — e.g. ACT/365F on the vol surface and ACT/360 on the curve
//! — `r_eff` carries a small but nonzero bias that breaks
//! bump-and-reval consistency with the curve.
//!
//! # The two-clock convention
//!
//! The fix is to thread **both** clocks through:
//!
//! * `t_vol` — year fraction on the **instrument / vol-surface** day
//!   count. Drives the time grid for MC simulation and vol-surface
//!   lookups (so that `σ²·t_vol` stays consistent with how the surface
//!   was stripped).
//! * `t_disc` — year fraction on the **discount curve's** day count.
//!   Used *only* to compute the effective drift rate
//!   `r_disc = -ln(df) / t_disc` for the simulated process.
//! * `df` — the exact discount factor read from the curve. Applied
//!   directly to the final payoff, never back-computed from a rate.
//!
//! With this split, the final price is bump-and-reval-consistent: a
//! parallel shift of the curve moves `df` exactly as the curve's own
//! accrual logic expects, and the drift tracks.
//!
//! # Migration status
//!
//! [`TwoClockParams`] is the landing helper for the migration. Some
//! pricers still compute `r_eff = -ln(DF)/t_vol` inline; these will be
//! migrated one pricer at a time.
//!
//! # References
//!
//! - Hull, J.C. *Options, Futures, and Other Derivatives* (Ch. 15 on
//!   risk-neutral pricing under distinct rate and measurement
//!   conventions).

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::term_structures::DiscountCurve;

/// Bundled two-clock pricing inputs.
///
/// See the module docs for the motivation. Construct via
/// [`TwoClockParams::from_curve_and_instrument`] whenever a pricer has
/// access to the discount curve and the instrument's day-count.
///
/// All fields are public so pricers can read them directly in the hot
/// loop without indirection.
#[derive(Debug, Clone, Copy)]
pub struct TwoClockParams {
    /// Year fraction from `as_of` to `expiry` on the **discount curve's**
    /// day-count convention. Used to compute the drift rate
    /// `r_disc = -ln(df) / t_disc`.
    pub t_disc: f64,
    /// Year fraction from `as_of` to `expiry` on the **instrument's**
    /// (vol-surface) day-count convention. Drives the MC time grid and
    /// vol-surface lookups.
    pub t_vol: f64,
    /// Exact discount factor `P(as_of, expiry)` from the curve.
    pub df: f64,
}

impl TwoClockParams {
    /// Construct from the curve + instrument day-count + dates.
    ///
    /// Returns both year fractions (on their respective day counts) and
    /// the exact discount factor from the curve.
    ///
    /// # Errors
    ///
    /// Returns an error if either day-count computation or the
    /// discount factor lookup fails.
    pub fn from_curve_and_instrument(
        disc_curve: &DiscountCurve,
        instrument_day_count: DayCount,
        as_of: Date,
        expiry: Date,
    ) -> finstack_core::Result<Self> {
        let t_disc = disc_curve
            .day_count()
            .year_fraction(as_of, expiry, DayCountCtx::default())?;
        let t_vol = instrument_day_count.year_fraction(as_of, expiry, DayCountCtx::default())?;
        let df = disc_curve.df_between_dates(as_of, expiry)?;
        Ok(Self { t_disc, t_vol, df })
    }

    /// Drift rate consistent with the curve's own day-count convention.
    ///
    /// Returns `0.0` for non-positive `t_disc` or non-positive `df` —
    /// both degenerate inputs are handled as "no drift" rather than
    /// producing a non-finite rate.
    #[inline]
    pub fn r_disc(&self) -> f64 {
        if self.t_disc > 0.0 && self.df > 0.0 {
            -self.df.ln() / self.t_disc
        } else {
            0.0
        }
    }

    /// Legacy single-clock effective rate `r_eff = -ln(df) / t_vol`.
    ///
    /// Kept ONLY to preserve numerical behaviour during migration — new
    /// code should call [`r_disc`](Self::r_disc) instead. When the
    /// instrument and curve share a day-count convention the two are
    /// identical, which is the regression invariant pricers can test.
    #[allow(dead_code)] // Referenced by unit tests; also kept for migration audit trail.
    #[inline]
    pub fn r_eff_legacy(&self) -> f64 {
        if self.t_vol > 0.0 && self.df > 0.0 {
            -self.df.ln() / self.t_vol
        } else {
            0.0
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// When t_disc == t_vol (identical day counts), `r_disc` and
    /// `r_eff_legacy` must be bit-identical. This is the invariant
    /// every pricer migration must preserve.
    #[test]
    fn r_disc_matches_legacy_when_day_counts_match() {
        let p = TwoClockParams {
            t_disc: 2.0,
            t_vol: 2.0,
            df: 0.92,
        };
        assert_eq!(p.r_disc().to_bits(), p.r_eff_legacy().to_bits());
    }

    /// When t_disc != t_vol the drift rate MUST differ — proving the
    /// helper actually carries both clocks and isn't a trivial alias.
    #[test]
    fn r_disc_differs_from_legacy_when_day_counts_differ() {
        // ACT/365F (~2.0) vs ACT/360 (~2.0277) style mismatch.
        let p = TwoClockParams {
            t_disc: 2.0_f64,
            t_vol: 2.0_f64 * 365.0 / 360.0,
            df: 0.92,
        };
        let r_disc = p.r_disc();
        let r_legacy = p.r_eff_legacy();
        assert!(
            (r_disc - r_legacy).abs() > 1e-10,
            "r_disc={r_disc}, r_legacy={r_legacy} should differ under day-count mismatch"
        );
        // And the curve-consistent rate is the larger one for this
        // sign convention (shorter clock → higher rate for the same DF).
        assert!(
            r_disc > r_legacy,
            "curve rate should exceed vol-surface rate for t_disc < t_vol"
        );
    }

    /// Non-positive clocks / DFs return 0.0 (defensive defaults).
    #[test]
    fn degenerate_inputs_return_zero_rate() {
        let p_zero_t = TwoClockParams {
            t_disc: 0.0,
            t_vol: 0.0,
            df: 0.95,
        };
        assert_eq!(p_zero_t.r_disc(), 0.0);
        assert_eq!(p_zero_t.r_eff_legacy(), 0.0);

        let p_zero_df = TwoClockParams {
            t_disc: 1.0,
            t_vol: 1.0,
            df: 0.0,
        };
        assert_eq!(p_zero_df.r_disc(), 0.0);
        assert_eq!(p_zero_df.r_eff_legacy(), 0.0);
    }
}
