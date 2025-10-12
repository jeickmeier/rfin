//! Tests for Phase 2: Generic prepayment and default models
//!
//! Tests the extensible framework for modeling prepayments and defaults
//! across different asset classes.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::structured_credit::*;
use time::macros::date;

#[test]
fn test_cpr_model() {
    let cpr = CPRModel::new(0.06); // 6% annual CPR
    let market = MarketConditions::default();

    let orig = date!(2020 - 01 - 01);
    let as_of = date!(2023 - 01 - 01);

    let smm = cpr.prepayment_rate(as_of, orig, 36, &market);

    // 6% CPR = ~0.5143% SMM
    assert!((smm - 0.005143).abs() < 0.0001);
    assert_eq!(cpr.model_name(), "CPR");
}

#[test]
fn test_psa_model_ramp() {
    let psa = PSAModel::new(1.5); // 150% PSA
    let market = MarketConditions::default();

    let orig = date!(2020 - 01 - 01);
    let as_of = date!(2021 - 03 - 01); // 15 months seasoning

    // At month 15, should be 15/30 * 6% * 1.5 = 4.5% CPR
    let smm = psa.prepayment_rate(as_of, orig, 15, &market);
    let cpr = smm_to_cpr(smm);
    assert!((cpr - 0.045).abs() < 0.001);

    // At month 30 and beyond
    let smm_30 = psa.prepayment_rate(as_of, orig, 30, &market);
    let cpr_30 = smm_to_cpr(smm_30);
    assert!((cpr_30 - 0.09).abs() < 0.001); // 6% * 1.5 = 9%
}

#[test]
fn test_mortgage_prepayment_with_refi_incentive() {
    let model = MortgagePrepaymentModel::default();
    let market = MarketConditions {
        original_rate: Some(0.05), // 5% original rate
        refi_rate: 0.03,           // 3% current rate (200bps incentive)
        ..Default::default()
    };

    let orig = date!(2020 - 01 - 01);
    let as_of = date!(2023 - 07 - 01); // 42 months, July

    let smm = model.prepayment_rate(as_of, orig, 42, &market);

    // Should be elevated due to refi incentive
    // The model applies multiple factors that affect the final rate
    assert!(smm > 0.005); // Should be higher than base
    assert!(smm < 0.10); // But reasonable upper bound
}

#[test]
fn test_auto_prepayment_abs_speed() {
    let model = AutoPrepaymentModel {
        abs_speed: 0.015, // 1.5% ABS
        ramp_months: 12,
        loss_severity: 0.35,
    };
    let market = MarketConditions::default();

    let orig = date!(2020 - 01 - 01);

    // During ramp (month 6)
    let as_of_ramp = date!(2020 - 07 - 01);
    let rate_ramp = model.prepayment_rate(as_of_ramp, orig, 6, &market);
    assert!((rate_ramp - 0.0075).abs() < 0.001); // 50% of full speed

    // After ramp (month 24)
    let as_of_full = date!(2022 - 01 - 01);
    let rate_full = model.prepayment_rate(as_of_full, orig, 24, &market);
    assert!((rate_full - 0.015).abs() < 0.001); // Full 1.5% ABS
}

#[test]
fn test_commercial_prepayment_lockout() {
    let model = CommercialPrepaymentModel {
        lockout_months: 24,
        yield_maintenance_months: 36,
        defeasance_months: Some(24),
        open_cpr: 0.10,
        balloon_month: Some(120),
    };
    let market = MarketConditions::default();

    let orig = date!(2020 - 01 - 01);

    // During lockout (month 12)
    let as_of_lockout = date!(2021 - 01 - 01);
    let rate_lockout = model.prepayment_rate(as_of_lockout, orig, 12, &market);
    assert_eq!(rate_lockout, 0.0);

    // During yield maintenance (month 36)
    let as_of_ym = date!(2023 - 01 - 01);
    let rate_ym = model.prepayment_rate(as_of_ym, orig, 36, &market);
    assert_eq!(rate_ym, 0.001); // Exactly 0.1% CPR during yield maintenance

    // Open period (month 90)
    let as_of_open = date!(2027 - 07 - 01);
    let rate_open = model.prepayment_rate(as_of_open, orig, 90, &market);
    let cpr_open = smm_to_cpr(rate_open);
    assert!((cpr_open - 0.10).abs() < 0.01); // 10% CPR
}

#[test]
fn test_credit_card_payment_seasonality() {
    let model = CreditCardPaymentModel {
        payment_rate: 0.15,
        charge_off_rate: 0.005,
        use_seasonality: true,
    };
    let market = MarketConditions::default();

    let orig = date!(2020 - 01 - 01);

    // January - higher payments
    let as_of_jan = date!(2023 - 01 - 01);
    let rate_jan = model.prepayment_rate(as_of_jan, orig, 36, &market);
    assert!((rate_jan - 0.15 * 1.15).abs() < 0.01);

    // July - lower payments
    let as_of_jul = date!(2023 - 07 - 01);
    let rate_jul = model.prepayment_rate(as_of_jul, orig, 42, &market);
    assert!((rate_jul - 0.15 * 0.95).abs() < 0.01);
}

#[test]
fn test_vector_prepayment_model() {
    let cpr_schedule = vec![0.02, 0.04, 0.06, 0.08, 0.10];
    let model = VectorModel::new(cpr_schedule.clone(), 0.12);
    let market = MarketConditions::default();

    let orig = date!(2020 - 01 - 01);
    let as_of = date!(2020 - 03 - 01);

    // Month 2 should use index 2 (0.06 CPR)
    let smm_2 = model.prepayment_rate(as_of, orig, 2, &market);
    let cpr_2 = smm_to_cpr(smm_2);
    assert!((cpr_2 - 0.06).abs() < 0.001);

    // Month 10 should use terminal rate
    let smm_10 = model.prepayment_rate(as_of, orig, 10, &market);
    let cpr_10 = smm_to_cpr(smm_10);
    assert!((cpr_10 - 0.12).abs() < 0.001);
}

#[test]
fn test_cdr_model() {
    let cdr = CDRModel::new(0.02); // 2% annual default rate
    let factors = CreditFactors::default();

    let orig = date!(2020 - 01 - 01);
    let as_of = date!(2023 - 01 - 01);

    let mdr = cdr.default_rate(as_of, orig, 36, &factors);

    // 2% CDR = ~0.168% MDR
    assert!((mdr - 0.00168).abs() < 0.0001);
}

#[test]
fn test_sda_model_curve() {
    let sda = SDAModel {
        speed: 1.0,
        peak_month: 30,
        peak_cdr: 0.006,
        terminal_cdr: 0.0003,
    };
    let factors = CreditFactors::default();

    let orig = date!(2020 - 01 - 01);

    // At month 15 (halfway to peak)
    let mdr_15 = sda.default_rate(orig, orig, 15, &factors);
    let cdr_15 = mdr_to_cdr(mdr_15);
    assert!((cdr_15 - 0.003).abs() < 0.001); // Half of peak

    // At peak (month 30)
    let mdr_30 = sda.default_rate(orig, orig, 30, &factors);
    let cdr_30 = mdr_to_cdr(mdr_30);
    assert!((cdr_30 - 0.006).abs() < 0.001);

    // Terminal (month 90)
    let mdr_90 = sda.default_rate(orig, orig, 90, &factors);
    let cdr_90 = mdr_to_cdr(mdr_90);
    assert!((cdr_90 - 0.0003).abs() < 0.0001);
}

#[test]
fn test_mortgage_default_with_credit_factors() {
    let model = MortgageDefaultModel::default();
    let factors = CreditFactors {
        credit_score: Some(650),       // Below 700
        ltv: Some(0.90),               // Above 80%
        unemployment_rate: Some(0.06), // Above 4%
        ..Default::default()
    };

    let orig = date!(2020 - 01 - 01);
    let as_of = date!(2022 - 01 - 01); // 24 months (peak)

    let mdr = model.default_rate(as_of, orig, 24, &factors);

    // Base 0.2% CDR at peak
    // Multiple adjustments compound the rate significantly
    let cdr = mdr_to_cdr(mdr);
    assert!(cdr > 0.005);
    assert!(cdr < 0.02); // Allow for significant compounding effects
}

#[test]
fn test_auto_default_credit_tiers() {
    let model = AutoDefaultModel::default();
    let mut factors = CreditFactors::default();

    let orig = date!(2020 - 01 - 01);
    let as_of = date!(2021 - 07 - 01); // 18 months (peak)

    // Prime tier
    factors
        .custom_factors
        .insert("credit_tier".to_string(), 750.0);
    let mdr_prime = model.default_rate(as_of, orig, 18, &factors);
    let cdr_prime = mdr_to_cdr(mdr_prime);
    assert!(cdr_prime < 0.015); // Lower than base

    // Subprime tier
    factors
        .custom_factors
        .insert("credit_tier".to_string(), 650.0);
    let mdr_subprime = model.default_rate(as_of, orig, 18, &factors);
    let cdr_subprime = mdr_to_cdr(mdr_subprime);
    assert!(cdr_subprime > 0.04); // Much higher than base
}

#[test]
fn test_constant_recovery() {
    let model = ConstantRecoveryModel::new(0.40);
    let market = MarketFactors::default();

    let recovery = model.recovery_rate(
        date!(2023 - 01 - 01),
        12,
        None,
        Money::new(100_000.0, Currency::USD),
        &market,
    );

    assert_eq!(recovery, 0.40);
    assert_eq!(
        model.loss_severity(
            date!(2023 - 01 - 01),
            12,
            None,
            Money::new(100_000.0, Currency::USD),
            &market
        ),
        0.60
    );
}

#[test]
fn test_collateral_recovery_with_market_stress() {
    let model = CollateralRecoveryModel {
        base_recovery: 0.10,
        advance_rate: 0.85,
        time_decay: 0.01,
    };

    let market = MarketFactors {
        price_index: 0.80,          // 20% price decline
        liquidation_discount: 0.15, // 15% haircut
        foreclosure_costs: Money::new(10_000.0, Currency::USD),
        ..Default::default()
    };

    let collateral = Some(Money::new(200_000.0, Currency::USD));
    let outstanding = Money::new(250_000.0, Currency::USD);

    let recovery = model.recovery_rate(
        date!(2023 - 01 - 01),
        12, // 12 months to resolution
        collateral,
        outstanding,
        &market,
    );

    // Collateral: 200k * 0.80 * 0.85 = 136k
    // Less costs: 136k - 10k = 126k
    // Time decay: 126k * (1 - 12*0.01) = 110.88k
    // Recovery: 110.88k / 250k = 44.35%
    // With advance rate: 44.35% * 0.85 + 10% * 0.15 = 39.2%
    assert!(recovery > 0.35);
    assert!(recovery < 0.45);
}

#[test]
fn test_prepayment_factory() {
    // Test factory creation for different asset types
    let mortgage = PrepaymentModelFactory::create_default("mortgage");
    assert_eq!(mortgage.model_name(), "MortgagePrepayment");

    let auto = PrepaymentModelFactory::create_default("auto");
    assert_eq!(auto.model_name(), "AutoPrepayment");

    let card = PrepaymentModelFactory::create_default("credit_card");
    assert_eq!(card.model_name(), "CreditCardPayment");

    let commercial = PrepaymentModelFactory::create_default("cmbs");
    assert_eq!(commercial.model_name(), "CommercialPrepayment");

    let student = PrepaymentModelFactory::create_default("student_loan");
    assert_eq!(student.model_name(), "StudentLoanPrepayment");

    // Test specific model creation
    let psa = PrepaymentModelFactory::create_psa(2.0);
    assert_eq!(psa.model_name(), "PSA");

    let cpr = PrepaymentModelFactory::create_cpr(0.08);
    assert_eq!(cpr.model_name(), "CPR");
}

#[test]
fn test_default_factory() {
    // Test factory creation for different asset types
    let mortgage = DefaultModelFactory::create_default_model("mortgage");
    assert_eq!(mortgage.model_name(), "MortgageDefault");

    let auto = DefaultModelFactory::create_default_model("auto");
    assert_eq!(auto.model_name(), "AutoDefault");

    let card = DefaultModelFactory::create_default_model("credit_card");
    assert_eq!(card.model_name(), "CreditCardChargeOff");

    // Test recovery models
    let mortgage_recovery = DefaultModelFactory::create_recovery_model("mortgage");
    assert_eq!(mortgage_recovery.model_name(), "CollateralRecovery");

    let card_recovery = DefaultModelFactory::create_recovery_model("credit_card");
    assert_eq!(card_recovery.model_name(), "ConstantRecovery");
}

#[test]
fn test_cumulative_default_calculation() {
    let model = CDRModel::new(0.02); // 2% annual
    let factors = CreditFactors::default();

    let orig = date!(2020 - 01 - 01);
    let as_of = date!(2023 - 01 - 01); // 36 months

    let cumulative = model.cumulative_default_rate(as_of, orig, &factors);

    // Rough approximation: 1 - (1 - 0.02)^3 ≈ 5.88%
    assert!(cumulative > 0.055);
    assert!(cumulative < 0.065);
}

#[test]
fn test_student_loan_deferment() {
    let model = StudentLoanPrepaymentModel {
        grace_period_months: 6,
        deferment_months: 48,
        repayment_cpr: 0.03,
        consolidation_rate: 0.05,
    };
    let market = MarketConditions::default();

    let orig = date!(2020 - 01 - 01);

    // During deferment (month 24)
    let as_of_defer = date!(2022 - 01 - 01);
    let rate_defer = model.prepayment_rate(as_of_defer, orig, 24, &market);
    assert!((rate_defer - 0.05 / 12.0).abs() < 0.001); // Only consolidations

    // Full repayment (month 60)
    let as_of_repay = date!(2025 - 01 - 01);
    let rate_repay = model.prepayment_rate(as_of_repay, orig, 60, &market);
    assert!(rate_repay > 0.005); // Higher with both prepayment and consolidation
}
