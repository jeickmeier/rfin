use std::sync::Arc;

use crate::market_data::traits::Discounting;
use crate::types::CurveId;
use crate::Result;

use super::curve_storage::CurveStorage;
use super::MarketContext;

use crate::market_data::{
    dividends::DividendSchedule,
    scalars::InflationIndex,
    scalars::{MarketScalar, ScalarTimeSeries},
    surfaces::VolSurface,
    term_structures::{
        BaseCorrelationCurve, CreditIndexData, DiscountCurve, ForwardCurve, HazardCurve,
        InflationCurve, VolatilityIndexCurve,
    },
};

impl MarketContext {
    // -----------------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------------

    fn missing_curve_error(&self, id: &str) -> crate::Error {
        let available: Vec<String> = self.curves.keys().map(|k| k.to_string()).collect();
        crate::error::Error::missing_curve_with_suggestions(id, &available)
    }

    /// Helper method to extract curve with type checking and error handling
    fn get_curve_with_type_check<T, F>(
        &self,
        id: &str,
        expected_type: &'static str,
        extractor: F,
    ) -> Result<T>
    where
        F: FnOnce(&CurveStorage) -> Option<T>,
    {
        match self.curves.get(id) {
            Some(storage) => extractor(storage).ok_or_else(|| {
                crate::error::Error::from(crate::error::InputError::WrongCurveType {
                    id: id.to_string(),
                    expected: expected_type.to_string(),
                    actual: storage.curve_type().to_string(),
                })
            }),
            None => Err(self.missing_curve_error(id)),
        }
    }

    // -----------------------------------------------------------------------------
    // Public API: typed getters
    // -----------------------------------------------------------------------------

    /// Get a discount curve by identifier.
    pub fn get_discount(&self, id: impl AsRef<str>) -> Result<Arc<DiscountCurve>> {
        let id_str = id.as_ref();
        self.get_curve_with_type_check(id_str, "Discount", |storage| {
            storage.discount().map(Arc::clone)
        })
    }

    /// Get a forward curve by identifier.
    pub fn get_forward(&self, id: impl AsRef<str>) -> Result<Arc<ForwardCurve>> {
        let id_str = id.as_ref();
        self.get_curve_with_type_check(id_str, "Forward", |storage| {
            storage.forward().map(Arc::clone)
        })
    }

    /// Get a hazard curve by identifier.
    pub fn get_hazard(&self, id: impl AsRef<str>) -> Result<Arc<HazardCurve>> {
        let id_str = id.as_ref();
        self.get_curve_with_type_check(id_str, "Hazard", |storage| storage.hazard().map(Arc::clone))
    }

    /// Get an inflation curve by identifier.
    pub fn get_inflation(&self, id: impl AsRef<str>) -> Result<Arc<InflationCurve>> {
        let id_str = id.as_ref();
        self.get_curve_with_type_check(id_str, "Inflation", |storage| {
            storage.inflation().map(Arc::clone)
        })
    }

    /// Get a base correlation curve by identifier.
    pub fn get_base_correlation(&self, id: impl AsRef<str>) -> Result<Arc<BaseCorrelationCurve>> {
        let id_str = id.as_ref();
        self.get_curve_with_type_check(id_str, "BaseCorrelation", |storage| {
            storage.base_correlation().map(Arc::clone)
        })
    }

    /// Get a volatility index curve by identifier.
    pub fn get_vol_index(&self, id: impl AsRef<str>) -> Result<Arc<VolatilityIndexCurve>> {
        let id_str = id.as_ref();
        self.get_curve_with_type_check(id_str, "VolIndex", |storage| {
            storage.vol_index().map(Arc::clone)
        })
    }

    /// Clone a volatility surface by identifier.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::surfaces::VolSurface;
    /// # let surface = VolSurface::builder("IR-Swaption")
    /// #     .expiries(&[1.0, 2.0])
    /// #     .strikes(&[90.0, 100.0])
    /// #     .row(&[0.2, 0.2])
    /// #     .row(&[0.2, 0.2])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new().insert_surface(surface);
    /// let surface = ctx.surface("IR-Swaption").expect("Surface should exist");
    /// assert!((surface.value_clamped(1.5, 95.0) - 0.2).abs() < 1e-12);
    /// ```
    pub fn surface(&self, id: impl AsRef<str>) -> Result<Arc<VolSurface>> {
        let id_str = id.as_ref();
        self.surfaces.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Borrow a market price/scalar by identifier.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::scalars::MarketScalar;
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let ctx = MarketContext::new()
    ///     .insert_price("AAPL", MarketScalar::Price(Money::new(180.0, Currency::USD)));
    /// if let MarketScalar::Price(price) = ctx.price("AAPL").expect("Price should exist") {
    ///     assert_eq!(price.currency(), Currency::USD);
    /// }
    /// ```
    pub fn price(&self, id: impl AsRef<str>) -> Result<&MarketScalar> {
        let id_str = id.as_ref();
        self.prices.get(id_str).ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Borrow a scalar time series by identifier.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::scalars::ScalarTimeSeries;
    /// # use finstack_core::dates::Date;
    /// # use time::Month;
    /// # let series = ScalarTimeSeries::new(
    /// #     "VOL-TS",
    /// #     vec![
    /// #         (Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"), 0.2),
    /// #         (Date::from_calendar_date(2024, Month::February, 1).expect("Valid date"), 0.25),
    /// #     ],
    /// #     None,
    /// # ).expect("... creation should succeed");
    /// # let ctx = MarketContext::new().insert_series(series);
    /// let series = ctx.series("VOL-TS").expect("Series should exist");
    /// assert_eq!(series.id().as_str(), "VOL-TS");
    /// ```
    pub fn series(&self, id: impl AsRef<str>) -> Result<&ScalarTimeSeries> {
        let id_str = id.as_ref();
        self.series.get(id_str).ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Clone an inflation index by identifier.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::scalars::{InflationIndex, InflationInterpolation};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::Date;
    /// # use time::Month;
    /// # let observations = vec![
    /// #     (Date::from_calendar_date(2024, Month::January, 31).expect("Valid date"), 100.0),
    /// #     (Date::from_calendar_date(2024, Month::February, 29).expect("Valid date"), 101.0),
    /// # ];
    /// # let index = InflationIndex::new("US-CPI", observations, Currency::USD)
    /// #     .expect("... creation should succeed")
    /// #     .with_interpolation(InflationInterpolation::Linear);
    /// # let ctx = MarketContext::new().insert_inflation_index("US-CPI", index);
    /// let idx = ctx.inflation_index("US-CPI").expect("Inflation index should exist");
    /// assert_eq!(idx.id, "US-CPI");
    /// ```
    pub fn inflation_index(&self, id: impl AsRef<str>) -> Option<Arc<InflationIndex>> {
        self.inflation_indices.get(id.as_ref()).cloned()
    }

    /// Clone a dividend schedule by identifier.
    pub fn dividend_schedule(&self, id: impl AsRef<str>) -> Option<Arc<DividendSchedule>> {
        self.dividends.get(id.as_ref()).cloned()
    }

    /// Clone a credit index aggregate by identifier.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::term_structures::{BaseCorrelationCurve, CreditIndexData, HazardCurve};
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use time::Month;
    /// # let hazard = Arc::new(HazardCurve::builder("CDX")
    /// #     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 0.01), (5.0, 0.015)])
    /// #     .build()
    /// #     .expect("... creation should succeed"));
    /// # let base_corr = Arc::new(BaseCorrelationCurve::builder("CDX")
    /// #     .knots([(3.0, 0.25), (10.0, 0.55)])
    /// #     .build()
    /// #     .expect("... creation should succeed"));
    /// # let data = CreditIndexData::builder()
    /// #     .num_constituents(125)
    /// #     .recovery_rate(0.4)
    /// #     .index_credit_curve(Arc::clone(&hazard))
    /// #     .base_correlation_curve(base_corr)
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new().insert_credit_index("CDX-IG", data);
    /// let idx = ctx.credit_index("CDX-IG").expect("Credit index should exist");
    /// assert_eq!(idx.num_constituents, 125);
    /// ```
    pub fn credit_index(&self, id: impl AsRef<str>) -> Result<Arc<CreditIndexData>> {
        let id_str = id.as_ref();
        self.credit_indices.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Resolve a collateral discount curve for a CSA code.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// use finstack_core::math::interp::InterpStyle;
    /// use finstack_core::dates::Date;
    /// use finstack_core::types::CurveId;
    /// use time::Month;
    ///
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    ///     .knots([(0.0, 1.0), (1.0, 0.99)])
    ///     .build()
    ///     .expect("... builder should succeed");
    /// let ctx = MarketContext::new()
    ///     .insert_discount(curve)
    ///     .map_collateral("USD-CSA", CurveId::from("USD-OIS"));
    /// let discount = ctx.collateral("USD-CSA").expect("Collateral curve should exist");
    /// assert!(discount.df(0.5) <= 1.0);
    /// ```
    pub fn collateral(&self, csa_code: &str) -> Result<Arc<dyn Discounting + Send + Sync>> {
        let curve_id = self
            .collateral
            .get(csa_code)
            .ok_or(crate::error::InputError::NotFound {
                id: format!("collateral:{}", csa_code),
            })?;
        self.get_discount(curve_id.as_str())
            .map(|arc| arc as Arc<dyn Discounting + Send + Sync>)
    }

    // -----------------------------------------------------------------------------
    // Update methods for special cases
    // -----------------------------------------------------------------------------

    /// Update only the base correlation curve for a credit index.
    ///
    /// Handy for calibration loops that tweak base correlation while leaving
    /// other index data intact. Returns `false` if the index identifier cannot
    /// be found.
    pub fn update_base_correlation_curve(
        &mut self,
        id: impl AsRef<str>,
        new_curve: Arc<BaseCorrelationCurve>,
    ) -> bool {
        let cid = CurveId::from(id.as_ref());

        // Get the existing index data
        let Some(existing_index) = self.credit_indices.get(&cid) else {
            return false;
        };

        // Create a new index with the updated correlation curve
        let updated_index = CreditIndexData {
            num_constituents: existing_index.num_constituents,
            recovery_rate: existing_index.recovery_rate,
            index_credit_curve: Arc::clone(&existing_index.index_credit_curve),
            base_correlation_curve: new_curve,
            issuer_credit_curves: existing_index.issuer_credit_curves.clone(),
            issuer_recovery_rates: existing_index.issuer_recovery_rates.clone(),
            issuer_weights: existing_index.issuer_weights.clone(),
        };

        // Update the context
        self.credit_indices.insert(cid, Arc::new(updated_index));
        true
    }
}
