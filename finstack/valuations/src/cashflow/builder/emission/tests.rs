//! Tests for emission functions.

#[cfg(test)]
mod accrual_context_tests {
    use super::super::super::compiler::{FixedSchedule, FloatSchedule};
    use super::super::super::specs::{
        CouponType, FixedCouponSpec, FloatingCouponSpec, FloatingRateSpec,
    };
    use super::super::coupons::{emit_fixed_coupons_on, emit_float_coupons_on};
    use finstack_core::currency::Currency;
    use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
    use finstack_core::types::CurveId;
    use time::Month;

    #[test]
    fn fixed_accrual_with_actact_isma_full_period() {
        // Test that Act/Act ISMA with frequency context gives accrual = 1.0 for full coupon
        let start = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
        let end = Date::from_calendar_date(2025, Month::July, 15).expect("valid date");

        let spec = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.05,
            freq: Frequency::Months(6), // Semi-annual
            dc: DayCount::ActActIsma,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        let dates = vec![start, end];
        let mut prev_map = hashbrown::HashMap::new();
        prev_map.insert(end, start);
        let first_last = hashbrown::HashSet::new();

        let schedule: FixedSchedule = (spec, dates, prev_map, first_last);
        let outstanding_after = hashbrown::HashMap::new();
        let outstanding_fallback = 1_000_000.0;

        let (pik, flows) = emit_fixed_coupons_on(
            end,
            &[schedule],
            &outstanding_after,
            outstanding_fallback,
            Currency::USD,
        )
        .expect("should emit fixed coupons");

        assert_eq!(pik, 0.0);
        assert_eq!(flows.len(), 1);

        // Accrual factor should be 1.0 for full coupon period in ISMA
        assert!(
            (flows[0].accrual_factor - 1.0).abs() < 1e-6,
            "Expected accrual ~1.0, got {}",
            flows[0].accrual_factor
        );

        // Coupon amount: 1M × 5% × 1.0 = 50K
        assert!((flows[0].amount.amount() - 50_000.0).abs() < 1.0);
    }

    #[test]
    fn float_accrual_with_actact_isma_quarterly() {
        // Test quarterly floating with Act/Act ISMA
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let end = Date::from_calendar_date(2025, Month::April, 1).expect("valid date");

        let spec = FloatingCouponSpec {
            rate_spec: FloatingRateSpec {
                index_id: CurveId::new("USD-SOFR-3M"),
                spread_bp: 200.0,
                gearing: 1.0,
                gearing_includes_spread: true,
                floor_bp: None,
                cap_bp: None,
                all_in_floor_bp: None,
                index_cap_bp: None,
                reset_freq: Frequency::Months(3),
                reset_lag_days: 2,
                dc: DayCount::ActActIsma,
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
                fixing_calendar_id: None,
            },
            coupon_type: CouponType::Cash,
            freq: Frequency::Months(3),
            stub: StubKind::None,
        };

        let dates = vec![start, end];
        let mut prev_map = hashbrown::HashMap::new();
        prev_map.insert(end, start);

        let schedule: FloatSchedule = (spec, dates, prev_map);
        let outstanding_after = hashbrown::HashMap::new();
        let outstanding_fallback = 1_000_000.0;

        let (pik, flows) = emit_float_coupons_on(
            end,
            &[schedule],
            &outstanding_after,
            outstanding_fallback,
            Currency::USD,
            &[None], // One resolved curve slot (None) to match the one float schedule
        )
        .expect("should emit float coupons");

        assert_eq!(pik, 0.0);
        assert_eq!(flows.len(), 1);

        // For full quarterly period, ISMA accrual should be 1.0
        assert!(
            (flows[0].accrual_factor - 1.0).abs() < 1e-6,
            "Expected accrual ~1.0 for full ISMA quarter, got {}",
            flows[0].accrual_factor
        );
    }

    #[test]
    fn bus252_accrual_requires_calendar() {
        // Bus/252 with calendar should calculate business days

        let start = Date::from_calendar_date(2025, Month::January, 6).expect("valid date"); // Monday
        let end = Date::from_calendar_date(2025, Month::January, 13).expect("valid date"); // Next Monday (5 biz days)

        // This test verifies that the calendar lookup happens in coupons.rs
        // The actual year fraction calculation is tested in core's day-count tests
        // Here we just verify no panic/error when using Bus/252 with a valid calendar ID

        let spec = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.05,
            freq: Frequency::Days(7),
            dc: DayCount::Bus252,
            bdc: BusinessDayConvention::Following,
            calendar_id: Some("NYSE".to_string()),
            stub: StubKind::None,
        };

        let dates = vec![start, end];
        let mut prev_map = hashbrown::HashMap::new();
        prev_map.insert(end, start);
        let first_last = hashbrown::HashSet::new();

        let schedule: FixedSchedule = (spec, dates, prev_map, first_last);
        let outstanding_after = hashbrown::HashMap::new();
        let outstanding_fallback = 1_000_000.0;

        let result = emit_fixed_coupons_on(
            end,
            &[schedule],
            &outstanding_after,
            outstanding_fallback,
            Currency::USD,
        );

        // Should succeed with calendar available
        assert!(result.is_ok());
        let (pik, flows) = result.expect("should succeed with calendar");
        assert_eq!(pik, 0.0);
        assert_eq!(flows.len(), 1);

        // Year fraction should be roughly 5 business days / 252
        let expected_yf = 5.0 / 252.0;
        assert!(
            (flows[0].accrual_factor - expected_yf).abs() < 0.01,
            "Expected accrual ~{}, got {}",
            expected_yf,
            flows[0].accrual_factor
        );
    }
}

#[cfg(test)]
mod credit_emission_tests {
    use super::super::super::specs::DefaultEvent;
    use super::super::credit::{emit_default_on, emit_prepayment_on};
    use crate::cashflow::primitives::CFKind;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::dates::DateExt;
    use time::Month;

    #[test]
    fn test_default_and_recovery_mechanics() {
        let d = Date::from_calendar_date(2025, Month::March, 1).expect("valid date");
        let event = DefaultEvent {
            default_date: d,
            defaulted_amount: 400_000.0,
            recovery_rate: 0.40,
            recovery_lag: 12,
            recovery_bdc: None,
            recovery_calendar_id: None,
        };

        let mut outstanding = 1_000_000.0;
        let flows = emit_default_on(d, &[event], &mut outstanding, Currency::USD)
            .expect("should emit default");

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
        let expected_recovery_date = d.add_months(12);
        assert_eq!(flows[1].date, expected_recovery_date);
    }

    #[test]
    fn test_coupon_on_reduced_outstanding_after_default() {
        // CRITICAL TEST: Verify coupon uses reduced outstanding after default
        use super::super::super::specs::{CouponType, FixedCouponSpec};
        use super::super::coupons::emit_fixed_coupons_on;
        use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};

        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let mat = Date::from_calendar_date(2026, Month::January, 1).expect("valid date");
        let default_date = Date::from_calendar_date(2025, Month::July, 1).expect("valid date");
        let coupon_date = Date::from_calendar_date(2025, Month::October, 1).expect("valid date");

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
            recovery_bdc: None,
            recovery_calendar_id: None,
        };

        let mut outstanding = 1_000_000.0;
        let _ = emit_default_on(
            default_date,
            &[default_event],
            &mut outstanding,
            Currency::USD,
        )
        .expect("should emit default");

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
        .expect("should emit fixed coupons");

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
        let d = Date::from_calendar_date(2025, Month::March, 1).expect("valid date");
        let mut outstanding = 1_000_000.0;

        let flows = emit_prepayment_on(d, 50_000.0, &mut outstanding, Currency::USD);

        assert_eq!(outstanding, 950_000.0);
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].kind, CFKind::PrePayment);
        assert_eq!(flows[0].amount.amount(), 50_000.0);
    }

    #[test]
    fn test_prepayment_capped_by_outstanding() {
        let d = Date::from_calendar_date(2025, Month::March, 1).expect("valid date");
        let mut outstanding = 30_000.0;

        let flows = emit_prepayment_on(d, 50_000.0, &mut outstanding, Currency::USD);

        // Can only prepay what's outstanding
        assert_eq!(outstanding, 0.0);
        assert_eq!(flows[0].amount.amount(), 30_000.0);
    }

    #[test]
    fn test_zero_recovery_rate() {
        let d = Date::from_calendar_date(2025, Month::March, 1).expect("valid date");
        let event = DefaultEvent {
            default_date: d,
            defaulted_amount: 100_000.0,
            recovery_rate: 0.0, // Total loss
            recovery_lag: 12,
            recovery_bdc: None,
            recovery_calendar_id: None,
        };

        let mut outstanding = 1_000_000.0;
        let flows = emit_default_on(d, &[event], &mut outstanding, Currency::USD)
            .expect("should emit default");

        // Net loss is 100% of defaulted amount
        assert_eq!(outstanding, 900_000.0);
        assert_eq!(flows.len(), 1); // Only default, no recovery
    }

    #[test]
    fn test_full_recovery_rate() {
        let d = Date::from_calendar_date(2025, Month::March, 1).expect("valid date");
        let event = DefaultEvent {
            default_date: d,
            defaulted_amount: 100_000.0,
            recovery_rate: 1.0, // Full recovery
            recovery_lag: 12,
            recovery_bdc: None,
            recovery_calendar_id: None,
        };

        let mut outstanding = 1_000_000.0;
        let flows = emit_default_on(d, &[event], &mut outstanding, Currency::USD)
            .expect("should emit default");

        // Net loss is zero
        assert_eq!(outstanding, 1_000_000.0);
        assert_eq!(flows.len(), 2); // Default + full recovery
    }

    #[test]
    fn test_multiple_defaults_same_date() {
        let d = Date::from_calendar_date(2025, Month::March, 1).expect("valid date");
        let events = vec![
            DefaultEvent {
                default_date: d,
                defaulted_amount: 50_000.0,
                recovery_rate: 0.40,
                recovery_lag: 12,
                recovery_bdc: None,
                recovery_calendar_id: None,
            },
            DefaultEvent {
                default_date: d,
                defaulted_amount: 30_000.0,
                recovery_rate: 0.50,
                recovery_lag: 6,
                recovery_bdc: None,
                recovery_calendar_id: None,
            },
        ];

        let mut outstanding = 1_000_000.0;
        let flows = emit_default_on(d, &events, &mut outstanding, Currency::USD)
            .expect("should emit multiple defaults");

        // Net loss: 50K × 0.6 + 30K × 0.5 = 30K + 15K = 45K
        assert_eq!(
            outstanding,
            1_000_000.0 - 50_000.0 + 20_000.0 - 30_000.0 + 15_000.0
        );
        assert_eq!(flows.len(), 4); // 2 defaults + 2 recoveries
    }

    #[test]
    fn test_non_matching_dates_return_empty() {
        let d = Date::from_calendar_date(2025, Month::March, 1).expect("valid date");
        let other_date = Date::from_calendar_date(2025, Month::April, 1).expect("valid date");

        let event = DefaultEvent {
            default_date: other_date,
            defaulted_amount: 100_000.0,
            recovery_rate: 0.40,
            recovery_lag: 12,
            recovery_bdc: None,
            recovery_calendar_id: None,
        };

        let mut outstanding = 1_000_000.0;
        let flows = emit_default_on(d, &[event], &mut outstanding, Currency::USD)
            .expect("should emit default");

        assert_eq!(outstanding, 1_000_000.0); // Unchanged
        assert_eq!(flows.len(), 0);
    }

    #[test]
    fn test_recovery_lag_calculation() {
        let d = Date::from_calendar_date(2025, Month::January, 15).expect("valid date");
        let event = DefaultEvent {
            default_date: d,
            defaulted_amount: 100_000.0,
            recovery_rate: 0.40,
            recovery_lag: 6,
            recovery_bdc: None,
            recovery_calendar_id: None,
        };

        let mut outstanding = 1_000_000.0;
        let flows = emit_default_on(d, &[event], &mut outstanding, Currency::USD)
            .expect("should emit default");

        let expected_recovery_date = d.add_months(6);
        assert_eq!(flows[1].date, expected_recovery_date);
    }

    #[test]
    fn test_prepayment_model_psa_curve() {
        use super::super::super::credit_rates::smm_to_cpr;
        use super::super::super::specs::PrepaymentModelSpec;

        let model = PrepaymentModelSpec::psa(1.5); // 150% PSA

        // Month 15: should be 4.5% CPR (halfway to 9%)
        let smm = model.smm(15);
        assert!(smm > 0.0);
        let cpr = smm_to_cpr(smm);
        assert!((cpr - 0.045).abs() < 0.001);

        // Month 30: should be 9% CPR = ~0.77% SMM
        let smm = model.smm(30);
        let cpr = smm_to_cpr(smm);
        assert!((cpr - 0.09).abs() < 0.001);

        // Month 60: should still be 9% CPR (flat after month 30)
        let smm = model.smm(60);
        let cpr = smm_to_cpr(smm);
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
