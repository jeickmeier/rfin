#!/usr/bin/env python3
"""Execute all example scripts and report pass/fail status."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import subprocess
import sys
import time


PYTHON_RUNNER = ("uv", "run", "python")


def find_scripts(base_dir: Path, subdirectory: str | None = None) -> list[Path]:
    """Find all example scripts under *base_dir*, optionally filtered to *subdirectory*."""
    search_root = base_dir / subdirectory if subdirectory else base_dir
    if not search_root.exists():
        return []

    scripts = sorted(search_root.glob("**/*.py"))
    return [
        script
        for script in scripts
        if script.name != "run_all_scripts.py" and ".ipynb_checkpoints" not in str(script)
    ]


def _build_env() -> dict[str, str]:
    """Build an environment that can import the local finstack package."""
    repo_root = Path(__file__).resolve().parents[3]
    finstack_py_root = repo_root / "finstack-py"
    extra_paths = [str(finstack_py_root), str(repo_root)]

    env = dict(os.environ)
    existing = env.get("PYTHONPATH", "")
    pieces = [piece for piece in existing.split(os.pathsep) if piece]
    for path in extra_paths:
        if path not in pieces:
            pieces.insert(0, path)
    env["PYTHONPATH"] = os.pathsep.join(pieces)
    return env


def _tail_text(text: str, max_lines: int = 12) -> str:
    """Return the trailing lines of command output for concise reporting."""
    lines = [line for line in text.strip().splitlines() if line.strip()]
    if not lines:
        return "Completed successfully"
    return "\n".join(lines[-max_lines:])


def run_script(script_path: Path, timeout: int) -> tuple[bool, str, float]:
    """Run a single example script and return (success, message, elapsed_seconds)."""
    start = time.time()
    try:
        completed = subprocess.run(
            [*PYTHON_RUNNER, script_path.name],
            cwd=script_path.parent,
            capture_output=True,
            text=True,
            timeout=timeout,
            env=_build_env(),
            check=False,
        )
        elapsed = time.time() - start
        combined_output = "\n".join(part for part in (completed.stdout, completed.stderr) if part.strip())
        message = _tail_text(combined_output)
        return completed.returncode == 0, message, elapsed

    except subprocess.TimeoutExpired as exc:
        elapsed = time.time() - start
        partial_output = "\n".join(
            part
            for part in (
                exc.stdout.decode() if isinstance(exc.stdout, bytes) else exc.stdout or "",
                exc.stderr.decode() if isinstance(exc.stderr, bytes) else exc.stderr or "",
            )
            if part.strip()
        )
        detail = _tail_text(partial_output) if partial_output else ""
        timeout_message = f"Timed out (>{timeout}s)"
        if detail:
            timeout_message = f"{timeout_message}\n{detail}"
        return False, timeout_message, elapsed

    except Exception as exc:
        return False, f"{type(exc).__name__}: {exc}", time.time() - start


def _fmt(seconds: float) -> str:
    """Format elapsed time in a human-friendly way."""
    return f"{seconds * 1000:.0f}ms" if seconds < 1 else f"{seconds:.2f}s"


def main() -> int:
    """Run example scripts and print a pass/fail summary."""
    parser = argparse.ArgumentParser(description="Run finstack example scripts")
    parser.add_argument("--directory", help="Only run scripts in this subdirectory")
    parser.add_argument("--timeout", type=int, default=300, help="Per-script timeout in seconds")
    parser.add_argument("--verbose", action="store_true", help="Show detailed output")
    parser.add_argument(
        "--fail-fast",
        action="store_true",
        help="Stop after the first script failure",
    )
    args = parser.parse_args()

    base_dir = Path(__file__).parent
    scripts = find_scripts(base_dir, args.directory)
    if not scripts:
        print("No scripts found!")
        return 1

    print(f"Found {len(scripts)} scripts to run:\n")
    for script_path in scripts:
        print(f"  - {script_path.relative_to(base_dir)}")
    print()

    results: dict[Path, tuple[bool, str, float]] = {}
    successful: list[Path] = []
    failed: list[Path] = []
    started = time.time()

    for index, script_path in enumerate(scripts, start=1):
        rel_path = script_path.relative_to(base_dir)
        print(f"[{index}/{len(scripts)}] Running {rel_path}...", end=" ", flush=True)

        ok, message, elapsed = run_script(script_path, args.timeout)
        results[script_path] = (ok, message, elapsed)
        if ok:
            successful.append(script_path)
            print(f"PASS ({_fmt(elapsed)})")
        else:
            failed.append(script_path)
            print(f"FAIL ({_fmt(elapsed)})")
            if args.fail_fast:
                print("\nStopped early (--fail-fast).")
                break

    total = time.time() - started
    executed = len(successful) + len(failed)
    print("\n" + "=" * 60)
    print(f"SUMMARY: {len(successful)}/{executed} passed in {_fmt(total)}")
    if executed < len(scripts):
        print(f"({len(scripts) - executed} script(s) not run)")
    print("=" * 60)

    if successful:
        print(f"\nPASS ({len(successful)}):")
        for script_path in successful:
            _, _, elapsed = results[script_path]
            print(f"  {script_path.relative_to(base_dir)} ({_fmt(elapsed)})")

    if failed:
        print(f"\nFAIL ({len(failed)}):")
        for script_path in failed:
            _, message, elapsed = results[script_path]
            print(f"  {script_path.relative_to(base_dir)} ({_fmt(elapsed)})")
            for line in message.splitlines():
                print(f"    {line}")

    if args.verbose:
        print("\n" + "=" * 60)
        print("DETAILED OUTPUT")
        print("=" * 60)
        for script_path, (ok, message, elapsed) in results.items():
            status = "PASS" if ok else "FAIL"
            print(f"\n{status} {script_path.relative_to(base_dir)} ({_fmt(elapsed)}):")
            print("-" * 40)
            print(message)

    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main())
