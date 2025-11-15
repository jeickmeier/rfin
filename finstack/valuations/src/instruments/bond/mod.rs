//! Fixed and floating-rate bond instruments with embedded options.
//!
//! Provides comprehensive bond modeling including:
//! - Fixed-rate coupon bonds (bullet and amortizing)
//! - Floating-rate notes (FRNs) with caps/floors
//! - Callable and putable bonds (American/Bermudan exercise)
//! - Zero-coupon bonds
//! - Custom cashflow schedules
//!
//! # Bond Pricing
//!
//! Bonds are priced by discounting all future cashflows:
//!
//! ```text
//! PV = Σ CF_i · DF(t_i)
//! ```
//!
//! For bonds with embedded options (calls/puts), tree-based pricing is used
//! to value the optionality.
//!
//! # Regional Market Conventions
//!
//! Different bond markets follow distinct conventions:
//!
//! - **US Treasuries**: 30/360, Semi-annual, T+1 settlement
//! - **UK Gilts**: ACT/ACT, Semi-annual, T+1 settlement  
//! - **Eurozone**: 30E/360 or ACT/ACT, Annual, T+2 settlement
//! - **Japan**: ACT/365F, Semi-annual, T+3 settlement
//!
//! Use `Bond::with_convention()` for standard regional conventions.
//!
//! # Key Metrics
//!
//! - **Yield to Maturity (YTM)**: Internal rate of return
//! - **Modified Duration**: Interest rate sensitivity
//! - **Convexity**: Curvature of price/yield relationship
//! - **DV01**: Dollar value of 1bp rate change
//! - **Z-spread**: Spread over benchmark curve
//! - **Accrued Interest**: Interest accrued since last coupon
//!
//! # Examples
//!
//! See [`Bond`] for construction examples.
//!
//! # See Also
//!
//! - [`Bond`] for the main bond struct and factory methods
//! - [`CallPutSchedule`] for embedded option schedules
//! - [`CashflowSpec`] for fixed/floating/amortizing specifications
//! - [`AmortizationSpec`] for amortizing bonds
//! - [`metrics`] for bond-specific risk metrics

pub mod cashflow_spec;
pub mod cashflows;
pub mod metrics;
pub mod pricing;
mod types;

pub use cashflow_spec::CashflowSpec;
pub use types::AmortizationSpec;
pub use types::Bond;
pub use types::CallPut;
pub use types::CallPutSchedule;

#[cfg(test)]
mod tests {
    use crate::instruments::bond::{Bond, CashflowSpec};
    use crate::instruments::common::parameters::BondConvention;
    use crate::instruments::common::traits::{Attributes, Instrument};
    use crate::instruments::PricingOverrides;
    use crate::pricer::InstrumentType;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
    use finstack_core::money::Money;
    use finstack_core::types::CurveId;
    use time::macros::date;

    #[test]
    fn test_bond_builder_minimal() {
        let bond = Bond::builder()
            .id("BOND_MIN".into())
            .notional(Money::new(1000.0, Currency::USD))
            .issue(date!(2025 - 01 - 01))
            .maturity(date!(2030 - 01 - 01))
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                Frequency::semi_annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build();

        assert!(bond.is_ok());
        let bond = bond.expect("should succeed");
        assert_eq!(bond.id.as_str(), "BOND_MIN");
        assert_eq!(bond.notional.amount(), 1000.0);
        assert_eq!(bond.discount_curve_id.as_str(), "USD-OIS");
    }

    #[test]
    fn test_bond_fixed_factory() {
        let bond = Bond::fixed(
            "BOND_FIXED",
            Money::new(100.0, Currency::USD),
            0.04,
            date!(2025 - 01 - 01),
            date!(2030 - 01 - 01),
            "USD-TREASURY",
        );

        assert_eq!(bond.id.as_str(), "BOND_FIXED");
        assert_eq!(bond.cashflow_spec.frequency(), Frequency::semi_annual());
        assert_eq!(bond.cashflow_spec.day_count(), DayCount::Thirty360);
        assert_eq!(bond.discount_curve_id.as_str(), "USD-TREASURY");
    }

    #[test]
    fn test_bond_with_convention_us_treasury() {
        let bond = Bond::with_convention(
            "UST-10Y",
            Money::new(1000.0, Currency::USD),
            0.03,
            date!(2025 - 01 - 01),
            date!(2035 - 01 - 01),
            BondConvention::USTreasury,
            "USD-TREASURY",
        );

        assert_eq!(bond.id.as_str(), "UST-10Y");
        assert_eq!(
            bond.cashflow_spec.frequency(),
            BondConvention::USTreasury.frequency()
        );
        assert_eq!(
            bond.cashflow_spec.day_count(),
            BondConvention::USTreasury.day_count()
        );
    }

    #[test]
    fn test_bond_with_convention_uk_gilt() {
        let bond = Bond::with_convention(
            "GILT-10Y",
            Money::new(1000.0, Currency::GBP),
            0.025,
            date!(2025 - 01 - 01),
            date!(2035 - 01 - 01),
            BondConvention::UKGilt,
            "GBP-GILTS",
        );

        assert_eq!(
            bond.cashflow_spec.frequency(),
            BondConvention::UKGilt.frequency()
        );
        assert_eq!(
            bond.cashflow_spec.day_count(),
            BondConvention::UKGilt.day_count()
        );
    }

    #[test]
    fn test_bond_with_pricing_overrides() {
        let overrides = PricingOverrides::default()
            .with_clean_price(98.5)
            .with_ytm_bump_decimal(1e-4);

        let bond = Bond::builder()
            .id("BOND_OVERRIDE".into())
            .notional(Money::new(1000.0, Currency::USD))
            .issue(date!(2025 - 01 - 01))
            .maturity(date!(2030 - 01 - 01))
            .cashflow_spec(CashflowSpec::fixed(
                0.06,
                Frequency::annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(overrides)
            .attributes(Attributes::new())
            .build()
            .expect("should succeed");

        assert_eq!(bond.pricing_overrides.quoted_clean_price, Some(98.5));
        assert_eq!(bond.pricing_overrides.ytm_bump_decimal, Some(1e-4));
    }

    #[test]
    fn test_bond_with_settlement_convention() {
        let bond = Bond::builder()
            .id("BOND_SETTLE".into())
            .notional(Money::new(1000.0, Currency::USD))
            .issue(date!(2025 - 01 - 01))
            .maturity(date!(2030 - 01 - 01))
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                Frequency::semi_annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(PricingOverrides::default())
            .settlement_days_opt(Some(2))
            .ex_coupon_days_opt(Some(7))
            .attributes(Attributes::new())
            .build()
            .expect("should succeed");

        assert_eq!(bond.settlement_days, Some(2));
        assert_eq!(bond.ex_coupon_days, Some(7));
    }

    #[test]
    fn test_bond_with_attributes() {
        let mut attrs = Attributes::new();
        attrs
            .meta
            .insert("sector".to_string(), "corporate".to_string());
        attrs.meta.insert("rating".to_string(), "AA".to_string());

        let bond = Bond::builder()
            .id("BOND_ATTRS".into())
            .notional(Money::new(1000.0, Currency::USD))
            .issue(date!(2025 - 01 - 01))
            .maturity(date!(2030 - 01 - 01))
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                Frequency::semi_annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(PricingOverrides::default())
            .attributes(attrs)
            .build()
            .expect("should succeed");

        assert_eq!(
            bond.attributes.meta.get("sector"),
            Some(&"corporate".to_string())
        );
        assert_eq!(bond.attributes.meta.get("rating"), Some(&"AA".to_string()));
    }

    #[test]
    fn test_bond_zero_coupon() {
        let bond = Bond::builder()
            .id("ZERO_COUPON".into())
            .notional(Money::new(1000.0, Currency::USD))
            .issue(date!(2025 - 01 - 01))
            .maturity(date!(2030 - 01 - 01))
            .cashflow_spec(CashflowSpec::fixed(
                0.0,
                Frequency::annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("should succeed");

        // Zero coupon bond
        if let CashflowSpec::Fixed(spec) = &bond.cashflow_spec {
            assert_eq!(spec.rate, 0.0);
        } else {
            panic!("Expected Fixed cashflow spec");
        }
    }

    #[test]
    fn test_bond_high_frequency() {
        let bond = Bond::builder()
            .id("MONTHLY".into())
            .notional(Money::new(1000.0, Currency::USD))
            .issue(date!(2025 - 01 - 01))
            .maturity(date!(2027 - 01 - 01))
            .cashflow_spec(CashflowSpec::fixed(
                0.06,
                Frequency::monthly(),
                DayCount::Act360,
            ))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("should succeed");

        assert_eq!(bond.cashflow_spec.frequency(), Frequency::monthly());
    }

    #[test]
    fn test_bond_with_calendar() {
        use crate::cashflow::builder::specs::{CouponType, FixedCouponSpec};

        let spec = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.05,
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("USGS".to_string()),
            stub: StubKind::None,
        };

        let bond = Bond::builder()
            .id("BOND_CAL".into())
            .notional(Money::new(1000.0, Currency::USD))
            .issue(date!(2025 - 01 - 01))
            .maturity(date!(2030 - 01 - 01))
            .cashflow_spec(CashflowSpec::Fixed(spec.clone()))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("should succeed");

        if let CashflowSpec::Fixed(s) = &bond.cashflow_spec {
            assert_eq!(s.calendar_id, Some("USGS".to_string()));
            assert_eq!(s.bdc, BusinessDayConvention::ModifiedFollowing);
        } else {
            panic!("Expected Fixed cashflow spec");
        }
    }

    #[test]
    fn test_bond_stub_conventions() {
        use crate::cashflow::builder::specs::{CouponType, FixedCouponSpec};

        // Short front stub
        let spec_short = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.05,
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::ShortFront,
        };

        let bond_short_front = Bond::builder()
            .id("STUB_SHORT_FRONT".into())
            .notional(Money::new(1000.0, Currency::USD))
            .issue(date!(2025 - 01 - 15))
            .maturity(date!(2030 - 01 - 01))
            .cashflow_spec(CashflowSpec::Fixed(spec_short.clone()))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("should succeed");

        if let CashflowSpec::Fixed(s) = &bond_short_front.cashflow_spec {
            assert_eq!(s.stub, StubKind::ShortFront);
        }

        // Long back stub
        let spec_long = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.05,
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::LongBack,
        };

        let bond_long_back = Bond::builder()
            .id("STUB_LONG_BACK".into())
            .notional(Money::new(1000.0, Currency::USD))
            .issue(date!(2025 - 01 - 01))
            .maturity(date!(2030 - 02 - 15))
            .cashflow_spec(CashflowSpec::Fixed(spec_long.clone()))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("should succeed");

        if let CashflowSpec::Fixed(s) = &bond_long_back.cashflow_spec {
            assert_eq!(s.stub, StubKind::LongBack);
        }
    }

    #[test]
    fn test_bond_different_currencies() {
        let currencies = vec![
            (Currency::USD, "USD"),
            (Currency::EUR, "EUR"),
            (Currency::GBP, "GBP"),
            (Currency::JPY, "JPY"),
            (Currency::CHF, "CHF"),
        ];

        for (ccy, code) in currencies {
            let bond = Bond::builder()
                .id(format!("BOND_{}", code).into())
                .notional(Money::new(1000.0, ccy))
                .issue(date!(2025 - 01 - 01))
                .maturity(date!(2030 - 01 - 01))
                .cashflow_spec(CashflowSpec::fixed(
                    0.04,
                    Frequency::annual(),
                    DayCount::Act365F,
                ))
                .discount_curve_id(CurveId::new(format!("{}-OIS", code)))
                .pricing_overrides(PricingOverrides::default())
                .attributes(Attributes::new())
                .build()
                .expect("should succeed");

            assert_eq!(bond.notional.currency(), ccy);
        }
    }

    #[test]
    fn test_bond_instrument_trait() {
        let bond = Bond::fixed(
            "TRAIT_TEST",
            Money::new(1000.0, Currency::USD),
            0.05,
            date!(2025 - 01 - 01),
            date!(2030 - 01 - 01),
            "USD-OIS",
        );

        let inst: &dyn Instrument = &bond;
        assert_eq!(inst.id(), "TRAIT_TEST");
        assert_eq!(inst.key(), InstrumentType::Bond);
        assert!(inst.as_any().is::<Bond>());
    }

    #[test]
    fn test_bond_clone_and_equality() {
        let bond1 = Bond::fixed(
            "CLONE_TEST",
            Money::new(1000.0, Currency::USD),
            0.05,
            date!(2025 - 01 - 01),
            date!(2030 - 01 - 01),
            "USD-OIS",
        );

        let bond2 = bond1.clone();

        assert_eq!(bond1.id.as_str(), bond2.id.as_str());
        assert_eq!(bond1.notional.amount(), bond2.notional.amount());
        assert_eq!(bond1.maturity, bond2.maturity);
    }

    #[test]
    fn test_bond_near_maturity() {
        let issue = date!(2025 - 01 - 01);
        let maturity = date!(2025 - 02 - 01); // 1 month

        let bond = Bond::fixed(
            "SHORT_TERM",
            Money::new(1000.0, Currency::USD),
            0.03,
            issue,
            maturity,
            "USD-OIS",
        );

        assert!(bond.maturity > bond.issue);
        let days_to_maturity = (bond.maturity - bond.issue).whole_days();
        assert!(days_to_maturity < 365);
    }

    #[test]
    fn test_bond_long_maturity() {
        let issue = date!(2025 - 01 - 01);
        let maturity = date!(2055 - 01 - 01); // 30 years

        let bond = Bond::fixed(
            "LONG_TERM",
            Money::new(1000.0, Currency::USD),
            0.045,
            issue,
            maturity,
            "USD-OIS",
        );

        let years_to_maturity = (bond.maturity - bond.issue).whole_days() / 365;
        assert!(years_to_maturity >= 30);
    }

    #[test]
    fn test_bond_premium_discount_par() {
        // Premium bond (price > 100)
        let premium = Bond::builder()
            .id("PREMIUM".into())
            .notional(Money::new(100.0, Currency::USD))
            .issue(date!(2025 - 01 - 01))
            .maturity(date!(2030 - 01 - 01))
            .cashflow_spec(CashflowSpec::fixed(
                0.08,
                Frequency::semi_annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(PricingOverrides::default().with_clean_price(105.0))
            .attributes(Attributes::new())
            .build()
            .expect("should succeed");

        assert_eq!(premium.pricing_overrides.quoted_clean_price, Some(105.0));

        // Discount bond (price < 100)
        let discount = Bond::builder()
            .id("DISCOUNT".into())
            .notional(Money::new(100.0, Currency::USD))
            .issue(date!(2025 - 01 - 01))
            .maturity(date!(2030 - 01 - 01))
            .cashflow_spec(CashflowSpec::fixed(
                0.03,
                Frequency::semi_annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(PricingOverrides::default().with_clean_price(95.0))
            .attributes(Attributes::new())
            .build()
            .expect("should succeed");

        assert_eq!(discount.pricing_overrides.quoted_clean_price, Some(95.0));

        // Par bond (price = 100)
        let par = Bond::builder()
            .id("PAR".into())
            .notional(Money::new(100.0, Currency::USD))
            .issue(date!(2025 - 01 - 01))
            .maturity(date!(2030 - 01 - 01))
            .cashflow_spec(CashflowSpec::fixed(
                0.05,
                Frequency::semi_annual(),
                DayCount::Act365F,
            ))
            .discount_curve_id("USD-OIS".into())
            .pricing_overrides(PricingOverrides::default().with_clean_price(100.0))
            .attributes(Attributes::new())
            .build()
            .expect("should succeed");

        assert_eq!(par.pricing_overrides.quoted_clean_price, Some(100.0));
    }
}
