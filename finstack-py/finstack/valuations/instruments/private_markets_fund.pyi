"""Private markets fund instrument."""

from typing import Optional, Dict, Any, List, Tuple, Union
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

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
        ... }
        >>> fund = PrivateMarketsFund.from_json(json.dumps(json_data))
        >>> cashflows = fund.lp_cashflows()

    Notes
    -----
    - Private markets funds are defined via JSON
    - Include capital commitments, calls, and distributions
    - LP cashflows represent net cashflows to limited partners
    - Can include management fees and carried interest
    - NAV (net asset value) tracking for fund valuation

    See Also
    --------
    :class:`Bond`: Bonds
    :class:`TermLoan`: Term loans
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def from_json(cls, data: Union[str, Dict[str, Any]]) -> "PrivateMarketsFund": ...
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
        >>> cashflows = fund.lp_cashflows()
        >>> for date, amount in cashflows:
        ...     print(f"{date}: {amount}")
    """

    def to_json(self) -> str:
        """Serialize the fund to a JSON string."""
        ...

    def lp_cashflows(self) -> List[Tuple[date, Money]]:
        """Calculate LP cashflows."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def discount_curve(self) -> Optional[str]: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
