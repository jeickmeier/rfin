"""Persistence layer for Finstack domain objects.

This module provides a typed repository interface for storing and retrieving
market contexts, instruments, portfolios, scenarios, statement models, and
metric registries. The default implementation uses SQLite.

Examples:
    >>> from finstack.io import SqliteStore
    >>> from datetime import date
    >>> # Open or create a database
    >>> store = SqliteStore.open("finstack.db")
    >>> # Store a market context
    >>> from finstack.core.market_data import MarketContext
    >>> market = MarketContext.empty()
    >>> store.put_market_context("USD_MKT", date(2024, 1, 1), market)
    >>> # Retrieve it later
    >>> retrieved = store.get_market_context("USD_MKT", date(2024, 1, 1))
"""

from datetime import date
from typing import Any, Optional, overload

from finstack.core.market_data.context import MarketContext
from finstack.portfolio import Portfolio
from finstack.scenarios import ScenarioSpec
from finstack.statements import FinancialModelSpec
from finstack.statements.registry import MetricRegistry

__all__ = [
    "IoError",
    "NotFoundError",
    "SchemaVersionError",
    "SqliteStore",
    "MarketContextSnapshot",
    "PortfolioSnapshot",
    "PortfolioSpec",
]

# =============================================================================
# Exceptions
# =============================================================================

class IoError(Exception):
    """Base exception for IO-related errors.

    This inherits from FinstackError at runtime.
    """

    ...

class NotFoundError(IoError):
    """Raised when a requested entity is not found in the store."""

    ...

class SchemaVersionError(IoError):
    """Raised when the database schema version is unsupported."""

    ...

# =============================================================================
# Types
# =============================================================================

class MarketContextSnapshot:
    """A time-indexed market context snapshot returned from lookback queries.

    This represents a market context at a specific point in time, useful for
    historical analysis and time-series operations.

    Examples:
        >>> from finstack.io import SqliteStore
        >>> from datetime import date
        >>> store = SqliteStore.open("data.db")
        >>> snapshots = store.list_market_contexts("USD", date(2024, 1, 1), date(2024, 12, 31))
        >>> for snap in snapshots:
        ...     print(f"{snap.as_of}: {snap.context}")
    """

    @property
    def as_of(self) -> date:
        """The as-of date for this snapshot."""
        ...

    @property
    def context(self) -> MarketContext:
        """The market context snapshot."""
        ...

class PortfolioSpec:
    """A serializable portfolio specification.

    This is a JSON-serializable representation of a portfolio that can be stored
    and retrieved from the database. Use `Portfolio.from_spec()` to hydrate it
    into a full `Portfolio` object.

    Examples:
        >>> from finstack.io import SqliteStore, PortfolioSpec
        >>> store = SqliteStore.open("data.db")
        >>> spec = store.get_portfolio_spec("FUND_A", date(2024, 1, 1))
        >>> spec.id
        'FUND_A'
    """

    @property
    def id(self) -> str:
        """Portfolio identifier."""
        ...

    @property
    def name(self) -> Optional[str]:
        """Human-readable name."""
        ...

    @property
    def base_ccy(self) -> str:
        """Base currency for aggregation (as string)."""
        ...

    @property
    def as_of(self) -> date:
        """Valuation date."""
        ...

    @property
    def position_count(self) -> int:
        """Number of positions."""
        ...

    @property
    def entity_count(self) -> int:
        """Number of entities."""
        ...

    def to_dict(self) -> dict[str, Any]:
        """Convert to JSON-compatible dict.

        Returns:
            dict: The portfolio spec as a dictionary.
        """
        ...

    @staticmethod
    def from_dict(data: dict[str, Any]) -> "PortfolioSpec":
        """Create from JSON-compatible dict.

        Args:
            data: Dictionary containing portfolio spec data.

        Returns:
            PortfolioSpec: The deserialized portfolio spec.
        """
        ...

class PortfolioSnapshot:
    """A time-indexed portfolio snapshot returned from lookback queries.

    This represents a portfolio specification at a specific point in time,
    useful for historical analysis and time-series operations.

    Examples:
        >>> from finstack.io import SqliteStore
        >>> from datetime import date
        >>> store = SqliteStore.open("data.db")
        >>> snapshots = store.list_portfolios("FUND_A", date(2024, 1, 1), date(2024, 12, 31))
        >>> for snap in snapshots:
        ...     print(f"{snap.as_of}: {snap.spec.position_count} positions")
    """

    @property
    def as_of(self) -> date:
        """The as-of date for this snapshot."""
        ...

    @property
    def spec(self) -> PortfolioSpec:
        """The portfolio specification snapshot."""
        ...

# =============================================================================
# Store
# =============================================================================

class SqliteStore:
    """A SQLite-backed persistence store for Finstack domain objects.

    This store provides CRUD operations for market contexts, instruments, portfolios,
    scenarios, statement models, and metric registries. All operations are atomic
    and idempotent (upserts).

    Examples:
        >>> from finstack.io import SqliteStore
        >>> from datetime import date
        >>> # Open or create a database
        >>> store = SqliteStore.open("finstack.db")
        >>> # Store a market context
        >>> from finstack.core.market_data import MarketContext
        >>> market = MarketContext.empty()
        >>> store.put_market_context("USD_MKT", date(2024, 1, 1), market)
        >>> # Retrieve it later
        >>> retrieved = store.get_market_context("USD_MKT", date(2024, 1, 1))
    """

    @staticmethod
    def open(path: str) -> "SqliteStore":
        """Open or create a SQLite database at the given path.

        The database schema is automatically created and migrated on open.
        Parent directories are created if they don't exist.

        Args:
            path: Path to the SQLite database file. Use `:memory:` for an
                in-memory database.

        Returns:
            SqliteStore: The opened store instance.

        Raises:
            IoError: If the database cannot be opened or migrated.

        Examples:
            >>> store = SqliteStore.open("data/finstack.db")
            >>> store = SqliteStore.open(":memory:")  # In-memory database
        """
        ...

    @property
    def path(self) -> str:
        """Get the database file path."""
        ...

    # =========================================================================
    # Market Context Operations
    # =========================================================================

    def put_market_context(
        self,
        market_id: str,
        as_of: date,
        context: MarketContext,
        meta: Optional[dict[str, Any]] = None,
    ) -> None:
        """Store a market context snapshot.

        If a market context with the same ID and as_of date exists, it is replaced.

        Args:
            market_id: Unique identifier for the market context.
            as_of: Valuation date for the snapshot.
            context: The market context to store.
            meta: Optional metadata dict for provenance tracking.

        Examples:
            >>> store.put_market_context("USD_MKT", date(2024, 1, 1), market)
        """
        ...

    def get_market_context(self, market_id: str, as_of: date) -> Optional[MarketContext]:
        """Retrieve a market context snapshot.

        Args:
            market_id: Market context identifier.
            as_of: Valuation date to retrieve.

        Returns:
            MarketContext or None: The market context if found.

        Examples:
            >>> market = store.get_market_context("USD_MKT", date(2024, 1, 1))
        """
        ...

    def load_market_context(self, market_id: str, as_of: date) -> MarketContext:
        """Load a market context, raising an error if not found.

        Args:
            market_id: Market context identifier.
            as_of: Valuation date to retrieve.

        Returns:
            MarketContext: The market context.

        Raises:
            NotFoundError: If the market context is not found.
        """
        ...

    # =========================================================================
    # Instrument Operations
    # =========================================================================

    def put_instrument(
        self,
        instrument_id: str,
        instrument: dict[str, Any],
        meta: Optional[dict[str, Any]] = None,
    ) -> None:
        """Store an instrument definition.

        Instruments are stored as JSON and can be any supported instrument type.

        Args:
            instrument_id: Unique identifier for the instrument.
            instrument: Instrument definition as a dict (JSON-serializable).
            meta: Optional metadata dict.

        Examples:
            >>> instrument = {"type": "Deposit", "currency": "USD", ...}
            >>> store.put_instrument("DEP_1M_USD", instrument)
        """
        ...

    def get_instrument(self, instrument_id: str) -> Optional[dict[str, Any]]:
        """Retrieve an instrument definition.

        Args:
            instrument_id: Instrument identifier.

        Returns:
            dict or None: The instrument as a dict if found.

        Examples:
            >>> instr = store.get_instrument("DEP_1M_USD")
            >>> if instr:
            ...     print(instr["type"])
        """
        ...

    # =========================================================================
    # Portfolio Operations
    # =========================================================================

    def put_portfolio_spec(
        self,
        portfolio_id: str,
        as_of: date,
        spec: PortfolioSpec | dict[str, Any],
        meta: Optional[dict[str, Any]] = None,
    ) -> None:
        """Store a portfolio specification.

        Args:
            portfolio_id: Unique identifier for the portfolio.
            as_of: Valuation date for the snapshot.
            spec: Portfolio specification (PortfolioSpec or dict).
            meta: Optional metadata dict.

        Examples:
            >>> store.put_portfolio_spec("FUND_A", date(2024, 1, 1), spec)
        """
        ...

    def get_portfolio_spec(self, portfolio_id: str, as_of: date) -> Optional[PortfolioSpec]:
        """Retrieve a portfolio specification.

        Args:
            portfolio_id: Portfolio identifier.
            as_of: Valuation date.

        Returns:
            PortfolioSpec or None: The portfolio spec if found.
        """
        ...

    def load_portfolio(self, portfolio_id: str, as_of: date) -> Portfolio:
        """Load and hydrate a portfolio.

        This loads the portfolio spec and resolves any missing instrument definitions
        from the instrument registry.

        Args:
            portfolio_id: Portfolio identifier.
            as_of: Valuation date.

        Returns:
            Portfolio: The hydrated portfolio.

        Raises:
            NotFoundError: If the portfolio or required instruments are not found.
        """
        ...

    def load_portfolio_with_market(
        self, portfolio_id: str, market_id: str, as_of: date
    ) -> tuple[Portfolio, MarketContext]:
        """Load a portfolio and matching market context.

        Convenience method to load both a portfolio and its corresponding market
        context for the same as_of date.

        Args:
            portfolio_id: Portfolio identifier.
            market_id: Market context identifier.
            as_of: Valuation date.

        Returns:
            tuple[Portfolio, MarketContext]: The portfolio and market context.
        """
        ...

    # =========================================================================
    # Scenario Operations
    # =========================================================================

    def put_scenario(
        self,
        scenario_id: str,
        spec: ScenarioSpec | dict[str, Any],
        meta: Optional[dict[str, Any]] = None,
    ) -> None:
        """Store a scenario specification.

        Args:
            scenario_id: Unique identifier for the scenario.
            spec: Scenario specification.
            meta: Optional metadata dict.
        """
        ...

    def get_scenario(self, scenario_id: str) -> Optional[ScenarioSpec]:
        """Retrieve a scenario specification.

        Args:
            scenario_id: Scenario identifier.

        Returns:
            ScenarioSpec or None: The scenario spec if found.
        """
        ...

    # =========================================================================
    # Statement Model Operations
    # =========================================================================

    def put_statement_model(
        self,
        model_id: str,
        spec: FinancialModelSpec | dict[str, Any],
        meta: Optional[dict[str, Any]] = None,
    ) -> None:
        """Store a financial statement model specification.

        Args:
            model_id: Unique identifier for the model.
            spec: Financial model specification.
            meta: Optional metadata dict.
        """
        ...

    def get_statement_model(self, model_id: str) -> Optional[FinancialModelSpec]:
        """Retrieve a financial statement model specification.

        Args:
            model_id: Model identifier.

        Returns:
            FinancialModelSpec or None: The model spec if found.
        """
        ...

    # =========================================================================
    # Metric Registry Operations
    # =========================================================================

    def put_metric_registry(
        self,
        namespace: str,
        registry: MetricRegistry | dict[str, Any],
        meta: Optional[dict[str, Any]] = None,
    ) -> None:
        """Store a metric registry.

        Args:
            namespace: Registry namespace (e.g., "fin", "custom").
            registry: The metric registry.
            meta: Optional metadata dict.
        """
        ...

    def get_metric_registry(self, namespace: str) -> Optional[MetricRegistry]:
        """Retrieve a metric registry.

        Args:
            namespace: Registry namespace.

        Returns:
            MetricRegistry or None: The registry if found.
        """
        ...

    def load_metric_registry(self, namespace: str) -> MetricRegistry:
        """Load a metric registry, raising an error if not found.

        Args:
            namespace: Registry namespace.

        Returns:
            MetricRegistry: The registry.

        Raises:
            NotFoundError: If the registry is not found.
        """
        ...

    def list_metric_registries(self) -> list[str]:
        """List all metric registry namespaces.

        Returns:
            list[str]: List of namespace names.
        """
        ...

    def delete_metric_registry(self, namespace: str) -> bool:
        """Delete a metric registry.

        Args:
            namespace: Registry namespace to delete.

        Returns:
            bool: True if the registry was deleted, False if not found.
        """
        ...

    # =========================================================================
    # Bulk Operations
    # =========================================================================

    def put_instruments_batch(
        self,
        instruments: list[tuple[str, dict[str, Any]] | tuple[str, dict[str, Any], dict[str, Any] | None]],
    ) -> None:
        """Store multiple instruments in a single transaction.

        This is more efficient than calling put_instrument repeatedly.

        Args:
            instruments: List of (instrument_id, instrument_dict) tuples,
                or (instrument_id, instrument_dict, meta_dict) tuples.

        Examples:
            >>> instruments = [
            ...     ("DEP_1M", {"type": "Deposit", ...}),
            ...     ("DEP_3M", {"type": "Deposit", ...}),
            ... ]
            >>> store.put_instruments_batch(instruments)
        """
        ...

    def put_market_contexts_batch(
        self,
        contexts: list[tuple[str, date, MarketContext] | tuple[str, date, MarketContext, dict[str, Any] | None]],
    ) -> None:
        """Store multiple market contexts in a single transaction.

        Args:
            contexts: List of (market_id, as_of, context) tuples,
                or (market_id, as_of, context, meta) tuples.
        """
        ...

    def put_portfolio_specs_batch(
        self,
        portfolios: list[
            tuple[str, date, PortfolioSpec | dict[str, Any]]
            | tuple[str, date, PortfolioSpec | dict[str, Any], dict[str, Any] | None]
        ],
    ) -> None:
        """Store multiple portfolio specs in a single transaction.

        Args:
            portfolios: List of (portfolio_id, as_of, spec) tuples,
                or (portfolio_id, as_of, spec, meta) tuples.
        """
        ...

    # =========================================================================
    # Lookback Operations
    # =========================================================================

    def list_market_contexts(self, market_id: str, start: date, end: date) -> list[MarketContextSnapshot]:
        """List market context snapshots in a date range.

        Args:
            market_id: Market context identifier.
            start: Start date (inclusive).
            end: End date (inclusive).

        Returns:
            list[MarketContextSnapshot]: Snapshots ordered by as_of date.
        """
        ...

    def latest_market_context_on_or_before(self, market_id: str, as_of: date) -> Optional[MarketContextSnapshot]:
        """Get the latest market context on or before a date.

        Args:
            market_id: Market context identifier.
            as_of: Maximum date to search.

        Returns:
            MarketContextSnapshot or None: The latest snapshot if found.
        """
        ...

    def list_portfolios(self, portfolio_id: str, start: date, end: date) -> list[PortfolioSnapshot]:
        """List portfolio snapshots in a date range.

        Args:
            portfolio_id: Portfolio identifier.
            start: Start date (inclusive).
            end: End date (inclusive).

        Returns:
            list[PortfolioSnapshot]: Snapshots ordered by as_of date.
        """
        ...

    def latest_portfolio_on_or_before(self, portfolio_id: str, as_of: date) -> Optional[PortfolioSnapshot]:
        """Get the latest portfolio on or before a date.

        Args:
            portfolio_id: Portfolio identifier.
            as_of: Maximum date to search.

        Returns:
            PortfolioSnapshot or None: The latest snapshot if found.
        """
        ...
