"""Domain runner for `rates.irs` golden fixtures.

The real IRS pricing adapter lands in Phase 2 with the first IRS fixture.
"""

from __future__ import annotations

from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Run one IRS fixture through the Python bindings."""
    raise NotImplementedError("pricing_irs.run is implemented in Phase 2")
