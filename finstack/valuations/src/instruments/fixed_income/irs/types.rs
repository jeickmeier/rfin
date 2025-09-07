//! Interest Rate Swap (IRS) types and implementations.

use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::{BusinessDayConvention, Frequency, StubKind};
use finstack_core::market_data::traits::{Discount, Forward};
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use finstack_core::F;

use crate::cashflow::builder::{cf, CouponType, FixedCouponSpec, FloatingCouponSpec as BuilderFloat};
use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::fixed_income::discountable::Discountable;
use crate::instruments::traits::Attributes;
use crate::instruments::traits::Priceable;
use crate::metrics::{RiskBucket, RiskMeasurable, RiskReport};

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
    pub fn builder() -> crate::instruments::fixed_income::irs::mod_irs::IRSBuilder {
        crate::instruments::fixed_income::irs::mod_irs::IRSBuilder::new()
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
        let base = disc.base_date();
        let builder = finstack_core::dates::ScheduleBuilder::new(self.float.start, self.float.end)
            .frequency(self.float.freq)
            .stub_rule(self.float.stub);

        let sched_dates: Vec<Date> = if let Some(id) = self.float.calendar_id {
            if let Some(cal) = calendar_by_id(id) {
                builder
                    .adjust_with(self.float.bdc, cal)
                    .build()
                    .unwrap()
                    .into_iter()
                    .collect()
            } else {
                builder.build().unwrap().into_iter().collect()
            }
        } else {
            builder.build().unwrap().into_iter().collect()
        };

        if sched_dates.len() < 2 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        let mut prev = sched_dates[0];
        let mut flows: Vec<(Date, Money)> = Vec::with_capacity(sched_dates.len().saturating_sub(1));
        for &d in &sched_dates[1..] {
            let t1 = self
                .float
                .dc
                .year_fraction(base, prev, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let t2 = self
                .float
                .dc
                .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let yf = self
                .float
                .dc
                .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let f = fwd.rate_period(t1, t2);
            let rate = f + (self.float.spread_bp * 1e-4);
            let coupon = self.notional * (rate * yf);
            flows.push((d, coupon));
            prev = d;
        }
        flows.npv(disc, base, self.float.dc)
    }
}

impl_instrument!(
    InterestRateSwap,
    "InterestRateSwap",
    pv = |s, curves, _as_of| {
        let disc = curves.disc(s.fixed.disc_id)?;
        let fwd = curves.fwd(s.float.fwd_id)?;
        let pv_fixed = s.pv_fixed_leg(&*disc)?;
        let pv_float = s.pv_float_leg(&*disc, &*fwd)?;
        match s.side {
            PayReceive::PayFixed => pv_float - pv_fixed,
            PayReceive::ReceiveFixed => pv_fixed - pv_float,
        }
    }
);

impl RiskMeasurable for InterestRateSwap {
    fn risk_report(
        &self,
        curves: &MarketContext,
        as_of: Date,
        _bucket_spec: Option<&[RiskBucket]>,
    ) -> finstack_core::Result<RiskReport> {
        use crate::metrics::MetricContext;
        use crate::metrics::{standard_registry, MetricId};
        use std::sync::Arc;

        // Create risk report
        let mut report = RiskReport::new(&self.id, self.notional.currency());

        // Compute base value
        let base_value = self.value(curves, as_of)?;

        // Create metric context
        let mut context = MetricContext::new(
            Arc::new(self.clone()),
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
        let years_to_maturity = self
            .fixed
            .dc
            .year_fraction(
                as_of,
                self.fixed.end,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

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
        _curves: &MarketContext,
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


