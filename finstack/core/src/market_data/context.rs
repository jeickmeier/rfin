//! Lightweight container aggregating all market data needed by valuations.
//!
//! `MarketContext` groups together curves, FX (`FxMatrix`), 2-D surfaces
//! (`VolSurface`) and generic prices/scalars so that pricing and
//! risk components have a single handle to query required inputs.
//!
//! The container is intentionally minimal and cloning it is cheap because it
//! stores `Arc` references and small maps.

extern crate alloc;
use alloc::sync::Arc;
use hashbrown::HashMap;

use crate::money::fx::FxMatrix;

use super::{
    inflation::InflationCurve,
    inflation_index::InflationIndex,
    credit_index::CreditIndexData,
    primitives::{MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
    term_structures::base_correlation::BaseCorrelationCurve,
    traits::{Discount, Forward, TermStructure},
};
use crate::dates::Date;
use crate::currency::Currency;
use crate::types::CurveId;
use crate::F;
use core::str::FromStr;
use strum::IntoEnumIterator;

// -----------------------------------------------------------------------------
// Bump Specification Types (Unified)
// -----------------------------------------------------------------------------

/// Mode of applying a bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpMode {
    /// Additive bump expressed in a normalized fractional form (e.g., 100bp = 0.01, 2% = 0.02).
    Additive,
    /// Multiplicative bump expressed as a factor (e.g., 1.1 = +10%, 0.9 = -10%).
    Multiplicative,
}

/// Units for the bump magnitude. These control normalization to fraction or factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpUnits {
    /// Basis points for rates/spreads (100bp = 0.01).
    RateBp,
    /// Percent units (2.0 = 2%).
    Percent,
    /// Direct fraction (0.02 = 2%).
    Fraction,
    /// Direct factor (1.10 = +10%). Only valid for Multiplicative mode.
    Factor,
}

/// Unified bump specification capturing mode, units, and value.
#[derive(Debug, Clone, Copy)]
pub struct BumpSpec {
    /// How the bump should be applied (additive vs multiplicative).
    pub mode: BumpMode,
    /// Units the value is expressed in, controlling normalization.
    pub units: BumpUnits,
    /// Raw magnitude provided by the caller (interpreted using `units`).
    pub value: F,
}

impl BumpSpec {
    /// Create an additive bump specified in basis points (e.g., 100.0 = 100bp = 1%).
    pub fn parallel_bp(bump_bp: F) -> Self {
        Self {
            mode: BumpMode::Additive,
            units: BumpUnits::RateBp,
            value: bump_bp,
        }
    }

    /// Create a multiplicative bump given as a factor (e.g., 1.1 = +10%).
    pub fn multiplier(factor: F) -> Self {
        Self {
            mode: BumpMode::Multiplicative,
            units: BumpUnits::Factor,
            value: factor,
        }
    }

    /// Create an additive spread shift in basis points for credit curves.
    pub fn spread_shift_bp(bump_bp: F) -> Self {
        Self::parallel_bp(bump_bp)
    }

    /// Create an additive inflation shift specified in percent (e.g., 2.0 = +2%).
    pub fn inflation_shift_pct(bump_pct: F) -> Self {
        Self {
            mode: BumpMode::Additive,
            units: BumpUnits::Percent,
            value: bump_pct,
        }
    }

    /// Create an additive correlation shift specified in percent (e.g., 10.0 = +10%).
    pub fn correlation_shift_pct(bump_pct: F) -> Self {
        Self {
            mode: BumpMode::Additive,
            units: BumpUnits::Percent,
            value: bump_pct,
        }
    }

    /// If additive, return the bump as a normalized fraction (e.g., 100bp -> 0.01, 2% -> 0.02).
    fn additive_fraction(&self) -> Option<F> {
        if self.mode != BumpMode::Additive {
            return None;
        }
        let frac = match self.units {
            BumpUnits::RateBp => self.value / 10_000.0,
            BumpUnits::Percent => self.value / 100.0,
            BumpUnits::Fraction => self.value,
            BumpUnits::Factor => return None,
        };
        Some(frac)
    }

    /// Return a multiplicative factor for scaling (e.g., 1.1 for +10%).
    /// For additive specs, this returns 1.0 + additive_fraction.
    fn multiplier_value(&self) -> F {
        match self.mode {
            BumpMode::Multiplicative => self.value,
            BumpMode::Additive => 1.0 + self.additive_fraction().unwrap_or(0.0),
        }
    }
}

// -----------------------------------------------------------------------------
// Wrapper Curves for Bumping
// -----------------------------------------------------------------------------

/// Wrapper for a discount curve with a parallel rate bump applied.
///
/// This applies the formula: df_bumped(t) = df_original(t) * exp(-bump * t)
/// where bump is in rate units (e.g., 0.0001 for 1bp).
struct BumpedDiscountCurve {
    original: Arc<dyn Discount + Send + Sync>,
    bump_rate: F,
    bumped_id: CurveId,
}

impl BumpedDiscountCurve {
    fn new(original: Arc<dyn Discount + Send + Sync>, bump_bp: F, bumped_id: CurveId) -> Self {
        Self {
            original,
            bump_rate: bump_bp / 10_000.0, // Convert bp to rate
            bumped_id,
        }
    }
}

impl TermStructure for BumpedDiscountCurve {
    fn id(&self) -> &CurveId {
        &self.bumped_id
    }
}

impl Discount for BumpedDiscountCurve {
    #[inline]
    fn base_date(&self) -> Date {
        self.original.base_date()
    }

    #[inline]
    fn df(&self, t: F) -> F {
        let original_df = self.original.df(t);
        original_df * (-self.bump_rate * t).exp()
    }
}

/// Wrapper for a forward curve with a parallel rate bump applied.
struct BumpedForwardCurve {
    original: Arc<dyn Forward + Send + Sync>,
    bump_rate: F,
    bumped_id: CurveId,
}

impl BumpedForwardCurve {
    fn new(original: Arc<dyn Forward + Send + Sync>, bump_bp: F, bumped_id: CurveId) -> Self {
        Self {
            original,
            bump_rate: bump_bp / 10_000.0, // Convert bp to rate
            bumped_id,
        }
    }
}

impl TermStructure for BumpedForwardCurve {
    fn id(&self) -> &CurveId {
        &self.bumped_id
    }
}

impl Forward for BumpedForwardCurve {
    #[inline]
    fn rate(&self, t: F) -> F {
        self.original.rate(t) + self.bump_rate
    }
}

/// Create a bumped copy of a VolSurface by constructing a new one from bumped data.
fn create_bumped_vol_surface(
    original: &VolSurface,
    bump_spec: &BumpSpec,
    bumped_id: CurveId,
) -> crate::Result<VolSurface> {
    let bump_factor = bump_spec.multiplier_value();

    let expiries = original.expiries();
    let strikes = original.strikes();
    let mut builder = VolSurface::builder(bumped_id.as_str())
        .expiries(expiries)
        .strikes(strikes);

    // Apply bump to each volatility value
    for &expiry in expiries {
        let mut row = Vec::new();
        for &strike in strikes {
            let original_vol = original.value(expiry, strike);
            let bumped_vol = original_vol * bump_factor;
            row.push(bumped_vol);
        }
        builder = builder.row(&row);
    }

    builder.build()
}

// -----------------------------------------------------------------------------
// ID/Description formatting helpers (private)
// -----------------------------------------------------------------------------

#[inline]
fn id_bump_bp(id: &str, bp: F) -> CurveId {
    CurveId::new(format!("{}_bump_{:.0}bp", id, bp))
}

#[inline]
fn id_spread_bp(id: &str, bp: F) -> CurveId {
    CurveId::new(format!("{}_spread_{:.0}bp", id, bp))
}

#[inline]
fn id_infl_pct(id: &str, pct: F) -> CurveId {
    CurveId::new(format!("{}_infl_{:.1}pct", id, pct))
}

#[inline]
fn id_corr_pct(id: &str, pct: F) -> CurveId {
    CurveId::new(format!("{}_corr_{:.1}pct", id, pct))
}

#[inline]
fn desc_shift_bp(bp: F) -> alloc::string::String {
    format!("shift_{:.0}bp", bp)
}

#[inline]
fn desc_shift_pct(pct: F) -> alloc::string::String {
    format!("shift_{:.1}pct", pct)
}

#[inline]
fn desc_shift_fraction(frac: F) -> alloc::string::String {
    format!("shift_{:.6}", frac)
}

#[inline]
fn desc_mult_factor(factor: F) -> alloc::string::String {
    format!("mult_{:.2}", factor)
}

#[inline]
fn desc_infl_pct(pct: F) -> alloc::string::String {
    format!("infl_{:.1}pct", pct)
}

#[inline]
fn id_with_desc(id: &str, desc: &str) -> CurveId {
    CurveId::new(format!("{}_{}", id, desc))
}

// -----------------------------------------------------------------------------
// Market Context
// -----------------------------------------------------------------------------

/// Unified market data context
#[derive(Clone, Default)]
pub struct MarketContext {
    /// Discount curves keyed by identifier
    disc: HashMap<CurveId, Arc<dyn Discount + Send + Sync>>,
    /// Forecast curves keyed by identifier
    fwd: HashMap<CurveId, Arc<dyn Forward + Send + Sync>>,
    /// Hazard curves keyed by identifier
    hazard: HashMap<CurveId, Arc<crate::market_data::hazard_curve::HazardCurve>>,
    /// Inflation curves keyed by identifier
    inflation: HashMap<CurveId, Arc<InflationCurve>>,
    /// Inflation indices keyed by identifier
    inflation_indices: HashMap<CurveId, Arc<InflationIndex>>,
    /// Base correlation curves keyed by identifier
    base_correlation: HashMap<CurveId, Arc<BaseCorrelationCurve>>,
    /// Credit index aggregates keyed by index identifier (e.g., "CDX.NA.IG.42")
    credit_indices: HashMap<CurveId, Arc<CreditIndexData>>,
    /// Foreign-exchange matrix used for explicit FX conversions
    pub fx: Option<Arc<FxMatrix>>,
    /// Volatility surfaces keyed by identifier
    pub surfaces: HashMap<CurveId, Arc<VolSurface>>,
    /// Ad-hoc prices and constants
    pub prices: HashMap<CurveId, MarketScalar>,
    /// Generic date-indexed series
    pub series: HashMap<CurveId, ScalarTimeSeries>,
    /// Collateral CSA code → discount curve id mapping
    collat: HashMap<&'static str, CurveId>,
}

impl MarketContext {
    /// Create an empty context.
    pub fn new() -> Self {
        Self {
            disc: HashMap::new(),
            fwd: HashMap::new(),
            hazard: HashMap::new(),
            inflation: HashMap::new(),
            inflation_indices: HashMap::new(),
            base_correlation: HashMap::new(),
            credit_indices: HashMap::new(),
            fx: None,
            surfaces: HashMap::new(),
            prices: HashMap::new(),
            series: HashMap::new(),
            collat: HashMap::new(),
        }
    }

    /// Insert volatility surface.
    pub fn insert_surface(mut self, surface: VolSurface) -> Self {
        let id = crate::market_data::traits::TermStructure::id(&surface).clone();
        self.surfaces.insert(id, Arc::new(surface));
        self
    }

    /// Insert market scalar (price/constant) by id.
    pub fn insert_price(mut self, id: impl AsRef<str>, price: MarketScalar) -> Self {
        self.prices.insert(CurveId::from(id.as_ref()), price);
        self
    }

    /// Insert scalar time series.
    pub fn insert_series(mut self, series: ScalarTimeSeries) -> Self {
        let id = series.id().clone();
        self.series.insert(id, series);
        self
    }

    /// Insert discount curve.
    pub fn insert_discount<C: Discount + Send + Sync + 'static>(mut self, curve: C) -> Self {
        let cid = TermStructure::id(&curve).clone();
        self.disc.insert(cid, Arc::new(curve));
        self
    }

    /// Insert forward curve.
    pub fn insert_forward<C: Forward + Send + Sync + 'static>(mut self, curve: C) -> Self {
        let cid = TermStructure::id(&curve).clone();
        self.fwd.insert(cid, Arc::new(curve));
        self
    }

    /// Insert hazard curve.
    pub fn insert_hazard(mut self, curve: crate::market_data::hazard_curve::HazardCurve) -> Self {
        let cid = TermStructure::id(&curve).clone();
        self.hazard.insert(cid, Arc::new(curve));
        self
    }

    /// Insert inflation curve.
    pub fn insert_inflation(mut self, curve: InflationCurve) -> Self {
        let cid = TermStructure::id(&curve).clone();
        self.inflation.insert(cid, Arc::new(curve));
        self
    }

    /// Insert base correlation curve.
    pub fn insert_base_correlation(mut self, curve: BaseCorrelationCurve) -> Self {
        let cid = TermStructure::id(&curve).clone();
        self.base_correlation.insert(cid, Arc::new(curve));
        self
    }

    /// Insert inflation index.
    pub fn insert_inflation_index(self, id: impl AsRef<str>, index: InflationIndex) -> Self {
        let mut this = self;
        let cid = CurveId::from(id.as_ref());
        this.inflation_indices.insert(cid, Arc::new(index));
        this
    }

    /// Insert credit index aggregate data.
    pub fn insert_credit_index(mut self, id: impl AsRef<str>, data: CreditIndexData) -> Self {
        let cid = CurveId::from(id.as_ref());
        self.credit_indices.insert(cid, Arc::new(data));
        self
    }

    /// Insert FX matrix.
    pub fn insert_fx(mut self, fx: FxMatrix) -> Self {
        self.fx = Some(Arc::new(fx));
        self
    }

    /// Map collateral CSA code to discount curve id.
    pub fn map_collateral(mut self, csa_code: &'static str, disc_id: CurveId) -> Self {
        self.collat.insert(csa_code, disc_id);
        self
    }
}

// Note: we intentionally do not provide a blanket `From<MarketContext<Q>> for MarketContext<P>`
// because it conflicts with the standard library's `impl<T> From<T> for T`.

impl MarketContext {
    // New consolidated getter API with short names
    /// Get discount curve by id.
    pub fn disc(&self, id: impl AsRef<str>) -> crate::Result<Arc<dyn Discount + Send + Sync>> {
        let id_str = id.as_ref();
        self.disc.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get forward curve by id.
    pub fn fwd(&self, id: impl AsRef<str>) -> crate::Result<Arc<dyn Forward + Send + Sync>> {
        let id_str = id.as_ref();
        self.fwd.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get hazard curve by id.
    pub fn hazard(
        &self,
        id: impl AsRef<str>,
    ) -> crate::Result<Arc<crate::market_data::hazard_curve::HazardCurve>> {
        let id_str = id.as_ref();
        self.hazard.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get inflation curve by id.
    pub fn infl(&self, id: impl AsRef<str>) -> crate::Result<Arc<InflationCurve>> {
        let id_str = id.as_ref();
        self.inflation.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get volatility surface by id.
    pub fn surface(&self, id: impl AsRef<str>) -> crate::Result<Arc<VolSurface>> {
        let id_str = id.as_ref();
        self.surfaces.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get market scalar (price/constant) by id.
    pub fn price(&self, id: impl AsRef<str>) -> crate::Result<&MarketScalar> {
        let id_str = id.as_ref();
        self.prices.get(id_str).ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get scalar time series by id.
    pub fn series(&self, id: impl AsRef<str>) -> crate::Result<&ScalarTimeSeries> {
        let id_str = id.as_ref();
        self.series.get(id_str).ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get base correlation curve by id.
    pub fn base_correlation(
        &self,
        id: impl AsRef<str>,
    ) -> crate::Result<Arc<BaseCorrelationCurve>> {
        let id_str = id.as_ref();
        self.base_correlation.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get inflation index by id.
    pub fn inflation_index(&self, id: impl AsRef<str>) -> Option<Arc<InflationIndex>> {
        self.inflation_indices.get(id.as_ref()).cloned()
    }

    /// Get credit index data by identifier.
    pub fn credit_index(&self, id: impl AsRef<str>) -> crate::Result<Arc<CreditIndexData>> {
        let id_str = id.as_ref();
        self.credit_indices.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Resolve collateral discount curve for CSA code.
    pub fn collateral(&self, csa_code: &str) -> crate::Result<Arc<dyn Discount + Send + Sync>> {
        let id = match self.collat.get(csa_code) {
            Some(cid) => cid,
            None => {
                return Err(crate::error::InputError::NotFound {
                    id: format!("collateral:{}", csa_code),
                }
                .into())
            }
        };
        self.disc(id.as_str())
    }

    // -----------------------------------------------------------------------------
    // Scenario Analysis and Stress Testing
    // -----------------------------------------------------------------------------

    /// Apply one or more bumps to the market context in a single call.
    ///
    /// This consolidated API supports discount/forward/hazard/inflation/base-correlation
    /// curves, volatility surfaces, and market scalars.
    ///
    /// # Single-asset example
    /// ```rust
    /// # use hashbrown::HashMap;
    /// # use finstack_core::market_data::context::{MarketContext, BumpSpec};
    /// # use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// # use finstack_core::dates::Date;
    /// # use finstack_core::types::CurveId;
    /// # let curve = DiscountCurve::builder("USD-OIS")
    /// #     .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
    /// #     .knots([(0.0, 1.0), (5.0, 0.9)])
    /// #     .build().unwrap();
    /// # let context = MarketContext::new().insert_discount(curve);
    /// let mut bumps = HashMap::new();
    /// bumps.insert(CurveId::new("USD-OIS"), BumpSpec::parallel_bp(100.0));
    /// let bumped = context.bump(bumps).unwrap();
    /// assert!(bumped.disc("USD-OIS_bump_100bp").is_ok());
    /// ```
    ///
    /// # Multi-asset example
    /// ```rust
    /// # use hashbrown::HashMap;
    /// # use finstack_core::market_data::context::{MarketContext, BumpSpec};
    /// # use finstack_core::market_data::primitives::MarketScalar;
    /// # use finstack_core::types::CurveId;
    /// # let context = MarketContext::new()
    /// #     .insert_price("USD-OIS", MarketScalar::Unitless(0.05))
    /// #     .insert_price("USD-SOFR", MarketScalar::Unitless(0.052));
    /// let mut bumps = HashMap::new();
    /// bumps.insert(CurveId::new("USD-OIS"), BumpSpec::parallel_bp(100.0));
    /// bumps.insert(CurveId::new("USD-SOFR"), BumpSpec::parallel_bp(50.0));
    /// let bumped_context = context.bump(bumps).unwrap();
    /// ```
    pub fn bump(&self, bumps: HashMap<CurveId, BumpSpec>) -> crate::Result<Self> {
        let mut new_context = self.clone();

        for (curve_id, bump_spec) in bumps {
            let curve_id_str = curve_id.as_str();

            // Try each curve type until we find a match
            if let Ok(original) = self.disc(curve_id_str) {
                if bump_spec.mode == BumpMode::Additive && bump_spec.units == BumpUnits::RateBp {
                    let bump_bp = bump_spec.value;
                    let bumped_id = id_bump_bp(curve_id_str, bump_bp);
                    let bumped_curve = BumpedDiscountCurve::new(original, bump_bp, bumped_id.clone());
                    new_context.disc.insert(bumped_id, Arc::new(bumped_curve));
                }
            } else if let Ok(original) = self.fwd(curve_id_str) {
                if bump_spec.mode == BumpMode::Additive && bump_spec.units == BumpUnits::RateBp {
                    let bump_bp = bump_spec.value;
                    let bumped_id = id_bump_bp(curve_id_str, bump_bp);
                    let bumped_curve = BumpedForwardCurve::new(original, bump_bp, bumped_id.clone());
                    new_context.fwd.insert(bumped_id, Arc::new(bumped_curve));
                }
            } else if let Ok(original) = self.hazard(curve_id_str) {
                if bump_spec.mode == BumpMode::Additive && bump_spec.units == BumpUnits::RateBp {
                    let spread_rate = bump_spec.additive_fraction().unwrap_or(0.0);
                    if let Ok(bumped_curve) = original.with_hazard_shift(spread_rate) {
                        let bumped_id = id_spread_bp(curve_id_str, bump_spec.value);
                        new_context.hazard.insert(bumped_id, Arc::new(bumped_curve));
                    }
                }
            } else if let Ok(original) = self.infl(curve_id_str) {
                if bump_spec.mode == BumpMode::Additive && bump_spec.units == BumpUnits::Percent {
                    let multiplier = bump_spec.multiplier_value();
                    let original_knots = original.knots();
                    let original_cpi_levels = original.cpi_levels();

                    let bumped_points: Vec<(F, F)> = original_knots
                        .iter()
                        .zip(original_cpi_levels.iter())
                        .map(|(&t, &cpi)| (t, cpi * multiplier))
                        .collect();

                    let bumped_id = id_infl_pct(curve_id_str, bump_spec.value);
                    if let Ok(bumped_curve) = InflationCurve::builder("TEMP_BUMPED_INFLATION")
                        .base_cpi(original.base_cpi() * multiplier)
                        .knots(bumped_points)
                        .set_interp(crate::market_data::interp::InterpStyle::LogLinear)
                        .build()
                    {
                        new_context
                            .inflation
                            .insert(bumped_id, Arc::new(bumped_curve));
                    }
                }
            } else if let Ok(original) = self.base_correlation(curve_id_str) {
                if bump_spec.mode == BumpMode::Additive && bump_spec.units == BumpUnits::Percent {
                    let multiplier = bump_spec.multiplier_value();
                    let original_points = original.detachment_points();
                    let original_correlations = original.correlations();

                    let bumped_points: Vec<(F, F)> = original_points
                        .iter()
                        .zip(original_correlations.iter())
                        .map(|(&detach, &corr)| (detach, (corr * multiplier).clamp(0.0, 1.0)))
                        .collect();

                    let bumped_id = id_corr_pct(curve_id_str, bump_spec.value);
                    if let Ok(bumped_curve) =
                        BaseCorrelationCurve::builder("TEMP_BUMPED_CORRELATION")
                            .points(bumped_points)
                            .build()
                    {
                        new_context
                            .base_correlation
                            .insert(bumped_id, Arc::new(bumped_curve));
                    }
                }
            } else if let Ok(original_index) = self.credit_index(curve_id_str) {
                // Support hazard spread bp and correlation percent bumps on aggregated credit index data
                if bump_spec.mode == BumpMode::Additive && bump_spec.units == BumpUnits::RateBp {
                    // Hazard spread bump at the index level
                    let spread_rate = bump_spec.additive_fraction().unwrap_or(0.0);
                    if let Ok(bumped_hazard) = original_index.index_credit_curve.with_hazard_shift(spread_rate) {
                        let mut builder = crate::market_data::credit_index::CreditIndexData::builder()
                            .num_constituents(original_index.num_constituents)
                            .recovery_rate(original_index.recovery_rate)
                            .index_credit_curve(Arc::new(bumped_hazard))
                            .base_correlation_curve(original_index.base_correlation_curve.clone());
                        if let Some(issuer_curves) = &original_index.issuer_credit_curves {
                            builder = builder.with_issuer_curves(issuer_curves.clone());
                        }
                        if let Ok(bumped_index) = builder.build() {
                            let bumped_id = id_spread_bp(curve_id_str, bump_spec.value);
                            new_context
                                .credit_indices
                                .insert(bumped_id, Arc::new(bumped_index));
                        }
                    }
                } else if bump_spec.mode == BumpMode::Additive
                    && bump_spec.units == BumpUnits::Percent
                {
                    // Base correlation percent bump at the index level
                    let multiplier = bump_spec.multiplier_value();
                    let original_points = original_index.base_correlation_curve.detachment_points();
                    let original_correlations = original_index.base_correlation_curve.correlations();

                    let bumped_points: Vec<(F, F)> = original_points
                        .iter()
                        .zip(original_correlations.iter())
                        .map(|(&detach, &corr)| (detach, (corr * multiplier).clamp(0.0, 1.0)))
                        .collect();

                    if let Ok(bumped_bc) = BaseCorrelationCurve::builder("TEMP_BUMPED_CORRELATION")
                        .points(bumped_points)
                        .build()
                    {
                        let mut builder = crate::market_data::credit_index::CreditIndexData::builder()
                            .num_constituents(original_index.num_constituents)
                            .recovery_rate(original_index.recovery_rate)
                            .index_credit_curve(original_index.index_credit_curve.clone())
                            .base_correlation_curve(Arc::new(bumped_bc));
                        if let Some(issuer_curves) = &original_index.issuer_credit_curves {
                            builder = builder.with_issuer_curves(issuer_curves.clone());
                        }
                        if let Ok(bumped_index) = builder.build() {
                            let bumped_id = id_corr_pct(curve_id_str, bump_spec.value);
                            new_context
                                .credit_indices
                                .insert(bumped_id, Arc::new(bumped_index));
                        }
                    }
                }
            } else if let Ok(original) = self.surface(curve_id_str) {
                // Support additive bp or multiplicative factor for surfaces
                let bump_desc = if bump_spec.mode == BumpMode::Additive
                    && bump_spec.units == BumpUnits::RateBp
                {
                    Some(desc_shift_bp(bump_spec.value))
                } else if bump_spec.mode == BumpMode::Multiplicative
                    && bump_spec.units == BumpUnits::Factor
                {
                    Some(desc_mult_factor(bump_spec.value))
                } else {
                    None
                };
                if let Some(desc) = bump_desc {
                    let bumped_id = id_with_desc(curve_id_str, &desc);
                    if let Ok(bumped_surface) =
                        create_bumped_vol_surface(&original, &bump_spec, bumped_id.clone())
                    {
                        new_context
                            .surfaces
                            .insert(bumped_id, Arc::new(bumped_surface));
                    }
                }
            } else if let Ok(original) = self.price(curve_id_str) {
                let bump_desc = if bump_spec.mode == BumpMode::Multiplicative
                    && bump_spec.units == BumpUnits::Factor
                {
                    desc_mult_factor(bump_spec.value)
                } else if bump_spec.mode == BumpMode::Additive {
                    match bump_spec.units {
                        BumpUnits::RateBp => desc_shift_bp(bump_spec.value),
                        BumpUnits::Percent => desc_shift_pct(bump_spec.value),
                        BumpUnits::Fraction => desc_shift_fraction(bump_spec.value),
                        BumpUnits::Factor => unreachable!(),
                    }
                } else {
                    // Default fallback
                    "unknown".to_string()
                };

                let bumped_value = match original {
                    MarketScalar::Unitless(val) => match bump_spec.mode {
                        BumpMode::Additive => {
                            MarketScalar::Unitless(val + bump_spec.additive_fraction().unwrap_or(0.0))
                        }
                        BumpMode::Multiplicative => {
                            MarketScalar::Unitless(val * bump_spec.multiplier_value())
                        }
                    },
                    MarketScalar::Price(money) => match bump_spec.mode {
                        BumpMode::Additive => {
                            let factor = 1.0 + bump_spec.additive_fraction().unwrap_or(0.0);
                            let new_amount = money.amount() * factor;
                            MarketScalar::Price(crate::money::Money::new(new_amount, money.currency()))
                        }
                        BumpMode::Multiplicative => {
                            let new_amount = money.amount() * bump_spec.multiplier_value();
                            MarketScalar::Price(crate::money::Money::new(new_amount, money.currency()))
                        }
                    },
                };
                let bumped_id = id_with_desc(curve_id_str, &bump_desc);
                new_context.prices.insert(bumped_id, bumped_value);
            } else if let Some(index) = self.inflation_index(curve_id_str) {
                // Bump inflation index time series values
                let bump_desc = if bump_spec.mode == BumpMode::Multiplicative
                    && bump_spec.units == BumpUnits::Factor
                {
                    desc_mult_factor(bump_spec.value)
                } else if bump_spec.mode == BumpMode::Additive {
                    match bump_spec.units {
                        BumpUnits::Percent => desc_infl_pct(bump_spec.value),
                        BumpUnits::RateBp => desc_shift_bp(bump_spec.value),
                        BumpUnits::Fraction => desc_shift_fraction(bump_spec.value),
                        BumpUnits::Factor => unreachable!(),
                    }
                } else {
                    "unknown".to_string()
                };

                let multiplier = match bump_spec.mode {
                    BumpMode::Multiplicative => bump_spec.multiplier_value(),
                    BumpMode::Additive => 1.0 + bump_spec.additive_fraction().unwrap_or(0.0),
                };

                // Reconstruct observations and scale (borrow DataFrame; avoid clone)
                let df = index.as_dataframe();
                let dates = df
                    .column("date")
                    .map_err(|_| crate::Error::Internal)?
                    .i32()
                    .map_err(|_| crate::Error::Internal)?;
                let values = df
                    .column("value")
                    .map_err(|_| crate::Error::Internal)?
                    .f64()
                    .map_err(|_| crate::Error::Internal)?;
                let bumped_obs: Vec<(Date, F)> = dates
                    .into_no_null_iter()
                    .zip(values.into_no_null_iter())
                    .map(|(d, v)| {
                        (
                            crate::dates::utils::days_since_epoch_to_date(d),
                            v * multiplier,
                        )
                    })
                    .collect();

                let bumped_cid = id_with_desc(curve_id_str, &bump_desc);
                let builder = crate::market_data::inflation_index::InflationIndexBuilder::new(
                    bumped_cid.as_str(),
                    index.currency,
                )
                .with_observations(bumped_obs)
                .with_interpolation(index.interpolation())
                .with_lag(index.lag());

                if let Ok(bumped_index) = builder.build() {
                    new_context
                        .inflation_indices
                        .insert(bumped_cid, Arc::new(bumped_index));
                }
            } else if let Ok(series) = self.series(curve_id_str) {
                // Bump generic scalar time series
                let bump_desc = if bump_spec.mode == BumpMode::Multiplicative
                    && bump_spec.units == BumpUnits::Factor
                {
                    desc_mult_factor(bump_spec.value)
                } else if bump_spec.mode == BumpMode::Additive {
                    match bump_spec.units {
                        BumpUnits::RateBp => desc_shift_bp(bump_spec.value),
                        BumpUnits::Percent => desc_shift_pct(bump_spec.value),
                        BumpUnits::Fraction => desc_shift_fraction(bump_spec.value),
                        BumpUnits::Factor => unreachable!(),
                    }
                } else {
                    "unknown".to_string()
                };

                let (is_add, add_frac) = match bump_spec.mode {
                    BumpMode::Additive => (true, bump_spec.additive_fraction().unwrap_or(0.0)),
                    BumpMode::Multiplicative => (false, 0.0),
                };
                let mult = if is_add { 1.0 } else { bump_spec.multiplier_value() };

                let df = series.as_dataframe();
                let dates = df
                    .column("date")
                    .map_err(|_| crate::Error::Internal)?
                    .i32()
                    .map_err(|_| crate::Error::Internal)?;
                let values = df
                    .column("value")
                    .map_err(|_| crate::Error::Internal)?
                    .f64()
                    .map_err(|_| crate::Error::Internal)?;
                let bumped_obs: Vec<(Date, F)> = dates
                    .into_no_null_iter()
                    .zip(values.into_no_null_iter())
                    .map(|(d, v)| {
                        let scaled = if is_add { v + add_frac } else { v * mult };
                        (crate::dates::utils::days_since_epoch_to_date(d), scaled)
                    })
                    .collect();

                let bumped_cid = id_with_desc(curve_id_str, &bump_desc);
                let mut bumped_series = ScalarTimeSeries::new(
                    bumped_cid.as_str(),
                    bumped_obs,
                    series.currency(),
                )?;
                bumped_series = bumped_series.with_interpolation(series.interpolation());
                new_context
                    .series
                    .insert(bumped_cid, bumped_series);
            } else if let Some(fx) = &self.fx {
                // FX base-currency relative bump: curve_id must be a currency code (e.g., "USD")
                if let Ok(base_ccy) = Currency::from_str(curve_id_str) {
                    // Determine factor: Multiplicative factor preferred; additive percent supported
                    let factor = match bump_spec.mode {
                        BumpMode::Multiplicative => bump_spec.multiplier_value(),
                        BumpMode::Additive => 1.0 + bump_spec.additive_fraction().unwrap_or(0.0),
                    };

                    // Minimal provider that forces triangulation/use of seeded quotes only
                    struct StaticFxProvider;
                    impl crate::money::fx::FxProvider for StaticFxProvider {
                        fn rate(
                            &self,
                            _from: crate::currency::Currency,
                            _to: crate::currency::Currency,
                            _on: crate::dates::Date,
                            _policy: crate::money::fx::FxConversionPolicy,
                        ) -> crate::Result<crate::money::fx::FxRate> {
                            Err(crate::Error::Internal)
                        }
                    }

                    // Build a new FX matrix with pivot set to base_ccy
                    let cfg = crate::money::fx::FxConfig {
                        pivot_currency: base_ccy,
                        ..Default::default()
                    };
                    let bumped_fx = crate::money::fx::FxMatrix::with_config(Arc::new(StaticFxProvider), cfg);

                    // Use a neutral date; quotes are cached without date
                    let on = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
                    let policy = crate::money::fx::FxConversionPolicy::CashflowDate;

                    // Seed adjusted quotes for base_ccy vs all currencies
                    for other in crate::currency::Currency::iter() {
                        if other == base_ccy { continue; }
                        // base -> other
                        if let Ok(rate_bo) = fx.rate(crate::money::fx::FxQuery { from: base_ccy, to: other, on, policy, closure_check: None, want_meta: false }) {
                            let new_rate = rate_bo.rate * factor;
                            bumped_fx.set_quote(base_ccy, other, new_rate);
                        }
                        // other -> base (reciprocal adjusted)
                        if let Ok(rate_ob) = fx.rate(crate::money::fx::FxQuery { from: other, to: base_ccy, on, policy, closure_check: None, want_meta: false }) {
                            let new_rate = rate_ob.rate / factor;
                            bumped_fx.set_quote(other, base_ccy, new_rate);
                        }
                    }

                    let mut bumped_ctx = new_context.clone();
                    bumped_ctx.fx = Some(Arc::new(bumped_fx));
                    new_context = bumped_ctx;
                } else {
                    return Err(crate::error::InputError::NotFound {
                        id: curve_id_str.to_string(),
                    }
                    .into());
                }
            } else {
                return Err(crate::error::InputError::NotFound {
                    id: curve_id_str.to_string(),
                }
                .into());
            }
        }

        Ok(new_context)
    }

    // -----------------------------------------------------------------------------
    // Forward Price/Rate Calculators
    // -----------------------------------------------------------------------------

    /// Build forward function for equity underlyings: F(t) = S₀ × exp((r - q) × t)
    ///
    /// # Arguments
    /// * `underlying` - Identifier for the underlying asset (e.g., "SPY", "AAPL")
    /// * `base_currency` - Base currency for the risk-free rate
    ///
    /// # Returns
    /// A closure that calculates forward prices for given time `t` (in years)
    pub fn equity_forward<'a>(
        &'a self,
        underlying: &str,
        base_currency: crate::currency::Currency,
    ) -> crate::Result<Box<dyn Fn(crate::F) -> crate::F + 'a>> {
        // Get spot price
        let spot_scalar = self.price(underlying)?;
        let spot = match spot_scalar {
            crate::market_data::primitives::MarketScalar::Price(money) => money.amount(),
            crate::market_data::primitives::MarketScalar::Unitless(value) => *value,
        };

        // Get dividend yield (default to 0.0 if not available)
        let div_yield_key = format!("{}-DIVYIELD", underlying);
        let dividend_yield = self
            .price(&div_yield_key)
            .map(|scalar| match scalar {
                crate::market_data::primitives::MarketScalar::Unitless(yield_val) => *yield_val,
                _ => 0.0,
            })
            .unwrap_or(0.0);

        // Get risk-free rate from discount curve
        let disc_curve_id = format!("{}-OIS", base_currency);
        let discount_curve = self.disc(&disc_curve_id)?;

        Ok(Box::new(move |t: crate::F| -> crate::F {
            let risk_free_rate = discount_curve.zero(t);
            spot * ((risk_free_rate - dividend_yield) * t).exp()
        }))
    }

    /// Build forward function for FX underlyings: F(t) = S₀ × exp((r_dom - r_for) × t)
    ///
    /// # Arguments
    /// * `underlying` - FX pair identifier (6-char format like "EURUSD")
    ///
    /// # Returns
    /// A closure that calculates forward FX rates for given time `t` (in years)
    pub fn fx_forward<'a>(
        &'a self,
        underlying: &str,
    ) -> crate::Result<Box<dyn Fn(crate::F) -> crate::F + 'a>> {
        // Parse FX pair (assume 6-char format like "EURUSD")
        if underlying.len() != 6 {
            return Err(crate::error::InputError::Invalid.into());
        }

        let foreign_ccy = &underlying[0..3];
        let domestic_ccy = &underlying[3..6];

        // Get spot rate
        let spot_scalar = self.price(underlying)?;
        let spot = match spot_scalar {
            crate::market_data::primitives::MarketScalar::Price(money) => money.amount(),
            crate::market_data::primitives::MarketScalar::Unitless(value) => *value,
        };

        // Get domestic and foreign discount curves
        let dom_disc_id = format!("{}-OIS", domestic_ccy);
        let for_disc_id = format!("{}-OIS", foreign_ccy);
        let dom_curve = self.disc(&dom_disc_id)?;
        let for_curve = self.disc(&for_disc_id)?;

        Ok(Box::new(move |t: crate::F| -> crate::F {
            let domestic_rate = dom_curve.zero(t);
            let foreign_rate = for_curve.zero(t);
            spot * ((domestic_rate - foreign_rate) * t).exp()
        }))
    }

    /// Build forward function for interest rate underlyings: F(t) = forward_curve.rate(t)
    ///
    /// # Arguments
    /// * `underlying` - Forward curve identifier (e.g., "USD-SOFR3M")
    ///
    /// # Returns
    /// A closure that returns forward rates for given time `t` (in years)
    pub fn rates_forward<'a>(
        &'a self,
        underlying: &str,
    ) -> crate::Result<Box<dyn Fn(crate::F) -> crate::F + 'a>> {
        // Get forward curve for this index
        let forward_curve = self.fwd(underlying)?;

        Ok(Box::new(move |t: crate::F| -> crate::F { forward_curve.rate(t) }))
    }

    /// Auto-detect asset class and build appropriate forward function.
    ///
    /// Determines asset class from underlying identifier and constructs
    /// appropriate forward calculation using market data.
    ///
    /// # Arguments
    /// * `underlying` - Asset identifier to detect and build forward for
    /// * `base_currency` - Base currency for equity forward calculations
    ///
    /// # Returns
    /// A closure that calculates appropriate forward values for given time `t`
    pub fn auto_forward<'a>(
        &'a self,
        underlying: &str,
        base_currency: crate::currency::Currency,
    ) -> crate::Result<Box<dyn Fn(crate::F) -> crate::F + 'a>> {
        // Detect asset class from underlying identifier
        if underlying.contains("-")
            && (underlying.contains("SOFR")
                || underlying.contains("EURIBOR")
                || underlying.contains("SONIA"))
        {
            // Interest rate underlying (e.g., "USD-SOFR3M", "EUR-EURIBOR3M")
            self.rates_forward(underlying)
        } else if underlying.len() == 6 && underlying.chars().all(|c| c.is_ascii_alphabetic()) {
            // FX pair (e.g., "EURUSD", "GBPJPY")
            self.fx_forward(underlying)
        } else {
            // Equity underlying (e.g., "SPY", "AAPL")
            self.equity_forward(underlying, base_currency)
        }
    }
}

// -----------------------------------------------------------------------------
// Tests for Bumping Functionality
// -----------------------------------------------------------------------------
#[cfg(test)]
mod bump_tests {
    use super::*;
    use crate::market_data::interp::InterpStyle;
    use crate::market_data::surfaces::vol_surface::VolSurface;
    use crate::market_data::term_structures::{
        base_correlation::BaseCorrelationCurve, discount_curve::DiscountCurve,
        forward_curve::ForwardCurve, hazard_curve::HazardCurve, inflation::InflationCurve,
    };

    fn test_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap()
    }

    fn test_forward_curve() -> ForwardCurve {
        ForwardCurve::builder("USD-SOFR3M", 0.25)
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots([(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap()
    }

    fn test_vol_surface() -> VolSurface {
        VolSurface::builder("USD-ATM-VOL")
            .expiries(&[0.25, 1.0])
            .strikes(&[90.0, 100.0])
            .row(&[0.20, 0.22])
            .row(&[0.18, 0.19])
            .build()
            .unwrap()
    }

    fn test_hazard_curve() -> HazardCurve {
        HazardCurve::builder("CORP-HAZARD")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .recovery_rate(0.4)
            .knots([(0.0, 0.01), (1.0, 0.015), (5.0, 0.02)])
            .build()
            .unwrap()
    }

    fn test_inflation_curve() -> InflationCurve {
        InflationCurve::builder("US-CPI")
            .base_cpi(300.0)
            .knots([(0.0, 300.0), (1.0, 303.0), (5.0, 315.0)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .unwrap()
    }

    fn test_base_correlation_curve() -> BaseCorrelationCurve {
        BaseCorrelationCurve::builder("CDX-NA-IG")
            .points(vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)])
            .build()
            .unwrap()
    }

    #[test]
    fn test_discount_curve_bump() {
        let curve = test_discount_curve();
        let context = MarketContext::new().insert_discount(curve);

        // Test original curve values
        let original = context.disc("USD-OIS").unwrap();
        let original_df_1y = original.df(1.0);
        let original_zero_1y = original.zero(1.0);

        // Apply 100bp bump
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("USD-OIS"), BumpSpec::parallel_bp(100.0));
        let bumped_context = context.bump(bumps).unwrap();
        let bumped_id = "USD-OIS_bump_100bp";
        let bumped = bumped_context.disc(bumped_id).unwrap();

        // Bumped discount factor should be lower (higher rates)
        let bumped_df_1y = bumped.df(1.0);
        let bumped_zero_1y = bumped.zero(1.0);

        assert!(bumped_df_1y < original_df_1y, "Bumped DF should be lower");
        assert!(
            bumped_zero_1y > original_zero_1y,
            "Bumped zero rate should be higher"
        );

        // Check the mathematical relationship: df_bumped = df_original * exp(-0.01 * 1.0)
        let expected_df = original_df_1y * (-0.01_f64).exp();
        assert!(
            (bumped_df_1y - expected_df).abs() < 1e-12,
            "DF bump formula should be precise"
        );
    }

    #[test]
    fn test_forward_curve_bump() {
        let curve = test_forward_curve();
        let context = MarketContext::new().insert_forward(curve);

        let original = context.fwd("USD-SOFR3M").unwrap();
        let original_rate_1y = original.rate(1.0);

        // Apply 50bp bump
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("USD-SOFR3M"), BumpSpec::parallel_bp(50.0));
        let bumped_context = context.bump(bumps).unwrap();
        let bumped_id = "USD-SOFR3M_bump_50bp";
        let bumped = bumped_context.fwd(bumped_id).unwrap();

        let bumped_rate_1y = bumped.rate(1.0);

        // Forward rate should increase by exactly 50bp
        let expected_rate = original_rate_1y + 0.005; // 50bp = 0.005
        assert!(
            (bumped_rate_1y - expected_rate).abs() < 1e-12,
            "Forward bump should be additive"
        );
    }

    #[test]
    fn test_vol_surface_bump() {
        let surface = test_vol_surface();
        let context = MarketContext::new().insert_surface(surface);

        let original = context.surface("USD-ATM-VOL").unwrap();
        let original_vol = original.value(0.5, 95.0); // Use valid coordinates

        // Apply 10% multiplier shock
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("USD-ATM-VOL"), BumpSpec::multiplier(1.1));
        let bumped_context = context.bump(bumps).unwrap();
        let bumped_id = "USD-ATM-VOL_mult_1.10";
        let bumped = bumped_context.surface(bumped_id).unwrap();

        let bumped_vol = bumped.value(0.5, 95.0);
        let expected_vol = original_vol * 1.1;
        assert!(
            (bumped_vol - expected_vol).abs() < 1e-12,
            "Vol bump should be multiplicative"
        );
    }

    #[test]
    fn test_market_scalar_bump() {
        let context =
            MarketContext::new().insert_price("GOLD_SPOT", MarketScalar::Unitless(2000.0));

        let original = context.price("GOLD_SPOT").unwrap();

        // Apply additive bump
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("GOLD_SPOT"), BumpSpec::parallel_bp(500.0)); // 5% in bp terms
        let bumped_context = context.bump(bumps).unwrap();
        let bumped_id = "GOLD_SPOT_shift_500bp";
        let bumped = bumped_context.price(bumped_id).unwrap();

        if let (MarketScalar::Unitless(orig_val), MarketScalar::Unitless(bump_val)) =
            (original, bumped)
        {
            let expected = orig_val + 0.05; // 500bp = 0.05
            assert!(
                (bump_val - expected).abs() < 1e-12,
                "Scalar additive bump should be precise"
            );
        } else {
            panic!("Expected Unitless MarketScalar values");
        }

        // Apply multiplicative bump
        let mut bumps2 = hashbrown::HashMap::new();
        bumps2.insert(CurveId::new("GOLD_SPOT"), BumpSpec::multiplier(1.2));
        let mult_context = context.bump(bumps2).unwrap();
        let mult_id = "GOLD_SPOT_mult_1.20";
        let mult_bumped = mult_context.price(mult_id).unwrap();

        if let (MarketScalar::Unitless(orig_val), MarketScalar::Unitless(mult_val)) =
            (original, mult_bumped)
        {
            let expected_mult = orig_val * 1.2;
            assert!(
                (mult_val - expected_mult).abs() < 1e-12,
                "Scalar multiplicative bump should be precise"
            );
        } else {
            panic!("Expected Unitless MarketScalar values");
        }
    }

    #[test]
    fn test_parallel_rate_shock() {
        // replaced with consolidated bump API
        let disc_curve = test_discount_curve();
        let fwd_curve = test_forward_curve();
        let context = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_forward(fwd_curve);

        // Apply 200bp shock across both curves via bump
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("USD-OIS"), BumpSpec::parallel_bp(200.0));
        bumps.insert(CurveId::new("USD-SOFR3M"), BumpSpec::parallel_bp(200.0));
        let shocked_context = context.bump(bumps).unwrap();

        // Verify both curves were bumped
        let bumped_disc = shocked_context.disc("USD-OIS_bump_200bp").unwrap();
        let bumped_fwd = shocked_context.fwd("USD-SOFR3M_bump_200bp").unwrap();

        // Check that the bumped curves behave as expected
        let original_disc = context.disc("USD-OIS").unwrap();
        let original_fwd = context.fwd("USD-SOFR3M").unwrap();

        assert!(
            bumped_disc.df(1.0) < original_disc.df(1.0),
            "Bumped discount should be lower"
        );
        assert!(
            bumped_fwd.rate(1.0) > original_fwd.rate(1.0),
            "Bumped forward should be higher"
        );
    }

    #[test]
    fn test_volatility_shock() {
        // replaced with consolidated bump API
        let surface = test_vol_surface();
        let context = MarketContext::new().insert_surface(surface);

        // Apply 20% vol shock via bump
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("USD-ATM-VOL"), BumpSpec::multiplier(1.2));
        let shocked_context = context.bump(bumps).unwrap();

        let original = context.surface("USD-ATM-VOL").unwrap();
        let bumped = shocked_context.surface("USD-ATM-VOL_mult_1.20").unwrap();

        let original_vol = original.value(0.5, 95.0); // Use valid coordinates
        let bumped_vol = bumped.value(0.5, 95.0);

        assert!(
            (bumped_vol - original_vol * 1.2).abs() < 1e-12,
            "Vol shock should be multiplicative"
        );
    }

    #[test]
    fn test_multiple_bumps() {
        let disc_curve = test_discount_curve();
        let fwd_curve = test_forward_curve();
        let context = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_forward(fwd_curve)
            .insert_price("SPOT_PRICE", MarketScalar::Unitless(100.0));

        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("USD-OIS"), BumpSpec::parallel_bp(100.0));
        bumps.insert(CurveId::new("USD-SOFR3M"), BumpSpec::parallel_bp(-25.0));
        bumps.insert(CurveId::new("SPOT_PRICE"), BumpSpec::multiplier(1.15));

        let bumped_context = context.bump(bumps).unwrap();

        // Verify all bumps were applied
        assert!(bumped_context.disc("USD-OIS_bump_100bp").is_ok());
        assert!(bumped_context.fwd("USD-SOFR3M_bump_-25bp").is_ok());
        assert!(bumped_context.price("SPOT_PRICE_mult_1.15").is_ok());
    }

    #[test]
    fn test_hazard_curve_bump() {
        let curve = test_hazard_curve();
        let context = MarketContext::new().insert_hazard(curve);

        let original = context.hazard("CORP-HAZARD").unwrap();
        let original_sp_1y = original.sp(1.0);

        // Apply 100bp spread shift
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(
            CurveId::new("CORP-HAZARD"),
            BumpSpec::spread_shift_bp(100.0),
        );
        let bumped_context = context.bump(bumps).unwrap();
        let bumped_id = "CORP-HAZARD_spread_100bp";
        let bumped = bumped_context.hazard(bumped_id).unwrap();

        let bumped_sp_1y = bumped.sp(1.0);

        // Higher hazard rates should lead to lower survival probability
        assert!(
            bumped_sp_1y < original_sp_1y,
            "Bumped survival probability should be lower"
        );
    }

    #[test]
    fn test_inflation_curve_bump() {
        let curve = test_inflation_curve();
        let context = MarketContext::new().insert_inflation(curve);

        let original = context.infl("US-CPI").unwrap();
        let original_cpi_1y = original.cpi(1.0);
        let original_base_cpi = original.base_cpi();

        // Apply 2% inflation shock
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("US-CPI"), BumpSpec::inflation_shift_pct(2.0));
        let bumped_context = context.bump(bumps).unwrap();
        let bumped_id = "US-CPI_infl_2.0pct";
        let bumped = bumped_context.infl(bumped_id).unwrap();

        let bumped_cpi_1y = bumped.cpi(1.0);
        let bumped_base_cpi = bumped.base_cpi();

        // CPI levels should be scaled by 1.02
        let expected_cpi_1y = original_cpi_1y * 1.02;
        let expected_base_cpi = original_base_cpi * 1.02;

        assert!(
            (bumped_cpi_1y - expected_cpi_1y).abs() < 1e-10,
            "Inflation bump should scale CPI levels"
        );
        assert!(
            (bumped_base_cpi - expected_base_cpi).abs() < 1e-10,
            "Base CPI should be scaled"
        );
    }

    #[test]
    fn test_base_correlation_bump() {
        let curve = test_base_correlation_curve();
        let context = MarketContext::new().insert_base_correlation(curve);

        let original = context.base_correlation("CDX-NA-IG").unwrap();
        let original_corr_5pct = original.correlation(5.0);

        // Apply 10% correlation increase
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(
            CurveId::new("CDX-NA-IG"),
            BumpSpec::correlation_shift_pct(10.0),
        );
        let bumped_context = context.bump(bumps).unwrap();
        let bumped_id = "CDX-NA-IG_corr_10.0pct";
        let bumped = bumped_context.base_correlation(bumped_id).unwrap();

        let bumped_corr_5pct = bumped.correlation(5.0);

        // Correlation should increase by 10%
        let expected_corr = (original_corr_5pct * 1.1).clamp(0.0, 1.0);
        assert!(
            (bumped_corr_5pct - expected_corr).abs() < 1e-10,
            "Correlation bump should be multiplicative and clamped"
        );
    }

    #[test]
    fn test_inflation_index_bump() {
        use crate::currency::Currency;
        use time::Month;

        // Build a small CPI index
        let observations = vec![
            (
                Date::from_calendar_date(2025, Month::January, 31).unwrap(),
                300.0,
            ),
            (
                Date::from_calendar_date(2025, Month::February, 28).unwrap(),
                303.0,
            ),
        ];
        let index = crate::market_data::inflation_index::InflationIndex::new(
            "US-CPI",
            observations,
            Currency::USD,
        )
        .unwrap();

        let context = MarketContext::new().insert_inflation_index("US-CPI", index);

        // Baseline value
        let orig = context
            .inflation_index("US-CPI")
            .expect("existing index");
        let date = Date::from_calendar_date(2025, Month::February, 28).unwrap();
        let orig_val = orig.value_on(date).unwrap();

        // Apply +2% bump to the index
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("US-CPI"), BumpSpec::inflation_shift_pct(2.0));
        let bumped = context.bump(bumps).unwrap();

        let bumped_idx = bumped
            .inflation_index("US-CPI_infl_2.0pct")
            .expect("bumped index present");
        let bumped_val = bumped_idx.value_on(date).unwrap();

        let expected = orig_val * 1.02;
        assert!((bumped_val - expected).abs() < 1e-12);
    }

    #[test]
    fn test_scalar_time_series_bump() {
        use time::Month;

        let d0 = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let d1 = Date::from_calendar_date(2025, Month::February, 1).unwrap();
        let s = ScalarTimeSeries::new("SERIES_A", vec![(d0, 1.0), (d1, 2.0)], None).unwrap();

        let context = MarketContext::new().insert_series(s);

        // Baseline
        let orig = context.series("SERIES_A").unwrap();
        let orig_v = orig.value_on(d0).unwrap();

        // Additive 100bp → +0.01
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("SERIES_A"), BumpSpec::parallel_bp(100.0));
        let bumped = context.bump(bumps).unwrap();
        let bumped_series = bumped.series("SERIES_A_shift_100bp").unwrap();
        let bumped_v = bumped_series.value_on(d0).unwrap();

        assert!((bumped_v - (orig_v + 0.01)).abs() < 1e-12);

        // Multiplicative 20% → ×1.2
        let mut bumps2 = hashbrown::HashMap::new();
        bumps2.insert(CurveId::new("SERIES_A"), BumpSpec::multiplier(1.2));
        let bumped2 = context.bump(bumps2).unwrap();
        let bumped_series2 = bumped2.series("SERIES_A_mult_1.20").unwrap();
        let bumped_v2 = bumped_series2.value_on(d0).unwrap();
        assert!((bumped_v2 - (orig_v * 1.2)).abs() < 1e-12);
    }

    #[test]
    fn test_fx_base_currency_bump() {
        use crate::currency::Currency;
        use crate::money::fx::{FxConversionPolicy, FxMatrix, FxProvider};
        use alloc::sync::Arc;
        use hashbrown::HashMap as HbMap;
        use time::Month;

        // Minimal mock provider with USD pivot quotes
        struct MockFxProvider {
            rates: HbMap<(Currency, Currency), f64>,
        }
        impl FxProvider for MockFxProvider {
            fn rate(
                &self,
                from: Currency,
                to: Currency,
                _on: Date,
                _policy: FxConversionPolicy,
            ) -> crate::Result<f64> {
                self.rates
                    .get(&(from, to))
                    .copied()
                    .ok_or(crate::Error::Internal)
            }
        }

        let mut rates = HbMap::new();
        // USD weakness test: define base quotes
        rates.insert((Currency::USD, Currency::EUR), 0.90);
        rates.insert((Currency::EUR, Currency::USD), 1.10);
        rates.insert((Currency::USD, Currency::JPY), 110.0);
        rates.insert((Currency::JPY, Currency::USD), 0.0091);

        let provider = Arc::new(MockFxProvider { rates });
        let fx = FxMatrix::new(provider);
        let context = MarketContext::new().insert_fx(fx);

        let on = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let policy = FxConversionPolicy::CashflowDate;

        // Record original USD→EUR, EUR→USD rates
        let orig_usd_eur = context
            .fx
            .as_ref()
            .unwrap()
            .rate(crate::money::fx::FxQuery {
                from: Currency::USD,
                to: Currency::EUR,
                on,
                policy,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;
        let orig_eur_usd = context
            .fx
            .as_ref()
            .unwrap()
            .rate(crate::money::fx::FxQuery {
                from: Currency::EUR,
                to: Currency::USD,
                on,
                policy,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;

        // Weaken USD by 10% → base→other ×1.1, other→base ÷1.1
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("USD"), BumpSpec::multiplier(1.1));
        let bumped = context.bump(bumps).unwrap();
        let bumped_fx = bumped.fx.as_ref().expect("bumped fx present");

        let usd_eur = bumped_fx
            .rate(crate::money::fx::FxQuery {
                from: Currency::USD,
                to: Currency::EUR,
                on,
                policy,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;
        let eur_usd = bumped_fx
            .rate(crate::money::fx::FxQuery {
                from: Currency::EUR,
                to: Currency::USD,
                on,
                policy,
                closure_check: None,
                want_meta: false,
            })
            .unwrap()
            .rate;

        assert!((usd_eur - orig_usd_eur * 1.1).abs() < 1e-12);
        assert!((eur_usd - orig_eur_usd / 1.1).abs() < 1e-12);
    }

    #[test]
    fn test_credit_index_bump() {
        // Build base hazard and base correlation curves using helpers
        let hazard_curve = test_hazard_curve();
        let base_corr = test_base_correlation_curve();

        let index = crate::market_data::credit_index::CreditIndexData::builder()
            .num_constituents(125)
            .recovery_rate(0.40)
            .index_credit_curve(Arc::new(hazard_curve))
            .base_correlation_curve(Arc::new(base_corr))
            .build()
            .unwrap();

        let context = MarketContext::new().insert_credit_index("CDX.NA.IG.42", index);

        // Apply +25bp hazard (spread) and +5% correlation bumps
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(
            CurveId::new("CDX.NA.IG.42"),
            BumpSpec::spread_shift_bp(25.0),
        );
        let bumped_spread = context.bump(bumps).unwrap();
        assert!(
            bumped_spread
                .credit_index("CDX.NA.IG.42_spread_25bp")
                .is_ok()
        );

        let mut bumps2 = hashbrown::HashMap::new();
        bumps2.insert(
            CurveId::new("CDX.NA.IG.42"),
            BumpSpec::correlation_shift_pct(5.0),
        );
        let bumped_corr = context.bump(bumps2).unwrap();
        assert!(
            bumped_corr
                .credit_index("CDX.NA.IG.42_corr_5.0pct")
                .is_ok()
        );
    }

    #[test]
    fn test_comprehensive_multi_curve_bump() {
        let disc_curve = test_discount_curve();
        let hazard_curve = test_hazard_curve();
        let inflation_curve = test_inflation_curve();
        let base_corr_curve = test_base_correlation_curve();

        let context = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_hazard(hazard_curve)
            .insert_inflation(inflation_curve)
            .insert_base_correlation(base_corr_curve);

        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("USD-OIS"), BumpSpec::parallel_bp(50.0));
        bumps.insert(CurveId::new("CORP-HAZARD"), BumpSpec::spread_shift_bp(25.0));
        bumps.insert(CurveId::new("US-CPI"), BumpSpec::inflation_shift_pct(1.5));
        bumps.insert(
            CurveId::new("CDX-NA-IG"),
            BumpSpec::correlation_shift_pct(5.0),
        );

        let bumped_context = context.bump(bumps).unwrap();

        // Verify all curve types were bumped
        assert!(bumped_context.disc("USD-OIS_bump_50bp").is_ok());
        assert!(bumped_context.hazard("CORP-HAZARD_spread_25bp").is_ok());
        assert!(bumped_context.infl("US-CPI_infl_1.5pct").is_ok());
        assert!(bumped_context
            .base_correlation("CDX-NA-IG_corr_5.0pct")
            .is_ok());
    }

    fn create_forward_test_context() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();

        // Create discount curve
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        // Create forward curve
        let fwd_curve = ForwardCurve::builder("USD-SOFR3M", 0.25)
            .base_date(base_date)
            .knots([(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        MarketContext::new()
            .insert_discount(disc_curve)
            .insert_forward(fwd_curve)
            .insert_price("SPY", crate::market_data::primitives::MarketScalar::Unitless(100.0))
            .insert_price("SPY-DIVYIELD", crate::market_data::primitives::MarketScalar::Unitless(0.02))
            .insert_price("EURUSD", crate::market_data::primitives::MarketScalar::Unitless(1.1))
    }

    #[test]
    fn test_equity_forward_function() {
        let context = create_forward_test_context();
        let forward_fn = context.equity_forward("SPY", crate::currency::Currency::USD).unwrap();

        // Test forward price calculation
        let forward_1y = forward_fn(1.0);

        // Should be positive and reasonable
        assert!(forward_1y > 0.0);
        assert!(forward_1y > 90.0 && forward_1y < 110.0); // Reasonable range around spot
    }

    #[test]
    fn test_rates_forward_function() {
        let context = create_forward_test_context();
        let forward_fn = context.rates_forward("USD-SOFR3M").unwrap();

        // Test forward rate
        let forward_rate_1y = forward_fn(1.0);

        // Should match the forward curve
        assert!((forward_rate_1y - 0.035).abs() < 1e-6);
    }

    #[test]
    fn test_auto_forward_detection_equity() {
        let context = create_forward_test_context();
        let forward_fn = context.auto_forward("SPY", crate::currency::Currency::USD).unwrap();

        let forward_1y = forward_fn(1.0);
        assert!(forward_1y > 0.0);
    }

    #[test]
    fn test_auto_forward_detection_rates() {
        let context = create_forward_test_context();
        let forward_fn = context.auto_forward("USD-SOFR3M", crate::currency::Currency::USD).unwrap();

        let forward_rate_1y = forward_fn(1.0);
        assert!((forward_rate_1y - 0.035).abs() < 1e-6);
    }

    #[test]
    fn test_auto_forward_detection_fx() {
        // Create additional discount curve for EUR
        let base_date = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let eur_disc = DiscountCurve::builder("EUR-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.82)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        let context = create_forward_test_context().insert_discount(eur_disc);
        let forward_fn = context.auto_forward("EURUSD", crate::currency::Currency::USD).unwrap();

        let forward_1y = forward_fn(1.0);
        assert!(forward_1y > 0.0);
    }

    #[test]
    fn test_invalid_fx_pair() {
        let context = create_forward_test_context();
        let result = context.fx_forward("INVALID");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_market_data_for_forward() {
        let context = MarketContext::new(); // Empty context

        let result = context.equity_forward("SPY", crate::currency::Currency::USD);
        assert!(result.is_err()); // Should fail due to missing spot price
    }

    #[test]
    fn test_bump_specification_constructors() {
        // Test convenience constructors
        let parallel = BumpSpec::parallel_bp(100.0);
        let spread = BumpSpec::spread_shift_bp(50.0);
        let inflation = BumpSpec::inflation_shift_pct(2.0);
        let correlation = BumpSpec::correlation_shift_pct(10.0);
        let multiplier = BumpSpec::multiplier(1.2);

        assert_eq!(parallel.mode, BumpMode::Additive);
        assert_eq!(parallel.units, BumpUnits::RateBp);
        assert!((parallel.value - 100.0).abs() < 1e-12);

        assert_eq!(spread.mode, BumpMode::Additive);
        assert_eq!(spread.units, BumpUnits::RateBp);
        assert!((spread.value - 50.0).abs() < 1e-12);

        assert_eq!(inflation.mode, BumpMode::Additive);
        assert_eq!(inflation.units, BumpUnits::Percent);
        assert!((inflation.value - 2.0).abs() < 1e-12);

        assert_eq!(correlation.mode, BumpMode::Additive);
        assert_eq!(correlation.units, BumpUnits::Percent);
        assert!((correlation.value - 10.0).abs() < 1e-12);

        assert_eq!(multiplier.mode, BumpMode::Multiplicative);
        assert_eq!(multiplier.units, BumpUnits::Factor);
        assert!((multiplier.value - 1.2).abs() < 1e-12);
    }

    #[test]
    fn test_bump_nonexistent_curve() {
        let context = MarketContext::new();
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(CurveId::new("NONEXISTENT"), BumpSpec::parallel_bp(100.0));
        let result = context.bump(bumps);
        assert!(result.is_err(), "Should fail for nonexistent curve");
    }
}
