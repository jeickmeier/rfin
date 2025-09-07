//! Zero-coupon Inflation Swap types and pricing implementation.

use crate::instruments::traits::Attributes;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use finstack_core::F;

/// Direction from the perspective of paying fixed real vs receiving inflation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayReceiveInflation {
    /// Pay fixed (real) leg, receive inflation leg
    PayFixed,
    /// Receive fixed (real) leg, pay inflation leg
    ReceiveFixed,
}

/// Inflation swap definition (boilerplate)
///
/// Minimal fields to represent a zero-coupon inflation swap. We keep this
/// intentionally compact until full pricing is implemented.
#[derive(Clone, Debug)]
pub struct InflationSwap {
    /// Unique instrument identifier
    pub id: String,
    /// Notional in quote currency
    pub notional: Money,
    /// Start date of indexation
    pub start: Date,
    /// Maturity date
    pub maturity: Date,
    /// Fixed real rate (as decimal)
    pub fixed_rate: F,
    /// Inflation index identifier (e.g., US-CPI-U)
    pub inflation_id: &'static str,
    /// Discount curve identifier (quote currency)
    pub disc_id: &'static str,
    /// Day count for any accrual-style metrics if needed
    pub dc: DayCount,
    /// Trade side
    pub side: PayReceiveInflation,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl InflationSwap {
    /// Builder entrypoint
    pub fn builder() -> crate::instruments::fixed_income::inflation_swap::builder::InflationSwapBuilder {
        crate::instruments::fixed_income::inflation_swap::builder::InflationSwapBuilder::new()
    }
}

impl InflationSwap {
    /// Calculate PV of the fixed leg (real rate leg)
    pub fn pv_fixed_leg(&self, curves: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
        let disc = curves.disc(self.disc_id)?;
        let base = disc.base_date();

        // Year fraction for the full term of the swap
        let tau_accrual = self.dc.year_fraction(
            self.start,
            self.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        // Fixed payment at maturity: N * ((1 + K)^tau - 1)
        let fixed_payment = self.notional * ((1.0 + self.fixed_rate).powf(tau_accrual) - 1.0);

        // Discount factor from as_of to maturity
        let t_discount = DayCount::Act365F
            .year_fraction(
                base,
                self.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df = disc.df(t_discount);

        Ok(fixed_payment * df)
    }

    /// Calculate PV of the inflation leg
    pub fn pv_inflation_leg(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let disc = curves.disc(self.disc_id)?;
        let base = disc.base_date();

        // Get inflation index for historical reference value
        let inflation_index = curves.inflation_index(self.inflation_id).ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "inflation_index".to_string(),
            })
        })?;

        // Get inflation curve for forward projection
        let inflation_curve = curves.infl(self.inflation_id)?;

        // Historical index value at start (with any lag applied by the index)
        let i_start = inflation_index.value_on(self.start)?;

        // Project inflation index value at maturity
        let t_maturity = DayCount::Act365F
            .year_fraction(
                as_of,
                self.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let i_maturity_projected = inflation_curve.cpi(t_maturity);

        // Inflation payment at maturity: N * (I(T_mat)/I(T_start) - 1)
        let inflation_payment = self.notional * (i_maturity_projected / i_start - 1.0);

        // Discount factor from as_of to maturity
        let t_discount = DayCount::Act365F
            .year_fraction(
                base,
                self.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df = disc.df(t_discount);

        Ok(inflation_payment * df)
    }
}

impl_instrument!(
    InflationSwap,
    "InflationSwap",
    pv = |s, curves, as_of| {
        // Calculate PV of both legs
        let pv_fixed = s.pv_fixed_leg(curves, as_of)?;
        let pv_inflation = s.pv_inflation_leg(curves, as_of)?;

        // Net PV based on trade direction
        match s.side {
            PayReceiveInflation::ReceiveFixed => pv_fixed - pv_inflation,
            PayReceiveInflation::PayFixed => pv_inflation - pv_fixed,
        }
    },
);


