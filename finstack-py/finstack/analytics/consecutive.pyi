"""Type stubs for consecutive streak counting functions."""

from __future__ import annotations

def count_consecutive_positive(values: list[float]) -> int:
    """Longest consecutive streak of positive values."""
    ...

def count_consecutive_negative(values: list[float]) -> int:
    """Longest consecutive streak of negative values."""
    ...

def count_consecutive_above(values: list[float], threshold: float) -> int:
    """Longest consecutive streak of values above a threshold."""
    ...

def count_consecutive_below(values: list[float], threshold: float) -> int:
    """Longest consecutive streak of values below a threshold."""
    ...
