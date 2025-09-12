//! Serialization support for MarketContext
//!
//! Provides type definitions for serializing MarketContext.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

extern crate alloc;
use alloc::{string::String, vec::Vec};

use super::{
    context::BumpSpec,
    inflation::InflationCurve,
    primitives::{MarketScalar, ScalarTimeSeriesState},
    surfaces::vol_surface::VolSurfaceState,
    term_structures::{
        base_correlation::BaseCorrelationCurve, discount_curve::DiscountCurve,
        forward_curve::ForwardCurve, hazard_curve::HazardCurveState,
    },
};
use crate::{dates::Date, types::CurveId, F};

// -----------------------------------------------------------------------------
// Serializable Market Context
// -----------------------------------------------------------------------------

/// Serializable representation of MarketContext
///
/// This structure mirrors MarketContext but uses serializable types,
/// avoiding trait object limitations.
#[cfg(feature = "serde")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketContextData {
    /// Discount curve IDs and bump specifications (for reconstruction)
    pub disc_curves: Vec<DiscountCurveEntry>,
    /// Forward curve IDs and bump specifications
    pub fwd_curves: Vec<ForwardCurveEntry>,
    /// Hazard curves in state form
    pub hazard_curves: Vec<(CurveId, HazardCurveState)>,
    /// Inflation curves (directly serializable)
    pub inflation_curves: Vec<(CurveId, InflationCurve)>,
    /// Inflation indices
    pub inflation_indices: Vec<(CurveId, InflationIndexData)>,
    /// Base correlation curves (directly serializable)
    pub base_correlation_curves: Vec<(CurveId, BaseCorrelationCurve)>,
    /// Credit index data
    pub credit_indices: Vec<(CurveId, CreditIndexEntry)>,
    /// FX matrix quotes
    pub fx: Option<FxMatrixData>,
    /// Volatility surfaces in state form
    pub surfaces: Vec<(CurveId, VolSurfaceState)>,
    /// Market scalars/prices
    pub prices: Vec<(CurveId, MarketScalar)>,
    /// Time series data in state form
    pub series: Vec<(CurveId, ScalarTimeSeriesState)>,
    /// Collateral mappings (CSA code -> curve ID)
    pub collateral_mappings: Vec<(String, CurveId)>,
}

/// Entry for discount curves - stores ID and optional bump info
///
/// TODO: Once DiscountCurve has state methods, replace Option<DiscountCurve>
/// with DiscountCurveState for full serialization support.
#[cfg(feature = "serde")]
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscountCurveEntry {
    /// Curve identifier
    pub id: CurveId,
    /// If this is a bumped curve, store the original ID and bump
    pub bump_info: Option<BumpInfo>,
    /// The actual curve data (if not bumped)
    /// TODO: Replace with DiscountCurveState once implemented
    pub curve: Option<DiscountCurve>,
}

/// Entry for forward curves
///
/// TODO: Once ForwardCurve has state methods, replace Option<ForwardCurve>
/// with ForwardCurveState for full serialization support.
#[cfg(feature = "serde")]
#[derive(Debug, Serialize, Deserialize)]
pub struct ForwardCurveEntry {
    /// Curve identifier
    pub id: CurveId,
    /// Bump information for reconstruction
    pub bump_info: Option<BumpInfo>,
    /// The actual curve data (if not bumped)
    /// TODO: Replace with ForwardCurveState once implemented
    pub curve: Option<ForwardCurve>,
}

/// Bump information for reconstructing bumped curves
#[cfg(feature = "serde")]
#[derive(Debug, Serialize, Deserialize)]
pub struct BumpInfo {
    /// The original curve ID before bumping
    pub original_id: CurveId,
    /// The bump specification that was applied
    pub bump_spec: BumpSpec,
}

/// Serializable inflation index data
#[cfg(feature = "serde")]
#[derive(Debug, Serialize, Deserialize)]
pub struct InflationIndexData {
    /// Inflation index identifier
    pub id: String,
    /// Historical observations as (date, value) pairs
    pub observations: Vec<(Date, F)>,
    /// Currency of the inflation index
    pub currency: crate::currency::Currency,
    /// Interpolation method for inflation data
    pub interpolation: crate::market_data::inflation_index::InflationInterpolation,
    /// Publication lag for inflation data
    pub lag: crate::market_data::inflation_index::InflationLag,
}

/// Serializable credit index entry
#[cfg(feature = "serde")]
#[derive(Debug, Serialize, Deserialize)]
pub struct CreditIndexEntry {
    /// Number of constituents in the credit index
    pub num_constituents: u16,
    /// Expected recovery rate for defaults
    pub recovery_rate: F,
    /// Hazard curve for the credit index
    pub index_credit_curve: HazardCurveState,
    /// Base correlation curve for the index
    pub base_correlation_curve: BaseCorrelationCurve,
    /// Individual issuer credit curves if available
    pub issuer_credit_curves: Option<Vec<(String, HazardCurveState)>>,
}

/// Serializable FX matrix data
#[cfg(feature = "serde")]
#[derive(Debug, Serialize, Deserialize)]
pub struct FxMatrixData {
    /// Currency pair quotes: ((from_code, to_code), rate)
    pub quotes: Vec<((String, String), F)>,
    /// Pivot currency code if set
    pub pivot_currency: Option<String>,
}

// The implementation of MarketContext serialization methods are in context.rs
// to have access to private fields
