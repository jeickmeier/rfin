#!/usr/bin/env python3
"""Script to run all example scripts and report their status."""

from pathlib import Path
import subprocess
import sys
import time


def find_python_scripts(base_dir: Path) -> list[Path]:
    """Find all Python scripts in the examples/scripts directory and all subdirectories."""
    # Recursively find all Python files in all subdirectories
    scripts = sorted(base_dir.glob("**/*.py"))

    # Exclude this script itself if it's in the same directory
    script_name = Path(__file__).name
    scripts = [s for s in scripts if s.name != script_name]

    # Exclude slow Monte Carlo examples (100k paths each, can take 30-60s)
    # These are comprehensive demonstrations but too slow for routine testing
    # Run them manually when needed: uv run python finstack-py/examples/scripts/valuations/<example>.py
    excluded_patterns = []
    scripts = [s for s in scripts if s.name not in excluded_patterns]

    return scripts


def run_script(script_path: Path) -> tuple[bool, str, float]:
    """Run a Python script and return success status, output/error, and execution time.

    Returns:
        Tuple of (success, output_or_error, execution_time_seconds)
    """
    start_time = time.time()

    try:
        # Run the script with uv run as per user preference
        result = subprocess.run(
            ["uv", "run", "python", str(script_path)],
            check=False,
            capture_output=True,
            text=True,
            timeout=60,  # 60 second timeout per script (increased for calibration examples)
            cwd=script_path.parent.parent.parent.parent,  # Run from project root
        )

        execution_time = time.time() - start_time

        if result.returncode == 0:
            # Get last few lines of output for summary
            output_lines = result.stdout.strip().split("\n")
            summary = "\n".join(output_lines[-3:]) if output_lines else "No output"
            return True, summary, execution_time
        else:
            # Get error message
            error = result.stderr.strip() or result.stdout.strip()
            error_lines = error.split("\n")
            # Get the most relevant error lines
            summary = "\n".join(error_lines[-5:]) if error_lines else "Unknown error"
            return False, summary, execution_time

    except subprocess.TimeoutExpired:
        execution_time = time.time() - start_time
        return False, "Script timed out (>60s)", execution_time
    except Exception as e:
        execution_time = time.time() - start_time
        return False, f"Exception: {e!s}", execution_time


def format_time(seconds: float) -> str:
    """Format execution time nicely."""
    if seconds < 1:
        return f"{seconds * 1000:.0f}ms"
    return f"{seconds:.2f}s"


def main() -> int:
    """Main function to run all scripts and report results."""
    # Find all scripts
    base_dir = Path(__file__).parent
    scripts = find_python_scripts(base_dir)

    if not scripts:
        return 1

    for script in scripts:
        script.relative_to(base_dir)

    # Run each script and collect results
    results: dict[Path, tuple[bool, str, float]] = {}
    successful = []
    failed = []

    total_start = time.time()

    for _i, script in enumerate(scripts, 1):
        script.relative_to(base_dir)

        success, output, exec_time = run_script(script)
        results[script] = (success, output, exec_time)

        if success:
            successful.append(script)
        else:
            failed.append(script)

    time.time() - total_start

    # Print summary

    # List successful scripts
    if successful:
        for script in successful:
            script.relative_to(base_dir)
            _, _, exec_time = results[script]

    # List failed scripts with error details
    if failed:
        for script in failed:
            script.relative_to(base_dir)
            _, error, exec_time = results[script]
            for _line in error.split("\n"):
                pass

    # Print detailed output for all scripts if requested
    if "--verbose" in sys.argv:
        for script in scripts:
            script.relative_to(base_dir)
            success, output, exec_time = results[script]

    # Return exit code based on whether all scripts passed
    return 0 if len(failed) == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
