"""Basket instrument."""

from typing import Union, Dict, Any
from ..common import InstrumentType

class Basket:
    """Basket instrument for multi-asset exposure.

    Basket represents a portfolio of underlying assets (equities, indices,
    etc.) with a single instrument wrapper. Baskets are used for correlation
    trading, index replication, and structured products.

    Basket instruments are typically defined via JSON and can include various
    payoff structures (best-of, worst-of, average, etc.).

    Examples
    --------
    Create a basket from JSON:

        >>> from finstack.valuations.instruments import Basket
        >>> import json
        >>> json_str = json.dumps({
        ...     "id": "BASKET-TECH",
        ...     "constituents": [
        ...         {
        ...             "id": "EQ-AAPL",
        ...             "reference": {"asset_type": "equity", "price_id": "AAPL-SPOT"},
        ...             "ticker": "AAPL",
        ...             "weight": 0.4,
        ...             "units": None,
        ...         },
        ...         {
        ...             "id": "EQ-MSFT",
        ...             "reference": {"asset_type": "equity", "price_id": "MSFT-SPOT"},
        ...             "ticker": "MSFT",
        ...             "weight": 0.3,
        ...             "units": None,
        ...         },
        ...         {
        ...             "id": "EQ-GOOGL",
        ...             "reference": {"asset_type": "equity", "price_id": "GOOGL-SPOT"},
        ...             "ticker": "GOOGL",
        ...             "weight": 0.3,
        ...             "units": None,
        ...         },
        ...     ],
        ...     "currency": "USD",
        ...     "expense_ratio": 0.0025,
        ...     "discount_curve_id": "USD-OIS",
        ...     "attributes": {"meta": {}, "tags": []},
        ...     "pricing_config": {"days_in_year": 365.25, "fx_policy": "cashflow_date"},
        ... })
        >>> basket = Basket.from_json(json_str)

    Notes
    -----
    - Basket instruments are defined via JSON
    - Can include multiple underlyings with weights
    - Payoff types: "average", "best_of", "worst_of", "spread", etc.
    - Requires spot prices for all underlyings
    - Correlation between underlyings affects pricing

    MarketContext Requirements
    -------------------------
    - Underlying spot prices: referenced by IDs in the JSON payload (required).
    - Discount curve: referenced by ``discount_curve_id`` in the JSON payload when provided/used by the pricer.

    See Also
    --------
    :class:`Equity`: Single equity positions
    :class:`EquityOption`: Equity options
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def from_json(cls, data: Union[str, Dict[str, Any]]) -> "Basket":
        """Parse a basket definition from a JSON string or dictionary.

        Parameters
        ----------
        data : str or Dict[str, Any]
            JSON string or dictionary containing basket definition.
            Must include underlyings, weights, and payoff_type.

        Returns
        -------
        Basket
            Configured basket instrument ready for pricing.

        Raises
        ------
        ValueError
            If JSON is invalid or required fields are missing.

        Examples
        --------
            >>> import json
            >>> json_data = {
            ...     "id": "BASKET-2",
            ...     "constituents": [{"ticker": "AAPL", "weight": 0.5}, {"ticker": "MSFT", "weight": 0.5}],
            ...     "currency": "USD",
            ... }
            >>> json_str = json.dumps(json_data)
            >>> basket = Basket.from_json(json_str)
        """
        ...

    def to_json(self) -> str:
        """Serialize the basket definition to a JSON string."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
