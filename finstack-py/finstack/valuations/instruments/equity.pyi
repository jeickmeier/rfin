"""Equity instrument."""

from typing import Optional
from ...core.currency import Currency
from ..common import InstrumentType

class Equity:
    """Spot equity position for equity valuation and portfolio modeling.

    Equity represents a long or short position in a single equity or equity
    index. It can be used for portfolio valuation, risk calculations, and
    as an underlying for equity derivatives.

    Equity positions are valued using spot prices from MarketContext and can
    include dividend yield for total return calculations.

    Examples
    --------
    Create an equity position:

        >>> from finstack.valuations.instruments import Equity
        >>> from finstack import Currency
        >>> equity = Equity.create(
        ...     "EQUITY-AAPL",
        ...     ticker="AAPL",
        ...     currency=Currency("USD"),
        ...     shares=100.0,
        ...     price_id="AAPL",  # Spot price ID in MarketContext
        ... )

    Notes
    -----
    - Equity requires spot price in MarketContext (via price_id or MarketScalar)
    - Shares can be positive (long) or negative (short)
    - Dividend yield can be specified for total return calculations
    - Price can be provided directly or retrieved from MarketContext

    See Also
    --------
    :class:`EquityOption`: Equity options
    :class:`EquityTotalReturnSwap`: Equity TRS
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def create(
        cls,
        instrument_id: str,
        ticker: str,
        currency: Currency,
        *,
        shares: Optional[float] = None,
        price: Optional[float] = None,
        price_id: Optional[str] = None,
        div_yield_id: Optional[str] = None,
    ) -> "Equity": ...
    """Create an equity instrument optionally specifying share count and price.

    Parameters
    ----------
    instrument_id : str
        Unique identifier for the equity position (e.g., "EQUITY-AAPL").
    ticker : str
        Equity ticker symbol (e.g., "AAPL", "MSFT", "SPX").
    currency : Currency
        Currency of the equity (e.g., Currency("USD")).
    shares : float, optional
        Number of shares (can be positive for long, negative for short).
        If None, uses a unit position.
    price : float, optional
        Spot price override. If None, price is retrieved from MarketContext
        using price_id.
    price_id : str, optional
        Spot price identifier in MarketContext. If None, uses ticker as price_id.
    div_yield_id : str, optional
        Dividend yield identifier in MarketContext for total return calculations.

    Returns
    -------
    Equity
        Configured equity position ready for valuation.

    Examples
    --------
        >>> equity = Equity.create(
        ...     "EQUITY-AAPL",
        ...     ticker="AAPL",
        ...     currency=Currency("USD"),
        ...     shares=100.0,
        ...     price_id="AAPL"
        ... )
    """

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def shares(self) -> float: ...
    @property
    def price_quote(self) -> Optional[float]: ...
    @property
    def price_id(self) -> Optional[str]: ...
    @property
    def div_yield_id(self) -> Optional[str]: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
