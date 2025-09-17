//! Generic market primitives used by valuations: scalars and ad-hoc time series.
//!
//! This module provides two minimal building blocks that are not modeled as
//! classic term structures but are still required by pricing and risk engines:
//!
//! - `MarketScalar`: single numeric value (unitless or price in a currency)
//! - `ScalarTimeSeries`: generic date → value series with step/linear interp
//!
//! Both are integrated into the [`crate::market_data::MarketContext`]
//! so downstream code can reference them by `CurveId` alongside other curves.

use crate::currency::Currency;
use crate::dates::Date;
use crate::types::CurveId;
use crate::{error::InputError, Result};
use polars::prelude::*;
#[cfg(test)]
use time::Duration as TimeDuration;

/// Interpolation strategy for [`ScalarTimeSeries`].
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::scalars::{ScalarTimeSeries, SeriesInterpolation};
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let series = ScalarTimeSeries::new(
///     "TS",
///     vec![
///         (Date::from_calendar_date(2024, Month::January, 1).unwrap(), 100.0),
///         (Date::from_calendar_date(2024, Month::February, 1).unwrap(), 105.0),
///     ],
///     None,
/// )
/// .unwrap();
/// let stepped = series.clone().with_interpolation(SeriesInterpolation::Step);
/// let mid_date = Date::from_calendar_date(2024, Month::January, 15).unwrap();
/// assert_eq!(stepped.value_on(mid_date).unwrap(), 100.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum SeriesInterpolation {
    /// Last observation carried forward
    Step,
    /// Linear interpolation between observed points
    Linear,
}

impl Default for SeriesInterpolation {
    fn default() -> Self {
        Self::Step
    }
}

/// A single market scalar which can be unitless or a price in a currency.
///
/// Scalars are frequently used for simple quotes (spots, spreads, recovery
/// assumptions) that do not warrant a full term structure.
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::scalars::MarketScalar;
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
///
/// let unitless = MarketScalar::Unitless(0.75);
/// let priced = MarketScalar::Price(Money::new(99.5, Currency::USD));
///
/// assert!(matches!(unitless, MarketScalar::Unitless(_)));
/// if let MarketScalar::Price(m) = priced {
///     assert_eq!(m.currency(), Currency::USD);
/// }
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum MarketScalar {
    /// Unitless numeric (e.g., equity beta, recovery rate assumption)
    Unitless(crate::F),
    /// Monetary price or amount with currency
    Price(crate::money::Money),
}

/// Generic date-indexed time series with configurable interpolation.
///
/// Note: This struct cannot be directly serialized due to the internal DataFrame.
/// Use `to_state()` and `from_state()` for serialization.
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::scalars::{ScalarTimeSeries, SeriesInterpolation};
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let series = ScalarTimeSeries::new(
///     "US CPI",
///     vec![
///         (Date::from_calendar_date(2024, Month::January, 31).unwrap(), 100.0),
///         (Date::from_calendar_date(2024, Month::February, 29).unwrap(), 101.2),
///     ],
///     None,
/// )
/// .unwrap()
/// .with_interpolation(SeriesInterpolation::Linear);
/// let mid = Date::from_calendar_date(2024, Month::February, 14).unwrap();
/// let interpolated = series.value_on(mid).unwrap();
/// assert!(interpolated > 100.0);
/// ```
#[derive(Clone, Debug)]
pub struct ScalarTimeSeries {
    id: CurveId,
    currency: Option<Currency>,
    /// DataFrame with schema: date (i32 days since epoch), value (f64)
    data: DataFrame,
    interpolation: SeriesInterpolation,
}

impl ScalarTimeSeries {
    /// Create a new time series from `(Date, value)` observations.
    ///
    /// # Parameters
    /// - `id`: identifier used when storing the series inside [`MarketContext`](crate::market_data::context::MarketContext)
    /// - `observations`: sorted or unsorted list of observations (duplicates not allowed)
    /// - `currency`: optional currency tag when the series represents a monetary amount
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::scalars::ScalarTimeSeries;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let series = ScalarTimeSeries::new(
    ///     "TREASURY",
    ///     vec![
    ///         (Date::from_calendar_date(2024, Month::January, 1).unwrap(), 4.5),
    ///         (Date::from_calendar_date(2024, Month::February, 1).unwrap(), 4.7),
    ///     ],
    ///     None,
    /// )
    /// .unwrap();
    /// assert_eq!(series.id().as_str(), "TREASURY");
    /// ```
    pub fn new(
        id: impl AsRef<str>,
        observations: Vec<(Date, crate::F)>,
        currency: Option<Currency>,
    ) -> Result<Self> {
        if observations.is_empty() {
            return Err(crate::Error::Input(InputError::TooFewPoints));
        }

        let mut dates: Vec<i32> = Vec::with_capacity(observations.len());
        let mut values: Vec<crate::F> = Vec::with_capacity(observations.len());
        for (d, v) in observations {
            let days = crate::dates::utils::date_to_days_since_epoch(d);
            dates.push(days);
            values.push(v);
        }

        let df = DataFrame::new(vec![
            Series::new("date".into(), dates).into_column(),
            Series::new("value".into(), values).into_column(),
        ])
        .map_err(|_| crate::Error::Internal)?
        .sort(["date"], SortMultipleOptions::default())
        .map_err(|_| crate::Error::Internal)?;

        // Check for strictly increasing dates (no duplicates)
        let date_col = df.column("date").map_err(|_| crate::Error::Internal)?;
        let dates_series = date_col.as_series().ok_or(crate::Error::Internal)?;
        if dates_series
            .n_unique()
            .map_err(|_| crate::Error::Internal)?
            != dates_series.len()
        {
            return Err(crate::Error::Input(InputError::NonMonotonicKnots));
        }

        Ok(Self {
            id: CurveId::from(id.as_ref()),
            currency,
            data: df,
            interpolation: SeriesInterpolation::default(),
        })
    }

    /// Override the interpolation method used for lookups.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::scalars::{ScalarTimeSeries, SeriesInterpolation};
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let series = ScalarTimeSeries::new(
    ///     "TS",
    ///     vec![
    ///         (Date::from_calendar_date(2024, Month::January, 1).unwrap(), 10.0),
    ///         (Date::from_calendar_date(2024, Month::February, 1).unwrap(), 20.0),
    ///     ],
    ///     None,
    /// )
    /// .unwrap()
    /// .with_interpolation(SeriesInterpolation::Linear);
    /// assert!(matches!(series.interpolation(), SeriesInterpolation::Linear));
    /// ```
    pub fn with_interpolation(mut self, interpolation: SeriesInterpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Identifier accessor.
    pub fn id(&self) -> &CurveId {
        &self.id
    }

    /// Optional currency accessor.
    pub fn currency(&self) -> Option<Currency> {
        self.currency
    }

    /// Current interpolation method accessor.
    pub fn interpolation(&self) -> SeriesInterpolation {
        self.interpolation
    }

    /// Retrieve the value on a given date according to the active interpolation.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::scalars::{ScalarTimeSeries, SeriesInterpolation};
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let series = ScalarTimeSeries::new(
    ///     "TS",
    ///     vec![
    ///         (Date::from_calendar_date(2024, Month::January, 1).unwrap(), 10.0),
    ///         (Date::from_calendar_date(2024, Month::February, 1).unwrap(), 20.0),
    ///     ],
    ///     None,
    /// )
    /// .unwrap();
    /// let jan = Date::from_calendar_date(2024, Month::January, 15).unwrap();
    /// assert_eq!(series.value_on(jan).unwrap(), 10.0);
    ///
    /// let linear = series.clone().with_interpolation(SeriesInterpolation::Linear);
    /// assert!(linear.value_on(jan).unwrap() > 10.0);
    /// ```
    pub fn value_on(&self, date: Date) -> Result<crate::F> {
        let days = crate::dates::utils::date_to_days_since_epoch(date);
        self.values_on_days(&[days]).map(|v| v[0])
    }

    /// Retrieve values for multiple dates at once.
    ///
    /// The returned vector is aligned with the input order. Step interpolation
    /// carries the last observation forward while Linear blends between
    /// neighboring observations.
    pub fn values_on(&self, dates: &[Date]) -> Result<Vec<crate::F>> {
        let query_days: Vec<i32> = dates
            .iter()
            .map(|&d| crate::dates::utils::date_to_days_since_epoch(d))
            .collect();
        self.values_on_days(&query_days)
    }

    /// Internal vectorized lookup using days since epoch.
    /// Leverages Polars native operations for optimal performance.
    fn values_on_days(&self, query_days: &[i32]) -> Result<Vec<crate::F>> {
        if query_days.is_empty() {
            return Ok(Vec::new());
        }

        // For efficiency, cache the extracted vectors and use optimized lookup
        let date_col = self
            .data
            .column("date")
            .map_err(|_| crate::Error::Internal)?;
        let value_col = self
            .data
            .column("value")
            .map_err(|_| crate::Error::Internal)?;
        let dates_series = date_col.i32().map_err(|_| crate::Error::Internal)?;
        let values_series = value_col.f64().map_err(|_| crate::Error::Internal)?;

        // Convert to Vec once for all lookups
        let date_vec: Vec<i32> = dates_series.into_no_null_iter().collect();
        let value_vec: Vec<crate::F> = values_series.into_no_null_iter().collect();

        // Vectorized interpolation with optimized branch prediction
        match self.interpolation {
            SeriesInterpolation::Step => {
                self.vectorized_step_interpolation(&date_vec, &value_vec, query_days)
            }
            SeriesInterpolation::Linear => {
                self.vectorized_linear_interpolation(&date_vec, &value_vec, query_days)
            }
        }
    }

    /// Optimized step interpolation for multiple query points.
    fn vectorized_step_interpolation(
        &self,
        date_vec: &[i32],
        value_vec: &[crate::F],
        query_days: &[i32],
    ) -> Result<Vec<crate::F>> {
        let mut result = Vec::with_capacity(query_days.len());

        for &query_day in query_days {
            let value = match date_vec.binary_search(&query_day) {
                Ok(idx) => value_vec[idx],
                Err(idx) => {
                    if idx == 0 {
                        value_vec[0] // Use first value for dates before series
                    } else {
                        value_vec[idx - 1] // Last observation carried forward
                    }
                }
            };
            result.push(value);
        }

        Ok(result)
    }

    /// Optimized linear interpolation for multiple query points.
    fn vectorized_linear_interpolation(
        &self,
        date_vec: &[i32],
        value_vec: &[crate::F],
        query_days: &[i32],
    ) -> Result<Vec<crate::F>> {
        let mut result = Vec::with_capacity(query_days.len());

        for &query_day in query_days {
            let value = match date_vec.binary_search(&query_day) {
                Ok(idx) => value_vec[idx],
                Err(idx) => {
                    if idx == 0 {
                        value_vec[0] // Use first value for dates before series
                    } else if idx >= date_vec.len() {
                        *value_vec.last().unwrap() // Use last value for dates after series
                    } else {
                        // Linear interpolation between adjacent points
                        let x0 = date_vec[idx - 1] as crate::F;
                        let x1 = date_vec[idx] as crate::F;
                        let y0 = value_vec[idx - 1];
                        let y1 = value_vec[idx];
                        let weight = (query_day as crate::F - x0) / (x1 - x0);
                        y0 + weight * (y1 - y0)
                    }
                }
            };
            result.push(value);
        }

        Ok(result)
    }

    /// Expose the underlying DataFrame for advanced consumers.
    pub fn as_dataframe(&self) -> &DataFrame {
        &self.data
    }
}

// (moved) helper centralized in crate::dates::utils

/// Serializable state of a ScalarTimeSeries
#[cfg(feature = "serde")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ScalarTimeSeriesState {
    /// Series identifier
    pub id: String,
    /// Optional currency
    pub currency: Option<Currency>,
    /// Observations as (date, value) pairs
    pub observations: Vec<(Date, crate::F)>,
    /// Interpolation method
    pub interpolation: SeriesInterpolation,
}

#[cfg(feature = "serde")]
impl ScalarTimeSeries {
    /// Extract serializable state
    pub fn to_state(&self) -> Result<ScalarTimeSeriesState> {
        let dates = self
            .data
            .column("date")
            .map_err(|_| crate::Error::Internal)?
            .i32()
            .map_err(|_| crate::Error::Internal)?;
        let values = self
            .data
            .column("value")
            .map_err(|_| crate::Error::Internal)?
            .f64()
            .map_err(|_| crate::Error::Internal)?;

        let observations: Vec<(Date, crate::F)> = dates
            .into_no_null_iter()
            .zip(values.into_no_null_iter())
            .map(|(d, v)| (crate::dates::utils::days_since_epoch_to_date(d), v))
            .collect();

        Ok(ScalarTimeSeriesState {
            id: self.id.to_string(),
            currency: self.currency,
            observations,
            interpolation: self.interpolation,
        })
    }

    /// Create from serialized state
    pub fn from_state(state: ScalarTimeSeriesState) -> Result<Self> {
        Self::new(state.id, state.observations, state.currency)
            .map(|s| s.with_interpolation(state.interpolation))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_step_and_linear() {
        let d0 = time::Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let d1 = time::Date::from_calendar_date(2025, time::Month::February, 1).unwrap();
        let d2 = time::Date::from_calendar_date(2025, time::Month::March, 1).unwrap();
        let s =
            ScalarTimeSeries::new("US-UNEMP", vec![(d0, 3.0), (d1, 4.0), (d2, 5.0)], None).unwrap();

        // Midpoint between d0 and d1
        let mid = d0 + TimeDuration::days(15);
        let step_v = s
            .clone()
            .with_interpolation(SeriesInterpolation::Step)
            .value_on(mid)
            .unwrap();
        assert!((step_v - 3.0).abs() < 1e-12);

        let lin_v = s
            .clone()
            .with_interpolation(SeriesInterpolation::Linear)
            .value_on(mid)
            .unwrap();
        assert!(lin_v > 3.0 && lin_v < 4.0);
    }
}
