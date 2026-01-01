"""Market Conventions Registry type stubs."""

from typing import Optional
from finstack.core.currency import Currency
from finstack.core.dates.daycount import DayCount
from finstack.core.dates.tenor import Tenor
from finstack.core.dates.calendar import BusinessDayConvention

class CdsDocClause:
    """CDS documentation clause (ISDA standard).

    Represents the restructuring clause for CDS contracts.

    Attributes:
        CR14: Cum-Restructuring 2014
        MR14: Modified-Restructuring 2014
        MM14: Modified-Modified-Restructuring 2014
        XR14: No-Restructuring 2014
        ISDA_NA: ISDA North American Corporate
        ISDA_EU: ISDA European Corporate
        ISDA_AS: ISDA Asia Corporate
        ISDA_AU: ISDA Australia Corporate
        ISDA_NZ: ISDA New Zealand Corporate
        CUSTOM: Custom / Other
    """

    CR14: CdsDocClause
    MR14: CdsDocClause
    MM14: CdsDocClause
    XR14: CdsDocClause
    ISDA_NA: CdsDocClause
    ISDA_EU: CdsDocClause
    ISDA_AS: CdsDocClause
    ISDA_AU: CdsDocClause
    ISDA_NZ: CdsDocClause
    CUSTOM: CdsDocClause

    @classmethod
    def from_name(cls, name: str) -> CdsDocClause:
        """Parse a CDS doc clause from string.

        Args:
            name: Name of the doc clause (e.g., "CR14", "ISDA_NA").

        Returns:
            CdsDocClause: Parsed doc clause.

        Raises:
            ValueError: If the name is not recognized.
        """
        ...

    @property
    def name(self) -> str:
        """Name of the doc clause."""
        ...

class CdsConventionKey:
    """Key for looking up CDS conventions (currency + doc clause).

    Args:
        currency: Currency code (e.g., "USD").
        doc_clause: CDS documentation clause.

    Examples:
        >>> key = CdsConventionKey(currency="USD", doc_clause=CdsDocClause.ISDA_NA)
    """

    def __init__(self, currency: str, doc_clause: CdsDocClause) -> None: ...
    @property
    def currency(self) -> Currency:
        """Currency of the CDS conventions."""
        ...

    @property
    def doc_clause(self) -> CdsDocClause:
        """Documentation clause."""
        ...

class RateIndexKind:
    """Type of rate index.

    Attributes:
        OVERNIGHT_RFR: Overnight Risk-Free Rate (e.g., SOFR, SONIA)
        TERM: Term index (e.g., 3M LIBOR, 6M EURIBOR)
    """

    OVERNIGHT_RFR: RateIndexKind
    TERM: RateIndexKind

    @property
    def name(self) -> str:
        """Name of the index kind."""
        ...

class RateIndexConventions:
    """Conventions for rate index instruments.

    Contains day count, payment frequency, reset lag, and other market-standard
    parameters for instruments referencing a rate index.
    """

    @property
    def currency(self) -> Currency:
        """Currency of the index."""
        ...

    @property
    def kind(self) -> RateIndexKind:
        """Type of rate index (overnight RFR or term)."""
        ...

    @property
    def tenor(self) -> Optional[Tenor]:
        """Native tenor (for term indices)."""
        ...

    @property
    def day_count(self) -> DayCount:
        """Day count convention."""
        ...

    @property
    def default_payment_frequency(self) -> Tenor:
        """Default payment frequency."""
        ...

    @property
    def default_payment_delay_days(self) -> int:
        """Default payment delay in days."""
        ...

    @property
    def default_reset_lag_days(self) -> int:
        """Default reset lag in days."""
        ...

    @property
    def market_calendar_id(self) -> str:
        """Market calendar identifier."""
        ...

    @property
    def market_settlement_days(self) -> int:
        """Market settlement days."""
        ...

    @property
    def market_business_day_convention(self) -> BusinessDayConvention:
        """Market business day convention."""
        ...

    @property
    def default_fixed_leg_day_count(self) -> DayCount:
        """Default fixed leg day count."""
        ...

    @property
    def default_fixed_leg_frequency(self) -> Tenor:
        """Default fixed leg payment frequency."""
        ...

class CdsConventions:
    """Conventions for Credit Default Swaps."""

    @property
    def calendar_id(self) -> str:
        """Calendar identifier."""
        ...

    @property
    def day_count(self) -> DayCount:
        """Day count convention."""
        ...

    @property
    def business_day_convention(self) -> BusinessDayConvention:
        """Business day convention."""
        ...

    @property
    def settlement_days(self) -> int:
        """Settlement days."""
        ...

    @property
    def payment_frequency(self) -> Tenor:
        """Payment frequency."""
        ...

class SwaptionConventions:
    """Conventions for Swaptions."""

    @property
    def calendar_id(self) -> str:
        """Calendar identifier."""
        ...

    @property
    def settlement_days(self) -> int:
        """Settlement days."""
        ...

    @property
    def business_day_convention(self) -> BusinessDayConvention:
        """Business day convention."""
        ...

    @property
    def fixed_leg_frequency(self) -> Tenor:
        """Fixed leg payment frequency."""
        ...

    @property
    def fixed_leg_day_count(self) -> DayCount:
        """Fixed leg day count."""
        ...

    @property
    def float_leg_index(self) -> str:
        """Float leg rate index ID."""
        ...

class InflationSwapConventions:
    """Conventions for Inflation Swaps."""

    @property
    def calendar_id(self) -> str:
        """Calendar identifier."""
        ...

    @property
    def settlement_days(self) -> int:
        """Settlement days."""
        ...

    @property
    def business_day_convention(self) -> BusinessDayConvention:
        """Business day convention."""
        ...

    @property
    def day_count(self) -> DayCount:
        """Day count convention."""
        ...

    @property
    def inflation_lag(self) -> Tenor:
        """Inflation lag (observation delay)."""
        ...

class OptionConventions:
    """Conventions for Options (Equity/FX/Commodity)."""

    @property
    def calendar_id(self) -> str:
        """Calendar identifier."""
        ...

    @property
    def settlement_days(self) -> int:
        """Settlement days."""
        ...

    @property
    def business_day_convention(self) -> BusinessDayConvention:
        """Business day convention."""
        ...

class IrFutureConventions:
    """Conventions for Interest Rate Futures."""

    @property
    def index_id(self) -> str:
        """Rate index identifier."""
        ...

    @property
    def calendar_id(self) -> str:
        """Calendar identifier."""
        ...

    @property
    def settlement_days(self) -> int:
        """Settlement days."""
        ...

    @property
    def delivery_months(self) -> int:
        """Delivery months bitmap (Mar=3, Jun=6, Sep=9, Dec=12)."""
        ...

    @property
    def face_value(self) -> float:
        """Face value of the contract."""
        ...

    @property
    def tick_size(self) -> float:
        """Minimum price movement."""
        ...

    @property
    def tick_value(self) -> float:
        """Dollar value of one tick."""
        ...

    @property
    def convexity_adjustment(self) -> Optional[float]:
        """Default convexity adjustment (if any)."""
        ...

class ConventionRegistry:
    """Global registry of market conventions.

    Provides lookup methods for rate index, CDS, swaption, inflation swap,
    option, and IR future conventions.

    Examples:
        >>> registry = ConventionRegistry.global_instance()
        >>> sofr = registry.require_rate_index("USD-SOFR-OIS")
        >>> print(sofr.day_count)
    """

    @staticmethod
    def global_instance() -> ConventionRegistry:
        """Get the global convention registry instance.

        Returns:
            ConventionRegistry: The global singleton registry.
        """
        ...

    def require_rate_index(self, index_id: str) -> RateIndexConventions:
        """Look up conventions for a rate index.

        Args:
            index_id: Rate index identifier (e.g., "USD-SOFR-OIS").

        Returns:
            RateIndexConventions: Conventions for the rate index.

        Raises:
            ValueError: If the index is not found.
        """
        ...

    def require_cds(self, key: CdsConventionKey) -> CdsConventions:
        """Look up conventions for a CDS.

        Args:
            key: CDS convention key (currency + doc clause).

        Returns:
            CdsConventions: Conventions for the CDS.

        Raises:
            ValueError: If the conventions are not found.
        """
        ...

    def require_swaption(self, convention_id: str) -> SwaptionConventions:
        """Look up conventions for a swaption.

        Args:
            convention_id: Swaption convention identifier (e.g., "USD").

        Returns:
            SwaptionConventions: Conventions for the swaption.

        Raises:
            ValueError: If the conventions are not found.
        """
        ...

    def require_inflation_swap(self, convention_id: str) -> InflationSwapConventions:
        """Look up conventions for an inflation swap.

        Args:
            convention_id: Inflation swap convention identifier (e.g., "USD-CPI").

        Returns:
            InflationSwapConventions: Conventions for the inflation swap.

        Raises:
            ValueError: If the conventions are not found.
        """
        ...

    def require_option(self, convention_id: str) -> OptionConventions:
        """Look up conventions for an option.

        Args:
            convention_id: Option convention identifier (e.g., "USD-EQUITY").

        Returns:
            OptionConventions: Conventions for the option.

        Raises:
            ValueError: If the conventions are not found.
        """
        ...

    def require_ir_future(self, contract_id: str) -> IrFutureConventions:
        """Look up conventions for an IR future contract.

        Args:
            contract_id: IR future contract identifier (e.g., "CME:SR3").

        Returns:
            IrFutureConventions: Conventions for the IR future.

        Raises:
            ValueError: If the conventions are not found.
        """
        ...

__all__ = [
    "CdsDocClause",
    "RateIndexKind",
    "CdsConventionKey",
    "RateIndexConventions",
    "CdsConventions",
    "SwaptionConventions",
    "InflationSwapConventions",
    "OptionConventions",
    "IrFutureConventions",
    "ConventionRegistry",
]
