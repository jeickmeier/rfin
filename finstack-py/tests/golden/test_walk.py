"""Walk-test validating every committed golden fixture."""

from __future__ import annotations

from pathlib import Path
import re

import pytest

from .conftest import DATA_ROOTS, WORKSPACE_ROOT
from .schema import SCHEMA_VERSION, GoldenFixture

VALID_SOURCES = {
    "quantlib",
    "bloomberg-api",
    "bloomberg-screen",
    "intex",
    "formula",
    "textbook",
}
MANUAL_SCREENSHOT_SOURCES = {"bloomberg-screen", "intex"}
ZERO_RISK_METRICS_REQUIRING_REASON = {
    "bucketed_dv01",
    "convexity",
    "cs01",
    "delta",
    "duration_mod",
    "dv01",
    "foreign_rho",
    "gamma",
    "inflation01",
    "recovery_01",
    "rho",
    "spread_dv01",
    "vega",
}
VALUATION_DATA_ROOT = WORKSPACE_ROOT / "finstack/valuations/tests/golden/data"
RUST_GOLDEN_TEST_SOURCES = [
    WORKSPACE_ROOT / "finstack/valuations/tests/golden/pricing.rs",
    WORKSPACE_ROOT / "finstack/valuations/tests/golden/calibration.rs",
    WORKSPACE_ROOT / "finstack/valuations/tests/golden/integration.rs",
    WORKSPACE_ROOT / "finstack/valuations/tests/golden/attribution.rs",
]
RUN_GOLDEN_RE = re.compile(r'run_golden!\("([^"]+)"\)')
RUST_DISCOVERED_FIXTURE_PREFIXES = ("integration/", "pricing/")


def _all_fixtures() -> list[Path]:
    paths: list[Path] = []
    seen: set[Path] = set()
    for root in DATA_ROOTS.values():
        if root in seen or not root.exists():
            continue
        seen.add(root)
        paths.extend(path for path in root.rglob("*.json") if "screenshots" not in path.parts)
    return sorted(paths)


def _source_reference_strings(source_reference: dict, key: str) -> set[str]:
    values = source_reference.get(key, [])
    assert isinstance(values, list), f"inputs.source_reference.{key} must be a list"
    assert all(isinstance(value, str) for value in values), f"inputs.source_reference.{key} entries must be strings"
    return set(values)


def _has_metric_omission_reason(source_reference: dict) -> bool:
    reason_keys = {
        "planned_metrics_reason",
        "non_compared_metrics_reason",
        "omission_reason",
        "delta_convention_note",
        "waterfall_reference",
        "note",
    }
    return any(str(source_reference.get(key, "")).strip() for key in reason_keys)


def _has_zero_metric_reason(fixture: GoldenFixture, metric: str) -> bool:
    tolerance = fixture.tolerances.get(metric)
    if tolerance and tolerance.tolerance_reason and tolerance.tolerance_reason.strip():
        return True
    source_reference = fixture.inputs.get("source_reference", {})
    if not isinstance(source_reference, dict):
        return False
    zero_metric_reasons = source_reference.get("zero_metric_reasons", {})
    if not isinstance(zero_metric_reasons, dict):
        return False
    reason = zero_metric_reasons.get(metric)
    return isinstance(reason, str) and bool(reason.strip())


def _design_metric_aliases(source_reference: dict, metric: str) -> set[str]:
    aliases: set[str] = set()
    alias = source_reference.get(f"{metric}_key")
    if isinstance(alias, str):
        aliases.add(alias)
    if metric == "mod_duration":
        duration_alias = source_reference.get("duration_key")
        if isinstance(duration_alias, str):
            aliases.add(duration_alias)
    strict_metric_keys = source_reference.get("strict_metric_keys", {})
    if isinstance(strict_metric_keys, dict):
        strict_alias = strict_metric_keys.get(metric)
        if isinstance(strict_alias, str):
            aliases.add(strict_alias)
    return aliases


def _declared_rust_fixture_paths() -> set[str]:
    declared: set[str] = set()
    for source in RUST_GOLDEN_TEST_SOURCES:
        if not source.exists():
            continue
        declared.update(RUN_GOLDEN_RE.findall(source.read_text(encoding="utf-8")))
    return declared


@pytest.mark.parametrize("path", _all_fixtures(), ids=lambda path: str(path.relative_to(WORKSPACE_ROOT)))
def test_fixture_well_formed(path: Path) -> None:
    fixture = GoldenFixture.from_path(path)
    assert fixture.schema_version == SCHEMA_VERSION, (
        f"schema_version is {fixture.schema_version!r}, expected {SCHEMA_VERSION!r}"
    )
    assert fixture.name.strip(), "name is empty"
    assert fixture.domain.strip(), "domain is empty"
    assert fixture.description.strip(), "description is empty"
    assert fixture.provenance.source in VALID_SOURCES, f"unknown provenance.source {fixture.provenance.source!r}"
    assert fixture.provenance.as_of.strip(), "provenance.as_of is empty"
    assert fixture.provenance.source_detail.strip(), "provenance.source_detail is empty"
    assert fixture.provenance.captured_by.strip(), "provenance.captured_by is empty"
    assert fixture.provenance.captured_on.strip(), "provenance.captured_on is empty"
    assert fixture.provenance.last_reviewed_by.strip(), "provenance.last_reviewed_by is empty"
    assert fixture.provenance.last_reviewed_on.strip(), "provenance.last_reviewed_on is empty"

    extra_tolerances = set(fixture.tolerances) - set(fixture.expected_outputs)
    missing_tolerances = set(fixture.expected_outputs) - set(fixture.tolerances)
    assert not extra_tolerances, f"tolerances has extra keys: {extra_tolerances}"
    assert not missing_tolerances, f"tolerances missing keys: {missing_tolerances}"

    for metric, tolerance in fixture.tolerances.items():
        assert tolerance.abs is not None or tolerance.rel is not None, (
            f"tolerance for {metric!r} has neither abs nor rel"
        )

    for metric, expected in fixture.expected_outputs.items():
        if expected == 0.0 and metric in ZERO_RISK_METRICS_REQUIRING_REASON:
            assert _has_zero_metric_reason(fixture, metric), (
                f"zero risk metric {metric!r} requires a tolerance_reason or "
                "inputs.source_reference.zero_metric_reasons entry"
            )

    if ".integration" in fixture.domain or ".calibration." in fixture.domain:
        return

    if (
        fixture.domain.startswith("rates.")
        and fixture.domain != "rates.integration"
        and not fixture.domain.startswith("rates.calibration.")
    ):
        assert "dv01" in fixture.expected_outputs, "rates pricing fixtures must assert dv01"

    if fixture.domain.startswith("fixed_income."):
        assert "dv01" in fixture.expected_outputs, "fixed-income pricing fixtures must assert dv01"

    if fixture.domain.startswith("credit."):
        assert "dv01" in fixture.expected_outputs, "credit pricing fixtures must assert dv01"
        assert "cs01" in fixture.expected_outputs, "credit pricing fixtures must assert cs01"

    if fixture.provenance.source in MANUAL_SCREENSHOT_SOURCES:
        assert fixture.provenance.screenshots, f"source {fixture.provenance.source!r} requires at least one screenshot"

    for screenshot in fixture.provenance.screenshots:
        screenshot_path = path.parent / screenshot.path
        assert screenshot_path.exists(), (
            f"screenshot {screenshot.path!r} does not exist (resolved to {screenshot_path})"
        )

    source_reference = fixture.inputs.get("source_reference")
    if source_reference is None:
        return
    assert isinstance(source_reference, dict), "inputs.source_reference must be an object"
    planned = _source_reference_strings(source_reference, "planned_metrics_not_compared")
    non_compared = _source_reference_strings(source_reference, "non_compared_metrics")
    if planned or non_compared:
        assert _has_metric_omission_reason(source_reference), (
            "inputs.source_reference planned/non-compared metrics require an explicit reason"
        )

    asserted = set(fixture.expected_outputs)
    omitted = planned | non_compared
    for metric in _source_reference_strings(source_reference, "design_metrics"):
        aliases = _design_metric_aliases(source_reference, metric)
        assert metric in asserted or metric in omitted or aliases & asserted or aliases & omitted, (
            f"inputs.source_reference design metric {metric!r} is neither asserted nor listed as planned/non-compared"
        )


def test_valuation_fixtures_are_declared_in_rust_golden_tests() -> None:
    declared = _declared_rust_fixture_paths()
    fixture_paths = sorted(
        str(path.relative_to(VALUATION_DATA_ROOT))
        for path in VALUATION_DATA_ROOT.rglob("*.json")
        if "screenshots" not in path.parts
    )
    missing = [
        path
        for path in fixture_paths
        if path not in declared and not path.startswith(RUST_DISCOVERED_FIXTURE_PREFIXES)
    ]
    assert not missing, "fixtures missing Rust run_golden! declarations:\n" + "\n".join(missing)
