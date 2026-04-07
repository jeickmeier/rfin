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


def test_statements_extensions_stub_reexports_runtime_config_types() -> None:
    """The statements extensions stub should re-export the same helper types as runtime."""
    stub_path = Path(__file__).resolve().parent.parent / "finstack" / "statements" / "extensions" / "__init__.pyi"

    stub_text = stub_path.read_text()

    assert '"AccountType"' in stub_text
    assert '"CorkscrewAccount"' in stub_text
    assert '"CorkscrewConfig"' in stub_text
    assert '"ScorecardMetric"' in stub_text
    assert '"ScorecardConfig"' in stub_text


def test_statements_templates_stub_advertises_validate_helpers() -> None:
    """The published templates stub should expose Rust-backed validate methods."""
    stub_path = Path(__file__).resolve().parent.parent / "finstack" / "statements" / "templates.pyi"

    stub_text = stub_path.read_text()

    assert "class SimpleLeaseSpec:" in stub_text
    assert "def validate(self) -> None: ..." in stub_text
    assert "class RenewalSpec:" in stub_text
    assert "class LeaseSpec:" in stub_text


def test_statements_builder_stub_does_not_claim_fluent_returns_for_in_place_methods() -> None:
    """Builder stub docs should describe in-place mutation instead of stale fluent returns."""
    stub_path = Path(__file__).resolve().parent.parent / "finstack" / "statements" / "builder" / "builder.pyi"

    stub_text = stub_path.read_text()

    assert "ModelBuilder: Builder instance ready for node definitions" not in stub_text
    assert "ModelBuilder: Builder instance for chaining" not in stub_text
