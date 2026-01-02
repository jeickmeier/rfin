"""Dividend schedule bindings.

Provides dividend event modeling and schedule management
for equity instruments.
"""

from typing import List, Optional, Union, Tuple
from datetime import date
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

    def __init__(self, date: Union[str, date], kind: str) -> None: ...
    def date(self) -> date:
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

    def cash_amount(self) -> Optional[Money]:
        """Get the cash amount if this is a cash dividend.

        Returns
        -------
        Money or None
            Cash amount if applicable.
        """
        ...

    def dividend_yield(self) -> Optional[float]:
        """Get the dividend yield if this is a yield dividend.

        Returns
        -------
        float or None
            Dividend yield if applicable.
        """
        ...

    def stock_ratio(self) -> Optional[float]:
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
        underlying: Optional[str] = None,
        currency: Optional[Currency] = None,
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
    def underlying(self) -> Optional[str]:
        """Get the underlying identifier.

        Returns
        -------
        str or None
            Underlying instrument ID.
        """
        ...

    @property
    def currency(self) -> Optional[Currency]:
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

    def cash_events(self) -> List[Tuple[date, Money]]:
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

    def cash(self, date: Union[str, date], amount: Money) -> None:
        """Add a cash dividend.

        Parameters
        ----------
        date : str or date
            Dividend date.
        amount : Money
            Cash amount.
        """
        ...

    def yield_div(self, date: Union[str, date], yield_value: float) -> None:
        """Add a yield dividend.

        Parameters
        ----------
        date : str or date
            Dividend date.
        yield_value : float
            Dividend yield.
        """
        ...

    def stock(self, date: Union[str, date], ratio: float) -> None:
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
