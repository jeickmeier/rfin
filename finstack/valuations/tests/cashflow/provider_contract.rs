//! Contract tests for `CashflowProvider` implementations.
//!
//! These tests ensure all instruments correctly implement the trait contract.
//! Add new instruments here when they implement `CashflowProvider` to catch
//! drift and ensure consistent behavior across the codebase.
//!
//! # Contract Properties Verified
//!
//! 1. `build_schedule` succeeds with minimal valid market context
//! 2. Returned flows are sorted by date (ascending)
//! 3. All flows have the same currency as the instrument's notional (if provided)
//! 4. Future flows (after as_of) are included; past flows may be filtered
//!
//! # Adding New Instruments
//!
//! To add contract tests for a new instrument:
//!
//! ```rust,ignore
//! #[test]
//! fn my_instrument_satisfies_contract() {
//!     let as_of = d(2025, 1, 1);
//!     let inst = MyInstrument::new(/* ... */);
//!     verify_provider_contract(&inst, &minimal_market(), as_of);
//! }
//! ```

use crate::cashflow_tests::test_helpers::d;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::cashflow::traits::CashflowProvider;

// =============================================================================
// Contract Verification
// =============================================================================

/// Verifies basic `CashflowProvider` contract properties.
///
/// # Contract Properties
///
/// 1. `build_schedule` returns `Ok` with valid market context
/// 2. Returned flows are sorted by date (non-decreasing)
/// 3. All flows have the same currency as notional (if notional is provided)
///
/// # Panics
///
/// Panics with descriptive message if any contract property is violated.
fn verify_provider_contract<T: CashflowProvider>(
    provider: &T,
    market: &MarketContext,
    as_of: Date,
) {
    // Contract 1: build_schedule should succeed with valid inputs
    let flows = provider
        .build_schedule(market, as_of)
        .expect("build_schedule should succeed with valid market context");

    // Contract 2: Flows must be sorted by date (non-decreasing)
    for window in flows.windows(2) {
        let (d1, _) = window[0];
        let (d2, _) = window[1];
        assert!(
            d1 <= d2,
            "Flows must be sorted by date: found {} after {}",
            d2,
            d1
        );
    }

    // Contract 3: Currency consistency (if notional provided)
    if let Some(notional) = provider.notional() {
        let expected_ccy = notional.currency();
        for (date, money) in &flows {
            assert_eq!(
                money.currency(),
                expected_ccy,
                "Flow on {} has currency {:?}, expected {:?} (from notional)",
                date,
                money.currency(),
                expected_ccy
            );
        }
    }
}

/// Creates a minimal market context for contract testing.
///
/// Contains flat discount and forward curves sufficient for most instruments.
fn minimal_market() -> MarketContext {
    let base = d(2025, 1, 1);

    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([(0.0, 1.0), (10.0, 0.75)])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("valid discount curve");

    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base)
        .knots([(0.0, 0.04), (10.0, 0.05)])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("valid forward curve");

    MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd)
}

// =============================================================================
// Bond Contract Tests
// =============================================================================

mod bond_contract {
    use super::*;
    use finstack_valuations::instruments::Bond;

    #[test]
    fn fixed_bond_satisfies_contract() {
        let as_of = d(2025, 1, 1);
        let issue = d(2025, 1, 15);
        let maturity = d(2030, 1, 15);

        let bond = Bond::fixed(
            "TEST-FIXED-BOND",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            issue,
            maturity,
            "USD-OIS",
        )
        .unwrap();

        verify_provider_contract(&bond, &minimal_market(), as_of);
    }

    #[test]
    fn floating_bond_satisfies_contract() {
        use finstack_core::dates::{DayCount, Tenor};

        let as_of = d(2025, 1, 1);
        let issue = d(2025, 1, 15);
        let maturity = d(2030, 1, 15);

        let bond = Bond::floating(
            "TEST-FLOAT-BOND",
            Money::new(1_000_000.0, Currency::USD),
            "USD-SOFR-3M",
            200.0, // 200 bps spread
            issue,
            maturity,
            Tenor::quarterly(),
            DayCount::Act360,
            "USD-OIS",
        )
        .unwrap();

        verify_provider_contract(&bond, &minimal_market(), as_of);
    }
}

// =============================================================================
// IRS Contract Tests
// =============================================================================

mod irs_contract {
    use super::*;
    use finstack_valuations::instruments::irs::{InterestRateSwap, PayReceive};

    #[test]
    fn usd_swap_pay_fixed_satisfies_contract() {
        let as_of = d(2025, 1, 1);
        let start = d(2025, 1, 15);
        let end = d(2030, 1, 15);

        let swap = InterestRateSwap::create_usd_swap(
            "TEST-IRS-PAY".into(),
            Money::new(10_000_000.0, Currency::USD),
            0.04,
            start,
            end,
            PayReceive::PayFixed,
        )
        .expect("valid swap");

        verify_provider_contract(&swap, &minimal_market(), as_of);
    }

    #[test]
    fn usd_swap_receive_fixed_satisfies_contract() {
        let as_of = d(2025, 1, 1);
        let start = d(2025, 1, 15);
        let end = d(2030, 1, 15);

        let swap = InterestRateSwap::create_usd_swap(
            "TEST-IRS-REC".into(),
            Money::new(10_000_000.0, Currency::USD),
            0.04,
            start,
            end,
            PayReceive::ReceiveFixed,
        )
        .expect("valid swap");

        verify_provider_contract(&swap, &minimal_market(), as_of);
    }
}

// =============================================================================
// Term Loan Contract Tests
// =============================================================================

mod term_loan_contract {
    use super::*;
    use finstack_valuations::instruments::TermLoan;

    #[test]
    fn example_term_loan_satisfies_contract() {
        // TermLoan only exposes example() constructor; use builder for production
        let as_of = d(2025, 1, 1);
        let loan = TermLoan::example();

        verify_provider_contract(&loan, &minimal_market(), as_of);
    }
}
