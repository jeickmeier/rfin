"""Dividend schedule bindings.

Provides dividend event modeling and schedule management
for equity instruments.
"""

from __future__ import annotations

from datetime import date as _Date
from typing import List, Tuple
from ..currency import Currency
from ..money import Money

class DividendEvent:
    """A single dividend event.

    Parameters
    ----------
    date : str or date
        Dividend date.
    kind : str
        Dividend kind ("cash", "yield", "stock").
    """

    def __init__(self, date: str | _Date, kind: str) -> None: ...
    def date(self) -> _Date:
        """Get the dividend date.

        Returns
        -------
        date
            Dividend date.
        """
        ...

    @property
    def kind(self) -> str:
        """Get the dividend kind.

        Returns
        -------
        str
            Dividend kind.
        """
        ...

    def cash_amount(self) -> Money | None:
        """Get the cash amount if this is a cash dividend.

        Returns
        -------
        Money or None
            Cash amount if applicable.
        """
        ...

    def dividend_yield(self) -> float | None:
        """Get the dividend yield if this is a yield dividend.

        Returns
        -------
        float or None
            Dividend yield if applicable.
        """
        ...

    def stock_ratio(self) -> float | None:
        """Get the stock ratio if this is a stock dividend.

        Returns
        -------
        float or None
            Stock ratio if applicable.
        """
        ...

    def __repr__(self) -> str: ...

class DividendSchedule:
    """Dividend schedule for an equity instrument.

    Parameters
    ----------
    id : str
        Schedule identifier.
    events : list[DividendEvent]
        Dividend events.
    underlying : str, optional
        Underlying instrument identifier.
    currency : Currency, optional
        Currency of the dividends.
    """

    def __init__(
        self,
        id: str,
        events: List[DividendEvent],
        underlying: str | None = None,
        currency: Currency | None = None,
    ) -> None: ...
    @property
    def id(self) -> str:
        """Get the schedule identifier.

        Returns
        -------
        str
            Schedule ID.
        """
        ...

    @property
    def underlying(self) -> str | None:
        """Get the underlying identifier.

        Returns
        -------
        str or None
            Underlying instrument ID.
        """
        ...

    @property
    def currency(self) -> Currency | None:
        """Get the currency.

        Returns
        -------
        Currency or None
            Currency of the dividends.
        """
        ...

    @property
    def events(self) -> List[DividendEvent]:
        """Get the dividend events.

        Returns
        -------
        List[DividendEvent]
            All dividend events.
        """
        ...

    def cash_events(self) -> List[Tuple[_Date, Money]]:
        """Get cash dividend events.

        Returns
        -------
        List[Tuple[date, Money]]
            (date, amount) pairs for cash dividends.
        """
        ...

    def __repr__(self) -> str: ...

class DividendScheduleBuilder:
    """Builder for dividend schedules.

    Parameters
    ----------
    id : str
        Schedule identifier.
    """

    def __init__(self, id: str) -> None: ...
    def underlying(self, underlying: str) -> None:
        """Set the underlying instrument.

        Parameters
        ----------
        underlying : str
            Underlying instrument identifier.
        """
        ...

    def currency(self, currency: Currency) -> None:
        """Set the currency.

        Parameters
        ----------
        currency : Currency
            Currency for the dividends.
        """
        ...

    def cash(self, date: str | _Date, amount: Money) -> None:
        """Add a cash dividend.

        Parameters
        ----------
        date : str or date
            Dividend date.
        amount : Money
            Cash amount.
        """
        ...

    def yield_div(self, date: str | _Date, yield_value: float) -> None:
        """Add a yield dividend.

        Parameters
        ----------
        date : str or date
            Dividend date.
        yield_value : float
            Dividend yield.
        """
        ...

    def stock(self, date: str | _Date, ratio: float) -> None:
        """Add a stock dividend.

        Parameters
        ----------
        date : str or date
            Dividend date.
        ratio : float
            Stock ratio.
        """
        ...

    def build(self) -> DividendSchedule:
        """Build the dividend schedule.

        Returns
        -------
        DividendSchedule
            Constructed dividend schedule.
        """
        ...

    def __repr__(self) -> str: ...
