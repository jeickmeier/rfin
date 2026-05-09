"""CDS option pricing goldens."""

from __future__ import annotations

import pytest

from .conftest import discover_fixtures, fixture_path, run_golden
from .schema import GoldenFixture


def test_cdx_ig_46_expected_outputs_are_raw_bloomberg_screen_values() -> None:
    """Guard against replacing Bloomberg source values with model-current outputs."""
    fixture = GoldenFixture.from_path(fixture_path("pricing/cds_option/cdx_ig_46_payer_atm_jun26.json"))
    source_reference = fixture.inputs["source_reference"]
    bloomberg_outputs = source_reference["bloomberg_outputs"]
    screen_meta = fixture.inputs["instrument_json"]["spec"]["attributes"]["meta"]

    assert fixture.expected_outputs["npv"] == bloomberg_outputs["market_value"]
    assert fixture.expected_outputs["par_spread"] == float(screen_meta["atm_forward_bp"])
    assert fixture.expected_outputs["vega"] == bloomberg_outputs["vega_1pct"]
    assert fixture.expected_outputs["dv01"] == bloomberg_outputs["ir_dv01"]
    assert fixture.expected_outputs["cs01"] == bloomberg_outputs["spread_dv01"]
    assert fixture.expected_outputs["theta"] == bloomberg_outputs["theta_per_day"]


@pytest.mark.parametrize("fixture", discover_fixtures("pricing/cds_option"))
def test_pricing_cds_option(fixture: str) -> None:
    """Run every CDS option pricing fixture through the Python bindings."""
    run_golden(fixture)
