"""Dataclasses mirroring the Rust `finstack.golden/1` fixture schema."""

from __future__ import annotations

from dataclasses import dataclass, field
import json
from pathlib import Path
from typing import Any

SCHEMA_VERSION = "finstack.golden/1"


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
        prov_raw = raw["provenance"]
        screenshots = [Screenshot(**screenshot) for screenshot in prov_raw.get("screenshots", [])]
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
        tolerances = {metric: ToleranceEntry(**tolerance) for metric, tolerance in raw["tolerances"].items()}
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
