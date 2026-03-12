from __future__ import annotations

from pathlib import Path

import pytest

from finstack.core import volatility


def test_linalg_stub_does_not_advertise_tolerance_parameter() -> None:
    """The published linalg stub should match the Rust-backed runtime signature."""
    stub_path = Path(__file__).resolve().parent.parent / "finstack" / "core" / "math" / "linalg.pyi"

    stub_text = stub_path.read_text()

    assert "validate_correlation_matrix(matrix: List[List[float]]) -> bool" in stub_text
    assert "tolerance" not in stub_text


def test_volatility_initial_guess_helpers_are_exported() -> None:
    """Core volatility helper approximations should be available from Python."""
    bs = volatility.brenner_subrahmanyam_approx(100.0, 100.0, 8.0, 1.0)
    mk = volatility.manaster_koehler_approx(100.0, 110.0, 1.0)
    near_atm_guess = volatility.implied_vol_initial_guess(100.0, 110.0, 5.0, 1.0)
    far_otm_guess = volatility.implied_vol_initial_guess(100.0, 140.0, 5.0, 1.0)
    far_otm_bs = volatility.brenner_subrahmanyam_approx(100.0, 140.0, 5.0, 1.0)
    far_otm_mk = volatility.manaster_koehler_approx(100.0, 140.0, 1.0)

    assert near_atm_guess == pytest.approx(volatility.brenner_subrahmanyam_approx(100.0, 110.0, 5.0, 1.0))
    assert far_otm_guess == pytest.approx((far_otm_bs + far_otm_mk) / 2.0)
    assert 0.01 <= bs <= 5.0
    assert 0.01 <= mk <= 5.0


def test_top_level_stub_declares_exception_hierarchy() -> None:
    """The published top-level stub should advertise the shared exception hierarchy."""
    stub_path = Path(__file__).resolve().parent.parent / "finstack" / "__init__.pyi"

    stub_text = stub_path.read_text()

    assert "class FinstackError(Exception):" in stub_text
    assert "class ValidationError(FinstackError):" in stub_text
    assert "class ParameterError(ValidationError):" in stub_text
    assert "class ConstraintValidationError(ParameterError):" in stub_text
    assert "class CholeskyError(ParameterError):" in stub_text
