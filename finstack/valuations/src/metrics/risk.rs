//! Risk-specific metric calculators.
//!
//! Provides specialized calculators for risk metrics including bucketed DV01
//! and time decay (theta). These metrics help quantify interest rate risk
//! and time value of financial instruments.

use super::ids::MetricId;
use super::traits::{MetricCalculator, MetricContext};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::traits::{Discount, Forward, TermStructure};
use finstack_core::prelude::*;
use finstack_core::types::CurveId;
use finstack_core::F;
use hashbrown::HashMap;
use std::sync::Arc;

/// Wrapper for a discount curve aged by a time shift (typically 1 day).
///
/// This shifts the effective time axis: df_aged(u) = df_original(u + dt) / df_original(dt)
/// where dt is the time shift in year fractions.
struct AgedDiscountCurve {
    original: Arc<dyn Discount + Send + Sync>,
    shift_date: Date,
    dt: F,
}

impl AgedDiscountCurve {
    fn new(
        original: Arc<dyn Discount + Send + Sync>,
        shift_date: Date,
        day_count: DayCount,
    ) -> finstack_core::Result<Self> {
        let base_date = original.base_date();
        let dt = day_count.year_fraction(
            base_date,
            shift_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        Ok(Self {
            original,
            shift_date,
            dt,
        })
    }
}

impl TermStructure for AgedDiscountCurve {
    fn id(&self) -> &CurveId {
        self.original.id()
    }
}

impl Discount for AgedDiscountCurve {
    #[inline]
    fn base_date(&self) -> Date {
        self.shift_date
    }

    #[inline]
    fn df(&self, t: F) -> F {
        let original_df_shifted = self.original.df(t + self.dt);
        let original_df_dt = self.original.df(self.dt);
        if original_df_dt > 0.0 {
            original_df_shifted / original_df_dt
        } else {
            original_df_shifted
        }
    }
}

/// Wrapper for a forward curve aged by a time shift.
struct AgedForwardCurve {
    original: Arc<dyn Forward + Send + Sync>,
    dt: F,
}

impl AgedForwardCurve {
    fn new(original: Arc<dyn Forward + Send + Sync>, dt: F) -> Self {
        Self { original, dt }
    }
}

impl TermStructure for AgedForwardCurve {
    fn id(&self) -> &CurveId {
        self.original.id()
    }
}

impl Forward for AgedForwardCurve {
    #[inline]
    fn rate(&self, t: F) -> F {
        // For forward curves, we shift the time coordinate
        self.original.rate(t + self.dt)
    }
}

/// Specification for DV01 tenor buckets.
///
/// Defines the tenor points used for bucketed DV01 calculations.
/// Standard buckets cover 3M to 30Y with configurable points for
/// detailed risk analysis and hedging decisions.
///
/// # Default Buckets
///
/// The default specification includes standard tenor points:
/// - 3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y
///
/// See unit tests and `examples/` for usage.
#[derive(Clone, Debug)]
pub struct BucketSpec {
    /// Tenor points in years from curve base date.
    pub tenors: Vec<F>,
}

impl Default for BucketSpec {
    fn default() -> Self {
        // Standard bucket points: 3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y
        Self {
            tenors: vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
        }
    }
}

/// Calculates bucketed DV01 (sensitivity per tenor bucket).
///
/// Breaks down interest rate sensitivity by maturity buckets for
/// better risk management and hedging decisions. Each bucket represents
/// the sensitivity to a parallel shift in that specific tenor region.
///
/// # How It Works
///
/// 1. **Flow Assignment**: Each cashflow is assigned to the nearest tenor bucket
/// 2. **Sensitivity Calculation**: DV01 is computed per bucket using small rate shifts
/// 3. **Risk Aggregation**: Total risk is the sum of all bucket sensitivities
///
/// See unit tests and `examples/` for usage.
#[derive(Default)]
pub struct BucketedDv01Calculator {
    /// Bucket specification to use.
    pub buckets: BucketSpec,
}

impl BucketedDv01Calculator {
    /// Creates a calculator with custom bucket specification.
    ///
    /// # Arguments
    /// * `buckets` - Custom bucket specification for the analysis
    ///
    /// See unit tests and `examples/` for usage.
    pub fn with_buckets(buckets: BucketSpec) -> Self {
        Self { buckets }
    }

    /// Formats a bucket label for display.
    ///
    /// Converts tenor years to human-readable labels:
    /// - < 1 year: "XM" (e.g., "6M" for 0.5 years)
    /// - ≥ 1 year: "XY" (e.g., "5Y" for 5.0 years)
    ///
    /// # Arguments
    /// * `tenor_years` - Tenor in years
    ///
    /// # Returns
    /// Formatted string label for the bucket
    fn bucket_label(&self, tenor_years: F) -> String {
        if tenor_years < 1.0 {
            format!("{}M", (tenor_years * 12.0).round() as i32)
        } else {
            format!("{:.0}Y", tenor_years)
        }
    }

    /// Computes bucketed DV01 for given cashflows.
    ///
    /// Assigns each cashflow to the nearest tenor bucket and calculates
    /// the sensitivity within each bucket. This provides detailed risk
    /// breakdown for hedging and risk management.
    ///
    /// # Arguments
    /// * `flows` - Vector of (date, money) tuples representing cashflows
    /// * `disc` - Discount curve for present value calculations
    /// * `dc` - Day count convention for time calculations
    /// * `base` - Base date for year fraction calculations
    ///
    /// # Returns
    /// HashMap mapping bucket labels to DV01 values
    fn compute_bucketed(
        &self,
        flows: &[(Date, Money)],
        disc: &dyn Discount,
        dc: DayCount,
        base: Date,
    ) -> HashMap<String, F> {
        let mut result = HashMap::new();

        // Early return if no flows
        if flows.is_empty() {
            result.insert("bucketed_dv01_total".to_string(), 0.0);
            return result;
        }

        // Precompute each flow's time and assign to nearest bucket
        let mut idx_to_label: HashMap<usize, String> = HashMap::new();
        let mut bucket_flows: HashMap<usize, Vec<(Date, Money)>> = HashMap::new();
        let mut flow_data: Vec<(Date, Money, F, usize)> = Vec::with_capacity(flows.len());

        for &(date, amount) in flows {
            let t = dc
                .year_fraction(base, date, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0)
                .max(0.0);

            // Find nearest bucket
            let (idx, _) = self
                .buckets
                .tenors
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| (*a - t).abs().partial_cmp(&(*b - t).abs()).unwrap())
                .unwrap_or((0, &self.buckets.tenors[0]));

            idx_to_label
                .entry(idx)
                .or_insert_with(|| self.bucket_label(self.buckets.tenors[idx]));
            bucket_flows.entry(idx).or_default().push((date, amount));
            flow_data.push((date, amount, t, idx));
        }

        // Compute baseline PV and cache discount factors
        let mut base_pv = 0.0;
        let mut df_cache: Vec<F> = Vec::with_capacity(flow_data.len());

        for (date, amount, _, _) in &flow_data {
            let df = DiscountCurve::df_on(disc, base, *date, dc);
            base_pv += amount.amount() * df;
            df_cache.push(df);
        }

        // Compute per-bucket DV01 by bumping each bucket
        let bp = 1e-4; // 1 basis point
        let mut total_dv01 = 0.0;

        for (bucket_idx, _) in bucket_flows.iter() {
            let mut bumped_pv = 0.0;

            for ((_, amount, t, idx), df) in flow_data.iter().zip(df_cache.iter()) {
                // Apply bump only to flows in this bucket
                let df_bumped = if idx == bucket_idx {
                    *df * (-bp * *t).exp()
                } else {
                    *df
                };
                bumped_pv += amount.amount() * df_bumped;
            }

            let dv01 = (base_pv - bumped_pv) / bp;
            let label = idx_to_label
                .get(bucket_idx)
                .cloned()
                .unwrap_or_else(|| self.bucket_label(self.buckets.tenors[*bucket_idx]));

            result.insert(format!("bucketed_dv01_{}", label.to_lowercase()), dv01);
            total_dv01 += dv01;
        }

        result.insert("bucketed_dv01_total".to_string(), total_dv01);
        result
    }
}

impl MetricCalculator for BucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        // Get or compute cashflows
        let flows = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "vol_surface".to_string(),
            })
        })?;

        // Get discount curve - try to infer from instrument or use default
        let disc_id = context.discount_curve_id.unwrap_or("USD-OIS");

        let disc = context.curves.disc(disc_id)?;

        // Get day count - try to infer or use default
        let dc = context.day_count.unwrap_or(DayCount::Act365F);

        let base = disc.base_date();

        // Compute all bucketed DV01s
        let bucketed = self.compute_bucketed(flows, &*disc, dc, base);

        // Store individual bucket results in context when a resolver is provided.
        // This enables dynamic bucket keys keyed by MetricId without changing
        // default behavior unless explicitly opted in.
        if let Some(resolver) = &context.bucket_key_resolver {
            for (key, value) in bucketed.iter() {
                if key == "bucketed_dv01_total" {
                    continue;
                }

                // Expect keys like "bucketed_dv01_5y" → extract label "5y"
                let label = key.strip_prefix("bucketed_dv01_").unwrap_or(key.as_str());

                let metric_id = resolver(&MetricId::BucketedDv01, label, &*context.instrument);
                if let Some(existing) = context.computed.get_mut(&metric_id) {
                    *existing += *value;
                } else {
                    context.computed.insert(metric_id, *value);
                }
            }
        }

        // Return total as primary result
        Ok(bucketed.get("bucketed_dv01_total").copied().unwrap_or(0.0))
    }

    fn dependencies(&self) -> &[MetricId] {
        // No hard dependencies, but works better if cashflows are cached
        &[]
    }
}

/// Calculates theta (time decay) for options and time-sensitive instruments.
///
/// Theta measures the rate of change in an option's value with respect to time.
/// This implementation uses a 1-day time shift approach: it ages all rate curves
/// by 1 calendar day and reprices the instrument to measure time decay.
///
/// The calculation is: Theta = PV(t+1day) - PV(t)
///
/// This approach works generically for any instrument and captures the combined
/// effect of time decay across discount rates, forward rates, and credit spreads.
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let base_price = context.base_value.amount();
        let as_of = context.as_of;

        // Shift valuation date by 1 calendar day
        let shifted_date = as_of + time::Duration::days(1);
        let day_count = context.day_count.unwrap_or(DayCount::Act365F);

        // Create aged market context - for simplicity, we'll try to age the specific
        // curves that the instrument uses. For a fully generic approach, we would
        // need to discover all curve IDs from the market context, but for now we'll
        // handle the most common cases and let instruments that need more complex
        // aging override this calculator.
        let original_curves = &context.curves;
        let mut aged_context = finstack_core::market_data::MarketContext::new();

        // Try to age the discount curve used by the instrument
        if let Some(disc_id) = context.discount_curve_id {
            if let Ok(original_disc) = original_curves.disc(disc_id) {
                let aged_disc = AgedDiscountCurve::new(original_disc, shifted_date, day_count)?;
                aged_context = aged_context.insert_discount(aged_disc);
            }
        }

        // For options and swaps, try common curve IDs
        let common_disc_ids = ["USD-OIS", "EUR-OIS", "GBP-OIS", "JPY-OIS"];
        let common_fwd_ids = ["USD-LIBOR-3M", "USD-SOFR", "EUR-EURIBOR-3M", "GBP-SONIA"];

        for &curve_id in &common_disc_ids {
            if let Ok(original_disc) = original_curves.disc(curve_id) {
                let aged_disc = AgedDiscountCurve::new(original_disc, shifted_date, day_count)?;
                aged_context = aged_context.insert_discount(aged_disc);
            }
        }

        for &curve_id in &common_fwd_ids {
            if let Ok(original_fwd) = original_curves.fwd(curve_id) {
                let dt = day_count.year_fraction(
                    as_of,
                    shifted_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let aged_fwd = AgedForwardCurve::new(original_fwd, dt);
                aged_context = aged_context.insert_forward(aged_fwd);
            }
        }

        // Copy over vol surfaces and FX data (assumed constant over 1 day)
        aged_context.surfaces = original_curves.surfaces.clone();
        aged_context.prices = original_curves.prices.clone();
        aged_context.series = original_curves.series.clone();
        // FX matrix is assumed constant over 1 day, so keep same Arc reference
        aged_context.fx = original_curves.fx.clone();

        // Reprice instrument with aged market context
        let aged_price = context
            .instrument
            .value(&aged_context, shifted_date)?
            .amount();

        // Theta per calendar day
        Ok(aged_price - base_price)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

// tests moved to end of file to satisfy clippy::items-after-test-module

/// Helper trait for instruments to cache their cashflows for risk calculations.
///
/// This trait provides convenience methods for instruments to cache
/// commonly needed data in the metric context, improving performance
/// for risk calculations that need cashflows, discount curves, and
/// day count conventions.
///
/// See unit tests and `examples/` for usage.
pub trait CashflowCaching {
    /// Caches cashflows in the metric context for risk calculations.
    ///
    /// This method stores the instrument's cashflow schedule in the context,
    /// allowing risk calculators to access it without recomputation.
    ///
    /// # Arguments
    /// * `context` - Metric context to cache cashflows in
    /// * `flows` - Vector of (date, money) tuples representing cashflows
    fn cache_cashflows(&self, context: &mut MetricContext, flows: Vec<(Date, Money)>) {
        context.cashflows = Some(flows);
    }

    /// Caches the discount curve ID to use.
    ///
    /// This method stores the identifier for the discount curve that should
    /// be used for risk calculations involving this instrument.
    ///
    /// # Arguments
    /// * `context` - Metric context to cache the curve ID in
    /// * `curve_id` - Static string identifier for the discount curve
    fn cache_discount_curve(&self, context: &mut MetricContext, curve_id: &'static str) {
        context.discount_curve_id = Some(curve_id);
    }

    /// Caches the day count convention.
    ///
    /// This method stores the day count convention that should be used
    /// for time calculations in risk metrics.
    ///
    /// # Arguments
    /// * `context` - Metric context to cache the day count in
    /// * `dc` - Day count convention to use
    fn cache_day_count(&self, context: &mut MetricContext, dc: DayCount) {
        context.day_count = Some(dc);
    }
}

/// Registers all risk metrics to a registry.
///
/// This function adds the standard risk metrics (bucketed DV01 and theta)
/// to the provided metric registry. Bucketed DV01 is registered for all
/// instrument types, while theta is registered globally.
///
/// # Arguments
/// * `registry` - Metric registry to add risk metrics to
///
/// See unit tests and `examples/` for usage.
pub fn register_risk_metrics(registry: &mut super::MetricRegistry) {
    use super::MetricId;
    use std::sync::Arc;

    registry
        .register_metric(
            MetricId::BucketedDv01,
            Arc::new(BucketedDv01Calculator::default()),
            &["Bond", "IRS", "Deposit"],
        )
        .register_metric(MetricId::Theta, Arc::new(ThetaCalculator), &[]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

    fn simple_usd_ois() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.99), (5.0, 0.95), (10.0, 0.90)])
            .set_interp(finstack_core::market_data::interp::InterpStyle::Linear)
            .build()
            .unwrap()
    }

    #[test]
    fn static_bucket_labels_are_stable() {
        let calc = BucketedDv01Calculator::default();
        assert_eq!(calc.bucket_label(0.25), "3M");
        assert_eq!(calc.bucket_label(0.5), "6M");
        assert_eq!(calc.bucket_label(1.0), "1Y");
        assert_eq!(calc.bucket_label(5.0), "5Y");
    }

    #[test]
    fn compute_bucketed_static_and_total_present() {
        let calc = BucketedDv01Calculator::default();
        let disc = simple_usd_ois();
        let base = disc.base_date();
        let dc = DayCount::Act365F;

        let flows = vec![
            (
                base + time::Duration::days(365),
                Money::new(100.0, Currency::USD),
            ),
            (
                base + time::Duration::days(365 * 5),
                Money::new(200.0, Currency::USD),
            ),
        ];

        let out = calc.compute_bucketed(&flows, &disc, dc, base);
        assert!(out.contains_key("bucketed_dv01_1y"));
        assert!(out.contains_key("bucketed_dv01_5y"));
        assert!(out.contains_key("bucketed_dv01_total"));

        let total = out.get("bucketed_dv01_total").copied().unwrap_or(0.0);
        let sum_sub: F = out
            .iter()
            .filter(|(k, _)| k.as_str() != "bucketed_dv01_total")
            .map(|(_, v)| *v)
            .sum();
        assert!((total - sum_sub).abs() < 1e-8);
    }

    #[test]
    fn dynamic_bucket_keys_are_inserted_when_resolver_set() {
        use crate::metrics::traits::BucketKeyResolverFn;

        let calc = BucketedDv01Calculator::default();
        let disc = simple_usd_ois();
        let base = disc.base_date();
        let dc = DayCount::Act365F;

        let flows = vec![
            (
                base + time::Duration::days(365),
                Money::new(100.0, Currency::USD),
            ),
            (
                base + time::Duration::days(365 * 5),
                Money::new(200.0, Currency::USD),
            ),
        ];

        struct DummyInstr {
            attrs: crate::instruments::traits::Attributes,
        }
        impl crate::instruments::traits::Priceable for DummyInstr {
            fn value(
                &self,
                _curves: &finstack_core::market_data::MarketContext,
                _as_of: Date,
            ) -> finstack_core::Result<Money> {
                Ok(Money::new(0.0, Currency::USD))
            }
            fn price_with_metrics(
                &self,
                _curves: &finstack_core::market_data::MarketContext,
                _as_of: Date,
                _metrics: &[MetricId],
            ) -> finstack_core::Result<crate::results::ValuationResult> {
                Err(finstack_core::error::InputError::Invalid.into())
            }
        }
        impl crate::instruments::traits::Attributable for DummyInstr {
            fn attributes(&self) -> &crate::instruments::traits::Attributes {
                &self.attrs
            }
            fn attributes_mut(&mut self) -> &mut crate::instruments::traits::Attributes {
                &mut self.attrs
            }
        }
        impl crate::instruments::traits::InstrumentLike for DummyInstr {
            fn id(&self) -> &str {
                "DUMMY"
            }
            fn instrument_type(&self) -> &'static str {
                "Dummy"
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn clone_box(&self) -> Box<dyn crate::instruments::traits::InstrumentLike> {
                Box::new(DummyInstr { attrs: self.attrs.clone() })
            }
        }

        // Build curves and also keep a separate handle to a discount curve for later checks
        let disc_for_ctx = simple_usd_ois();
        let curves = Arc::new(
            finstack_core::market_data::MarketContext::new().insert_discount(disc_for_ctx),
        );
        let instrument: Arc<dyn crate::instruments::traits::InstrumentLike> =
            Arc::new(DummyInstr {
                attrs: crate::instruments::traits::Attributes::new(),
            });
        let mut ctx = crate::metrics::traits::MetricContext::new(
            instrument,
            curves,
            base,
            Money::new(0.0, Currency::USD),
        );
        ctx.cashflows = Some(flows.clone());
        ctx.discount_curve_id = Some("USD-OIS");
        ctx.day_count = Some(dc);

        let resolver: Arc<BucketKeyResolverFn> = Arc::new(|base_id, label, _instr| {
            MetricId::custom(format!("{}:{}", base_id.as_str(), label))
        });
        ctx.set_bucket_key_resolver(resolver);

        let value_total = calc.calculate(&mut ctx).unwrap();

        assert!(ctx
            .computed
            .contains_key(&MetricId::custom("bucketed_dv01:1y")));
        assert!(ctx
            .computed
            .contains_key(&MetricId::custom("bucketed_dv01:5y")));

        // And values match expected compute_bucketed components
        let disc_for_check = simple_usd_ois();
        let standalone = calc.compute_bucketed(&flows, &disc_for_check, dc, base);
        let v1 = ctx
            .computed
            .get(&MetricId::custom("bucketed_dv01:1y"))
            .copied()
            .unwrap_or(0.0);
        let v5 = ctx
            .computed
            .get(&MetricId::custom("bucketed_dv01:5y"))
            .copied()
            .unwrap_or(0.0);
        assert!((v1 - standalone["bucketed_dv01_1y"]).abs() < 1e-10);
        assert!((v5 - standalone["bucketed_dv01_5y"]).abs() < 1e-10);
        let total = standalone["bucketed_dv01_total"];
        assert!((value_total - total).abs() < 1e-10);
    }
}
