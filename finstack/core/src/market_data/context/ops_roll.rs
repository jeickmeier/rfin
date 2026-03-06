use crate::collections::HashMap;
use std::sync::Arc;

use super::curve_storage::CurveStorage;
use super::MarketContext;

impl MarketContext {
    // -----------------------------------------------------------------------------
    // Curve Rolling (Time Roll-Forward Support)
    // -----------------------------------------------------------------------------

    /// Roll all curves forward by a specified number of days.
    ///
    /// This creates a new `MarketContext` with all curves rolled forward:
    /// - Base dates advanced by `days`
    /// - Knot times shifted backwards (expired points filtered out)
    /// - Curve values preserved (no carry/theta adjustment)
    ///
    /// This is the "constant curves" scenario used for roll-down P&L calculations.
    ///
    /// # Arguments
    /// * `days` - Number of days to roll forward
    ///
    /// # Returns
    /// A new `MarketContext` with all curves rolled forward.
    ///
    /// # Errors
    /// Returns an error if any curve cannot be rolled (e.g., too few points remain).
    ///
    /// # Notes
    /// - Surfaces and other market data are cloned without modification
    /// - FX matrices are preserved as-is (assumed static spot rates)
    /// - Curves with insufficient remaining points will cause an error
    ///
    /// # Examples
    /// ```rust,no_run
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::term_structures::DiscountCurve;
    /// use time::macros::date;
    /// # fn main() -> finstack_core::Result<()> {
    ///
    /// let base_date = date!(2025 - 01 - 01);
    /// let curve = DiscountCurve::builder("USD_OIS")
    ///     .base_date(base_date)
    ///     .knots(vec![(1.0, 0.98), (2.0, 0.96), (5.0, 0.90)])
    ///     .build()
    ///     ?;
    ///
    /// let ctx = MarketContext::new().insert_discount(curve);
    ///
    /// // Roll 6 months forward
    /// let rolled_ctx = ctx.roll_forward(182)?;
    /// # let _ = rolled_ctx;
    /// # Ok(())
    /// # }
    /// ```
    pub fn roll_forward(&self, days: i64) -> crate::Result<Self> {
        let mut new_ctx = Self {
            curves: {
                let mut m = HashMap::default();
                m.reserve(self.curves.len());
                m
            },
            fx: self.fx.clone(),
            surfaces: self.surfaces.clone(),
            prices: self.prices.clone(),
            series: self.series.clone(),
            inflation_indices: self.inflation_indices.clone(),
            credit_indices: self.credit_indices.clone(),
            dividends: self.dividends.clone(),
            fx_delta_vol_surfaces: self.fx_delta_vol_surfaces.clone(),
            collateral: self.collateral.clone(),
        };

        // Roll each curve forward
        for (id, storage) in &self.curves {
            let rolled_storage = match storage {
                CurveStorage::Discount(curve) => {
                    let rolled = curve.roll_forward(days)?;
                    CurveStorage::Discount(Arc::new(rolled))
                }
                CurveStorage::Forward(curve) => {
                    let rolled = curve.roll_forward(days)?;
                    CurveStorage::Forward(Arc::new(rolled))
                }
                CurveStorage::Hazard(curve) => {
                    let rolled = curve.roll_forward(days)?;
                    CurveStorage::Hazard(Arc::new(rolled))
                }
                CurveStorage::Inflation(curve) => {
                    let rolled = curve.roll_forward(days)?;
                    CurveStorage::Inflation(Arc::new(rolled))
                }
                CurveStorage::BaseCorrelation(curve) => {
                    // Base correlation curves don't have time-dependent knots
                    // in the same way - they're keyed by detachment point, not time
                    CurveStorage::BaseCorrelation(curve.clone())
                }
                CurveStorage::VolIndex(curve) => {
                    let rolled = curve.roll_forward(days)?;
                    CurveStorage::VolIndex(Arc::new(rolled))
                }
                CurveStorage::Price(curve) => {
                    let rolled = curve.roll_forward(days)?;
                    CurveStorage::Price(Arc::new(rolled))
                }
            };
            new_ctx.curves.insert(id.clone(), rolled_storage);
        }

        Ok(new_ctx)
    }
}
