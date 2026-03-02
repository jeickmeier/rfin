"""Monte Carlo configuration and results for Merton structural credit pricing."""

from __future__ import annotations
from typing import Sequence
from .merton import MertonModel
from .endogenous_hazard import EndogenousHazardSpec
from .dynamic_recovery import DynamicRecoverySpec
from .toggle_exercise import ToggleExerciseModel

PikScheduleInput = str | Sequence[tuple[float, str | dict[str, float]]] | None
"""Accepted forms for ``pik_schedule``:

- ``None`` — derived from the bond's coupon type
- ``"cash"`` / ``"pik"`` / ``"toggle"`` — uniform mode for all coupons
- ``[(0.0, "pik"), (2.0, "cash")]`` — stepped schedule (time in years, mode)
- ``[(0.0, "toggle"), (3.0, {"cash": 0.5, "pik": 0.5})]`` — mixed schedule
"""

class MertonMcConfig:
    """Monte Carlo configuration for Merton structural credit pricing.

    Bundles a MertonModel with optional credit extensions, a PIK schedule,
    and simulation parameters.
    """

    def __init__(
        self,
        merton: MertonModel,
        *,
        pik_schedule: PikScheduleInput = None,
        endogenous_hazard: EndogenousHazardSpec | None = None,
        dynamic_recovery: DynamicRecoverySpec | None = None,
        toggle_model: ToggleExerciseModel | None = None,
        num_paths: int = 10_000,
        seed: int = 42,
        antithetic: bool = True,
        time_steps_per_year: int = 12,
    ) -> None: ...
    @property
    def num_paths(self) -> int: ...
    @property
    def seed(self) -> int: ...
    @property
    def antithetic(self) -> bool: ...
    @property
    def time_steps_per_year(self) -> int: ...
    def __repr__(self) -> str: ...

class MertonMcResult:
    """Result from Monte Carlo Merton structural credit pricing.

    Contains clean/dirty prices, loss metrics, spread, and path statistics.
    All properties are read-only.
    """

    @property
    def clean_price_pct(self) -> float: ...
    @property
    def dirty_price_pct(self) -> float: ...
    @property
    def expected_loss(self) -> float: ...
    @property
    def unexpected_loss(self) -> float: ...
    @property
    def expected_shortfall_95(self) -> float: ...
    @property
    def average_pik_fraction(self) -> float: ...
    @property
    def effective_spread_bp(self) -> float: ...
    @property
    def default_rate(self) -> float: ...
    @property
    def avg_default_time(self) -> float: ...
    @property
    def avg_terminal_notional(self) -> float: ...
    @property
    def avg_recovery_pct(self) -> float: ...
    @property
    def pik_exercise_rate(self) -> float: ...
    @property
    def num_paths(self) -> int: ...
    @property
    def standard_error(self) -> float: ...
    def __repr__(self) -> str: ...
