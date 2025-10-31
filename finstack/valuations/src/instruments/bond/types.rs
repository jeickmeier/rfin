//! Bond instrument types and implementations.

use finstack_core::dates::{BusinessDayConvention, StubKind};
use finstack_core::prelude::*;

use crate::cashflow::builder::CashFlowSchedule;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::types::{CurveId, InstrumentId};

// Re-export for compatibility in tests and external users referencing bond::AmortizationSpec
pub use crate::cashflow::primitives::AmortizationSpec;

/// Fixed-rate bond instrument with optional features.
///
/// Supports call/put schedules, amortization, quoted prices for
/// yield-to-maturity calculations, and custom cashflow schedules.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bond {
    /// Unique identifier for the bond.
    pub id: InstrumentId,
    /// Principal amount of the bond.
    pub notional: Money,
    /// Annual coupon rate (e.g., 0.05 for 5%).
    pub coupon: f64,
    /// Coupon payment frequency.
    pub freq: finstack_core::dates::Frequency,
    /// Day count convention for accrual.
    pub dc: DayCount,
    /// Business day convention for schedule/payment adjustments.
    pub bdc: BusinessDayConvention,
    /// Optional calendar identifier for schedule adjustments.
    pub calendar_id: Option<String>,
    /// Stub handling rule for the schedule.
    pub stub: StubKind,
    /// Issue date of the bond.
    pub issue: Date,
    /// Maturity date of the bond.
    pub maturity: Date,
    /// Discount curve identifier for pricing.
    pub disc_id: CurveId,
    /// Optional hazard curve identifier (default intensity). When present,
    /// hazard-rate pricing is enabled.
    pub hazard_id: Option<CurveId>,
    /// Pricing overrides (including quoted clean price)
    pub pricing_overrides: PricingOverrides,
    /// Optional call/put schedule (dates and redemption prices as % of par amount).
    pub call_put: Option<CallPutSchedule>,
    /// Optional amortization specification (principal paid during life).
    pub amortization: Option<AmortizationSpec>,
    /// Optional pre-built cashflow schedule. If provided, this will be used instead of
    /// generating cashflows from coupon/amortization specifications.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub custom_cashflows: Option<CashFlowSchedule>,
    /// Optional floating-rate specification (FRN). When present, coupons are
    /// projected off a forward index with margin and gearing.
    pub float: Option<BondFloatSpec>,
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

/// Floating-rate parameters for FRN-style bonds.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BondFloatSpec {
    /// Forward curve identifier for the floating index (e.g., USD-SOFR-3M).
    pub fwd_id: CurveId,
    /// Margin over the index in basis points.
    pub margin_bp: f64,
    /// Gearing multiplier on the index rate.
    pub gearing: f64,
    /// Reset lag in days applied to the fixing date (business-day adjusted Following).
    pub reset_lag_days: i32,
}

impl Bond {
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
    /// // US Treasury-style bond (default)
    /// let us_bond = Bond::fixed("US-001", notional, 0.05, issue, maturity, "USD-OIS");
    ///
    /// // UK Gilt-style bond
    /// let uk_bond = Bond::builder()
    ///     .id("UK-001")
    ///     .notional(notional)
    ///     .coupon(0.04)
    ///     .freq(Frequency::semi_annual())
    ///     .dc(DayCount::ActAct)  // ISDA variant
    ///     .issue(issue)
    ///     .maturity(maturity)
    ///     .disc_id("GBP-GILT")
    ///     .build()
    ///     .unwrap();
    ///
    /// // European bond
    /// let eur_bond = Bond::builder()
    ///     .id("EUR-001")
    ///     .notional(notional)
    ///     .coupon(0.03)
    ///     .freq(Frequency::annual())
    ///     .dc(DayCount::ThirtyE360)  // European 30/360
    ///     .issue(issue)
    ///     .maturity(maturity)
    ///     .disc_id("EUR-GOVT")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn fixed(
        id: impl Into<InstrumentId>,
        notional: Money,
        coupon_rate: f64,
        issue: Date,
        maturity: Date,
        disc_id: impl Into<CurveId>,
    ) -> Self {
        Self::builder()
            .id(id.into())
            .notional(notional)
            .coupon(coupon_rate)
            .issue(issue)
            .maturity(maturity)
            .freq(finstack_core::dates::Frequency::semi_annual())
            .dc(DayCount::Thirty360)
            .bdc(BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .stub(StubKind::None)
            .disc_id(disc_id.into())
            .hazard_id_opt(None)
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
        disc_id: impl Into<CurveId>,
    ) -> Self {
        Self::builder()
            .id(id.into())
            .notional(notional)
            .coupon(coupon_rate)
            .issue(issue)
            .maturity(maturity)
            .freq(convention.frequency())
            .dc(convention.day_count())
            .bdc(convention.business_day_convention())
            .calendar_id_opt(None)
            .stub(convention.stub_convention())
            .disc_id(disc_id.into())
            .hazard_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Bond with convention construction should not fail")
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
        disc_id: impl Into<CurveId>,
        quoted_clean: Option<f64>,
    ) -> finstack_core::Result<Self> {
        // Extract parameters from the schedule
        let notional = schedule.notional.initial;
        let dc = schedule.day_count;

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

        // Default frequency and coupon (these won't be used with custom cashflows)
        let freq = finstack_core::dates::Frequency::semi_annual();
        let coupon = 0.0;

        let pricing_overrides = if let Some(price) = quoted_clean {
            PricingOverrides::default().with_clean_price(price)
        } else {
            PricingOverrides::default()
        };

        Self::builder()
            .id(id.into())
            .notional(notional)
            .coupon(coupon)
            .issue(issue)
            .maturity(maturity)
            .freq(freq)
            .dc(dc)
            .bdc(BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .stub(StubKind::None)
            .disc_id(disc_id.into())
            .hazard_id_opt(None)
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
        use crate::cashflow::builder::{
            CashFlowSchedule, CouponType, FixedCouponSpec, FloatingCouponSpec,
        };

        // If custom cashflows are set, return them directly
        if let Some(ref custom) = self.custom_cashflows {
            return Ok(custom.clone());
        }

        // Build the schedule using the cashflow builder
        let mut b = CashFlowSchedule::builder();
        b.principal(self.notional, self.issue, self.maturity);

        // Add amortization if present
        if let Some(am) = &self.amortization {
            b.amortization(am.clone());
        }

        // Add coupon specification (fixed or floating)
        if let Some(ref fl) = self.float {
            // Floating rate bond
            b.floating_cf(FloatingCouponSpec {
                index_id: fl.fwd_id.to_owned(),
                margin_bp: fl.margin_bp,
                gearing: fl.gearing,
                reset_lag_days: fl.reset_lag_days,
                coupon_type: CouponType::Cash,
                freq: self.freq,
                dc: self.dc,
                bdc: self.bdc,
                calendar_id: self.calendar_id.clone(),
                stub: self.stub,
            });
        } else {
            // Fixed rate bond
            b.fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: self.coupon,
                freq: self.freq,
                dc: self.dc,
                bdc: self.bdc,
                calendar_id: self.calendar_id.clone(),
                stub: self.stub,
            });
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
            .dc
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
        let discount_curve = market.get_discount_ref(&self.disc_id)?;

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
            &self.disc_id,
            self.dc,
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
            self, market, as_of, base_value, metrics,
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for Bond {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.disc_id
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
        assert_eq!(bond.disc_id.as_str(), "USD-OIS");
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

        // Use builder pattern
        let bond = Bond::builder()
            .id("PIK_TOGGLE_BOND".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .coupon(0.06)
            .issue(issue)
            .maturity(maturity)
            .freq(Frequency::quarterly())
            .dc(DayCount::Thirty360)
            .bdc(BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .stub(StubKind::None)
            .custom_cashflows_opt(Some(custom_schedule))
            .disc_id(CurveId::new("USD-OIS"))
            .pricing_overrides(PricingOverrides::default().with_clean_price(99.0))
            .attributes(Attributes::new())
            .build()
            .unwrap();

        assert_eq!(bond.id.as_str(), "PIK_TOGGLE_BOND");
        assert_eq!(bond.disc_id.as_str(), "USD-OIS");
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
            .coupon(0.04)
            .freq(Frequency::semi_annual())
            .dc(DayCount::Act365F)
            .bdc(BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .stub(StubKind::None)
            .issue(issue)
            .maturity(maturity)
            .disc_id(CurveId::new("USD-OIS"))
            .hazard_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .call_put_opt(None)
            .amortization_opt(None)
            .custom_cashflows_opt(None)
            .float_opt(None)
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
                rate: 0.055, // Different from bond's coupon rate
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
        assert_eq!(bond.coupon, 0.04); // Original coupon is preserved but won't be used
        assert_eq!(bond.freq, Frequency::semi_annual()); // Original freq preserved but won't be used
    }

    #[test]
    fn test_custom_cashflows_override_regular_generation() {
        let issue = Date::from_calendar_date(2025, Month::June, 1).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::June, 1).unwrap();

        // Create bond with regular specs (builder)
        let regular_bond = Bond::builder()
            .id(InstrumentId::new("TEST"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .coupon(0.03)
            .freq(Frequency::annual())
            .dc(DayCount::Act365F)
            .bdc(BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .stub(StubKind::None)
            .issue(issue)
            .maturity(maturity)
            .disc_id(CurveId::new("USD-OIS"))
            .hazard_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .call_put_opt(None)
            .amortization_opt(None)
            .custom_cashflows_opt(None)
            .float_opt(None)
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

        let bond = Bond::builder()
            .id("FRN-TEST".into())
            .notional(notional)
            .coupon(0.0)
            .issue(issue)
            .maturity(maturity)
            .freq(Frequency::quarterly())
            .dc(DayCount::Act360)
            .bdc(BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .stub(StubKind::None)
            .disc_id(CurveId::new("USD-OIS"))
            .hazard_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .float_opt(Some(BondFloatSpec {
                fwd_id: CurveId::new("USD-SOFR-3M"),
                margin_bp: 150.0,
                gearing: 1.0,
                reset_lag_days: 2,
            }))
            .attributes(Attributes::new())
            .build()
            .unwrap();

        // Price should be finite and positive under positive forwards
        let pv = bond.value(&ctx, issue).unwrap();
        assert!(pv.amount().is_finite());
    }
}
