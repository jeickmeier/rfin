from finstack.valuations.instruments import (
    EndogenousHazardSpec,
    DynamicRecoverySpec,
    ToggleExerciseModel,
)


# ---------------------------------------------------------------------------
# EndogenousHazardSpec tests
# ---------------------------------------------------------------------------


def test_power_law_at_base():
    spec = EndogenousHazardSpec.power_law(0.10, 1.5, 2.5)
    assert abs(spec.hazard_at_leverage(1.5) - 0.10) < 1e-10


def test_power_law_increases():
    spec = EndogenousHazardSpec.power_law(0.10, 1.5, 2.5)
    assert spec.hazard_at_leverage(2.0) > spec.hazard_at_leverage(1.5)


def test_exponential_at_base():
    spec = EndogenousHazardSpec.exponential(0.10, 1.5, 5.0)
    assert abs(spec.hazard_at_leverage(1.5) - 0.10) < 1e-10


def test_pik_accrual_increases_hazard():
    spec = EndogenousHazardSpec.power_law(0.10, 1.5, 2.5)
    h_before = spec.hazard_after_pik_accrual(100.0, 100.0, 66.67)
    h_after = spec.hazard_after_pik_accrual(100.0, 120.0, 66.67)
    assert h_after > h_before


def test_tabular():
    spec = EndogenousHazardSpec.tabular([1.0, 1.5, 2.0, 3.0], [0.02, 0.05, 0.12, 0.30])
    h = spec.hazard_at_leverage(1.75)
    assert 0.05 < h < 0.12


def test_endogenous_properties():
    spec = EndogenousHazardSpec.power_law(0.10, 1.5, 2.5)
    assert spec.base_hazard_rate == 0.10
    assert spec.base_leverage == 1.5
    assert "Power" in spec.name or "power" in spec.name


def test_endogenous_repr():
    spec = EndogenousHazardSpec.exponential(0.10, 1.5, 5.0)
    assert "EndogenousHazardSpec" in repr(spec)


# ---------------------------------------------------------------------------
# DynamicRecoverySpec tests
# ---------------------------------------------------------------------------


def test_constant_recovery():
    spec = DynamicRecoverySpec.constant(0.40)
    assert abs(spec.recovery_at_notional(150.0) - 0.40) < 1e-10
    assert abs(spec.recovery_at_notional(50.0) - 0.40) < 1e-10


def test_inverse_linear():
    spec = DynamicRecoverySpec.inverse_linear(0.40, 100.0)
    assert abs(spec.recovery_at_notional(100.0) - 0.40) < 1e-10
    assert spec.recovery_at_notional(150.0) < 0.40


def test_floored_inverse():
    spec = DynamicRecoverySpec.floored_inverse(0.40, 100.0, 0.15)
    assert abs(spec.recovery_at_notional(1000.0) - 0.15) < 1e-10


def test_linear_decline():
    spec = DynamicRecoverySpec.linear_decline(0.40, 100.0, 0.5, 0.10)
    r = spec.recovery_at_notional(120.0)
    assert abs(r - 0.36) < 1e-6


def test_inverse_power():
    spec = DynamicRecoverySpec.inverse_power(0.40, 100.0, 0.5)
    r = spec.recovery_at_notional(200.0)
    expected = 0.40 * (0.5**0.5)
    assert abs(r - expected) < 1e-6


def test_recovery_properties():
    spec = DynamicRecoverySpec.constant(0.40)
    assert spec.base_recovery == 0.40
    assert "Constant" in spec.name or "constant" in spec.name


def test_recovery_repr():
    spec = DynamicRecoverySpec.floored_inverse(0.40, 100.0, 0.15)
    assert "DynamicRecoverySpec" in repr(spec)


# ---------------------------------------------------------------------------
# ToggleExerciseModel tests
# ---------------------------------------------------------------------------


def test_threshold_above():
    model = ToggleExerciseModel.threshold("hazard_rate", 0.15)
    assert "Threshold" in model.name or "threshold" in model.name


def test_threshold_below():
    model = ToggleExerciseModel.threshold("distance_to_default", 2.0, direction="below")
    assert "Threshold" in model.name


def test_stochastic():
    model = ToggleExerciseModel.stochastic("hazard_rate", -3.0, 20.0)
    assert "Stochastic" in model.name or "stochastic" in model.name


def test_optimal_exercise():
    model = ToggleExerciseModel.optimal_exercise(nested_paths=200, equity_discount_rate=0.10)
    assert "Optimal" in model.name or "optimal" in model.name


def test_toggle_repr():
    model = ToggleExerciseModel.threshold("leverage", 2.0)
    assert "ToggleExerciseModel" in repr(model)


def test_toggle_invalid_variable():
    import pytest

    with pytest.raises(ValueError):
        ToggleExerciseModel.threshold("invalid_var", 0.15)


def test_toggle_invalid_direction():
    import pytest

    with pytest.raises(ValueError):
        ToggleExerciseModel.threshold("hazard_rate", 0.15, direction="sideways")
