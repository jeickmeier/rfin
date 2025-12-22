use std::collections::BTreeMap;
use std::sync::Arc;

use crate::types::CurveId;

use super::curve_storage::CurveStorage;
use super::MarketContext;

use crate::market_data::{
    dividends::DividendSchedule,
    scalars::inflation_index::InflationIndex,
    scalars::{MarketScalar, ScalarTimeSeries},
};

impl MarketContext {
    // -----------------------------------------------------------------------------
    // Introspection and statistics
    // -----------------------------------------------------------------------------

    /// Get curve storage by ID (for generic access)
    pub fn curve(&self, id: impl AsRef<str>) -> Option<&CurveStorage> {
        self.curves.get(id.as_ref())
    }

    /// Get all curve IDs
    pub fn curve_ids(&self) -> impl Iterator<Item = &CurveId> {
        self.curves.keys()
    }

    /// Iterate over curves matching a specific type name.
    ///
    /// # Parameters
    /// - `curve_type`: string as returned by [`CurveStorage::curve_type`]
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// # use finstack_core::dates::Date;
    /// # use time::Month;
    /// # let curve = DiscountCurve::builder("USD-OIS")
    /// #     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 1.0), (1.0, 0.99)])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new().insert_discount(curve);
    /// let mut iter = ctx.curves_of_type("Discount");
    /// assert!(iter.next().is_some());
    /// ```
    pub fn curves_of_type<'a>(
        &'a self,
        curve_type: &'a str,
    ) -> impl Iterator<Item = (&'a CurveId, &'a CurveStorage)> + 'a {
        self.curves
            .iter()
            .filter(move |(_, storage)| storage.curve_type() == curve_type)
    }

    /// Count curves grouped by type string.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// # use finstack_core::dates::Date;
    /// # use time::Month;
    /// # let curve = DiscountCurve::builder("USD-OIS")
    /// #     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 1.0), (1.0, 0.99)])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new().insert_discount(curve);
    /// let counts = ctx.count_by_type();
    /// assert_eq!(counts.get("Discount"), Some(&1));
    /// ```
    pub fn count_by_type(&self) -> BTreeMap<&'static str, usize> {
        let mut counts = BTreeMap::new();
        for storage in self.curves.values() {
            *counts.entry(storage.curve_type()).or_insert(0) += 1;
        }
        counts
    }

    /// Compute aggregate statistics about the context contents.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # let stats = MarketContext::new().stats();
    /// assert_eq!(stats.total_curves, 0);
    /// ```
    pub fn stats(&self) -> ContextStats {
        ContextStats {
            curve_counts: self.count_by_type(),
            total_curves: self.curves.len(),
            has_fx: self.fx.is_some(),
            surface_count: self.surfaces.len(),
            price_count: self.prices.len(),
            series_count: self.series.len(),
            inflation_index_count: self.inflation_indices.len(),
            credit_index_count: self.credit_indices.len(),
            dividend_schedule_count: self.dividends.len(),
            collateral_mapping_count: self.collateral.len(),
        }
    }

    /// Return `true` when no market data has been inserted.
    pub fn is_empty(&self) -> bool {
        self.curves.is_empty()
            && self.fx.is_none()
            && self.surfaces.is_empty()
            && self.prices.is_empty()
            && self.series.is_empty()
            && self.inflation_indices.is_empty()
            && self.credit_indices.is_empty()
            && self.instruments.is_empty()
            && self.collateral.is_empty()
    }

    /// Get total number of objects
    pub fn total_objects(&self) -> usize {
        self.curves.len()
            + self.surfaces.len()
            + self.prices.len()
            + self.series.len()
            + self.inflation_indices.len()
            + self.credit_indices.len()
            + self.instruments.len()
            + if self.fx.is_some() { 1 } else { 0 }
    }

    // -----------------------------------------------------------------------------
    // Iterators for Market Scalars (P&L Attribution Support)
    // -----------------------------------------------------------------------------

    /// Iterate over all market prices/scalars.
    ///
    /// Returns an iterator over (CurveId, MarketScalar) pairs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::scalars::MarketScalar;
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let ctx = MarketContext::new()
    ///     .insert_price("AAPL", MarketScalar::Price(Money::new(180.0, Currency::USD)));
    ///
    /// for (id, scalar) in ctx.prices_iter() {
    ///     println!("{}: {:?}", id, scalar);
    /// }
    /// ```
    pub fn prices_iter(&self) -> impl Iterator<Item = (&CurveId, &MarketScalar)> {
        self.prices.iter()
    }

    /// Iterate over all time series.
    ///
    /// Returns an iterator over (CurveId, ScalarTimeSeries) pairs.
    pub fn series_iter(&self) -> impl Iterator<Item = (&CurveId, &ScalarTimeSeries)> {
        self.series.iter()
    }

    /// Iterate over all inflation indices.
    ///
    /// Returns an iterator over `(CurveId, Arc<InflationIndex>)` pairs.
    pub fn inflation_indices_iter(&self) -> impl Iterator<Item = (&CurveId, &Arc<InflationIndex>)> {
        self.inflation_indices.iter()
    }

    /// Iterate over all dividend schedules.
    ///
    /// Returns an iterator over `(CurveId, Arc<DividendSchedule>)` pairs.
    pub fn dividends_iter(&self) -> impl Iterator<Item = (&CurveId, &Arc<DividendSchedule>)> {
        self.dividends.iter()
    }

    /// Set or update a market price (mutable).
    ///
    /// # Arguments
    ///
    /// * `id` - Price identifier
    /// * `price` - Market scalar to store
    ///
    /// # Returns
    ///
    /// Mutable reference to self for chaining.
    pub fn set_price_mut(&mut self, id: CurveId, price: MarketScalar) -> &mut Self {
        self.prices.insert(id, price);
        self
    }

    /// Set or update a time series (mutable).
    ///
    /// # Arguments
    ///
    /// * `series` - Time series to store
    ///
    /// # Returns
    ///
    /// Mutable reference to self for chaining.
    pub fn set_series_mut(&mut self, series: ScalarTimeSeries) -> &mut Self {
        let id = series.id().to_owned();
        self.series.insert(id, series);
        self
    }

    /// Set or update an inflation index (mutable).
    ///
    /// # Arguments
    ///
    /// * `id` - Index identifier
    /// * `index` - Inflation index to store
    ///
    /// # Returns
    ///
    /// Mutable reference to self for chaining.
    pub fn set_inflation_index_mut(
        &mut self,
        id: impl AsRef<str>,
        index: Arc<InflationIndex>,
    ) -> &mut Self {
        self.inflation_indices
            .insert(CurveId::from(id.as_ref()), index);
        self
    }

    /// Set or update a dividend schedule (mutable).
    ///
    /// # Arguments
    ///
    /// * `schedule` - Dividend schedule to store
    ///
    /// # Returns
    ///
    /// Mutable reference to self for chaining.
    pub fn set_dividends_mut(&mut self, schedule: Arc<DividendSchedule>) -> &mut Self {
        let id = schedule.id.to_owned();
        self.dividends.insert(id, schedule);
        self
    }
}

// -----------------------------------------------------------------------------
// Context Statistics
// -----------------------------------------------------------------------------

/// Statistics about the contents of a [`MarketContext`].
///
/// Obtain via [`MarketContext::stats`] to feed dashboards or diagnostics.
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::context::MarketContext;
///
/// let stats = MarketContext::new().stats();
/// assert_eq!(stats.total_curves, 0);
/// assert!(!stats.has_fx);
/// ```
#[derive(Debug, Clone)]
pub struct ContextStats {
    /// Count of curves by type
    pub curve_counts: BTreeMap<&'static str, usize>,
    /// Total number of curves
    pub total_curves: usize,
    /// Whether FX matrix is present
    pub has_fx: bool,
    /// Number of volatility surfaces
    pub surface_count: usize,
    /// Number of market prices/scalars
    pub price_count: usize,
    /// Number of time series
    pub series_count: usize,
    /// Number of inflation indices
    pub inflation_index_count: usize,
    /// Number of credit indices
    pub credit_index_count: usize,
    /// Number of dividend schedules
    pub dividend_schedule_count: usize,
    /// Number of collateral mappings
    pub collateral_mapping_count: usize,
}

impl core::fmt::Display for ContextStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "MarketContext Statistics:")?;
        writeln!(f, "  Total curves: {}", self.total_curves)?;
        for (curve_type, count) in &self.curve_counts {
            writeln!(f, "    {}: {}", curve_type, count)?;
        }
        writeln!(f, "  Surfaces: {}", self.surface_count)?;
        writeln!(f, "  Prices: {}", self.price_count)?;
        writeln!(f, "  Series: {}", self.series_count)?;
        writeln!(f, "  Inflation indices: {}", self.inflation_index_count)?;
        writeln!(f, "  Credit indices: {}", self.credit_index_count)?;
        writeln!(f, "  Dividend schedules: {}", self.dividend_schedule_count)?;
        writeln!(
            f,
            "  Collateral mappings: {}",
            self.collateral_mapping_count
        )?;
        writeln!(f, "  Has FX: {}", self.has_fx)?;
        Ok(())
    }
}
