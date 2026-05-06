"""Dataclasses mirroring the Rust `finstack.golden/1` fixture schema."""

from __future__ import annotations

from dataclasses import dataclass, field
import json
from pathlib import Path
from typing import Any

SCHEMA_VERSION = "finstack.golden/1"
_GOLDEN_FIXTURE_KEYS = {
    "schema_version",
    "name",
    "domain",
    "description",
    "provenance",
    "inputs",
    "expected_outputs",
    "tolerances",
}
_PROVENANCE_KEYS = {
    "as_of",
    "source",
    "source_detail",
    "captured_by",
    "captured_on",
    "last_reviewed_by",
    "last_reviewed_on",
    "review_interval_months",
    "regen_command",
    "screenshots",
}
_SCREENSHOT_KEYS = {"path", "screen", "captured_on", "description"}
_TOLERANCE_KEYS = {"abs", "rel", "tolerance_reason"}


@dataclass
class Screenshot:
    """Screenshot evidence for manually captured external references."""

    path: str
    screen: str
    captured_on: str
    description: str


@dataclass
class Provenance:
    """Fixture provenance and review metadata."""

    as_of: str
    source: str
    source_detail: str
    captured_by: str
    captured_on: str
    last_reviewed_by: str
    last_reviewed_on: str
    review_interval_months: int
    regen_command: str
    screenshots: list[Screenshot] = field(default_factory=list)


@dataclass
class ToleranceEntry:
    """Per-metric tolerance entry."""

    abs: float | None = None
    rel: float | None = None
    tolerance_reason: str | None = None


@dataclass
class GoldenFixture:
    """Top-level fixture envelope loaded from one JSON file."""

    schema_version: str
    name: str
    domain: str
    description: str
    provenance: Provenance
    inputs: dict[str, Any]
    expected_outputs: dict[str, float]
    tolerances: dict[str, ToleranceEntry]

    @classmethod
    def from_path(cls, path: Path) -> GoldenFixture:
        """Load and parse a golden fixture from disk."""
        raw = json.loads(path.read_text(encoding="utf-8"))
        _reject_unknown_keys("fixture", raw, _GOLDEN_FIXTURE_KEYS)
        prov_raw = raw["provenance"]
        _reject_unknown_keys("provenance", prov_raw, _PROVENANCE_KEYS)
        screenshots = []
        for screenshot in prov_raw.get("screenshots", []):
            _reject_unknown_keys("screenshot", screenshot, _SCREENSHOT_KEYS)
            screenshots.append(Screenshot(**screenshot))
        provenance = Provenance(
            as_of=prov_raw["as_of"],
            source=prov_raw["source"],
            source_detail=prov_raw["source_detail"],
            captured_by=prov_raw["captured_by"],
            captured_on=prov_raw["captured_on"],
            last_reviewed_by=prov_raw["last_reviewed_by"],
            last_reviewed_on=prov_raw["last_reviewed_on"],
            review_interval_months=prov_raw["review_interval_months"],
            regen_command=prov_raw["regen_command"],
            screenshots=screenshots,
        )
        tolerances = {}
        for metric, tolerance in raw["tolerances"].items():
            _reject_unknown_keys(f"tolerances.{metric}", tolerance, _TOLERANCE_KEYS)
            tolerances[metric] = ToleranceEntry(**tolerance)
        return cls(
            schema_version=raw["schema_version"],
            name=raw["name"],
            domain=raw["domain"],
            description=raw["description"],
            provenance=provenance,
            inputs=raw["inputs"],
            expected_outputs={metric: float(value) for metric, value in raw["expected_outputs"].items()},
            tolerances=tolerances,
        )


def _reject_unknown_keys(label: str, value: dict[str, Any], allowed: set[str]) -> None:
    extra = sorted(set(value) - allowed)
    if extra:
        msg = f"{label} has unknown key(s): {extra}"
        raise ValueError(msg)
