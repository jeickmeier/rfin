//! Discounted Cash Flow instrument types and implementations.
//!
//! Core DCF types used for corporate valuation:
//! - [`TerminalValueSpec`] for Gordon Growth and Exit Multiple terminals
//! - [`DiscountedCashFlow`] instrument implementing the standard DCF formula

use crate::instruments::common::pricing::HasDiscountCurve;
use crate::instruments::common::traits::{
    Attributes, CurveDependencies, CurveIdVec, Instrument, InstrumentCurves,
};
use crate::pricer::InstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Error as CoreError;
use smallvec::smallvec;
use std::any::Any;

/// Terminal value calculation method for DCF.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "snake_case"))]
pub enum TerminalValueSpec {
    /// Gordon Growth Model: TV = FCF_terminal × (1 + g) / (WACC - g)
    GordonGrowth {
        /// Perpetual growth rate (e.g., 0.02 for 2%)
        growth_rate: f64,
    },
    /// Exit Multiple: TV = Terminal_Metric × Multiple
    ExitMultiple {
        /// Terminal metric value (e.g., EBITDA)
        terminal_metric: f64,
        /// Multiple to apply (e.g., 10.0 for 10x EBITDA)
        multiple: f64,
    },
}

/// Discounted Cash Flow instrument for corporate valuation.
///
/// DCF values a company by discounting projected free cash flows and terminal value.
/// The equity value is calculated as: Enterprise Value - Net Debt.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DiscountedCashFlow {
    /// Unique identifier for the DCF.
    pub id: InstrumentId,
    /// Currency for all cashflows.
    pub currency: Currency,
    /// Explicit period free cash flows (date, amount pairs).
    pub flows: Vec<(Date, f64)>,
    /// Weighted Average Cost of Capital (discount rate).
    pub wacc: f64,
    /// Terminal value specification.
    pub terminal_value: TerminalValueSpec,
    /// Net debt (debt - cash) to subtract from enterprise value.
    pub net_debt: f64,
    /// Valuation date (as-of date for the DCF).
    pub valuation_date: Date,
    /// Discount curve identifier for pricing.
    pub discount_curve_id: CurveId,
    /// Attributes for tagging and scenarios.
    pub attributes: Attributes,
}

impl DiscountedCashFlow {
    /// Calculate present value of explicit period cash flows.
    pub fn calculate_pv_explicit_flows(&self) -> f64 {
        self.flows
            .iter()
            .map(|(date, amount)| {
                let years = self.year_fraction(self.valuation_date, *date);
                amount / (1.0 + self.wacc).powf(years)
            })
            .sum()
    }

    /// Calculate terminal value (undiscounted).
    pub fn calculate_terminal_value(&self) -> f64 {
        match &self.terminal_value {
            TerminalValueSpec::GordonGrowth { growth_rate } => {
                // Get the last explicit flow
                if let Some((_, last_fcf)) = self.flows.last() {
                    let g = *growth_rate;
                    if self.wacc > g {
                        last_fcf * (1.0 + g) / (self.wacc - g)
                    } else {
                        // Invalid case: WACC <= growth rate (guarded by npv)
                        0.0
                    }
                } else {
                    0.0
                }
            }
            TerminalValueSpec::ExitMultiple {
                terminal_metric,
                multiple,
            } => terminal_metric * multiple,
        }
    }

    /// Discount terminal value to present value.
    pub fn discount_terminal_value(&self, terminal_value: f64) -> f64 {
        if let Some((terminal_date, _)) = self.flows.last() {
            let years = self.year_fraction(self.valuation_date, *terminal_date);
            terminal_value / (1.0 + self.wacc).powf(years)
        } else {
            0.0
        }
    }

    /// Calculate year fraction between two dates.
    fn year_fraction(&self, start: Date, end: Date) -> f64 {
        // Simple ACT/365.25 day count (sufficient for corporate DCF use cases)
        let days = (end - start).whole_days() as f64;
        days / 365.25
    }

    /// Calculate NPV (equity value) using market context.
    pub fn npv(&self, _market: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
        // Validate terminal value configuration (Gordon Growth requires WACC > g)
        if let TerminalValueSpec::GordonGrowth { growth_rate } = &self.terminal_value {
            if self.wacc <= *growth_rate {
                return Err(CoreError::Validation(format!(
                    "DCF terminal growth rate ({:.6}) must be strictly less than WACC ({:.6})",
                    growth_rate, self.wacc
                )));
            }
        }

        // For now, use the internal WACC-based calculation with WACC as the discount rate.
        let pv_explicit = self.calculate_pv_explicit_flows();
        let terminal_value = self.calculate_terminal_value();
        let pv_terminal = self.discount_terminal_value(terminal_value);
        let enterprise_value = pv_explicit + pv_terminal;
        let equity_value = enterprise_value - self.net_debt;

        Ok(Money::new(equity_value, self.currency))
    }
}

impl Instrument for DiscountedCashFlow {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> InstrumentType {
        InstrumentType::DCF
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }

    fn value(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        self.npv(market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            None,
        )
    }

    fn required_discount_curves(&self) -> CurveIdVec {
        smallvec![self.discount_curve_id.clone()]
    }
}

// Implement HasDiscountCurve so generic DV01 calculators can discover the primary curve.
impl HasDiscountCurve for DiscountedCashFlow {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

// Declare curve dependencies for unified DV01 / bucketed DV01 calculators.
impl CurveDependencies for DiscountedCashFlow {
    fn curve_dependencies(&self) -> InstrumentCurves {
        InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::metrics::{MetricContext, MetricId};
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::{context::MarketContext, term_structures::DiscountCurve};
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn build_simple_dcf_gordon() -> DiscountedCashFlow {
        let valuation_date =
            Date::from_calendar_date(2025, Month::January, 1).expect("valid test valuation date");
        let cf_date =
            Date::from_calendar_date(2026, Month::January, 1).expect("valid test cashflow date");

        let discount_curve_id = CurveId::new("USD-OIS");

        DiscountedCashFlow {
            id: InstrumentId::new("TEST-DCF-GORDON"),
            currency: Currency::USD,
            flows: vec![(cf_date, 100.0)],
            wacc: 0.10,
            terminal_value: TerminalValueSpec::GordonGrowth { growth_rate: 0.02 },
            net_debt: 0.0,
            valuation_date,
            discount_curve_id,
            attributes: Attributes::default(),
        }
    }

    #[test]
    fn gordon_growth_terminal_value_matches_formula() {
        let dcf = build_simple_dcf_gordon();

        let tv = dcf.calculate_terminal_value();

        // TV = FCF_terminal × (1 + g) / (WACC - g)
        let expected_tv = 100.0 * (1.0 + 0.02) / (0.10 - 0.02);
        let diff = (tv - expected_tv).abs();
        assert!(
            diff < 1e-9,
            "terminal value mismatch: got {}, expected {}, diff {}",
            tv,
            expected_tv,
            diff
        );
    }

    #[test]
    fn exit_multiple_terminal_value_matches_product() {
        let valuation_date =
            Date::from_calendar_date(2025, Month::January, 1).expect("valid test valuation date");
        let cf_date =
            Date::from_calendar_date(2026, Month::January, 1).expect("valid test cashflow date");

        let dcf = DiscountedCashFlow {
            id: InstrumentId::new("TEST-DCF-EXIT"),
            currency: Currency::USD,
            flows: vec![(cf_date, 100.0)],
            wacc: 0.10,
            terminal_value: TerminalValueSpec::ExitMultiple {
                terminal_metric: 150.0,
                multiple: 8.0,
            },
            net_debt: 0.0,
            valuation_date,
            discount_curve_id: CurveId::new("USD-OIS"),
            attributes: Attributes::default(),
        };

        let tv = dcf.calculate_terminal_value();
        let expected_tv = 150.0 * 8.0;
        assert!(
            (tv - expected_tv).abs() < 1e-9,
            "exit multiple TV mismatch: got {}, expected {}",
            tv,
            expected_tv
        );
    }

    #[test]
    fn npv_gordon_growth_matches_manual_calculation() {
        let dcf = build_simple_dcf_gordon();
        let market = MarketContext::new();
        let as_of = dcf.valuation_date;

        let money = dcf.npv(&market, as_of).expect("npv should succeed");
        let npv = money.amount();

        // Manual NPV:
        // PV_explicit = 100 / (1.10)^1
        // TV = 100 * (1.02) / (0.10 - 0.02)
        // PV_TV = TV / (1.10)^1
        let pv_explicit = 100.0 / 1.10_f64.powf(1.0);
        let tv = 100.0 * (1.0 + 0.02) / (0.10 - 0.02);
        let pv_tv = tv / 1.10_f64.powf(1.0);
        let expected_npv = pv_explicit + pv_tv;

        let diff = (npv - expected_npv).abs();
        // Allow for minor rounding differences introduced by Money's fixed-point representation.
        assert!(
            diff < 0.1,
            "NPV mismatch: got {}, expected {}, diff {}",
            npv,
            expected_npv,
            diff
        );
    }

    #[test]
    fn npv_errors_when_wacc_not_greater_than_growth() {
        let valuation_date =
            Date::from_calendar_date(2025, Month::January, 1).expect("valid test valuation date");
        let cf_date =
            Date::from_calendar_date(2026, Month::January, 1).expect("valid test cashflow date");

        let dcf = DiscountedCashFlow {
            id: InstrumentId::new("TEST-DCF-BAD-G"),
            currency: Currency::USD,
            flows: vec![(cf_date, 100.0)],
            // WACC <= growth_rate should be rejected
            wacc: 0.02,
            terminal_value: TerminalValueSpec::GordonGrowth { growth_rate: 0.03 },
            net_debt: 0.0,
            valuation_date,
            discount_curve_id: CurveId::new("USD-OIS"),
            attributes: Attributes::default(),
        };

        let market = MarketContext::new();
        let as_of = valuation_date;

        let result = dcf.npv(&market, as_of);
        assert!(
            result.is_err(),
            "expected validation error when WACC <= growth"
        );
    }

    #[test]
    fn required_discount_curves_and_has_discount_curve_are_consistent() {
        let dcf = build_simple_dcf_gordon();

        let required = dcf.required_discount_curves();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], dcf.discount_curve_id);

        let from_trait: &CurveId = HasDiscountCurve::discount_curve_id(&dcf);
        assert_eq!(from_trait, &dcf.discount_curve_id);

        let deps = dcf.curve_dependencies();
        assert_eq!(deps.discount_curves.len(), 1);
        assert_eq!(deps.discount_curves[0], dcf.discount_curve_id);
    }

    fn build_flat_discount_curve(id: &CurveId, as_of: Date, rate: f64) -> DiscountCurve {
        // Simple flat discount curve with a few knots approximating exp(-r t)
        let knots = [(0.0, 1.0), (1.0, (-rate).exp()), (5.0, (-rate * 5.0).exp())];

        DiscountCurve::builder(id.clone())
            .base_date(as_of)
            .knots(knots)
            .build()
            .expect("failed to build flat discount curve")
    }

    fn build_market_with_flat_curve(as_of: Date, curve_id: &CurveId, rate: f64) -> MarketContext {
        let curve = build_flat_discount_curve(curve_id, as_of, rate);
        MarketContext::new().insert_discount(curve)
    }

    fn build_metric_context(
        dcf: DiscountedCashFlow,
        market: MarketContext,
        as_of: Date,
    ) -> MetricContext {
        let base_value = dcf.value(&market, as_of).expect("base value");
        let instrument: std::sync::Arc<dyn Instrument> = std::sync::Arc::new(dcf);
        MetricContext::new(instrument, std::sync::Arc::new(market), as_of, base_value)
    }

    #[test]
    fn dcf_theta_metric_computes() {
        let as_of =
            Date::from_calendar_date(2025, Month::January, 1).expect("valid test date for theta");
        let mut dcf = build_simple_dcf_gordon();
        dcf.valuation_date = as_of;

        let market = build_market_with_flat_curve(as_of, &dcf.discount_curve_id, 0.05);
        let mut mctx = build_metric_context(dcf, market, as_of);

        let mut registry = crate::metrics::standard_registry();
        crate::instruments::dcf::metrics::register_dcf_metrics(&mut registry);

        let results = registry
            .compute(&[MetricId::Theta], &mut mctx)
            .expect("theta metric should compute");

        assert!(
            results.contains_key(&MetricId::Theta),
            "Theta metric should be present for DCF"
        );
    }

    #[test]
    fn dcf_dv01_metric_computes_and_is_reasonable() {
        let as_of =
            Date::from_calendar_date(2025, Month::January, 1).expect("valid test date for dv01");
        let mut dcf = build_simple_dcf_gordon();
        dcf.valuation_date = as_of;

        let market = build_market_with_flat_curve(as_of, &dcf.discount_curve_id, 0.05);
        let mut mctx = build_metric_context(dcf, market, as_of);

        let mut registry = crate::metrics::standard_registry();
        crate::instruments::dcf::metrics::register_dcf_metrics(&mut registry);

        let results = registry
            .compute(&[MetricId::Dv01], &mut mctx)
            .expect("Dv01 metric should compute");

        let dv01 = *results
            .get(&MetricId::Dv01)
            .expect("Dv01 metric should be present");

        // Higher rates should reduce PV, so DV01 (PV_up - PV_base) should be <= 0
        assert!(
            dv01 <= 0.0,
            "DCF DV01 should be non-positive (PV decreases when rates increase), got {}",
            dv01
        );
    }

    #[test]
    fn dcf_bucketed_dv01_metric_computes() {
        let as_of = Date::from_calendar_date(2025, Month::January, 1)
            .expect("valid test date for bucketed dv01");
        let mut dcf = build_simple_dcf_gordon();
        dcf.valuation_date = as_of;

        let market = build_market_with_flat_curve(as_of, &dcf.discount_curve_id, 0.05);
        let mut mctx = build_metric_context(dcf, market, as_of);

        let mut registry = crate::metrics::standard_registry();
        crate::instruments::dcf::metrics::register_dcf_metrics(&mut registry);

        let results = registry
            .compute(&[MetricId::BucketedDv01], &mut mctx)
            .expect("BucketedDv01 metric should compute");

        assert!(
            results.contains_key(&MetricId::BucketedDv01),
            "BucketedDv01 metric should be present for DCF"
        );
    }
}
