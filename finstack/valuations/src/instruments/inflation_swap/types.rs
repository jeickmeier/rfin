//! Zero-coupon Inflation Swap types and pricing implementation.

use crate::instruments::common::traits::Attributes;
use finstack_core::market_data::scalars::inflation_index::InflationLag;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use finstack_core::types::{CurveId, InstrumentId};
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
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct InflationSwap {
    /// Unique instrument identifier
    pub id: InstrumentId,
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
    pub disc_id: CurveId,
    /// Day count for any accrual-style metrics if needed
    pub dc: DayCount,
    /// Trade side
    pub side: PayReceiveInflation,
    /// Optional contract-level lag override (if set, overrides index lag)
    #[builder(optional)]
    pub lag_override: Option<InflationLag>,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl InflationSwap {}

impl InflationSwap {
    /// Calculate PV of the fixed leg (real rate leg)
    pub fn pv_fixed_leg(
        &self,
        curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<Money> {
        let disc = curves.get_discount_ref(self.disc_id.as_str())?;
        let base = disc.base_date();

        let tau_accrual = self.dc.year_fraction(
            self.start,
            self.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        let fixed_payment = self.notional * ((1.0 + self.fixed_rate).powf(tau_accrual) - 1.0);

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
        _as_of: Date,
    ) -> finstack_core::Result<Money> {
        let disc = curves.get_discount_ref(self.disc_id.as_str())?;
        let base = disc.base_date();

        let inflation_index = curves
            .inflation_index_ref(self.inflation_id)
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "inflation_index".to_string(),
                })
            })?;

        let inflation_curve = curves.get_inflation_ref(self.inflation_id)?;

        let i_start = inflation_index.value_on(self.start)?;

        // Apply the same lag policy as the index to the maturity for projection
        // Use contract override if present, else index lag
        let lag_policy = self.lag_override.unwrap_or(inflation_index.lag());
        let lagged_maturity = match lag_policy {
            finstack_core::market_data::scalars::inflation_index::InflationLag::None => {
                self.maturity
            }
            finstack_core::market_data::scalars::inflation_index::InflationLag::Months(m) => {
                finstack_core::dates::add_months(self.maturity, -(m as i32))
            }
            finstack_core::market_data::scalars::inflation_index::InflationLag::Days(d) => {
                self.maturity - time::Duration::days(d as i64)
            }
            _ => self.maturity,
        };

        // Use a common base (discount base) to derive time in years for the inflation curve
        let t_maturity_infl = DayCount::Act365F
            .year_fraction(
                base,
                lagged_maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let i_maturity_projected = inflation_curve.cpi(t_maturity_infl);

        let inflation_payment = self.notional * (i_maturity_projected / i_start - 1.0);

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

    /// Net present value of the instrument via legs
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        let pv_fixed = self.pv_fixed_leg(curves, as_of)?;
        let pv_inflation = self.pv_inflation_leg(curves, as_of)?;
        match self.side {
            PayReceiveInflation::ReceiveFixed => pv_fixed - pv_inflation,
            PayReceiveInflation::PayFixed => pv_inflation - pv_fixed,
        }
    }
}

impl_instrument!(
    InflationSwap,
    crate::pricer::InstrumentType::InflationSwap,
    "InflationSwap",
    pv = |s, curves, as_of| {
        // Call the instrument's own npv method
        s.npv(curves, as_of)
    },
);

impl crate::instruments::common::HasDiscountCurve for InflationSwap {
    fn discount_curve_id(&self) -> &CurveId {
        &self.disc_id
    }
}
