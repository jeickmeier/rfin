"""Portfolio margin aggregation bindings."""

from __future__ import annotations
from typing import Dict, Iterator, List, Tuple, Any
from datetime import date
from finstack.core.currency import Currency
from finstack.core.money import Money
from finstack.core.market_data.context import MarketContext
from .types import Position
from .portfolio import Portfolio

DateLike = str | date
SimmSensitivities = Any

class NettingSetId:
    """Identifier for a margin netting set (CSA or CCP)."""

    counterparty_id: str
    csa_id: str | None
    ccp_id: str | None

    @staticmethod
    def bilateral(counterparty_id: str, csa_id: str) -> "NettingSetId": ...
    @staticmethod
    def cleared(ccp_id: str) -> "NettingSetId": ...
    def is_cleared(self) -> bool: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class NettingSet:
    """Netting set containing positions for margin aggregation."""

    def __init__(self, id: NettingSetId) -> None: ...
    @property
    def id(self) -> str:
        """Netting set identifier as string."""
        ...

    def add_position(self, position_id: str) -> None:
        """Attach a position id to the netting set."""
        ...

    def position_count(self) -> int:
        """Number of positions in this netting set."""
        ...

    def is_cleared(self) -> bool:
        """Return True if the netting set is cleared (CCP)."""
        ...

    def merge_sensitivities(self, sensitivities: SimmSensitivities) -> None:
        """Merge SIMM sensitivities into the aggregated set."""
        ...

    def __repr__(self) -> str: ...

class NettingSetMargin:
    """Margin results for a single netting set."""

    def __init__(
        self,
        netting_set_id: NettingSetId,
        as_of: date,
        initial_margin: Money,
        variation_margin: Money,
        position_count: int,
        im_methodology: str,
    ) -> None:
        """Create a NettingSetMargin result.

        Args:
            netting_set_id: Netting set identifier.
            as_of: Calculation date.
            initial_margin: IM requirement.
            variation_margin: VM requirement.
            position_count: Number of positions.
            im_methodology: IM methodology name (e.g., "Simm", "Schedule", "ClearingHouse").
        """
        ...

    @property
    def netting_set_id(self) -> str:
        """Netting set identifier."""
        ...

    @property
    def as_of(self) -> DateLike:
        """Calculation date."""
        ...

    @property
    def initial_margin(self) -> Money:
        """Initial margin requirement."""
        ...

    @property
    def variation_margin(self) -> Money:
        """Variation margin requirement."""
        ...

    @property
    def total_margin(self) -> Money:
        """Total margin (IM + positive VM)."""
        ...

    @property
    def position_count(self) -> int:
        """Number of positions included."""
        ...

    @property
    def im_methodology(self) -> str:
        """Method used (SIMM/Schedule/ClearingHouse)."""
        ...

    @property
    def sensitivities(self) -> SimmSensitivities | None:
        """Aggregated SIMM sensitivities if available."""
        ...

    @property
    def im_breakdown(self) -> Dict[str, Money]:
        """SIMM breakdown by risk class."""
        ...

    def is_cleared(self) -> bool:
        """Whether this netting set is cleared."""
        ...

    def __repr__(self) -> str: ...
    def to_json(self) -> str:
        """Serialize to JSON string."""
        ...

    @staticmethod
    def from_json(json_str: str) -> "NettingSetMargin":
        """Deserialize from JSON string."""
        ...

class NettingSetManager:
    """Organize positions into netting sets."""

    def __init__(self) -> None: ...
    def with_default_set(self, id: NettingSetId) -> "NettingSetManager":
        """Configure a default netting set for positions without explicit spec."""
        ...

    def add_position(self, position: Position, netting_set_id: NettingSetId | None = ...) -> None:
        """Add a position to a netting set (explicit or default)."""
        ...

    def count(self) -> int:
        """Number of netting sets tracked."""
        ...

    def ids(self) -> List[NettingSetId]:
        """All netting set identifiers."""
        ...

    def get(self, id: NettingSetId) -> NettingSet | None:
        """Fetch a netting set by id."""
        ...

    def __len__(self) -> int:
        """Number of netting sets."""
        ...

    def __iter__(self) -> Iterator[Tuple[NettingSetId, NettingSet]]:
        """Iterate over (NettingSetId, NettingSet) pairs."""
        ...

    def get_or_create(self, id: NettingSetId) -> NettingSet:
        """Get or create a netting set by id."""
        ...

    def merge_sensitivities(self, netting_set_id: NettingSetId, sensitivities: SimmSensitivities) -> None:
        """Merge sensitivities into a netting set."""
        ...

class PortfolioMarginResult:
    """Portfolio-wide margin calculation results."""

    @property
    def as_of(self) -> DateLike:
        """Calculation date."""
        ...

    @property
    def base_currency(self) -> Currency:
        """Base currency for totals."""
        ...

    @property
    def total_initial_margin(self) -> Money:
        """Sum of IM across all netting sets."""
        ...

    @property
    def total_variation_margin(self) -> Money:
        """Sum of VM across all netting sets."""
        ...

    @property
    def total_margin(self) -> Money:
        """Total margin requirement."""
        ...

    @property
    def by_netting_set(self) -> Dict[str, NettingSetMargin]:
        """Margin results keyed by netting set id."""
        ...

    @property
    def total_positions(self) -> int:
        """Positions included in margin calculation."""
        ...

    @property
    def positions_without_margin(self) -> int:
        """Positions not marginable (excluded)."""
        ...
    def add_netting_set(self, result: NettingSetMargin) -> None:
        """Add a netting-set result in the base currency."""
        ...
    def add_netting_set_with_fx(self, result: NettingSetMargin, fx_rate: float) -> None:
        """Add a cross-currency netting-set result with explicit FX conversion."""
        ...

    def cleared_bilateral_split(self) -> Tuple[Money, Money]:
        """Return (cleared_total, bilateral_total) margin amounts."""
        ...

    def netting_set_count(self) -> int:
        """Number of netting sets in the result."""
        ...

    def __len__(self) -> int:
        """Number of netting sets."""
        ...

    def __iter__(self) -> Iterator[Tuple[str, NettingSetMargin]]:
        """Iterate over (netting_set_id_str, NettingSetMargin) pairs."""
        ...

    def __repr__(self) -> str: ...
    def to_json(self) -> str:
        """Serialize to JSON string."""
        ...

    @staticmethod
    def from_json(json_str: str) -> "PortfolioMarginResult":
        """Deserialize from JSON string."""
        ...

class PortfolioMarginAggregator:
    """Aggregate margin requirements across a portfolio."""

    def __init__(self, base_ccy: Currency) -> None:
        """Create a margin aggregator with a base currency."""
        ...

    @staticmethod
    def from_portfolio(portfolio: Portfolio) -> "PortfolioMarginAggregator":
        """Initialize from an existing portfolio (auto-build netting sets)."""
        ...

    def add_position(self, position: Position) -> None:
        """Add a single position to aggregation."""
        ...

    def netting_set_count(self) -> int:
        """Number of netting sets being tracked."""
        ...

    def calculate(
        self,
        portfolio: Portfolio,
        market_context: MarketContext,
        as_of: DateLike,
    ) -> PortfolioMarginResult:
        """Calculate margin requirements by netting set."""
        ...
