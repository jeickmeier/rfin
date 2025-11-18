//! Interest Rate Swap (IRS) types and instrument trait implementations.
//!
//! Defines the `InterestRateSwap` instrument following the modern instrument
//! standards used across valuations: types live here; pricing is delegated to
//! `pricing::engine`; and metrics are split under `metrics/`.
//!
//! Public fields use strong newtype identifiers for safety: `InstrumentId` and
//! `CurveId`. Calendar identifiers remain `Option<&'static str>` for stable
//! serde and lookups.
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::traits::Attributes;
// Risk types used in risk.rs

// Re-export common enums from parameters
pub use crate::instruments::common::parameters::legs::{ParRateMethod, PayReceive};

// Re-export from common parameters
pub use crate::instruments::common::parameters::legs::FixedLegSpec;
pub use crate::instruments::common::parameters::legs::FloatLegSpec;

/// Interest rate swap with fixed and floating legs.
///
/// Represents a standard interest rate swap where one party pays
/// a fixed rate and the other pays a floating rate plus spread.
///
/// # Market Standards & Citations (Week 5)
///
/// ## ISDA Definitions
///
/// This implementation follows the **ISDA 2006 Definitions** for interest rate derivatives:
/// - **Section 4.1:** Fixed Rate Payer calculation conventions
/// - **Section 4.2:** Floating Rate Option conventions
/// - **Section 4.5:** Compounding methods
/// - **Section 4.16:** Business Day Conventions
///
/// ## USD Market Standard (Default)
///
/// Per **ISDA 2006 Definitions** and US market practice:
/// - **Fixed Leg:** Semi-annual, 30/360, Modified Following
/// - **Floating Leg:** Quarterly, ACT/360, Modified Following
/// - **Reset Lag:** T-2 (2 business days before period start)
/// - **Discounting:** OIS curve (post-2008 multi-curve framework)
///
/// ## References
///
/// - ISDA 2006 Definitions (incorporating 2008 Supplement for OIS)
/// - "Interest Rate Swaps and Their Derivatives" by Amir Sadr
/// - Bloomberg SWPM function documentation
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct InterestRateSwap {
    /// Unique identifier for the swap.
    pub id: InstrumentId,
    /// Notional amount for both legs.
    pub notional: Money,
    /// Direction of the swap (PayFixed or ReceiveFixed).
    pub side: PayReceive,
    /// Fixed leg specification.
    pub fixed: FixedLegSpec,
    /// Floating leg specification.
    pub float: FloatLegSpec,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

/// Configuration for standard swap construction.
struct SwapConfig<'a> {
    disc_curve: &'a str,
    fwd_curve: &'a str,
    reset_lag_days: i32,
    sched: IRSScheduleConfig,
}

/// Schedule configuration with separate fixed and float leg parameters
struct IRSScheduleConfig {
    fixed_freq: finstack_core::dates::Frequency,
    fixed_dc: finstack_core::dates::DayCount,
    float_freq: finstack_core::dates::Frequency,
    float_dc: finstack_core::dates::DayCount,
    bdc: finstack_core::dates::BusinessDayConvention,
    calendar_id: Option<String>,
    stub: finstack_core::dates::StubKind,
}

impl IRSScheduleConfig {
    /// USD market standard: Fixed semiannual 30/360; Float quarterly Act/360
    fn usd_isda_standard() -> Self {
        use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
        Self {
            fixed_freq: Frequency::semi_annual(),
            fixed_dc: DayCount::Thirty360,
            float_freq: Frequency::quarterly(),
            float_dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("USD".to_string()),
            stub: StubKind::None,
        }
    }
}

impl InterestRateSwap {
    /// Create a standard USD OIS-discounted IRS using ISDA market conventions.
    ///
    /// This is the primary convenience constructor used throughout tests and
    /// examples. It builds a vanilla fixed-vs-floating swap with:
    /// - Discount curve: `USD-OIS`
    /// - Forward curve: `USD-SOFR-3M`
    /// - Fixed leg: semi-annual, 30/360, Modified Following
    /// - Float leg: quarterly, ACT/360, Modified Following, 2-day reset lag
    pub fn create_usd_swap(
        id: InstrumentId,
        notional: Money,
        fixed_rate: f64,
        start: Date,
        end: Date,
        side: PayReceive,
    ) -> Self {
        let config = SwapConfig {
            disc_curve: "USD-OIS",
            fwd_curve: "USD-SOFR-3M",
            reset_lag_days: 2,
            sched: IRSScheduleConfig::usd_isda_standard(),
        };

        Self::create_swap_with_config(id, notional, fixed_rate, start, end, side, config)
    }

    /// Create a canonical example IRS for testing and documentation.
    ///
    /// Returns a 5-year pay-fixed swap with semi-annual fixed vs quarterly floating.
    pub fn example() -> Self {
        use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};

        Self::builder()
            .id(InstrumentId::new("IRS-5Y-USD"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(PayReceive::PayFixed)
            .fixed(crate::instruments::common::parameters::FixedLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                rate: 0.03,
                freq: Frequency::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                start: Date::from_calendar_date(2024, time::Month::January, 1)
                    .expect("Valid example date"),
                end: Date::from_calendar_date(2029, time::Month::January, 1)
                    .expect("Valid example date"),
                par_method: None,
                compounding_simple: true,
            })
            .float(crate::instruments::common::parameters::FloatLegSpec {
                discount_curve_id: CurveId::new("USD-OIS"),
                forward_curve_id: CurveId::new("USD-SOFR-3M"),
                spread_bp: 0.0,
                freq: Frequency::quarterly(),
                dc: DayCount::Act360,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: StubKind::None,
                reset_lag_days: 2,
                start: Date::from_calendar_date(2024, time::Month::January, 1)
                    .expect("Valid example date"),
                end: Date::from_calendar_date(2029, time::Month::January, 1)
                    .expect("Valid example date"),
                compounding: Default::default(),
            })
            .build()
            .expect("Example IRS construction should not fail")
    }

    /// Helper to construct a swap with specified curve configuration.
    fn create_swap_with_config(
        id: InstrumentId,
        notional: Money,
        fixed_rate: f64,
        start: Date,
        end: Date,
        side: PayReceive,
        config: SwapConfig<'_>,
    ) -> Self {
        let fixed = FixedLegSpec {
            discount_curve_id: finstack_core::types::CurveId::from(config.disc_curve),
            rate: fixed_rate,
            freq: config.sched.fixed_freq,
            dc: config.sched.fixed_dc,
            bdc: config.sched.bdc,
            calendar_id: config.sched.calendar_id.clone(),
            stub: config.sched.stub,
            start,
            end,
            par_method: None,
            compounding_simple: true,
        };
        let float = FloatLegSpec {
            discount_curve_id: finstack_core::types::CurveId::from(config.disc_curve),
            forward_curve_id: finstack_core::types::CurveId::from(config.fwd_curve),
            spread_bp: 0.0,
            freq: config.sched.float_freq,
            dc: config.sched.float_dc,
            bdc: config.sched.bdc,
            calendar_id: config.sched.calendar_id.clone(),
            stub: config.sched.stub,
            reset_lag_days: config.reset_lag_days,
            start,
            end,
            compounding: Default::default(),
        };
        Self::builder()
            .id(id)
            .notional(notional)
            .side(side)
            .fixed(fixed)
            .float(float)
            .build()
            .expect("Swap construction should not fail")
    }

}

// Explicit trait implementations for modern instrument style
// Attributable implementation is provided by the impl_instrument! macro

impl crate::instruments::common::traits::Instrument for InterestRateSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::IRS
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        crate::instruments::irs::pricer::npv(self, curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
        )
    }
}

impl CashflowProvider for InterestRateSwap {
    fn build_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        crate::instruments::irs::cashflow::signed_dated_flows(self)
    }

    /// Build full cashflow schedule with CFKind metadata for precise classification.
    ///
    /// This creates a proper CashFlowSchedule with CFKind information for each leg,
    /// enabling precise classification of fixed vs floating rate payments.
    fn build_full_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        crate::instruments::irs::cashflow::full_signed_schedule(self)
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for InterestRateSwap {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.fixed.discount_curve_id
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for InterestRateSwap {
    fn forward_curve_ids(&self) -> Vec<finstack_core::types::CurveId> {
        vec![self.float.forward_curve_id.clone()]
    }
}

impl crate::instruments::common::traits::CurveDependencies for InterestRateSwap {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.fixed.discount_curve_id.clone())
            .forward(self.float.forward_curve_id.clone())
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_swap_with_config_uses_usd_isda_standard_schedule() {
        let config = SwapConfig {
            disc_curve: "USD-OIS",
            fwd_curve: "USD-SOFR-3M",
            reset_lag_days: 2,
            sched: IRSScheduleConfig::usd_isda_standard(),
        };

        let start =
            Date::from_calendar_date(2024, time::Month::January, 1).expect("Valid start date");
        let end =
            Date::from_calendar_date(2029, time::Month::January, 1).expect("Valid end date");

        let swap = InterestRateSwap::create_swap_with_config(
            InstrumentId::new("IRS-TEST-USD"),
            Money::new(1_000_000.0, Currency::USD),
            0.03,
            start,
            end,
            PayReceive::PayFixed,
            config,
        );

        let sched = IRSScheduleConfig::usd_isda_standard();

        // Discount and forward curve wiring
        assert_eq!(swap.fixed.discount_curve_id, CurveId::new("USD-OIS"));
        assert_eq!(swap.float.discount_curve_id, CurveId::new("USD-OIS"));
        assert_eq!(swap.float.forward_curve_id, CurveId::new("USD-SOFR-3M"));

        // Schedule conventions match usd_isda_standard configuration
        assert_eq!(swap.fixed.freq, sched.fixed_freq);
        assert_eq!(swap.fixed.dc, sched.fixed_dc);
        assert_eq!(swap.float.freq, sched.float_freq);
        assert_eq!(swap.float.dc, sched.float_dc);
        assert_eq!(swap.fixed.bdc, sched.bdc);
        assert_eq!(swap.float.bdc, sched.bdc);
        assert_eq!(swap.fixed.calendar_id, sched.calendar_id);
        assert_eq!(swap.float.calendar_id, sched.calendar_id);
        assert_eq!(swap.fixed.stub, sched.stub);
        assert_eq!(swap.float.stub, sched.stub);

        // Reset lag and date range are propagated correctly
        assert_eq!(swap.float.reset_lag_days, 2);
        assert_eq!(swap.fixed.start, start);
        assert_eq!(swap.fixed.end, end);
        assert_eq!(swap.float.start, start);
        assert_eq!(swap.float.end, end);
    }
}
