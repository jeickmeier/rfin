use std::sync::Arc;

use crate::market_data::traits::Discounting;
use crate::types::{CurveId, InstrumentId};
use crate::Result;

use super::curve_storage::CurveStorage;
use super::MarketContext;

use crate::market_data::{
    dividends::DividendSchedule,
    scalars::inflation_index::InflationIndex,
    scalars::{MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
    term_structures::{
        base_correlation::BaseCorrelationCurve, credit_index::CreditIndexData,
        discount_curve::DiscountCurve, forward_curve::ForwardCurve, hazard_curve::HazardCurve,
        inflation::InflationCurve,
    },
};

impl MarketContext {
    // -----------------------------------------------------------------------------
    // Single generic typed getters for curves
    // -----------------------------------------------------------------------------

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
                crate::error::Error::Validation(format!(
                    "Type mismatch: curve '{}' is '{}', expected '{}'",
                    id,
                    storage.curve_type(),
                    expected_type
                ))
            }),
            None => Err(crate::error::InputError::NotFound { id: id.to_string() }.into()),
        }
    }

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

    /// Borrow a discount curve by identifier.
    pub fn get_discount_ref(&self, id: impl AsRef<str>) -> Result<&DiscountCurve> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::Discount(curve)) => Ok(curve.as_ref()),
            Some(storage) => Err(crate::error::Error::Validation(format!(
                "Type mismatch: curve '{}' is '{}', expected 'Discount'",
                id_str,
                storage.curve_type()
            ))),
            None => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into()),
        }
    }

    /// Borrow a forward curve by identifier.
    pub fn get_forward_ref(&self, id: impl AsRef<str>) -> Result<&ForwardCurve> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::Forward(curve)) => Ok(curve.as_ref()),
            Some(storage) => Err(crate::error::Error::Validation(format!(
                "Type mismatch: curve '{}' is '{}', expected 'Forward'",
                id_str,
                storage.curve_type()
            ))),
            None => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into()),
        }
    }

    /// Borrow a hazard curve by identifier.
    pub fn get_hazard_ref(&self, id: impl AsRef<str>) -> Result<&HazardCurve> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::Hazard(curve)) => Ok(curve.as_ref()),
            Some(storage) => Err(crate::error::Error::Validation(format!(
                "Type mismatch: curve '{}' is '{}', expected 'Hazard'",
                id_str,
                storage.curve_type()
            ))),
            None => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into()),
        }
    }

    /// Borrow an inflation curve by identifier.
    pub fn get_inflation_ref(&self, id: impl AsRef<str>) -> Result<&InflationCurve> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::Inflation(curve)) => Ok(curve.as_ref()),
            Some(storage) => Err(crate::error::Error::Validation(format!(
                "Type mismatch: curve '{}' is '{}', expected 'Inflation'",
                id_str,
                storage.curve_type()
            ))),
            None => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into()),
        }
    }

    /// Borrow a base correlation curve by identifier.
    pub fn get_base_correlation_ref(&self, id: impl AsRef<str>) -> Result<&BaseCorrelationCurve> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::BaseCorrelation(curve)) => Ok(curve.as_ref()),
            Some(storage) => Err(crate::error::Error::Validation(format!(
                "Type mismatch: curve '{}' is '{}', expected 'BaseCorrelation'",
                id_str,
                storage.curve_type()
            ))),
            None => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into()),
        }
    }

    /// Clone a volatility surface by identifier.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::surfaces::vol_surface::VolSurface;
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

    /// Borrow a volatility surface without cloning the `Arc`.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::surfaces::vol_surface::VolSurface;
    /// # let surface = VolSurface::builder("IR-Swaption")
    /// #     .expiries(&[1.0, 2.0])
    /// #     .strikes(&[90.0, 100.0])
    /// #     .row(&[0.2, 0.2])
    /// #     .row(&[0.2, 0.2])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new().insert_surface(surface);
    /// let surface = ctx.surface_ref("IR-Swaption").expect("Surface should exist");
    /// assert!((surface.value_clamped(1.5, 95.0) - 0.2).abs() < 1e-12);
    /// ```
    pub fn surface_ref(&self, id: impl AsRef<str>) -> Result<&VolSurface> {
        let id_str = id.as_ref();
        self.surfaces
            .get(id_str)
            .map(|arc| arc.as_ref())
            .ok_or_else(|| {
                crate::error::Error::from(crate::error::InputError::NotFound {
                    id: id_str.to_string(),
                })
            })
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
    /// # use finstack_core::market_data::scalars::inflation_index::{InflationIndex, InflationInterpolation};
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

    /// Borrow an inflation index without cloning the `Arc`.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::scalars::inflation_index::{InflationIndex, InflationInterpolation};
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
    /// let idx = ctx.inflation_index_ref("US-CPI").expect("Inflation index should exist");
    /// assert_eq!(idx.id, "US-CPI");
    /// ```
    pub fn inflation_index_ref(&self, id: impl AsRef<str>) -> Option<&InflationIndex> {
        self.inflation_indices
            .get(id.as_ref())
            .map(|arc| arc.as_ref())
    }

    /// Clone a dividend schedule by identifier.
    pub fn dividend_schedule(&self, id: impl AsRef<str>) -> Option<Arc<DividendSchedule>> {
        self.dividends.get(id.as_ref()).cloned()
    }

    /// Borrow a dividend schedule by identifier.
    pub fn dividend_schedule_ref(&self, id: impl AsRef<str>) -> Option<&DividendSchedule> {
        self.dividends.get(id.as_ref()).map(|arc| arc.as_ref())
    }

    /// Clone a credit index aggregate by identifier.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::term_structures::credit_index::CreditIndexData;
    /// # use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
    /// # use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use time::Month;
    /// # let hazard = Arc::new(HazardCurve::builder("CDX")
    /// #     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 0.01), (5.0, 0.015)])
    /// #     .build()
    /// #     .expect("... creation should succeed"));
    /// # let base_corr = Arc::new(BaseCorrelationCurve::builder("CDX")
    /// #     .points([(3.0, 0.25), (10.0, 0.55)])
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

    /// Borrow a credit index without cloning the `Arc`.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::term_structures::credit_index::CreditIndexData;
    /// # use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
    /// # use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use time::Month;
    /// # let hazard = Arc::new(HazardCurve::builder("CDX")
    /// #     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 0.01), (5.0, 0.015)])
    /// #     .build()
    /// #     .expect("... creation should succeed"));
    /// # let base_corr = Arc::new(BaseCorrelationCurve::builder("CDX")
    /// #     .points([(3.0, 0.25), (10.0, 0.55)])
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
    /// let idx = ctx.credit_index_ref("CDX-IG").expect("Credit index should exist");
    /// assert_eq!(idx.recovery_rate, 0.4);
    /// ```
    pub fn credit_index_ref(&self, id: impl AsRef<str>) -> Result<&CreditIndexData> {
        let id_str = id.as_ref();
        self.credit_indices
            .get(id_str)
            .map(|arc| arc.as_ref())
            .ok_or_else(|| {
                crate::error::Error::from(crate::error::InputError::NotFound {
                    id: id_str.to_string(),
                })
            })
    }

    /// Resolve a collateral discount curve for a CSA code.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
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

    /// Borrow the collateral discount curve without cloning the `Arc`.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// # use finstack_core::math::interp::InterpStyle;
    /// # use finstack_core::dates::Date;
    /// # use finstack_core::types::CurveId;
    /// # use time::Month;
    /// # let curve = DiscountCurve::builder("USD-OIS")
    /// #     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 1.0), (1.0, 0.99)])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new()
    /// #     .insert_discount(curve)
    /// #     .map_collateral("USD-CSA", CurveId::from("USD-OIS"));
    /// let discount = ctx.collateral_ref("USD-CSA").expect("Collateral curve should exist");
    /// assert!(discount.df(0.5) <= 1.0);
    /// ```
    pub fn collateral_ref(&self, csa_code: &str) -> Result<&dyn Discounting> {
        let curve_id = self
            .collateral
            .get(csa_code)
            .ok_or(crate::error::InputError::NotFound {
                id: format!("collateral:{}", csa_code),
            })?;
        self.get_discount_ref(curve_id.as_str())
            .map(|r| r as &dyn Discounting)
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

    // -----------------------------------------------------------------------------
    // Instrument registry (type-erased)
    // -----------------------------------------------------------------------------

    /// Borrow a type-erased instrument from the registry.
    pub fn get_instrument(&self, id: impl AsRef<str>) -> Result<&dyn std::any::Any> {
        let key = id.as_ref();
        let id = InstrumentId::from(key);
        self.instruments
            .get(&id)
            .map(|arc| arc.as_ref() as &dyn std::any::Any)
            .ok_or_else(|| crate::error::InputError::NotFound { id: key.to_string() }.into())
    }

    /// Alias for [`MarketContext::get_instrument`].
    ///
    /// Named to read naturally at call sites (`market.instrument("...")?`).
    pub fn instrument(&self, id: impl AsRef<str>) -> Result<&dyn std::any::Any> {
        self.get_instrument(id)
    }
}


