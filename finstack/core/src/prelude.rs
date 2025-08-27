//! finstack-core prelude: commonly used items + Polars re-exports
//!
//! Import this module to get a convenient set of core types and Polars
//! helpers in scope without pulling `polars` directly at call sites.
//! Downstream crates should prefer `use finstack_core::prelude::*;`.

// Core re-exports (keep list focused on ergonomic entry points)  
pub use crate::currency::Currency;
pub use crate::types::{
    Id, TypeTag, Rate, Bps, Percentage, 
    CurveId, PositionId, TradeId, PortfolioId, ScenarioId, InstrumentId, CounterpartyId,
    Amount, Timestamp,
};
pub use crate::dates::{
    adjust, available_calendars, next_cds_date, next_imm, third_wednesday, BusinessDayConvention,
    Date, DateExt, DayCount, HolidayCalendar, MergeMode, OffsetDateTime, OffsetDateTimeExt, Period,
    PeriodId, PeriodKey, ScheduleBuilder, StubKind, build_periods,
    IndexId, IndexInterpolation, IndexLag, IndexSeries, SeasonalityPolicy,
};
pub use crate::error::{Error, InputError};
// Expression engine - only re-export public items
pub use crate::expr::{CompiledExpr};
pub use crate::market_data::{
    interp::{InterpFn, InterpStyle},
    traits::{Discount, Forward, Inflation as InflationTs, Surface, Survival, TermStructure},
};
pub use crate::money::{
    fx::{
        FxProvider, FxMatrix, FxRate, FxConversionPolicy, FxPolicyMeta,
        FxCacheConfig, ClosureCheckResult,
    },
    Money,
};
pub use crate::Result;

// Validation framework
pub use crate::validation::{
    Validator, ValidationResult, ValidationStatus, ValidationWarning,
    ValidatorExt, RangeValidator, LengthValidator,
};

// Math utilities  
pub use crate::math::{
    root_finding::{brent, newton_bracketed},
    summation::{kahan_sum, pairwise_sum, stable_sum},
    stats::{mean, variance, mean_var, covariance, correlation},
};

// Configuration and policy management
pub use crate::config::{
    FinstackConfig, RoundingMode, RoundingPolicy, RoundingContext, NumericMode, ResultsMeta,
    config, with_temp_config, results_meta, output_scale_for, ingest_scale_for,
};

// Re-export Polars with an alias to avoid `Expr` name collision and to keep a
// consistent surface across the workspace.
pub use polars::prelude::{DataFrame, Series, DataType as PolarsDataType, AnyValue};
pub use polars::lazy::prelude::{
    col, lit, when,
    Expr as PolarsExpr, LazyFrame,
    // Common aggregation functions  
    sum as polars_sum, mean as polars_mean, median as polars_median,
    min as polars_min, max as polars_max,
};

/// Helper functions for constructing DataFrames from nested data structures
/// 
/// These functions provide convenient ways to convert common data structures
/// into Polars DataFrames for use in statements, valuations, and other crates.
pub mod df {
    use super::*;
    use std::collections::HashMap;
    use polars::prelude::NamedFrom;
    
    /// Build a long DataFrame from nested mapping (node->period->value)
    /// 
    /// Creates a DataFrame with columns [col1, col2, colv] where:
    /// - col1 contains the outer keys (e.g., node names)  
    /// - col2 contains the inner keys (e.g., period names)
    /// - colv contains the values
    pub fn long_from_nested<K1, K2, V>(
        nested: &HashMap<K1, HashMap<K2, V>>,
        col1: &str,
        col2: &str, 
        colv: &str,
    ) -> crate::Result<DataFrame>
    where
        K1: AsRef<str> + Clone,
        K2: AsRef<str> + Clone,
        V: Into<f64> + Copy,
    {
        let mut outer_keys = Vec::new();
        let mut inner_keys = Vec::new();
        let mut values = Vec::new();

        for (outer_key, inner_map) in nested {
            for (inner_key, value) in inner_map {
                outer_keys.push(outer_key.as_ref().to_string());
                inner_keys.push(inner_key.as_ref().to_string());
                values.push((*value).into());
            }
        }

        // Create the DataFrame using Polars
        let df = DataFrame::new(vec![
            Series::new(col1.into(), outer_keys).into(),
            Series::new(col2.into(), inner_keys).into(), 
            Series::new(colv.into(), values).into(),
        ]).map_err(|_| crate::Error::Internal)?;

        Ok(df)
    }
    
    /// Build a wide DataFrame where rows are the second key and columns are the first key
    /// 
    /// The resulting DataFrame has:
    /// - Index column with name `row_key` containing the inner keys
    /// - One column per outer key containing the values
    pub fn wide_from_nested<K1, K2, V>(
        nested: &HashMap<K1, HashMap<K2, V>>,
        row_key: &str,
    ) -> crate::Result<DataFrame> 
    where
        K1: AsRef<str> + Clone + std::hash::Hash + Eq,
        K2: AsRef<str> + Clone + std::hash::Hash + Eq, 
        V: Into<f64> + Copy,
    {
        // Collect all unique inner keys (rows) and outer keys (columns)  
        let mut inner_keys_set = std::collections::HashSet::new();
        let mut outer_keys: Vec<&K1> = Vec::new();

        for (outer_key, inner_map) in nested {
            outer_keys.push(outer_key);
            for inner_key in inner_map.keys() {
                inner_keys_set.insert(inner_key.as_ref());
            }
        }

        // Convert inner keys to sorted vector for consistent ordering
        let mut inner_keys: Vec<_> = inner_keys_set.into_iter().collect();
        inner_keys.sort();
        outer_keys.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));

        // Start with the row key column
        let mut series_vec = vec![Series::new(row_key.into(), inner_keys.iter().map(|k| k.to_string()).collect::<Vec<_>>()).into()];

        // Create one column per outer key
        for outer_key in &outer_keys {
            let mut column_values = Vec::new();
            
            if let Some(inner_map) = nested.get(outer_key) {
                for inner_key in &inner_keys {
                    let value = inner_map.iter()
                        .find(|(k, _)| k.as_ref() == *inner_key)
                        .map(|(_, v)| (*v).into())
                        .unwrap_or(f64::NAN);
                    column_values.push(value);
                }
            } else {
                column_values.resize(inner_keys.len(), f64::NAN);
            }
            
            series_vec.push(Series::new(outer_key.as_ref().into(), column_values).into());
        }

        // Create the DataFrame using Polars
        let df = DataFrame::new(series_vec).map_err(|_| crate::Error::Internal)?;

        Ok(df)
    }
}
