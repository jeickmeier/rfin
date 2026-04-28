"""Fixture-driven numerical parity tests for analytics bindings."""

from __future__ import annotations

from datetime import date
import json
from pathlib import Path
from typing import Any

import pytest

from finstack.analytics import (
    CagrBasis,
    cagr,
    expected_shortfall,
    multi_factor_greeks,
    rolling_greeks,
    sharpe,
    sortino,
    value_at_risk,
)


def _fixture() -> dict[str, Any]:
    path = (
        Path(__file__).resolve().parents[2] / "finstack" / "analytics" / "tests" / "fixtures" / "analytics_parity.json"
    )
    return json.loads(path.read_text())


def _dates(values: list[str]) -> list[date]:
    return [date.fromisoformat(value) for value in values]


def _assert_close(actual: float, expected: float) -> None:
    assert actual == pytest.approx(expected, abs=1e-12)


def _assert_sequence_close(actual: list[float], expected: list[float]) -> None:
    assert actual == pytest.approx(expected, abs=1e-12)


def test_python_analytics_matches_shared_parity_fixture() -> None:
    """Python analytics bindings should match the Rust-generated fixture."""
    fixture = _fixture()
    returns = fixture["returns"]
    benchmark = fixture["benchmark"]
    factors = fixture["factors"]
    expected = fixture["expected"]

    _assert_close(cagr(returns, CagrBasis.factor(252.0)), expected["cagr_factor"])
    _assert_close(sharpe(0.12, 0.18, 0.02), expected["sharpe"])
    _assert_close(sortino(returns, True, 252.0, 0.0), expected["sortino"])
    _assert_close(value_at_risk(returns, 0.95), expected["value_at_risk"])
    _assert_close(expected_shortfall(returns, 0.95), expected["expected_shortfall"])

    rolling = rolling_greeks(returns, benchmark, _dates(fixture["dates"]), 5, 252.0)
    _assert_sequence_close(rolling.alphas, expected["rolling_greeks"]["alphas"])
    _assert_sequence_close(rolling.betas, expected["rolling_greeks"]["betas"])

    multi = multi_factor_greeks(returns, factors, 252.0)
    _assert_close(multi.alpha, expected["multi_factor_greeks"]["alpha"])
    _assert_sequence_close(multi.betas, expected["multi_factor_greeks"]["betas"])
    _assert_close(multi.r_squared, expected["multi_factor_greeks"]["r_squared"])
    _assert_close(
        multi.adjusted_r_squared,
        expected["multi_factor_greeks"]["adjusted_r_squared"],
    )
    _assert_close(multi.residual_vol, expected["multi_factor_greeks"]["residual_vol"])
