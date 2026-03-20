//! DataFrame export functionality for Results.

#[cfg(feature = "dataframes")]
use super::StatementResult;
#[cfg(feature = "dataframes")]
use crate::error::Result;
#[cfg(feature = "dataframes")]
use finstack_core::dates::PeriodId;
#[cfg(feature = "dataframes")]
use polars::prelude::*;

/// Export results to long-format Polars DataFrame.
///
/// Schema: `(node_id, period_id, value, value_money, currency, value_type)`
///
/// Includes both f64 and Money representations where applicable.
/// Requires the `dataframes` feature.
///
/// # Arguments
///
/// * `results` - Evaluation output to serialize
///
/// # Returns
///
/// Returns one row per `(node_id, period_id)` pair. Monetary nodes populate
/// both `value` and `value_money`; scalar nodes leave `value_money` and
/// `currency` null.
///
/// # Example
///
/// ```text
/// let df = to_polars_long(&results)?;
/// // Output:
/// // ┌─────────────┬───────────┬────────────┬──────────────┬──────────┬────────────┐
/// // │ node_id     │ period_id │ value      │ value_money  │ currency │ value_type │
/// // ├─────────────┼───────────┼────────────┼──────────────┼──────────┼────────────┤
/// // │ revenue     │ 2025Q1    │ 100000.0   │ 100000.0     │ USD      │ monetary   │
/// // │ revenue     │ 2025Q2    │ 105000.0   │ 105000.0     │ USD      │ monetary   │
/// // │ margin_pct  │ 2025Q1    │ 0.35       │ null         │ null     │ scalar     │
/// // └─────────────┴───────────┴────────────┴──────────────┴──────────┴────────────┘
/// ```
#[cfg(feature = "dataframes")]
pub fn to_polars_long(results: &StatementResult) -> Result<DataFrame> {
    use crate::types::NodeValueType;

    let mut node_ids = Vec::new();
    let mut period_ids = Vec::new();
    let mut values = Vec::new();
    let mut money_values = Vec::new();
    let mut currencies = Vec::new();
    let mut value_types = Vec::new();

    for (node_id, period_map) in &results.nodes {
        let node_value_type = results.node_value_types.get(node_id);

        for (period_id, value) in period_map {
            node_ids.push(node_id.as_str());
            period_ids.push(period_id.to_string());
            values.push(*value);

            // Add Money-specific columns if this is a monetary node
            if let Some(NodeValueType::Monetary { currency }) = node_value_type {
                money_values.push(Some(*value));
                currencies.push(Some(currency.to_string()));
                value_types.push("monetary");
            } else {
                money_values.push(None);
                currencies.push(None);
                value_types.push("scalar");
            }
        }
    }

    let df = DataFrame::new_infer_height(vec![
        Series::new("node_id".into(), node_ids).into(),
        Series::new("period_id".into(), period_ids).into(),
        Series::new("value".into(), values).into(),
        Series::new("value_money".into(), money_values).into(),
        Series::new("currency".into(), currencies).into(),
        Series::new("value_type".into(), value_types).into(),
    ])
    .map_err(|e| {
        crate::error::Error::invalid_input(format!("Failed to create DataFrame: {}", e))
    })?;

    Ok(df)
}

/// Export results to long-format Polars DataFrame with node filtering.
///
/// Schema: `(node_id, period_id, value, value_money, currency, value_type)`
/// Requires the `dataframes` feature.
///
/// # Arguments
///
/// * `results` - The results to export
/// * `node_filter` - List of node IDs to include (if empty, includes all)
///
/// # Returns
///
/// Returns the same schema as [`to_polars_long`] after filtering rows to the
/// requested node ids.
///
/// # Example
///
/// ```text
/// let df = to_polars_long_filtered(&results, &["revenue", "cogs"])?;
/// ```
#[cfg(feature = "dataframes")]
pub fn to_polars_long_filtered(
    results: &StatementResult,
    node_filter: &[&str],
) -> Result<DataFrame> {
    use crate::types::NodeValueType;

    let mut node_ids = Vec::new();
    let mut period_ids = Vec::new();
    let mut values = Vec::new();
    let mut money_values = Vec::new();
    let mut currencies = Vec::new();
    let mut value_types = Vec::new();

    for (node_id, period_map) in &results.nodes {
        // Skip if filter is specified and node not in filter
        if !node_filter.is_empty() && !node_filter.contains(&node_id.as_str()) {
            continue;
        }

        let node_value_type = results.node_value_types.get(node_id);

        for (period_id, value) in period_map {
            node_ids.push(node_id.as_str());
            period_ids.push(period_id.to_string());
            values.push(*value);

            // Add Money-specific columns if this is a monetary node
            if let Some(NodeValueType::Monetary { currency }) = node_value_type {
                money_values.push(Some(*value));
                currencies.push(Some(currency.to_string()));
                value_types.push("monetary");
            } else {
                money_values.push(None);
                currencies.push(None);
                value_types.push("scalar");
            }
        }
    }

    let df = DataFrame::new_infer_height(vec![
        Series::new("node_id".into(), node_ids).into(),
        Series::new("period_id".into(), period_ids).into(),
        Series::new("value".into(), values).into(),
        Series::new("value_money".into(), money_values).into(),
        Series::new("currency".into(), currencies).into(),
        Series::new("value_type".into(), value_types).into(),
    ])
    .map_err(|e| {
        crate::error::Error::invalid_input(format!("Failed to create DataFrame: {}", e))
    })?;

    Ok(df)
}

/// Export results to wide-format Polars DataFrame.
///
/// Schema: periods as rows, nodes as columns
/// Requires the `dataframes` feature.
///
/// # Arguments
///
/// * `results` - Evaluation output to serialize
///
/// # Returns
///
/// Returns a DataFrame with one row per unique period and one column per node.
/// Missing `(node, period)` values are encoded as `NaN`.
///
/// # Example
///
/// ```text
/// let df = to_polars_wide(&results)?;
/// // Output:
/// // ┌───────────┬────────────┬──────────┐
/// // │ period_id │ revenue    │ cogs     │
/// // ├───────────┼────────────┼──────────┤
/// // │ 2025Q1    │ 100000.0   │ 60000.0  │
/// // │ 2025Q2    │ 105000.0   │ 63000.0  │
/// // └───────────┴────────────┴──────────┘
/// ```
#[cfg(feature = "dataframes")]
pub fn to_polars_wide(results: &StatementResult) -> Result<DataFrame> {
    // Collect all unique periods in order
    let mut all_periods: Vec<PeriodId> = results
        .nodes
        .values()
        .flat_map(|period_map| period_map.keys().cloned())
        .collect();
    all_periods.sort();
    all_periods.dedup();

    if all_periods.is_empty() {
        return DataFrame::new_infer_height(vec![Series::new(
            "period_id".into(),
            Vec::<String>::new(),
        )
        .into()])
        .map_err(|e| {
            crate::error::Error::invalid_input(format!("Failed to create empty DataFrame: {}", e))
        });
    }

    // Start with period_id column
    let period_strings: Vec<String> = all_periods.iter().map(|p| p.to_string()).collect();
    let mut series_list = vec![Series::new("period_id".into(), period_strings).into()];

    // Add a column for each node
    for (node_id, period_map) in &results.nodes {
        let mut node_values = Vec::new();

        for period_id in &all_periods {
            let value = period_map.get(period_id).copied().unwrap_or(f64::NAN);
            node_values.push(value);
        }

        series_list.push(Series::new(node_id.as_str().into(), node_values).into());
    }

    let df = DataFrame::new_infer_height(series_list).map_err(|e| {
        crate::error::Error::invalid_input(format!("Failed to create DataFrame: {}", e))
    })?;

    Ok(df)
}

#[cfg(all(test, feature = "dataframes"))]
#[allow(clippy::expect_used)]
mod tests {
    use super::super::ResultsMeta;
    use super::*;
    use finstack_core::dates::PeriodId;
    use indexmap::IndexMap;

    fn create_test_results() -> StatementResult {
        let mut nodes = IndexMap::new();

        // Revenue
        let mut revenue_periods = IndexMap::new();
        revenue_periods.insert(PeriodId::quarter(2025, 1), 100_000.0);
        revenue_periods.insert(PeriodId::quarter(2025, 2), 110_000.0);
        nodes.insert("revenue".to_string(), revenue_periods);

        // COGS
        let mut cogs_periods = IndexMap::new();
        cogs_periods.insert(PeriodId::quarter(2025, 1), 60_000.0);
        cogs_periods.insert(PeriodId::quarter(2025, 2), 66_000.0);
        nodes.insert("cogs".to_string(), cogs_periods);

        // Gross profit
        let mut gp_periods = IndexMap::new();
        gp_periods.insert(PeriodId::quarter(2025, 1), 40_000.0);
        gp_periods.insert(PeriodId::quarter(2025, 2), 44_000.0);
        nodes.insert("gross_profit".to_string(), gp_periods);

        StatementResult {
            nodes,
            monetary_nodes: IndexMap::new(),
            node_value_types: IndexMap::new(),
            cs_cashflows: None,
            meta: ResultsMeta::default(),
        }
    }

    #[test]
    fn test_to_polars_long() {
        let results = create_test_results();
        let df = to_polars_long(&results).expect("should convert to polars DataFrame");

        assert_eq!(df.height(), 6); // 3 nodes × 2 periods
        assert_eq!(df.width(), 6); // node_id, period_id, value, value_money, currency, value_type

        // Check column names
        let columns = df.get_column_names();
        assert_eq!(columns.len(), 6);
        assert_eq!(columns[0].as_str(), "node_id");
        assert_eq!(columns[1].as_str(), "period_id");
        assert_eq!(columns[2].as_str(), "value");
        assert_eq!(columns[3].as_str(), "value_money");
        assert_eq!(columns[4].as_str(), "currency");
        assert_eq!(columns[5].as_str(), "value_type");

        // Check first row
        let node_ids = df
            .column("node_id")
            .expect("node_id column should exist")
            .str()
            .expect("node_id should be string type");
        assert_eq!(node_ids.get(0), Some("revenue"));

        let values = df
            .column("value")
            .expect("value column should exist")
            .f64()
            .expect("value should be f64 type");
        assert_eq!(values.get(0), Some(100_000.0));
    }

    #[test]
    fn test_to_polars_long_filtered() {
        let results = create_test_results();
        let df = to_polars_long_filtered(&results, &["revenue", "cogs"])
            .expect("should convert to polars DataFrame");

        assert_eq!(df.height(), 4); // 2 nodes × 2 periods
        assert_eq!(df.width(), 6); // Updated for new columns

        // Check that only revenue and cogs are included
        let node_ids = df
            .column("node_id")
            .expect("node_id column should exist")
            .str()
            .expect("node_id should be string type");
        let unique_nodes: std::collections::HashSet<String> = node_ids
            .into_iter()
            .flatten()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(unique_nodes.len(), 2);
        assert!(unique_nodes.contains("revenue"));
        assert!(unique_nodes.contains("cogs"));
    }

    #[test]
    fn test_to_polars_long_filtered_empty_includes_all() {
        let results = create_test_results();
        let df =
            to_polars_long_filtered(&results, &[]).expect("should convert to polars DataFrame");

        assert_eq!(df.height(), 6); // All 3 nodes × 2 periods
    }

    #[test]
    fn test_to_polars_wide() {
        let results = create_test_results();
        let df = to_polars_wide(&results).expect("should convert to polars DataFrame");

        assert_eq!(df.height(), 2); // 2 periods
        assert_eq!(df.width(), 4); // period_id + 3 nodes

        // Check column names
        let columns = df.get_column_names();
        let col_names: Vec<String> = columns.iter().map(|c| c.as_str().to_string()).collect();
        assert!(col_names.contains(&"period_id".to_string()));
        assert!(col_names.contains(&"revenue".to_string()));
        assert!(col_names.contains(&"cogs".to_string()));
        assert!(col_names.contains(&"gross_profit".to_string()));

        // Check first row values
        let revenue = df
            .column("revenue")
            .expect("revenue column should exist")
            .f64()
            .expect("revenue should be f64 type");
        assert_eq!(revenue.get(0), Some(100_000.0));

        let cogs = df
            .column("cogs")
            .expect("cogs column should exist")
            .f64()
            .expect("cogs should be f64 type");
        assert_eq!(cogs.get(0), Some(60_000.0));
    }

    #[test]
    fn test_to_polars_wide_period_order() {
        let results = create_test_results();
        let df = to_polars_wide(&results).expect("should convert to polars DataFrame");

        let period_ids = df
            .column("period_id")
            .expect("period_id column should exist")
            .str()
            .expect("period_id should be string type");
        assert_eq!(period_ids.get(0), Some("2025Q1"));
        assert_eq!(period_ids.get(1), Some("2025Q2"));
    }

    #[test]
    fn test_empty_results() {
        let results = StatementResult {
            nodes: IndexMap::new(),
            monetary_nodes: IndexMap::new(),
            node_value_types: IndexMap::new(),
            cs_cashflows: None,
            meta: ResultsMeta::default(),
        };

        let df_long = to_polars_long(&results).expect("test should succeed");
        assert_eq!(df_long.height(), 0);

        let df_wide = to_polars_wide(&results).expect("test should succeed");
        assert_eq!(df_wide.height(), 0);
    }
}
