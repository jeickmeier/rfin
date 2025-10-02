//! Error types for the statements crate.

use thiserror::Error;

/// Result type alias for statements operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Comprehensive error type for statements operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Model building error (e.g., invalid period range, missing periods)
    #[error("Model build error: {0}")]
    Build(String),

    /// Formula parsing error
    #[error("Formula parse error: {0}")]
    FormulaParse(String),

    /// Evaluation error (e.g., circular dependency, undefined node)
    #[error("Evaluation error: {0}")]
    Eval(String),

    /// Node not found in model
    #[error("Node not found: {node_id}")]
    NodeNotFound {
        /// ID of the node that was not found
        node_id: String,
    },

    /// Circular dependency detected
    #[error("Circular dependency detected: {}", format_path(.path))]
    CircularDependency {
        /// Path through the circular dependency
        path: Vec<String>,
    },

    /// Currency mismatch error
    #[error("Currency mismatch: expected {expected}, found {found}")]
    CurrencyMismatch {
        /// Expected currency
        expected: finstack_core::currency::Currency,
        /// Found currency
        found: finstack_core::currency::Currency,
    },

    /// Period validation error
    #[error("Period error: {0}")]
    Period(String),

    /// Missing required data
    #[error("Missing required data: {0}")]
    MissingData(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Forecast method error
    #[error("Forecast error: {0}")]
    Forecast(String),

    /// Registry error
    #[error("Registry error: {0}")]
    Registry(String),

    /// Capital structure error (feature-gated)
    #[cfg(feature = "capital_structure")]
    #[error("Capital structure error: {0}")]
    CapitalStructure(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Core crate error
    #[error("Core error: {0}")]
    Core(#[from] finstack_core::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// Helper function to format circular dependency path
fn format_path(path: &[String]) -> String {
    path.join(" → ")
}

impl Error {
    /// Create a build error with context
    pub fn build(msg: impl Into<String>) -> Self {
        Self::Build(msg.into())
    }

    /// Create an evaluation error with context
    pub fn eval(msg: impl Into<String>) -> Self {
        Self::Eval(msg.into())
    }

    /// Create a formula parse error
    pub fn formula_parse(msg: impl Into<String>) -> Self {
        Self::FormulaParse(msg.into())
    }

    /// Create a period error
    pub fn period(msg: impl Into<String>) -> Self {
        Self::Period(msg.into())
    }

    /// Create a missing data error
    pub fn missing_data(msg: impl Into<String>) -> Self {
        Self::MissingData(msg.into())
    }

    /// Create an invalid input error
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }
}
