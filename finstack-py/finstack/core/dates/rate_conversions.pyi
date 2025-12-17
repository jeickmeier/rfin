"""Interest rate conversion utilities.

Provides functions to convert between simple, periodic, and continuous interest rates.
"""

def simple_to_periodic(
    simple_rate: float,
    year_fraction: float,
    periods_per_year: int,
) -> float:
    """Convert a simple (linear) interest rate to a periodically compounded rate.

    Parameters
    ----------
    simple_rate : float
        The simple interest rate.
    year_fraction : float
        Time period as a fraction of a year.
    periods_per_year : int
        Compounding frequency (e.g., 2 for semi-annual).

    Returns
    -------
    float
        Equivalent periodically compounded rate.
    """
    ...

def periodic_to_simple(
    periodic_rate: float,
    year_fraction: float,
    periods_per_year: int,
) -> float:
    """Convert a periodically compounded rate to a simple (linear) rate.

    Parameters
    ----------
    periodic_rate : float
        Periodically compounded rate.
    year_fraction : float
        Time period as a fraction of a year.
    periods_per_year : int
        Compounding frequency.

    Returns
    -------
    float
        Equivalent simple interest rate.
    """
    ...

def periodic_to_continuous(
    periodic_rate: float,
    periods_per_year: int,
) -> float:
    """Convert a periodically compounded rate to a continuously compounded rate.

    Parameters
    ----------
    periodic_rate : float
        Periodically compounded rate.
    periods_per_year : int
        Compounding frequency.

    Returns
    -------
    float
        Equivalent continuously compounded rate.
    """
    ...

def continuous_to_periodic(
    continuous_rate: float,
    periods_per_year: int,
) -> float:
    """Convert a continuously compounded rate to a periodically compounded rate.

    Parameters
    ----------
    continuous_rate : float
        Continuously compounded rate.
    periods_per_year : int
        Target compounding frequency.

    Returns
    -------
    float
        Equivalent periodically compounded rate.
    """
    ...

def simple_to_continuous(
    simple_rate: float,
    year_fraction: float,
) -> float:
    """Convert a simple rate to a continuously compounded rate.

    Parameters
    ----------
    simple_rate : float
        Simple interest rate.
    year_fraction : float
        Time period as a fraction of a year.

    Returns
    -------
    float
        Equivalent continuously compounded rate.
    """
    ...

def continuous_to_simple(
    continuous_rate: float,
    year_fraction: float,
) -> float:
    """Convert a continuously compounded rate to a simple rate.

    Parameters
    ----------
    continuous_rate : float
        Continuously compounded rate.
    year_fraction : float
        Time period as a fraction of a year.

    Returns
    -------
    float
        Equivalent simple interest rate.
    """
    ...
