#!/usr/bin/env python3
"""
Script to run all example notebooks and report their status.
Uses nbclient to execute notebooks programmatically.
"""

import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Tuple

import nbformat
from nbclient import NotebookClient
from nbclient.exceptions import CellExecutionError


def find_notebooks(base_dir: Path) -> List[Path]:
    """Find all Jupyter notebooks in the notebooks directory and all subdirectories."""
    # Recursively find all .ipynb files
    notebooks = sorted(base_dir.glob("**/*.ipynb"))

    # Exclude checkpoint files
    notebooks = [nb for nb in notebooks if ".ipynb_checkpoints" not in str(nb)]

    return notebooks


def run_notebook(notebook_path: Path) -> Tuple[bool, str, float]:
    """
    Run a Jupyter notebook and return success status, output/error, and execution time.

    Returns:
        Tuple of (success, output_or_error, execution_time_seconds)
    """
    start_time = time.time()

    try:
        # Read the notebook
        with open(notebook_path, "r", encoding="utf-8") as f:
            nb = nbformat.read(f, as_version=4)

        # Create a notebook client
        client = NotebookClient(
            nb,
            timeout=120,  # 120 second timeout per notebook
            kernel_name="python3",
            resources={"metadata": {"path": str(notebook_path.parent)}},
        )

        # Execute the notebook
        client.execute()

        execution_time = time.time() - start_time

        # Count cells executed
        cell_count = len([cell for cell in nb.cells if cell.cell_type == "code"])
        summary = f"Successfully executed {cell_count} code cells"

        return True, summary, execution_time

    except CellExecutionError as e:
        execution_time = time.time() - start_time

        # Extract error information from the exception
        error_summary = []

        # Try to get the traceback from the exception
        if hasattr(e, "traceback"):
            # Extract the error message and traceback
            tb_lines = str(e.traceback).split("\n") if e.traceback else []
            error_lines = str(e).split("\n")

            # Find the actual error type and message
            for line in error_lines + tb_lines:
                if any(keyword in line for keyword in ["Error", "Exception", "Traceback", "File"]):
                    error_summary.append(line)
                    if len(error_summary) >= 10:  # Limit to 10 lines
                        break

            # If we found an error, include a few more context lines
            if error_summary:
                # Get the last error/exception line and a few before/after
                for i, line in enumerate(error_lines):
                    if "Error" in line or "Exception" in line:
                        start_idx = max(0, i - 2)
                        end_idx = min(len(error_lines), i + 5)
                        error_summary = error_lines[start_idx:end_idx]
                        break

        if not error_summary:
            # Fallback: use the exception string representation
            error_summary = [str(e)]
            # Try to extract meaningful parts
            error_str = str(e)
            if "\n" in error_str:
                lines = error_str.split("\n")
                # Find lines with Error or Exception
                for i, line in enumerate(lines):
                    if "Error" in line or "Exception" in line:
                        start_idx = max(0, i - 1)
                        end_idx = min(len(lines), i + 4)
                        error_summary = lines[start_idx:end_idx]
                        break
                if len(error_summary) == 1:
                    # Get last few lines as fallback
                    error_summary = lines[-5:] if len(lines) >= 5 else lines

        summary = "\n".join(error_summary[:15])  # Limit to 15 lines max
        return False, summary, execution_time

    except TimeoutError:
        execution_time = time.time() - start_time
        return False, "Notebook execution timed out (>120s)", execution_time

    except Exception as e:
        execution_time = time.time() - start_time
        error_type = type(e).__name__
        return False, f"{error_type}: {str(e)}", execution_time


def format_time(seconds: float) -> str:
    """Format execution time nicely."""
    if seconds < 1:
        return f"{seconds * 1000:.0f}ms"
    return f"{seconds:.2f}s"


def main():
    """Main function to run all notebooks and report results."""

    print("=" * 80)
    print("FINSTACK EXAMPLE NOTEBOOKS TEST RUNNER")
    print("=" * 80)
    print(f"Started at: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print()

    # Find all notebooks
    # Assuming this script is in finstack-py/examples/scripts/
    base_dir = Path(__file__).parent.parent / "notebooks"

    if not base_dir.exists():
        print(f"Notebooks directory not found: {base_dir}")
        return 1

    notebooks = find_notebooks(base_dir)

    if not notebooks:
        print("No notebooks found!")
        return 1

    print(f"Found {len(notebooks)} notebooks to run:")
    for notebook in notebooks:
        rel_path = notebook.relative_to(base_dir)
        print(f"  - {rel_path}")
    print()
    print("-" * 80)

    # Run each notebook and collect results
    results: Dict[Path, Tuple[bool, str, float]] = {}
    successful = []
    failed = []

    total_start = time.time()

    for i, notebook in enumerate(notebooks, 1):
        rel_path = notebook.relative_to(base_dir)
        print(f"\n[{i}/{len(notebooks)}] Running: {rel_path}")
        print("  ", end="", flush=True)

        success, output, exec_time = run_notebook(notebook)
        results[notebook] = (success, output, exec_time)

        if success:
            print(f"✓ SUCCESS ({format_time(exec_time)})")
            successful.append(notebook)
        else:
            print(f"✗ FAILED ({format_time(exec_time)})")
            failed.append(notebook)

    total_time = time.time() - total_start

    # Print summary
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)

    print(f"\nTotal notebooks run: {len(notebooks)}")
    print(f"Successful: {len(successful)} ({len(successful) * 100 / len(notebooks):.1f}%)")
    print(f"Failed: {len(failed)} ({len(failed) * 100 / len(notebooks):.1f}%)")
    print(f"Total execution time: {format_time(total_time)}")

    # List successful notebooks
    if successful:
        print("\n" + "-" * 40)
        print("SUCCESSFUL NOTEBOOKS:")
        print("-" * 40)
        for notebook in successful:
            rel_path = notebook.relative_to(base_dir)
            _, _, exec_time = results[notebook]
            print(f"  ✓ {rel_path} ({format_time(exec_time)})")

    # List failed notebooks with error details
    if failed:
        print("\n" + "-" * 40)
        print("FAILED NOTEBOOKS:")
        print("-" * 40)
        for notebook in failed:
            rel_path = notebook.relative_to(base_dir)
            _, error, exec_time = results[notebook]
            print(f"\n  ✗ {rel_path} ({format_time(exec_time)})")
            print("    Error:")
            for line in error.split("\n"):
                print(f"      {line}")

    # Print detailed output for all notebooks if requested
    if "--verbose" in sys.argv:
        print("\n" + "=" * 80)
        print("DETAILED OUTPUT")
        print("=" * 80)
        for notebook in notebooks:
            rel_path = notebook.relative_to(base_dir)
            success, output, exec_time = results[notebook]
            status = "SUCCESS" if success else "FAILED"
            print(f"\n{rel_path} [{status}] ({format_time(exec_time)}):")
            print("-" * 40)
            print(output)

    print("\n" + "=" * 80)
    print(f"Completed at: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 80)

    # Return exit code based on whether all notebooks passed
    return 0 if len(failed) == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
