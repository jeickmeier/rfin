"""DataFrame I/O Guide for Finstack Python Bindings.
================================================

This guide demonstrates how to export finstack results to DataFrames and
use native polars/pandas I/O methods for CSV, Parquet, Excel, SQL, etc.

Finstack provides `to_polars()` methods on all result types. For pandas conversion,
simply call `.to_pandas()` on the Polars DataFrame from Python.

Key Principles:
1. All result types expose `to_polars()` instance methods
2. Use Polars' native `.to_pandas()` for pandas conversion
3. Use native polars/pandas I/O methods (no custom CSV/Parquet wrappers)
4. Polars provides richer I/O capabilities (Excel, SQL, JSON, Arrow, etc.)
"""

import sys

try:
    import pandas as pd
    import polars as pl
except ImportError:
    print("Please install polars and pandas: pip install polars pandas")
    sys.exit(1)


# =============================================================================
# 1. VALUATION RESULTS - Single Result to DataFrame
# =============================================================================


def example_valuation_single_result():
    """Export single ValuationResult to DataFrame."""
    # Note: This is a conceptual example. Real usage requires actual pricing.

    # result = price_bond(...)  # Hypothetical
    #
    # # Export to Polars
    # df_pl = result.to_polars()
    # print("Polars DataFrame:")
    # print(df_pl)
    #
    # # Convert to Pandas
    # df_pd = df_pl.to_pandas()
    # print("\nPandas DataFrame:")
    # print(df_pd)
    #
    # # Export to CSV via Polars
    # df_pl.write_csv("bond_valuation.csv")
    #
    # # Export to Parquet via Polars
    # df_pl.write_parquet("bond_valuation.parquet")
    #
    # # Export to Excel via Pandas (requires openpyxl)
    # df_pd.to_excel("bond_valuation.xlsx", index=False)


# =============================================================================
# 2. VALUATION RESULTS - Batch Results to DataFrame
# =============================================================================


def example_valuation_batch_results():
    """Export multiple ValuationResults to DataFrame."""
    from finstack.valuations.dataframe import results_to_pandas, results_to_polars

    # results = [result1, result2, result3]  # List of ValuationResults
    #
    # # Export batch to Polars
    # df_pl = results_to_polars(results)
    # print("Batch Polars DataFrame:")
    # print(df_pl)
    # print(f"Schema: {df_pl.schema}")
    #
    # # Export batch to Pandas
    # df_pd = results_to_pandas(results)
    # print("\nBatch Pandas DataFrame:")
    # print(df_pd.dtypes)
    #
    # # I/O Examples:
    # # CSV
    # df_pl.write_csv("portfolio_valuations.csv")
    # df_read = pl.read_csv("portfolio_valuations.csv")
    #
    # # Parquet (compressed)
    # df_pl.write_parquet("portfolio_valuations.parquet", compression="zstd")
    # df_read = pl.read_parquet("portfolio_valuations.parquet")
    #
    # # JSON
    # df_pl.write_json("portfolio_valuations.json")
    # df_read = pl.read_json("portfolio_valuations.json")
    #
    # # SQL (requires database connection)
    # # df_pl.write_database("valuations_table", connection_uri)


# =============================================================================
# 3. PORTFOLIO VALUATION - Position and Entity DataFrames
# =============================================================================


def example_portfolio_valuation():
    """Export portfolio valuations to DataFrame."""
    # valuation = value_portfolio(portfolio, market, config)
    #
    # # Export position-level values to Polars
    # df_positions_pl = valuation.to_polars()
    # print("Positions DataFrame:")
    # print(df_positions_pl)
    #
    # # Export entity-level aggregates to Polars
    # df_entities_pl = valuation.entities_to_polars()
    # print("\nEntities DataFrame:")
    # print(df_entities_pl)
    #
    # # Convert to Pandas
    # df_positions_pd = df_positions_pl.to_pandas()
    # df_entities_pd = df_entities_pl.to_pandas()
    #
    # # Export both to multi-sheet Excel
    # with pd.ExcelWriter("portfolio_report.xlsx") as writer:
    #     df_positions_pd.to_excel(writer, sheet_name="Positions", index=False)
    #     df_entities_pd.to_excel(writer, sheet_name="Entities", index=False)
    #
    # # Export to Parquet (partitioned)
    # df_positions_pl.write_parquet("positions.parquet")
    # df_entities_pl.write_parquet("entities.parquet")


# =============================================================================
# 4. PORTFOLIO METRICS - Metrics DataFrame
# =============================================================================


def example_portfolio_metrics():
    """Export portfolio metrics to DataFrame."""
    from finstack.portfolio.dataframe import aggregated_metrics_to_polars, metrics_to_polars

    # metrics = aggregate_metrics(valuation, base_ccy, fx_matrix, as_of)
    #
    # # Export per-position metrics
    # df_metrics = metrics_to_polars(metrics)
    # print("Per-Position Metrics:")
    # print(df_metrics)
    #
    # # Export aggregated metrics
    # df_agg = aggregated_metrics_to_polars(metrics)
    # print("\nAggregated Metrics:")
    # print(df_agg)
    #
    # # Convert to Pandas and export
    # df_metrics.to_pandas().to_csv("position_metrics.csv", index=False)
    # df_agg.to_pandas().to_csv("portfolio_metrics.csv", index=False)


# =============================================================================
# 5. STATEMENTS RESULTS - Long and Wide Format
# =============================================================================


def example_statements_results():
    """Export statement results to DataFrame."""
    # results = evaluator.evaluate(model, market)
    #
    # # Long format (normalized, database-friendly)
    # df_long = results.to_polars_long()
    # print("Long Format:")
    # print(df_long)
    # print(f"Columns: {df_long.columns}")
    #
    # # Wide format (spreadsheet-friendly, periods as rows)
    # df_wide = results.to_polars_wide()
    # print("\nWide Format:")
    # print(df_wide)
    #
    # # Filtered long format (specific nodes only)
    # df_filtered = results.to_polars_long_filtered(["revenue", "cogs", "ebitda"])
    # print("\nFiltered Long Format:")
    # print(df_filtered)
    #
    # # Convert to Pandas
    # df_long_pd = df_long.to_pandas()
    # df_wide_pd = df_wide.to_pandas()
    #
    # # Export wide format to Excel for analysis
    # df_wide_pd.to_excel("financial_statements.xlsx", index=False)
    #
    # # Export long format to database
    # # df_long.write_database("statements_table", connection_uri)
    #
    # # Export to CSV with custom separator
    # df_long.write_csv("statements.tsv", separator="\t")


# =============================================================================
# 6. ADVANCED I/O PATTERNS
# =============================================================================


def example_advanced_io_patterns():
    """Advanced I/O patterns with polars and pandas."""
    # --- Streaming large DataFrames ---
    # Polars supports lazy evaluation for memory efficiency
    # df_large = results_to_polars(large_result_list)
    # lazy_df = df_large.lazy()
    # lazy_df.filter(pl.col("pv") > 1000).collect().write_csv("filtered.csv")

    # --- Partitioned Parquet writes ---
    # Write data partitioned by a column (e.g., entity_id, as_of_date)
    # df.write_parquet("output/", partition_by="entity_id")

    # --- SQL database integration ---
    # from sqlalchemy import create_engine
    # engine = create_engine("postgresql://user:pass@localhost/db")
    # df_pd.to_sql("valuations", engine, if_exists="append", index=False)

    # --- Arrow IPC (for cross-language compatibility) ---
    # df.write_ipc("valuations.arrow")
    # df_read = pl.read_ipc("valuations.arrow")

    # --- JSON Lines (streaming JSON) ---
    # df.write_ndjson("valuations.jsonl")
    # df_read = pl.read_ndjson("valuations.jsonl")

    # --- Excel with multiple sheets and formatting ---
    # with pd.ExcelWriter("report.xlsx", engine="openpyxl") as writer:
    #     df1_pd.to_excel(writer, sheet_name="Valuations", index=False)
    #     df2_pd.to_excel(writer, sheet_name="Metrics", index=False)
    #     # Access workbook for formatting
    #     workbook = writer.book
    #     worksheet = writer.sheets["Valuations"]
    #     # Add formatting, charts, etc.


# =============================================================================
# 7. SCHEMA VERIFICATION
# =============================================================================


def example_schema_verification():
    """Verify DataFrame schemas are stable."""
    # --- Portfolio Positions DataFrame Schema ---

    # --- Portfolio Entities DataFrame Schema ---

    # --- Statements Long Format Schema ---

    # Verify schema
    # actual_schema = df.schema
    # assert actual_schema == expected_schema


# =============================================================================
# 8. PERFORMANCE TIPS
# =============================================================================


def example_performance_tips():
    """Performance tips for DataFrame I/O."""
    # 1. Use Parquet for large datasets (columnar, compressed)
    # df.write_parquet("data.parquet", compression="zstd", compression_level=3)

    # 2. Use lazy evaluation for filtering/transformations
    # lazy_df = df.lazy()
    # result = (
    #     lazy_df
    #     .filter(pl.col("value") > 1000)
    #     .select(["instrument_id", "pv", "currency"])
    #     .collect()
    # )

    # 3. Use streaming for very large files
    # lazy_df = pl.scan_parquet("large_file.parquet")
    # result = lazy_df.filter(...).collect()

    # 4. Batch operations for multiple files
    # dfs = [results_to_polars(batch) for batch in batches]
    # combined = pl.concat(dfs)
    # combined.write_parquet("combined.parquet")

    # 5. Use Apache Arrow for zero-copy data sharing
    # arrow_table = df.to_arrow()
    # # Share with other libraries (PyArrow, DuckDB, etc.)


# =============================================================================
# MAIN - Run Examples
# =============================================================================

if __name__ == "__main__":
    print("DataFrame I/O Guide for Finstack\n")
    print("=" * 70)

    print("\n1. Single Valuation Result:")
    example_valuation_single_result()

    print("\n2. Batch Valuation Results:")
    example_valuation_batch_results()

    print("\n3. Portfolio Valuation:")
    example_portfolio_valuation()

    print("\n4. Portfolio Metrics:")
    example_portfolio_metrics()

    print("\n5. Statement Results:")
    example_statements_results()

    print("\n6. Advanced I/O Patterns:")
    example_advanced_io_patterns()

    print("\n7. Schema Verification:")
    example_schema_verification()

    print("\n8. Performance Tips:")
    example_performance_tips()

    print("\n" + "=" * 70)
    print("\nKey Takeaways:")
    print("- Use to_polars() on result objects")
    print("- Convert to pandas with df.to_pandas() from Python")
    print("- Use native polars/pandas I/O for CSV, Parquet, Excel, SQL, etc.")
    print("- Polars provides better performance for large datasets")
    print("- Schemas are stable across versions")
