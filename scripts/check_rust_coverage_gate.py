"""Enforce Rust coverage thresholds for the workspace and staged source files."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
from pathlib import Path
import shutil
import subprocess
import sys
from typing import Any

DEFAULT_THRESHOLD = 80.0
EXCLUDED_ROOTS = {"finstack-py", "finstack-wasm", "target", ".cargo"}
TEST_PATH_PARTS = {"test", "tests"}


class CoverageGateError(RuntimeError):
    """Raised when the Rust coverage gate fails."""


@dataclass(slots=True)
class GateResult:
    """Coverage summary for the workspace gate and touched-file checks."""

    workspace_line_percent: float
    checked_files: dict[str, float]


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Enforce workspace and touched-file Rust line coverage thresholds.",
    )
    parser.add_argument(
        "--coverage-json",
        type=Path,
        required=True,
        help="Path to cargo llvm-cov JSON summary output.",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root used to normalize paths and inspect staged files.",
    )
    parser.add_argument(
        "--workspace-threshold",
        type=float,
        default=DEFAULT_THRESHOLD,
        help="Minimum workspace Rust line coverage percentage.",
    )
    parser.add_argument(
        "--touched-threshold",
        type=float,
        default=DEFAULT_THRESHOLD,
        help="Minimum line coverage percentage for touched Rust source files.",
    )
    return parser.parse_args()


def _load_coverage_report(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        report = json.load(handle)
    if not isinstance(report, dict):
        msg = f"Coverage report at {path} is not a JSON object"
        raise CoverageGateError(msg)
    return report


def get_touched_rust_source_paths(repo_root: Path, staged_paths: list[str] | None = None) -> list[Path]:
    """Return staged Rust source files that should satisfy the per-file gate."""
    if staged_paths is None:
        staged_paths = _get_staged_paths(repo_root)

    touched_paths: list[Path] = []
    for staged_path in staged_paths:
        relative_path = Path(staged_path)
        if _should_check_rust_source(relative_path):
            touched_paths.append((repo_root / relative_path).resolve())
    return touched_paths


def _get_staged_paths(repo_root: Path) -> list[str]:
    git_path = shutil.which("git")
    if git_path is None:
        msg = "Could not locate the 'git' executable"
        raise CoverageGateError(msg)

    result = subprocess.run(  # noqa: S603
        [git_path, "diff", "--cached", "--name-only", "--diff-filter=ACMR"],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=True,
    )
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def _should_check_rust_source(relative_path: Path) -> bool:
    if relative_path.suffix != ".rs":
        return False
    if any(part in EXCLUDED_ROOTS for part in relative_path.parts):
        return False
    if any(part in TEST_PATH_PARTS for part in relative_path.parts):
        return False
    name = relative_path.name
    return not (name.startswith("test_") or name.endswith("_test.rs"))


def evaluate_gate(
    *,
    report: dict[str, Any],
    repo_root: Path,
    touched_paths: list[Path],
    workspace_threshold: float,
    touched_threshold: float,
) -> GateResult:
    """Validate workspace coverage and per-file coverage for touched Rust sources."""
    workspace_line_percent = _extract_workspace_line_percent(report)
    if workspace_line_percent <= workspace_threshold:
        msg = (
            "Rust workspace line coverage gate failed: "
            f"workspace Rust line coverage is {workspace_line_percent:.2f}% "
            f"and must be greater than {workspace_threshold:.2f}%"
        )
        raise CoverageGateError(msg)

    file_percents = _extract_file_line_percents(report, repo_root)
    checked_files: dict[str, float] = {}
    for touched_path in touched_paths:
        relative_path = touched_path.relative_to(repo_root).as_posix()
        if not _should_check_rust_source(Path(relative_path)):
            continue
        line_percent = file_percents.get(touched_path)
        if line_percent is None:
            msg = f"Missing line coverage data for touched Rust source file: {relative_path}"
            raise CoverageGateError(msg)
        checked_files[relative_path] = line_percent
        if line_percent <= touched_threshold:
            msg = (
                "Rust touched-file coverage gate failed: "
                f"{relative_path} has {line_percent:.2f}% line coverage "
                f"and must be greater than {touched_threshold:.2f}%"
            )
            raise CoverageGateError(msg)

    return GateResult(
        workspace_line_percent=workspace_line_percent,
        checked_files=checked_files,
    )


def _extract_workspace_line_percent(report: dict[str, Any]) -> float:
    total_count = 0
    total_covered = 0
    for bundle in _coverage_bundles(report):
        lines = _line_summary(bundle.get("totals"), context="workspace totals")
        total_count += lines["count"]
        total_covered += lines["covered"]
    if total_count == 0:
        msg = "Workspace coverage report contains zero executable lines"
        raise CoverageGateError(msg)
    return total_covered * 100.0 / total_count


def _extract_file_line_percents(report: dict[str, Any], repo_root: Path) -> dict[Path, float]:
    file_counts: dict[Path, tuple[int, int]] = {}
    for bundle in _coverage_bundles(report):
        files = bundle.get("files")
        if not isinstance(files, list):
            msg = "Coverage report is missing the 'files' list"
            raise CoverageGateError(msg)
        for file_entry in files:
            if not isinstance(file_entry, dict):
                msg = "Coverage report contains a non-object file entry"
                raise CoverageGateError(msg)
            filename = file_entry.get("filename")
            if not isinstance(filename, str):
                msg = "Coverage report file entry is missing a filename"
                raise CoverageGateError(msg)
            normalized_path = _normalize_path(repo_root, filename)
            lines = _line_summary(file_entry.get("summary"), context=f"file summary for {filename}")
            count, covered = file_counts.get(normalized_path, (0, 0))
            file_counts[normalized_path] = (count + lines["count"], covered + lines["covered"])

    result: dict[Path, float] = {}
    for path, (count, covered) in file_counts.items():
        if count == 0:
            continue
        result[path] = covered * 100.0 / count
    return result


def _normalize_path(repo_root: Path, raw_path: str) -> Path:
    candidate = Path(raw_path)
    if candidate.is_absolute():
        return candidate.resolve()
    return (repo_root / candidate).resolve()


def _coverage_bundles(report: dict[str, Any]) -> list[dict[str, Any]]:
    bundles = report.get("data")
    if not isinstance(bundles, list) or not bundles:
        msg = "Coverage report is missing the top-level 'data' list"
        raise CoverageGateError(msg)
    normalized: list[dict[str, Any]] = []
    for bundle in bundles:
        if not isinstance(bundle, dict):
            msg = "Coverage report contains a non-object data bundle"
            raise CoverageGateError(msg)
        normalized.append(bundle)
    return normalized


def _line_summary(summary: Any, *, context: str) -> dict[str, int]:
    if not isinstance(summary, dict):
        msg = f"Coverage report is missing {context}"
        raise CoverageGateError(msg)
    lines = summary.get("lines")
    if not isinstance(lines, dict):
        msg = f"Coverage report is missing line summary for {context}"
        raise CoverageGateError(msg)
    count = lines.get("count")
    covered = lines.get("covered")
    if not isinstance(count, int) or not isinstance(covered, int):
        msg = f"Coverage report line summary for {context} must include integer count and covered values"
        raise CoverageGateError(msg)
    return {"count": count, "covered": covered}


def _print_summary(result: GateResult, touched_threshold: float) -> None:
    print(f"Rust workspace line coverage: {result.workspace_line_percent:.2f}%")
    if not result.checked_files:
        print("No staged Rust source files require per-file coverage checks.")
        return
    print(f"Checked touched Rust source files (> {touched_threshold:.2f}% required):")
    for relative_path, percent in sorted(result.checked_files.items()):
        print(f"  - {relative_path}: {percent:.2f}%")


def main() -> int:
    """Run the Rust coverage gate CLI."""
    args = _parse_args()
    repo_root = args.repo_root.resolve()
    report = _load_coverage_report(args.coverage_json)
    touched_paths = get_touched_rust_source_paths(repo_root)
    try:
        result = evaluate_gate(
            report=report,
            repo_root=repo_root,
            touched_paths=touched_paths,
            workspace_threshold=args.workspace_threshold,
            touched_threshold=args.touched_threshold,
        )
    except CoverageGateError as error:
        print(error, file=sys.stderr)
        return 1
    _print_summary(result, args.touched_threshold)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
