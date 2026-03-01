"""Basket instrument and pricing calculator."""

from __future__ import annotations
from typing import Dict, Any, List
from datetime import date
from ....core.money import Money
from ....core.currency import Currency
from ....core.market_data.fx import FxConversionPolicy
from ....core.market_data.context import MarketContext
from ...common import InstrumentType

class AssetType:
    """Asset type classification for basket constituents."""

    EQUITY: "AssetType"
    BOND: "AssetType"
    ETF: "AssetType"
    CASH: "AssetType"
    COMMODITY: "AssetType"
    DERIVATIVE: "AssetType"

    @classmethod
    def from_name(cls, name: str) -> "AssetType": ...
    @property
    def name(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class BasketPricingConfig:
    """Pricing configuration for basket instruments.

    Parameters
    ----------
    days_in_year : float, optional
        Day basis for fee accrual (default: 365.25).
    fx_policy : str or FxConversionPolicy, optional
        FX conversion policy (default: "cashflow_date").
    """

    def __init__(
        self,
        days_in_year: float = 365.25,
        fx_policy: str | FxConversionPolicy | None = "cashflow_date",
    ) -> None: ...
    @property
    def days_in_year(self) -> float: ...
    @property
    def fx_policy(self) -> FxConversionPolicy: ...
    def __repr__(self) -> str: ...

class BasketConstituent:
    """Read-only view of a basket constituent."""

    @property
    def id(self) -> str: ...
    @property
    def weight(self) -> float: ...
    @property
    def units(self) -> float | None: ...
    @property
    def ticker(self) -> str | None: ...
    def __repr__(self) -> str: ...

class BasketCalculator:
    """Basket calculation engine for NAV and value computations.

    Parameters
    ----------
    config : BasketPricingConfig, optional
        Pricing configuration. Uses defaults if omitted.

    Examples
    --------
        >>> calc = BasketCalculator()
        >>> nav = calc.nav(basket, market_context, as_of_date, shares_outstanding=1_000_000)
    """

    def __init__(self, config: BasketPricingConfig | None = None) -> None: ...
    def nav(
        self,
        basket: Basket,
        market_context: MarketContext,
        as_of: date,
        shares_outstanding: float,
    ) -> Money:
        """Calculate Net Asset Value per share.

        Parameters
        ----------
        basket : Basket
            The basket instrument to value.
        market_context : MarketContext
            Market data context with pricing data.
        as_of : date
            Valuation date.
        shares_outstanding : float
            Total shares outstanding.

        Returns
        -------
        Money
            NAV per share.
        """
        ...
    def basket_value(
        self,
        basket: Basket,
        market_context: MarketContext,
        as_of: date,
        shares_outstanding: float | None = None,
    ) -> Money:
        """Calculate total basket value.

        Parameters
        ----------
        basket : Basket
            The basket instrument to value.
        market_context : MarketContext
            Market data context with pricing data.
        as_of : date
            Valuation date.
        shares_outstanding : float, optional
            Total shares outstanding for weight-based calculations.

        Returns
        -------
        Money
            Total basket value.
        """
        ...
    def nav_with_aum(
        self,
        basket: Basket,
        market_context: MarketContext,
        as_of: date,
        aum: Money,
        shares_outstanding: float,
    ) -> Money:
        """Calculate NAV per share using explicit AUM.

        Parameters
        ----------
        basket : Basket
            The basket instrument to value.
        market_context : MarketContext
            Market data context with pricing data.
        as_of : date
            Valuation date.
        aum : Money
            Assets under management.
        shares_outstanding : float
            Total shares outstanding.

        Returns
        -------
        Money
            NAV per share computed from AUM.
        """
        ...
    def basket_value_with_aum(
        self,
        basket: Basket,
        market_context: MarketContext,
        as_of: date,
        aum: Money,
    ) -> Money:
        """Calculate total basket value using explicit AUM.

        Parameters
        ----------
        basket : Basket
            The basket instrument to value.
        market_context : MarketContext
            Market data context with pricing data.
        as_of : date
            Valuation date.
        aum : Money
            Assets under management in basket currency.

        Returns
        -------
        Money
            Total basket value computed from AUM.
        """
        ...
    def __repr__(self) -> str: ...

class Basket:
    """Basket instrument wrapper parsed from JSON definitions.

    Examples
    --------
        >>> basket = Basket.from_json(json.dumps({...}))
        >>> basket.instrument_type.name
        'basket'
    """

    @classmethod
    def from_json(cls, data: str | Dict[str, Any]) -> "Basket":
        """Parse a basket definition from a JSON string or dictionary.

        Parameters
        ----------
        data : str or dict
            JSON string or dictionary describing the basket.

        Returns
        -------
        Basket
            Parsed basket instrument.

        Raises
        ------
        ValueError
            If parsing fails or the basket ID is missing.
        TypeError
            If data is neither a string nor dict-like object.
        """
        ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def notional(self) -> Money: ...
    @property
    def expense_ratio(self) -> float: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def constituent_count(self) -> int: ...
    @property
    def constituents(self) -> List[BasketConstituent]: ...
    @property
    def pricing_config(self) -> BasketPricingConfig: ...
    def calculator(self) -> BasketCalculator:
        """Get a configured calculator for this basket.

        Returns
        -------
        BasketCalculator
            Calculator using this basket's pricing configuration.
        """
        ...
    def validate(self) -> None:
        """Validate basket consistency.

        Raises
        ------
        ValueError
            If weights do not sum to approximately 1.0 or currency is inconsistent.
        """
        ...
    def to_json(self) -> str:
        """Serialize the basket definition to a JSON string.

        Returns
        -------
        str
            Pretty-printed JSON representation.
        """
        ...
    def __repr__(self) -> str: ...
