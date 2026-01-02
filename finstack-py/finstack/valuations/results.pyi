"""Valuation result envelopes, metadata, and covenant report bindings."""

from typing import Dict, Optional, Any, List
from datetime import date
from ..core.money import Money

class CovenantReport:
    """Covenant evaluation outcome attached to a valuation result.

    CovenantReport represents the result of a covenant check performed during
    valuation. Covenants are conditions that must be satisfied (e.g., LTV ratios,
    debt service coverage, leverage limits) and are commonly used in private
    credit and real-estate lending.

    Example
    -------
    When an attribution run includes covenant results you can inspect them
    through :pyattr:`ValuationResult.covenants`::

        report = result.covenants["ltv"]
        status = "passed" if report.passed else "failed"
        details = f"{report.actual_value:.2%} vs {report.threshold:.2%}"

    Notes
    -----
    - Covenants are optional and only present if evaluated
    - Common covenant types: "ltv", "dscr", "leverage", "interest_coverage"
    - Passed=True means the covenant condition is satisfied
    - Actual value and threshold are provided for transparency

    See Also
    --------
    :class:`ValuationResult`: Result envelope containing covenants
    :meth:`ValuationResult.all_covenants_passed`: Check all covenants
    :meth:`ValuationResult.failed_covenants`: Get failed covenant list
    """

    @property
    def covenant_type(self) -> str:
        """Covenant identifier describing the check performed.

        Returns
        -------
        str
            Covenant label (e.g., "ltv", "dscr", "leverage") supplied by
            the originating configuration.

        Examples
        --------
            >>> report = result.covenants["ltv"]
            >>> print(report.covenant_type)
            'ltv'
        """
        ...

    @property
    def passed(self) -> bool:
        """Whether the covenant passed for the evaluated scenario.

        Returns
        -------
        bool
            True when the covenant conditions are satisfied, False otherwise.

        Examples
        --------
            >>> report = result.covenants["ltv"]
            >>> if report.passed:
            ...     print("LTV covenant satisfied")
            ... else:
            ...     print(f"LTV covenant failed: {report.actual_value} > {report.threshold}")
        """
        ...

    @property
    def actual_value(self) -> Optional[float]:
        """Observed metric value when available.

        Returns
        -------
        float or None
            Realized metric value used in the covenant check. Units depend
            on covenant type (e.g., percentage for LTV, ratio for DSCR).

        Examples
        --------
            >>> report = result.covenants["ltv"]
            >>> if report.actual_value:
            ...     print(f"Actual LTV: {report.actual_value:.2%}")
            Actual LTV: 65.00%
        """
        ...

    @property
    def threshold(self) -> Optional[float]:
        """Required threshold for the covenant, when provided.

        Returns
        -------
        float or None
            Target threshold or limit for the covenant. The covenant passes
            if actual_value meets the threshold condition (e.g., <= threshold
            for LTV, >= threshold for DSCR).

        Examples
        --------
            >>> report = result.covenants["ltv"]
            >>> if report.threshold:
            ...     print(f"LTV limit: {report.threshold:.2%}")
            LTV limit: 75.00%
        """
        ...

    @property
    def details(self) -> Optional[str]:
        """Additional free-form details attached to the report.

        Returns
        -------
        str or None
            Supplemental information captured during evaluation, such as
            calculation methodology or context.

        Examples
        --------
            >>> report = result.covenants["ltv"]
            >>> if report.details:
            ...     print(report.details)
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class ResultsMeta:
    """Snapshot describing numeric mode, rounding context, and FX policy applied to results.

    ResultsMeta provides transparency into the calculation environment, including
    the numeric precision used (f64 vs decimal), rounding rules applied, and any
    FX conversion policies. This metadata is critical for audit trails, reproducibility,
    and understanding result precision.

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import Bond
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> def _meta_example():
        ...     registry = create_standard_registry()
        ...     ctx = MarketContext()
        ...     ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (5.0, 0.95)]))
        ...     bond = Bond.fixed_semiannual(
        ...         "BOND-META",
        ...         Money(1_000_000, Currency("USD")),
        ...         0.04,
        ...         date(2024, 1, 1),
        ...         date(2029, 1, 1),
        ...         "USD",
        ...     )
        ...     return registry.price(bond, "discounting", ctx).meta
        >>> meta = _meta_example()
        >>> meta.numeric_mode in {"f64", "decimal"}
        True

    The returned metadata also exposes rounding information via ``meta.rounding`` and
    can be serialized with ``meta.to_dict()`` for downstream logging.

    Notes
    -----
    - Numeric mode indicates calculation precision ("f64" or "decimal")
    - Rounding context shows rounding mode and per-currency decimal scales
    - FX policy is only present for multi-currency valuations
    - Metadata is immutable and reflects the calculation environment

    See Also
    --------
    :class:`FinstackConfig`: Rounding configuration
    :class:`ValuationResult`: Result envelope containing metadata
    """

    @property
    def numeric_mode(self) -> str:
        """Numeric engine mode used by the pricing engine.

        Returns the numeric precision mode used for calculations. "f64" indicates
        floating-point arithmetic (faster, less precise), while "decimal" indicates
        decimal arithmetic (slower, accounting-grade precision).

        Returns
        -------
        str
            Numeric mode identifier: "f64" (floating-point) or "decimal"
            (decimal arithmetic).
        """
        ...

    @property
    def fx_policy_applied(self) -> Optional[str]:
        """Optional FX policy key applied during result aggregation.

        Returns the FX conversion policy identifier if multi-currency aggregation
        was performed. None for single-currency valuations.

        Returns
        -------
        str or None
            FX policy identifier (e.g., "cashflow_date", "valuation_date") or
            None if no FX conversion was applied.
        """
        ...

    @property
    def rounding(self) -> Dict[str, Any]:
        """Rounding context snapshot as a dictionary.

        Returns the rounding configuration used during calculation, including
        the rounding mode and per-currency decimal scales.

        Returns
        -------
        Dict[str, Any]
            Dictionary containing:
            - "mode": Rounding mode (e.g., "half_even", "ceil", "floor")
            - "scales": Dict[str, int] mapping currency codes to decimal places
        """
        ...

    def to_dict(self) -> Dict[str, Any]:
        """Convert the metadata to a Python dictionary for downstream serialization.

        Serializes the metadata to a dictionary suitable for JSON encoding or
        DataFrame conversion.

        Returns
        -------
        Dict[str, Any]
            Dictionary containing numeric_mode, fx_policy_applied, and rounding
            fields as primitive types.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class ValuationResult:
    """Complete valuation output envelope containing PV, risk metrics, metadata, and covenant reports.

    ValuationResult is the standard return type from all pricing operations in finstack.
    It provides a comprehensive view of the valuation including present value, computed
    risk metrics (DV01, CS01, yield, spread, Greeks, etc.), metadata about the calculation,
    and optional covenant evaluation reports.

    The result envelope is designed for serialization, aggregation, and reporting. It
    maintains currency safety, includes rounding context, and records FX policies applied.

    Examples
    --------
    Basic pricing result:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> from finstack.valuations.instruments import Bond
        >>> registry = create_standard_registry()
        >>> market_ctx = MarketContext()
        >>> market_ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
        >>> bond = Bond.fixed_semiannual(
        ...     "BOND-EXAMPLE",
        ...     Money(1_000_000, Currency("USD")),
        ...     0.05,
        ...     date(2024, 1, 1),
        ...     date(2029, 1, 1),
        ...     "USD",
        ... )
        >>> value = registry.price(bond, "discounting", market_ctx).value
        >>> (value.currency.code, isinstance(value.amount, float))
        ('USD', True)

    Result with risk metrics:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> from finstack.valuations.instruments import Bond
        >>> registry = create_standard_registry()
        >>> market_ctx = MarketContext()
        >>> market_ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
        >>> bond = Bond.fixed_semiannual(
        ...     "BOND-EXAMPLE",
        ...     Money(1_000_000, Currency("USD")),
        ...     0.05,
        ...     date(2024, 1, 1),
        ...     date(2029, 1, 1),
        ...     "USD",
        ... )
        >>> result = registry.price_with_metrics(bond, "discounting", market_ctx, ["dv01", "ytm"])
        >>> sorted(result.measures.keys())
        ['dv01', 'ytm']

    Inspect metadata:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> from finstack.valuations.instruments import Bond
        >>> def _example_result():
        ...     registry = create_standard_registry()
        ...     ctx = MarketContext()
        ...     ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
        ...     bond = Bond.fixed_semiannual(
        ...         "BOND-003",
        ...         Money(1_000_000, Currency("USD")),
        ...         0.05,
        ...         date(2024, 1, 1),
        ...         date(2029, 1, 1),
        ...         "USD",
        ...     )
        ...     return registry.price(bond, "discounting", ctx)
        >>> meta = _example_result().meta
        >>> meta.numeric_mode in {"f64", "decimal"}
        True

    Check covenants:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> from finstack.valuations.instruments import Bond
        >>> def _example_result():
        ...     registry = create_standard_registry()
        ...     ctx = MarketContext()
        ...     ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
        ...     bond = Bond.fixed_semiannual(
        ...         "BOND-004",
        ...         Money(1_000_000, Currency("USD")),
        ...         0.05,
        ...         date(2024, 1, 1),
        ...         date(2029, 1, 1),
        ...         "USD",
        ...     )
        ...     return registry.price(bond, "discounting", ctx)
        >>> result = _example_result()
        >>> if result.covenants:
        ...     for name, report in result.covenants.items():
        ...         if not report.passed:
        ...             print(f"Covenant {name} failed: {report.details}")

    Serialize for reporting:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> from finstack.valuations.instruments import Bond
        >>> def _example_result():
        ...     registry = create_standard_registry()
        ...     ctx = MarketContext()
        ...     ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
        ...     bond = Bond.fixed_semiannual(
        ...         "BOND-005",
        ...         Money(1_000_000, Currency("USD")),
        ...         0.05,
        ...         date(2024, 1, 1),
        ...         date(2029, 1, 1),
        ...         "USD",
        ...     )
        ...     return registry.price(bond, "discounting", ctx)
        >>> data = _example_result().to_dict()
        >>> import json
        >>> json_str = json.dumps(data, default=str)

    Notes
    -----
    - Present value is always in the instrument's currency
    - Metrics are stored as floats (units depend on metric type)
    - Metadata captures numeric mode, rounding, and FX policies
    - Covenants are optional and only present if evaluated
    - Results are immutable and can be safely shared

    See Also
    --------
    :class:`ResultsMeta`: Metadata about the calculation
    :class:`CovenantReport`: Covenant evaluation results
    :class:`PricerRegistry`: Pricing entry point
    :class:`MetricId`: Metric identifiers
    """

    @property
    def instrument_id(self) -> str:
        """Instrument identifier used when stamping the result.

        Returns
        -------
        str
            Unique instrument identifier supplied at pricing time.
            Matches the instrument_id from the instrument being priced.

        Examples
        --------
            >>> from datetime import date
            >>> from finstack.core.currency import Currency
            >>> from finstack.core.money import Money
            >>> from finstack.core.market_data.context import MarketContext
            >>> from finstack.core.market_data.term_structures import DiscountCurve
            >>> from finstack.valuations.pricer import create_standard_registry
            >>> from finstack.valuations.instruments import Bond
            >>> def _example_result():
            ...     registry = create_standard_registry()
            ...     ctx = MarketContext()
            ...     ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
            ...     bond = Bond.fixed_semiannual(
            ...         "BOND-001",
            ...         Money(1_000_000, Currency("USD")),
            ...         0.05,
            ...         date(2024, 1, 1),
            ...         date(2029, 1, 1),
            ...         "USD",
            ...     )
            ...     return registry.price(bond, "discounting", ctx)
            >>> result = _example_result()
            >>> result.instrument_id
            'BOND-001'
        """
        ...

    @property
    def as_of(self) -> date:
        """Valuation date associated with the pricing run.

        Returns
        -------
        date
            Effective market date for the valuation. All market data
            (curves, surfaces, FX rates) are observed as of this date.

        Examples
        --------
            >>> from datetime import date
            >>> from finstack.core.currency import Currency
            >>> from finstack.core.money import Money
            >>> from finstack.core.market_data.context import MarketContext
            >>> from finstack.core.market_data.term_structures import DiscountCurve
            >>> from finstack.valuations.pricer import create_standard_registry
            >>> from finstack.valuations.instruments import Bond
            >>> def _example_result():
            ...     registry = create_standard_registry()
            ...     ctx = MarketContext()
            ...     ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
            ...     bond = Bond.fixed_semiannual(
            ...         "BOND-001",
            ...         Money(1_000_000, Currency("USD")),
            ...         0.05,
            ...         date(2024, 1, 1),
            ...         date(2029, 1, 1),
            ...         "USD",
            ...     )
            ...     return registry.price(bond, "discounting", ctx)
            >>> _example_result().as_of == date(2024, 1, 1)
            True
        """
        ...

    @property
    def value(self) -> Money:
        """Present value expressed as Money.

        The present value is the net present value of all cashflows discounted
        to the valuation date using the appropriate discount curves. For options
        and derivatives, this represents the option premium or swap value.

        Returns
        -------
        Money
            Present value of the instrument in its native currency.

        Examples
        --------
            >>> from datetime import date
            >>> from finstack.core.currency import Currency
            >>> from finstack.core.money import Money
            >>> from finstack.core.market_data.context import MarketContext
            >>> from finstack.core.market_data.term_structures import DiscountCurve
            >>> from finstack.valuations.pricer import create_standard_registry
            >>> from finstack.valuations.instruments import Bond
            >>> def _example_result():
            ...     registry = create_standard_registry()
            ...     ctx = MarketContext()
            ...     ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
            ...     bond = Bond.fixed_semiannual(
            ...         "BOND-001",
            ...         Money(1_000_000, Currency("USD")),
            ...         0.05,
            ...         date(2024, 1, 1),
            ...         date(2029, 1, 1),
            ...         "USD",
            ...     )
            ...     return registry.price(bond, "discounting", ctx)
            >>> value = _example_result().value
            >>> value.currency.code
            'USD'
            >>> isinstance(value.amount, float)
            True

        Notes
        -----
        - PV is always in the instrument's currency
        - For bonds: PV = sum of discounted cashflows
        - For swaps: PV = fixed leg PV - floating leg PV (or vice versa)
        - For options: PV = option premium
        - Negative PV indicates a liability (e.g., short position)
        """
        ...

    @property
    def measures(self) -> Dict[str, float]:
        """Dictionary of computed risk and return measures.

        The measures dictionary contains all requested metrics computed during
        pricing. Common metrics include:
        - Risk: "dv01", "cs01", "theta", "delta", "gamma", "vega", "rho"
        - Yield: "ytm", "ytw", "yield"
        - Spread: "z_spread", "oas", "i_spread", "asw_spread"
        - Pricing: "clean_price", "dirty_price", "accrued_interest"
        - Duration: "duration_modified", "duration_macaulay", "convexity"

        Returns
        -------
        Dict[str, float]
            Dictionary of calculated metrics keyed by metric identifier
            (snake_case). Values are floats with units depending on the metric:
            - DV01/CS01: Dollar value (currency units)
            - Yield/Spread: Decimal (e.g., 0.035 for 3.5%)
            - Greeks: Sensitivity units (e.g., delta in shares, vega in vol points)

        Examples
        --------
            >>> from datetime import date
            >>> from finstack.core.currency import Currency
            >>> from finstack.core.money import Money
            >>> from finstack.core.market_data.context import MarketContext
            >>> from finstack.core.market_data.term_structures import DiscountCurve
            >>> from finstack.valuations.pricer import create_standard_registry
            >>> from finstack.valuations.instruments import Bond
            >>> def _metrics_result():
            ...     registry = create_standard_registry()
            ...     ctx = MarketContext()
            ...     ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
            ...     bond = Bond.fixed_semiannual(
            ...         "BOND-002",
            ...         Money(1_000_000, Currency("USD")),
            ...         0.05,
            ...         date(2024, 1, 1),
            ...         date(2029, 1, 1),
            ...         "USD",
            ...     )
            ...     return registry.price_with_metrics(bond, "discounting", ctx, ["dv01", "ytm"])
            >>> sorted(_metrics_result().measures.keys())
            ['dv01', 'ytm']

        Notes
        -----
        - Metrics are only present if requested via price_with_metrics()
        - Metric availability depends on instrument type
        - Missing metrics are simply absent from the dictionary (no error)
        - Use MetricRegistry to check which metrics are available for an instrument

        See Also
        --------
        :class:`MetricId`: Standard metric identifiers
        :class:`MetricRegistry`: Metric availability checker
        :meth:`PricerRegistry.price_with_metrics`: Request metrics during pricing
        """
        ...

    @property
    def meta(self) -> ResultsMeta:
        """Metadata describing numeric mode, rounding context, and FX policy.

        The metadata provides transparency into how the valuation was computed,
        including the numeric precision used, rounding rules applied, and any
        FX conversion policies. This is critical for audit trails and ensuring
        reproducibility.

        Returns
        -------
        ResultsMeta
            Snapshot of metadata associated with the valuation, including:
            - numeric_mode: Precision mode ("f64" or "decimal")
            - rounding: Rounding context (mode and per-currency scales)
            - fx_policy_applied: FX policy identifier if multi-currency

        Examples
        --------
            >>> result = registry.price(bond, "discounting", market_ctx)
            >>> print(result.meta.numeric_mode)
            'f64'
            >>> print(result.meta.rounding)
            {'mode': 'half_even', 'scales': {'USD': 2}}
            >>> print(result.meta.fx_policy_applied)
            None

        Notes
        -----
        - Metadata is immutable and reflects the calculation environment
        - Use to_dict() for serialization
        - Rounding context affects how monetary values are displayed
        - FX policy is only present for multi-currency valuations

        See Also
        --------
        :class:`ResultsMeta`: Metadata structure
        :class:`FinstackConfig`: Rounding configuration
        """
        ...

    @property
    def covenants(self) -> Optional[Dict[str, CovenantReport]]:
        """Covenant reports (if any) keyed by covenant identifier.

        Covenants are optional checks performed during valuation (e.g., LTV ratios,
        debt service coverage, leverage limits). They are typically used for
        private credit and real estate instruments.

        Returns
        -------
        Dict[str, CovenantReport] or None
            Dictionary of covenant evaluations when available, keyed by covenant
            identifier (e.g., "ltv", "dscr"). None if no covenants were evaluated.

        Examples
        --------
            >>> result = registry.price(loan, "credit", market_ctx)
            >>> if result.covenants:
            ...     ltv_report = result.covenants.get("ltv")
            ...     if ltv_report:
            ...         print(f"LTV: {ltv_report.actual_value:.2%}")
            ...         print(f"Threshold: {ltv_report.threshold:.2%}")
            ...         print(f"Passed: {ltv_report.passed}")
            LTV: 65.00%
            Threshold: 75.00%
            Passed: True

        Notes
        -----
        - Covenants are optional and only present if evaluated
        - Use all_covenants_passed() for quick check
        - Use failed_covenants() to get list of failures
        - Covenant evaluation requires instrument-specific configuration

        See Also
        --------
        :class:`CovenantReport`: Individual covenant report structure
        :meth:`all_covenants_passed`: Check if all covenants passed
        :meth:`failed_covenants`: Get list of failed covenants
        """
        ...

    def all_covenants_passed(self) -> bool:
        """Convenience helper returning True when all covenants passed.

        Returns True if there are no covenants, or if all covenant reports
        have passed=True. Returns False if any covenant failed.

        Returns
        -------
        bool
            True when there are no failing covenant reports. False if any
            covenant failed or if covenants is None.

        Examples
        --------
            >>> result = registry.price(loan, "credit", market_ctx)
            >>> if result.all_covenants_passed():
            ...     print("All covenants passed")
            ... else:
            ...     print(f"Failed: {result.failed_covenants()}")
            All covenants passed

        Notes
        -----
        - Returns True if covenants is None (no covenants evaluated)
        - Returns True if all reports have passed=True
        - Returns False if any report has passed=False
        """
        ...

    def failed_covenants(self) -> List[str]:
        """List of covenant identifiers that failed (empty when all pass).

        Returns a list of covenant identifiers where the covenant check failed
        (passed=False). Returns an empty list if all passed or if no covenants
        were evaluated.

        Returns
        -------
        List[str]
            List of covenant identifiers that evaluated to False. Empty list
            if all passed or if covenants is None.

        Examples
        --------
            >>> result = registry.price(loan, "credit", market_ctx)
            >>> failures = result.failed_covenants()
            >>> if failures:
            ...     for name in failures:
            ...         report = result.covenants[name]
            ...         print(f"{name}: {report.actual_value} vs {report.threshold}")
            >>> else:
            ...     print("All covenants passed")
            All covenants passed

        Notes
        -----
        - Returns empty list if covenants is None
        - Returns empty list if all covenants passed
        - Use with covenants dictionary to get full report details
        """
        ...

    def to_dict(self) -> Dict[str, Any]:
        """Convert to a Python dictionary for JSON/Arrow serialization.

        Serializes the entire valuation result to a dictionary suitable for
        JSON encoding, Arrow serialization, or DataFrame conversion. All
        nested objects (Money, date, ResultsMeta) are converted to primitive
        types.

        Returns
        -------
        Dict[str, Any]
            Serializable dictionary containing:
            - instrument_id: str
            - as_of: str (ISO date format)
            - value: dict with "amount" and "currency"
            - measures: dict[str, float]
            - meta: dict (from ResultsMeta.to_dict())
            - covenants: dict[str, dict] or None

        Examples
        --------
            >>> result = registry.price(bond, "discounting", market_ctx)
            >>> data = result.to_dict()
            >>> # Serialize to JSON
            >>> import json
            >>> json_str = json.dumps(data, indent=2)
            >>> # Convert to DataFrame row
            >>> import pandas as pd
            >>> df = pd.DataFrame([data])

        Notes
        -----
        - All dates are serialized as ISO strings ("YYYY-MM-DD")
        - Money objects become {"amount": float, "currency": str}
        - Metadata is flattened to a dictionary
        - Suitable for pipeline integration and reporting

        See Also
        --------
        :meth:`ResultsMeta.to_dict`: Metadata serialization
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
