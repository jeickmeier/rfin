"""Domain runner for flattened attribution golden fixtures."""

from __future__ import annotations

from tests.golden.runners import reject_flattened_outputs, validate_source_validation_fixture
from tests.golden.schema import GoldenFixture


def run(fixture: GoldenFixture) -> dict[str, float]:
    """Require executable attribution inputs before running."""
    source_validation = validate_source_validation_fixture("attribution runner", fixture)
    if not source_validation:
        return reject_flattened_outputs("attribution runner", fixture)

    actuals = {key: float(value) for key, value in fixture.inputs.get("components", {}).items()}
    pending = dict(fixture.inputs.get("sums", {}))
    while pending:
        ready = {
            output: sum(actuals[term] for term in terms)
            for output, terms in pending.items()
            if all(term in actuals for term in terms)
        }
        if not ready:
            unresolved = {output: [term for term in terms if term not in actuals] for output, terms in pending.items()}
            msg = f"attribution sums contain unresolved references: {unresolved}"
            raise ValueError(msg)
        for output, total in ready.items():
            actuals[output] = total
            del pending[output]
    return {}
