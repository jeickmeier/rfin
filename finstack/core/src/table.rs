//! Serializable columnar table envelope for tabular result exports.
//!
//! This module provides a lightweight, serde-friendly alternative to returning
//! a third-party DataFrame type from Rust APIs. Callers can inspect columns
//! directly, serialize them for Python/WASM consumers, or convert them into a
//! host-language table type at the binding layer.

use crate::{Error, Result};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Serializable columnar table envelope.
///
/// Tables preserve column order and validate that every column has the same row
/// count. Optional metadata can record domain-specific hints such as which
/// column is a metric, what a numeric field represents, or how a host-language
/// binding should interpret the data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TableEnvelope {
    /// Number of rows in the table.
    pub row_count: usize,
    /// Ordered set of columns.
    pub columns: Vec<TableColumn>,
    /// Table-level metadata for downstream bindings and documentation.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub metadata: IndexMap<String, serde_json::Value>,
}

impl TableEnvelope {
    /// Construct a table from validated columns.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if columns have mismatched lengths or if
    /// duplicate column names are provided.
    pub fn new(columns: Vec<TableColumn>) -> Result<Self> {
        Self::new_with_metadata(columns, IndexMap::new())
    }

    /// Construct a table from validated columns and metadata.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if columns have mismatched lengths or if
    /// duplicate column names are provided.
    pub fn new_with_metadata(
        columns: Vec<TableColumn>,
        metadata: IndexMap<String, serde_json::Value>,
    ) -> Result<Self> {
        let row_count = columns.first().map(TableColumn::len).unwrap_or(0);
        let mut seen = crate::HashSet::default();

        for column in &columns {
            if column.len() != row_count {
                return Err(Error::Validation(format!(
                    "column '{}' has {} rows but expected {}",
                    column.name,
                    column.len(),
                    row_count
                )));
            }

            if !seen.insert(column.name.clone()) {
                return Err(Error::Validation(format!(
                    "duplicate column name '{}'",
                    column.name
                )));
            }
        }

        Ok(Self {
            row_count,
            columns,
            metadata,
        })
    }

    /// Return whether the table has no rows.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.row_count == 0
    }

    /// Look up a column by name.
    #[must_use]
    pub fn column(&self, name: &str) -> Option<&TableColumn> {
        self.columns.iter().find(|column| column.name == name)
    }
}

/// A single named column in a [`TableEnvelope`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TableColumn {
    /// Column name.
    pub name: String,
    /// Column values.
    pub data: TableColumnData,
    /// Optional semantic hint for bindings and consumers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<TableColumnRole>,
    /// Optional per-column metadata.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub metadata: IndexMap<String, serde_json::Value>,
}

impl TableColumn {
    /// Create a column with no role or metadata.
    #[must_use]
    pub fn new(name: impl Into<String>, data: TableColumnData) -> Self {
        Self {
            name: name.into(),
            data,
            role: None,
            metadata: IndexMap::new(),
        }
    }

    /// Attach a semantic role to the column.
    #[must_use]
    pub fn with_role(mut self, role: TableColumnRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Attach metadata to the column.
    #[must_use]
    pub fn with_metadata(mut self, metadata: IndexMap<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Number of rows in the column.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Return whether the column has no values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return string values if this is a non-null string column.
    #[must_use]
    pub fn as_strings(&self) -> Option<&[String]> {
        self.data.as_strings()
    }

    /// Return nullable string values if this is a nullable string column.
    #[must_use]
    pub fn as_nullable_strings(&self) -> Option<&[Option<String>]> {
        self.data.as_nullable_strings()
    }

    /// Return floating-point values if this is a non-null float column.
    #[must_use]
    pub fn as_f64(&self) -> Option<&[f64]> {
        self.data.as_f64()
    }

    /// Return nullable floating-point values if this is a nullable float column.
    #[must_use]
    pub fn as_nullable_f64(&self) -> Option<&[Option<f64>]> {
        self.data.as_nullable_f64()
    }

    /// Return unsigned integer values if this is a non-null `u32` column.
    #[must_use]
    pub fn as_u32(&self) -> Option<&[u32]> {
        self.data.as_u32()
    }

    /// Return nullable unsigned integer values if this is a nullable `u32` column.
    #[must_use]
    pub fn as_nullable_u32(&self) -> Option<&[Option<u32>]> {
        self.data.as_nullable_u32()
    }

    /// Return signed integer values if this is a non-null `i64` column.
    #[must_use]
    pub fn as_i64(&self) -> Option<&[i64]> {
        self.data.as_i64()
    }

    /// Return nullable signed integer values if this is a nullable `i64` column.
    #[must_use]
    pub fn as_nullable_i64(&self) -> Option<&[Option<i64>]> {
        self.data.as_nullable_i64()
    }
}

/// Column storage variants supported by the table envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "values", rename_all = "snake_case")]
pub enum TableColumnData {
    /// Non-null string column.
    String(Vec<String>),
    /// Nullable string column.
    NullableString(Vec<Option<String>>),
    /// Non-null floating-point column.
    Float64(Vec<f64>),
    /// Nullable floating-point column.
    NullableFloat64(Vec<Option<f64>>),
    /// Non-null unsigned 32-bit integer column.
    UInt32(Vec<u32>),
    /// Nullable unsigned 32-bit integer column.
    NullableUInt32(Vec<Option<u32>>),
    /// Non-null signed 64-bit integer column.
    Int64(Vec<i64>),
    /// Nullable signed 64-bit integer column.
    NullableInt64(Vec<Option<i64>>),
}

impl TableColumnData {
    /// Number of rows in the column.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::String(values) => values.len(),
            Self::NullableString(values) => values.len(),
            Self::Float64(values) => values.len(),
            Self::NullableFloat64(values) => values.len(),
            Self::UInt32(values) => values.len(),
            Self::NullableUInt32(values) => values.len(),
            Self::Int64(values) => values.len(),
            Self::NullableInt64(values) => values.len(),
        }
    }

    /// Return whether the column stores no values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return string values if this is a non-null string column.
    #[must_use]
    pub fn as_strings(&self) -> Option<&[String]> {
        match self {
            Self::String(values) => Some(values),
            _ => None,
        }
    }

    /// Return nullable string values if this is a nullable string column.
    #[must_use]
    pub fn as_nullable_strings(&self) -> Option<&[Option<String>]> {
        match self {
            Self::NullableString(values) => Some(values),
            _ => None,
        }
    }

    /// Return floating-point values if this is a non-null float column.
    #[must_use]
    pub fn as_f64(&self) -> Option<&[f64]> {
        match self {
            Self::Float64(values) => Some(values),
            _ => None,
        }
    }

    /// Return nullable floating-point values if this is a nullable float column.
    #[must_use]
    pub fn as_nullable_f64(&self) -> Option<&[Option<f64>]> {
        match self {
            Self::NullableFloat64(values) => Some(values),
            _ => None,
        }
    }

    /// Return unsigned integer values if this is a non-null `u32` column.
    #[must_use]
    pub fn as_u32(&self) -> Option<&[u32]> {
        match self {
            Self::UInt32(values) => Some(values),
            _ => None,
        }
    }

    /// Return nullable unsigned integer values if this is a nullable `u32` column.
    #[must_use]
    pub fn as_nullable_u32(&self) -> Option<&[Option<u32>]> {
        match self {
            Self::NullableUInt32(values) => Some(values),
            _ => None,
        }
    }

    /// Return signed integer values if this is a non-null `i64` column.
    #[must_use]
    pub fn as_i64(&self) -> Option<&[i64]> {
        match self {
            Self::Int64(values) => Some(values),
            _ => None,
        }
    }

    /// Return nullable signed integer values if this is a nullable `i64` column.
    #[must_use]
    pub fn as_nullable_i64(&self) -> Option<&[Option<i64>]> {
        match self {
            Self::NullableInt64(values) => Some(values),
            _ => None,
        }
    }
}

/// Optional semantic hint for a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TableColumnRole {
    /// Row identifier or primary grouping field.
    Dimension,
    /// Ordinal or time-like axis column.
    Index,
    /// Numeric value column.
    Measure,
    /// Auxiliary classification column.
    Attribute,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn table_validates_column_lengths() {
        let result = TableEnvelope::new(vec![
            TableColumn::new("a", TableColumnData::String(vec!["x".into(), "y".into()])),
            TableColumn::new("b", TableColumnData::Float64(vec![1.0])),
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn table_rejects_duplicate_column_names() {
        let result = TableEnvelope::new(vec![
            TableColumn::new("a", TableColumnData::String(vec!["x".into()])),
            TableColumn::new("a", TableColumnData::Float64(vec![1.0])),
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn table_round_trips_with_metadata() {
        let column = TableColumn::new(
            "period_id",
            TableColumnData::String(vec!["2025Q1".into(), "2025Q2".into()]),
        )
        .with_role(TableColumnRole::Index);

        let mut metadata = IndexMap::new();
        metadata.insert("orientation".to_string(), json!("long"));

        let table = TableEnvelope::new_with_metadata(vec![column], metadata).expect("valid table");
        let json = serde_json::to_string(&table).expect("serializes");
        let restored: TableEnvelope = serde_json::from_str(&json).expect("deserializes");

        assert_eq!(restored.row_count, 2);
        assert_eq!(restored.metadata.get("orientation"), Some(&json!("long")));
        assert_eq!(restored.column("period_id").map(TableColumn::len), Some(2));
    }
}
