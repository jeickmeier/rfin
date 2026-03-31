"""Portfolio dependency index for selective repricing."""

from __future__ import annotations
from typing import List, Literal, Optional

from finstack.core.currency import Currency
from finstack.portfolio.portfolio import Portfolio

class MarketFactorKey:
    """Normalized market factor key for portfolio-level dependency tracking.

    Use static constructors to create instances.

    Examples:
        >>> key = MarketFactorKey.curve("USD-OIS", "discount")
        >>> key = MarketFactorKey.spot("SPX")
        >>> key = MarketFactorKey.fx("EUR", "USD")
    """

    @staticmethod
    def curve(id: str, kind: Literal["discount", "forward", "credit"]) -> MarketFactorKey:
        """Create a curve market factor key.

        Args:
            id: Curve identifier (e.g. ``"USD-OIS"``).
            kind: Curve kind — ``"discount"``, ``"forward"``, or ``"credit"``.
        """
        ...

    @staticmethod
    def spot(id: str) -> MarketFactorKey:
        """Create a spot market factor key.

        Args:
            id: Spot identifier (e.g. ``"SPX"``).
        """
        ...

    @staticmethod
    def vol_surface(id: str) -> MarketFactorKey:
        """Create a volatility surface market factor key.

        Args:
            id: Vol surface identifier.
        """
        ...

    @staticmethod
    def fx(base: Currency | str, quote: Currency | str) -> MarketFactorKey:
        """Create an FX pair market factor key.

        Args:
            base: Base currency.
            quote: Quote currency.
        """
        ...

    @staticmethod
    def series(id: str) -> MarketFactorKey:
        """Create a time-series market factor key.

        Args:
            id: Series identifier.
        """
        ...

    @property
    def variant(self) -> Literal["curve", "spot", "vol_surface", "fx", "series"]:
        """The variant name."""
        ...

    @property
    def curve_id(self) -> Optional[str]:
        """The curve identifier (only for Curve variant)."""
        ...

    @property
    def curve_kind(self) -> Optional[Literal["discount", "forward", "credit"]]:
        """The curve kind label (only for Curve variant)."""
        ...

    @property
    def id(self) -> Optional[str]:
        """The spot/vol_surface/series identifier."""
        ...

    @property
    def base(self) -> Optional[str]:
        """The base currency code (only for Fx variant)."""
        ...

    @property
    def quote_ccy(self) -> Optional[str]:
        """The quote currency code (only for Fx variant)."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...


class DependencyIndex:
    """Inverted index mapping market factor keys to affected portfolio positions.

    Examples:
        >>> index = DependencyIndex.build(portfolio)
        >>> keys = index.factors()
        >>> affected = index.affected_positions([MarketFactorKey.spot("SPX")])
    """

    @staticmethod
    def build(portfolio: Portfolio) -> DependencyIndex:
        """Build a dependency index from a portfolio.

        Args:
            portfolio: Portfolio to index.

        Returns:
            Newly built dependency index.
        """
        ...

    def affected_positions(self, keys: List[MarketFactorKey]) -> List[int]:
        """Get sorted position indices affected by any of the given keys.

        Unresolved positions are always included.

        Args:
            keys: Market factor keys to query.

        Returns:
            Sorted position indices.
        """
        ...

    def factors(self) -> List[MarketFactorKey]:
        """Return all tracked market factor keys.

        Returns:
            All normalized factor keys in the index.
        """
        ...

    def positions_for_factor(self, key: MarketFactorKey) -> Optional[List[int]]:
        """Look up position indices for a single market factor key.

        Args:
            key: Market factor key to look up.

        Returns:
            Position indices, or ``None`` if the key is absent.
        """
        ...

    def unresolved(self) -> List[int]:
        """Return unresolved position indices.

        Returns:
            Positions whose instruments failed to report dependencies.
        """
        ...

    @property
    def factor_count(self) -> int:
        """Number of distinct market factor keys tracked."""
        ...

    def __len__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def __repr__(self) -> str: ...
