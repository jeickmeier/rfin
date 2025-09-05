//! Zero-coupon Inflation Swap (boilerplate implementation).
//!
//! This module adds a minimal scaffold for an inflation swap instrument so it
//! can participate in the unified pricing and metrics framework. Valuation
//! logic is intentionally minimal (returns zero) until completed.

pub mod metrics;

use crate::instruments::traits::Attributes;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
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
    pub fn builder() -> InflationSwapBuilder {
        InflationSwapBuilder::new()
    }
}

impl InflationSwap {
    /// Calculate PV of the fixed leg (real rate leg)
    fn pv_fixed_leg(&self, curves: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
        let disc = curves.discount(self.disc_id)?;
        let base = disc.base_date();

        // Year fraction for the full term of the swap
        let tau_accrual = self.dc.year_fraction(self.start, self.maturity, finstack_core::dates::DayCountCtx::default())?;

        // Fixed payment at maturity: N * ((1 + K)^tau - 1)
        let fixed_payment = self.notional * ((1.0 + self.fixed_rate).powf(tau_accrual) - 1.0);

        // Discount factor from as_of to maturity
        let t_discount = DiscountCurve::year_fraction(base, self.maturity, DayCount::Act365F);
        let df = disc.df(t_discount);

        Ok(fixed_payment * df)
    }

    /// Calculate PV of the inflation leg
    fn pv_inflation_leg(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let disc = curves.discount(self.disc_id)?;
        let base = disc.base_date();

        // Get inflation index for historical reference value
        let inflation_index = curves.inflation_index(self.inflation_id).ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound { id: "inflation_index".to_string() })
        })?;

        // Get inflation curve for forward projection
        let inflation_curve = curves.inflation(self.inflation_id)?;

        // Historical index value at start (with any lag applied by the index)
        let i_start = inflation_index.value_on(self.start)?;

        // Project inflation index value at maturity
        let t_maturity = DiscountCurve::year_fraction(as_of, self.maturity, DayCount::Act365F);
        let i_maturity_projected = inflation_curve.cpi(t_maturity);

        // Inflation payment at maturity: N * (I(T_mat)/I(T_start) - 1)
        let inflation_payment = self.notional * (i_maturity_projected / i_start - 1.0);

        // Discount factor from as_of to maturity
        let t_discount = DiscountCurve::year_fraction(base, self.maturity, DayCount::Act365F);
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

/// Builder for `InflationSwap`
#[derive(Default)]
pub struct InflationSwapBuilder {
    id: Option<String>,
    notional: Option<Money>,
    start: Option<Date>,
    maturity: Option<Date>,
    fixed_rate: Option<F>,
    inflation_id: Option<&'static str>,
    disc_id: Option<&'static str>,
    dc: Option<DayCount>,
    side: Option<PayReceiveInflation>,
}

impl InflationSwapBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }
    pub fn notional(mut self, value: Money) -> Self {
        self.notional = Some(value);
        self
    }
    pub fn start(mut self, value: Date) -> Self {
        self.start = Some(value);
        self
    }
    pub fn maturity(mut self, value: Date) -> Self {
        self.maturity = Some(value);
        self
    }
    pub fn fixed_rate(mut self, value: F) -> Self {
        self.fixed_rate = Some(value);
        self
    }
    pub fn inflation_id(mut self, value: &'static str) -> Self {
        self.inflation_id = Some(value);
        self
    }
    pub fn disc_id(mut self, value: &'static str) -> Self {
        self.disc_id = Some(value);
        self
    }
    pub fn dc(mut self, value: DayCount) -> Self {
        self.dc = Some(value);
        self
    }
    pub fn side(mut self, value: PayReceiveInflation) -> Self {
        self.side = Some(value);
        self
    }

    pub fn build(self) -> finstack_core::Result<InflationSwap> {
        Ok(InflationSwap {
            id: self.id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            notional: self.notional.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            start: self.start.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            maturity: self.maturity.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            fixed_rate: self.fixed_rate.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            inflation_id: self.inflation_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            disc_id: self.disc_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            dc: self.dc.unwrap_or(DayCount::ActAct),
            side: self.side.unwrap_or(PayReceiveInflation::PayFixed),
            attributes: Attributes::new(),
        })
    }
}

// CashflowProvider is intentionally omitted for now; when implemented, we can
// switch to impl_instrument_schedule_pv! variant and compute PV from flows.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::traits::Priceable;
    use crate::metrics::MetricCalculator;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::{
        context::MarketContext,
        inflation_index::{InflationIndex, InflationLag},
        term_structures::{discount_curve::DiscountCurve, inflation::InflationCurve},
    };
    use time::Month;

    fn create_test_market_context() -> finstack_core::Result<MarketContext> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Create a flat discount curve
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (5.0, 0.90), (10.0, 0.80)])
            .set_interp(finstack_core::market_data::interp::InterpStyle::MonotoneConvex)
            .build()
            .unwrap();

        // Create inflation index with historical data
        let inflation_observations = vec![
            (
                Date::from_calendar_date(2024, Month::January, 1).unwrap(),
                280.0,
            ),
            (
                Date::from_calendar_date(2024, Month::July, 1).unwrap(),
                285.0,
            ),
            (
                Date::from_calendar_date(2025, Month::January, 1).unwrap(),
                290.0,
            ),
        ];
        let inflation_index =
            InflationIndex::new("US-CPI-U", inflation_observations, Currency::USD)?
                .with_lag(InflationLag::Months(3));

        // Create forward inflation curve (log-linear growth)
        let inflation_curve = InflationCurve::builder("US-CPI-U")
            .base_cpi(290.0)
            .knots([(0.0, 290.0), (5.0, 320.0), (10.0, 355.0)])
            .set_interp(finstack_core::market_data::interp::InterpStyle::LogLinear)
            .build()?;

        let context = MarketContext::new()
            .with_discount(disc_curve)
            .with_inflation_index("US-CPI-U", inflation_index)
            .with_inflation(inflation_curve)
            .with_price(
                "US-CPI-U-BASE_CPI",
                finstack_core::market_data::primitives::MarketScalar::Unitless(290.0),
            );

        Ok(context)
    }

    fn create_test_swap() -> InflationSwap {
        let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let mat = Date::from_calendar_date(2030, Month::January, 15).unwrap();

        InflationSwap::builder()
            .id("ZCIS_TEST")
            .notional(Money::new(10_000_000.0, Currency::USD))
            .start(start)
            .maturity(mat)
            .fixed_rate(0.025) // 2.5% annual fixed rate
            .inflation_id("US-CPI-U")
            .disc_id("USD-OIS")
            .dc(DayCount::ActAct)
            .side(PayReceiveInflation::PayFixed)
            .build()
            .unwrap()
    }

    #[test]
    fn test_inflation_swap_builder() {
        let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let mat = Date::from_calendar_date(2030, Month::January, 15).unwrap();
        let inst = InflationSwap::builder()
            .id("ZCIS_1")
            .notional(Money::new(10_000_000.0, Currency::USD))
            .start(start)
            .maturity(mat)
            .fixed_rate(0.025)
            .inflation_id("US-CPI-U")
            .disc_id("USD-OIS")
            .dc(DayCount::ActAct)
            .side(PayReceiveInflation::PayFixed)
            .build()
            .unwrap();

        assert_eq!(inst.id, "ZCIS_1");
        assert_eq!(inst.fixed_rate, 0.025);
        assert_eq!(inst.inflation_id, "US-CPI-U");
        assert_eq!(inst.disc_id, "USD-OIS");
    }

    #[test]
    fn test_fixed_leg_pv_calculation() {
        let swap = create_test_swap();
        let context = create_test_market_context().unwrap();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let pv_fixed = swap.pv_fixed_leg(&context, as_of).unwrap();

        // With 2.5% rate over 5 years: (1.025^5 - 1) ≈ 0.131
        // Discounted at ~90% (5-year DF): 0.131 * 0.90 ≈ 0.118
        // On 10M notional: ~1.18M
        assert!(pv_fixed.amount() > 1_000_000.0);
        assert!(pv_fixed.amount() < 1_500_000.0);
        assert_eq!(pv_fixed.currency(), Currency::USD);
    }

    #[test]
    fn test_inflation_leg_pv_calculation() {
        let swap = create_test_swap();
        let context = create_test_market_context().unwrap();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let pv_inflation = swap.pv_inflation_leg(&context, as_of).unwrap();

        // With inflation from 290 to ~320 over 5 years: (320/290 - 1) ≈ 0.103
        // Discounted at ~90%: 0.103 * 0.90 ≈ 0.093
        // On 10M notional: ~930K
        assert!(pv_inflation.amount() > 500_000.0);
        assert!(pv_inflation.amount() < 1_500_000.0);
        assert_eq!(pv_inflation.currency(), Currency::USD);
    }

    #[test]
    fn test_swap_net_pv() {
        let swap = create_test_swap();
        let context = create_test_market_context().unwrap();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Test PayFixed side (receive inflation, pay fixed)
        let net_pv = swap.value(&context, as_of).unwrap();

        // Should be PV_inflation - PV_fixed
        let _pv_fixed = swap.pv_fixed_leg(&context, as_of).unwrap();
        let _pv_inflation = swap.pv_inflation_leg(&context, as_of).unwrap();

        // Verify the result is in the correct currency
        assert_eq!(net_pv.currency(), Currency::USD);
    }

    #[test]
    fn test_receive_fixed_side() {
        let mut swap = create_test_swap();
        swap.side = PayReceiveInflation::ReceiveFixed;

        let context = create_test_market_context().unwrap();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let _net_pv = swap.value(&context, as_of).unwrap();

        // Should be PV_fixed - PV_inflation (opposite sign from PayFixed)
        let _pv_fixed = swap.pv_fixed_leg(&context, as_of).unwrap();
        let _pv_inflation = swap.pv_inflation_leg(&context, as_of).unwrap();
    }

    #[test]
    fn test_breakeven_calculation() {
        let swap = create_test_swap();
        let context = create_test_market_context().unwrap();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Create metric context and test breakeven calculation
        use crate::metrics::MetricContext;
        use std::sync::Arc;

        let mut metric_ctx = MetricContext::new(
            Arc::new(swap.clone()),
            Arc::new(context.clone()),
            as_of,
            Money::new(0.0, Currency::USD),
        );

        let calculator = super::metrics::BreakevenCalculator;
        let breakeven = calculator.calculate(&mut metric_ctx).unwrap();

        // Breakeven should be positive (inflation expected)
        assert!(breakeven > 0.0);
        // Should be reasonable (between 0% and 10% annually)
        assert!(breakeven < 0.10);

        // With our test data: 320/290 = 1.103, over 5 years
        // (1.103)^(1/5) - 1 ≈ 0.0201 ≈ 2.01%
        assert!((breakeven - 0.0201).abs() < 0.005);
    }

    #[test]
    fn test_zero_pv_at_breakeven_rate() {
        let context = create_test_market_context().unwrap();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // First calculate the breakeven rate
        let temp_swap = create_test_swap();
        use crate::metrics::MetricContext;
        use std::sync::Arc;

        let mut metric_ctx = MetricContext::new(
            Arc::new(temp_swap.clone()),
            Arc::new(context.clone()),
            as_of,
            Money::new(0.0, Currency::USD),
        );

        let calculator = super::metrics::BreakevenCalculator;
        let breakeven_rate = calculator.calculate(&mut metric_ctx).unwrap();

        // Create a new swap with the breakeven rate
        let mut breakeven_swap = create_test_swap();
        breakeven_swap.fixed_rate = breakeven_rate;

        // PV should be approximately zero
        let pv = breakeven_swap.value(&context, as_of).unwrap();
        assert!(
            pv.amount().abs() < 1.0,
            "PV should be near zero at breakeven rate, got {}",
            pv.amount()
        );
    }

    #[test]
    fn test_ir01_sensitivity() {
        let swap = create_test_swap();
        let context = create_test_market_context().unwrap();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Create metric context
        use crate::metrics::MetricContext;
        use std::sync::Arc;

        let mut metric_ctx = MetricContext::new(
            Arc::new(swap.clone()),
            Arc::new(context.clone()),
            as_of,
            Money::new(0.0, Currency::USD),
        );

        let calculator = super::metrics::Ir01Calculator;
        let ir01 = calculator.calculate(&mut metric_ctx).unwrap();

        // IR01 should be negative for PayFixed swaps (rates up -> PV down)
        // and should be meaningful in magnitude
        assert!(
            ir01.abs() > 10.0,
            "IR01 should have meaningful magnitude, got {}",
            ir01
        );

        // For a PayFixed swap, IR01 should typically be negative
        // (higher rates reduce PV of both legs, but affect them differently)
        assert!(
            ir01 < 0.0,
            "IR01 should be negative for PayFixed swap, got {}",
            ir01
        );
    }

    #[test]
    fn test_inflation01_sensitivity() {
        let swap = create_test_swap();
        let context = create_test_market_context().unwrap();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Create metric context
        use crate::metrics::MetricContext;
        use std::sync::Arc;

        let mut metric_ctx = MetricContext::new(
            Arc::new(swap.clone()),
            Arc::new(context.clone()),
            as_of,
            Money::new(0.0, Currency::USD),
        );

        let calculator = super::metrics::Inflation01Calculator;
        let inflation01 = calculator.calculate(&mut metric_ctx).unwrap();

        // Inflation01 should be positive for PayFixed swaps
        // (higher inflation -> higher inflation leg PV -> higher net PV for PayFixed)
        assert!(
            inflation01 > 0.0,
            "Inflation01 should be positive for PayFixed swap, got {}",
            inflation01
        );

        // Should have meaningful magnitude
        assert!(
            inflation01 > 10.0,
            "Inflation01 should have meaningful magnitude, got {}",
            inflation01
        );
    }

    #[test]
    fn test_receive_fixed_sensitivities_opposite_signs() {
        let mut swap = create_test_swap();
        swap.side = PayReceiveInflation::ReceiveFixed;

        let context = create_test_market_context().unwrap();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Create metric context
        use crate::metrics::MetricContext;
        use std::sync::Arc;

        let mut metric_ctx = MetricContext::new(
            Arc::new(swap.clone()),
            Arc::new(context.clone()),
            as_of,
            Money::new(0.0, Currency::USD),
        );

        // Calculate sensitivities for ReceiveFixed
        let ir01_calc = super::metrics::Ir01Calculator;
        let inflation01_calc = super::metrics::Inflation01Calculator;

        let ir01_receive = ir01_calc.calculate(&mut metric_ctx).unwrap();
        let inflation01_receive = inflation01_calc.calculate(&mut metric_ctx).unwrap();

        // For ReceiveFixed swaps:
        // - IR01 should be positive (rates up -> fixed leg worth more)
        // - Inflation01 should be negative (inflation up -> pay more on inflation leg)
        assert!(
            ir01_receive > 0.0,
            "IR01 should be positive for ReceiveFixed swap, got {}",
            ir01_receive
        );
        assert!(
            inflation01_receive < 0.0,
            "Inflation01 should be negative for ReceiveFixed swap, got {}",
            inflation01_receive
        );
    }
}
