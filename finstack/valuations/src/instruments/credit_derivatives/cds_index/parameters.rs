//! CDS Index preset descriptors.
//!
//! `CDSIndexParams` is a lightweight metadata bundle for well-known
//! standardized indices (CDX.NA.IG, CDX.NA.HY, iTraxx Europe, etc.). It
//! captures only the index identity (name, series, version), the running
//! coupon, and the regional convention. Trade-specific state — notional,
//! side, dates, curves, defaulted constituents, index factor — lives on
//! the `CDSIndex` instrument itself.
//!
//! Use `CDSIndex::from_preset(&preset, ...)` to build an instrument from a
//! preset, then chain `with_constituents`, `with_constituents_equal_weight`,
//! and `with_index_factor` to attach trade state.

use crate::instruments::credit_derivatives::cds::CDSConvention;

/// Preset metadata for a well-known CDS index series.
///
/// Captures the index identity (name + series + version), running coupon,
/// and regional ISDA convention. Pair with the trade-specific arguments
/// (id, notional, side, dates, recovery, curves) on
/// `CDSIndex::from_preset` to obtain a `CDSIndex` instrument.
#[derive(Debug, Clone, PartialEq)]
pub struct CDSIndexParams {
    /// Index name (e.g., "CDX.NA.IG", "iTraxx Europe").
    pub index_name: String,
    /// Index series number (e.g., 42).
    pub series: u16,
    /// Index version number within the series.
    pub version: u16,
    /// Running fixed coupon in basis points (e.g. 100bp for CDX.NA.IG).
    pub fixed_coupon_bp: f64,
    /// Regional ISDA convention. Bundled into the preset because each
    /// well-known index has a fixed convention (CDX uses `IsdaNa`, iTraxx
    /// uses `IsdaEu`).
    pub convention: CDSConvention,
}

impl CDSIndexParams {
    /// Construct a custom preset.
    ///
    /// For standard indices prefer the dedicated factories:
    /// [`CDSIndexParams::cdx_na_ig`], [`cdx_na_hy`](Self::cdx_na_hy),
    /// [`itraxx_europe`](Self::itraxx_europe).
    pub fn new(
        index_name: impl Into<String>,
        series: u16,
        version: u16,
        fixed_coupon_bp: f64,
        convention: CDSConvention,
    ) -> Self {
        Self {
            index_name: index_name.into(),
            series,
            version,
            fixed_coupon_bp,
            convention,
        }
    }

    /// CDX.NA.IG (North American investment-grade) preset on `IsdaNa`.
    pub fn cdx_na_ig(series: u16, version: u16, fixed_coupon_bp: f64) -> Self {
        Self::new(
            "CDX.NA.IG",
            series,
            version,
            fixed_coupon_bp,
            CDSConvention::IsdaNa,
        )
    }

    /// CDX.NA.HY (North American high-yield) preset on `IsdaNa`.
    pub fn cdx_na_hy(series: u16, version: u16, fixed_coupon_bp: f64) -> Self {
        Self::new(
            "CDX.NA.HY",
            series,
            version,
            fixed_coupon_bp,
            CDSConvention::IsdaNa,
        )
    }

    /// iTraxx Europe (European investment-grade) preset on `IsdaEu`.
    pub fn itraxx_europe(series: u16, version: u16, fixed_coupon_bp: f64) -> Self {
        Self::new(
            "iTraxx Europe",
            series,
            version,
            fixed_coupon_bp,
            CDSConvention::IsdaEu,
        )
    }
}
