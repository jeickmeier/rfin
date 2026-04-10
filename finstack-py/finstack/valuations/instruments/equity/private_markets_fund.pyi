"""Private markets fund instrument."""

from __future__ import annotations
from typing import Dict, Any
from datetime import date
from ....core.currency import Currency
from ....core.market_data.context import MarketContext
from ...cashflow.builder import CashFlowSchedule
from ...common import InstrumentType

class PrivateMarketsFund:
    """Private markets fund for private equity and venture capital modeling.

    PrivateMarketsFund represents a private investment fund (private equity,
    venture capital, etc.) with capital calls, distributions, and NAV tracking.
    Funds are used for modeling LP (limited partner) cashflows and returns.

    Private markets funds have complex cashflow patterns including capital
    commitments, capital calls, distributions, and management fees. They are
    typically defined via JSON specifications.

    Examples
    --------
    Create a private markets fund from JSON:

        >>> from finstack.valuations.instruments import PrivateMarketsFund
        >>> import json
        >>> # ... more code: see the integration examples for a fully validated tagged payload
        >>> # PrivateMarketsFund requires complex JSON with spec and events
        >>> # Minimal example (may need additional fields):
        >>> json_data = {
        ...     "id": "PE-FUND-001",
        ...     "currency": "USD",
        ...     "spec": {
        ...         "style": "european",
        ...         "tranches": ["return_of_capital"],
        ...         "clawback": None,
        ...         "irr_basis": "Act365F",
        ...         "catchup_mode": "full",
        ...     },
        ...     "events": [
        ...         {
        ...             "date": "2024-01-15",
        ...             "amount": {"amount": "5000000", "currency": "USD"},
        ...             "kind": "contribution",
        ...             "deal_id": None,
        ...         },
        ...         {
        ...             "date": "2026-03-01",
        ...             "amount": {"amount": "4000000", "currency": "USD"},
        ...             "kind": "proceeds",
        ...             "deal_id": None,
        ...         },
        ...     ],
        ...     "attributes": {"tags": [], "meta": {}},
        ... }
        >>> tagged_payload = {"type": "private_markets_fund", "spec": json_data}
        >>> fund = PrivateMarketsFund.from_json(json.dumps(tagged_payload))
        >>> schedule = fund.cashflow_schedule(market, date(2024, 1, 2))

    Notes
    -----
    - Private markets funds are defined via JSON
    - Include capital commitments, calls, and distributions
    - The canonical cashflow schedule represents net cashflows to limited partners
    - Can include management fees and carried interest
    - NAV (net asset value) tracking for fund valuation

    MarketContext Requirements
    -------------------------
    - Parsing and waterfall construction do not require market data by themselves.
    - The canonical `cashflow_schedule()` method follows the standard instrument interface and therefore accepts a market context and valuation date.

    See Also
    --------
    :class:`Bond`: Bonds
    :class:`TermLoan`: Term loans
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Damodaran (valuation / DCF context): see ``docs/REFERENCES.md#damodaranInvestmentValuation``.
    """

    @classmethod
    def from_json(cls, data: str | Dict[str, Any]) -> "PrivateMarketsFund":
        """Create a private markets fund from JSON string or dictionary.

        Parameters
        ----------
        data : str or Dict[str, Any]
            JSON string or dictionary containing fund specification. Must include
            instrument_id, currency, and cashflow schedules (commitments, calls,
            distributions).

        Returns
        -------
        PrivateMarketsFund
            Configured private markets fund ready for analysis.

        Raises
        ------
        ValueError
            If JSON is invalid or required fields are missing.

        Examples
        --------
            >>> fund = PrivateMarketsFund.from_json(json_data)
            >>> schedule = fund.cashflow_schedule(market, date(2024, 1, 2))
            >>> for flow in schedule.flows():
            ...     print(flow)
        """
        ...

    def to_json(self) -> str:
        """Serialize the fund to a JSON string."""
        ...

    def cashflow_schedule(self, market: MarketContext, as_of: date) -> CashFlowSchedule:
        """Return the canonical LP cashflow schedule."""
        ...

    def run_waterfall(self) -> str:
        """Run the waterfall allocation and return the result as JSON."""
        ...

    def run_waterfall_tabular(self) -> tuple[list[str], list[list[str]]]:
        """Run the waterfall allocation and return headers and rows for DataFrame construction."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def discount_curve(self) -> str | None: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
