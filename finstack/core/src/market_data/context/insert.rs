use std::sync::Arc;

use crate::money::fx::FxMatrix;
use crate::types::{CurveId, InstrumentId};

use super::CurveStorage;
use super::MarketContext;

use crate::market_data::{
    dividends::DividendSchedule,
    scalars::inflation_index::InflationIndex,
    scalars::{MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
    term_structures::credit_index::CreditIndexData,
    term_structures::{
        base_correlation::BaseCorrelationCurve, discount_curve::DiscountCurve,
        forward_curve::ForwardCurve, hazard_curve::HazardCurve, inflation::InflationCurve,
        vol_index_curve::VolatilityIndexCurve,
    },
};

impl MarketContext {
    // -----------------------------------------------------------------------------
    // Insert methods - builder pattern
    // -----------------------------------------------------------------------------

    /// Insert a generic curve storage entry.
    ///
    /// This is primarily intended for downstream crates that operate on heterogeneous
    /// curve types (e.g., calibration pipelines) and want to update the context
    /// without matching on concrete curve variants.
    pub fn insert<C>(mut self, curve: C) -> Self
    where
        C: Into<CurveStorage>,
    {
        let curve: CurveStorage = curve.into();
        let id = curve.id().to_owned();
        self.curves.insert(id, curve);
        self
    }

    /// In-place insert of a generic curve storage entry.
    ///
    /// See [`MarketContext::insert`] for details.
    pub fn insert_mut<C>(&mut self, curve: C) -> &mut Self
    where
        C: Into<CurveStorage>,
    {
        let curve: CurveStorage = curve.into();
        let id = curve.id().to_owned();
        self.curves.insert(id, curve);
        self
    }

    /// Insert a discount curve.
    pub fn insert_discount(mut self, curve: DiscountCurve) -> Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// In-place insert of a discount curve.
    pub fn insert_discount_mut(&mut self, curve: DiscountCurve) -> &mut Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// Insert a forward curve.
    pub fn insert_forward(mut self, curve: ForwardCurve) -> Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// In-place insert of a forward curve.
    pub fn insert_forward_mut(&mut self, curve: ForwardCurve) -> &mut Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// Insert a hazard curve.
    pub fn insert_hazard(mut self, curve: HazardCurve) -> Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// In-place insert of a hazard curve.
    pub fn insert_hazard_mut(&mut self, curve: HazardCurve) -> &mut Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// Insert an inflation curve.
    pub fn insert_inflation(mut self, curve: InflationCurve) -> Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// In-place insert of an inflation curve.
    pub fn insert_inflation_mut(&mut self, curve: InflationCurve) -> &mut Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// Insert a base correlation curve.
    pub fn insert_base_correlation(mut self, curve: BaseCorrelationCurve) -> Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// In-place insert of a base correlation curve.
    pub fn insert_base_correlation_mut(&mut self, curve: BaseCorrelationCurve) -> &mut Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// Insert a volatility index curve.
    ///
    /// # Parameters
    /// - `curve`: [`VolatilityIndexCurve`] to store (VIX, VXN, VSTOXX)
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::term_structures::vol_index_curve::VolatilityIndexCurve;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let curve = VolatilityIndexCurve::builder("VIX")
    ///     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    ///     .spot_level(18.5)
    ///     .knots([(0.0, 18.5), (0.5, 20.0)])
    ///     .build()
    ///     .expect("VolatilityIndexCurve builder should succeed");
    /// let ctx = MarketContext::new().insert_vol_index(curve);
    /// assert!(ctx.get_vol_index("VIX").is_ok());
    /// ```
    pub fn insert_vol_index(mut self, curve: VolatilityIndexCurve) -> Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// In-place insert of a volatility index curve.
    pub fn insert_vol_index_mut(&mut self, curve: VolatilityIndexCurve) -> &mut Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// Insert a volatility surface.
    ///
    /// # Parameters
    /// - `surface`: built [`VolSurface`]
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
    /// let ctx = MarketContext::new().insert_surface(surface);
    /// assert_eq!(ctx.stats().surface_count, 1);
    /// ```
    pub fn insert_surface(mut self, surface: VolSurface) -> Self {
        let id = surface.id().to_owned();
        self.surfaces.insert(id, Arc::new(surface));
        self
    }

    /// In-place insert of a volatility surface.
    pub fn insert_surface_mut(&mut self, surface: VolSurface) -> &mut Self {
        let id = surface.id().to_owned();
        self.surfaces.insert(id, Arc::new(surface));
        self
    }

    /// In-place insert of a shared volatility surface.
    pub fn insert_surface_arc_mut(&mut self, surface: Arc<VolSurface>) -> &mut Self {
        let id = surface.id().to_owned();
        self.surfaces.insert(id, surface);
        self
    }

    /// Insert a shared dividend schedule.
    ///
    /// # Parameters
    /// - `schedule`: a [`DividendSchedule`] built via its builder
    pub fn insert_dividends(mut self, schedule: DividendSchedule) -> Self {
        let id = schedule.id.to_owned();
        self.dividends.insert(id, Arc::new(schedule));
        self
    }

    /// In-place insert of a dividend schedule.
    pub fn insert_dividends_mut(&mut self, schedule: DividendSchedule) -> &mut Self {
        let id = schedule.id.to_owned();
        self.dividends.insert(id, Arc::new(schedule));
        self
    }

    /// Insert a market scalar/price.
    ///
    /// # Parameters
    /// - `id`: identifier (string-like) stored as [`CurveId`]
    /// - `price`: scalar value to store
    pub fn insert_price(mut self, id: impl AsRef<str>, price: MarketScalar) -> Self {
        self.prices.insert(CurveId::from(id.as_ref()), price);
        self
    }

    /// In-place insert of a market scalar/price.
    pub fn insert_price_mut(&mut self, id: impl AsRef<str>, price: MarketScalar) -> &mut Self {
        self.prices.insert(CurveId::from(id.as_ref()), price);
        self
    }

    /// Insert a scalar time series.
    ///
    /// # Parameters
    /// - `series`: [`ScalarTimeSeries`] to store
    pub fn insert_series(mut self, series: ScalarTimeSeries) -> Self {
        let id = series.id().to_owned();
        self.series.insert(id, series);
        self
    }

    /// In-place insert of a scalar time series.
    pub fn insert_series_mut(&mut self, series: ScalarTimeSeries) -> &mut Self {
        let id = series.id().to_owned();
        self.series.insert(id, series);
        self
    }

    /// Insert an inflation index.
    ///
    /// # Parameters
    /// - `id`: identifier stored as [`CurveId`]
    /// - `index`: inflation index object
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::scalars::inflation_index::{InflationIndex, InflationInterpolation};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let observations = vec![
    ///     (Date::from_calendar_date(2024, Month::January, 31).expect("Valid date"), 100.0),
    ///     (Date::from_calendar_date(2024, Month::February, 29).expect("Valid date"), 101.0),
    /// ];
    /// let index = InflationIndex::new("US-CPI", observations, Currency::USD)
    ///     .expect("InflationIndex creation should succeed")
    ///     .with_interpolation(InflationInterpolation::Linear);
    /// let ctx = MarketContext::new().insert_inflation_index("US-CPI", index);
    /// assert!(ctx.inflation_index("US-CPI").is_some());
    /// ```
    pub fn insert_inflation_index(mut self, id: impl AsRef<str>, index: InflationIndex) -> Self {
        self.inflation_indices
            .insert(CurveId::from(id.as_ref()), Arc::new(index));
        self
    }

    /// Insert a credit index aggregate.
    ///
    /// # Parameters
    /// - `id`: identifier stored as [`CurveId`]
    /// - `data`: [`CreditIndexData`] bundle
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::term_structures::credit_index::CreditIndexData;
    /// use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
    /// use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
    /// use finstack_core::dates::Date;
    /// use std::sync::Arc;
    /// use time::Month;
    ///
    /// let hazard = Arc::new(HazardCurve::builder("CDX")
    ///     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    ///     .knots([(0.0, 0.01), (5.0, 0.015)])
    ///     .build()
    ///     .expect("HazardCurve builder should succeed"));
    /// let base_corr = Arc::new(BaseCorrelationCurve::builder("CDX")
    ///     .knots([(3.0, 0.25), (10.0, 0.55)])
    ///     .build()
    ///     .expect("BaseCorrelationCurve builder should succeed"));
    /// let data = CreditIndexData::builder()
    ///     .num_constituents(125)
    ///     .recovery_rate(0.4)
    ///     .index_credit_curve(Arc::clone(&hazard))
    ///     .base_correlation_curve(base_corr)
    ///     .build()
    ///     .expect("CreditIndexData builder should succeed");
    /// let ctx = MarketContext::new().insert_credit_index("CDX-IG", data);
    /// assert!(ctx.credit_index("CDX-IG").is_ok());
    /// ```
    pub fn insert_credit_index(mut self, id: impl AsRef<str>, data: CreditIndexData) -> Self {
        self.credit_indices
            .insert(CurveId::from(id.as_ref()), Arc::new(data));
        self
    }

    /// In-place insert of a credit index aggregate.
    pub fn insert_credit_index_mut(
        &mut self,
        id: impl AsRef<str>,
        data: CreditIndexData,
    ) -> &mut Self {
        self.credit_indices
            .insert(CurveId::from(id.as_ref()), Arc::new(data));
        self
    }

    /// Insert an FX matrix.
    ///
    /// # Parameters
    /// - `fx`: [`FxMatrix`] instance used for currency conversions
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::money::fx::{FxMatrix, FxProvider, FxConversionPolicy};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use std::sync::Arc;
    /// use time::Month;
    ///
    /// struct StaticFx;
    /// impl FxProvider for StaticFx {
    ///     fn rate(
    ///         &self,
    ///         _from: Currency,
    ///         _to: Currency,
    ///         _on: Date,
    ///         _policy: FxConversionPolicy,
    ///     ) -> finstack_core::Result<f64> {
    ///         Ok(1.1)
    ///     }
    /// }
    ///
    /// let fx = FxMatrix::new(Arc::new(StaticFx));
    /// let ctx = MarketContext::new().insert_fx(fx);
    /// assert!(ctx.fx.is_some());
    /// ```
    pub fn insert_fx(mut self, fx: FxMatrix) -> Self {
        self.fx = Some(Arc::new(fx));
        self
    }

    /// In-place insert of FX matrix from Arc.
    pub fn insert_fx_mut(&mut self, fx: FxMatrix) -> &mut Self {
        self.fx = Some(Arc::new(fx));
        self
    }

    /// Insert historical market scenarios for VaR calculation.
    ///
    /// # Parameters
    /// - `history`: Historical market scenarios (type-erased)
    pub fn insert_market_history(mut self, history: Arc<dyn std::any::Any + Send + Sync>) -> Self {
        self.market_history = Some(history);
        self
    }

    /// In-place insert of historical market scenarios.
    pub fn insert_market_history_mut(
        &mut self,
        history: Arc<dyn std::any::Any + Send + Sync>,
    ) -> &mut Self {
        self.market_history = Some(history);
        self
    }

    /// Insert a type-erased instrument into the context registry.
    ///
    /// This is used by higher-level pricing layers for instruments that reference other
    /// instruments (e.g., bond futures referencing a CTD bond).
    pub fn insert_instrument(
        mut self,
        id: impl AsRef<str>,
        instrument: Arc<dyn std::any::Any + Send + Sync>,
    ) -> Self {
        self.instruments
            .insert(InstrumentId::new(id.as_ref()), instrument);
        self
    }

    /// In-place insert of an instrument into the registry.
    pub fn insert_instrument_mut(
        &mut self,
        id: impl AsRef<str>,
        instrument: Arc<dyn std::any::Any + Send + Sync>,
    ) -> &mut Self {
        self.instruments
            .insert(InstrumentId::new(id.as_ref()), instrument);
        self
    }

    /// Map collateral CSA code to a discount curve identifier.
    ///
    /// # Parameters
    /// - `csa_code`: CSA identifier (e.g., "USD-CSA")
    /// - `discount_id`: target discount curve [`CurveId`]
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
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
    /// assert!(ctx.collateral("USD-CSA").is_ok());
    /// ```
    pub fn map_collateral(mut self, csa_code: impl Into<String>, discount_id: CurveId) -> Self {
        self.collateral.insert(csa_code.into(), discount_id);
        self
    }

    /// In-place map collateral to curve id.
    pub fn map_collateral_mut(
        &mut self,
        csa_code: impl Into<String>,
        discount_id: CurveId,
    ) -> &mut Self {
        self.collateral.insert(csa_code.into(), discount_id);
        self
    }
}
