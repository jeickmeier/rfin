"""pytest helpers for golden tests that consume Rust crate JSON fixtures."""

from __future__ import annotations

from collections.abc import Iterator
from contextlib import contextmanager
import csv
import importlib
import os
from pathlib import Path
import time
from types import ModuleType

from .schema import GoldenFixture
from .tolerance import compare

WORKSPACE_ROOT = Path(__file__).resolve().parents[3]
REPORT_HEADER = [
    "runner",
    "fixture",
    "metric",
    "actual",
    "expected",
    "abs_diff",
    "rel_diff",
    "abs_tolerance",
    "rel_tolerance",
    "passed",
    "tolerance_reason",
]
REPORT_LOCK_TIMEOUT_SECONDS = 30.0
REPORT_LOCK_POLL_SECONDS = 0.01

DATA_ROOTS = {
    "pricing": WORKSPACE_ROOT / "finstack/valuations/tests/golden/data",
    "calibration": WORKSPACE_ROOT / "finstack/valuations/tests/golden/data",
    "integration": WORKSPACE_ROOT / "finstack/valuations/tests/golden/data",
    "attribution": WORKSPACE_ROOT / "finstack/valuations/tests/golden/data",
    "analytics": WORKSPACE_ROOT / "finstack/analytics/tests/golden/data",
}

_DOMAIN_RUNNERS = {
    "fx.fx_swap": "pricing_fx_swap",
    "rates.calibration.curves": "calibration_curves",
    "rates.calibration.swaption_vol": "calibration_swaption_vol",
    "rates.integration": "integration_rates",
    "rates.cap_floor": "pricing_cap_floor",
    "rates.deposit": "pricing_deposit",
    "rates.fra": "pricing_fra",
    "rates.irs": "pricing_irs",
    "rates.ir_future": "pricing_ir_future",
    "rates.swaption": "pricing_swaption",
}


def _data_root_for(relative_path: str) -> Path:
    top = relative_path.split("/", 1)[0]
    if top not in DATA_ROOTS:
        known = ", ".join(sorted(DATA_ROOTS))
        msg = f"path '{relative_path}' does not start with a known top-level domain ({known})"
        raise ValueError(msg)
    return DATA_ROOTS[top]


def fixture_path(relative_path: str) -> Path:
    """Resolve a fixture path relative to its owning Rust crate's data root."""
    return _data_root_for(relative_path) / relative_path


def discover_fixtures(relative_dir: str) -> list[str]:
    """Return JSON fixtures under a relative data directory."""
    data_root = _data_root_for(relative_dir)
    root = data_root / relative_dir
    if not root.exists():
        return []
    return sorted(str(path.relative_to(data_root)) for path in root.rglob("*.json"))


def _load_runner(domain: str) -> ModuleType:
    if domain not in _DOMAIN_RUNNERS:
        msg = f"no Python runner registered for domain '{domain}'"
        raise ValueError(msg)
    module_name = _DOMAIN_RUNNERS[domain]
    return importlib.import_module(f".runners.{module_name}", package=__package__)


def run_golden(relative_path: str) -> None:
    """Load, dispatch, compare, and assert one golden fixture."""
    path = fixture_path(relative_path)
    fixture = GoldenFixture.from_path(path)
    runner = _load_runner(fixture.domain)
    actuals = runner.run(fixture)

    failures = []
    results = []
    for metric, expected in fixture.expected_outputs.items():
        if metric not in actuals:
            failures.append(f"{path}: runner did not produce metric '{metric}'")
            continue
        tolerance = fixture.tolerances[metric]
        result = compare(metric, actuals[metric], expected, tolerance)
        results.append(result)
        if not result.passed:
            failures.append(result.failure_message(str(path)))

    _write_comparison_csv(relative_path, results)

    if failures:
        msg = f"{len(failures)} metric(s) failed:\n" + "\n\n".join(failures)
        raise AssertionError(msg)


def _write_comparison_csv(relative_path: str, results: list) -> None:
    """Write a dataframe-shaped comparison report for analyst review."""
    report_path = WORKSPACE_ROOT / "target/golden-reports/golden-comparisons.csv"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    with _report_lock(report_path):
        existing_rows = _existing_comparison_rows(report_path, "python", relative_path)
        rows = [REPORT_HEADER, *existing_rows]
        rows.extend(
            [
                "python",
                relative_path,
                result.metric,
                f"{result.actual:.12f}",
                f"{result.expected:.12f}",
                f"{result.abs_diff:.12e}",
                f"{result.rel_diff:.12e}",
                "" if result.used_tolerance.abs is None else f"{result.used_tolerance.abs:.12f}",
                "" if result.used_tolerance.rel is None else f"{result.used_tolerance.rel:.12f}",
                str(result.passed).lower(),
                result.used_tolerance.tolerance_reason or "",
            ]
            for result in results
        )
        _write_report_atomically(report_path, rows)


@contextmanager
def _report_lock(report_path: Path) -> Iterator[None]:
    """Acquire a process-wide lock for read/modify/write report updates."""
    lock_path = report_path.with_suffix(".csv.lock")
    deadline = time.monotonic() + REPORT_LOCK_TIMEOUT_SECONDS
    fd: int | None = None
    while fd is None:
        try:
            fd = os.open(lock_path, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
        except FileExistsError:
            if time.monotonic() >= deadline:
                msg = f"timed out waiting for report lock {lock_path}"
                raise TimeoutError(msg) from None
            time.sleep(REPORT_LOCK_POLL_SECONDS)

    try:
        yield
    finally:
        os.close(fd)
        lock_path.unlink(missing_ok=True)


def _write_report_atomically(report_path: Path, rows: list[list[str]]) -> None:
    temp_path = report_path.with_suffix(f".csv.{os.getpid()}.tmp")
    try:
        with temp_path.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.writer(handle)
            writer.writerows(rows)
        temp_path.replace(report_path)
    finally:
        temp_path.unlink(missing_ok=True)


def _existing_comparison_rows(report_path: Path, runner: str, relative_path: str) -> list[list[str]]:
    """Read existing aggregate rows, dropping stale rows for this runner/fixture."""
    if not report_path.exists():
        return []

    with report_path.open("r", encoding="utf-8", newline="") as handle:
        rows = list(csv.reader(handle))
    return [row for row in rows[1:] if len(row) >= 2 and row[:2] != [runner, relative_path]]
