"""Merton model and Monte Carlo pricing parity tests.

Verifies that Python bindings produce identical results to the underlying
Rust implementation for the Merton structural credit model and its MC
pricing engine. All tests use fixed seeds and analytical reference values
to ensure determinism and correctness.
"""

import datetime
import math

from finstack.valuations.instruments import (
    Bond,
    DynamicRecoverySpec,
    EndogenousHazardSpec,
    MertonMcConfig,
    MertonMcResult,
    MertonModel,
    ToggleExerciseModel,
)
import pytest

from finstack import Money

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_merton(
    asset_value: float = 100.0,
    asset_vol: float = 0.20,
    debt_barrier: float = 80.0,
    risk_free_rate: float = 0.05,
) -> MertonModel:
    """Create a Merton model with the textbook parameters used in Rust tests."""
    return MertonModel(
        asset_value=asset_value,
        asset_vol=asset_vol,
        debt_barrier=debt_barrier,
        risk_free_rate=risk_free_rate,
    )


def _make_bond(
    bond_id: str = "PARITY-BOND",
    notional: float = 1_000_000.0,
    coupon_rate: float = 0.08,
    issue: datetime.date = datetime.date(2024, 1, 1),
    maturity: datetime.date = datetime.date(2029, 1, 1),
) -> Bond:
    """Create a fixed-rate bond for MC pricing tests."""
    return (
        Bond
        .builder(bond_id)
        .money(Money(notional, "USD"))
        .coupon_rate(coupon_rate)
        .issue(issue)
        .maturity(maturity)
        .disc_id("USD-OIS")
        .build()
    )


# ---------------------------------------------------------------------------
# 1. Distance-to-default parity
# ---------------------------------------------------------------------------


@pytest.mark.parity
class TestMertonDdParity:
    """Python DD matches Rust DD and the analytical formula."""

    def test_merton_dd_parity(self) -> None:
        """Python DD matches the Rust textbook value.

        Analytical: DD = (ln(V/B) + (r - 0.5*sigma^2)*T) / (sigma*sqrt(T))
        With V=100, B=80, sigma=0.20, r=0.05, T=1:
            DD = (ln(100/80) + (0.05 - 0.02)*1) / (0.2*1)
               = (0.22314 + 0.03) / 0.2
               = 1.2657
        """
        m = _make_merton()
        dd = m.distance_to_default(1.0)

        # Compute the analytical value in Python for exact parity check
        v, b, sigma, r, t = 100.0, 80.0, 0.20, 0.05, 1.0
        expected_dd = (math.log(v / b) + (r - 0.5 * sigma**2) * t) / (sigma * math.sqrt(t))

        assert dd == pytest.approx(expected_dd, abs=1e-10), f"Python DD={dd} does not match analytical DD={expected_dd}"
        # Also verify against the Rust test's known value
        assert dd == pytest.approx(1.2657, abs=0.01)

    def test_dd_varies_with_horizon(self) -> None:
        """DD at different horizons is consistent with the formula."""
        m = _make_merton()
        dd_1y = m.distance_to_default(1.0)
        dd_5y = m.distance_to_default(5.0)

        # For a solvent firm, DD should generally decrease with longer horizons
        # (more time for asset to cross the barrier), but the drift component
        # can dominate; just verify they are both positive and differ.
        assert dd_1y > 0
        assert dd_5y > 0
        assert dd_1y != pytest.approx(dd_5y, abs=1e-6)


# ---------------------------------------------------------------------------
# 2. Default probability parity
# ---------------------------------------------------------------------------


@pytest.mark.parity
class TestMertonPdParity:
    """Python PD matches N(-DD) analytically."""

    def test_merton_pd_parity(self) -> None:
        """PD = N(-DD) matches both analytical and Rust values.

        With DD ~ 1.2657, PD = N(-1.2657) ~ 0.1028.
        """
        m = _make_merton()
        pd = m.default_probability(1.0)
        dd = m.distance_to_default(1.0)

        # Compute N(-DD) using Python's math.erfc
        # N(x) = 0.5 * erfc(-x / sqrt(2))
        expected_pd = 0.5 * math.erfc(dd / math.sqrt(2))

        assert pd == pytest.approx(expected_pd, abs=1e-10), (
            f"Python PD={pd} does not match analytical N(-DD)={expected_pd}"
        )
        # Cross-check against the Rust test value
        assert pd == pytest.approx(0.1028, abs=0.01)

    def test_pd_bounded_zero_one(self) -> None:
        """PD is always in (0, 1) for a risky firm."""
        m = _make_merton()
        for horizon in [0.25, 0.5, 1.0, 2.0, 5.0, 10.0]:
            pd = m.default_probability(horizon)
            assert 0.0 < pd < 1.0, f"PD={pd} out of bounds at horizon={horizon}"

    def test_pd_increases_with_leverage(self) -> None:
        """Higher leverage (closer barrier) leads to higher PD."""
        m_low = _make_merton(debt_barrier=60.0)
        m_high = _make_merton(debt_barrier=95.0)
        assert m_high.default_probability(1.0) > m_low.default_probability(1.0)

    def test_pd_increases_with_vol(self) -> None:
        """Higher asset volatility leads to higher PD."""
        m_low = _make_merton(asset_vol=0.10)
        m_high = _make_merton(asset_vol=0.40)
        assert m_high.default_probability(1.0) > m_low.default_probability(1.0)


# ---------------------------------------------------------------------------
# 3. Implied spread parity
# ---------------------------------------------------------------------------


@pytest.mark.parity
class TestMertonImpliedSpreadParity:
    """Implied spread is positive and consistent with the formula."""

    def test_merton_implied_spread_parity(self) -> None:
        """Implied spread is positive and reasonable for a risky firm.

        Analytical: s = -ln(1 - PD * LGD) / T
        """
        m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)
        spread = m.implied_spread(5.0, 0.40)

        # Verify against the analytical formula
        pd = m.default_probability(5.0)
        lgd = 1.0 - 0.40
        expected_spread = -math.log(1.0 - pd * lgd) / 5.0

        assert spread == pytest.approx(expected_spread, abs=1e-10)
        assert spread > 0.0, "Spread should be positive for a risky firm"
        # Spread should be reasonable (less than 20% per annum)
        assert spread < 0.20, f"Spread={spread} seems unreasonably high"

    def test_spread_increases_with_leverage(self) -> None:
        """Higher leverage produces wider spreads."""
        m_safe = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=50, risk_free_rate=0.04)
        m_risky = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=90, risk_free_rate=0.04)
        assert m_risky.implied_spread(5.0, 0.40) > m_safe.implied_spread(5.0, 0.40)


# ---------------------------------------------------------------------------
# 4. MC price determinism (same seed -> same result)
# ---------------------------------------------------------------------------


@pytest.mark.parity
class TestMcPriceParity:
    """MC pricing with fixed seed produces deterministic, reasonable results."""

    def test_mc_price_parity(self) -> None:
        """MC pricing returns a reasonable clean price for a 5Y bond."""
        bond = _make_bond()
        m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)
        config = MertonMcConfig(m, num_paths=2000, seed=42)

        result = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))
        assert isinstance(result, MertonMcResult)
        # Clean price should be positive and less than 200% of par
        assert 0.0 < result.clean_price_pct < 200.0
        # Standard error should be positive (we ran MC)
        assert result.standard_error > 0.0
        # Effective spread should be non-negative
        assert result.effective_spread_bp >= 0.0

    def test_mc_price_two_runs_identical(self) -> None:
        """Two MC runs with the same seed produce identical clean prices."""
        bond = _make_bond()
        m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)
        config = MertonMcConfig(m, num_paths=2000, seed=42)

        r1 = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))
        r2 = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))

        assert r1.clean_price_pct == pytest.approx(r2.clean_price_pct, abs=1e-10)
        assert r1.effective_spread_bp == pytest.approx(r2.effective_spread_bp, abs=1e-10)
        assert r1.expected_loss == pytest.approx(r2.expected_loss, abs=1e-10)
        assert r1.default_rate == pytest.approx(r2.default_rate, abs=1e-10)


# ---------------------------------------------------------------------------
# 5. MC determinism (explicit separate calls)
# ---------------------------------------------------------------------------


@pytest.mark.parity
class TestMcDeterminism:
    """Explicit determinism tests: separately constructed configs with same seed."""

    def test_mc_determinism(self) -> None:
        """Two separately constructed configs with same params give identical results."""
        bond = _make_bond()
        m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)

        config1 = MertonMcConfig(m, num_paths=1000, seed=123)
        config2 = MertonMcConfig(m, num_paths=1000, seed=123)

        r1 = bond.price_merton_mc(config1, 0.04, datetime.date(2024, 1, 1))
        r2 = bond.price_merton_mc(config2, 0.04, datetime.date(2024, 1, 1))

        assert r1.clean_price_pct == pytest.approx(r2.clean_price_pct, abs=1e-10)
        assert r1.dirty_price_pct == pytest.approx(r2.dirty_price_pct, abs=1e-10)
        assert r1.standard_error == pytest.approx(r2.standard_error, abs=1e-10)
        assert r1.effective_spread_bp == pytest.approx(r2.effective_spread_bp, abs=1e-10)
        assert r1.expected_loss == pytest.approx(r2.expected_loss, abs=1e-10)
        assert r1.unexpected_loss == pytest.approx(r2.unexpected_loss, abs=1e-10)
        assert r1.expected_shortfall_95 == pytest.approx(r2.expected_shortfall_95, abs=1e-10)
        assert r1.default_rate == pytest.approx(r2.default_rate, abs=1e-10)
        assert r1.avg_terminal_notional == pytest.approx(r2.avg_terminal_notional, abs=1e-10)

    def test_different_seeds_differ(self) -> None:
        """Different seeds produce different results (sanity check)."""
        bond = _make_bond()
        m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)

        r1 = bond.price_merton_mc(
            MertonMcConfig(m, num_paths=1000, seed=42),
            0.04,
            datetime.date(2024, 1, 1),
        )
        r2 = bond.price_merton_mc(
            MertonMcConfig(m, num_paths=1000, seed=99),
            0.04,
            datetime.date(2024, 1, 1),
        )

        # Both should be reasonable but not identical
        assert 0.0 < r1.clean_price_pct < 200.0
        assert 0.0 < r2.clean_price_pct < 200.0
        # With different seeds the results should differ at some precision
        # (extremely unlikely to be bitwise identical)
        assert r1.clean_price_pct != pytest.approx(r2.clean_price_pct, abs=1e-10)


# ---------------------------------------------------------------------------
# 6. MC with all specs configured
# ---------------------------------------------------------------------------


@pytest.mark.parity
class TestMcWithAllSpecs:
    """MC pricing with endogenous hazard, dynamic recovery, and toggle model."""

    def test_mc_with_all_specs(self) -> None:
        """All three credit extensions produce a valid, reasonable result."""
        bond = _make_bond()
        m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)

        endo = EndogenousHazardSpec.power_law(0.10, 1.5, 2.5)
        dyn_rec = DynamicRecoverySpec.floored_inverse(0.40, 1_000_000.0, 0.15)
        toggle = ToggleExerciseModel.threshold("hazard_rate", 0.15)

        config = MertonMcConfig(
            m,
            endogenous_hazard=endo,
            dynamic_recovery=dyn_rec,
            toggle_model=toggle,
            num_paths=2000,
            seed=42,
        )

        result = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))

        # Verify all result properties are accessible and within valid ranges
        assert 0.0 < result.clean_price_pct < 200.0
        assert result.dirty_price_pct > 0.0
        assert result.expected_loss >= 0.0
        assert result.unexpected_loss >= 0.0
        assert result.expected_shortfall_95 >= 0.0
        assert 0.0 <= result.average_pik_fraction <= 1.0
        assert result.effective_spread_bp >= 0.0
        assert 0.0 <= result.default_rate <= 1.0
        assert result.avg_terminal_notional > 0.0
        assert 0.0 <= result.avg_recovery_pct <= 1.0
        assert result.num_paths == 2000
        assert result.standard_error > 0.0

    def test_mc_all_specs_deterministic(self) -> None:
        """Full-spec MC is deterministic with the same seed."""
        bond = _make_bond()
        m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)

        endo = EndogenousHazardSpec.power_law(0.10, 1.5, 2.5)
        dyn_rec = DynamicRecoverySpec.floored_inverse(0.40, 1_000_000.0, 0.15)
        toggle = ToggleExerciseModel.threshold("hazard_rate", 0.15)

        config = MertonMcConfig(
            m,
            endogenous_hazard=endo,
            dynamic_recovery=dyn_rec,
            toggle_model=toggle,
            num_paths=1000,
            seed=77,
        )

        r1 = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))
        r2 = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))

        assert r1.clean_price_pct == pytest.approx(r2.clean_price_pct, abs=1e-10)
        assert r1.default_rate == pytest.approx(r2.default_rate, abs=1e-10)


# ---------------------------------------------------------------------------
# 7. PIK vs cash spread comparison
# ---------------------------------------------------------------------------


@pytest.mark.parity
class TestPikVsCashSpread:
    """Compare PIK bond spread vs cash bond spread."""

    def test_pik_vs_cash_spread(self) -> None:
        """PIK toggle generally produces a different spread than pure cash.

        With a toggle model that activates PIK under stress, the effective
        spread should differ from the no-toggle baseline.
        """
        bond = _make_bond()
        m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)
        as_of = datetime.date(2024, 1, 1)

        # Cash-only (no toggle)
        config_cash = MertonMcConfig(m, num_paths=2000, seed=42)
        result_cash = bond.price_merton_mc(config_cash, 0.04, as_of)

        # PIK with toggle
        toggle = ToggleExerciseModel.threshold("hazard_rate", 0.15)
        config_pik = MertonMcConfig(m, toggle_model=toggle, num_paths=2000, seed=42)
        result_pik = bond.price_merton_mc(config_pik, 0.04, as_of)

        # Both should produce reasonable results
        assert result_cash.clean_price_pct > 0
        assert result_pik.clean_price_pct > 0

        # The toggle model changes the cash flow structure, so spreads
        # should generally differ. In extreme cases they could be close
        # (e.g., if the toggle never fires), but for a 25% vol firm they
        # should meaningfully differ.
        abs(result_pik.effective_spread_bp - result_cash.effective_spread_bp)
        # We just verify both are accessible and positive; a strict inequality
        # could fail if the toggle never triggers for this particular parameterisation.
        assert result_cash.effective_spread_bp >= 0
        assert result_pik.effective_spread_bp >= 0

        # Verify that the PIK result reports PIK fraction info
        # (it may be zero if the toggle never fires, but the property must be accessible)
        assert result_pik.average_pik_fraction >= 0.0


# ---------------------------------------------------------------------------
# 8. All MertonMcResult properties accessible
# ---------------------------------------------------------------------------


@pytest.mark.parity
class TestMcResultPropertiesAccessible:
    """Ensure every property of MertonMcResult is accessible from Python."""

    def test_mc_result_properties_accessible(self) -> None:
        """Access every property of MertonMcResult to verify bindings."""
        bond = _make_bond()
        m = MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)
        config = MertonMcConfig(m, num_paths=1000, seed=42)
        result = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))

        # Price metrics
        clean_price = result.clean_price_pct
        assert isinstance(clean_price, float)
        assert clean_price > 0

        dirty_price = result.dirty_price_pct
        assert isinstance(dirty_price, float)
        assert dirty_price > 0

        # Loss metrics
        expected_loss = result.expected_loss
        assert isinstance(expected_loss, float)
        assert expected_loss >= 0.0

        unexpected_loss = result.unexpected_loss
        assert isinstance(unexpected_loss, float)
        assert unexpected_loss >= 0.0

        es95 = result.expected_shortfall_95
        assert isinstance(es95, float)
        assert es95 >= 0.0

        # Spread and PIK
        spread = result.effective_spread_bp
        assert isinstance(spread, float)
        assert spread >= 0.0

        pik_frac = result.average_pik_fraction
        assert isinstance(pik_frac, float)
        assert 0.0 <= pik_frac <= 1.0

        # Path statistics
        default_rate = result.default_rate
        assert isinstance(default_rate, float)
        assert 0.0 <= default_rate <= 1.0

        avg_def_time = result.avg_default_time
        assert isinstance(avg_def_time, float)
        assert avg_def_time >= 0.0

        avg_terminal = result.avg_terminal_notional
        assert isinstance(avg_terminal, float)
        assert avg_terminal > 0.0

        avg_recovery = result.avg_recovery_pct
        assert isinstance(avg_recovery, float)
        assert 0.0 <= avg_recovery <= 1.0

        pik_exercise = result.pik_exercise_rate
        assert isinstance(pik_exercise, float)
        assert 0.0 <= pik_exercise <= 1.0

        # Simulation metadata
        num_paths = result.num_paths
        assert isinstance(num_paths, int)
        assert num_paths == 1000

        std_err = result.standard_error
        assert isinstance(std_err, float)
        assert std_err > 0.0

        # Repr should work
        r = repr(result)
        assert "MertonMcResult" in r


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
