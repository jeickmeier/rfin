"""Instrument pricing: bonds, swaps, options, calibration, attribution."""

from __future__ import annotations

__all__ = [
    "ValuationResult",
    "validate_instrument_json",
    "price_instrument",
    "price_instrument_with_metrics",
    "list_standard_metrics",
]

class ValuationResult:
    """Valuation envelope: PV, currency, risk metrics, covenant flags, and JSON round-trip.

    Instantiate via :meth:`from_json` or the ``price_*`` helpers that emit JSON.

    Args:
        None (use ``from_json``).

    Returns:
        A ``ValuationResult`` instance (type description only).

    Example:
        >>> from finstack.valuations import ValuationResult
        >>> ValuationResult.from_json(result_json)  # doctest: +SKIP
    """

    @staticmethod
    def from_json(json: str) -> ValuationResult:
        """Deserialize a ``ValuationResult`` from JSON.

        Args:
            json: JSON string produced by the pricing pipeline or ``to_json``.

        Returns:
            Parsed ``ValuationResult`` instance.

        Example:
            >>> from finstack.valuations import ValuationResult
            >>> ValuationResult.from_json('{"instrument_id":"x","value":{}}')  # doctest: +SKIP
        """
        ...

    def to_json(self) -> str:
        """Serialize this result to pretty-printed JSON.

        Args:
            (none)

        Returns:
            Pretty-printed JSON string.

        Example:
            >>> ValuationResult.from_json(
            ...     '{"instrument_id":"i","value":{"amount":1.0,"currency":"USD"},"measures":{}}'
            ... ).to_json()  # doctest: +SKIP
            ''
        """
        ...

    @property
    def instrument_id(self) -> str:
        """Instrument identifier assigned by the pricer.

        Args:
            None (read-only property).

        Returns:
            Instrument ID string.

        Example:
            >>> vr = ValuationResult.from_json("{}")  # doctest: +SKIP
            >>> vr.instrument_id  # doctest: +SKIP
            ''
        """
        ...

    @property
    def get_price(self) -> float:
        """Present value amount (Rust exposes this as the ``get_price`` getter).

        Args:
            None (read-only property).

        Returns:
            NPV amount as a float.

        Example:
            >>> vr = ValuationResult.from_json("{}")  # doctest: +SKIP
            >>> vr.get_price  # doctest: +SKIP
            0.0
        """
        ...

    @property
    def currency(self) -> str:
        """Currency code for the present value.

        Args:
            None (read-only property).

        Returns:
            Currency code string.

        Example:
            >>> vr = ValuationResult.from_json("{}")  # doctest: +SKIP
            >>> vr.currency  # doctest: +SKIP
            'USD'
        """
        ...

    def get_metric(self, key: str) -> float | None:
        """Return a scalar risk measure by string key.

        Args:
            key: Metric identifier (e.g. ``"ytm"``, ``"dv01"``).

        Returns:
            Metric value, or ``None`` if missing.

        Example:
            >>> vr = ValuationResult.from_json(
            ...     '{"instrument_id":"i","value":{"amount":1,"currency":"USD"},"measures":{}}'
            ... )  # doctest: +SKIP
            >>> vr.get_metric("ytm")  # doctest: +SKIP
        """
        ...

    def metric_keys(self) -> list[str]:
        """List metric keys present on this result.

        Args:
            (none)

        Returns:
            All measure keys as strings.

        Example:
            >>> ValuationResult.from_json(
            ...     '{"instrument_id":"i","value":{"amount":1,"currency":"USD"},"measures":{}}'
            ... ).metric_keys()  # doctest: +SKIP
            []
        """
        ...

    def metric_count(self) -> int:
        """Count of measures stored on this result.

        Args:
            (none)

        Returns:
            Number of entries in the measures map.

        Example:
            >>> ValuationResult.from_json(
            ...     '{"instrument_id":"i","value":{"amount":1,"currency":"USD"},"measures":{}}'
            ... ).metric_count()  # doctest: +SKIP
            0
        """
        ...

    def all_covenants_passed(self) -> bool:
        """Whether every covenant passed (or none were evaluated).

        Args:
            (none)

        Returns:
            ``True`` if no covenant failures are recorded.

        Example:
            >>> ValuationResult.from_json(
            ...     '{"instrument_id":"i","value":{"amount":1,"currency":"USD"},"measures":{}}'
            ... ).all_covenants_passed()  # doctest: +SKIP
            True
        """
        ...

    def failed_covenants(self) -> list[str]:
        """Covenant IDs that failed, if any.

        Args:
            (none)

        Returns:
            List of failed covenant identifiers.

        Example:
            >>> ValuationResult.from_json(
            ...     '{"instrument_id":"i","value":{"amount":1,"currency":"USD"},"measures":{}}'
            ... ).failed_covenants()  # doctest: +SKIP
            []
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug string for this result.

        Args:
            None (uses ``self``).

        Returns:
            ``ValuationResult(id=..., price=..., currency=..., metrics=...)`` text.

        Example:
            >>> repr(ValuationResult.from_json("{}"))  # doctest: +SKIP
            ''
        """
        ...

def validate_instrument_json(json: str) -> str:
    """Parse tagged instrument JSON and return canonical pretty JSON.

    Args:
        json: Tagged instrument JSON (e.g. ``{"type": "bond", ...}``).

    Returns:
        Canonical pretty-printed JSON accepted by the instrument loader.

    Example:
        >>> from finstack.valuations import validate_instrument_json
        >>> validate_instrument_json(inst_json)  # doctest: +SKIP
        ''
    """
    ...

def price_instrument(
    instrument_json: str,
    market_json: str,
    as_of: str,
    model: str = "discounting",
) -> str:
    """Price an instrument using the standard registry and a model key.

    Args:
        instrument_json: Tagged instrument JSON.
        market_json: JSON-serialized ``MarketContext``.
        as_of: Valuation date in ISO 8601 format.
        model: Model key: ``discounting`` (default), ``black76``, ``hazard_rate``,
            ``hull_white_1f``, ``tree``, ``normal``, ``monte_carlo_gbm``, etc.

    Returns:
        Pretty-printed JSON ``ValuationResult``.

    Example:
        >>> from finstack.valuations import price_instrument
        >>> price_instrument(inst_json, mkt_json, "2025-01-15")  # doctest: +SKIP
        ''
    """
    ...

def price_instrument_with_metrics(
    instrument_json: str,
    market_json: str,
    as_of: str,
    model: str = "discounting",
    metrics: list[str] = [],
) -> str:
    """Price an instrument and request explicit risk metrics.

    Args:
        instrument_json: Tagged instrument JSON.
        market_json: JSON-serialized ``MarketContext``.
        as_of: Valuation date in ISO 8601 format.
        model: Model key string (same vocabulary as ``price_instrument``).
        metrics: Metric names to compute (default empty list).

    Returns:
        Pretty-printed JSON ``ValuationResult`` including requested metrics.

    Example:
        >>> from finstack.valuations import price_instrument_with_metrics
        >>> price_instrument_with_metrics(inst_json, mkt_json, "2025-01-15", metrics=["dv01"])  # doctest: +SKIP
        ''
    """
    ...

def list_standard_metrics() -> list[str]:
    """Return every metric ID exposed by the standard metric registry.

    Args:
        (none)

    Returns:
        Sorted metric identifier strings.

    Example:
        >>> from finstack.valuations import list_standard_metrics
        >>> isinstance(list_standard_metrics(), list)
        True
    """
    ...
