//! Interest rate swap instrument implementation.

pub mod metrics;

use crate::impl_attributable;
use finstack_core::dates::holiday::calendars::calendar_by_id;
use finstack_core::dates::{BusinessDayConvention, Frequency, StubKind};
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::traits::{Discount, Forward};
use finstack_core::prelude::*;
use finstack_core::F;

use crate::cashflow::builder::{
    cf, CouponType, FixedCouponSpec, FloatingCouponSpec as BuilderFloat,
};
use crate::metrics::MetricId;
use crate::instruments::fixed_income::discountable::Discountable;
use crate::results::ValuationResult;
use crate::traits::{
    Attributes, CashflowProvider, DatedFlows, Priceable, RiskBucket, RiskMeasurable, RiskReport,
};

/// Direction of the swap from the perspective of the fixed rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayReceive {
    /// Pay fixed rate, receive floating rate.
    PayFixed,
    /// Receive fixed rate, pay floating rate.
    ReceiveFixed,
}

/// Specification for the fixed leg of an interest rate swap.
#[derive(Clone, Debug)]
pub struct FixedLegSpec {
    /// Discount curve identifier for pricing.
    pub disc_id: &'static str,
    /// Fixed rate (e.g., 0.05 for 5%).
    pub rate: F,
    /// Payment frequency.
    pub freq: Frequency,
    /// Day count convention for accrual.
    pub dc: DayCount,
    /// Business day convention for payment dates.
    pub bdc: BusinessDayConvention,
    /// Optional calendar for business day adjustments.
    pub calendar_id: Option<&'static str>,
    /// Stub period handling rule.
    pub stub: StubKind,
    /// Start date of the fixed leg.
    pub start: Date,
    /// End date of the fixed leg.
    pub end: Date,
}

/// Specification for the floating leg of an interest rate swap.
#[derive(Clone, Debug)]
pub struct FloatLegSpec {
    /// Discount curve identifier for pricing.
    pub disc_id: &'static str,
    /// Forward curve identifier for rate projections.
    pub fwd_id: &'static str,
    /// Spread in basis points added to the forward rate.
    pub spread_bp: F,
    /// Payment frequency.
    pub freq: Frequency,
    /// Day count convention for accrual.
    pub dc: DayCount,
    /// Business day convention for payment dates.
    pub bdc: BusinessDayConvention,
    /// Optional calendar for business day adjustments.
    pub calendar_id: Option<&'static str>,
    /// Stub period handling rule.
    pub stub: StubKind,
    /// Start date of the floating leg.
    pub start: Date,
    /// End date of the floating leg.
    pub end: Date,
}

/// Interest rate swap with fixed and floating legs.
///
/// Represents a standard interest rate swap where one party pays
/// a fixed rate and the other pays a floating rate plus spread.
#[derive(Clone, Debug)]
pub struct InterestRateSwap {
    /// Unique identifier for the swap.
    pub id: String,
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

impl InterestRateSwap {
    /// Create a new IRS builder.
    pub fn builder() -> IRSBuilder {
        IRSBuilder::new()
    }

    /// Compute PV of fixed leg (helper for value calculation).
    fn pv_fixed_leg(&self, disc: &dyn Discount) -> finstack_core::Result<Money> {
        let base = disc.base_date();
        let mut b = cf();
        b.principal(self.notional, self.fixed.start, self.fixed.end)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: self.fixed.rate,
                freq: self.fixed.freq,
                dc: self.fixed.dc,
                bdc: self.fixed.bdc,
                calendar_id: self.fixed.calendar_id,
                stub: self.fixed.stub,
            });
        let sched = b.build()?;

        // Discount coupon flows only
        let flows: Vec<(Date, Money)> = sched
            .flows
            .iter()
            .filter(|cf| {
                cf.kind == crate::cashflow::primitives::CFKind::Fixed
                    || cf.kind == crate::cashflow::primitives::CFKind::Stub
            })
            .map(|cf| (cf.date, cf.amount))
            .collect();
        flows.npv(disc, base, sched.day_count)
    }

    /// Compute PV of floating leg (helper for value calculation).
    fn pv_float_leg(&self, disc: &dyn Discount, fwd: &dyn Forward) -> finstack_core::Result<Money> {
        use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

        let base = disc.base_date();
        let builder = finstack_core::dates::ScheduleBuilder::new(self.float.start, self.float.end)
            .frequency(self.float.freq)
            .stub_rule(self.float.stub);

        let sched_dates: Vec<Date> = if let Some(id) = self.float.calendar_id {
            if let Some(cal) = calendar_by_id(id) {
                builder.adjust_with(self.float.bdc, cal).build().collect()
            } else {
                builder.build_raw().collect()
            }
        } else {
            builder.build_raw().collect()
        };

        if sched_dates.len() < 2 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        let mut prev = sched_dates[0];
        let mut flows: Vec<(Date, Money)> = Vec::with_capacity(sched_dates.len().saturating_sub(1));
        for &d in &sched_dates[1..] {
            let t1 = DiscountCurve::year_fraction(base, prev, self.float.dc);
            let t2 = DiscountCurve::year_fraction(base, d, self.float.dc);
            let yf = DiscountCurve::year_fraction(prev, d, self.float.dc);
            let f = fwd.rate_period(t1, t2);
            let rate = f + (self.float.spread_bp * 1e-4);
            let coupon = self.notional * (rate * yf);
            flows.push((d, coupon));
            prev = d;
        }
        flows.npv(disc, base, self.float.dc)
    }
}

impl Priceable for InterestRateSwap {
    /// Compute only the base present value (fast, no metrics).
    fn value(&self, curves: &CurveSet, _as_of: Date) -> finstack_core::Result<Money> {
        let disc = curves.discount(self.fixed.disc_id)?;
        let fwd = curves.forecast(self.float.fwd_id)?;

        let pv_fixed = self.pv_fixed_leg(&*disc)?;
        let pv_float = self.pv_float_leg(&*disc, &*fwd)?;

        match self.side {
            PayReceive::PayFixed => pv_float - pv_fixed,
            PayReceive::ReceiveFixed => pv_fixed - pv_float,
        }
    }

    /// Compute value with specific metrics using the metrics framework.
    fn price_with_metrics(
        &self,
        curves: &CurveSet,
        as_of: Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<ValuationResult> {

        // Compute base value
        let base_value = self.value(curves, as_of)?;

        crate::instruments::build_with_metrics(
            crate::instruments::Instrument::IRS(self.clone()),
            curves,
            as_of,
            base_value,
            metrics,
        )
    }

    /// Compute full valuation with all standard IRS metrics (backward compatible).
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        // Standard IRS metrics
        let standard_metrics = [
            MetricId::Annuity,
            MetricId::ParRate,
            MetricId::Dv01,
            MetricId::PvFixed,
            MetricId::PvFloat,
        ];
        self.price_with_metrics(curves, as_of, &standard_metrics)
    }
}

// Generate standard Attributable implementation using macro
impl_attributable!(InterestRateSwap);

impl From<InterestRateSwap> for crate::instruments::Instrument {
    fn from(value: InterestRateSwap) -> Self {
        crate::instruments::Instrument::IRS(value)
    }
}

impl std::convert::TryFrom<crate::instruments::Instrument> for InterestRateSwap {
    type Error = finstack_core::Error;

    fn try_from(value: crate::instruments::Instrument) -> finstack_core::Result<Self> {
        match value {
            crate::instruments::Instrument::IRS(v) => Ok(v),
            _ => Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }
}

/// Builder pattern for IRS instruments
#[derive(Default)]
pub struct IRSBuilder {
    id: Option<String>,
    notional: Option<Money>,
    side: Option<PayReceive>,
    // Fixed leg fields
    fixed_disc_id: Option<&'static str>,
    fixed_rate: Option<F>,
    fixed_freq: Option<Frequency>,
    fixed_dc: Option<DayCount>,
    fixed_bdc: Option<BusinessDayConvention>,
    fixed_calendar_id: Option<&'static str>,
    fixed_stub: Option<StubKind>,
    fixed_start: Option<Date>,
    fixed_end: Option<Date>,
    // Float leg fields
    float_disc_id: Option<&'static str>,
    float_fwd_id: Option<&'static str>,
    float_spread_bp: Option<F>,
    float_freq: Option<Frequency>,
    float_dc: Option<DayCount>,
    float_bdc: Option<BusinessDayConvention>,
    float_calendar_id: Option<&'static str>,
    float_stub: Option<StubKind>,
    float_start: Option<Date>,
    float_end: Option<Date>,
}

impl IRSBuilder {
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

    pub fn side(mut self, value: PayReceive) -> Self {
        self.side = Some(value);
        self
    }

    // Fixed leg setters
    pub fn fixed_disc_id(mut self, value: &'static str) -> Self {
        self.fixed_disc_id = Some(value);
        self
    }

    pub fn fixed_rate(mut self, value: F) -> Self {
        self.fixed_rate = Some(value);
        self
    }

    pub fn fixed_freq(mut self, value: Frequency) -> Self {
        self.fixed_freq = Some(value);
        self
    }

    pub fn fixed_dc(mut self, value: DayCount) -> Self {
        self.fixed_dc = Some(value);
        self
    }

    pub fn fixed_bdc(mut self, value: BusinessDayConvention) -> Self {
        self.fixed_bdc = Some(value);
        self
    }

    pub fn fixed_calendar_id(mut self, value: &'static str) -> Self {
        self.fixed_calendar_id = Some(value);
        self
    }

    pub fn fixed_stub(mut self, value: StubKind) -> Self {
        self.fixed_stub = Some(value);
        self
    }

    pub fn fixed_start(mut self, value: Date) -> Self {
        self.fixed_start = Some(value);
        self
    }

    pub fn fixed_end(mut self, value: Date) -> Self {
        self.fixed_end = Some(value);
        self
    }

    // Float leg setters
    pub fn float_disc_id(mut self, value: &'static str) -> Self {
        self.float_disc_id = Some(value);
        self
    }

    pub fn float_fwd_id(mut self, value: &'static str) -> Self {
        self.float_fwd_id = Some(value);
        self
    }

    pub fn float_spread_bp(mut self, value: F) -> Self {
        self.float_spread_bp = Some(value);
        self
    }

    pub fn float_freq(mut self, value: Frequency) -> Self {
        self.float_freq = Some(value);
        self
    }

    pub fn float_dc(mut self, value: DayCount) -> Self {
        self.float_dc = Some(value);
        self
    }

    pub fn float_bdc(mut self, value: BusinessDayConvention) -> Self {
        self.float_bdc = Some(value);
        self
    }

    pub fn float_calendar_id(mut self, value: &'static str) -> Self {
        self.float_calendar_id = Some(value);
        self
    }

    pub fn float_stub(mut self, value: StubKind) -> Self {
        self.float_stub = Some(value);
        self
    }

    pub fn float_start(mut self, value: Date) -> Self {
        self.float_start = Some(value);
        self
    }

    pub fn float_end(mut self, value: Date) -> Self {
        self.float_end = Some(value);
        self
    }

    /// Convenience method to set both legs to the same start/end dates
    pub fn dates(mut self, start: Date, end: Date) -> Self {
        self.fixed_start = Some(start);
        self.fixed_end = Some(end);
        self.float_start = Some(start);
        self.float_end = Some(end);
        self
    }

    /// Convenience method to set standard fixed leg defaults
    pub fn standard_fixed_leg(
        mut self,
        disc_id: &'static str,
        rate: F,
        freq: Frequency,
        dc: DayCount,
    ) -> Self {
        self.fixed_disc_id = Some(disc_id);
        self.fixed_rate = Some(rate);
        self.fixed_freq = Some(freq);
        self.fixed_dc = Some(dc);
        self.fixed_bdc = Some(BusinessDayConvention::ModifiedFollowing);
        self.fixed_stub = Some(StubKind::None);
        self
    }

    /// Convenience method to set standard float leg defaults
    pub fn standard_float_leg(
        mut self,
        disc_id: &'static str,
        fwd_id: &'static str,
        spread_bp: F,
        freq: Frequency,
        dc: DayCount,
    ) -> Self {
        self.float_disc_id = Some(disc_id);
        self.float_fwd_id = Some(fwd_id);
        self.float_spread_bp = Some(spread_bp);
        self.float_freq = Some(freq);
        self.float_dc = Some(dc);
        self.float_bdc = Some(BusinessDayConvention::ModifiedFollowing);
        self.float_stub = Some(StubKind::None);
        self
    }

    pub fn build(self) -> finstack_core::Result<InterestRateSwap> {
        let id = self
            .id
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let notional = self
            .notional
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let side = self
            .side
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

        // Build fixed leg spec
        let fixed = FixedLegSpec {
            disc_id: self.fixed_disc_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            rate: self.fixed_rate.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            freq: self.fixed_freq.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            dc: self.fixed_dc.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            bdc: self
                .fixed_bdc
                .unwrap_or(BusinessDayConvention::ModifiedFollowing),
            calendar_id: self.fixed_calendar_id,
            stub: self.fixed_stub.unwrap_or(StubKind::None),
            start: self.fixed_start.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            end: self.fixed_end.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
        };

        // Build float leg spec
        let float = FloatLegSpec {
            disc_id: self.float_disc_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            fwd_id: self.float_fwd_id.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            spread_bp: self.float_spread_bp.unwrap_or(0.0),
            freq: self.float_freq.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            dc: self.float_dc.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            bdc: self
                .float_bdc
                .unwrap_or(BusinessDayConvention::ModifiedFollowing),
            calendar_id: self.float_calendar_id,
            stub: self.float_stub.unwrap_or(StubKind::None),
            start: self.float_start.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
            end: self.float_end.ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::Invalid)
            })?,
        };

        Ok(InterestRateSwap {
            id,
            notional,
            side,
            fixed,
            float,
            attributes: Attributes::new(),
        })
    }
}

impl RiskMeasurable for InterestRateSwap {
    fn risk_report(
        &self,
        curves: &CurveSet,
        as_of: Date,
        _bucket_spec: Option<&[RiskBucket]>,
    ) -> finstack_core::Result<RiskReport> {
        use crate::instruments::Instrument;
        use crate::metrics::MetricContext;
        use crate::metrics::{standard_registry, MetricId};
        use std::sync::Arc;

        // Create risk report
        let mut report = RiskReport::new(&self.id, self.notional.currency());

        // Compute base value
        let base_value = self.value(curves, as_of)?;

        // Create metric context
        let mut context = MetricContext::new(
            Arc::new(Instrument::IRS(self.clone())),
            Arc::new(curves.clone()),
            as_of,
            base_value,
        );

        // Compute key risk metrics for swaps
        let registry = standard_registry();
        let risk_metrics = [MetricId::Dv01, MetricId::Annuity, MetricId::ParRate];

        // Compute available metrics
        for metric_id in &risk_metrics {
            if let Ok(metrics) = registry.compute(&[metric_id.clone()], &mut context) {
                if let Some(value) = metrics.get(metric_id) {
                    report = report.with_metric(metric_id.as_str(), *value);
                }
            }
        }

        // Add maturity bucket
        let years_to_maturity = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(
            as_of, self.fixed.end, self.fixed.dc
        );

        let bucket = if years_to_maturity <= 2.0 {
            RiskBucket {
                id: "2Y".to_string(),
                tenor_years: Some(years_to_maturity),
                classification: Some("Short".to_string()),
            }
        } else if years_to_maturity <= 5.0 {
            RiskBucket {
                id: "5Y".to_string(),
                tenor_years: Some(years_to_maturity),
                classification: Some("Medium".to_string()),
            }
        } else if years_to_maturity <= 10.0 {
            RiskBucket {
                id: "10Y".to_string(),
                tenor_years: Some(years_to_maturity),
                classification: Some("Long".to_string()),
            }
        } else {
            RiskBucket {
                id: "30Y".to_string(),
                tenor_years: Some(years_to_maturity),
                classification: Some("Ultra-Long".to_string()),
            }
        };

        report = report.with_bucket(bucket);

        // Add side information
        report
            .meta
            .insert("side".to_string(), format!("{:?}", self.side));
        report
            .meta
            .insert("fixed_rate".to_string(), format!("{:.4}", self.fixed.rate));
        report.meta.insert(
            "float_spread_bp".to_string(),
            format!("{:.1}", self.float.spread_bp),
        );

        Ok(report)
    }

    fn default_risk_buckets(&self) -> Option<Vec<RiskBucket>> {
        Some(vec![
            RiskBucket {
                id: "2Y".to_string(),
                tenor_years: Some(2.0),
                classification: Some("Short".to_string()),
            },
            RiskBucket {
                id: "5Y".to_string(),
                tenor_years: Some(5.0),
                classification: Some("Medium".to_string()),
            },
            RiskBucket {
                id: "10Y".to_string(),
                tenor_years: Some(10.0),
                classification: Some("Long".to_string()),
            },
            RiskBucket {
                id: "30Y".to_string(),
                tenor_years: Some(30.0),
                classification: Some("Ultra-Long".to_string()),
            },
        ])
    }
}

impl CashflowProvider for InterestRateSwap {
    fn build_schedule(
        &self,
        _curves: &CurveSet,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Use builder to generate both legs; then map signs by side
        let mut fixed_b = cf();
        fixed_b
            .principal(self.notional, self.fixed.start, self.fixed.end)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: self.fixed.rate,
                freq: self.fixed.freq,
                dc: self.fixed.dc,
                bdc: self.fixed.bdc,
                calendar_id: self.fixed.calendar_id,
                stub: self.fixed.stub,
            });
        let fixed_sched = fixed_b.build()?;

        let mut float_b = cf();
        float_b
            .principal(self.notional, self.float.start, self.float.end)
            .floating_cf(BuilderFloat {
                index_id: self.float.fwd_id,
                margin_bp: self.float.spread_bp,
                gearing: 1.0,
                coupon_type: CouponType::Cash,
                freq: self.float.freq,
                dc: self.float.dc,
                bdc: self.float.bdc,
                calendar_id: self.float.calendar_id,
                stub: self.float.stub,
                reset_lag_days: 2,
            });
        let float_sched = float_b.build()?;

        let mut flows: Vec<(Date, Money)> = Vec::new();
        for cf in fixed_sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::Fixed
                || cf.kind == crate::cashflow::primitives::CFKind::Stub
            {
                let amt = match self.side {
                    PayReceive::ReceiveFixed => cf.amount,
                    PayReceive::PayFixed => cf.amount * -1.0,
                };
                flows.push((cf.date, amt));
            }
        }
        for cf in float_sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::FloatReset {
                let amt = match self.side {
                    PayReceive::ReceiveFixed => cf.amount * -1.0,
                    PayReceive::PayFixed => cf.amount,
                };
                flows.push((cf.date, amt));
            }
        }
        Ok(flows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_irs_builder_pattern() {
        let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let end = Date::from_calendar_date(2030, Month::January, 15).unwrap();

        let irs = InterestRateSwap::builder()
            .id("IRS001")
            .notional(Money::new(10_000_000.0, Currency::USD))
            .side(PayReceive::PayFixed)
            .dates(start, end)
            .standard_fixed_leg("USD-OIS", 0.03, Frequency::semi_annual(), DayCount::Act365F)
            .standard_float_leg(
                "USD-OIS",
                "USD-SOFR-3M",
                0.0,
                Frequency::quarterly(),
                DayCount::Act360,
            )
            .build()
            .unwrap();

        assert_eq!(irs.id, "IRS001");
        assert_eq!(irs.notional.amount(), 10_000_000.0);
        assert_eq!(irs.side, PayReceive::PayFixed);
        assert_eq!(irs.fixed.rate, 0.03);
        assert_eq!(irs.float.fwd_id, "USD-SOFR-3M");
        assert_eq!(irs.float.spread_bp, 0.0);
    }
}
