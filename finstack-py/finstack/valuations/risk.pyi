"""Value-at-Risk (VaR) calculation and risk metrics."""

from typing import List, Optional, Union, Dict, Any, Tuple, Sequence
from datetime import date
from ...core.market_data.context import MarketContext
from .instruments import Bond  # Use a representative instrument for typing if needed
from .common import InstrumentType

class VarMethod:
    """VaR calculation method."""

    FULL_REVALUATION: VarMethod
    TAYLOR_APPROXIMATION: VarMethod

class VarConfig:
    """Configuration for VaR calculation."""
    def __init__(self, confidence_level: float = 0.95, method: Optional[VarMethod] = None) -> None: ...
    @staticmethod
    def var_95() -> VarConfig: ...
    @staticmethod
    def var_99() -> VarConfig: ...
    @property
    def confidence_level(self) -> float: ...
    @property
    def method(self) -> VarMethod: ...

class VarResult:
    """Result of VaR calculation."""
    @property
    def var(self) -> float: ...
    @property
    def expected_shortfall(self) -> float: ...
    @property
    def pnl_distribution(self) -> List[float]: ...
    @property
    def confidence_level(self) -> float: ...
    @property
    def num_scenarios(self) -> int: ...

class RiskFactorType:
    """Risk factor type for VaR scenarios."""
    @staticmethod
    def discount_rate(curve_id: str, tenor_years: float) -> RiskFactorType: ...
    @staticmethod
    def forward_rate(curve_id: str, tenor_years: float) -> RiskFactorType: ...
    @staticmethod
    def credit_spread(curve_id: str, tenor_years: float) -> RiskFactorType: ...

class RiskFactorShift:
    """Single risk factor shift for a scenario."""
    def __init__(self, factor: RiskFactorType, shift: float) -> None: ...
    @property
    def shift(self) -> float: ...

class MarketScenario:
    """Historical market scenario for a single date."""
    def __init__(self, date: Tuple[int, int, int], shifts: List[RiskFactorShift]) -> None: ...

class MarketHistory:
    """Historical market data for VaR calculation."""
    def __init__(self, base_date: Tuple[int, int, int], window_days: int, scenarios: List[MarketScenario]) -> None: ...
    @property
    def num_scenarios(self) -> int: ...

def calculate_var(
    instruments: Union[Any, Sequence[Any]],
    market: MarketContext,
    history: MarketHistory,
    as_of: Tuple[int, int, int],
    config: VarConfig,
) -> VarResult:
    """Calculate Historical VaR for one or more instruments."""
    ...

def krd_dv01_ladder(instrument: Any, market: MarketContext, as_of: Union[date, str]) -> Dict[str, Any]:
    """Compute key-rate DV01 ladder."""
    ...

def cs01_ladder(instrument: Any, market: MarketContext, as_of: Union[date, str]) -> Dict[str, Any]:
    """Compute key-rate CS01 ladder."""
    ...

__all__ = [
    "VarMethod",
    "VarConfig",
    "VarResult",
    "RiskFactorType",
    "RiskFactorShift",
    "MarketScenario",
    "MarketHistory",
    "calculate_var",
    "krd_dv01_ladder",
    "cs01_ladder",
]
