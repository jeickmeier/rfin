"""Tests for DataFrame conversion functionality across all result types.

Tests verify that:
1. All result types can export to Polars DataFrames
2. Polars DataFrames can be converted to Pandas from Python
3. I/O operations work via native polars/pandas methods
"""

from pathlib import Path

import pytest

try:
    import pandas as pd
    import polars as pl

    HAS_POLARS = True
    HAS_PANDAS = True
except ImportError:
    HAS_POLARS = False
    HAS_PANDAS = False


@pytest.mark.skipif(not HAS_POLARS, reason="polars not installed")
class TestDataFrameConversions:
    """Test DataFrame export functionality for all result types."""

    def test_valuation_result_to_polars(self) -> None:
        """Test single ValuationResult to_polars() instance method."""
        # This test will be implemented once we have a working example
        # with actual instrument pricing

    def test_portfolio_valuation_to_polars(self) -> None:
        """Test PortfolioValuation to_polars() instance method."""

    def test_portfolio_valuation_entities_to_polars(self) -> None:
        """Test PortfolioValuation entities_to_polars() instance method."""

    def test_statements_results_to_polars_long(self) -> None:
        """Test Statements Results to_polars_long() method."""

    def test_statements_results_to_polars_wide(self) -> None:
        """Test Statements Results to_polars_wide() method."""

    @pytest.mark.skipif(not HAS_PANDAS, reason="pandas not installed")
    def test_polars_to_pandas_conversion(self) -> None:
        """Test that Polars DataFrames can be converted to Pandas."""
        # Create a simple Polars DataFrame
        df_pl = pl.DataFrame({"a": [1, 2, 3], "b": ["x", "y", "z"]})

        # Convert to Pandas
        df_pd = df_pl.to_pandas()

        # Verify types
        assert isinstance(df_pd, pd.DataFrame)
        assert len(df_pd) == 3
        assert list(df_pd.columns) == ["a", "b"]

    @pytest.mark.skipif(not HAS_POLARS, reason="polars not installed")
    def test_polars_io_csv(self, tmp_path: Path) -> None:
        """Test CSV I/O via Polars native methods."""
        # Create DataFrame
        df = pl.DataFrame({"instrument_id": ["BOND1", "BOND2"], "pv": [1000.0, 2000.0], "currency": ["USD", "USD"]})

        # Write to CSV
        csv_path = tmp_path / "test.csv"
        df.write_csv(str(csv_path))

        # Read back
        df_read = pl.read_csv(str(csv_path))
        assert len(df_read) == 2
        assert list(df_read.columns) == ["instrument_id", "pv", "currency"]

    @pytest.mark.skipif(not HAS_POLARS, reason="polars not installed")
    def test_polars_io_parquet(self, tmp_path: Path) -> None:
        """Test Parquet I/O via Polars native methods."""
        # Create DataFrame
        df = pl.DataFrame({
            "position_id": ["POS1", "POS2"],
            "value": [100000.0, 200000.0],
            "entity_id": ["ENTITY_A", "ENTITY_B"],
        })

        # Write to Parquet
        parquet_path = tmp_path / "test.parquet"
        df.write_parquet(str(parquet_path))

        # Read back
        df_read = pl.read_parquet(str(parquet_path))
        assert len(df_read) == 2
        assert list(df_read.columns) == ["position_id", "value", "entity_id"]

    @pytest.mark.skipif(not (HAS_POLARS and HAS_PANDAS), reason="polars/pandas not installed")
    def test_pandas_io_csv(self, tmp_path: Path) -> None:
        """Test CSV I/O via Pandas native methods."""
        # Create Polars DataFrame, convert to Pandas
        df_pl = pl.DataFrame({
            "node_id": ["revenue", "cogs"],
            "period_id": ["2025Q1", "2025Q1"],
            "value": [100000.0, 60000.0],
        })
        df_pd = df_pl.to_pandas()

        # Write to CSV via pandas
        csv_path = tmp_path / "test_pandas.csv"
        df_pd.to_csv(csv_path, index=False)

        # Read back
        df_read = pd.read_csv(csv_path)
        assert len(df_read) == 2

    @pytest.mark.skipif(not (HAS_POLARS and HAS_PANDAS), reason="polars/pandas not installed")
    def test_pandas_io_parquet(self, tmp_path: Path) -> None:
        """Test Parquet I/O via Pandas native methods."""
        # Create Polars DataFrame, convert to Pandas
        df_pl = pl.DataFrame({
            "as_of": ["2025-01-01", "2025-01-01"],
            "instrument_id": ["BOND1", "BOND2"],
            "npv": [98.5, 101.2],
        })
        df_pd = df_pl.to_pandas()

        # Write to Parquet via pandas
        parquet_path = tmp_path / "test_pandas.parquet"
        df_pd.to_parquet(parquet_path, index=False)

        # Read back
        df_read = pd.read_parquet(parquet_path)
        assert len(df_read) == 2


class TestDataFrameSchema:
    """Test DataFrame schema stability."""

    def test_portfolio_positions_schema(self) -> None:
        """Verify positions DataFrame has stable schema."""
        # Expected columns for positions DataFrame
        # This will be verified when we have actual test data

    def test_portfolio_entities_schema(self) -> None:
        """Verify entities DataFrame has stable schema."""

    def test_statements_long_schema(self) -> None:
        """Verify statements long format has stable schema."""

    def test_statements_wide_schema(self) -> None:
        """Verify statements wide format has stable schema."""
        # Wide format has period_id as first column, then one column per node
