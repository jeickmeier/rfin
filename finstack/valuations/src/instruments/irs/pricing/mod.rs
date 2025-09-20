//! IRS pricing facade and engine re-export.
//!
//! Provides the pricing entrypoints for `InterestRateSwap`. Core pricing
//! logic lives in `engine`. The `Priceable` implementation delegates to it
//! and composes metrics via the shared helpers for consistency.

pub mod engine;

use crate::instruments::helpers::build_with_metrics_dyn;
use crate::instruments::irs::types::InterestRateSwap;
use crate::instruments::traits::Priceable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use indexmap::IndexMap;

use engine::IrsEngine;

impl Priceable for InterestRateSwap {
    /// Calculates the present value of the IRS using the core engine.
    fn value(&self, context: &MarketContext, _as_of: Date) -> Result<Money> {
        IrsEngine::pv(self, context)
    }

    /// Calculates the present value with additional metrics.
    fn price_with_metrics(
        &self,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> Result<ValuationResult> {
        let base = <Self as Priceable>::value(self, context, as_of)?;
        // Prefer the shared metrics framework. If it fails (e.g., during refactors),
        // fall back to direct computations for core IRS metrics to keep APIs stable.
        match build_with_metrics_dyn(self, context, as_of, base, metrics) {
            Ok(v) => Ok(v),
            Err(_) => {
                use finstack_core::market_data::term_structures::{
                    discount_curve::DiscountCurve, forward_curve::ForwardCurve,
                };

                let mut measures: IndexMap<String, finstack_core::F> = IndexMap::new();

                // Preload curves once
                let disc = context.get_ref::<DiscountCurve>(self.fixed.disc_id)?;

                // Helper: compute fixed-leg annuity (discounted accrual sum)
                let compute_annuity = || -> finstack_core::F {
                    let sched = crate::cashflow::builder::build_dates(
                        self.fixed.start,
                        self.fixed.end,
                        self.fixed.freq,
                        self.fixed.stub,
                        self.fixed.bdc,
                        self.fixed.calendar_id,
                    );
                    let dates = sched.dates;
                    if dates.len() < 2 {
                        return 0.0;
                    }
                    let mut ann = 0.0;
                    let mut prev = dates[0];
                    for &d in &dates[1..] {
                        let yf = self
                            .fixed
                            .dc
                            .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                            .unwrap_or(0.0);
                        let df = disc.df_on_date_curve(d);
                        ann += yf * df;
                        prev = d;
                    }
                    ann
                };

                // Pre-compute requested metrics
                for m in metrics {
                    match m {
                        MetricId::Annuity => {
                            let ann = compute_annuity();
                            measures.insert(MetricId::Annuity.as_str().to_string(), ann);
                        }
                        MetricId::ParRate => {
                            // Ensure annuity available or compute locally
                            let ann = if let Some(a) = measures.get(MetricId::Annuity.as_str()) {
                                *a
                            } else {
                                let a = compute_annuity();
                                measures.insert(MetricId::Annuity.as_str().to_string(), a);
                                a
                            };
                            let fwd = context.get_ref::<ForwardCurve>(self.float.fwd_id)?;

                            // Build floating schedule and sum discounted projected coupons
                            let fs = crate::cashflow::builder::build_dates(
                                self.float.start,
                                self.float.end,
                                self.float.freq,
                                self.float.stub,
                                self.float.bdc,
                                self.float.calendar_id,
                            );
                            let schedule = fs.dates;
                            let mut pv = 0.0;
                            if schedule.len() >= 2 && ann != 0.0 {
                                let base_date = disc.base_date();
                                let mut prev = schedule[0];
                                for &d in &schedule[1..] {
                                    let t1 = self
                                        .float
                                        .dc
                                        .year_fraction(
                                            base_date,
                                            prev,
                                            finstack_core::dates::DayCountCtx::default(),
                                        )
                                        .unwrap_or(0.0);
                                    let t2 = self
                                        .float
                                        .dc
                                        .year_fraction(
                                            base_date,
                                            d,
                                            finstack_core::dates::DayCountCtx::default(),
                                        )
                                        .unwrap_or(0.0);
                                    let yf = self
                                        .float
                                        .dc
                                        .year_fraction(
                                            prev,
                                            d,
                                            finstack_core::dates::DayCountCtx::default(),
                                        )
                                        .unwrap_or(0.0);
                                    let rate =
                                        fwd.rate_period(t1, t2) + self.float.spread_bp * 1e-4;
                                    let coupon = self.notional.amount() * rate * yf;
                                    let df = disc.df_on_date_curve(d);
                                    pv += coupon * df;
                                    prev = d;
                                }
                                let par = pv / self.notional.amount() / ann;
                                measures.insert(MetricId::ParRate.as_str().to_string(), par);
                            } else {
                                measures.insert(MetricId::ParRate.as_str().to_string(), 0.0);
                            }
                        }
                        MetricId::Dv01 => {
                            // DV01 ≈ annuity * notional * 1bp with side sign
                            let ann = if let Some(a) = measures.get(MetricId::Annuity.as_str()) {
                                *a
                            } else {
                                let a = compute_annuity();
                                measures.insert(MetricId::Annuity.as_str().to_string(), a);
                                a
                            };
                            let mag = ann * self.notional.amount() * 1e-4;
                            let dv01 = match self.side {
                                crate::instruments::irs::PayReceive::ReceiveFixed => mag,
                                crate::instruments::irs::PayReceive::PayFixed => -mag,
                            };
                            measures.insert(MetricId::Dv01.as_str().to_string(), dv01);
                        }
                        MetricId::PvFixed => {
                            let pv_fixed = self.pv_fixed_leg(disc)?;
                            measures
                                .insert(MetricId::PvFixed.as_str().to_string(), pv_fixed.amount());
                        }
                        MetricId::PvFloat => {
                            let fwd = context.get_ref::<ForwardCurve>(self.float.fwd_id)?;
                            let pv_float = self.pv_float_leg(disc, fwd)?;
                            measures
                                .insert(MetricId::PvFloat.as_str().to_string(), pv_float.amount());
                        }
                        _ => {}
                    }
                }

                let mut result = ValuationResult::stamped(
                    <InterestRateSwap as crate::instruments::traits::Instrument>::id(self),
                    as_of,
                    base,
                );
                result.measures = measures;
                Ok(result)
            }
        }
    }
}
