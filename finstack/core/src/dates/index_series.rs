//! Index series support for CPI/RPI and other economic indices
//!
//! This module provides types and functionality for working with economic indices
//! such as Consumer Price Index (CPI) and Retail Price Index (RPI). These indices
//! are commonly used in inflation-linked bonds (TIPS/ILBs) and other financial
//! instruments that require indexation.
//!
//! # Features
//!
//! - Multiple interpolation methods (step and linear)
//! - Configurable lag policies (months, days, or no lag)
//! - Optional seasonal adjustment factors
//! - Deterministic index ratio calculations
//! - Support for both point-in-time values and period ratios

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use time::Date;

// Use the appropriate decimal type based on feature flags
#[cfg(feature = "decimal128")]
type IndexDecimal = rust_decimal::Decimal;
#[cfg(not(feature = "decimal128"))]
type IndexDecimal = f64;

// Helper functions for decimal operations
#[cfg(feature = "decimal128")]
mod decimal_ops {
    use super::IndexDecimal;

    pub fn zero() -> IndexDecimal {
        rust_decimal::Decimal::ZERO
    }

    #[allow(dead_code)]
    pub fn one() -> IndexDecimal {
        rust_decimal::Decimal::ONE
    }

    #[allow(dead_code)]
    pub fn new(mantissa: i64, scale: u32) -> IndexDecimal {
        rust_decimal::Decimal::new(mantissa, scale)
    }

    pub fn try_from_f64(f: f64) -> Result<IndexDecimal, crate::Error> {
        rust_decimal::Decimal::try_from(f).map_err(|_| crate::Error::Internal)
    }

    pub fn try_to_f64(d: IndexDecimal) -> Result<f64, crate::Error> {
        d.try_into().map_err(|_| crate::Error::Internal)
    }
}

#[cfg(not(feature = "decimal128"))]
mod decimal_ops {
    use super::IndexDecimal;

    pub fn zero() -> IndexDecimal {
        0.0
    }

    #[allow(dead_code)]
    pub fn one() -> IndexDecimal {
        1.0
    }

    #[allow(dead_code)]
    pub fn new(mantissa: i64, scale: u32) -> IndexDecimal {
        mantissa as f64 / (10_i64.pow(scale) as f64)
    }

    pub fn try_from_f64(f: f64) -> Result<IndexDecimal, crate::Error> {
        Ok(f)
    }

    pub fn try_to_f64(d: IndexDecimal) -> Result<f64, crate::Error> {
        Ok(d)
    }
}

/// Identifier for an index series (e.g., "US-CPI-U", "UK-RPI")
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IndexId(pub String);

impl IndexId {
    /// Create a new index identifier
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the string representation of the index ID
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for IndexId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for IndexId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for IndexId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Interpolation method for index values between observation points
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IndexInterpolation {
    /// Last observation carried forward (typical for monthly CPI)
    Step,
    /// Linear interpolation between observed points
    Linear,
}

impl Default for IndexInterpolation {
    fn default() -> Self {
        Self::Step
    }
}

/// Lag policy for index application
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IndexLag {
    /// Lag by specified number of months (e.g., 3-month lag)
    Months(u8),
    /// Lag by specified number of calendar days
    Days(u16),
    /// No lag applied
    None,
}

impl Default for IndexLag {
    fn default() -> Self {
        Self::None
    }
}

/// Seasonal adjustment policy
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SeasonalityPolicy {
    /// No seasonal adjustment
    None,
    /// Apply multiplicative seasonal factors by month
    Multiplicative,
}

impl Default for SeasonalityPolicy {
    fn default() -> Self {
        Self::None
    }
}

/// Economic index series with observation data and policies
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IndexSeries {
    /// Unique identifier for this index
    pub id: IndexId,
    /// Observation dates (typically month-ends) and index values
    /// Must be sorted by date in ascending order
    pub observations: Vec<(Date, IndexDecimal)>,
    /// Interpolation method between observations
    pub interpolation: IndexInterpolation,
    /// Lag policy for index application
    pub lag: IndexLag,
    /// Optional seasonal adjustment factors (one per calendar month)
    pub seasonality: Option<[IndexDecimal; 12]>,
}

impl IndexSeries {
    /// Create a new index series with the given observations
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the series
    /// * `observations` - Vector of (date, value) tuples, must be sorted by date
    ///
    /// # Errors
    ///
    /// Returns an error if observations are empty or not sorted by date
    pub fn new<I>(id: IndexId, observations: I) -> crate::Result<Self>
    where
        I: IntoIterator<Item = (Date, IndexDecimal)>,
    {
        let mut obs: Vec<(Date, IndexDecimal)> = observations.into_iter().collect();

        if obs.is_empty() {
            return Err(crate::Error::Input(crate::error::InputError::TooFewPoints));
        }

        // Sort by date to ensure proper ordering
        obs.sort_by(|a, b| a.0.cmp(&b.0));

        // Verify no duplicate dates
        for window in obs.windows(2) {
            if window[0].0 == window[1].0 {
                return Err(crate::Error::Input(
                    crate::error::InputError::NonMonotonicKnots,
                ));
            }
        }

        Ok(Self {
            id,
            observations: obs,
            interpolation: IndexInterpolation::default(),
            lag: IndexLag::default(),
            seasonality: None,
        })
    }

    /// Set the interpolation method
    pub fn with_interpolation(mut self, interpolation: IndexInterpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Set the lag policy
    pub fn with_lag(mut self, lag: IndexLag) -> Self {
        self.lag = lag;
        self
    }

    /// Set seasonal adjustment factors (12 factors for Jan-Dec)
    pub fn with_seasonality(mut self, factors: [IndexDecimal; 12]) -> Self {
        self.seasonality = Some(factors);
        self
    }

    /// Get the index value applicable on a given date after applying lag,
    /// interpolation, and seasonality adjustments
    pub fn value_on(&self, date: Date) -> crate::Result<IndexDecimal> {
        // Apply lag to get the effective date for index lookup
        let effective_date = self.apply_lag(date)?;

        // Get the base index value using interpolation
        let base_value = self.interpolate_value(effective_date)?;

        // Apply seasonal adjustment if configured
        let adjusted_value = self.apply_seasonality(base_value, effective_date);

        Ok(adjusted_value)
    }

    /// Calculate index ratio I(settle_date)/I(base_date)
    ///
    /// This is commonly used in inflation-linked bond calculations
    pub fn ratio(&self, base_date: Date, settle_date: Date) -> crate::Result<IndexDecimal> {
        let base_value = self.value_on(base_date)?;
        let settle_value = self.value_on(settle_date)?;

        if base_value == decimal_ops::zero() {
            return Err(crate::Error::Input(
                crate::error::InputError::NonPositiveValue,
            ));
        }

        Ok(settle_value / base_value)
    }

    /// Get the index identifier
    pub fn id(&self) -> &IndexId {
        &self.id
    }

    /// Get the observation count
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// Check if the series has no observations
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    /// Get the date range covered by observations
    pub fn date_range(&self) -> Option<(Date, Date)> {
        if self.observations.is_empty() {
            return None;
        }

        let first_date = self.observations.first().unwrap().0;
        let last_date = self.observations.last().unwrap().0;
        Some((first_date, last_date))
    }

    // Private helper methods

    /// Apply the lag policy to a date
    fn apply_lag(&self, date: Date) -> crate::Result<Date> {
        match self.lag {
            IndexLag::None => Ok(date),
            IndexLag::Days(days) => {
                let duration = time::Duration::days(days as i64);
                date.checked_sub(duration).ok_or(crate::Error::Input(
                    crate::error::InputError::InvalidDateRange,
                ))
            }
            IndexLag::Months(months) => {
                // Subtract months using a simple approximation
                // For more precise month arithmetic, we'd need a proper date library
                let days_approx = (months as i64) * 30;
                let duration = time::Duration::days(days_approx);
                date.checked_sub(duration).ok_or(crate::Error::Input(
                    crate::error::InputError::InvalidDateRange,
                ))
            }
        }
    }

    /// Interpolate the index value for a given date
    fn interpolate_value(&self, date: Date) -> crate::Result<IndexDecimal> {
        // Find the appropriate observations to interpolate between
        let pos = self.observations.binary_search_by_key(&date, |(d, _)| *d);

        match pos {
            // Exact match found
            Ok(idx) => Ok(self.observations[idx].1),
            // Need to interpolate or extrapolate
            Err(idx) => {
                if idx == 0 {
                    // Date is before first observation - use first value (flat extrapolation)
                    Ok(self.observations[0].1)
                } else if idx >= self.observations.len() {
                    // Date is after last observation - use last value (flat extrapolation)
                    Ok(self.observations.last().unwrap().1)
                } else {
                    // Interpolate between observations
                    let (d1, v1) = self.observations[idx - 1];
                    let (d2, v2) = self.observations[idx];

                    match self.interpolation {
                        IndexInterpolation::Step => {
                            // Last observation carried forward
                            Ok(v1)
                        }
                        IndexInterpolation::Linear => {
                            // Linear interpolation
                            let total_days = (d2 - d1).whole_days() as f64;
                            let elapsed_days = (date - d1).whole_days() as f64;

                            if total_days == 0.0 {
                                return Ok(v1);
                            }

                            let weight = elapsed_days / total_days;
                            let v1_f64 = decimal_ops::try_to_f64(v1)?;
                            let v2_f64 = decimal_ops::try_to_f64(v2)?;

                            let interpolated = v1_f64 + weight * (v2_f64 - v1_f64);

                            decimal_ops::try_from_f64(interpolated)
                        }
                    }
                }
            }
        }
    }

    /// Apply seasonal adjustment factors
    fn apply_seasonality(&self, base_value: IndexDecimal, date: Date) -> IndexDecimal {
        if let Some(seasonal_factors) = &self.seasonality {
            let month_idx = (date.month() as usize) - 1; // Convert to 0-based index
            let factor = seasonal_factors[month_idx];
            base_value * factor
        } else {
            base_value
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn make_date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
    }

    fn sample_cpi_series() -> IndexSeries {
        let observations = vec![
            (make_date(2023, 1, 31), decimal_ops::new(1000, 1)), // 100.0
            (make_date(2023, 2, 28), decimal_ops::new(1010, 1)), // 101.0
            (make_date(2023, 3, 31), decimal_ops::new(1020, 1)), // 102.0
            (make_date(2023, 4, 30), decimal_ops::new(1025, 1)), // 102.5
        ];

        IndexSeries::new(IndexId::new("TEST-CPI"), observations).unwrap()
    }

    #[test]
    fn index_series_creation() {
        let series = sample_cpi_series();
        assert_eq!(series.id().as_str(), "TEST-CPI");
        assert_eq!(series.len(), 4);
        assert!(!series.is_empty());
    }

    #[test]
    fn index_exact_date_lookup() {
        let series = sample_cpi_series();
        let value = series.value_on(make_date(2023, 2, 28)).unwrap();
        assert_eq!(value, decimal_ops::new(1010, 1));
    }

    #[test]
    fn index_step_interpolation() {
        let series = sample_cpi_series();
        // Date between Feb 28 and Mar 31 should use Feb value (step interpolation)
        let value = series.value_on(make_date(2023, 3, 15)).unwrap();
        assert_eq!(value, decimal_ops::new(1010, 1));
    }

    #[test]
    fn index_linear_interpolation() {
        let mut series = sample_cpi_series();
        series.interpolation = IndexInterpolation::Linear;

        // Midpoint between Feb 28 (101.0) and Mar 31 (102.0) should be ~101.5
        let value = series.value_on(make_date(2023, 3, 15)).unwrap();

        // The exact value depends on the number of days, but should be between 101.0 and 102.0
        let v_f64 = decimal_ops::try_to_f64(value).unwrap();
        assert!(v_f64 > 101.0 && v_f64 < 102.0);
    }

    #[test]
    fn index_ratio_calculation() {
        let series = sample_cpi_series();
        let ratio = series
            .ratio(make_date(2023, 1, 31), make_date(2023, 4, 30))
            .unwrap();

        // Ratio should be 102.5 / 100.0 = 1.025
        let expected = decimal_ops::new(1025, 3);
        assert_eq!(ratio, expected);
    }

    #[test]
    fn index_with_lag() {
        let mut series = sample_cpi_series();
        series.lag = IndexLag::Days(30);

        // Requesting Apr 30 with 30-day lag should give us approximately Mar 31 value
        let value = series.value_on(make_date(2023, 4, 30)).unwrap();
        // Due to flat extrapolation and step interpolation, we should get the Mar 31 value
        assert_eq!(value, decimal_ops::new(1020, 1));
    }

    #[test]
    fn index_seasonal_adjustment() {
        let mut series = sample_cpi_series();

        // Set up seasonal factors where January has a 1.1 multiplier
        let mut seasonal_factors = [decimal_ops::one(); 12];
        seasonal_factors[0] = decimal_ops::new(11, 1); // 1.1 for January
        series.seasonality = Some(seasonal_factors);

        let value = series.value_on(make_date(2023, 1, 31)).unwrap();
        let expected = decimal_ops::new(1100, 1); // 110.0

        // Use approximate equality for floating point comparisons
        #[cfg(feature = "decimal128")]
        assert_eq!(value, expected);
        #[cfg(not(feature = "decimal128"))]
        {
            let eps = 1e-12;
            assert!(
                (value - expected).abs() < eps,
                "Expected ~{}, got {}",
                expected,
                value
            );
        }
    }

    #[test]
    fn empty_series_error() {
        let result = IndexSeries::new(IndexId::new("EMPTY"), Vec::<(Date, IndexDecimal)>::new());
        assert!(result.is_err());
    }

    #[test]
    fn date_range_extraction() {
        let series = sample_cpi_series();
        let (start, end) = series.date_range().unwrap();
        assert_eq!(start, make_date(2023, 1, 31));
        assert_eq!(end, make_date(2023, 4, 30));
    }
}
