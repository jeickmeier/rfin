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
    
    /// Build a long DataFrame from nested mapping (node->period->value)
    /// 
    /// Creates a DataFrame with columns [col1, col2, colv] where:
    /// - col1 contains the outer keys (e.g., node names)  
    /// - col2 contains the inner keys (e.g., period names)
    /// - colv contains the values
    /// 
    /// This is a placeholder implementation. The actual implementation should
    /// be provided by consumer crates that have access to the specific Polars
    /// version and can handle the DataFrame construction details.
    pub fn long_from_nested<K1, K2, V>(
        _nested: &HashMap<K1, HashMap<K2, V>>,
        _col1: &str,
        _col2: &str, 
        _colv: &str,
    ) -> crate::Result<DataFrame>
    where
        K1: AsRef<str> + Clone,
        K2: AsRef<str> + Clone,
        V: Into<f64> + Copy,
    {
        // Placeholder - to be implemented in consumer crates
        Err(crate::Error::Internal)
    }
    
    /// Build a wide DataFrame where rows are the second key and columns are the first key
    /// 
    /// The resulting DataFrame has:
    /// - Index column with name `row_key` containing the inner keys
    /// - One column per outer key containing the values
    /// 
    /// This is a placeholder implementation. The actual implementation should
    /// be provided by consumer crates that have access to the specific Polars
    /// version and can handle the DataFrame construction details.
    pub fn wide_from_nested<K1, K2, V>(
        _nested: &HashMap<K1, HashMap<K2, V>>,
        _row_key: &str,
    ) -> crate::Result<DataFrame> 
    where
        K1: AsRef<str> + Clone + std::hash::Hash + Eq,
        K2: AsRef<str> + Clone + std::hash::Hash + Eq, 
        V: Into<f64> + Copy,
    {
        // Placeholder - to be implemented in consumer crates
        Err(crate::Error::Internal)
    }
}
