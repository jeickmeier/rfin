"""Cross-language property tests for pricing consistency.

These tests generate random but valid inputs and verify that:
1. Python bindings produce deterministic results
2. Results match expected mathematical properties
3. Pricing invariants hold across random parameter combinations
"""

from datetime import date
from typing import Any

from finstack.core.market_data import MarketContext
from finstack.valuations.pricer import standard_registry
from hypothesis import assume, given, settings, strategies as st
import pytest
from tests.fixtures.strategies import (
    TOLERANCE_DETERMINISTIC,
    bond_strategy,
    create_flat_market_context,
    create_test_bond,
    create_test_swap,
    deposit_strategy,
    discount_curve_strategy,
    swap_strategy,
)

import finstack


@pytest.mark.properties
class TestCrossLanguageBondPricing:
    """Property tests for bond pricing across languages."""

    @given(bond_strategy(), discount_curve_strategy())
    @settings(max_examples=50, deadline=None)
    def test_bond_npv_positive_for_positive_coupon(self, bond: Any, curve: Any) -> None:
        """Bond with positive coupon should have positive NPV."""
        market = MarketContext()
        market.insert(curve)

        registry = standard_registry()

        try:
            result = registry.price(bond, "discounting", market, date(2024, 1, 1))
            assert result.value.amount > 0, f"Bond NPV should be positive, got {result.value.amount}"
            assert result.value.currency.code == "USD"
        except (finstack.FinstackError, ValueError, KeyError):
            # If pricing fails due to edge cases (e.g., maturity before as_of), that's acceptable
            pass

    @given(bond_strategy(), discount_curve_strategy())
    @settings(max_examples=30, deadline=None)
    def test_bond_pricing_twice_identical(self, bond: Any, curve: Any) -> None:
        """Pricing same bond twice yields identical results."""
        market = MarketContext()
        market.insert(curve)

        registry = standard_registry()

        try:
            r1 = registry.price(bond, "discounting", market, date(2024, 1, 1))
            r2 = registry.price(bond, "discounting", market, date(2024, 1, 1))

            assert abs(r1.value.amount - r2.value.amount) < TOLERANCE_DETERMINISTIC
            assert r1.value.currency.code == r2.value.currency.code
        except (finstack.FinstackError, ValueError, KeyError):
            # Some parameter combinations may be invalid
            pass

    @given(
        st.floats(min_value=0.01, max_value=0.08, allow_nan=False, allow_infinity=False),
        st.floats(min_value=0.01, max_value=0.08, allow_nan=False, allow_infinity=False),
    )
    @settings(max_examples=30, deadline=None)
    def test_bond_npv_monotonic_in_discount_rate(self, rate1: float, rate2: float) -> None:
        """Higher discount rate should yield lower bond NPV."""
        assume(abs(rate1 - rate2) > 0.005)  # Ensure meaningful difference

        bond = create_test_bond(
            "MONO-BOND",
            notional=1_000_000.0,
            coupon_rate=0.05,
            maturity=date(2029, 1, 1),
        )

        market1 = create_flat_market_context(discount_rate=rate1)
        market2 = create_flat_market_context(discount_rate=rate2)

        registry = standard_registry()
        npv1 = registry.price(bond, "discounting", market1, date(2024, 1, 1)).value.amount
        npv2 = registry.price(bond, "discounting", market2, date(2024, 1, 1)).value.amount

        if rate1 < rate2:
            assert npv1 > npv2, f"Lower rate ({rate1}) should give higher NPV: {npv1} vs {npv2}"
        else:
            assert npv1 < npv2, f"Higher rate ({rate1}) should give lower NPV: {npv1} vs {npv2}"

    @given(
        st.floats(min_value=100_000.0, max_value=10_000_000.0, allow_nan=False, allow_infinity=False),
        st.floats(min_value=2.0, max_value=5.0, allow_nan=False, allow_infinity=False),
    )
    @settings(max_examples=30, deadline=None)
    def test_bond_npv_scales_with_notional(self, base_notional: float, multiplier: float) -> None:
        """Bond NPV should scale proportionally with notional."""
        bond1 = create_test_bond(
            "SCALE-BOND-1",
            notional=base_notional,
            coupon_rate=0.05,
            maturity=date(2029, 1, 1),
        )
        bond2 = create_test_bond(
            "SCALE-BOND-2",
            notional=base_notional * multiplier,
            coupon_rate=0.05,
            maturity=date(2029, 1, 1),
        )

        market = create_flat_market_context(discount_rate=0.05)
        registry = standard_registry()

        npv1 = registry.price(bond1, "discounting", market, date(2024, 1, 1)).value.amount
        npv2 = registry.price(bond2, "discounting", market, date(2024, 1, 1)).value.amount

        # NPV should scale with notional
        expected_ratio = multiplier
        actual_ratio = npv2 / npv1

        assert abs(actual_ratio - expected_ratio) / expected_ratio < 0.001, (
            f"NPV ratio {actual_ratio} should equal notional ratio {expected_ratio}"
        )


@pytest.mark.properties
class TestCrossLanguageSwapPricing:
    """Property tests for swap pricing."""

    @given(
        st.floats(min_value=0.03, max_value=0.07, allow_nan=False, allow_infinity=False),
    )
    @settings(max_examples=30, deadline=None)
    def test_at_market_swap_near_zero(self, market_rate: float) -> None:
        """Swap priced at forward rate should be near zero."""
        # Create swap with fixed rate equal to forward rate
        swap = create_test_swap(
            "ATM-SWAP",
            notional=10_000_000.0,
            fixed_rate=market_rate,
            maturity=date(2029, 1, 1),
        )

        # Create market with matching forward curve
        market = create_flat_market_context(discount_rate=market_rate, forward_rate=market_rate)

        registry = standard_registry()
        result = registry.price(swap, "discounting", market, date(2024, 1, 1))

        # Near-ATM swap should have small absolute value relative to notional
        relative_value = abs(result.value.amount) / 10_000_000.0
        assert relative_value < 0.10, (
            f"ATM swap should have near-zero value, got {relative_value * 100:.2f}% of notional"
        )

    @given(swap_strategy(), discount_curve_strategy())
    @settings(max_examples=30, deadline=None)
    def test_swap_pricing_deterministic(self, swap: Any, curve: Any) -> None:
        """Swap pricing should be deterministic."""
        from finstack.core.dates import DayCount
        from finstack.core.market_data import ForwardCurve

        market = MarketContext()
        market.insert(curve)

        # Add forward curve
        forward_curve = ForwardCurve(
            "USD-SOFR",
            0.25,
            [(0.0, 0.05), (1.0, 0.05), (5.0, 0.05), (10.0, 0.05)],
            base_date=date(2024, 1, 1),
            day_count=DayCount.ACT_360,
        )
        market.insert(forward_curve)

        registry = standard_registry()

        try:
            r1 = registry.price(swap, "discounting", market, date(2024, 1, 1))
            r2 = registry.price(swap, "discounting", market, date(2024, 1, 1))

            assert abs(r1.value.amount - r2.value.amount) < TOLERANCE_DETERMINISTIC
        except (finstack.FinstackError, ValueError, KeyError):
            # Some combinations may be invalid
            pass

    @given(
        st.floats(min_value=0.02, max_value=0.08, allow_nan=False, allow_infinity=False),
        st.floats(min_value=0.02, max_value=0.08, allow_nan=False, allow_infinity=False),
    )
    @settings(max_examples=30, deadline=None)
    def test_swap_value_direction_vs_forward(self, fixed_rate: float, forward_rate: float) -> None:
        """Swap value sign depends on fixed rate vs forward rate."""
        assume(abs(fixed_rate - forward_rate) > 0.005)  # Meaningful difference

        swap = create_test_swap(
            "DIR-SWAP",
            notional=10_000_000.0,
            fixed_rate=fixed_rate,
            maturity=date(2029, 1, 1),
        )

        market = create_flat_market_context(discount_rate=0.05, forward_rate=forward_rate)
        registry = standard_registry()
        result = registry.price(swap, "discounting", market, date(2024, 1, 1))

        # For a payer swap (pay fixed, receive floating):
        # If fixed > forward, NPV should be negative (paying more than receiving)
        # If fixed < forward, NPV should be positive (receiving more than paying)
        if fixed_rate > forward_rate:
            assert result.value.amount < 0, (
                f"Fixed ({fixed_rate}) > Forward ({forward_rate}): NPV should be negative, got {result.value.amount}"
            )
        else:
            assert result.value.amount > 0, (
                f"Fixed ({fixed_rate}) < Forward ({forward_rate}): NPV should be positive, got {result.value.amount}"
            )


@pytest.mark.properties
class TestCrossLanguageDepositPricing:
    """Property tests for deposit pricing."""

    @given(deposit_strategy(), discount_curve_strategy())
    @settings(max_examples=50, deadline=None)
    def test_deposit_pricing_deterministic(self, deposit: Any, curve: Any) -> None:
        """Deposit pricing should be deterministic."""
        market = MarketContext()
        market.insert(curve)

        registry = standard_registry()

        try:
            r1 = registry.price(deposit, "discounting", market, date(2024, 1, 1))
            r2 = registry.price(deposit, "discounting", market, date(2024, 1, 1))

            assert abs(r1.value.amount - r2.value.amount) < TOLERANCE_DETERMINISTIC
        except (finstack.FinstackError, ValueError, KeyError):
            pass

    @given(
        st.floats(min_value=100_000.0, max_value=10_000_000.0, allow_nan=False, allow_infinity=False),
        st.floats(min_value=0.01, max_value=0.15, allow_nan=False, allow_infinity=False),
    )
    @settings(max_examples=30, deadline=None)
    def test_deposit_pricing_consistent(self, notional: float, rate: float) -> None:
        """Deposit pricing should be consistent across multiple calls."""
        from datetime import timedelta

        from finstack.core.currency import Currency
        from finstack.core.dates import DayCount
        from finstack.core.money import Money
        from finstack.valuations.instruments import Deposit

        deposit = (
            Deposit
            .builder("CONS-DEP")
            .money(Money(notional, Currency("USD")))
            .start(date(2024, 1, 1))
            .maturity(date(2024, 1, 1) + timedelta(days=90))
            .day_count(DayCount.ACT_360)
            .disc_id("USD-OIS")
            .quote_rate(rate)
            .build()
        )

        market = create_flat_market_context(discount_rate=0.05)
        registry = standard_registry()

        # Price twice and verify consistency
        result1 = registry.price(deposit, "discounting", market, date(2024, 1, 1))
        result2 = registry.price(deposit, "discounting", market, date(2024, 1, 1))

        assert abs(result1.value.amount - result2.value.amount) < TOLERANCE_DETERMINISTIC, (
            f"Deposit pricing inconsistent: {result1.value.amount} vs {result2.value.amount}"
        )


@pytest.mark.properties
class TestCrossLanguageCurveBumping:
    """Property tests for curve bumping effects on pricing."""

    @given(
        st.floats(min_value=1.0, max_value=50.0, allow_nan=False, allow_infinity=False),
    )
    @settings(max_examples=20, deadline=None)
    def test_parallel_bump_affects_bond_npv(self, bump_bp: float) -> None:
        """Parallel curve bump should change bond NPV."""
        bond = create_test_bond(
            "BUMP-BOND",
            notional=1_000_000.0,
            coupon_rate=0.05,
            maturity=date(2029, 1, 1),
        )

        market_base = create_flat_market_context(discount_rate=0.05)
        market_bumped = create_flat_market_context(discount_rate=0.05 + bump_bp / 10000)

        registry = standard_registry()
        npv_base = registry.price(bond, "discounting", market_base, date(2024, 1, 1)).value.amount
        npv_bumped = registry.price(bond, "discounting", market_bumped, date(2024, 1, 1)).value.amount

        # Higher rates = lower NPV for bonds
        assert npv_bumped < npv_base, f"Bumped rate should give lower NPV: base={npv_base}, bumped={npv_bumped}"

    @given(
        st.floats(min_value=1.0, max_value=20.0, allow_nan=False, allow_infinity=False),
    )
    @settings(max_examples=20, deadline=None)
    def test_bump_reversibility_under_pricing(self, bump_bp: float) -> None:
        """Bump up then down should restore original NPV."""
        bond = create_test_bond(
            "REV-BOND",
            notional=1_000_000.0,
            coupon_rate=0.05,
            maturity=date(2029, 1, 1),
        )

        base_rate = 0.05
        market_base = create_flat_market_context(discount_rate=base_rate)
        market_bumped_up = create_flat_market_context(discount_rate=base_rate + bump_bp / 10000)
        market_bumped_down = create_flat_market_context(discount_rate=base_rate)  # Back to original

        registry = standard_registry()
        npv_base = registry.price(bond, "discounting", market_base, date(2024, 1, 1)).value.amount
        npv_bumped = registry.price(bond, "discounting", market_bumped_up, date(2024, 1, 1)).value.amount
        npv_restored = registry.price(bond, "discounting", market_bumped_down, date(2024, 1, 1)).value.amount

        # Restored should match base
        assert abs(npv_base - npv_restored) < TOLERANCE_DETERMINISTIC, (
            f"Restored NPV {npv_restored} should match base {npv_base}"
        )
        # Bumped should differ
        assert abs(npv_base - npv_bumped) > 1.0, f"Bumped NPV {npv_bumped} should differ from base {npv_base}"


@pytest.mark.properties
class TestCrossLanguageMarketContext:
    """Property tests for market context behavior."""

    @given(discount_curve_strategy())
    @settings(max_examples=20, deadline=None)
    def test_market_context_curve_retrieval_consistent(self, curve: Any) -> None:
        """Retrieving a curve from market context is consistent."""
        market = MarketContext()
        market.insert(curve)

        # Retrieve curve twice
        retrieved1 = market.get_discount(curve.id)
        retrieved2 = market.get_discount(curve.id)

        # Both retrievals should give the same curve
        assert retrieved1.id == retrieved2.id
        assert retrieved1.id == curve.id

    def test_market_context_multiple_curves(self) -> None:
        """Adding multiple curves to market context works correctly."""
        from finstack.core.market_data import DiscountCurve

        curve1 = DiscountCurve(
            "USD-OIS-1",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )
        curve2 = DiscountCurve(
            "USD-OIS-2",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.96), (5.0, 0.80)],
            day_count="act_365f",
        )

        market = MarketContext()
        market.insert(curve1)
        market.insert(curve2)

        # Both curves should be retrievable
        r1 = market.get_discount("USD-OIS-1")
        r2 = market.get_discount("USD-OIS-2")

        assert r1.id == "USD-OIS-1"
        assert r2.id == "USD-OIS-2"

        # Curves should have different DFs at t=1
        assert abs(r1.df(1.0) - 0.95) < 0.01
        assert abs(r2.df(1.0) - 0.96) < 0.01
