"""Walk-test validating every committed golden fixture."""

from __future__ import annotations

from pathlib import Path

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


def _all_fixtures() -> list[Path]:
    paths: list[Path] = []
    seen: set[Path] = set()
    for root in DATA_ROOTS.values():
        if root in seen or not root.exists():
            continue
        seen.add(root)
        paths.extend(path for path in root.rglob("*.json") if "screenshots" not in path.parts)
    return sorted(paths)


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

    if fixture.provenance.source in MANUAL_SCREENSHOT_SOURCES:
        assert fixture.provenance.screenshots, f"source {fixture.provenance.source!r} requires at least one screenshot"

    for screenshot in fixture.provenance.screenshots:
        screenshot_path = path.parent / screenshot.path
        assert screenshot_path.exists(), (
            f"screenshot {screenshot.path!r} does not exist (resolved to {screenshot_path})"
        )
