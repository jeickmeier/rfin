//! Tests for emission functions.

#[cfg(test)]
mod credit_emission_tests {
    use super::super::super::specs::DefaultEvent;
    use super::super::credit::{emit_default_on, emit_prepayment_on};
    use crate::cashflow::primitives::CFKind;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use time::Month;

    #[test]
    fn test_default_and_recovery_mechanics() {
        let d = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        let event = DefaultEvent {
            default_date: d,
            defaulted_amount: 400_000.0,
            recovery_rate: 0.40,
            recovery_lag: 12,
        };

        let mut outstanding = 1_000_000.0;
        let flows = emit_default_on(d, &[event], &mut outstanding, Currency::USD).unwrap();

        // Net loss: 400K × (1 - 0.40) = 240K
        // Outstanding: 1M - 400K + 160K = 760K
        assert_eq!(outstanding, 760_000.0);
        assert_eq!(flows.len(), 2);

        // First flow: default
        assert_eq!(flows[0].kind, CFKind::DefaultedNotional);
        assert_eq!(flows[0].amount.amount(), 400_000.0);
        assert_eq!(flows[0].date, d);

        // Second flow: recovery
        assert_eq!(flows[1].kind, CFKind::Recovery);
        assert_eq!(flows[1].amount.amount(), 160_000.0);
        let expected_recovery_date = finstack_core::dates::utils::add_months(d, 12);
        assert_eq!(flows[1].date, expected_recovery_date);
    }

    #[test]
    fn test_coupon_on_reduced_outstanding_after_default() {
        // CRITICAL TEST: Verify coupon uses reduced outstanding after default
        use super::super::super::specs::{CouponType, FixedCouponSpec};
        use super::super::coupons::emit_fixed_coupons_on;
        use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};

        let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let mat = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let default_date = Date::from_calendar_date(2025, Month::July, 1).unwrap();
        let coupon_date = Date::from_calendar_date(2025, Month::October, 1).unwrap();

        // Setup: 1M notional, 5% coupon, quarterly payments
        let spec = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.05,
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        // Generate coupon dates
        let period_schedule = super::super::super::date_generation::build_dates(
            issue,
            mat,
            spec.freq,
            spec.stub,
            spec.bdc,
            spec.calendar_id.as_deref(),
        );

        let mut outstanding_after = hashbrown::HashMap::new();
        outstanding_after.insert(issue, 1_000_000.0);

        // Apply default on July 1: 400K defaults, 40% recovery
        let default_event = DefaultEvent {
            default_date,
            defaulted_amount: 400_000.0,
            recovery_rate: 0.40,
            recovery_lag: 12,
        };

        let mut outstanding = 1_000_000.0;
        let _ = emit_default_on(
            default_date,
            &[default_event],
            &mut outstanding,
            Currency::USD,
        )
        .unwrap();

        // Outstanding now 760K (1M - 400K + 160K recovery)
        outstanding_after.insert(default_date, outstanding);

        // Generate coupon on Oct 1 using reduced outstanding
        let schedule = (
            spec,
            period_schedule.dates.clone(),
            period_schedule.prev.clone(),
            period_schedule.first_or_last.clone(),
        );

        let (pik, coupons) = emit_fixed_coupons_on(
            coupon_date,
            &[schedule],
            &outstanding_after,
            1_000_000.0,
            Currency::USD,
        )
        .unwrap();

        assert_eq!(pik, 0.0);
        assert_eq!(coupons.len(), 1);

        // Coupon should be on 760K, not 1M
        // Year fraction for Jul 1 - Oct 1 = 92 days
        let yf = 92.0 / 360.0;
        let expected_coupon = 760_000.0 * 0.05 * yf;

        assert!(
            (coupons[0].amount.amount() - expected_coupon).abs() < 1.0,
            "Expected coupon ~{}, got {}",
            expected_coupon,
            coupons[0].amount.amount()
        );
    }

    #[test]
    fn test_prepayment_reduces_outstanding() {
        let d = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        let mut outstanding = 1_000_000.0;

        let flows = emit_prepayment_on(d, 50_000.0, &mut outstanding, Currency::USD);

        assert_eq!(outstanding, 950_000.0);
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].kind, CFKind::PrePayment);
        assert_eq!(flows[0].amount.amount(), 50_000.0);
    }

    #[test]
    fn test_prepayment_capped_by_outstanding() {
        let d = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        let mut outstanding = 30_000.0;

        let flows = emit_prepayment_on(d, 50_000.0, &mut outstanding, Currency::USD);

        // Can only prepay what's outstanding
        assert_eq!(outstanding, 0.0);
        assert_eq!(flows[0].amount.amount(), 30_000.0);
    }

    #[test]
    fn test_zero_recovery_rate() {
        let d = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        let event = DefaultEvent {
            default_date: d,
            defaulted_amount: 100_000.0,
            recovery_rate: 0.0, // Total loss
            recovery_lag: 12,
        };

        let mut outstanding = 1_000_000.0;
        let flows = emit_default_on(d, &[event], &mut outstanding, Currency::USD).unwrap();

        // Net loss is 100% of defaulted amount
        assert_eq!(outstanding, 900_000.0);
        assert_eq!(flows.len(), 1); // Only default, no recovery
    }

    #[test]
    fn test_full_recovery_rate() {
        let d = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        let event = DefaultEvent {
            default_date: d,
            defaulted_amount: 100_000.0,
            recovery_rate: 1.0, // Full recovery
            recovery_lag: 12,
        };

        let mut outstanding = 1_000_000.0;
        let flows = emit_default_on(d, &[event], &mut outstanding, Currency::USD).unwrap();

        // Net loss is zero
        assert_eq!(outstanding, 1_000_000.0);
        assert_eq!(flows.len(), 2); // Default + full recovery
    }

    #[test]
    fn test_multiple_defaults_same_date() {
        let d = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        let events = vec![
            DefaultEvent {
                default_date: d,
                defaulted_amount: 50_000.0,
                recovery_rate: 0.40,
                recovery_lag: 12,
            },
            DefaultEvent {
                default_date: d,
                defaulted_amount: 30_000.0,
                recovery_rate: 0.50,
                recovery_lag: 6,
            },
        ];

        let mut outstanding = 1_000_000.0;
        let flows = emit_default_on(d, &events, &mut outstanding, Currency::USD).unwrap();

        // Net loss: 50K × 0.6 + 30K × 0.5 = 30K + 15K = 45K
        assert_eq!(
            outstanding,
            1_000_000.0 - 50_000.0 + 20_000.0 - 30_000.0 + 15_000.0
        );
        assert_eq!(flows.len(), 4); // 2 defaults + 2 recoveries
    }

    #[test]
    fn test_non_matching_dates_return_empty() {
        let d = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        let other_date = Date::from_calendar_date(2025, Month::April, 1).unwrap();

        let event = DefaultEvent {
            default_date: other_date,
            defaulted_amount: 100_000.0,
            recovery_rate: 0.40,
            recovery_lag: 12,
        };

        let mut outstanding = 1_000_000.0;
        let flows = emit_default_on(d, &[event], &mut outstanding, Currency::USD).unwrap();

        assert_eq!(outstanding, 1_000_000.0); // Unchanged
        assert_eq!(flows.len(), 0);
    }

    #[test]
    fn test_recovery_lag_calculation() {
        let d = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let event = DefaultEvent {
            default_date: d,
            defaulted_amount: 100_000.0,
            recovery_rate: 0.40,
            recovery_lag: 6,
        };

        let mut outstanding = 1_000_000.0;
        let flows = emit_default_on(d, &[event], &mut outstanding, Currency::USD).unwrap();

        let expected_recovery_date = finstack_core::dates::utils::add_months(d, 6);
        assert_eq!(flows[1].date, expected_recovery_date);
    }

    #[test]
    fn test_prepayment_model_psa_curve() {
        use super::super::super::credit_rates::monthly_to_annual;
        use super::super::super::specs::PrepaymentModelSpec;

        let model = PrepaymentModelSpec::psa(1.5); // 150% PSA

        // Month 15: should be 4.5% CPR (halfway to 9%)
        let smm = model.smm(15);
        assert!(smm > 0.0);
        let cpr = monthly_to_annual(smm);
        assert!((cpr - 0.045).abs() < 0.001);

        // Month 30: should be 9% CPR = ~0.77% SMM
        let smm = model.smm(30);
        let cpr = monthly_to_annual(smm);
        assert!((cpr - 0.09).abs() < 0.001);

        // Month 60: should still be 9% CPR (flat after month 30)
        let smm = model.smm(60);
        let cpr = monthly_to_annual(smm);
        assert!((cpr - 0.09).abs() < 0.001);
    }

    #[test]
    fn test_default_model_sda_curve() {
        use super::super::super::specs::DefaultModelSpec;

        let model = DefaultModelSpec::sda(2.0); // 200% SDA

        // Should ramp up then decline
        let early = model.mdr(10);
        let peak = model.mdr(30);
        let late = model.mdr(70);

        assert!(peak > early, "Peak should be greater than early");
        assert!(peak > late, "Peak should be greater than late");
        assert!(early > 0.0, "Early should be positive");
    }
}
