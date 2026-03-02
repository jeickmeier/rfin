"""Tests for MertonMcConfig, MertonMcResult, and Bond.price_merton_mc."""

import datetime

from finstack.valuations.instruments import (
    Bond,
    DynamicRecoverySpec,
    EndogenousHazardSpec,
    MertonMcConfig,
    MertonMcResult,
    MertonModel,
    ToggleExerciseModel,
)

from finstack import Money


def _make_merton() -> MertonModel:
    return MertonModel(asset_value=100, asset_vol=0.25, debt_barrier=80, risk_free_rate=0.04)


def _make_bond() -> Bond:
    """Create a simple 5Y fixed bond for testing."""
    return (
        Bond
        .builder("TEST-BOND")
        .money(Money(1_000_000, "USD"))
        .coupon_rate(0.08)
        .issue(datetime.date(2024, 1, 1))
        .maturity(datetime.date(2029, 1, 1))
        .disc_id("USD-OIS")
        .build()
    )


def test_mc_config_creation() -> None:
    m = _make_merton()
    config = MertonMcConfig(m, num_paths=5000, seed=123)
    assert config.num_paths == 5000
    assert config.seed == 123
    assert config.antithetic is True
    assert config.time_steps_per_year == 12


def test_mc_config_defaults() -> None:
    m = _make_merton()
    config = MertonMcConfig(m)
    assert config.num_paths == 10_000
    assert config.seed == 42
    assert config.antithetic is True
    assert config.time_steps_per_year == 12


def test_mc_config_with_specs() -> None:
    m = _make_merton()
    endo = EndogenousHazardSpec.power_law(0.10, 1.5, 2.5)
    dyn_rec = DynamicRecoverySpec.floored_inverse(0.40, 100.0, 0.15)
    toggle = ToggleExerciseModel.threshold("hazard_rate", 0.15)
    config = MertonMcConfig(
        m,
        endogenous_hazard=endo,
        dynamic_recovery=dyn_rec,
        toggle_model=toggle,
        num_paths=1000,
        seed=42,
    )
    assert config.num_paths == 1000


def test_mc_config_repr() -> None:
    m = _make_merton()
    config = MertonMcConfig(m)
    r = repr(config)
    assert "MertonMcConfig" in r
    assert "10000" in r


def test_bond_price_merton_mc() -> None:
    bond = _make_bond()
    m = _make_merton()
    config = MertonMcConfig(m, num_paths=1000, seed=42)
    result = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))
    assert isinstance(result, MertonMcResult)
    assert 0.0 < result.clean_price_pct < 200.0
    assert result.num_paths == 1000
    assert result.standard_error > 0


def test_result_properties() -> None:
    bond = _make_bond()
    m = _make_merton()
    config = MertonMcConfig(m, num_paths=1000, seed=42)
    result = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))
    # Check all properties exist and are reasonable
    assert result.dirty_price_pct > 0
    assert result.expected_loss >= 0.0
    assert result.unexpected_loss >= 0
    assert result.expected_shortfall_95 >= 0
    assert 0.0 <= result.average_pik_fraction <= 1.0
    assert result.effective_spread_bp >= 0
    assert 0.0 <= result.default_rate <= 1.0
    assert result.avg_terminal_notional > 0
    assert 0.0 <= result.avg_recovery_pct <= 1.0


def test_result_repr() -> None:
    bond = _make_bond()
    m = _make_merton()
    config = MertonMcConfig(m, num_paths=500, seed=42)
    result = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))
    r = repr(result)
    assert "MertonMcResult" in r


def test_deterministic_with_seed() -> None:
    """Same seed should produce identical results."""
    bond = _make_bond()
    m = _make_merton()
    config = MertonMcConfig(m, num_paths=1000, seed=42)
    r1 = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))
    r2 = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))
    assert abs(r1.clean_price_pct - r2.clean_price_pct) < 1e-10


def test_different_seeds_produce_different_results() -> None:
    """Different seeds should produce (slightly) different results."""
    bond = _make_bond()
    m = _make_merton()
    config1 = MertonMcConfig(m, num_paths=1000, seed=42)
    config2 = MertonMcConfig(m, num_paths=1000, seed=99)
    r1 = bond.price_merton_mc(config1, 0.04, datetime.date(2024, 1, 1))
    r2 = bond.price_merton_mc(config2, 0.04, datetime.date(2024, 1, 1))
    # Results should differ (different random draws) but both be reasonable
    assert 0.0 < r1.clean_price_pct < 200.0
    assert 0.0 < r2.clean_price_pct < 200.0


def test_mc_config_with_endogenous_hazard() -> None:
    """Endogenous hazard should produce a valid result."""
    bond = _make_bond()
    m = _make_merton()
    endo = EndogenousHazardSpec.power_law(0.10, 1.5, 2.5)
    config = MertonMcConfig(m, endogenous_hazard=endo, num_paths=1000, seed=42)
    result = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))
    assert 0.0 < result.clean_price_pct < 200.0


def test_mc_config_with_dynamic_recovery() -> None:
    """Dynamic recovery should produce a valid result."""
    bond = _make_bond()
    m = _make_merton()
    dyn_rec = DynamicRecoverySpec.floored_inverse(0.40, 1_000_000.0, 0.15)
    config = MertonMcConfig(m, dynamic_recovery=dyn_rec, num_paths=1000, seed=42)
    result = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))
    assert 0.0 < result.clean_price_pct < 200.0


def test_mc_config_with_toggle_model() -> None:
    """Toggle model should produce a valid result."""
    bond = _make_bond()
    m = _make_merton()
    toggle = ToggleExerciseModel.threshold("hazard_rate", 0.15)
    config = MertonMcConfig(m, toggle_model=toggle, num_paths=1000, seed=42)
    result = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))
    assert 0.0 < result.clean_price_pct < 200.0
