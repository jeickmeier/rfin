//! Construction and parameter validation tests for Inflation-Linked Bonds
//!
//! Tests cover:
//! - Creation via builder pattern
//! - TIPS and UK Gilt helper constructors
//! - Parameter validation
//! - Various indexation methods
//! - Deflation protection settings

use super::common::*;
use finstack_core::currency::Currency;
use finstack_core::dates::{DayCount, Frequency};
use finstack_core::money::Money;
use finstack_valuations::instruments::inflation_linked_bond::parameters::InflationLinkedBondParams;
use finstack_valuations::instruments::inflation_linked_bond::{
    DeflationProtection, IndexationMethod, InflationLinkedBond,
};

#[test]
fn test_tips_creation_via_helper() {
    // Arrange
    let notional = Money::new(1_000_000.0, Currency::USD);
    let issue = d(2020, 1, 15);
    let maturity = d(2030, 1, 15);

    let bond_params = InflationLinkedBondParams::tips(
        notional, 0.0125, // 1.25% real coupon
        issue, maturity, 250.0, // Base CPI
    );

    // Act
    let tips = InflationLinkedBond::new_tips("US_TIPS_2030", &bond_params, "USD-REAL", "US-CPI-U");

    // Assert
    assert_eq!(tips.id.as_str(), "US_TIPS_2030");
    assert_eq!(tips.indexation_method, IndexationMethod::TIPS);
    assert_eq!(tips.real_coupon, 0.0125);
    assert_eq!(tips.base_index, 250.0);
    assert_eq!(tips.notional.amount(), 1_000_000.0);
    assert_eq!(tips.notional.currency(), Currency::USD);
    assert_eq!(tips.freq, Frequency::semi_annual());
    assert_eq!(tips.dc, DayCount::ActAct);
    assert_eq!(tips.deflation_protection, DeflationProtection::MaturityOnly);
}

#[test]
fn test_uk_linker_creation_via_helper() {
    // Arrange
    let notional = Money::new(1_000_000.0, Currency::GBP);
    let issue = d(2020, 3, 22);
    let maturity = d(2040, 3, 22);
    let base_date = d(2019, 7, 1);

    let bond_params = InflationLinkedBondParams::uk_linker(
        notional, 0.00625, // 0.625% real coupon
        issue, maturity, 280.0, // Base RPI
    );

    // Act
    let uk_gilt = InflationLinkedBond::new_uk_linker(
        "UK_GILT_2040",
        &bond_params,
        base_date,
        "GBP-NOMINAL",
        "UK-RPI",
    );

    // Assert
    assert_eq!(uk_gilt.id.as_str(), "UK_GILT_2040");
    assert_eq!(uk_gilt.indexation_method, IndexationMethod::UK);
    assert_eq!(uk_gilt.real_coupon, 0.00625);
    assert_eq!(uk_gilt.base_index, 280.0);
    assert_eq!(uk_gilt.base_date, base_date);
    assert_eq!(uk_gilt.notional.currency(), Currency::GBP);
    assert_eq!(uk_gilt.deflation_protection, DeflationProtection::None);
}

#[test]
fn test_builder_pattern_full_customization() {
    // Arrange & Act
    let bond = sample_tips();

    // Assert - verify all fields are set correctly
    assert_eq!(bond.id.as_str(), "TIPS-TEST");
    assert_eq!(bond.notional.amount(), 1_000_000.0);
    assert_eq!(bond.real_coupon, 0.0125);
    assert_eq!(bond.issue, d(2020, 1, 15));
    assert_eq!(bond.maturity, d(2030, 1, 15));
    assert_eq!(bond.base_index, 250.0);
    assert_eq!(bond.indexation_method, IndexationMethod::TIPS);
}

#[test]
fn test_indexation_method_display() {
    // Arrange & Act & Assert
    assert_eq!(IndexationMethod::TIPS.to_string(), "tips");
    assert_eq!(IndexationMethod::UK.to_string(), "uk");
    assert_eq!(IndexationMethod::Canadian.to_string(), "canadian");
    assert_eq!(IndexationMethod::French.to_string(), "french");
    assert_eq!(IndexationMethod::Japanese.to_string(), "japanese");
}

#[test]
fn test_indexation_method_from_str() {
    // Arrange & Act & Assert
    use std::str::FromStr;

    assert_eq!(
        IndexationMethod::from_str("tips").unwrap(),
        IndexationMethod::TIPS
    );
    assert_eq!(
        IndexationMethod::from_str("us").unwrap(),
        IndexationMethod::TIPS
    );
    assert_eq!(
        IndexationMethod::from_str("UK").unwrap(),
        IndexationMethod::UK
    );
    assert_eq!(
        IndexationMethod::from_str("canadian").unwrap(),
        IndexationMethod::Canadian
    );
    assert_eq!(
        IndexationMethod::from_str("french").unwrap(),
        IndexationMethod::French
    );
    assert_eq!(
        IndexationMethod::from_str("japanese").unwrap(),
        IndexationMethod::Japanese
    );
    assert_eq!(
        IndexationMethod::from_str("jgb").unwrap(),
        IndexationMethod::Japanese
    );

    assert!(IndexationMethod::from_str("invalid").is_err());
}

#[test]
fn test_indexation_method_standard_lags() {
    // Arrange & Act
    use finstack_core::market_data::scalars::inflation_index::InflationLag;

    // Assert
    assert_eq!(
        IndexationMethod::TIPS.standard_lag(),
        InflationLag::Months(3)
    );
    assert_eq!(
        IndexationMethod::Canadian.standard_lag(),
        InflationLag::Months(3)
    );
    assert_eq!(IndexationMethod::UK.standard_lag(), InflationLag::Months(8));
    assert_eq!(
        IndexationMethod::French.standard_lag(),
        InflationLag::Months(3)
    );
    assert_eq!(
        IndexationMethod::Japanese.standard_lag(),
        InflationLag::Months(3)
    );
}

#[test]
fn test_indexation_method_interpolation_flags() {
    // Arrange & Act & Assert
    assert!(IndexationMethod::TIPS.uses_daily_interpolation());
    assert!(IndexationMethod::Canadian.uses_daily_interpolation());
    assert!(!IndexationMethod::UK.uses_daily_interpolation());
    assert!(!IndexationMethod::French.uses_daily_interpolation());
    assert!(!IndexationMethod::Japanese.uses_daily_interpolation());
}

#[test]
fn test_deflation_protection_display() {
    // Arrange & Act & Assert
    assert_eq!(DeflationProtection::None.to_string(), "none");
    assert_eq!(
        DeflationProtection::MaturityOnly.to_string(),
        "maturity_only"
    );
    assert_eq!(DeflationProtection::AllPayments.to_string(), "all_payments");
}

#[test]
fn test_deflation_protection_from_str() {
    // Arrange & Act & Assert
    use std::str::FromStr;

    assert_eq!(
        DeflationProtection::from_str("none").unwrap(),
        DeflationProtection::None
    );
    assert_eq!(
        DeflationProtection::from_str("maturity_only").unwrap(),
        DeflationProtection::MaturityOnly
    );
    assert_eq!(
        DeflationProtection::from_str("maturity").unwrap(),
        DeflationProtection::MaturityOnly
    );
    assert_eq!(
        DeflationProtection::from_str("all_payments").unwrap(),
        DeflationProtection::AllPayments
    );
    assert_eq!(
        DeflationProtection::from_str("all").unwrap(),
        DeflationProtection::AllPayments
    );
    // Test case insensitivity and dash/underscore normalization
    assert_eq!(
        DeflationProtection::from_str("MATURITY-ONLY").unwrap(),
        DeflationProtection::MaturityOnly
    );

    assert!(DeflationProtection::from_str("invalid").is_err());
}

#[test]
fn test_parameter_struct_tips() {
    // Arrange
    let notional = Money::new(100_000.0, Currency::USD);
    let issue = d(2020, 1, 1);
    let maturity = d(2025, 1, 1);

    // Act
    let params = InflationLinkedBondParams::tips(notional, 0.02, issue, maturity, 200.0);

    // Assert
    assert_eq!(params.notional.amount(), 100_000.0);
    assert_eq!(params.real_coupon, 0.02);
    assert_eq!(params.issue, issue);
    assert_eq!(params.maturity, maturity);
    assert_eq!(params.base_index, 200.0);
    assert_eq!(params.frequency, Frequency::semi_annual());
    assert_eq!(params.day_count, DayCount::ActAct);
}

#[test]
fn test_parameter_struct_uk_linker() {
    // Arrange
    let notional = Money::new(100_000.0, Currency::GBP);
    let issue = d(2020, 1, 1);
    let maturity = d(2030, 1, 1);

    // Act
    let params = InflationLinkedBondParams::uk_linker(notional, 0.005, issue, maturity, 300.0);

    // Assert
    assert_eq!(params.notional.amount(), 100_000.0);
    assert_eq!(params.real_coupon, 0.005);
    assert_eq!(params.issue, issue);
    assert_eq!(params.maturity, maturity);
    assert_eq!(params.base_index, 300.0);
    assert_eq!(params.frequency, Frequency::semi_annual());
    assert_eq!(params.day_count, DayCount::ActAct);
}

#[test]
fn test_various_currencies() {
    // Arrange
    let issue = d(2020, 1, 1);
    let maturity = d(2030, 1, 1);

    // Act & Assert - Test various currencies
    for (ccy, base_cpi) in [
        (Currency::USD, 250.0),
        (Currency::GBP, 280.0),
        (Currency::EUR, 100.0),
        (Currency::CAD, 140.0),
        (Currency::JPY, 100.0),
    ] {
        let notional = Money::new(1_000_000.0, ccy);
        let params = InflationLinkedBondParams::new(
            notional,
            0.01,
            issue,
            maturity,
            base_cpi,
            Frequency::semi_annual(),
            DayCount::ActAct,
        );

        let bond = InflationLinkedBond::new_tips(
            format!("ILB-{}", ccy),
            &params,
            format!("{}-REAL", ccy),
            format!("{}-CPI", ccy),
        );

        assert_eq!(bond.notional.currency(), ccy);
    }
}

#[test]
fn test_various_frequencies() {
    // Arrange
    let notional = Money::new(1_000_000.0, Currency::USD);
    let issue = d(2020, 1, 1);
    let maturity = d(2030, 1, 1);

    // Act & Assert - Test various payment frequencies
    for freq in [
        Frequency::annual(),
        Frequency::semi_annual(),
        Frequency::quarterly(),
    ] {
        let params = InflationLinkedBondParams::new(
            notional,
            0.01,
            issue,
            maturity,
            250.0,
            freq,
            DayCount::ActAct,
        );

        let bond = InflationLinkedBond::new_tips("ILB-TEST", &params, "USD-REAL", "US-CPI-U");

        assert_eq!(bond.freq, freq);
    }
}

#[test]
fn test_various_day_count_conventions() {
    // Arrange
    let notional = Money::new(1_000_000.0, Currency::USD);
    let issue = d(2020, 1, 1);
    let maturity = d(2030, 1, 1);

    // Act & Assert - Test various day count conventions
    for dc in [DayCount::ActAct, DayCount::Act360, DayCount::Thirty360] {
        let params = InflationLinkedBondParams::new(
            notional,
            0.01,
            issue,
            maturity,
            250.0,
            Frequency::semi_annual(),
            dc,
        );

        let bond = InflationLinkedBond::new_tips("ILB-TEST", &params, "USD-REAL", "US-CPI-U");

        assert_eq!(bond.dc, dc);
    }
}

#[test]
fn test_quoted_clean_price() {
    // Arrange & Act
    let mut bond = sample_tips();

    // Assert - quoted price can be set and cleared
    // Note: sample_tips() may or may not have a default quoted_clean

    // Act - update quoted price
    bond.quoted_clean = Some(105.5);

    // Assert
    assert_eq!(bond.quoted_clean, Some(105.5));

    // Act - clear quoted price
    bond.quoted_clean = None;

    // Assert
    assert_eq!(bond.quoted_clean, None);
}
