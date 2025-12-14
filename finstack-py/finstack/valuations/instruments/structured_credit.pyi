"""Structured credit instrument."""

from typing import Optional, Dict, Any, Union
from ..common import InstrumentType

class StructuredCredit:
    """Unified structured credit instrument wrapper (ABS, CLO, CMBS, RMBS).

    StructuredCredit represents securitized credit products including asset-backed
    securities (ABS), collateralized loan obligations (CLO), commercial mortgage-backed
    securities (CMBS), and residential mortgage-backed securities (RMBS).

    Structured credit instruments are complex multi-tranche products with waterfall
    payment structures. They are typically defined via JSON and require sophisticated
    cashflow modeling.

    Examples
    --------
    Create a structured credit from JSON:

        >>> from finstack.valuations.instruments import StructuredCredit
        >>> import json
        >>> # StructuredCredit requires complex JSON - see examples/structured_credit_capabilities.py
        >>> # for complete examples. Minimal example:
        >>> json_data = {
        ...     "id": "CLO-001",
        ...     "deal_type": "CLO",
        ...     "closing_date": "2024-01-01",
        ...     "first_payment_date": "2025-04-01",
        ...     "legal_maturity": "2034-01-01",
        ...     "payment_frequency": {"count": 3, "unit": "months"},
        ...     "discount_curve_id": "USD-OIS",
        ...     "tranches": {
        ...         "total_size": {"amount": 100000000, "currency": "USD"},
        ...         "tranches": [
        ...             {
        ...                 "id": "A",
        ...                 "attachment_point": 0.0,
        ...                 "detachment_point": 10.0,
        ...                 "seniority": "Senior",
        ...                 "original_balance": {"amount": 10000000, "currency": "USD"},
        ...                 "current_balance": {"amount": 10000000, "currency": "USD"},
        ...                 "coupon": {"Fixed": {"rate": 0.05}},
        ...                 "legal_maturity": "2030-01-01",
        ...                 "payment_frequency": {"count": 3, "unit": "months"},
        ...                 "day_count": "Act360",
        ...                 "payment_priority": 1,
        ...                 "credit_enhancement": {
        ...                     "cash_trap_active": False,
        ...                     "excess_spread": 0.0,
        ...                     "overcollateralization": {"amount": 0, "currency": "USD"},
        ...                     "reserve_account": {"amount": 0, "currency": "USD"},
        ...                     "subordination": {"amount": 0, "currency": "USD"},
        ...                 },
        ...                 "attributes": {"meta": {}, "tags": []},
        ...                 "behavior_type": "Standard",
        ...                 "is_revolving": False,
        ...                 "can_reinvest": False,
        ...                 "deferred_interest": {"amount": 0, "currency": "USD"},
        ...             }
        ...         ],
        ...     },
        ...     "pool": {
        ...         "id": "POOL-1",
        ...         "deal_type": "CLO",
        ...         "assets": [],
        ...         "cumulative_defaults": {"amount": 0, "currency": "USD"},
        ...         "cumulative_recoveries": {"amount": 0, "currency": "USD"},
        ...         "cumulative_prepayments": {"amount": 0, "currency": "USD"},
        ...         "reinvestment_period": None,
        ...         "collection_account": {"amount": 0, "currency": "USD"},
        ...         "reserve_account": {"amount": 0, "currency": "USD"},
        ...         "excess_spread_account": {"amount": 0, "currency": "USD"},
        ...     },
        ...     "attributes": {"meta": {}, "tags": []},
        ...     "prepayment_spec": {"type": "constant_cpr", "cpr": 0.15},
        ...     "default_spec": {"type": "constant_cdr", "cdr": 0.025},
        ...     "recovery_spec": {"type": "constant", "rate": 0.4, "recovery_lag": 12},
        ...     "market_conditions": {
        ...         "refi_rate": 0.04,
        ...         "original_rate": None,
        ...         "hpa": None,
        ...         "unemployment": None,
        ...         "seasonal_factor": 1.0,
        ...         "custom_factors": {},
        ...     },
        ...     "credit_factors": {
        ...         "credit_score": None,
        ...         "dti": None,
        ...         "ltv": None,
        ...         "delinquency_days": 0,
        ...         "unemployment_rate": None,
        ...         "custom_factors": {},
        ...     },
        ...     "deal_metadata": {
        ...         "manager_id": None,
        ...         "servicer_id": None,
        ...         "master_servicer_id": None,
        ...         "special_servicer_id": None,
        ...         "trustee_id": None,
        ...     },
        ...     "behavior_overrides": {
        ...         "cpr_annual": None,
        ...         "abs_speed": None,
        ...         "psa_speed_multiplier": None,
        ...         "cdr_annual": None,
        ...         "sda_speed_multiplier": None,
        ...         "recovery_rate": None,
        ...         "recovery_lag_months": None,
        ...         "reinvestment_price": None,
        ...     },
        ...     "default_assumptions": {
        ...         "base_cdr_annual": 0.02,
        ...         "base_recovery_rate": 0.4,
        ...         "base_cpr_annual": 0.15,
        ...         "psa_speed": None,
        ...         "sda_speed": None,
        ...         "abs_speed_monthly": None,
        ...         "cpr_by_asset_type": {},
        ...         "cdr_by_asset_type": {},
        ...         "recovery_by_asset_type": {},
        ...     },
        ... }
        >>> structured_credit = StructuredCredit.from_json(json.dumps(json_data))

    Notes
    -----
    - Structured credit instruments are defined via JSON
    - Deal types: "ABS", "CLO", "CMBS", "RMBS"
    - Multiple tranches with different seniority and risk
    - Waterfall payment structure determines cashflow distribution
    - Requires collateral pool and payment rules

    See Also
    --------
    :class:`CdsTranche`: CDS tranches
    :class:`Bond`: Standard bonds
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def from_json(cls, data: Union[str, Dict[str, Any]]) -> "StructuredCredit": ...
    """Parse a JSON payload into a structured credit instrument.

    Parameters
    ----------
    data : str or Dict[str, Any]
        JSON string or dictionary containing structured credit definition.
        Must include deal_type, tranches, and collateral pool specifications.

    Returns
    -------
    StructuredCredit
        Configured structured credit instrument ready for pricing.

    Raises
    ------
    ValueError
        If JSON is invalid or required fields are missing.

    Examples
    --------
        >>> json_str = '{"deal_type": "CLO", "tranches": [...]}'
        >>> structured_credit = StructuredCredit.from_json(json_str)
        >>> structured_credit.deal_type
        'CLO'
    """

    def to_json(self) -> str:
        """Serialize the structured credit definition back to JSON."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def deal_type(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def tranche_count(self) -> int: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
