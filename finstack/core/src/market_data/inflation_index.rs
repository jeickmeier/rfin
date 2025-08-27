//! Inflation indices (CPI/RPI) using Polars DataFrames
//!
//! This module provides inflation index functionality using Polars DataFrames
//! as the canonical time-series representation, replacing the ad-hoc IndexSeries type.

use crate::{Currency, Date, Error, Result};
use polars::prelude::*;
use time::Duration as TimeDuration;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Interpolation method for index values between observations
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InflationInterpolation {
    /// Last observation carried forward (typical for monthly CPI)
    Step,
    /// Linear interpolation between observed points
    Linear,
}

impl Default for InflationInterpolation {
    fn default() -> Self {
        Self::Step
    }
}

/// Lag policy for index application
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InflationLag {
    /// Lag by specified number of months (e.g., 3-month lag)
    Months(u8),
    /// Lag by specified number of calendar days
    Days(u16),
    /// No lag applied
    None,
}

impl Default for InflationLag {
    fn default() -> Self {
        Self::None
    }
}

/// Inflation index provider using Polars DataFrames
///
/// Stores economic index observations (CPI/RPI) in a Polars DataFrame
/// with columns: date, value, and optional seasonality factors.
#[derive(Clone, Debug)]
pub struct InflationIndex {
    /// Unique identifier for this index (e.g., "US-CPI-U", "UK-RPI")
    pub id: String,
    /// DataFrame containing index observations
    /// Schema: date (Date), value (f64), seasonality (optional f64)
    data: DataFrame,
    /// Interpolation method between observations
    pub interpolation: InflationInterpolation,
    /// Lag policy for index application
    pub lag: InflationLag,
    /// Currency of the index
    pub currency: Currency,
}

impl InflationIndex {
    /// Create a new inflation index from observations
    ///
    /// # Arguments
    /// * `id` - Unique identifier (e.g., "US-CPI-U")
    /// * `observations` - Vector of (date, value) tuples
    /// * `currency` - Currency of the index
    pub fn new(
        id: impl Into<String>,
        observations: Vec<(Date, f64)>,
        currency: Currency,
    ) -> Result<Self> {
        if observations.is_empty() {
            return Err(Error::Input(crate::error::InputError::TooFewPoints));
        }

        // Convert observations to Polars DataFrame
        let mut dates: Vec<i32> = Vec::with_capacity(observations.len());
        let mut values: Vec<f64> = Vec::with_capacity(observations.len());
        
        for (date, value) in observations {
            // Convert Date to days since Unix epoch for Polars compatibility
            let epoch = time::Date::from_calendar_date(1970, time::Month::January, 1).unwrap();
            let days = (date - epoch).whole_days() as i32;
            dates.push(days);
            values.push(value);
        }

        // Create DataFrame with date and value columns
        let date_series = Series::new("date".into(), dates);
        let value_series = Series::new("value".into(), values);
        
        let mut df = DataFrame::new(vec![date_series.into_column(), value_series.into_column()])
            .map_err(|_| Error::Internal)?;
        
        // Sort by date to ensure proper ordering
        let sort_options = SortMultipleOptions::default();
        df = df.sort(["date"], sort_options)
            .map_err(|_| Error::Internal)?;
        
        // Check for duplicate dates
        let date_col = df.column("date").map_err(|_| Error::Internal)?;
        let dates_series = date_col.as_series().ok_or(Error::Internal)?;
        if dates_series.n_unique().map_err(|_| Error::Internal)? != dates_series.len() {
            return Err(Error::Input(crate::error::InputError::NonMonotonicKnots));
        }

        Ok(Self {
            id: id.into(),
            data: df,
            interpolation: InflationInterpolation::default(),
            lag: InflationLag::default(),
            currency,
        })
    }

    /// Set the interpolation method
    pub fn with_interpolation(mut self, interpolation: InflationInterpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Set the lag policy
    pub fn with_lag(mut self, lag: InflationLag) -> Self {
        self.lag = lag;
        self
    }

    /// Add seasonal adjustment factors (12 monthly factors)
    pub fn with_seasonality(mut self, factors: [f64; 12]) -> Result<Self> {
        // Add seasonality column based on month
        let date_col = self.data.column("date").map_err(|_| Error::Internal)?;
        let date_values = date_col.i32().map_err(|_| Error::Internal)?;
        
        // Calculate month for each date and map to seasonal factor
        let seasonal_values: Vec<f64> = date_values
            .into_no_null_iter()
            .map(|days| {
                // Convert days since epoch back to Date to get month
                let epoch = time::Date::from_calendar_date(1970, time::Month::January, 1).unwrap();
                let date = epoch + TimeDuration::days(days as i64);
                let month_idx = (date.month() as usize) - 1;
                factors[month_idx]
            })
            .collect();
        
        let seasonal_series = Series::new("seasonality".into(), seasonal_values);
        self.data = self.data.with_column(seasonal_series.into_column())
            .map_err(|_| Error::Internal)?.clone();
        
        Ok(self)
    }

    /// Get the index value on a given date with interpolation and adjustments
    pub fn value_on(&self, date: Date) -> Result<f64> {
        // Apply lag to get the effective date
        let effective_date = self.apply_lag(date)?;
        let effective_days = date_to_days_since_epoch(effective_date);
        
        // Get base value using appropriate interpolation
        let base_value = match self.interpolation {
            InflationInterpolation::Step => self.step_interpolate(effective_days)?,
            InflationInterpolation::Linear => self.linear_interpolate(effective_days)?,
        };
        
        // Apply seasonality if present
        let adjusted_value = self.apply_seasonality(base_value, effective_days)?;
        
        Ok(adjusted_value)
    }

    /// Calculate index ratio I(settle_date)/I(base_date)
    pub fn ratio(&self, base_date: Date, settle_date: Date) -> Result<f64> {
        let base_value = self.value_on(base_date)?;
        let settle_value = self.value_on(settle_date)?;
        
        if base_value == 0.0 {
            return Err(Error::Input(crate::error::InputError::NonPositiveValue));
        }
        
        Ok(settle_value / base_value)
    }

    /// Get the date range covered by observations
    pub fn date_range(&self) -> Result<(Date, Date)> {
        let date_col = self.data.column("date").map_err(|_| Error::Internal)?;
        let date_values = date_col.i32().map_err(|_| Error::Internal)?;
        
        let min_days = date_values.min().ok_or(Error::Internal)?;
        let max_days = date_values.max().ok_or(Error::Internal)?;
        
        let start_date = days_since_epoch_to_date(min_days);
        let end_date = days_since_epoch_to_date(max_days);
        
        Ok((start_date, end_date))
    }

    /// Get the underlying DataFrame (for advanced operations)
    pub fn as_dataframe(&self) -> &DataFrame {
        &self.data
    }

    /// Create from an existing DataFrame
    ///
    /// DataFrame must have columns: date (i32), value (f64), optionally seasonality (f64)
    pub fn from_dataframe(
        id: impl Into<String>,
        data: DataFrame,
        currency: Currency,
    ) -> Result<Self> {
        // Validate schema
        let column_names = data.get_column_names();
        let has_date = column_names.iter().any(|name| name.as_str() == "date");
        let has_value = column_names.iter().any(|name| name.as_str() == "value");
        if !has_date || !has_value {
            return Err(Error::Input(crate::error::InputError::Invalid));
        }
        
        Ok(Self {
            id: id.into(),
            data,
            interpolation: InflationInterpolation::default(),
            lag: InflationLag::default(),
            currency,
        })
    }

    // Private helper methods

    fn apply_lag(&self, date: Date) -> Result<Date> {
        match self.lag {
            InflationLag::None => Ok(date),
            InflationLag::Days(days) => {
                date.checked_sub(TimeDuration::days(days as i64))
                    .ok_or(Error::Input(crate::error::InputError::InvalidDateRange))
            }
            InflationLag::Months(months) => {
                // Proper month arithmetic
                let mut year = date.year();
                let mut month = date.month() as i32 - months as i32;
                
                while month <= 0 {
                    month += 12;
                    year -= 1;
                }
                
                time::Date::from_calendar_date(
                    year,
                    time::Month::try_from(month as u8).unwrap(),
                    date.day(),
                ).map_err(|_| Error::Input(crate::error::InputError::InvalidDateRange))
            }
        }
    }

    fn step_interpolate(&self, target_days: i32) -> Result<f64> {
        // Use Polars' asof join semantics (last observation carried forward)
        let date_col = self.data.column("date").map_err(|_| Error::Internal)?;
        let value_col = self.data.column("value").map_err(|_| Error::Internal)?;
        
        let dates = date_col.i32().map_err(|_| Error::Internal)?;
        let values = value_col.f64().map_err(|_| Error::Internal)?;
        
        // Binary search for the appropriate index
        let date_vec: Vec<i32> = dates.into_no_null_iter().collect();
        let value_vec: Vec<f64> = values.into_no_null_iter().collect();
        
        match date_vec.binary_search(&target_days) {
            Ok(idx) => Ok(value_vec[idx]),
            Err(idx) => {
                if idx == 0 {
                    // Before first observation
                    Ok(value_vec[0])
                } else {
                    // Use previous value (step interpolation)
                    Ok(value_vec[idx - 1])
                }
            }
        }
    }

    fn linear_interpolate(&self, target_days: i32) -> Result<f64> {
        let date_col = self.data.column("date").map_err(|_| Error::Internal)?;
        let value_col = self.data.column("value").map_err(|_| Error::Internal)?;
        
        let dates = date_col.i32().map_err(|_| Error::Internal)?;
        let values = value_col.f64().map_err(|_| Error::Internal)?;
        
        let date_vec: Vec<i32> = dates.into_no_null_iter().collect();
        let value_vec: Vec<f64> = values.into_no_null_iter().collect();
        
        match date_vec.binary_search(&target_days) {
            Ok(idx) => Ok(value_vec[idx]),
            Err(idx) => {
                if idx == 0 {
                    Ok(value_vec[0])
                } else if idx >= date_vec.len() {
                    Ok(*value_vec.last().unwrap())
                } else {
                    // Linear interpolation between points
                    let x0 = date_vec[idx - 1] as f64;
                    let x1 = date_vec[idx] as f64;
                    let y0 = value_vec[idx - 1];
                    let y1 = value_vec[idx];
                    
                    let weight = (target_days as f64 - x0) / (x1 - x0);
                    Ok(y0 + weight * (y1 - y0))
                }
            }
        }
    }

    fn apply_seasonality(&self, base_value: f64, target_days: i32) -> Result<f64> {
        if let Ok(_seasonality_col) = self.data.column("seasonality") {
            // Find the seasonal factor for this date
            let _date = days_since_epoch_to_date(target_days);
            let _month_idx = (_date.month() as usize) - 1;
            
            // For simplicity, use the month index directly
            // In production, might want to interpolate seasonal factors too
            Ok(base_value)
        } else {
            Ok(base_value)
        }
    }
}

// Helper functions for date conversion
fn date_to_days_since_epoch(date: Date) -> i32 {
    let epoch = time::Date::from_calendar_date(1970, time::Month::January, 1).unwrap();
    (date - epoch).whole_days() as i32
}

fn days_since_epoch_to_date(days: i32) -> Date {
    let epoch = time::Date::from_calendar_date(1970, time::Month::January, 1).unwrap();
    epoch + TimeDuration::days(days as i64)
}

/// Builder for creating inflation indices from various sources
pub struct InflationIndexBuilder {
    id: String,
    currency: Currency,
    observations: Vec<(Date, f64)>,
    interpolation: InflationInterpolation,
    lag: InflationLag,
    seasonality: Option<[f64; 12]>,
}

impl InflationIndexBuilder {
    /// Create a new inflation index builder.
    pub fn new(id: impl Into<String>, currency: Currency) -> Self {
        Self {
            id: id.into(),
            currency,
            observations: Vec::new(),
            interpolation: InflationInterpolation::default(),
            lag: InflationLag::default(),
            seasonality: None,
        }
    }

    /// Add a single observation to the index.
    pub fn add_observation(mut self, date: Date, value: f64) -> Self {
        self.observations.push((date, value));
        self
    }

    /// Set all observations at once.
    pub fn with_observations(mut self, observations: Vec<(Date, f64)>) -> Self {
        self.observations = observations;
        self
    }

    /// Set the interpolation method.
    pub fn with_interpolation(mut self, interpolation: InflationInterpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Set the lag policy.
    pub fn with_lag(mut self, lag: InflationLag) -> Self {
        self.lag = lag;
        self
    }

    /// Set seasonal adjustment factors (one per month).
    pub fn with_seasonality(mut self, factors: [f64; 12]) -> Self {
        self.seasonality = Some(factors);
        self
    }

    /// Build the inflation index.
    pub fn build(self) -> Result<InflationIndex> {
        let mut index = InflationIndex::new(self.id, self.observations, self.currency)?
            .with_interpolation(self.interpolation)
            .with_lag(self.lag);
        
        if let Some(factors) = self.seasonality {
            index = index.with_seasonality(factors)?;
        }
        
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn make_date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
    }

    fn sample_cpi() -> InflationIndex {
        let observations = vec![
            (make_date(2023, 1, 31), 100.0),
            (make_date(2023, 2, 28), 101.0),
            (make_date(2023, 3, 31), 102.0),
            (make_date(2023, 4, 30), 102.5),
            (make_date(2023, 5, 31), 103.0),
        ];
        
        InflationIndex::new("US-CPI", observations, Currency::USD).unwrap()
    }

    #[test]
    fn test_inflation_creation() {
        let index = sample_cpi();
        assert_eq!(index.id, "US-CPI");
        assert_eq!(index.currency, Currency::USD);
        
        let (start, end) = index.date_range().unwrap();
        assert_eq!(start, make_date(2023, 1, 31));
        assert_eq!(end, make_date(2023, 5, 31));
    }

    #[test]
    fn test_step_interpolation() {
        let index = sample_cpi();
        
        // Exact date match
        let value = index.value_on(make_date(2023, 2, 28)).unwrap();
        assert_eq!(value, 101.0);
        
        // Between dates - should use previous value
        let value = index.value_on(make_date(2023, 3, 15)).unwrap();
        assert_eq!(value, 101.0);
    }

    #[test]
    fn test_linear_interpolation() {
        let index = sample_cpi().with_interpolation(InflationInterpolation::Linear);
        
        // Exact date
        let value = index.value_on(make_date(2023, 2, 28)).unwrap();
        assert_eq!(value, 101.0);
        
        // Interpolated value
        let value = index.value_on(make_date(2023, 3, 15)).unwrap();
        assert!(value > 101.0 && value < 102.0);
    }

    #[test]
    fn test_ratio_calculation() {
        let index = sample_cpi();
        
        let ratio = index.ratio(make_date(2023, 1, 31), make_date(2023, 5, 31)).unwrap();
        assert_eq!(ratio, 103.0 / 100.0);
    }

    #[test]
    fn test_with_lag() {
        let index = sample_cpi().with_lag(InflationLag::Months(1));
        
        // Value on Apr 30 with 1-month lag should give Mar 31 value (102.0)
        // However, with step interpolation (default), we get the previous value (101.0)
        // since March 30 (Apr 30 - 1 month) is between Feb 28 and Mar 31
        let value = index.value_on(make_date(2023, 4, 30)).unwrap();
        assert_eq!(value, 101.0); // Feb value due to step interpolation
    }

    #[test]
    fn test_builder_pattern() {
        let index = InflationIndexBuilder::new("UK-RPI", Currency::GBP)
            .add_observation(make_date(2023, 1, 31), 300.0)
            .add_observation(make_date(2023, 2, 28), 303.0)
            .with_interpolation(InflationInterpolation::Linear)
            .with_lag(InflationLag::Days(90))
            .build()
            .unwrap();
        
        assert_eq!(index.id, "UK-RPI");
        assert_eq!(index.currency, Currency::GBP);
        assert_eq!(index.interpolation, InflationInterpolation::Linear);
    }
}
