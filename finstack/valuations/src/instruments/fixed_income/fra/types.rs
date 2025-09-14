//! Forward Rate Agreement (FRA) instrument types and implementation.

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::{DateRange, FRAParams, MarketRefs};
use crate::instruments::traits::Attributes;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::traits::{Discount, Forward};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;

/// Forward Rate Agreement instrument.
///
/// A FRA is a forward contract on an interest rate. The holder receives
/// the difference between the realized rate and the fixed rate, paid at
/// the start of the interest period (FRA convention).
#[derive(Clone, Debug)]
pub struct ForwardRateAgreement {
    /// Unique identifier
    pub id: String,
    /// Notional amount
    pub notional: Money,
    /// Rate fixing date (start of interest period)
    pub fixing_date: Date,
    /// Interest period start date
    pub start_date: Date,
    /// Interest period end date
    pub end_date: Date,
    /// Fixed rate (decimal, e.g., 0.05 for 5%)
    pub fixed_rate: F,
    /// Day count convention for interest accrual
    pub day_count: DayCount,
    /// Reset lag in business days (fixing to value date)
    pub reset_lag: i32,
    /// Discount curve identifier
    pub disc_id: &'static str,
    /// Forward curve identifier
    pub forward_id: &'static str,
    /// Pay/receive flag (true = receive fixed, pay floating)
    pub pay_fixed: bool,
    /// Attributes for scenario selection
    pub attributes: Attributes,
}

impl ForwardRateAgreement {
    /// Create a new FRA using parameter structs.
    pub fn new(
        id: impl Into<String>,
        fra_params: &FRAParams,
        date_range: &DateRange,
        market_refs: &MarketRefs,
    ) -> Self {
        let forward_id = market_refs
            .fwd_id
            .as_ref()
            .expect("Forward curve required for FRA");

        Self {
            id: id.into(),
            notional: fra_params.notional,
            fixing_date: fra_params.fixing_date,
            start_date: date_range.start,
            end_date: date_range.end,
            fixed_rate: fra_params.fixed_rate,
            day_count: fra_params.day_count,
            reset_lag: 2, // Standard T+2 settlement
            disc_id: Box::leak(market_refs.disc_id.to_string().into_boxed_str()),
            forward_id: Box::leak(forward_id.to_string().into_boxed_str()),
            pay_fixed: false, // Default to receive fixed
            attributes: Attributes::new(),
        }
    }

    /// Set pay/receive direction.
    pub fn with_pay_fixed(mut self, pay_fixed: bool) -> Self {
        self.pay_fixed = pay_fixed;
        self
    }

    /// Set reset lag.
    pub fn with_reset_lag(mut self, reset_lag: i32) -> Self {
        self.reset_lag = reset_lag;
        self
    }

    /// Calculate FRA value using market curves.
    pub fn fra_value(
        &self,
        discount_curve: &dyn Discount,
        forward_curve: &dyn Forward,
        _as_of: Date,
    ) -> finstack_core::Result<Money> {
        // Calculate time fractions
        let base_date = discount_curve.base_date();
        let t_fixing = self
            .day_count
            .year_fraction(
                base_date,
                self.fixing_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let t_start = self
            .day_count
            .year_fraction(
                base_date,
                self.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let t_end = self
            .day_count
            .year_fraction(
                base_date,
                self.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        // Interest period length
        let tau = self
            .day_count
            .year_fraction(
                self.start_date,
                self.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        if tau <= 0.0 || t_fixing < 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        // Get forward rate for the period
        let forward_rate = forward_curve.rate_period(t_start, t_end);

        // Get discount factor to settlement date (start of interest period for FRAs)
        let df_settlement = discount_curve.df(t_start);

        // FRA payoff: (Forward - Fixed) * tau * Notional * DF
        // Discounted to start of period (FRA convention)
        let rate_diff = forward_rate - self.fixed_rate;
        let pv = self.notional.amount() * rate_diff * tau * df_settlement;

        // Apply pay/receive direction
        let signed_pv = if self.pay_fixed { -pv } else { pv };

        Ok(Money::new(signed_pv, self.notional.currency()))
    }
}

impl_instrument!(
    ForwardRateAgreement,
    "FRA",
    pv = |s, curves, as_of| {
        let discount_curve = curves.disc(s.disc_id)?;
        let forward_curve = curves.fwd(s.forward_id)?;
        s.fra_value(discount_curve.as_ref(), forward_curve.as_ref(), as_of)
    }
);

impl CashflowProvider for ForwardRateAgreement {
    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, Money)>> {
        // FRA settlement is at start of interest period
        if self.start_date <= as_of {
            return Ok(vec![]); // Already settled
        }

        // Calculate the FRA settlement amount
        let pv = self.fra_value(
            curves.disc(self.disc_id)?.as_ref(),
            curves.fwd(self.forward_id)?.as_ref(),
            as_of,
        )?;

        Ok(vec![(self.start_date, pv)])
    }
}
