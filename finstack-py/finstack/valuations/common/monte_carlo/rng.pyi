"""Philox 4x32-10 counter-based random number generator."""

from __future__ import annotations

class PhiloxRng:
    """Philox 4x32-10 counter-based PRNG.

    Parallel-friendly, reproducible random number generator used by the
    Monte Carlo engine.
    """

    def __init__(self, seed: int) -> None: ...

    @staticmethod
    def from_string(seed_str: str) -> PhiloxRng:
        """Create a deterministic RNG from a human-readable string seed."""
        ...

    @property
    def seed(self) -> int: ...

    def uniform(self, n: int) -> list[float]:
        """Generate *n* uniform random numbers in [0, 1)."""
        ...

    def standard_normal(self, n: int) -> list[float]:
        """Generate *n* standard-normal random numbers."""
        ...
