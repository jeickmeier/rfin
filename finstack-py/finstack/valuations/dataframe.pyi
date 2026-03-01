"""DataFrame export utilities for valuation results (Polars, Pandas, Parquet).

Convenience functions for converting a batch of :class:`ValuationResult`
objects into tabular formats suitable for analysis, reporting, and
serialization.

Examples
--------
>>> from finstack.valuations.dataframe import results_to_polars
>>> df = results_to_polars([result1, result2, result3])
>>> print(df.schema)
"""

from __future__ import annotations

import pandas
import polars

from finstack.valuations.results import ValuationResult


def results_to_polars(results: list[ValuationResult]) -> polars.DataFrame:
    """Convert valuation results to a Polars DataFrame.

    Parameters
    ----------
    results : list[ValuationResult]
        Valuation results to convert.

    Returns
    -------
    polars.DataFrame
        DataFrame with columns including ``instrument_id``, ``as_of_date``,
        ``pv``, ``currency``, and any computed metrics (``dv01``, ``ytm``,
        ``duration``, ``convexity``, etc.).

    Examples
    --------
    >>> from finstack.valuations.dataframe import results_to_polars
    >>> df = results_to_polars([result1, result2])
    >>> df.columns
    ['instrument_id', 'as_of_date', 'pv', 'currency', ...]

    Raises
    ------
    RuntimeError
        If DataFrame construction fails.
    """
    ...


def results_to_pandas(results: list[ValuationResult]) -> pandas.DataFrame:
    """Convert valuation results to a Pandas DataFrame.

    Internally converts to Polars first and then calls ``.to_pandas()``.

    Parameters
    ----------
    results : list[ValuationResult]
        Valuation results to convert.

    Returns
    -------
    pandas.DataFrame
        DataFrame with the same schema as :func:`results_to_polars`.

    Examples
    --------
    >>> from finstack.valuations.dataframe import results_to_pandas
    >>> df = results_to_pandas([result1, result2])
    >>> df.dtypes

    Raises
    ------
    RuntimeError
        If DataFrame construction fails.
    """
    ...


def results_to_parquet(results: list[ValuationResult], path: str) -> None:
    """Write valuation results to a Parquet file.

    Internally converts to Polars first and then calls
    ``.write_parquet(path)``.

    Parameters
    ----------
    results : list[ValuationResult]
        Valuation results to write.
    path : str
        Output file path.

    Examples
    --------
    >>> from finstack.valuations.dataframe import results_to_parquet
    >>> results_to_parquet([result1, result2], "valuations.parquet")

    Raises
    ------
    RuntimeError
        If Parquet writing fails.
    """
    ...


__all__ = [
    "results_to_polars",
    "results_to_pandas",
    "results_to_parquet",
]
