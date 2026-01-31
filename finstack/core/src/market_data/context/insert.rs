use std::sync::Arc;

use crate::money::fx::FxMatrix;
use crate::types::CurveId;

use super::CurveStorage;
use super::MarketContext;

use crate::market_data::{
    dividends::DividendSchedule,
    scalars::InflationIndex,
    scalars::{MarketScalar, ScalarTimeSeries},
    surfaces::VolSurface,
    term_structures::{
        BaseCorrelationCurve, CreditIndexData, DiscountCurve, ForwardCurve, HazardCurve,
        InflationCurve, PriceCurve, VolatilityIndexCurve,
    },
};

impl MarketContext {
    // -----------------------------------------------------------------------------
    // Insert methods (canonical: builder-by-value)
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

    /// Insert a discount curve.
    pub fn insert_discount(mut self, curve: DiscountCurve) -> Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// Insert a forward curve.
    pub fn insert_forward(mut self, curve: ForwardCurve) -> Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// Insert a hazard curve.
    pub fn insert_hazard(mut self, curve: HazardCurve) -> Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// Insert an inflation curve.
    pub fn insert_inflation(mut self, curve: InflationCurve) -> Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// Insert a base correlation curve.
    pub fn insert_base_correlation(mut self, curve: BaseCorrelationCurve) -> Self {
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
    /// use finstack_core::market_data::term_structures::VolatilityIndexCurve;
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

    /// Insert a price curve (forward prices for commodities/indices).
    ///
    /// # Parameters
    /// - `curve`: [`PriceCurve`] to store (e.g., WTI forward prices)
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::term_structures::PriceCurve;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let curve = PriceCurve::builder("WTI-FORWARD")
    ///     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    ///     .spot_price(75.0)
    ///     .knots([(0.0, 75.0), (0.5, 77.0)])
    ///     .build()
    ///     .expect("PriceCurve builder should succeed");
    /// let ctx = MarketContext::new().insert_price_curve(curve);
    /// assert!(ctx.get_price_curve("WTI-FORWARD").is_ok());
    /// ```
    pub fn insert_price_curve(mut self, curve: PriceCurve) -> Self {
        self.curves.insert(curve.id().to_owned(), curve.into());
        self
    }

    /// Insert a volatility surface.
    ///
    /// Accepts either an owned [`VolSurface`] or an `Arc<VolSurface>`.
    /// When passing an owned value, it will be wrapped in an `Arc` automatically.
    /// When passing an `Arc`, it is used directly (enabling surface sharing between contexts).
    ///
    /// # Parameters
    /// - `surface`: a [`VolSurface`] or `Arc<VolSurface>`
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::surfaces::VolSurface;
    /// # use std::sync::Arc;
    /// # let surface = VolSurface::builder("IR-Swaption")
    /// #     .expiries(&[1.0, 2.0])
    /// #     .strikes(&[90.0, 100.0])
    /// #     .row(&[0.2, 0.2])
    /// #     .row(&[0.2, 0.2])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// // Owned value (wrapped in Arc automatically)
    /// let ctx = MarketContext::new().insert_surface(surface);
    /// assert_eq!(ctx.stats().surface_count, 1);
    ///
    /// // Pre-wrapped Arc (for sharing across contexts)
    /// # let surface2 = VolSurface::builder("EQ-Vol")
    /// #     .expiries(&[1.0, 2.0])
    /// #     .strikes(&[90.0, 100.0])
    /// #     .row(&[0.2, 0.2])
    /// #     .row(&[0.2, 0.2])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// let shared = Arc::new(surface2);
    /// let ctx2 = MarketContext::new().insert_surface(Arc::clone(&shared));
    /// ```
    pub fn insert_surface(mut self, surface: impl Into<Arc<VolSurface>>) -> Self {
        let arc_surface = surface.into();
        let id = arc_surface.id().to_owned();
        self.surfaces.insert(id, arc_surface);
        self
    }

    /// Insert a dividend schedule.
    ///
    /// Accepts either an owned [`DividendSchedule`] or an `Arc<DividendSchedule>`.
    /// When passing an owned value, it will be wrapped in an `Arc` automatically.
    /// When passing an `Arc`, it is used directly (enabling schedule sharing between contexts).
    ///
    /// # Parameters
    /// - `schedule`: a [`DividendSchedule`] or `Arc<DividendSchedule>` built via its builder
    pub fn insert_dividends(mut self, schedule: impl Into<Arc<DividendSchedule>>) -> Self {
        let arc_schedule = schedule.into();
        let id = arc_schedule.id.to_owned();
        self.dividends.insert(id, arc_schedule);
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

    /// Insert a scalar time series.
    ///
    /// # Parameters
    /// - `series`: [`ScalarTimeSeries`] to store
    pub fn insert_series(mut self, series: ScalarTimeSeries) -> Self {
        let id = series.id().to_owned();
        self.series.insert(id, series);
        self
    }

    /// Insert an inflation index.
    ///
    /// Accepts either an owned [`InflationIndex`] or an `Arc<InflationIndex>`.
    /// When passing an owned value, it will be wrapped in an `Arc` automatically.
    /// When passing an `Arc`, it is used directly (enabling index sharing between contexts).
    ///
    /// # Parameters
    /// - `id`: identifier stored as [`CurveId`]
    /// - `index`: an [`InflationIndex`] or `Arc<InflationIndex>`
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::scalars::{InflationIndex, InflationInterpolation};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use std::sync::Arc;
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
    ///
    /// // With Arc for sharing
    /// # let observations2 = vec![
    /// #     (Date::from_calendar_date(2024, Month::January, 31).expect("Valid date"), 100.0),
    /// #     (Date::from_calendar_date(2024, Month::February, 29).expect("Valid date"), 101.0),
    /// # ];
    /// # let index2 = InflationIndex::new("EU-HICP", observations2, Currency::EUR)
    /// #     .expect("InflationIndex creation should succeed");
    /// let shared = Arc::new(index2);
    /// let ctx2 = MarketContext::new().insert_inflation_index("EU-HICP", Arc::clone(&shared));
    /// ```
    pub fn insert_inflation_index(
        mut self,
        id: impl AsRef<str>,
        index: impl Into<Arc<InflationIndex>>,
    ) -> Self {
        self.inflation_indices
            .insert(CurveId::from(id.as_ref()), index.into());
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
    /// use finstack_core::market_data::term_structures::{BaseCorrelationCurve, CreditIndexData, HazardCurve};
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

    /// Insert an FX matrix.
    ///
    /// Accepts either an owned [`FxMatrix`] or an `Arc<FxMatrix>`.
    /// When passing an owned value, it will be wrapped in an `Arc` automatically.
    /// When passing an `Arc`, it is used directly (enabling FX matrix sharing between contexts).
    ///
    /// # Parameters
    /// - `fx`: [`FxMatrix`] or `Arc<FxMatrix>` instance used for currency conversions
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
    /// // Owned value
    /// let fx = FxMatrix::new(Arc::new(StaticFx));
    /// let ctx = MarketContext::new().insert_fx(fx);
    /// assert!(ctx.fx().is_some());
    ///
    /// // Pre-wrapped Arc for sharing
    /// # struct StaticFx2;
    /// # impl FxProvider for StaticFx2 {
    /// #     fn rate(&self, _from: Currency, _to: Currency, _on: Date, _policy: FxConversionPolicy) -> finstack_core::Result<f64> { Ok(1.2) }
    /// # }
    /// let shared_fx = Arc::new(FxMatrix::new(Arc::new(StaticFx2)));
    /// let ctx2 = MarketContext::new().insert_fx(Arc::clone(&shared_fx));
    /// ```
    pub fn insert_fx(mut self, fx: impl Into<Arc<FxMatrix>>) -> Self {
        self.fx = Some(fx.into());
        self
    }

    /// Clear the FX matrix from this context.
    ///
    /// After calling this method, `ctx.fx()` will return `None`.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::money::fx::{FxMatrix, FxProvider, FxConversionPolicy};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use std::sync::Arc;
    ///
    /// struct StaticFx;
    /// impl FxProvider for StaticFx {
    ///     fn rate(&self, _: Currency, _: Currency, _: Date, _: FxConversionPolicy) -> finstack_core::Result<f64> { Ok(1.0) }
    /// }
    ///
    /// let fx = FxMatrix::new(Arc::new(StaticFx));
    /// let ctx = MarketContext::new().insert_fx(fx);
    /// assert!(ctx.fx().is_some());
    ///
    /// let ctx = ctx.clear_fx();
    /// assert!(ctx.fx().is_none());
    /// ```
    pub fn clear_fx(mut self) -> Self {
        self.fx = None;
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
    /// use finstack_core::market_data::term_structures::DiscountCurve;
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
}
