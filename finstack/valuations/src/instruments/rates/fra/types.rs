//! Forward Rate Agreement (FRA) instrument types and trait implementations.
//!
//! Defines the `ForwardRateAgreement` instrument in the modern instrument style
//! used across valuations. Core PV logic is delegated to the pricing engine in
//! `pricing::engine`, and metrics are provided in the `metrics` submodule.

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::traits::Attributes;
use finstack_core::currency::Currency;
use finstack_core::dates::{adjust, BusinessDayConvention, CalendarRegistry, Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId, Rate};
use time::macros::date;

// =============================================================================
// Constants
// =============================================================================

/// Minimum denominator for settlement adjustment to avoid division issues.
/// When 1 + F × τ is below this threshold, the forward rate is considered invalid.
const MIN_SETTLEMENT_DENOM: f64 = 1e-12;

/// Minimum period length (in year fractions) for a valid FRA.
const MIN_PERIOD_LENGTH: f64 = 1e-12;

/// Minimum reasonable forward rate for validation warnings (-10%).
const MIN_REASONABLE_RATE: f64 = -0.10;

/// Maximum reasonable forward rate for validation warnings (50%).
const MAX_REASONABLE_RATE: f64 = 0.50;

/// Forward Rate Agreement instrument.
///
/// A FRA is a forward contract on an interest rate. The holder receives
/// the difference between the realized rate and the fixed rate, paid at
/// the start of the interest period (FRA convention).
///
/// # Direction Convention
///
/// - `receive_fixed = true`: Receive fixed rate, pay floating rate.
///   When forward rate > fixed rate, PV is negative (you're paying more than receiving).
/// - `receive_fixed = false`: Pay fixed rate, receive floating rate.
///   When forward rate > fixed rate, PV is positive (you're receiving more than paying).
///
/// # Migration from `pay_fixed`
///
/// Prior versions used `pay_fixed` with opposite semantics. The deserializer
/// handles this by:
/// 1. Detecting the legacy `pay_fixed` field
/// 2. Logging a deprecation warning
/// 3. Inverting the value to maintain correct trade economics
///
/// **Action Required**: Update JSON/configs to use `receive_fixed` with correct semantics.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ForwardRateAgreement {
    /// Unique identifier
    pub id: InstrumentId,
    /// Notional amount
    pub notional: Money,
    /// Rate fixing date. If `None`, inferred from `start_date - reset_lag` business days.
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fixing_date: Option<Date>,
    /// Interest period start date
    pub start_date: Date,
    /// Interest period end date
    pub end_date: Date,
    /// Fixed rate (decimal, e.g., 0.05 for 5%)
    pub fixed_rate: f64,
    /// Day count convention for interest accrual
    pub day_count: DayCount,
    /// Reset lag in business days (fixing to value date)
    pub reset_lag: i32,
    /// Optional fixing calendar identifier for business day adjustment
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fixing_calendar_id: Option<String>,
    /// Optional business day convention for fixing date adjustment (default: ModifiedFollowing)
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fixing_bdc: Option<BusinessDayConvention>,
    /// Optional observed fixing (locked rate) when known
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub observed_fixing: Option<f64>,
    /// Discount curve identifier
    pub discount_curve_id: CurveId,
    /// Forward curve identifier
    pub forward_id: CurveId,
    /// Direction: true = receive fixed rate, pay floating rate.
    pub receive_fixed: bool,
    /// Attributes for scenario selection
    pub attributes: Attributes,
}

/// Custom deserializer for ForwardRateAgreement that handles the `pay_fixed` → `receive_fixed`
/// migration with proper semantic inversion.
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ForwardRateAgreement {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        /// Helper struct that matches the JSON structure, accepting either field name.
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct FraHelper {
            id: InstrumentId,
            notional: Money,
            #[serde(default)]
            fixing_date: Option<Date>,
            start_date: Date,
            end_date: Date,
            fixed_rate: f64,
            day_count: DayCount,
            reset_lag: i32,
            #[serde(default)]
            fixing_calendar_id: Option<String>,
            #[serde(default)]
            fixing_bdc: Option<BusinessDayConvention>,
            #[serde(default)]
            observed_fixing: Option<f64>,
            discount_curve_id: CurveId,
            forward_id: CurveId,
            /// New field name with correct semantics
            #[serde(default)]
            receive_fixed: Option<bool>,
            /// Legacy field name with inverted semantics (DEPRECATED)
            #[serde(default)]
            pay_fixed: Option<bool>,
            attributes: Attributes,
        }

        let helper = FraHelper::deserialize(deserializer)?;

        // Determine receive_fixed value, handling the semantic inversion for pay_fixed
        let receive_fixed = match (helper.receive_fixed, helper.pay_fixed) {
            (Some(rf), None) => rf,
            (None, Some(pf)) => {
                // DEPRECATED: pay_fixed has inverted semantics
                // pay_fixed=true meant "pay fixed, receive floating" which is receive_fixed=false
                tracing::warn!(
                    instrument_id = %helper.id.as_str(),
                    pay_fixed = pf,
                    computed_receive_fixed = !pf,
                    "FRA uses deprecated 'pay_fixed' field. This field has inverted semantics \
                     and will be removed in a future version. Please migrate to 'receive_fixed'. \
                     pay_fixed=true → receive_fixed=false (pay fixed, receive floating); \
                     pay_fixed=false → receive_fixed=true (receive fixed, pay floating)"
                );
                // Invert the value to correct the semantics
                !pf
            }
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "FRA cannot have both 'receive_fixed' and 'pay_fixed' fields; \
                     remove the deprecated 'pay_fixed' field",
                ));
            }
            (None, None) => {
                return Err(serde::de::Error::missing_field("receive_fixed"));
            }
        };

        Ok(ForwardRateAgreement {
            id: helper.id,
            notional: helper.notional,
            fixing_date: helper.fixing_date,
            start_date: helper.start_date,
            end_date: helper.end_date,
            fixed_rate: helper.fixed_rate,
            day_count: helper.day_count,
            reset_lag: helper.reset_lag,
            fixing_calendar_id: helper.fixing_calendar_id,
            fixing_bdc: helper.fixing_bdc,
            observed_fixing: helper.observed_fixing,
            discount_curve_id: helper.discount_curve_id,
            forward_id: helper.forward_id,
            receive_fixed,
            attributes: helper.attributes,
        })
    }
}

impl ForwardRateAgreement {
    /// Create a canonical example FRA for testing and documentation.
    ///
    /// Returns a 3x6 FRA (3 months forward, 3 month tenor).
    pub fn example() -> Self {
        // SAFETY: All inputs are compile-time validated constants
        Self::builder()
            .id(InstrumentId::new("FRA-3X6-USD"))
            .notional(Money::new(10_000_000.0, Currency::USD))
            .fixing_date(date!(2024 - 04 - 01))
            .start_date(date!(2024 - 04 - 03))
            .end_date(date!(2024 - 07 - 03))
            .fixed_rate(0.045)
            .day_count(DayCount::Act360)
            .reset_lag(2)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .forward_id(CurveId::new("USD-SOFR-3M"))
            .receive_fixed(true)
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| unreachable!("Example FRA with valid constants should never fail"))
    }

    /// Calculate the net present value of this FRA (rounded Money).
    pub fn npv(
        &self,
        context: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        let pv = self.npv_raw(context, as_of)?;
        Ok(Money::new(pv, self.notional.currency()))
    }

    /// Settlement amount at period start (undiscounted).
    ///
    /// Returns the cashflow paid at `start_date` using standard FRA settlement
    /// convention: N × τ × (F - K) / (1 + F × τ), signed by direction.
    ///
    /// # Errors
    ///
    /// - Returns error if fixing date has passed but no `observed_fixing` is provided
    /// - Returns error if settlement denominator is pathological (forward rate ≈ -1/τ)
    fn settlement_amount_raw(
        &self,
        context: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        use finstack_core::dates::DateExt;

        if as_of >= self.start_date {
            return Ok(0.0);
        }

        // Determine fixing date: use explicit fixing_date if provided,
        // otherwise infer from start_date - reset_lag business days.
        //
        // IMPORTANT: reset_lag is in BUSINESS DAYS, not calendar days.
        let fixing_date = match self.fixing_date {
            Some(explicit_date) => explicit_date,
            None => {
                let bdc = self
                    .fixing_bdc
                    .unwrap_or(BusinessDayConvention::ModifiedFollowing);

                // Compute base fixing date by subtracting reset_lag business days
                let base_fixing_date = if let Some(cal_id) = self.fixing_calendar_id.as_deref() {
                    if let Some(cal) = CalendarRegistry::global().resolve_str(cal_id) {
                        // Use calendar-aware business day subtraction
                        self.start_date.add_business_days(-(self.reset_lag), cal)?
                    } else {
                        // Calendar specified but not found - fall back to weekday-only
                        tracing::warn!(
                            instrument_id = %self.id.as_str(),
                            calendar_id = cal_id,
                            "FRA fixing calendar not found; using weekday-only reset lag"
                        );
                        self.start_date.add_weekdays(-(self.reset_lag))
                    }
                } else {
                    // No calendar specified - use weekday-only (Mon-Fri) subtraction
                    self.start_date.add_weekdays(-(self.reset_lag))
                };

                // Apply business day convention adjustment to the resulting date
                if let Some(cal_id) = self.fixing_calendar_id.as_deref() {
                    if let Some(cal) = CalendarRegistry::global().resolve_str(cal_id) {
                        adjust(base_fixing_date, bdc, cal)?
                    } else {
                        base_fixing_date
                    }
                } else {
                    base_fixing_date
                }
            }
        };

        let fwd = context.get_forward(&self.forward_id)?;

        // Time fractions for mapping into the forward curve domain must use the
        // forward curve's own day-count/time basis, not the instrument accrual basis.
        let fwd_base = fwd.base_date();
        let fwd_dc = fwd.day_count();
        let t_start = fwd_dc
            .year_fraction(
                fwd_base,
                self.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);
        let t_end = fwd_dc
            .year_fraction(
                fwd_base,
                self.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(t_start);

        // Accrual factor
        let tau = self
            .day_count
            .year_fraction(
                self.start_date,
                self.end_date,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);

        // Zero-length period produces zero settlement
        if tau < MIN_PERIOD_LENGTH {
            return Ok(0.0);
        }

        // Forward rate over the period
        // If fixing date has passed, require observed fixing to avoid ambiguity
        let forward_rate = if as_of >= fixing_date {
            self.observed_fixing.ok_or_else(|| {
                finstack_core::Error::Validation(format!(
                    "FRA '{}': fixing date {} has passed (as_of={}) but no observed_fixing provided",
                    self.id, fixing_date, as_of
                ))
            })?
        } else {
            fwd.rate_period(t_start, t_end)
        };

        // Warn if forward rate is outside reasonable bounds (likely data error)
        if !(MIN_REASONABLE_RATE..=MAX_REASONABLE_RATE).contains(&forward_rate) {
            tracing::warn!(
                instrument_id = %self.id.as_str(),
                forward_rate,
                min_bound = MIN_REASONABLE_RATE,
                max_bound = MAX_REASONABLE_RATE,
                "FRA forward rate outside typical bounds; possible market data error"
            );
        }

        // Market-standard FRA settlement at period start includes the
        // settlement discounting adjustment 1 / (1 + F × τ).
        let rate_diff = forward_rate - self.fixed_rate;
        let denom = 1.0_f64 + forward_rate * tau;

        // Denominator near zero indicates pathological forward rate (F ≈ -1/τ)
        if denom.abs() <= MIN_SETTLEMENT_DENOM {
            return Err(finstack_core::Error::Validation(format!(
                "FRA '{}': settlement denominator near zero (forward_rate={}, tau={}); \
                 forward rate implies F ≈ {:.2}% which is pathological",
                self.id,
                forward_rate,
                tau,
                -100.0 / tau
            )));
        }

        let settlement = self.notional.amount() * rate_diff * tau / denom;

        // Apply direction: receive_fixed means we receive K and pay F
        // When F > K: rate_diff > 0, settlement > 0 (we owe money)
        // So negate when receive_fixed = true
        Ok(if self.receive_fixed {
            -settlement
        } else {
            settlement
        })
    }

    /// Calculate the raw net present value of this FRA (unrounded f64)
    ///
    /// # Reset Lag Handling
    ///
    /// The fixing date is inferred from `start_date - reset_lag` using **business days**
    /// when a calendar is available, or weekday-only subtraction otherwise. This aligns
    /// with market conventions where reset lag is specified in business days (e.g., T-2).
    ///
    /// The inferred date is then adjusted according to `fixing_bdc` (defaults to ModifiedFollowing).
    pub fn npv_raw(
        &self,
        context: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        // Settlement for a FRA occurs at the start of the accrual period; past
        // settlement implies zero PV.
        if as_of >= self.start_date {
            return Ok(0.0);
        }

        let settlement = self.settlement_amount_raw(context, as_of)?;
        let disc = context.get_discount(&self.discount_curve_id)?;
        let df_settlement = disc.df_between_dates(as_of, self.start_date)?;
        Ok(settlement * df_settlement)
    }
}

impl ForwardRateAgreementBuilder {
    /// Set the fixed rate using a typed rate.
    pub fn fixed_rate_rate(mut self, rate: Rate) -> Self {
        self.fixed_rate = Some(rate.as_decimal());
        self
    }

    /// Set the observed fixing using a typed rate.
    pub fn observed_fixing_rate(mut self, rate: Rate) -> Self {
        self.observed_fixing = Some(rate.as_decimal());
        self
    }

    /// Deprecated alias for `receive_fixed()`.
    ///
    /// This method exists for backward compatibility. New code should use
    /// `receive_fixed()` which has clearer semantics.
    #[deprecated(
        since = "0.9.0",
        note = "Use receive_fixed() instead; pay_fixed was misnamed"
    )]
    pub fn pay_fixed(self, receive_fixed: bool) -> Self {
        self.receive_fixed(receive_fixed)
    }
}

// Explicit Instrument trait implementation (replaces macro for better IDE visibility)
impl crate::instruments::common::traits::Instrument for ForwardRateAgreement {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FRA
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
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Call the instrument's own NPV method
        self.npv(curves, as_of)
    }

    fn value_raw(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        self.npv_raw(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
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
            None,
            None,
        )
    }

    fn as_cashflow_provider(&self) -> Option<&dyn crate::cashflow::traits::CashflowProvider> {
        Some(self)
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for ForwardRateAgreement {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::traits::CurveDependencies for ForwardRateAgreement {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.forward_id.clone())
            .build()
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for ForwardRateAgreement {
    fn forward_curve_ids(&self) -> Vec<finstack_core::types::CurveId> {
        vec![self.forward_id.clone()]
    }
}

impl CashflowProvider for ForwardRateAgreement {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

    fn build_full_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        // Settlement at start of accrual period; if already settled, no flows
        let flows = if self.start_date <= as_of {
            Vec::new()
        } else {
            let settlement = self.settlement_amount_raw(curves, as_of)?;
            vec![(
                self.start_date,
                Money::new(settlement, self.notional.currency()),
            )]
        };

        Ok(crate::cashflow::traits::schedule_from_dated_flows(
            flows,
            self.notional(),
            self.day_count,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    #[cfg(feature = "slow")]
    use super::*;
    #[cfg(feature = "slow")]
    use crate::instruments::common::traits::Instrument;
    #[cfg(feature = "slow")]
    use finstack_core::currency::Currency;
    #[cfg(feature = "slow")]
    use finstack_core::dates::Date;
    #[cfg(feature = "slow")]
    use finstack_core::market_data::term_structures::DiscountCurve;
    #[cfg(feature = "slow")]
    use finstack_core::market_data::term_structures::ForwardCurve;
    #[cfg(feature = "slow")]
    use finstack_core::math::interp::InterpStyle;
    #[cfg(feature = "slow")]
    use time::Month;

    #[test]
    #[cfg(feature = "slow")]
    fn fra_par_pv_near_zero_with_settlement_adjustment() {
        // Build simple flat curves: 5% forward, discount with reasonable decay
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = DiscountCurve::builder("DISC")
            .base_date(base)
            .knots([(0.0, 1.0), (5.0, 0.78)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("FRA builder should succeed in test");

        let fwd = ForwardCurve::builder("FWD-3M", 0.25)
            .base_date(base)
            .knots([(0.0, 0.05), (5.0, 0.05)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("FRA builder should succeed in test");

        let ctx = MarketContext::new()
            .insert_discount(disc)
            .insert_forward(fwd);

        // FRA 3M x 6M
        let start = base + time::Duration::days(90);
        let end = base + time::Duration::days(180);
        let fixing = start - time::Duration::days(2); // 2 days before start for reset_lag
        let fra = ForwardRateAgreement::builder()
            .id("FRA-3x6".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .fixing_date(fixing)
            .start_date(start)
            .end_date(end)
            .fixed_rate(0.05)
            .day_count(finstack_core::dates::DayCount::Act360)
            .reset_lag(2)
            .discount_curve_id("DISC".into())
            .forward_id("FWD-3M".into())
            .receive_fixed(false) // Pay fixed, receive floating
            .build()
            .expect("FRA builder should succeed in test");

        let pv = fra
            .value(&ctx, base)
            .expect("FRA valuation should succeed in test");
        // With settlement adjustment PV should be very close to zero at par
        assert!(
            pv.amount().abs() < 0.01,
            "FRA PV not near zero: {}",
            pv.amount()
        );
    }

    #[test]
    #[cfg(feature = "slow")]
    fn fra_par_rate_metric() {
        // Build simple flat curves
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let disc = DiscountCurve::builder("DISC")
            .base_date(base)
            .knots([(0.0, 1.0), (5.0, 0.78)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("FRA builder should succeed in test");

        let fwd_rate = 0.05;
        let fwd = ForwardCurve::builder("FWD-3M", 0.25)
            .base_date(base)
            .knots([(0.0, fwd_rate), (5.0, fwd_rate)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("FRA builder should succeed in test");

        let ctx = MarketContext::new()
            .insert_discount(disc)
            .insert_forward(fwd);

        let start = base + time::Duration::days(90);
        let end = base + time::Duration::days(180);
        let fixing = start - time::Duration::days(2);

        let fra = ForwardRateAgreement::builder()
            .id("FRA-TEST".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .fixing_date(fixing)
            .start_date(start)
            .end_date(end)
            .fixed_rate(0.04) // Different from market rate
            .day_count(finstack_core::dates::DayCount::Act360)
            .reset_lag(2)
            .discount_curve_id("DISC".into())
            .forward_id("FWD-3M".into())
            .receive_fixed(true)
            .build()
            .expect("Builder failed");

        use crate::instruments::common::traits::Instrument;
        use crate::instruments::fra::metrics::FraParRateCalculator;
        use crate::metrics::{MetricCalculator, MetricContext};
        use std::sync::Arc;

        // Wrap in Arc for metric context
        let fra_arc = Arc::new(fra);
        let ctx_arc = Arc::new(ctx);

        // Calculate base PV
        let base_pv = fra_arc
            .value(&ctx_arc, base)
            .expect("PV calculation failed");

        let calc = FraParRateCalculator;
        let mut m_ctx = MetricContext::new(
            fra_arc as Arc<dyn Instrument>,
            ctx_arc,
            base,
            base_pv,
            MetricContext::default_config(),
        );

        let par_rate = calc
            .calculate(&mut m_ctx)
            .expect("Par rate calculation failed");

        assert!(
            (par_rate - fwd_rate).abs() < 1e-10,
            "Par rate {} should equal forward rate {}",
            par_rate,
            fwd_rate
        );
    }
}
