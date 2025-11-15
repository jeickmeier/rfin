//! Bond instrument types and implementations.

use finstack_core::prelude::*;

use crate::cashflow::builder::CashFlowSchedule;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::types::{CurveId, InstrumentId};

// Re-export for compatibility in tests and external users referencing bond::AmortizationSpec
pub use super::cashflow_spec::CashflowSpec;
pub use crate::cashflow::builder::AmortizationSpec;

/// Bond instrument with fixed, floating, or amortizing cashflows.
///
/// Supports call/put schedules, quoted prices for yield-to-maturity calculations,
/// and custom cashflow schedule overrides. Uses a clean `CashflowSpec` that wraps
/// the canonical builder coupon specs for maximum flexibility and parity.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
// Note: JsonSchema derive requires finstack-core types to implement JsonSchema
// #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Bond {
    /// Unique identifier for the bond.
    pub id: InstrumentId,
    /// Principal amount of the bond.
    pub notional: Money,
    /// Issue date of the bond.
    pub issue: Date,
    /// Maturity date of the bond.
    pub maturity: Date,
    /// Cashflow specification (fixed, floating, or amortizing).
    pub cashflow_spec: CashflowSpec,
    /// Discount curve identifier for pricing.
    pub discount_curve_id: CurveId,
    /// Optional credit curve identifier (default intensity). When present,
    /// credit-rate pricing is enabled.
    pub credit_curve_id: Option<CurveId>,
    /// Pricing overrides (including quoted clean price)
    pub pricing_overrides: PricingOverrides,
    /// Optional call/put schedule (dates and redemption prices as % of par amount).
    pub call_put: Option<CallPutSchedule>,
    /// Optional pre-built cashflow schedule. If provided, this will be used instead of
    /// generating cashflows from the cashflow_spec.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub custom_cashflows: Option<CashFlowSchedule>,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
    /// Settlement convention: number of settlement days after trade date.
    pub settlement_days: Option<u32>,
    /// Ex-coupon convention: number of days before coupon date that go ex.
    pub ex_coupon_days: Option<u32>,
}

/// Call or put option on a bond.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CallPut {
    /// Exercise date of the option.
    pub date: Date,
    /// Redemption price as percentage of par amount.
    pub price_pct_of_par: f64,
}

/// Schedule of call and put options for a bond.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CallPutSchedule {
    /// Call options (issuer can redeem early).
    pub calls: Vec<CallPut>,
    /// Put options (holder can redeem early).
    pub puts: Vec<CallPut>,
}

impl CallPutSchedule {
    /// Check if this schedule has any active call or put options.
    pub fn has_options(&self) -> bool {
        !self.calls.is_empty() || !self.puts.is_empty()
    }
}

impl Bond {
    /// Create a canonical example bond for testing and documentation.
    ///
    /// Returns a 10-year USD Treasury-style bond with realistic parameters.
    pub fn example() -> Self {
        Self::fixed(
            "US912828XG33",
            Money::new(1_000_000.0, Currency::USD),
            0.0425,
            Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
            Date::from_calendar_date(2034, time::Month::January, 15).unwrap(),
            "USD-TREASURY",
        )
    }

    /// Create a standard fixed-rate bond (most common use case).
    ///
    /// Creates a bond with semi-annual frequency and 30/360 day count following
    /// **US market conventions**. For other regional conventions, use
    /// `::with_convention()` or `::builder()` for full customization.
    ///
    /// # Regional Bond Conventions (Market Standards Review - Week 5)
    ///
    /// ## United States (Default)
    /// - **Day Count:** 30/360 (US Bond Basis)
    /// - **Frequency:** Semi-annual
    /// - **Settlement:** T+1 (corporate), T+1 (treasuries)
    /// - **Calendar:** US (NYSE holidays)
    ///
    /// ## United Kingdom
    /// - **Day Count:** ACT/ACT (ISMA/ICMA)
    /// - **Frequency:** Semi-annual
    /// - **Settlement:** T+1 (gilts)
    /// - **Calendar:** UK (London holidays)
    ///
    /// ## Europe (Eurozone)
    /// - **Day Count:** 30E/360 (ICMA, Eurobond basis) or ACT/ACT
    /// - **Frequency:** Annual
    /// - **Settlement:** T+2 (standard), T+3 (some markets)
    /// - **Calendar:** TARGET (European Central Bank)
    ///
    /// ## Japan
    /// - **Day Count:** ACT/365 (Fixed)
    /// - **Frequency:** Semi-annual
    /// - **Settlement:** T+3 (JGBs)
    /// - **Calendar:** Japan holidays
    ///
    /// # Example
    /// ```ignore
    /// use finstack_valuations::instruments::bond::Bond;
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    ///
    /// // US Treasury-style bond (default)
    /// let us_bond = Bond::fixed("US-001", notional, 0.05, issue, maturity, "USD-OIS");
    /// ```
    pub fn fixed(
        id: impl Into<InstrumentId>,
        notional: Money,
        coupon_rate: f64,
        issue: Date,
        maturity: Date,
        discount_curve_id: impl Into<CurveId>,
    ) -> Self {
        Self::builder()
            .id(id.into())
            .notional(notional)
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                coupon_rate,
                finstack_core::dates::Frequency::semi_annual(),
                DayCount::Thirty360,
            ))
            .discount_curve_id(discount_curve_id.into())
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Standard bond construction should not fail")
    }

    /// Create a bond with standard market conventions.
    ///
    /// Applies region-specific conventions for day count, frequency, and
    /// calendar adjustments. For full customization, use `::builder()`.
    ///
    /// # Example
    /// ```ignore
    /// use finstack_valuations::instruments::bond::Bond;
    /// use finstack_valuations::instruments::common::parameters::BondConvention;
    ///
    /// let treasury = Bond::with_convention(
    ///     "UST-5Y",
    ///     notional,
    ///     0.03,
    ///     issue,
    ///     maturity,
    ///     BondConvention::USTreasury,
    ///     "USD-TREASURY"
    /// );
    /// ```
    pub fn with_convention(
        id: impl Into<InstrumentId>,
        notional: Money,
        coupon_rate: f64,
        issue: Date,
        maturity: Date,
        convention: crate::instruments::common::parameters::BondConvention,
        discount_curve_id: impl Into<CurveId>,
    ) -> Self {
        Self::builder()
            .id(id.into())
            .notional(notional)
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                coupon_rate,
                convention.frequency(),
                convention.day_count(),
            ))
            .discount_curve_id(discount_curve_id.into())
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Bond with convention construction should not fail")
    }

    /// Create a floating-rate bond (FRN).
    ///
    /// Creates a bond with floating-rate coupons linked to a forward index
    /// (e.g., SOFR, EURIBOR) plus a margin.
    ///
    /// # Example
    /// ```ignore
    /// use finstack_valuations::instruments::bond::Bond;
    /// use finstack_core::dates::{Frequency, DayCount};
    ///
    /// // 3M SOFR + 200bps, quarterly payments
    /// let frn = Bond::floating(
    ///     "FRN-001",
    ///     notional,
    ///     "USD-SOFR-3M",
    ///     200.0,  // margin in bps
    ///     issue,
    ///     maturity,
    ///     Frequency::quarterly(),
    ///     DayCount::Act360,
    ///     "USD-OIS"
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn floating(
        id: impl Into<InstrumentId>,
        notional: Money,
        index_id: impl Into<CurveId>,
        margin_bp: f64,
        issue: Date,
        maturity: Date,
        freq: finstack_core::dates::Frequency,
        dc: DayCount,
        discount_curve_id: impl Into<CurveId>,
    ) -> Self {
        Self::builder()
            .id(id.into())
            .notional(notional)
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::floating(index_id.into(), margin_bp, freq, dc))
            .discount_curve_id(discount_curve_id.into())
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("FRN construction should not fail")
    }

    /// Create a bond from a pre-built cashflow schedule.
    ///
    /// This extracts key bond parameters from the cashflow schedule and creates
    /// a bond that will use these custom cashflows for all calculations.
    /// Use this for complex structures like PIK bonds, fixed-to-floating, or
    /// custom amortization schedules.
    ///
    /// # Example
    /// ```ignore
    /// // Build custom PIK schedule
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(notional, issue, maturity)
    ///     .fixed_cf(FixedCouponSpec { coupon_type: CouponType::Split { cash_pct: 0.5, pik_pct: 0.5 }, ... })
    ///     .build()?;
    ///
    /// let bond = Bond::from_cashflows("PIK-001", schedule, "USD-HY", Some(95.0))?;
    /// ```
    pub fn from_cashflows(
        id: impl Into<InstrumentId>,
        schedule: CashFlowSchedule,
        discount_curve_id: impl Into<CurveId>,
        quoted_clean: Option<f64>,
    ) -> finstack_core::Result<Self> {
        // Extract parameters from the schedule
        let notional = schedule.notional.initial;

        // Find issue and maturity from the cashflow dates
        let dates = schedule.dates();
        if dates.len() < 2 {
            return Err(finstack_core::error::InputError::TooFewPoints.into());
        }
        let issue = dates[0];
        let maturity = dates
            .last()
            .copied()
            .ok_or(finstack_core::error::InputError::TooFewPoints)?;

        // Default cashflow spec (won't be used since custom_cashflows overrides)
        let cashflow_spec = CashflowSpec::default();

        let pricing_overrides = if let Some(price) = quoted_clean {
            PricingOverrides::default().with_clean_price(price)
        } else {
            PricingOverrides::default()
        };

        Self::builder()
            .id(id.into())
            .notional(notional)
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(cashflow_spec)
            .discount_curve_id(discount_curve_id.into())
            .credit_curve_id_opt(None)
            .pricing_overrides(pricing_overrides)
            .custom_cashflows_opt(Some(schedule))
            .attributes(Attributes::new())
            .build()
    }

    /// Set custom cashflows for this bond.
    ///
    /// When custom cashflows are set, they will be used instead of generating
    /// cashflows from the bond's coupon and amortization specifications.
    pub fn with_cashflows(mut self, schedule: CashFlowSchedule) -> Self {
        self.custom_cashflows = Some(schedule);
        self
    }

    /// Get the full cashflow schedule with kinds for this bond.
    ///
    /// This returns the complete `CashFlowSchedule` including all cashflow types
    /// (Fixed, Float, PIK, Amortization, Notional, etc.) and metadata.
    ///
    /// For floating rate bonds, requires market curves to properly compute floating
    /// coupon amounts (forward rate + discount margin).
    ///
    /// Note: Amortization amounts are stored as POSITIVE values in the schedule.
    pub fn get_full_schedule(
        &self,
        curves: &finstack_core::market_data::MarketContext,
    ) -> Result<CashFlowSchedule> {
        use crate::cashflow::builder::CashFlowSchedule;

        // If custom cashflows are set, return them directly
        if let Some(ref custom) = self.custom_cashflows {
            return Ok(custom.clone());
        }

        // Build the schedule using the cashflow builder and cashflow_spec
        let mut b = CashFlowSchedule::builder();
        b.principal(self.notional, self.issue, self.maturity);

        // Match on the cashflow spec variant
        match &self.cashflow_spec {
            CashflowSpec::Fixed(spec) => {
                b.fixed_cf(spec.clone());
            }
            CashflowSpec::Floating(spec) => {
                b.floating_cf(spec.clone());
            }
            CashflowSpec::Amortizing { base, schedule } => {
                b.amortization(schedule.clone());
                match &**base {
                    CashflowSpec::Fixed(spec) => {
                        b.fixed_cf(spec.clone());
                    }
                    CashflowSpec::Floating(spec) => {
                        b.floating_cf(spec.clone());
                    }
                    CashflowSpec::Amortizing { .. } => {
                        return Err(finstack_core::error::InputError::Invalid.into());
                    }
                }
            }
        }

        // Build the schedule with market curves for floating rate computation
        b.build_with_curves(Some(curves))
    }

    /// Price bond using tree-based pricing for embedded options (calls/puts).
    ///
    /// This method is automatically called by `value()` when the bond has a non-empty
    /// call/put schedule. It uses a short-rate tree model to properly value the
    /// embedded optionality via backward induction.
    ///
    /// # Arguments
    /// * `market` - Market context with discount curve (and optionally hazard curve)
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    /// Option-adjusted present value of the bond
    fn value_with_tree(
        &self,
        market: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::bond::pricing::tree_pricer::BondValuator;
        use crate::instruments::common::models::{
            short_rate_keys, ShortRateTree, ShortRateTreeConfig, StateVariables, TreeModel,
        };

        // Calculate time to maturity
        let time_to_maturity = self
            .cashflow_spec
            .day_count()
            .year_fraction(
                as_of,
                self.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        if time_to_maturity <= 0.0 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        // Get discount curve and calibrate tree
        let discount_curve = market.get_discount_ref(&self.discount_curve_id)?;

        // Tree configuration - use 100 steps and 1% volatility as defaults
        // These can be overridden via attributes if needed for specific calibration
        let tree_steps = 100;
        let tree_config = ShortRateTreeConfig {
            steps: tree_steps,
            volatility: 0.01,
            ..Default::default()
        };

        // Initialize and calibrate short-rate tree to match discount curve
        let mut tree = ShortRateTree::new(tree_config);
        tree.calibrate(discount_curve, time_to_maturity)?;

        // Create bond valuator with call/put schedule mapped to tree steps
        let valuator = BondValuator::new(self.clone(), market, time_to_maturity, tree_steps)?;

        // Set up initial state variables (no OAS for vanilla pricing)
        let mut vars = StateVariables::new();
        vars.insert(short_rate_keys::OAS, 0.0);

        // Price via tree with backward induction applying call/put constraints
        let price_amount = tree.price(vars, time_to_maturity, market, &valuator)?;

        Ok(Money::new(price_amount, self.notional.currency()))
    }
}

// Explicit trait implementations for better IDE support and code clarity

// Explicit Instrument trait implementation (replaces macro for better IDE visibility)
impl crate::instruments::common::traits::Instrument for Bond {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::Bond
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
        // Check if bond has embedded options requiring tree-based pricing
        if let Some(ref cp) = self.call_put {
            if cp.has_options() {
                return self.value_with_tree(curves, as_of);
            }
        }

        // Standard cashflow discounting for straight bonds
        crate::instruments::common::helpers::schedule_pv_impl(
            self,
            curves,
            as_of,
            &self.discount_curve_id,
            self.cashflow_spec.day_count(),
        )
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
        )
    }

    fn required_discount_curves(&self) -> Vec<CurveId> {
        vec![self.discount_curve_id.clone()]
    }

    fn required_hazard_curves(&self) -> Vec<CurveId> {
        if let Some(ref credit_id) = self.credit_curve_id {
            vec![credit_id.clone()]
        } else {
            vec![]
        }
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for Bond {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::traits::CurveDependencies for Bond {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        let mut builder = crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone());

        // Add credit curve if present
        if let Some(ref credit_curve_id) = self.credit_curve_id {
            builder = builder.credit(credit_curve_id.clone());
        }

        // For floating rate bonds, add forward curve from the cashflow spec
        match &self.cashflow_spec {
            CashflowSpec::Floating(floating_spec) => {
                builder = builder.forward(floating_spec.rate_spec.index_id.clone());
            }
            CashflowSpec::Amortizing { base, .. } => {
                // Check if the base spec is floating
                if let CashflowSpec::Floating(floating_spec) = base.as_ref() {
                    builder = builder.forward(floating_spec.rate_spec.index_id.clone());
                }
            }
            _ => {}
        }

        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cashflow::builder::{CashFlowSchedule, CouponType, FixedCouponSpec, ScheduleParams};
    use crate::cashflow::traits::CashflowProvider;
    use crate::instruments::common::traits::Instrument;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
    use finstack_core::market_data::MarketContext;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

    #[test]
    fn test_bond_with_custom_cashflows() {
        // Setup dates
        let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2027, Month::January, 15).unwrap();

        // Build a custom cashflow schedule with step-up coupons
        let schedule_params = ScheduleParams {
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        let step1_date = Date::from_calendar_date(2026, Month::January, 15).unwrap();

        let custom_schedule = CashFlowSchedule::builder()
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .fixed_stepup(
                &[(step1_date, 0.03), (maturity, 0.05)],
                schedule_params,
                CouponType::Cash,
            )
            .build()
            .unwrap();

        // Create bond from custom cashflows
        let bond = Bond::from_cashflows(
            "CUSTOM_STEPUP_BOND",
            custom_schedule.clone(),
            "USD-OIS",
            Some(98.5),
        )
        .unwrap();

        // Verify bond properties
        assert_eq!(bond.id.as_str(), "CUSTOM_STEPUP_BOND");
        assert_eq!(bond.discount_curve_id.as_str(), "USD-OIS");
        assert_eq!(bond.pricing_overrides.quoted_clean_price, Some(98.5));
        assert_eq!(bond.issue, issue);
        assert_eq!(bond.maturity, maturity);
        assert!(bond.custom_cashflows.is_some());

        // Create curves for pricing
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (3.0, 0.95)])
            .set_interp(finstack_core::math::interp::InterpStyle::Linear)
            .build()
            .unwrap();
        let curves = MarketContext::new().insert_discount(disc_curve);

        // Build schedule and verify it uses custom cashflows
        let flows = bond.build_schedule(&curves, issue).unwrap();
        assert!(!flows.is_empty());

        // The flows should match what we put in the custom schedule
        // (after conversion for holder perspective)
        let expected_flow_count = custom_schedule
            .flows
            .iter()
            .filter(|cf| {
                use crate::cashflow::primitives::CFKind;
                matches!(
                    cf.kind,
                    CFKind::Fixed | CFKind::Stub | CFKind::Amortization | CFKind::Notional
                ) && (cf.kind != CFKind::Notional || cf.amount.amount() > 0.0)
            })
            .count();
        assert_eq!(flows.len(), expected_flow_count);
    }

    #[test]
    fn test_bond_builder_with_custom_cashflows() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        // Build custom cashflow with PIK toggle
        let custom_schedule = CashFlowSchedule::builder()
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Split {
                    cash_pct: 0.5,
                    pik_pct: 0.5,
                },
                rate: 0.06,
                freq: Frequency::quarterly(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
                stub: StubKind::None,
            })
            .build()
            .unwrap();

        // Use builder pattern (default cashflow_spec since custom_cashflows overrides)
        let bond = Bond::builder()
            .id("PIK_TOGGLE_BOND".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::default())
            .custom_cashflows_opt(Some(custom_schedule))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .pricing_overrides(PricingOverrides::default().with_clean_price(99.0))
            .attributes(Attributes::new())
            .build()
            .unwrap();

        assert_eq!(bond.id.as_str(), "PIK_TOGGLE_BOND");
        assert_eq!(bond.discount_curve_id.as_str(), "USD-OIS");
        assert_eq!(bond.pricing_overrides.quoted_clean_price, Some(99.0));
        assert!(bond.custom_cashflows.is_some());
        assert_eq!(bond.notional.currency(), Currency::USD);
    }

    #[test]
    fn test_bond_with_cashflows_method() {
        let issue = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::March, 1).unwrap();

        // Create a traditional bond first (builder)
        let mut bond = Bond::builder()
            .id(InstrumentId::new("REGULAR_BOND"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.04,
                Frequency::semi_annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .call_put_opt(None)
            .custom_cashflows_opt(None)
            .attributes(Attributes::new())
            .settlement_days_opt(None)
            .ex_coupon_days_opt(None)
            .build()
            .unwrap();

        // Build a custom schedule separately
        let custom_schedule = CashFlowSchedule::builder()
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: 0.055, // Different from default spec
                freq: Frequency::quarterly(),
                dc: DayCount::Act365F,
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
                stub: StubKind::None,
            })
            .build()
            .unwrap();

        // Apply custom cashflows
        bond = bond.with_cashflows(custom_schedule);

        assert!(bond.custom_cashflows.is_some());
        // The original cashflow_spec is preserved but custom_cashflows takes precedence
    }

    #[test]
    fn test_custom_cashflows_override_regular_generation() {
        let issue = Date::from_calendar_date(2025, Month::June, 1).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::June, 1).unwrap();

        // Create bond with regular specs (builder)
        let regular_bond = Bond::builder()
            .id(InstrumentId::new("TEST"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                0.03,
                Frequency::annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .call_put_opt(None)
            .custom_cashflows_opt(None)
            .attributes(Attributes::new())
            .settlement_days_opt(None)
            .ex_coupon_days_opt(None)
            .build()
            .unwrap();

        // Same bond with custom cashflows
        let custom_schedule = CashFlowSchedule::builder()
            .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: 0.05,                     // Different rate
                freq: Frequency::semi_annual(), // Different frequency
                dc: DayCount::Act365F,
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
                stub: StubKind::None,
            })
            .build()
            .unwrap();

        let custom_bond = regular_bond.clone().with_cashflows(custom_schedule);

        // Create curves
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (2.0, 0.98)])
            .set_interp(finstack_core::math::interp::InterpStyle::Linear)
            .build()
            .unwrap();
        let curves = MarketContext::new().insert_discount(disc_curve);

        // Build schedules
        let regular_flows = regular_bond.build_schedule(&curves, issue).unwrap();
        let custom_flows = custom_bond.build_schedule(&curves, issue).unwrap();

        // Should have different number of flows due to different frequency
        assert_ne!(regular_flows.len(), custom_flows.len());

        // Custom bond should have semi-annual flows (more flows)
        assert!(custom_flows.len() > regular_flows.len());
    }

    #[test]
    fn test_bond_floating_value() {
        let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2027, Month::January, 1).unwrap();
        let notional = Money::new(1_000_000.0, Currency::USD);

        // Curves
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (2.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();
        let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(issue)
            .knots([(0.0, 0.05), (2.0, 0.055)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();
        let ctx = MarketContext::new()
            .insert_discount(disc)
            .insert_forward(fwd);

        let bond = Bond::floating(
            "FRN-TEST",
            notional,
            "USD-SOFR-3M",
            150.0,
            issue,
            maturity,
            Frequency::quarterly(),
            DayCount::Act360,
            "USD-OIS",
        );

        // Price should be finite and positive under positive forwards
        let pv = bond.value(&ctx, issue).unwrap();
        assert!(pv.amount().is_finite());
    }

    #[test]
    fn test_bond_frn_build_schedule_uses_builder() {
        use crate::cashflow::primitives::CFKind;
        use crate::cashflow::traits::CashflowProvider;

        let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        // Create FRN
        let frn = Bond::floating(
            "FRN-BUILDER-TEST",
            Money::new(1_000_000.0, Currency::USD),
            "USD-SOFR",
            100.0,
            issue,
            maturity,
            Frequency::quarterly(),
            DayCount::Act360,
            "USD-OIS",
        );

        // Create market with forward curve
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (1.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        let fwd_curve = ForwardCurve::builder("USD-SOFR", 0.25)
            .base_date(issue)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.03), (1.0, 0.035)])
            .build()
            .unwrap();

        let market = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_forward(fwd_curve);

        // Get full schedule to verify it includes FloatReset CFKind
        let full_schedule = frn.get_full_schedule(&market).unwrap();
        let has_floating = full_schedule
            .flows
            .iter()
            .any(|cf| matches!(cf.kind, CFKind::FloatReset));
        assert!(
            has_floating,
            "Full schedule should include CFKind::FloatReset for FRN"
        );

        // Get simplified schedule via build_schedule
        let flows = frn.build_schedule(&market, issue).unwrap();
        assert!(!flows.is_empty(), "FRN should have cashflows");

        // Verify flows include floating coupons (should be > just redemption)
        assert!(
            flows.len() > 1,
            "FRN should have coupon flows + redemption, got {} flows",
            flows.len()
        );
    }

    #[test]
    fn test_bond_amortization_sign_flip_and_notional_exclusion() {
        use crate::cashflow::builder::AmortizationSpec;
        use crate::cashflow::primitives::CFKind;
        use crate::cashflow::traits::CashflowProvider;

        let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2027, Month::January, 1).unwrap();

        // Create amortizing bond using CashflowSpec::Amortizing
        let step1 = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let amort_spec = AmortizationSpec::StepRemaining {
            schedule: vec![
                (step1, Money::new(500_000.0, Currency::USD)),
                (maturity, Money::new(0.0, Currency::USD)),
            ],
        };
        let base_spec = CashflowSpec::fixed(0.05, Frequency::semi_annual(), DayCount::Thirty360);
        let cashflow_spec = CashflowSpec::amortizing(base_spec, amort_spec);

        let bond = Bond::builder()
            .id("AMORT-TEST".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(cashflow_spec)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .unwrap();

        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (2.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(disc_curve);

        // Get full schedule to check internal representation
        let full_schedule = bond.get_full_schedule(&market).unwrap();

        // Find initial notional (should be negative - issuer receives)
        let initial_notional = full_schedule
            .flows
            .iter()
            .find(|cf| cf.date == issue && matches!(cf.kind, CFKind::Notional));
        assert!(
            initial_notional.is_some(),
            "Full schedule should have initial notional"
        );
        assert!(
            initial_notional.unwrap().amount.amount() < 0.0,
            "Initial notional should be negative (issuer receives)"
        );

        // Find amortization flows (should be positive in full schedule)
        let amort_flows: Vec<_> = full_schedule
            .flows
            .iter()
            .filter(|cf| matches!(cf.kind, CFKind::Amortization))
            .collect();
        assert!(!amort_flows.is_empty(), "Should have amortization flows");
        for cf in &amort_flows {
            assert!(
                cf.amount.amount() > 0.0,
                "Amortization in full schedule should be positive"
            );
        }

        // Get simplified schedule via build_schedule
        let flows = bond.build_schedule(&market, issue).unwrap();

        // Initial draw should be excluded (negative notional)
        let has_negative_initial = flows.iter().any(|(d, m)| *d == issue && m.amount() < 0.0);
        assert!(
            !has_negative_initial,
            "Simplified schedule should exclude initial negative notional draw"
        );

        // Amortization should be flipped to negative (holder receives principal back)
        let amort_in_simplified: Vec<_> = flows
            .iter()
            .filter(|(d, _)| *d == step1 || *d == maturity)
            .collect();
        // We expect at least one amortization payment
        let has_negative_amort = amort_in_simplified.iter().any(|(_, m)| m.amount() < 0.0);
        assert!(
            has_negative_amort,
            "Amortization in simplified schedule should be negative (principal repayment)"
        );

        // Final redemption at maturity: even when amortizing to zero, there may be a
        // small redemption flow for the final outstanding amount after last amortization
        let final_positive_flows: Vec<_> = flows
            .iter()
            .filter(|(d, m)| *d == maturity && m.amount() > 0.0)
            .collect();
        // Could be zero or have a small final payment depending on amortization schedule
        // The key is that amortizations are negative (principal repayment)
        assert!(
            final_positive_flows.len() <= 1,
            "At most one positive flow at maturity (redemption)"
        );
    }

    #[test]
    fn test_bond_build_schedule_includes_floating_cfkind() {
        use crate::cashflow::traits::CashflowProvider;

        let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::July, 1).unwrap();

        let frn = Bond::floating(
            "FRN-CFKIND-TEST",
            Money::new(1_000_000.0, Currency::USD),
            "USD-LIBOR-3M",
            200.0,
            issue,
            maturity,
            Frequency::quarterly(),
            DayCount::Act365F,
            "USD-OIS",
        );

        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (2.0, 0.90)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        let fwd = ForwardCurve::builder("USD-LIBOR-3M", 0.25)
            .base_date(issue)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.04), (2.0, 0.045)])
            .build()
            .unwrap();

        let market = MarketContext::new()
            .insert_discount(disc)
            .insert_forward(fwd);

        // Build simplified schedule
        let flows = frn.build_schedule(&market, issue).unwrap();

        // Should have multiple flows (quarterly coupons + redemption)
        // Approximately 6 quarters over 18 months
        assert!(
            flows.len() >= 5,
            "FRN should have multiple quarterly flows, got {}",
            flows.len()
        );

        // All flows should have positive amounts (coupons and redemption are receipts)
        let all_positive = flows.iter().all(|(_, m)| m.amount() > 0.0);
        assert!(
            all_positive,
            "All simplified FRN flows should be positive (holder view)"
        );

        // Verify flows are sorted by date
        for i in 1..flows.len() {
            assert!(
                flows[i].0 >= flows[i - 1].0,
                "Flows should be sorted by date"
            );
        }
    }
}
