//! Aggregates multiple market-data objects (yield, forward, credit, inflation
//! curves and volatility surfaces) into a single lookup structure –
//! [`CurveSet`].
//!
//! `CurveSet` offers builder-style `with_*` helpers and convenient getters such
//! as `.discount("USD-OIS")` that return `Arc`-wrapped trait objects ready for
//! thread-safe sharing.
//!
//! ## Example
//! ```rust
//! use finstack_core::market_data::term_structures::*;
//! use finstack_core::market_data::multicurve::CurveSet;
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! let disc = DiscountCurve::builder("USD-OIS")
//!     .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
//!     .knots([(0.0, 1.0), (1.0, 0.98)])
//!     .linear_df()
//!     .build()
//!     .unwrap();
//! let set = CurveSet::new().with_discount(disc);
//! assert!(set.discount("USD-OIS").is_ok());
//! ```
#![allow(dead_code)]

extern crate alloc;
use alloc::sync::Arc;
use hashbrown::HashMap;

use crate::{
    error::InputError,
    market_data::{
        hazard_curve::HazardCurve,
        id::CurveId,
        inflation::InflationCurve,
        traits::{Discount, Forward, TermStructure},
    },
};

/// Multi-curve container holding discount, forecast and risk curves.
#[derive(Clone, Default)]
pub struct CurveSet {
    disc: HashMap<CurveId, Arc<dyn Discount + Send + Sync>>,
    fwd: HashMap<CurveId, Arc<dyn Forward + Send + Sync>>, // forecast curves
    hazard: HashMap<CurveId, Arc<HazardCurve>>,            // concrete type for now
    inflation: HashMap<CurveId, Arc<InflationCurve>>,      // concrete type
    vol2d: HashMap<CurveId, Arc<crate::market_data::vol_surface::VolSurface>>, // vol surfaces
    collat: HashMap<&'static str, CurveId>,                // CSA code → discount id
}

impl CurveSet {
    /// Create an empty `CurveSet`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert discount curve.
    pub fn with_discount<C: Discount + Send + Sync + 'static>(mut self, curve: C) -> Self {
        let cid = *TermStructure::id(&curve);
        self.disc.insert(cid, Arc::new(curve));
        self
    }

    /// Insert forecast curve.
    pub fn with_forecast<C: Forward + Send + Sync + 'static>(mut self, curve: C) -> Self {
        let cid = *TermStructure::id(&curve);
        self.fwd.insert(cid, Arc::new(curve));
        self
    }

    /// Insert hazard curve.
    pub fn with_hazard(mut self, curve: HazardCurve) -> Self {
        let cid = *crate::market_data::traits::TermStructure::id(&curve);
        self.hazard.insert(cid, Arc::new(curve));
        self
    }

    /// Insert inflation curve.
    pub fn with_inflation(mut self, curve: InflationCurve) -> Self {
        let cid = *crate::market_data::traits::TermStructure::id(&curve);
        self.inflation.insert(cid, Arc::new(curve));
        self
    }

    /// Insert vol surface
    pub fn with_vol_surface(
        mut self,
        surface: crate::market_data::vol_surface::VolSurface,
    ) -> Self {
        let cid = *crate::market_data::traits::TermStructure::id(&surface);
        self.vol2d.insert(cid, Arc::new(surface));
        self
    }

    /// Map collateral CSA code to discount curve id.
    pub fn map_collateral(mut self, csa_code: &'static str, disc_id: CurveId) -> Self {
        self.collat.insert(csa_code, disc_id);
        self
    }

    /// Get discount curve by id.
    pub fn discount(&self, id: &'static str) -> crate::Result<Arc<dyn Discount + Send + Sync>> {
        self.disc
            .get(&CurveId::new(id))
            .cloned()
            .ok_or(InputError::NotFound.into())
    }

    /// Get forecast curve by id.
    pub fn forecast(&self, id: &'static str) -> crate::Result<Arc<dyn Forward + Send + Sync>> {
        self.fwd
            .get(&CurveId::new(id))
            .cloned()
            .ok_or(InputError::NotFound.into())
    }

    /// Get hazard curve.
    pub fn hazard(&self, id: &'static str) -> crate::Result<Arc<HazardCurve>> {
        self.hazard
            .get(&CurveId::new(id))
            .cloned()
            .ok_or(InputError::NotFound.into())
    }

    /// Get inflation curve.
    pub fn inflation(&self, id: &'static str) -> crate::Result<Arc<InflationCurve>> {
        self.inflation
            .get(&CurveId::new(id))
            .cloned()
            .ok_or(InputError::NotFound.into())
    }

    /// Retrieve a 2-D volatility surface by identifier.
    pub fn vol_surface(
        &self,
        id: &'static str,
    ) -> crate::Result<Arc<crate::market_data::vol_surface::VolSurface>> {
        self.vol2d
            .get(&CurveId::new(id))
            .cloned()
            .ok_or(InputError::NotFound.into())
    }

    /// Resolve collateral discount curve for CSA code.
    pub fn collateral(&self, csa_code: &str) -> crate::Result<Arc<dyn Discount + Send + Sync>> {
        let id = match self.collat.get(csa_code) {
            Some(cid) => cid,
            None => return Err(InputError::NotFound.into()),
        };
        self.discount(id.as_str())
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::Date;
    use crate::market_data::discount_curve::DiscountCurve;

    fn sample_discount() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.98)])
            .linear_df()
            .build()
            .unwrap()
    }

    fn sample_hazard() -> HazardCurve {
        HazardCurve::builder("USD-CRED")
            .knots([(0.0, 0.01)])
            .build()
            .unwrap()
    }

    #[test]
    fn fetch_discount_curve() {
        let yc = sample_discount();
        let set = CurveSet::new().with_discount(yc);
        let disc = set.discount("USD-OIS").unwrap();
        assert_eq!(
            crate::market_data::traits::TermStructure::id(&*disc).as_str(),
            "USD-OIS"
        );
    }

    #[test]
    fn collateral_mapping() {
        let yc = sample_discount();
        let id = *crate::market_data::traits::TermStructure::id(&yc);
        let set = CurveSet::new()
            .with_discount(yc)
            .map_collateral("CSA-USD", id);
        let disc = set.collateral("CSA-USD").unwrap();
        assert_eq!(
            crate::market_data::traits::TermStructure::id(&*disc).as_str(),
            "USD-OIS"
        );
    }

    #[test]
    fn hazard_fetch() {
        let hc = sample_hazard();
        let set = CurveSet::new().with_hazard(hc);
        let hz = set.hazard("USD-CRED").unwrap();
        assert_eq!(
            crate::market_data::traits::TermStructure::id(&*hz).as_str(),
            "USD-CRED"
        );
    }
}
