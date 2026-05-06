"""Unit tests for golden fixture schema parsing."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from .schema import SCHEMA_VERSION, GoldenFixture


def test_parse_minimal_fixture(tmp_path: Path) -> None:
    fixture_json = _minimal_fixture_json()
    path = tmp_path / "fixture.json"
    path.write_text(json.dumps(fixture_json), encoding="utf-8")

    fixture = GoldenFixture.from_path(path)

    assert fixture.schema_version == SCHEMA_VERSION
    assert fixture.name == "test_fixture"
    assert fixture.expected_outputs["npv"] == 100.0
    assert fixture.provenance.screenshots == []


def test_rejects_unknown_top_level_field(tmp_path: Path) -> None:
    fixture_json = _minimal_fixture_json()
    fixture_json["unexpected"] = True
    path = tmp_path / "fixture.json"
    path.write_text(json.dumps(fixture_json), encoding="utf-8")

    with pytest.raises(ValueError, match="fixture has unknown key"):
        GoldenFixture.from_path(path)


def test_rejects_unknown_nested_metadata_field(tmp_path: Path) -> None:
    fixture_json = _minimal_fixture_json()
    fixture_json["provenance"]["unexpected"] = True
    path = tmp_path / "fixture.json"
    path.write_text(json.dumps(fixture_json), encoding="utf-8")

    with pytest.raises(ValueError, match="provenance has unknown key"):
        GoldenFixture.from_path(path)


def _minimal_fixture_json() -> dict:
    return {
        "schema_version": SCHEMA_VERSION,
        "name": "test_fixture",
        "domain": "rates.irs",
        "description": "Minimal smoke fixture",
        "provenance": {
            "as_of": "2026-04-30",
            "source": "quantlib",
            "source_detail": "QL 1.34",
            "captured_by": "test",
            "captured_on": "2026-04-30",
            "last_reviewed_by": "test",
            "last_reviewed_on": "2026-04-30",
            "review_interval_months": 6,
            "regen_command": "uv run scripts/goldens/regen.py --kind irs-par",
        },
        "inputs": {"foo": 1},
        "expected_outputs": {"npv": 100.0},
        "tolerances": {"npv": {"abs": 0.01}},
    }
