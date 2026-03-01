"""Levered real estate equity instrument."""

from __future__ import annotations

from datetime import date
from typing import Any

from ...core.currency import Currency
from .real_estate import RealEstateAsset
from ..common import InstrumentType


class LeveredRealEstateEquity:
    """Levered real estate equity position.

    Combines a real estate asset with a financing stack (term loans, bonds,
    convertible bonds, revolving credit facilities, repos) to produce
    a levered equity position.

    Examples
    --------
    Create a levered real estate equity position:

        >>> from finstack.valuations.instruments import LeveredRealEstateEquity, RealEstateAsset
        >>> equity = LeveredRealEstateEquity.create(
        ...     "LEVERED-OFFICE-001",
        ...     currency="USD",
        ...     asset=asset,
        ...     financing=[term_loan],
        ...     discount_curve_id="USD-OIS",
        ... )
    """

    @classmethod
    def create(
        cls,
        instrument_id: str,
        *,
        currency: str | Currency,
        asset: RealEstateAsset,
        financing: list[Any],
        discount_curve_id: str,
        exit_date: date | None = None,
    ) -> LeveredRealEstateEquity:
        """Create a levered real estate equity position.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for this instrument.
        currency : str or Currency
            Currency for valuation.
        asset : RealEstateAsset
            The underlying real estate asset.
        financing : list
            List of financing instruments (TermLoan, Bond, ConvertibleBond,
            RevolvingCredit, Repo, or JSON strings).
        discount_curve_id : str
            Discount curve ID for discounting.
        exit_date : date, optional
            Optional exit/disposition date.

        Returns
        -------
        LeveredRealEstateEquity
            Configured levered real estate equity position.
        """
        ...
    @classmethod
    def from_json(cls, json_str: str) -> LeveredRealEstateEquity:
        """Deserialize from JSON string."""
        ...
    def to_json(self) -> str:
        """Serialize to JSON string."""
        ...
    @property
    def instrument_id(self) -> str:
        """Instrument identifier."""
        ...
    @property
    def currency(self) -> Currency:
        """Currency."""
        ...
    @property
    def discount_curve_id(self) -> str:
        """Discount curve identifier."""
        ...
    @property
    def exit_date(self) -> date | None:
        """Exit/disposition date (if set)."""
        ...
    @property
    def asset(self) -> RealEstateAsset:
        """Underlying real estate asset."""
        ...
    @property
    def financing_json(self) -> list[str]:
        """Financing instruments as JSON strings."""
        ...
    @property
    def instrument_type(self) -> InstrumentType:
        """Instrument type key."""
        ...
    def __repr__(self) -> str: ...
