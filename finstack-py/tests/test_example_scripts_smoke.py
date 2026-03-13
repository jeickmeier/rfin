from __future__ import annotations

from pathlib import Path
import subprocess
import sys

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]

FAILING_EXAMPLE_SCRIPTS = [
    "finstack-py/examples/scripts/cookbook/02_stress_test.py",
    "finstack-py/examples/scripts/cookbook/21_full_portfolio_workflow.py",
    "finstack-py/examples/scripts/valuations/instruments/basket_capabilities.py",
    "finstack-py/examples/scripts/valuations/instruments/convertible_capabilities.py",
    "finstack-py/examples/scripts/valuations/instruments/equity_capabilities.py",
    "finstack-py/examples/scripts/valuations/instruments/inflation_capabilities.py",
    "finstack-py/examples/scripts/valuations/instruments/private_markets_capabilities.py",
]


@pytest.mark.parametrize("relative_script", FAILING_EXAMPLE_SCRIPTS)
def test_example_script_runs(relative_script: str) -> None:
    """Smoke test example scripts against the current public Python API."""
    script_path = REPO_ROOT / relative_script

    # The parametrized script names come from FAILING_EXAMPLE_SCRIPTS above,
    # a fixed allowlist of repository-owned example scripts.
    result = subprocess.run(  # noqa: S603
        [sys.executable, str(script_path)],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )

    assert result.returncode == 0, (
        f"{relative_script} failed with exit code {result.returncode}\n"
        f"stdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )
