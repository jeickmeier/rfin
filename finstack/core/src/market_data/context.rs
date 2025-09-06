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
    primitives::{MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
    term_structures::base_correlation::BaseCorrelationCurve,
    traits::{Discount, Forward, TermStructure},
};
use crate::dates::Date;
use crate::types::CurveId;
use crate::F;

// -----------------------------------------------------------------------------
// Bump Specification Types
// -----------------------------------------------------------------------------

/// Specification for parallel rate shifts (adding basis points).
#[derive(Debug, Clone)]
pub struct ParallelShift {
    /// Shift amount in basis points (e.g., 100.0 = 100bp = 1%).
    pub bump_bp: F,
}

impl ParallelShift {
    /// Create a new parallel shift in basis points.
    pub fn new(bump_bp: F) -> Self {
        Self { bump_bp }
    }

    /// Convert basis points to rate units.
    fn as_rate(&self) -> F {
        self.bump_bp / 10_000.0
    }
}

/// Specification for multiplicative shocks (scaling rates or prices).
#[derive(Debug, Clone)]
pub struct MultiplierShock {
    /// Multiplier factor (e.g., 1.1 = +10%, 0.9 = -10%).
    pub factor: F,
}

impl MultiplierShock {
    /// Create a new multiplier shock.
    pub fn new(factor: F) -> Self {
        Self { factor }
    }
}

/// Comprehensive bump specification for different types of market shocks.
#[derive(Debug, Clone)]
pub enum BumpSpec {
    /// Parallel shift in basis points for curves.
    ParallelShift(ParallelShift),
    /// Multiplicative shock factor for prices/volatilities.
    MultiplierShock(MultiplierShock),
    /// Spread shift in basis points for credit curves.
    SpreadShift(ParallelShift),
    /// Percentage shift for inflation curves (e.g., +2% inflation shock).
    InflationShift(ParallelShift),
    /// Percentage shift for correlation values (e.g., +10% correlation shock).
    CorrelationShift(ParallelShift),
}

impl BumpSpec {
    /// Convenience constructor for parallel shifts.
    pub fn parallel_bp(bump_bp: F) -> Self {
        Self::ParallelShift(ParallelShift::new(bump_bp))
    }

    /// Convenience constructor for multiplier shocks.
    pub fn multiplier(factor: F) -> Self {
        Self::MultiplierShock(MultiplierShock::new(factor))
    }

    /// Convenience constructor for spread shifts (credit curves).
    pub fn spread_shift_bp(bump_bp: F) -> Self {
        Self::SpreadShift(ParallelShift::new(bump_bp))
    }

    /// Convenience constructor for inflation shifts (as percentage change).
    pub fn inflation_shift_pct(bump_pct: F) -> Self {
        Self::InflationShift(ParallelShift::new(bump_pct * 100.0)) // Convert % to bp
    }

    /// Convenience constructor for correlation shifts (as percentage change).
    pub fn correlation_shift_pct(bump_pct: F) -> Self {
        Self::CorrelationShift(ParallelShift::new(bump_pct * 100.0)) // Convert % to bp
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
    fn base_date(&self) -> Date {
        self.original.base_date()
    }

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
    let bump_factor = match bump_spec {
        BumpSpec::ParallelShift(shift) => 1.0 + shift.as_rate(),
        BumpSpec::MultiplierShock(shock) => shock.factor,
        BumpSpec::SpreadShift(shift) => 1.0 + shift.as_rate(),
        BumpSpec::InflationShift(shift) => 1.0 + shift.as_rate(),
        BumpSpec::CorrelationShift(shift) => 1.0 + shift.as_rate(),
    };

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
        self.disc.get(&CurveId::from(id_str)).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get forward curve by id.
    pub fn fwd(&self, id: impl AsRef<str>) -> crate::Result<Arc<dyn Forward + Send + Sync>> {
        let id_str = id.as_ref();
        self.fwd.get(&CurveId::from(id_str)).cloned().ok_or(
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
        self.hazard.get(&CurveId::from(id_str)).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get inflation curve by id.
    pub fn infl(&self, id: impl AsRef<str>) -> crate::Result<Arc<InflationCurve>> {
        let id_str = id.as_ref();
        self.inflation.get(&CurveId::from(id_str)).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get volatility surface by id.
    pub fn surface(&self, id: impl AsRef<str>) -> crate::Result<Arc<VolSurface>> {
        let id_str = id.as_ref();
        self.surfaces.get(&CurveId::from(id_str)).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get market scalar (price/constant) by id.
    pub fn price(&self, id: impl AsRef<str>) -> crate::Result<&MarketScalar> {
        let id_str = id.as_ref();
        self.prices.get(&CurveId::from(id_str)).ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Get scalar time series by id.
    pub fn series(&self, id: impl AsRef<str>) -> crate::Result<&ScalarTimeSeries> {
        let id_str = id.as_ref();
        self.series.get(&CurveId::from(id_str)).ok_or(
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
        self.base_correlation
            .get(&CurveId::from(id_str))
            .cloned()
            .ok_or(
                crate::error::InputError::NotFound {
                    id: id_str.to_string(),
                }
                .into(),
            )
    }

    /// Get inflation index by id.
    pub fn inflation_index(&self, id: impl AsRef<str>) -> Option<Arc<InflationIndex>> {
        self.inflation_indices
            .get(&CurveId::from(id.as_ref()))
            .cloned()
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
                if let BumpSpec::ParallelShift(shift) = bump_spec {
                    let bumped_id =
                        CurveId::new(format!("{}_bump_{:.0}bp", curve_id_str, shift.bump_bp));
                    let bumped_curve =
                        BumpedDiscountCurve::new(original, shift.bump_bp, bumped_id.clone());
                    new_context.disc.insert(bumped_id, Arc::new(bumped_curve));
                }
            } else if let Ok(original) = self.fwd(curve_id_str) {
                if let BumpSpec::ParallelShift(shift) = bump_spec {
                    let bumped_id =
                        CurveId::new(format!("{}_bump_{:.0}bp", curve_id_str, shift.bump_bp));
                    let bumped_curve =
                        BumpedForwardCurve::new(original, shift.bump_bp, bumped_id.clone());
                    new_context.fwd.insert(bumped_id, Arc::new(bumped_curve));
                }
            } else if let Ok(original) = self.hazard(curve_id_str) {
                if let BumpSpec::SpreadShift(shift) = bump_spec {
                    let spread_rate = shift.bump_bp / 10_000.0; // Convert bp to rate
                    if let Ok(bumped_curve) = original.with_hazard_shift(spread_rate) {
                        let bumped_id =
                            CurveId::new(format!("{}_spread_{:.0}bp", curve_id_str, shift.bump_bp));
                        new_context.hazard.insert(bumped_id, Arc::new(bumped_curve));
                    }
                }
            } else if let Ok(original) = self.infl(curve_id_str) {
                if let BumpSpec::InflationShift(shift) = bump_spec {
                    let inflation_pct = shift.bump_bp / 100.0; // Convert bp back to percentage
                    let multiplier = 1.0 + inflation_pct / 100.0;
                    let original_knots = original.knots();
                    let original_cpi_levels = original.cpi_levels();

                    let bumped_points: Vec<(F, F)> = original_knots
                        .iter()
                        .zip(original_cpi_levels.iter())
                        .map(|(&t, &cpi)| (t, cpi * multiplier))
                        .collect();

                    let bumped_id =
                        CurveId::new(format!("{}_infl_{:.1}pct", curve_id_str, inflation_pct));
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
                if let BumpSpec::CorrelationShift(shift) = bump_spec {
                    let correlation_pct = shift.bump_bp / 100.0; // Convert bp back to percentage
                    let multiplier = 1.0 + correlation_pct / 100.0;
                    let original_points = original.detachment_points();
                    let original_correlations = original.correlations();

                    let bumped_points: Vec<(F, F)> = original_points
                        .iter()
                        .zip(original_correlations.iter())
                        .map(|(&detach, &corr)| (detach, (corr * multiplier).clamp(0.0, 1.0)))
                        .collect();

                    let bumped_id =
                        CurveId::new(format!("{}_corr_{:.1}pct", curve_id_str, correlation_pct));
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
            } else if let Ok(original) = self.surface(curve_id_str) {
                let bump_desc = match &bump_spec {
                    BumpSpec::ParallelShift(shift) => format!("shift_{:.0}bp", shift.bump_bp),
                    BumpSpec::MultiplierShock(shock) => format!("mult_{:.2}", shock.factor),
                    _ => continue, // Skip unsupported bump types for vol surfaces
                };
                let bumped_id = CurveId::new(format!("{}_{}", curve_id_str, bump_desc));
                if let Ok(bumped_surface) =
                    create_bumped_vol_surface(&original, &bump_spec, bumped_id.clone())
                {
                    new_context
                        .surfaces
                        .insert(bumped_id, Arc::new(bumped_surface));
                }
            } else if let Ok(original) = self.price(curve_id_str) {
                let bump_desc = match &bump_spec {
                    BumpSpec::ParallelShift(shift) => format!("shift_{:.0}bp", shift.bump_bp),
                    BumpSpec::MultiplierShock(shock) => format!("mult_{:.2}", shock.factor),
                    BumpSpec::SpreadShift(shift) => format!("spread_{:.0}bp", shift.bump_bp),
                    BumpSpec::InflationShift(shift) => format!("infl_{:.0}bp", shift.bump_bp),
                    BumpSpec::CorrelationShift(shift) => format!("corr_{:.0}bp", shift.bump_bp),
                };

                let bumped_value = match (original, bump_spec) {
                    (MarketScalar::Unitless(val), BumpSpec::ParallelShift(shift)) => {
                        MarketScalar::Unitless(val + shift.as_rate())
                    }
                    (MarketScalar::Unitless(val), BumpSpec::MultiplierShock(shock)) => {
                        MarketScalar::Unitless(val * shock.factor)
                    }
                    (MarketScalar::Unitless(val), BumpSpec::SpreadShift(shift)) => {
                        MarketScalar::Unitless(val + shift.as_rate())
                    }
                    (MarketScalar::Unitless(val), BumpSpec::InflationShift(shift)) => {
                        MarketScalar::Unitless(val + shift.as_rate())
                    }
                    (MarketScalar::Unitless(val), BumpSpec::CorrelationShift(shift)) => {
                        MarketScalar::Unitless(val + shift.as_rate())
                    }
                    (MarketScalar::Price(money), BumpSpec::ParallelShift(shift)) => {
                        let new_amount = money.amount() * (1.0 + shift.as_rate());
                        MarketScalar::Price(crate::money::Money::new(new_amount, money.currency()))
                    }
                    (MarketScalar::Price(money), BumpSpec::MultiplierShock(shock)) => {
                        let new_amount = money.amount() * shock.factor;
                        MarketScalar::Price(crate::money::Money::new(new_amount, money.currency()))
                    }
                    (MarketScalar::Price(money), BumpSpec::SpreadShift(shift)) => {
                        let new_amount = money.amount() * (1.0 + shift.as_rate());
                        MarketScalar::Price(crate::money::Money::new(new_amount, money.currency()))
                    }
                    (MarketScalar::Price(money), BumpSpec::InflationShift(shift)) => {
                        let new_amount = money.amount() * (1.0 + shift.as_rate());
                        MarketScalar::Price(crate::money::Money::new(new_amount, money.currency()))
                    }
                    (MarketScalar::Price(money), BumpSpec::CorrelationShift(shift)) => {
                        let new_amount = money.amount() * (1.0 + shift.as_rate());
                        MarketScalar::Price(crate::money::Money::new(new_amount, money.currency()))
                    }
                };
                let bumped_id = CurveId::new(format!("{}_{}", curve_id_str, bump_desc));
                new_context.prices.insert(bumped_id, bumped_value);
            } else {
                return Err(crate::error::InputError::NotFound {
                    id: curve_id_str.to_string(),
                }
                .into());
            }
        }

        Ok(new_context)
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

    #[test]
    fn test_bump_specification_constructors() {
        // Test convenience constructors
        let parallel = BumpSpec::parallel_bp(100.0);
        let spread = BumpSpec::spread_shift_bp(50.0);
        let inflation = BumpSpec::inflation_shift_pct(2.0);
        let correlation = BumpSpec::correlation_shift_pct(10.0);
        let multiplier = BumpSpec::multiplier(1.2);

        match parallel {
            BumpSpec::ParallelShift(shift) => assert!((shift.bump_bp - 100.0).abs() < 1e-12),
            _ => panic!("Expected ParallelShift"),
        }

        match spread {
            BumpSpec::SpreadShift(shift) => assert!((shift.bump_bp - 50.0).abs() < 1e-12),
            _ => panic!("Expected SpreadShift"),
        }

        match inflation {
            BumpSpec::InflationShift(shift) => assert!((shift.bump_bp - 200.0).abs() < 1e-12), // 2% * 100 = 200bp
            _ => panic!("Expected InflationShift"),
        }

        match correlation {
            BumpSpec::CorrelationShift(shift) => assert!((shift.bump_bp - 1000.0).abs() < 1e-12), // 10% * 100 = 1000bp
            _ => panic!("Expected CorrelationShift"),
        }

        match multiplier {
            BumpSpec::MultiplierShock(shock) => assert!((shock.factor - 1.2).abs() < 1e-12),
            _ => panic!("Expected MultiplierShock"),
        }
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
