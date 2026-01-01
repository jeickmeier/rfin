"""Property tests for curve reversibility invariants.

These tests verify that curve bumping operations are reversible:
- Bumping up then down restores original curve
- Bumping by +x then -x is identity
- Symmetric bumps preserve structure
- Multiple bump-unbump cycles don't accumulate errors
"""

from collections.abc import Callable
from datetime import date, timedelta
from typing import Any

from finstack.core.currency import Currency
from finstack.core.dates import DayCount
from finstack.core.market_data import DiscountCurve, MarketContext
from finstack.core.money import Money
from finstack.valuations.instruments import Deposit
from finstack.valuations.pricer import create_standard_registry
from hypothesis import assume, given, settings, strategies as st

# Strategies for generating test data
bump_sizes = st.floats(min_value=0.0001, max_value=0.05, allow_nan=False, allow_infinity=False)
small_bumps = st.floats(min_value=0.0001, max_value=0.01, allow_nan=False, allow_infinity=False)


@st.composite
def discount_curve_strategy(draw: Callable[[Any], Any]) -> DiscountCurve:
    """Generate valid discount curves for testing."""
    base_date = date(2024, 1, 1)
    curve_id = "USD-OIS"

    # Generate 5 pillar points with reasonable rates
    base_rate = draw(st.floats(min_value=0.02, max_value=0.08))
    dates = [base_date + timedelta(days=365 * i) for i in range(1, 6)]
    dfs = [1.0 / ((1.0 + base_rate) ** i) for i in range(1, 6)]

    day_count = DayCount.ACT_365F

    # Convert dates to time years using day_count
    knots = [(day_count.year_fraction(base_date, d, None), df) for d, df in zip(dates, dfs, strict=False)]

    return DiscountCurve(
        curve_id,
        base_date,
        knots,
        day_count=day_count,
    )


@st.composite
def deposit_strategy(draw: Callable[[Any], Any]) -> Deposit:
    """Generate valid deposits for testing."""
    start_date = date(2024, 1, 1)
    tenor_days = draw(st.integers(min_value=30, max_value=365))
    maturity_date = start_date + timedelta(days=tenor_days)

    notional = draw(st.floats(min_value=10000.0, max_value=1e6))
    rate = draw(st.floats(min_value=0.01, max_value=0.10))

    currency = Currency("USD")

    return Deposit(
        f"DEP-{tenor_days}D",
        Money(notional, currency),
        start_date,
        maturity_date,
        DayCount.ACT_360,
        "USD-OIS",
        quote_rate=rate,
    )


class TestCurveReversibility:
    """Property tests for curve bumping reversibility."""

    @given(discount_curve_strategy(), small_bumps, deposit_strategy())
    @settings(max_examples=30, deadline=None)
    def test_parallel_bump_reversible(self, curve: DiscountCurve, bump_bp: float, deposit: Deposit) -> None:
        """Bumping curve up then down restores original pricing."""
        # Setup market with original curve
        market_original = MarketContext()
        market_original.insert_discount(curve)
        registry = create_standard_registry()

        # Price with original curve
        result_original = registry.price_deposit(deposit, "discounting", market_original)
        pv_original = result_original.present_value.amount

        # Bump curve up
        bumped_up_curve = curve.bumped_parallel(bump_bp / 10000.0)  # Convert bp to decimal
        market_up = MarketContext()
        market_up.insert_discount(bumped_up_curve)
        registry.price_deposit(deposit, "discounting", market_up)

        # Bump curve down by same amount
        bumped_down_curve = bumped_up_curve.bumped_parallel(-bump_bp / 10000.0)
        market_down = MarketContext()
        market_down.insert_discount(bumped_down_curve)
        result_down = registry.price_deposit(deposit, "discounting", market_down)
        pv_down = result_down.present_value.amount

        # Result should be close to original (allowing for small numerical errors)
        relative_error = abs(pv_down - pv_original) / abs(pv_original) if abs(pv_original) > 1e-10 else 0
        assert relative_error < 1e-6, f"Original: {pv_original}, After bump cycle: {pv_down}, Error: {relative_error}"

    @given(discount_curve_strategy(), small_bumps)
    @settings(max_examples=30, deadline=None)
    def test_symmetric_bumps_cancel(self, curve: DiscountCurve, bump_bp: float) -> None:
        """Bumping by +x and -x in sequence cancels out."""
        # Bump up
        bumped_up = curve.bumped_parallel(bump_bp / 10000.0)

        # Bump down by same amount
        bumped_back = bumped_up.bumped_parallel(-bump_bp / 10000.0)

        # Check discount factors are restored
        original_dfs = curve.discount_factors()
        restored_dfs = bumped_back.discount_factors()

        for i, (orig, restored) in enumerate(zip(original_dfs, restored_dfs, strict=False)):
            relative_error = abs(restored - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            assert relative_error < 1e-6, f"Pillar {i}: original {orig}, restored {restored}, error {relative_error}"

    @given(discount_curve_strategy(), small_bumps, st.integers(min_value=2, max_value=5))
    @settings(max_examples=20, deadline=None)
    def test_multiple_bump_cycles_stable(self, curve: DiscountCurve, bump_bp: float, num_cycles: int) -> None:
        """Multiple bump-unbump cycles don't accumulate errors."""
        assume(num_cycles >= 2)

        current_curve = curve

        # Perform multiple bump cycles
        for _ in range(num_cycles):
            # Bump up
            current_curve = current_curve.bumped_parallel(bump_bp / 10000.0)
            # Bump down
            current_curve = current_curve.bumped_parallel(-bump_bp / 10000.0)

        # Check we're back to original
        original_dfs = curve.discount_factors()
        final_dfs = current_curve.discount_factors()

        for i, (orig, final) in enumerate(zip(original_dfs, final_dfs, strict=False)):
            # Allow slightly larger tolerance for multiple cycles
            relative_error = abs(final - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            assert relative_error < 1e-5, f"After {num_cycles} cycles - Pillar {i}: original {orig}, final {final}"

    @given(discount_curve_strategy(), small_bumps, deposit_strategy())
    @settings(max_examples=20, deadline=None)
    def test_bump_down_then_up_reversible(self, curve: DiscountCurve, bump_bp: float, deposit: Deposit) -> None:
        """Bumping down then up also restores original pricing."""
        # Setup
        market_original = MarketContext()
        market_original.insert_discount(curve)
        registry = create_standard_registry()

        # Price with original
        result_original = registry.price_deposit(deposit, "discounting", market_original)
        pv_original = result_original.present_value.amount

        # Bump down first
        bumped_down = curve.bumped_parallel(-bump_bp / 10000.0)
        market_down = MarketContext()
        market_down.insert_discount(bumped_down)
        registry.price_deposit(deposit, "discounting", market_down)

        # Then bump up
        bumped_up = bumped_down.bumped_parallel(bump_bp / 10000.0)
        market_up = MarketContext()
        market_up.insert_discount(bumped_up)
        result_up = registry.price_deposit(deposit, "discounting", market_up)
        pv_up = result_up.present_value.amount

        # Should restore original
        relative_error = abs(pv_up - pv_original) / abs(pv_original) if abs(pv_original) > 1e-10 else 0
        assert relative_error < 1e-6


class TestCurveBumpCommutativity:
    """Property tests for curve bump operation properties."""

    @given(discount_curve_strategy(), small_bumps, small_bumps)
    @settings(max_examples=20, deadline=None)
    def test_sequential_bumps_additive(self, curve: DiscountCurve, bump1_bp: float, bump2_bp: float) -> None:
        """Sequential bumps are additive: bump(a) then bump(b) = bump(a+b)."""
        # Path 1: Two sequential bumps
        bumped_once = curve.bumped_parallel(bump1_bp / 10000.0)
        bumped_twice = bumped_once.bumped_parallel(bump2_bp / 10000.0)

        # Path 2: Single combined bump
        bumped_combined = curve.bumped_parallel((bump1_bp + bump2_bp) / 10000.0)

        # Results should be equivalent
        dfs_twice = bumped_twice.discount_factors()
        dfs_combined = bumped_combined.discount_factors()

        for i, (df_twice, df_combined) in enumerate(zip(dfs_twice, dfs_combined, strict=False)):
            relative_error = abs(df_combined - df_twice) / abs(df_twice) if abs(df_twice) > 1e-10 else 0
            assert relative_error < 1e-6, f"Pillar {i}: sequential {df_twice}, combined {df_combined}"

    @given(discount_curve_strategy())
    @settings(max_examples=30, deadline=None)
    def test_zero_bump_identity(self, curve: DiscountCurve) -> None:
        """Bumping by zero is identity operation."""
        # Bump by zero
        bumped = curve.bumped_parallel(0.0)

        # Should be identical to original
        original_dfs = curve.discount_factors()
        bumped_dfs = bumped.discount_factors()

        for i, (orig, bump) in enumerate(zip(original_dfs, bumped_dfs, strict=False)):
            assert abs(orig - bump) < 1e-10, f"Pillar {i}: original {orig}, bumped {bump}"

    @given(discount_curve_strategy(), small_bumps)
    @settings(max_examples=20, deadline=None)
    def test_bump_preserves_curve_structure(self, curve: DiscountCurve, bump_bp: float) -> None:
        """Bumping preserves curve structure (dates, base_date, id)."""
        bumped = curve.bumped_parallel(bump_bp / 10000.0)

        # Check structure preserved
        assert bumped.base_date == curve.base_date
        assert bumped.id == curve.id

        # Dates should be preserved
        original_dates = curve.pillar_dates()
        bumped_dates = bumped.pillar_dates()
        assert original_dates == bumped_dates


class TestCurveStability:
    """Property tests for curve numerical stability."""

    @given(discount_curve_strategy(), st.lists(small_bumps, min_size=3, max_size=10))
    @settings(max_examples=15, deadline=None)
    def test_accumulated_bumps_stable(self, curve: DiscountCurve, bump_list: list[float]) -> None:
        """Accumulating bumps then reversing is stable."""
        if len(bump_list) < 3:
            return

        current_curve = curve
        total_bump = 0.0

        # Apply all bumps
        for bump_bp in bump_list:
            current_curve = current_curve.bumped_parallel(bump_bp / 10000.0)
            total_bump += bump_bp

        # Reverse total bump
        final_curve = current_curve.bumped_parallel(-total_bump / 10000.0)

        # Check restoration
        original_dfs = curve.discount_factors()
        final_dfs = final_curve.discount_factors()

        for i, (orig, final) in enumerate(zip(original_dfs, final_dfs, strict=False)):
            # Allow larger tolerance for accumulated operations
            relative_error = abs(final - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            assert relative_error < 1e-4, f"After {len(bump_list)} bumps - Pillar {i}: {orig} -> {final}"

    @given(discount_curve_strategy(), small_bumps, deposit_strategy())
    @settings(max_examples=20, deadline=None)
    def test_bump_magnitude_affects_pricing(self, curve: DiscountCurve, bump_bp: float, deposit: Deposit) -> None:
        """Bumping curve by non-zero amount changes pricing."""
        assume(abs(bump_bp) > 0.001)  # Ensure meaningful bump

        # Setup
        market_original = MarketContext()
        market_original.insert_discount(curve)
        registry = create_standard_registry()

        # Price with original
        result_original = registry.price_deposit(deposit, "discounting", market_original)
        pv_original = result_original.present_value.amount

        # Bump and price
        bumped = curve.bumped_parallel(bump_bp / 10000.0)
        market_bumped = MarketContext()
        market_bumped.insert_discount(bumped)
        result_bumped = registry.price_deposit(deposit, "discounting", market_bumped)
        pv_bumped = result_bumped.present_value.amount

        # PV should change (assuming positive duration)
        # For positive bump, PV should generally decrease (for fixed income)
        assert abs(pv_bumped - pv_original) > 1e-6 * abs(pv_original), "Bumping curve should affect pricing"


class TestCurveInversion:
    """Property tests for curve bump inversion properties."""

    @given(discount_curve_strategy(), small_bumps)
    @settings(max_examples=30, deadline=None)
    def test_bump_inverse_exists(self, curve: DiscountCurve, bump_bp: float) -> None:
        """Every bump has an inverse that restores original."""
        # Apply bump
        bumped = curve.bumped_parallel(bump_bp / 10000.0)

        # Apply inverse
        restored = bumped.bumped_parallel(-bump_bp / 10000.0)

        # Check restoration via discount factors
        original_dfs = curve.discount_factors()
        restored_dfs = restored.discount_factors()

        for orig, rest in zip(original_dfs, restored_dfs, strict=False):
            relative_error = abs(rest - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            assert relative_error < 1e-6

    @given(discount_curve_strategy(), small_bumps, small_bumps)
    @settings(max_examples=20, deadline=None)
    def test_bump_order_independent_for_reversal(self, curve: DiscountCurve, bump1_bp: float, bump2_bp: float) -> None:
        """Order of applying and reversing bumps doesn't matter for net zero."""
        # Path 1: bump1, bump2, -bump1, -bump2
        path1 = curve
        path1 = path1.bumped_parallel(bump1_bp / 10000.0)
        path1 = path1.bumped_parallel(bump2_bp / 10000.0)
        path1 = path1.bumped_parallel(-bump1_bp / 10000.0)
        path1 = path1.bumped_parallel(-bump2_bp / 10000.0)

        # Path 2: bump2, bump1, -bump2, -bump1
        path2 = curve
        path2 = path2.bumped_parallel(bump2_bp / 10000.0)
        path2 = path2.bumped_parallel(bump1_bp / 10000.0)
        path2 = path2.bumped_parallel(-bump2_bp / 10000.0)
        path2 = path2.bumped_parallel(-bump1_bp / 10000.0)

        # Both should restore original
        original_dfs = curve.discount_factors()
        path1_dfs = path1.discount_factors()
        path2_dfs = path2.discount_factors()

        for i, (orig, df1, df2) in enumerate(zip(original_dfs, path1_dfs, path2_dfs, strict=False)):
            error1 = abs(df1 - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            error2 = abs(df2 - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            assert error1 < 1e-5, f"Path 1 pillar {i} error: {error1}"
            assert error2 < 1e-5, f"Path 2 pillar {i} error: {error2}"
