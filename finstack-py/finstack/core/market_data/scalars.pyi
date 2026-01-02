"""Market scalar and time series bindings.

Provides market prices, time series, and interpolation
for scalar market data.
"""

from typing import List, Tuple, Optional, Union
from datetime import date
from ..currency import Currency
from ..money import Money

class SeriesInterpolation:
    """Interpolation method for time series.

    Available methods:
    - Linear: Linear interpolation
    - Step: Step function (left-continuous)
    - LogLinear: Log-linear interpolation
    """

    @classmethod
    def from_name(cls, name: str) -> SeriesInterpolation:
        """Create from string name.

        Parameters
        ----------
        name : str
            Method name (case-insensitive).

        Returns
        -------
        SeriesInterpolation
            Interpolation method instance.
        """
        ...

    @property
    def name(self) -> str:
        """Get the method name.

        Returns
        -------
        str
            Human-readable method name.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

# Interpolation method constants
Linear: SeriesInterpolation
Step: SeriesInterpolation
LogLinear: SeriesInterpolation

class MarketScalar:
    """Market scalar value (price or unitless).

    Parameters
    ----------
    value : float or Money
        Scalar value or money amount.
    """

    @classmethod
    def unitless(cls, value: float) -> MarketScalar:
        """Create a unitless scalar.

        Parameters
        ----------
        value : float
            Unitless value.

        Returns
        -------
        MarketScalar
            Unitless scalar.
        """
        ...

    @classmethod
    def price(cls, money: "Money") -> MarketScalar:
        """Create a price scalar.

        Parameters
        ----------
        money : Money
            Money amount.

        Returns
        -------
        MarketScalar
            Price scalar.
        """
        ...

    @property
    def is_unitless(self) -> bool:
        """Check if this is a unitless scalar.

        Returns
        -------
        bool
            True if unitless.
        """
        ...

    @property
    def is_price(self) -> bool:
        """Check if this is a price scalar.

        Returns
        -------
        bool
            True if price.
        """
        ...

    @property
    def value(self) -> Union[float, "Money"]:
        """Get the scalar value.

        This is exposed as a property in the Python bindings, so you access it
        as ``scalar.value`` (not ``scalar.value()``).

        Returns
        -------
        float or Money
            Scalar value.
        """
        ...

    def __repr__(self) -> str: ...

class ScalarTimeSeries:
    """Time series of scalar values.

    Parameters
    ----------
    id : str
        Series identifier.
    observations : list[tuple[date, float]]
        (date, value) pairs.
    currency : Currency, optional
        Currency for the series.
    interpolation : SeriesInterpolation, optional
        Interpolation method.
    """

    def __init__(
        self,
        id: str,
        observations: List[Tuple[Union[str, date], float]],
        currency: Optional[Currency] = None,
        interpolation: Optional[SeriesInterpolation] = None,
    ) -> None: ...
    def set_interpolation(self, interpolation: SeriesInterpolation) -> None:
        """Set the interpolation method.

        Parameters
        ----------
        interpolation : SeriesInterpolation
            New interpolation method.
        """
        ...

    @property
    def id(self) -> str:
        """Get the series identifier.

        Returns
        -------
        str
            Series ID.
        """
        ...

    @property
    def currency(self) -> Optional[Currency]:
        """Get the currency.

        Returns
        -------
        Currency or None
            Currency if set.
        """
        ...

    @property
    def interpolation(self) -> SeriesInterpolation:
        """Get the interpolation method.

        Returns
        -------
        SeriesInterpolation
            Interpolation method.
        """
        ...

    def value_on(self, date: Union[str, date]) -> float:
        """Get value on a specific date.

        Parameters
        ----------
        date : str or date
            Target date.

        Returns
        -------
        float
            Interpolated value.
        """
        ...

    def values_on(self, dates: List[Union[str, date]]) -> List[float]:
        """Get values on multiple dates.

        Parameters
        ----------
        dates : List[str or date]
            Target dates.

        Returns
        -------
        List[float]
            Interpolated values.
        """
        ...

    def __repr__(self) -> str: ...
