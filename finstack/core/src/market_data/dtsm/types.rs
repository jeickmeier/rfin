//! Shared types for dynamic term structure models.
//!
//! Provides the canonical data containers used by all DTSM estimators:
//! yield panel data, factor time series, and forecast results.

use nalgebra::DMatrix;
use serde::{Deserialize, Serialize};

fn rows_to_dmatrix(rows: &[Vec<f64>], label: &str) -> crate::Result<DMatrix<f64>> {
    if rows.is_empty() {
        return Err(crate::Error::Validation(format!(
            "{label} must not be empty"
        )));
    }

    let nrows = rows.len();
    let ncols = rows[0].len();
    for (i, row) in rows.iter().enumerate() {
        if row.len() != ncols {
            return Err(crate::Error::Validation(format!(
                "{label}: row {i} has length {} but expected {ncols} (first row)",
                row.len()
            )));
        }
    }

    let mut matrix = DMatrix::zeros(nrows, ncols);
    for (i, row) in rows.iter().enumerate() {
        for (j, &value) in row.iter().enumerate() {
            matrix[(i, j)] = value;
        }
    }
    Ok(matrix)
}

// ---------------------------------------------------------------------------
// YieldPanel
// ---------------------------------------------------------------------------

/// A panel of yield observations: rows = dates, columns = tenors.
///
/// This is the canonical input format for all DTSM estimators.
/// Yields are continuously compounded zero rates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldPanel {
    /// Yield matrix: T rows (dates) x N columns (tenors).
    /// Entry (t, i) is the zero rate at observation t for tenor i.
    pub yields: DMatrix<f64>,
    /// Tenor grid in years, length N. Must be sorted ascending, all > 0.
    pub tenors: Vec<f64>,
    /// Observation dates (optional, for labeling). Length T if provided.
    pub dates: Option<Vec<crate::dates::Date>>,
}

impl YieldPanel {
    /// Construct a yield panel from row-major yield observations.
    ///
    /// `yield_rows[date_idx][tenor_idx]` is converted into the canonical
    /// matrix representation and validated through [`Self::new`].
    ///
    /// # Errors
    /// - Yield rows are empty or ragged
    /// - Tenor grid and yield-row width do not match
    /// - Any invariant enforced by [`Self::new`] fails
    pub fn from_rows(
        tenors: Vec<f64>,
        yield_rows: Vec<Vec<f64>>,
        dates: Option<Vec<crate::dates::Date>>,
    ) -> crate::Result<Self> {
        let yields = rows_to_dmatrix(&yield_rows, "yield_rows")?;
        Self::new(yields, tenors, dates)
    }

    /// Reconstruct a pseudo-panel from row-major yield changes.
    ///
    /// PCA depends only on first differences, so this helper integrates the
    /// supplied changes from an arbitrary zero base and assigns a synthetic
    /// strictly ascending tenor grid. It is intended for callers that already
    /// have differenced yield data.
    ///
    /// # Errors
    /// - Yield-change rows are empty or ragged
    /// - The reconstructed panel violates [`Self::new`] invariants
    pub fn from_yield_changes(yield_changes: Vec<Vec<f64>>) -> crate::Result<Self> {
        let changes = rows_to_dmatrix(&yield_changes, "yield_changes")?;
        let n = changes.ncols();
        let m = changes.nrows();

        let mut levels = DMatrix::zeros(m + 1, n);
        for i in 0..m {
            for j in 0..n {
                levels[(i + 1, j)] = levels[(i, j)] + changes[(i, j)];
            }
        }

        let tenors: Vec<f64> = (1..=n).map(|i| i as f64).collect();
        Self::new(levels, tenors, None)
    }

    /// Construct and validate a yield panel.
    ///
    /// # Errors
    /// - Tenor grid not sorted ascending or contains non-positive values
    /// - Yield matrix column count does not match tenor grid length
    /// - Fewer than 2 observations (rows)
    /// - Any yield value is non-finite
    pub fn new(
        yields: DMatrix<f64>,
        tenors: Vec<f64>,
        dates: Option<Vec<crate::dates::Date>>,
    ) -> crate::Result<Self> {
        // Validate tenor grid
        if tenors.is_empty() {
            return Err(crate::Error::Validation(
                "Tenor grid must not be empty".into(),
            ));
        }
        for (i, tau) in tenors.iter().enumerate() {
            if !tau.is_finite() || *tau <= 0.0 {
                return Err(crate::Error::Validation(format!(
                    "Tenor at index {i} must be positive and finite, got {tau}"
                )));
            }
            if i > 0 && tenors[i] <= tenors[i - 1] {
                return Err(crate::Error::Validation(format!(
                    "Tenor grid must be strictly ascending: tenor[{}]={} <= tenor[{}]={}",
                    i,
                    tenors[i],
                    i - 1,
                    tenors[i - 1]
                )));
            }
        }

        // Validate matrix dimensions
        if yields.ncols() != tenors.len() {
            return Err(crate::Error::Validation(format!(
                "Yield matrix has {} columns but tenor grid has {} entries",
                yields.ncols(),
                tenors.len()
            )));
        }
        if yields.nrows() < 2 {
            return Err(crate::Error::Validation(format!(
                "Need at least 2 observations, got {}",
                yields.nrows()
            )));
        }

        // Validate dates length if provided
        if let Some(ref d) = dates {
            if d.len() != yields.nrows() {
                return Err(crate::Error::Validation(format!(
                    "Dates vector has length {} but yield matrix has {} rows",
                    d.len(),
                    yields.nrows()
                )));
            }
        }

        // Validate all yield values are finite
        for r in 0..yields.nrows() {
            for c in 0..yields.ncols() {
                if !yields[(r, c)].is_finite() {
                    return Err(crate::Error::Validation(format!(
                        "Non-finite yield at row {r}, col {c}: {}",
                        yields[(r, c)]
                    )));
                }
            }
        }

        Ok(Self {
            yields,
            tenors,
            dates,
        })
    }

    /// Number of observation dates.
    #[must_use]
    pub fn num_dates(&self) -> usize {
        self.yields.nrows()
    }

    /// Number of tenors.
    #[must_use]
    pub fn num_tenors(&self) -> usize {
        self.tenors.len()
    }

    /// Compute first differences of yields (T-1 x N matrix).
    #[must_use]
    pub fn yield_changes(&self) -> DMatrix<f64> {
        let t = self.yields.nrows();
        let n = self.yields.ncols();
        let mut changes = DMatrix::zeros(t - 1, n);
        for i in 0..(t - 1) {
            for j in 0..n {
                changes[(i, j)] = self.yields[(i + 1, j)] - self.yields[(i, j)];
            }
        }
        changes
    }
}

// ---------------------------------------------------------------------------
// FactorTimeSeries
// ---------------------------------------------------------------------------

/// Time series of extracted Nelson-Siegel factors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorTimeSeries {
    /// Factor matrix: T rows x 3 columns [beta1, beta2, beta3].
    /// beta1 = level, beta2 = slope, beta3 = curvature.
    pub factors: DMatrix<f64>,
    /// Residuals from OLS factor extraction: T x N.
    pub residuals: DMatrix<f64>,
    /// R-squared per tenor (length N).
    pub r_squared: Vec<f64>,
    /// Overall cross-sectional R-squared (average across tenors).
    pub r_squared_avg: f64,
}

// ---------------------------------------------------------------------------
// YieldForecast
// ---------------------------------------------------------------------------

/// h-step ahead yield curve forecast with confidence bands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldForecast {
    /// Forecast horizon in periods.
    pub horizon: usize,
    /// Point forecast: zero rates at each tenor (length N).
    pub yields: Vec<f64>,
    /// Tenor grid (length N).
    pub tenors: Vec<f64>,
    /// Factor point forecast [beta1, beta2, beta3].
    pub factors: [f64; 3],
    /// 95% confidence band lower bound per tenor (length N).
    pub lower_95: Vec<f64>,
    /// 95% confidence band upper bound per tenor (length N).
    pub upper_95: Vec<f64>,
}
