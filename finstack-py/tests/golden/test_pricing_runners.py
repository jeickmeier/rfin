"""Unit tests for the shared pricing runner's market-resolution logic."""

from __future__ import annotations

import pytest

from tests.golden.runners.pricing_common import run_pricing_fixture
from tests.golden.schema import GoldenFixture, Provenance, ToleranceEntry


def _provenance() -> Provenance:
    return Provenance(
        as_of="2026-04-30",
        source="formula",
        source_detail="unit test",
        captured_by="pytest",
        captured_on="2026-04-30",
        last_reviewed_by="pytest",
        last_reviewed_on="2026-04-30",
        review_interval_months=6,
        regen_command="n/a",
        screenshots=[],
    )


def _minimal_market_dict() -> dict:
    """Minimal valid MarketContext JSON shape (empty curves, no surfaces)."""
    return {
        "version": 2,
        "curves": [],
        "fx": None,
        "surfaces": [],
        "prices": {},
        "series": [],
        "inflation_indices": [],
        "dividends": [],
        "credit_indices": [],
        "fx_delta_vol_surfaces": [],
        "vol_cubes": [],
        "collateral": {},
    }


def _minimal_envelope_dict() -> dict:
    """Minimal valid CalibrationEnvelope JSON shape (no steps, no initial market)."""
    return {
        "schema": "finstack.calibration",
        "plan": {
            "id": "test_envelope",
            "quote_sets": {},
            "steps": [],
            "settings": {},
        },
    }


def _make_fixture(inputs: dict) -> GoldenFixture:
    """Construct a minimal GoldenFixture for unit-testing the runner."""
    return GoldenFixture(
        schema_version="finstack.golden/1",
        name="test",
        domain="test",
        description="test",
        provenance=_provenance(),
        inputs=inputs,
        expected_outputs={"npv": 0.0},
        tolerances={"npv": ToleranceEntry(abs=1e-6)},
    )


def test_pricing_inputs_reject_when_both_market_and_market_envelope() -> None:
    fixture = _make_fixture({
        "valuation_date": "2026-04-30",
        "model": "discounting",
        "metrics": [],
        "instrument_json": {},
        "market": _minimal_market_dict(),
        "market_envelope": _minimal_envelope_dict(),
    })
    with pytest.raises(ValueError, match=r"market.*market_envelope|market_envelope.*market"):
        run_pricing_fixture(fixture)


def test_pricing_inputs_reject_when_neither_market_nor_market_envelope() -> None:
    fixture = _make_fixture({
        "valuation_date": "2026-04-30",
        "model": "discounting",
        "metrics": [],
        "instrument_json": {},
    })
    with pytest.raises(ValueError, match=r"market.*market_envelope|market_envelope.*market"):
        run_pricing_fixture(fixture)
