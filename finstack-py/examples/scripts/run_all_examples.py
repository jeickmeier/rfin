#!/usr/bin/env python3
"""
Script to run all example scripts and report their status.
"""

import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Tuple
import time
from datetime import datetime


def find_python_scripts(base_dir: Path) -> List[Path]:
    """Find all Python scripts in the examples/scripts directory."""
    scripts = []
    
    # Core scripts
    core_dir = base_dir / "core"
    if core_dir.exists():
        scripts.extend(sorted(core_dir.glob("*.py")))
    
    # Valuations scripts
    valuations_dir = base_dir / "valuations"
    if valuations_dir.exists():
        scripts.extend(sorted(valuations_dir.glob("*.py")))

    # Statements scripts
    statements_dir = base_dir / "statements"
    if statements_dir.exists():
        scripts.extend(sorted(statements_dir.glob("*.py")))

    # Scenarios scripts
    scenarios_dir = base_dir / "scenarios"
    if scenarios_dir.exists():
        scripts.extend(sorted(scenarios_dir.glob("*.py")))

    # Portfolio scripts
    portfolio_dir = base_dir / "portfolio"
    if portfolio_dir.exists():
        scripts.extend(sorted(portfolio_dir.glob("*.py")))
    
    # Exclude this script itself if it's in the same directory
    script_name = Path(__file__).name
    scripts = [s for s in scripts if s.name != script_name]
    
    # Exclude slow Monte Carlo examples (100k paths each, can take 30-60s)
    # These are comprehensive demonstrations but too slow for routine testing
    # Run them manually when needed: uv run python finstack-py/examples/scripts/valuations/<example>.py
    excluded_patterns = [
        "asian_option_example.py",
        "barrier_option_example.py",
        "lookback_option_example.py",
        "cliquet_option_example.py",
        "range_accrual_example.py",
        "autocallable_example.py",
        "quanto_option_example.py",
        "cms_option_example.py",
    ]
    scripts = [s for s in scripts if s.name not in excluded_patterns]
    
    return scripts


def run_script(script_path: Path) -> Tuple[bool, str, float]:
    """
    Run a Python script and return success status, output/error, and execution time.
    
    Returns:
        Tuple of (success, output_or_error, execution_time_seconds)
    """
    start_time = time.time()
    
    try:
        # Run the script with uv run as per user preference
        result = subprocess.run(
            ["uv", "run", "python", str(script_path)],
            capture_output=True,
            text=True,
            timeout=60,  # 60 second timeout per script (increased for calibration examples)
            cwd=script_path.parent.parent.parent.parent  # Run from project root
        )
        
        execution_time = time.time() - start_time
        
        if result.returncode == 0:
            # Get last few lines of output for summary
            output_lines = result.stdout.strip().split('\n')
            summary = '\n'.join(output_lines[-3:]) if output_lines else "No output"
            return True, summary, execution_time
        else:
            # Get error message
            error = result.stderr.strip() or result.stdout.strip()
            error_lines = error.split('\n')
            # Get the most relevant error lines
            summary = '\n'.join(error_lines[-5:]) if error_lines else "Unknown error"
            return False, summary, execution_time
            
    except subprocess.TimeoutExpired:
        execution_time = time.time() - start_time
        return False, "Script timed out (>60s)", execution_time
    except Exception as e:
        execution_time = time.time() - start_time
        return False, f"Exception: {str(e)}", execution_time


def format_time(seconds: float) -> str:
    """Format execution time nicely."""
    if seconds < 1:
        return f"{seconds*1000:.0f}ms"
    return f"{seconds:.2f}s"


def main():
    """Main function to run all scripts and report results."""
    
    print("=" * 80)
    print("FINSTACK EXAMPLE SCRIPTS TEST RUNNER")
    print("=" * 80)
    print(f"Started at: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print()
    
    # Find all scripts
    base_dir = Path(__file__).parent
    scripts = find_python_scripts(base_dir)
    
    if not scripts:
        print("No example scripts found!")
        return 1
    
    print(f"Found {len(scripts)} example scripts to run:")
    for script in scripts:
        rel_path = script.relative_to(base_dir)
        print(f"  - {rel_path}")
    print()
    print("-" * 80)
    
    # Run each script and collect results
    results: Dict[Path, Tuple[bool, str, float]] = {}
    successful = []
    failed = []
    
    total_start = time.time()
    
    for i, script in enumerate(scripts, 1):
        rel_path = script.relative_to(base_dir)
        print(f"\n[{i}/{len(scripts)}] Running: {rel_path}")
        print("  ", end="", flush=True)
        
        success, output, exec_time = run_script(script)
        results[script] = (success, output, exec_time)
        
        if success:
            print(f"✓ SUCCESS ({format_time(exec_time)})")
            successful.append(script)
        else:
            print(f"✗ FAILED ({format_time(exec_time)})")
            failed.append(script)
    
    total_time = time.time() - total_start
    
    # Print summary
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)
    
    print(f"\nTotal scripts run: {len(scripts)}")
    print(f"Successful: {len(successful)} ({len(successful)*100/len(scripts):.1f}%)")
    print(f"Failed: {len(failed)} ({len(failed)*100/len(scripts):.1f}%)")
    print(f"Total execution time: {format_time(total_time)}")
    
    # List successful scripts
    if successful:
        print("\n" + "-" * 40)
        print("SUCCESSFUL SCRIPTS:")
        print("-" * 40)
        for script in successful:
            rel_path = script.relative_to(base_dir)
            _, _, exec_time = results[script]
            print(f"  ✓ {rel_path} ({format_time(exec_time)})")
    
    # List failed scripts with error details
    if failed:
        print("\n" + "-" * 40)
        print("FAILED SCRIPTS:")
        print("-" * 40)
        for script in failed:
            rel_path = script.relative_to(base_dir)
            _, error, exec_time = results[script]
            print(f"\n  ✗ {rel_path} ({format_time(exec_time)})")
            print("    Error:")
            for line in error.split('\n'):
                print(f"      {line}")
    
    # Print detailed output for all scripts if requested
    if "--verbose" in sys.argv:
        print("\n" + "=" * 80)
        print("DETAILED OUTPUT")
        print("=" * 80)
        for script in scripts:
            rel_path = script.relative_to(base_dir)
            success, output, exec_time = results[script]
            status = "SUCCESS" if success else "FAILED"
            print(f"\n{rel_path} [{status}] ({format_time(exec_time)}):")
            print("-" * 40)
            print(output)
    
    print("\n" + "=" * 80)
    print(f"Completed at: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 80)
    
    # Return exit code based on whether all scripts passed
    return 0 if len(failed) == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
