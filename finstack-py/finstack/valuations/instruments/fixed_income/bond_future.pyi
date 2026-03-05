"""Bond future contract instrument."""

from __future__ import annotations

from datetime import date
from typing import TYPE_CHECKING, Any

from ....core.money import Money
from ...common import InstrumentType

if TYPE_CHECKING:
    from .bond import Bond
    from ....core.market_data.context import MarketContext

class BondFutureSpecs:
    """Bond future contract specifications.

    Defines standard parameters for a bond future contract including contract size,
    tick size, and notional bond parameters for conversion factor calculations.
    """

    def __init__(
        self,
        contract_size: float,
        tick_size: float,
        tick_value: float,
        standard_coupon: float,
        standard_maturity_years: float,
        settlement_days: int = 2,
        calendar_id: str = "nyse",
    ) -> None: ...
    @property
    def contract_size(self) -> float:
        """Contract size (face value per contract)."""
        ...
    @property
    def tick_size(self) -> float:
        """Tick size (minimum price increment)."""
        ...
    @property
    def tick_value(self) -> float:
        """Tick value in currency units."""
        ...
    @property
    def standard_coupon(self) -> float:
        """Standard coupon rate for conversion factor calculation."""
        ...
    @property
    def standard_maturity_years(self) -> float:
        """Standard maturity in years."""
        ...
    @property
    def settlement_days(self) -> int:
        """Settlement days after expiry."""
        ...
    @property
    def calendar_id(self) -> str:
        """Holiday calendar identifier."""
        ...
    def __repr__(self) -> str: ...

class DeliverableBond:
    """A deliverable bond in a futures contract basket."""
    def __init__(self, bond_id: str, conversion_factor: float) -> None: ...
    @property
    def bond_id(self) -> str: ...
    @property
    def conversion_factor(self) -> float: ...
    def __repr__(self) -> str: ...

class BondFutureBuilder:
    """Fluent builder returned by :meth:`BondFuture.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, amount: float, currency: str | Any) -> BondFutureBuilder:
        """Set notional amount and currency."""
        ...
    def expiry_date(self, date: date) -> BondFutureBuilder:
        """Set expiry date (last trading day)."""
        ...
    def delivery_start(self, date: date) -> BondFutureBuilder:
        """Set first delivery date."""
        ...
    def delivery_end(self, date: date) -> BondFutureBuilder:
        """Set last delivery date."""
        ...
    def quoted_price(self, price: float) -> BondFutureBuilder:
        """Set quoted futures price."""
        ...
    def position(self, position: str) -> BondFutureBuilder:
        """Set position side ('long' or 'short')."""
        ...
    def contract_specs(self, specs: BondFutureSpecs) -> BondFutureBuilder:
        """Set contract specifications."""
        ...
    def deliverable_basket(self, basket: list[DeliverableBond | dict[str, Any]]) -> BondFutureBuilder:
        """Set deliverable basket of bonds with conversion factors."""
        ...
    def ctd_bond_id(self, bond_id: str) -> BondFutureBuilder:
        """Set Cheapest-to-Deliver (CTD) bond identifier."""
        ...
    def disc_id(self, curve_id: str) -> BondFutureBuilder:
        """Set discount curve identifier."""
        ...
    def repo_curve_id(self, curve_id: str) -> BondFutureBuilder:
        """Set repo/financing curve identifier for implied repo and carry calculations."""
        ...
    def build(self) -> BondFuture:
        """Build the BondFuture instrument."""
        ...

class BondFuture:
    """Bond future contract instrument.

    A standardized contract to buy or sell a government bond at a specified price
    on a future date. The contract has a basket of deliverable bonds, each with a
    conversion factor. The short position holder chooses which bond to deliver
    (typically the Cheapest-to-Deliver or CTD bond).

    Examples
    --------
    Create a UST 10-year future:

        >>> from finstack.valuations.instruments import BondFuture
        >>> future = (
        ...     BondFuture
        ...     .builder("TYH5")
        ...     .notional(1_000_000, "USD")
        ...     .expiry_date(date(2025, 3, 20))
        ...     .delivery_start(date(2025, 3, 21))
        ...     .delivery_end(date(2025, 3, 31))
        ...     .quoted_price(125.50)
        ...     .position("long")
        ...     .contract_specs(BondFuture.ust_10y_specs())
        ...     .deliverable_basket([
        ...         {"bond_id": "US912828XG33", "conversion_factor": 0.8234},
        ...     ])
        ...     .ctd_bond_id("US912828XG33")
        ...     .disc_id("USD-TREASURY")
        ...     .build()
        ... )

    See Also
    --------
    :class:`Bond`: Plain vanilla fixed income bond
    :class:`InterestRateFuture`: Short-term interest rate futures
    """

    @classmethod
    def builder(cls, instrument_id: str) -> BondFutureBuilder:
        """Create a builder for constructing BondFuture instruments."""
        ...
    @classmethod
    def ust_10y_specs(cls) -> BondFutureSpecs:
        """UST 10-year futures contract specifications."""
        ...
    @classmethod
    def ust_5y_specs(cls) -> BondFutureSpecs:
        """UST 5-year futures contract specifications."""
        ...
    @classmethod
    def ust_2y_specs(cls) -> BondFutureSpecs:
        """UST 2-year futures contract specifications."""
        ...
    @classmethod
    def bund_specs(cls) -> BondFutureSpecs:
        """German Bund futures contract specifications (Eurex)."""
        ...
    @classmethod
    def gilt_specs(cls) -> BondFutureSpecs:
        """UK Gilt futures contract specifications (LIFFE)."""
        ...
    @property
    def id(self) -> str:
        """Instrument identifier."""
        ...
    @property
    def notional(self) -> Money:
        """Notional exposure."""
        ...
    @property
    def expiry_date(self) -> date:
        """Future expiry date (last trading day)."""
        ...
    @property
    def delivery_start(self) -> date:
        """First delivery date."""
        ...
    @property
    def delivery_end(self) -> date:
        """Last delivery date."""
        ...
    @property
    def quoted_price(self) -> float:
        """Quoted futures price."""
        ...
    @property
    def position(self) -> str:
        """Position side ('long' or 'short')."""
        ...
    @property
    def discount_curve(self) -> str:
        """Discount curve identifier."""
        ...
    @property
    def instrument_type(self) -> InstrumentType:
        """Instrument type enum."""
        ...
    def invoice_price(
        self,
        ctd_bond: Bond,
        market: MarketContext,
        settlement_date: date,
    ) -> Money:
        """Compute the invoice price for delivering the CTD bond."""
        ...
    def determine_ctd(
        self,
        bond_clean_prices: list[tuple[str, float]],
    ) -> tuple[str, float]:
        """Determine the cheapest-to-deliver bond from clean prices."""
        ...
    def determine_ctd_with_accrued(
        self,
        bond_prices_with_accrued: list[tuple[str, float, float, float]],
    ) -> tuple[str, float]:
        """Determine the cheapest-to-deliver bond using gross basis with delivery accrued."""
        ...
    def implied_repo_rate(
        self,
        bond_id: str,
        clean_price: float,
        accrued_today: float,
        accrued_at_delivery: float,
        coupon_income: float,
        days_to_delivery: int,
    ) -> float:
        """Calculate the annualized implied repo rate for a deliverable bond."""
        ...
    def __repr__(self) -> str: ...
