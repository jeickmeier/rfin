from __future__ import annotations

from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import sys
from types import ModuleType

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "check_rust_coverage_gate.py"


def load_module() -> ModuleType:
    spec = spec_from_file_location("check_rust_coverage_gate", SCRIPT_PATH)
    if spec is None or spec.loader is None:
        msg = f"Could not load module from {SCRIPT_PATH}"
        raise AssertionError(msg)
    module = module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def sample_report(workspace_percent: float, file_percents: dict[str, float]) -> dict[str, object]:
    return {
        "data": [
            {
                "files": [
                    {
                        "filename": str(REPO_ROOT / relative_path),
                        "summary": {
                            "lines": {
                                "count": 1000,
                                "covered": round(percent * 10),
                                "percent": percent,
                            }
                        },
                    }
                    for relative_path, percent in file_percents.items()
                ],
                "totals": {
                    "lines": {
                        "count": 1000,
                        "covered": round(workspace_percent * 10),
                        "percent": workspace_percent,
                    }
                },
            }
        ]
    }


def test_evaluate_gate_accepts_workspace_and_touched_files_above_threshold() -> None:
    module = load_module()
    report = sample_report(
        workspace_percent=86.4,
        file_percents={
            "finstack/core/src/lib.rs": 91.0,
            "finstack/valuations/src/lib.rs": 82.5,
        },
    )

    result = module.evaluate_gate(
        report=report,
        repo_root=REPO_ROOT,
        touched_paths=[
            REPO_ROOT / "finstack/core/src/lib.rs",
            REPO_ROOT / "finstack/valuations/src/lib.rs",
        ],
        workspace_threshold=80.0,
        touched_threshold=80.0,
    )

    assert result.workspace_line_percent == pytest.approx(86.4)
    assert result.checked_files == {
        "finstack/core/src/lib.rs": pytest.approx(91.0),
        "finstack/valuations/src/lib.rs": pytest.approx(82.5),
    }


def test_evaluate_gate_rejects_workspace_below_threshold() -> None:
    module = load_module()
    report = sample_report(
        workspace_percent=79.9,
        file_percents={"finstack/core/src/lib.rs": 95.0},
    )

    with pytest.raises(module.CoverageGateError, match="workspace Rust line coverage"):
        module.evaluate_gate(
            report=report,
            repo_root=REPO_ROOT,
            touched_paths=[REPO_ROOT / "finstack/core/src/lib.rs"],
            workspace_threshold=80.0,
            touched_threshold=80.0,
        )


def test_evaluate_gate_rejects_touched_file_below_threshold() -> None:
    module = load_module()
    report = sample_report(
        workspace_percent=92.0,
        file_percents={
            "finstack/core/src/lib.rs": 95.0,
            "finstack/valuations/src/schema.rs": 72.0,
        },
    )

    with pytest.raises(module.CoverageGateError, match=r"finstack/valuations/src/schema\.rs"):
        module.evaluate_gate(
            report=report,
            repo_root=REPO_ROOT,
            touched_paths=[
                REPO_ROOT / "finstack/core/src/lib.rs",
                REPO_ROOT / "finstack/valuations/src/schema.rs",
            ],
            workspace_threshold=80.0,
            touched_threshold=80.0,
        )


def test_evaluate_gate_ignores_rust_test_files_when_checking_touched_files() -> None:
    module = load_module()
    report = sample_report(
        workspace_percent=88.0,
        file_percents={"finstack/core/src/lib.rs": 88.0},
    )

    result = module.evaluate_gate(
        report=report,
        repo_root=REPO_ROOT,
        touched_paths=[
            REPO_ROOT / "finstack/core/src/lib.rs",
            REPO_ROOT / "finstack/core/tests/regression.rs",
            REPO_ROOT / "finstack/valuations/tests/market/mod.rs",
        ],
        workspace_threshold=80.0,
        touched_threshold=80.0,
    )

    assert result.checked_files == {"finstack/core/src/lib.rs": pytest.approx(88.0)}


def test_evaluate_gate_rejects_missing_coverage_for_touched_production_file() -> None:
    module = load_module()
    report = sample_report(
        workspace_percent=91.0,
        file_percents={"finstack/core/src/lib.rs": 91.0},
    )

    with pytest.raises(module.CoverageGateError, match="Missing line coverage data"):
        module.evaluate_gate(
            report=report,
            repo_root=REPO_ROOT,
            touched_paths=[
                REPO_ROOT / "finstack/core/src/lib.rs",
                REPO_ROOT / "finstack/valuations/src/schema.rs",
            ],
            workspace_threshold=80.0,
            touched_threshold=80.0,
        )


def test_get_touched_rust_source_paths_filters_non_source_files() -> None:
    module = load_module()

    touched_paths = module.get_touched_rust_source_paths(
        repo_root=REPO_ROOT,
        staged_paths=[
            "README.md",
            "finstack/core/src/lib.rs",
            "finstack/core/tests/regression.rs",
            "finstack/valuations/src/schema.rs",
            "finstack-py/src/lib.rs",
            "finstack-wasm/src/lib.rs",
            "target/debug/build-script.rs",
        ],
    )

    assert touched_paths == [
        REPO_ROOT / "finstack/core/src/lib.rs",
        REPO_ROOT / "finstack/valuations/src/schema.rs",
    ]
