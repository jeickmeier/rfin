"""IMM (International Money Market) date utilities.

Provides functions for calculating IMM dates and related
financial market dates.
"""

from typing import Union
from datetime import date

def next_imm(date: Union[str, date]) -> date:
    """Get the next IMM date after the given date.

    Parameters
    ----------
    date : str or date
        Base date.

    Returns
    -------
    date
        Next IMM date (third Wednesday of March, June, September, or December).
    """
    ...

def next_cds_date(date: Union[str, date]) -> date:
    """Get the next CDS date after the given date.

    Parameters
    ----------
    date : str or date
        Base date.

    Returns
    -------
    date
        Next CDS date (20th of March, June, September, or December).
    """
    ...

def next_imm_option_expiry(date: Union[str, date]) -> date:
    """Get the next IMM option expiry date.

    Parameters
    ----------
    date : str or date
        Base date.

    Returns
    -------
    date
        Next IMM option expiry date.
    """
    ...

def imm_option_expiry(year: int, month: int) -> date:
    """Get the IMM option expiry for a specific year/month.

    Parameters
    ----------
    year : int
        Calendar year.
    month : int
        Month (1-12).

    Returns
    -------
    date
        IMM option expiry date.
    """
    ...

def next_equity_option_expiry(date: Union[str, date]) -> date:
    """Get the next equity option expiry date.

    Parameters
    ----------
    date : str or date
        Base date.

    Returns
    -------
    date
        Next equity option expiry date (third Friday of the month).
    """
    ...

def third_friday(year: int, month: int) -> date:
    """Get the third Friday of a month.

    Parameters
    ----------
    year : int
        Calendar year.
    month : int
        Month (1-12).

    Returns
    -------
    date
        Third Friday of the month.
    """
    ...

def third_wednesday(year: int, month: int) -> date:
    """Get the third Wednesday of a month.

    Parameters
    ----------
    year : int
        Calendar year.
    month : int
        Month (1-12).

    Returns
    -------
    date
        Third Wednesday of the month.
    """
    ...
