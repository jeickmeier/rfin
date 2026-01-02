"""Statement metric normalization helpers (type stubs).

This module is implemented by the compiled Rust extension and registered as a
submodule under :mod:`finstack.statements` at runtime. We ship this `.pyi` so
static analyzers (pyright/basedpyright) can resolve
`finstack.statements.adjustments` and provide type information.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

class NormalizationConfig:
    def __init__(self, target_node: str) -> None: ...
    def add_adjustment(self, adjustment: Adjustment) -> None: ...

class Adjustment:
    @staticmethod
    def fixed(id: str, name: str, amounts: Mapping[str, float]) -> Adjustment: ...
    @staticmethod
    def percentage(id: str, name: str, node_id: str, percentage: float) -> Adjustment: ...
    def with_cap(self, base_node: str | None, value: float) -> Adjustment: ...

class AppliedAdjustment:
    @property
    def name(self) -> str: ...
    @property
    def raw_amount(self) -> float: ...
    @property
    def capped_amount(self) -> float: ...
    @property
    def is_capped(self) -> bool: ...

class NormalizationResult:
    @property
    def period(self) -> str: ...
    @property
    def base_value(self) -> float: ...
    @property
    def final_value(self) -> float: ...
    @property
    def adjustments(self) -> list[AppliedAdjustment]: ...

class NormalizationEngine:
    @staticmethod
    def normalize(results: Any, config: Any) -> list[NormalizationResult]: ...
    @staticmethod
    def merge_into_results(
        results: Any,
        normalization_results: Sequence[NormalizationResult],
        output_node_id: str,
    ) -> None: ...

__all__ = [
    "Adjustment",
    "AppliedAdjustment",
    "NormalizationConfig",
    "NormalizationEngine",
    "NormalizationResult",
]
