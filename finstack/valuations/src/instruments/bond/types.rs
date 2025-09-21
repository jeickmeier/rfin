//! Bond instrument types and implementations.

use finstack_core::dates::{BusinessDayConvention, StubKind};
use finstack_core::prelude::*;
use finstack_core::F;

use crate::cashflow::builder::CashFlowSchedule;
#[allow(unused_imports)]
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::types::{CurveId, InstrumentId};

// Re-export for compatibility in tests and external users referencing bond::AmortizationSpec
pub use crate::cashflow::primitives::AmortizationSpec;

/// Fixed-rate bond instrument with optional features.
///
/// Supports call/put schedules, amortization, quoted prices for
/// yield-to-maturity calculations, and custom cashflow schedules.
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct Bond {
    /// Unique identifier for the bond.
    pub id: InstrumentId,
    /// Principal amount of the bond.
    pub notional: Money,
    /// Annual coupon rate (e.g., 0.05 for 5%).
    pub coupon: F,
    /// Coupon payment frequency.
    pub freq: finstack_core::dates::Frequency,
    /// Day count convention for accrual.
    pub dc: DayCount,
    /// Business day convention for schedule/payment adjustments.
    pub bdc: BusinessDayConvention,
    /// Optional calendar identifier for schedule adjustments.
    pub calendar_id: Option<&'static str>,
    /// Stub handling rule for the schedule.
    pub stub: StubKind,
    /// Issue date of the bond.
    pub issue: Date,
    /// Maturity date of the bond.
    pub maturity: Date,
    /// Discount curve identifier for pricing.
    pub disc_id: CurveId,
    /// Pricing overrides (including quoted clean price)
    pub pricing_overrides: PricingOverrides,
    /// Optional call/put schedule (dates and redemption prices as % of par amount).
    pub call_put: Option<CallPutSchedule>,
    /// Optional amortization specification (principal paid during life).
    pub amortization: Option<AmortizationSpec>,
    /// Optional pre-built cashflow schedule. If provided, this will be used instead of
    /// generating cashflows from coupon/amortization specifications.
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
pub struct CallPut {
    /// Exercise date of the option.
    pub date: Date,
    /// Redemption price as percentage of par amount.
    pub price_pct_of_par: F,
}

/// Schedule of call and put options for a bond.
#[derive(Clone, Debug, Default)]
pub struct CallPutSchedule {
    /// Call options (issuer can redeem early).
    pub calls: Vec<CallPut>,
    /// Put options (holder can redeem early).
    pub puts: Vec<CallPut>,
}

/// Floating-rate parameters for FRN-style bonds.
#[derive(Clone, Debug)]
pub struct BondFloatSpec {
    /// Forward curve identifier for the floating index (e.g., USD-SOFR-3M).
    pub fwd_id: CurveId,
    /// Margin over the index in basis points.
    pub margin_bp: F,
    /// Gearing multiplier on the index rate.
    pub gearing: F,
    /// Reset lag in days applied to the fixing date (business-day adjusted Following).
    pub reset_lag_days: i32,
}

impl Bond {
    /// Create a standard fixed-rate bond with semi-annual coupons.
    pub fn fixed_semiannual(
        id: impl Into<String>,
        notional: Money,
        coupon_rate: F,
        issue: Date,
        maturity: Date,
        disc_id: impl Into<CurveId>,
    ) -> Self {
        Self::builder()
            .id(id.into().into())
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
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Standard bond construction should not fail")
    }

    /// Create a standard Treasury bond with ActAct day count.
    pub fn treasury(
        id: impl Into<String>,
        notional: Money,
        coupon_rate: F,
        issue: Date,
        maturity: Date,
    ) -> Self {
        Self::builder()
            .id(id.into().into())
            .notional(notional)
            .coupon(coupon_rate)
            .issue(issue)
            .maturity(maturity)
            .freq(finstack_core::dates::Frequency::annual())
            .dc(DayCount::ActActIsma)
            .bdc(BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .stub(StubKind::None)
            .disc_id(CurveId::new("USD-TREASURY"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Treasury bond construction should not fail")
    }

    /// Create a zero-coupon bond.
    pub fn zero_coupon(
        id: impl Into<String>,
        notional: Money,
        issue: Date,
        maturity: Date,
        disc_id: impl Into<CurveId>,
    ) -> Self {
        Self::fixed_semiannual(id, notional, 0.0, issue, maturity, disc_id)
    }

    /// Create a simple floating-rate note (FRN) with typical conventions.
    ///
    /// Defaults:
    /// - Frequency: quarterly; DayCount: Act/360; Reset lag: 2 days; Gearing: 1.0
    pub fn floating(
        id: impl Into<String>,
        notional: Money,
        issue: Date,
        maturity: Date,
        disc_id: impl Into<CurveId>,
        fwd_id: impl Into<CurveId>,
        margin_bp: F,
    ) -> Self {
        Self {
            id: InstrumentId::new(id.into()),
            notional,
            coupon: 0.0,
            freq: finstack_core::dates::Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
            issue,
            maturity,
            disc_id: disc_id.into(),
            pricing_overrides: PricingOverrides::default(),
            call_put: None,
            amortization: None,
            custom_cashflows: None,
            float: Some(BondFloatSpec {
                fwd_id: fwd_id.into(),
                margin_bp,
                gearing: 1.0,
                reset_lag_days: 2,
            }),
            attributes: Attributes::new(),
            settlement_days: None,
            ex_coupon_days: None,
        }
    }

    /// Create a bond from a pre-built cashflow schedule.
    ///
    /// This extracts key bond parameters from the cashflow schedule and creates
    /// a bond that will use these custom cashflows for all calculations.
    pub fn from_cashflows(
        id: impl Into<String>,
        schedule: CashFlowSchedule,
        disc_id: impl Into<CurveId>,
        quoted_clean: Option<F>,
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
        let maturity = *dates.last().unwrap();

        // Default frequency and coupon (these won't be used with custom cashflows)
        let freq = finstack_core::dates::Frequency::semi_annual();
        let coupon = 0.0;

        Ok(Self {
            id: InstrumentId::new(id.into()),
            notional,
            coupon,
            freq,
            dc,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
            issue,
            maturity,
            disc_id: disc_id.into(),
            pricing_overrides: if let Some(price) = quoted_clean {
                PricingOverrides::default().with_clean_price(price)
            } else {
                PricingOverrides::default()
            },
            call_put: None,
            amortization: None,
            custom_cashflows: Some(schedule),
            float: None,
            attributes: Attributes::new(),
            settlement_days: None,
            ex_coupon_days: None,
        })
    }

    /// Set custom cashflows for this bond.
    ///
    /// When custom cashflows are set, they will be used instead of generating
    /// cashflows from the bond's coupon and amortization specifications.
    pub fn with_cashflows(mut self, schedule: CashFlowSchedule) -> Self {
        self.custom_cashflows = Some(schedule);
        self
    }
}

// Attributable and Instrument impls without tying pricing here
impl crate::instruments::common::traits::Attributable for Bond {
    #[inline]
    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    #[inline]
    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }
}

impl crate::instruments::common::traits::Instrument for Bond {
    #[inline]
    fn id(&self) -> &str {
        self.id.as_str()
    }

    #[inline]
    fn instrument_type(&self) -> &'static str {
        "Bond"
    }

    #[inline]
    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    #[inline]
    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    #[inline]
    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    #[inline]
    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cashflow::builder::{cf, CouponType, FixedCouponSpec, ScheduleParams};
    use crate::instruments::common::traits::Priceable;
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

        let custom_schedule = cf()
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
        let custom_schedule = cf()
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

        // Create a traditional bond first
        let mut bond = Bond {
            id: InstrumentId::new("REGULAR_BOND"),
            notional: Money::new(1_000_000.0, Currency::USD),
            coupon: 0.04,
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
            issue,
            maturity,
            disc_id: CurveId::new("USD-OIS"),
            pricing_overrides: PricingOverrides::default(),
            call_put: None,
            amortization: None,
            custom_cashflows: None,
            float: None,
            attributes: Attributes::new(),
            settlement_days: None,
            ex_coupon_days: None,
        };

        // Build a custom schedule separately
        let custom_schedule = cf()
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

        // Create bond with regular specs
        let regular_bond = Bond {
            id: InstrumentId::new("TEST"),
            notional: Money::new(1_000_000.0, Currency::USD),
            coupon: 0.03,
            freq: Frequency::annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
            issue,
            maturity,
            disc_id: CurveId::new("USD-OIS"),
            pricing_overrides: PricingOverrides::default(),
            call_put: None,
            amortization: None,
            custom_cashflows: None,
            float: None,
            attributes: Attributes::new(),
            settlement_days: None,
            ex_coupon_days: None,
        };

        // Same bond with custom cashflows
        let custom_schedule = cf()
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
            issue,
            maturity,
            CurveId::new("USD-OIS"),
            CurveId::new("USD-SOFR-3M"),
            150.0,
        );

        // Price should be finite and positive under positive forwards
        let pv = bond.value(&ctx, issue).unwrap();
        assert!(pv.amount().is_finite());
    }
}
