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
from finstack.core.market_data import DiscountCurve
from finstack.core.money import Money
from finstack.valuations.instruments import Deposit
from hypothesis import assume, given, settings, strategies as st

# Strategies for generating test data
# Values are in BASIS POINTS (1 = 0.01%, 100 = 1%)
bump_sizes = st.floats(min_value=1.0, max_value=50.0, allow_nan=False, allow_infinity=False)
small_bumps = st.floats(min_value=0.1, max_value=10.0, allow_nan=False, allow_infinity=False)


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

    @given(discount_curve_strategy(), small_bumps)
    @settings(max_examples=30, deadline=None)
    def test_parallel_bump_reversible(self, curve: DiscountCurve, bump_bp: float) -> None:
        """Bumping curve up then down restores original discount factors."""
        # Bump curve up (bump_bp is in basis points, Rust handles conversion)
        bumped_up_curve = curve.bumped_parallel(bump_bp)

        # Bump curve down by same amount
        bumped_down_curve = bumped_up_curve.bumped_parallel(-bump_bp)

        # Discount factors should be restored to original
        original_dfs = curve.discount_factors
        restored_dfs = bumped_down_curve.discount_factors

        for i, (orig, restored) in enumerate(zip(original_dfs, restored_dfs, strict=False)):
            relative_error = abs(restored - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            assert relative_error < 1e-6, (
                f"Pillar {i}: Original {orig}, After bump cycle: {restored}, Error: {relative_error}"
            )

    @given(discount_curve_strategy(), small_bumps)
    @settings(max_examples=30, deadline=None)
    def test_symmetric_bumps_cancel(self, curve: DiscountCurve, bump_bp: float) -> None:
        """Bumping by +x and -x in sequence cancels out."""
        # Bump up (bump_bp is in basis points)
        bumped_up = curve.bumped_parallel(bump_bp)

        # Bump down by same amount
        bumped_back = bumped_up.bumped_parallel(-bump_bp)

        # Check discount factors are restored
        original_dfs = curve.discount_factors
        restored_dfs = bumped_back.discount_factors

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
            # Bump up (bump_bp is in basis points)
            current_curve = current_curve.bumped_parallel(bump_bp)
            # Bump down
            current_curve = current_curve.bumped_parallel(-bump_bp)

        # Check we're back to original
        original_dfs = curve.discount_factors
        final_dfs = current_curve.discount_factors

        for i, (orig, final) in enumerate(zip(original_dfs, final_dfs, strict=False)):
            # Allow slightly larger tolerance for multiple cycles
            relative_error = abs(final - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            assert relative_error < 1e-5, f"After {num_cycles} cycles - Pillar {i}: original {orig}, final {final}"

    @given(discount_curve_strategy(), small_bumps)
    @settings(max_examples=20, deadline=None)
    def test_bump_down_then_up_reversible(self, curve: DiscountCurve, bump_bp: float) -> None:
        """Bumping down then up also restores original discount factors."""
        # Bump down first (bump_bp is in basis points)
        bumped_down = curve.bumped_parallel(-bump_bp)

        # Then bump up
        bumped_up = bumped_down.bumped_parallel(bump_bp)

        # Should restore original discount factors
        original_dfs = curve.discount_factors
        restored_dfs = bumped_up.discount_factors

        for i, (orig, restored) in enumerate(zip(original_dfs, restored_dfs, strict=False)):
            relative_error = abs(restored - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            assert relative_error < 1e-6, f"Pillar {i}: Original {orig}, Restored: {restored}"


class TestCurveBumpCommutativity:
    """Property tests for curve bump operation properties."""

    @given(discount_curve_strategy(), small_bumps, small_bumps)
    @settings(max_examples=20, deadline=None)
    def test_sequential_bumps_additive(self, curve: DiscountCurve, bump1_bp: float, bump2_bp: float) -> None:
        """Sequential bumps are additive: bump(a) then bump(b) = bump(a+b)."""
        # Path 1: Two sequential bumps (values are in basis points)
        bumped_once = curve.bumped_parallel(bump1_bp)
        bumped_twice = bumped_once.bumped_parallel(bump2_bp)

        # Path 2: Single combined bump
        bumped_combined = curve.bumped_parallel(bump1_bp + bump2_bp)

        # Results should be equivalent
        dfs_twice = bumped_twice.discount_factors
        dfs_combined = bumped_combined.discount_factors

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
        original_dfs = curve.discount_factors
        bumped_dfs = bumped.discount_factors

        for i, (orig, bump) in enumerate(zip(original_dfs, bumped_dfs, strict=False)):
            assert abs(orig - bump) < 1e-10, f"Pillar {i}: original {orig}, bumped {bump}"

    @given(discount_curve_strategy(), small_bumps)
    @settings(max_examples=20, deadline=None)
    def test_bump_preserves_curve_structure(self, curve: DiscountCurve, bump_bp: float) -> None:
        """Bumping preserves curve structure (base_date, pillar times)."""
        bumped = curve.bumped_parallel(bump_bp)

        # Check base date is preserved
        assert bumped.base_date == curve.base_date

        # Curve ID gets a bump suffix (e.g., USD-OIS -> USD-OIS_bump_Xbp)
        # This is expected behavior in Rust implementation
        assert curve.id in bumped.id  # Original ID is prefix of bumped ID

        # Pillar times should be preserved (use points property)
        original_times = [t for t, _ in curve.points]
        bumped_times = [t for t, _ in bumped.points]
        assert original_times == bumped_times


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
            current_curve = current_curve.bumped_parallel(bump_bp)
            total_bump += bump_bp

        # Reverse total bump
        final_curve = current_curve.bumped_parallel(-total_bump)

        # Check restoration
        original_dfs = curve.discount_factors
        final_dfs = final_curve.discount_factors

        for i, (orig, final) in enumerate(zip(original_dfs, final_dfs, strict=False)):
            # Allow larger tolerance for accumulated operations
            relative_error = abs(final - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            assert relative_error < 1e-4, f"After {len(bump_list)} bumps - Pillar {i}: {orig} -> {final}"

    @given(discount_curve_strategy(), bump_sizes)  # Use bump_sizes which has larger values
    @settings(max_examples=20, deadline=None)
    def test_bump_magnitude_affects_discount_factors(self, curve: DiscountCurve, bump_bp: float) -> None:
        """Bumping curve by non-zero amount changes discount factors."""
        assume(abs(bump_bp) > 0.001)  # Ensure meaningful bump

        # Bump the curve - note: bumped_parallel takes bump in decimal form already
        # The function internally divides by 10_000 so we pass bp directly
        bumped = curve.bumped_parallel(bump_bp)

        # Discount factors should be different
        original_dfs = curve.discount_factors
        bumped_dfs = bumped.discount_factors

        # At least one pillar should have different DF
        differences_found = False
        for orig, bump in zip(original_dfs, bumped_dfs, strict=False):
            if abs(bump - orig) > 1e-12:
                differences_found = True
                break
        assert differences_found, "Bumping curve should affect discount factors"


class TestCurveInversion:
    """Property tests for curve bump inversion properties."""

    @given(discount_curve_strategy(), small_bumps)
    @settings(max_examples=30, deadline=None)
    def test_bump_inverse_exists(self, curve: DiscountCurve, bump_bp: float) -> None:
        """Every bump has an inverse that restores original."""
        # Apply bump (values are in basis points)
        bumped = curve.bumped_parallel(bump_bp)

        # Apply inverse
        restored = bumped.bumped_parallel(-bump_bp)

        # Check restoration via discount factors
        original_dfs = curve.discount_factors
        restored_dfs = restored.discount_factors

        for orig, rest in zip(original_dfs, restored_dfs, strict=False):
            relative_error = abs(rest - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            assert relative_error < 1e-6

    @given(discount_curve_strategy(), small_bumps, small_bumps)
    @settings(max_examples=20, deadline=None)
    def test_bump_order_independent_for_reversal(self, curve: DiscountCurve, bump1_bp: float, bump2_bp: float) -> None:
        """Order of applying and reversing bumps doesn't matter for net zero."""
        # Path 1: bump1, bump2, -bump1, -bump2
        path1 = curve
        path1 = path1.bumped_parallel(bump1_bp)
        path1 = path1.bumped_parallel(bump2_bp)
        path1 = path1.bumped_parallel(-bump1_bp)
        path1 = path1.bumped_parallel(-bump2_bp)

        # Path 2: bump2, bump1, -bump2, -bump1
        path2 = curve
        path2 = path2.bumped_parallel(bump2_bp)
        path2 = path2.bumped_parallel(bump1_bp)
        path2 = path2.bumped_parallel(-bump2_bp)
        path2 = path2.bumped_parallel(-bump1_bp)

        # Both should restore original
        original_dfs = curve.discount_factors
        path1_dfs = path1.discount_factors
        path2_dfs = path2.discount_factors

        for i, (orig, df1, df2) in enumerate(zip(original_dfs, path1_dfs, path2_dfs, strict=False)):
            error1 = abs(df1 - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            error2 = abs(df2 - orig) / abs(orig) if abs(orig) > 1e-10 else 0
            assert error1 < 1e-5, f"Path 1 pillar {i} error: {error1}"
            assert error2 < 1e-5, f"Path 2 pillar {i} error: {error2}"
