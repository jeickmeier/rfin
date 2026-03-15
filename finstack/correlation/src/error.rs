//! Error types for correlation modeling.

/// Error types for correlation matrix validation.
#[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum CorrelationMatrixError {
    /// Matrix size does not match expected n×n.
    #[error("Invalid matrix size: expected {expected}×{expected}={}, got {actual}", expected * expected)]
    InvalidSize {
        /// Expected number of factors (n for n×n matrix).
        expected: usize,
        /// Actual length of the matrix array.
        actual: usize,
    },
    /// Diagonal element is not 1.
    #[error("Diagonal element [{index},{index}] = {value}, expected 1.0")]
    DiagonalNotOne {
        /// Index of the invalid diagonal element.
        index: usize,
        /// Actual value found on diagonal.
        value: f64,
    },
    /// Matrix is not symmetric.
    #[error("Matrix not symmetric: |ρ[{i},{j}] - ρ[{j},{i}]| = {diff}")]
    NotSymmetric {
        /// Row index.
        i: usize,
        /// Column index.
        j: usize,
        /// Absolute difference `|rho[i,j] - rho[j,i]|`.
        diff: f64,
    },
    /// Matrix is not positive semi-definite (Cholesky failed).
    #[error("Matrix not positive semi-definite: Cholesky failed at row {row}")]
    NotPositiveSemiDefinite {
        /// Row where Cholesky decomposition failed.
        row: usize,
    },
    /// Correlation value out of bounds [-1, 1].
    #[error("Correlation ρ[{i},{j}] = {value} out of bounds [-1, 1]")]
    OutOfBounds {
        /// Row index.
        i: usize,
        /// Column index.
        j: usize,
        /// Out-of-bounds value.
        value: f64,
    },
}
