//! DataFrame export utilities for P&L attribution.
//!
//! Provides methods to export attribution results to structured formats for
//! analysis and reporting.
//!
//! Note: Full Polars DataFrame integration pending. Current implementation
//! provides JSON-based exports.

use crate::attribution::types::*;

impl PnlAttribution {
    /// Export attribution summary as JSON.
    ///
    /// Returns a JSON object with all attribution factors.
    ///
    /// # Errors
    ///
    /// Returns error if JSON serialization fails.
    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> finstack_core::Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            finstack_core::Error::Validation(format!("JSON serialization failed: {}", e))
        })
    }

    /// Export attribution summary as a CSV-compatible string.
    ///
    /// Returns a string with headers and one row of data.
    ///
    /// # Returns
    ///
    /// CSV string with columns:
    /// - instrument_id, total, carry, rates_curves, credit_curves,
    ///   inflation_curves, correlations, fx, vol, model_params,
    ///   market_scalars, residual, residual_pct
    pub fn to_csv(&self) -> String {
        let mut lines = Vec::new();

        // Header
        lines.push(
            "instrument_id,total,carry,rates_curves,credit_curves,\
             inflation_curves,correlations,fx,vol,model_params,\
             market_scalars,residual,residual_pct"
                .to_string(),
        );

        // Data row
        lines.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            self.meta.instrument_id,
            self.total_pnl.amount(),
            self.carry.amount(),
            self.rates_curves_pnl.amount(),
            self.credit_curves_pnl.amount(),
            self.inflation_curves_pnl.amount(),
            self.correlations_pnl.amount(),
            self.fx_pnl.amount(),
            self.vol_pnl.amount(),
            self.model_params_pnl.amount(),
            self.market_scalars_pnl.amount(),
            self.residual.amount(),
            self.meta.residual_pct,
        ));

        lines.join("\n")
    }

    /// Export rates curves detail as CSV string.
    ///
    /// Returns CSV with columns: instrument_id, curve_id, tenor, pnl
    ///
    /// Returns None if no rates detail available.
    pub fn rates_detail_to_csv(&self) -> Option<String> {
        self.rates_detail.as_ref().map(|detail| {
            let mut lines = Vec::new();

            // Header
            lines.push("instrument_id,curve_id,tenor,pnl".to_string());

            // Per-curve aggregates
            for (curve_id, pnl) in &detail.by_curve {
                lines.push(format!(
                    "{},{},{},{}",
                    self.meta.instrument_id,
                    curve_id.as_str(),
                    "",
                    pnl.amount()
                ));
            }

            // Per-tenor details
            for ((curve_id, tenor), pnl) in &detail.by_tenor {
                lines.push(format!(
                    "{},{},{},{}",
                    self.meta.instrument_id,
                    curve_id.as_str(),
                    tenor,
                    pnl.amount()
                ));
            }

            lines.join("\n")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use time::macros::date;

    #[test]
    fn test_to_csv() {
        let total = Money::new(1000.0, Currency::USD);
        let mut attribution = PnlAttribution::new(
            total,
            "BOND-001",
            date!(2025 - 01 - 15),
            date!(2025 - 01 - 16),
            AttributionMethod::Parallel,
        );

        attribution.carry = Money::new(100.0, Currency::USD);
        attribution.rates_curves_pnl = Money::new(500.0, Currency::USD);
        attribution.compute_residual();

        let csv = attribution.to_csv();
        assert!(csv.contains("instrument_id"));
        assert!(csv.contains("BOND-001"));
        assert!(csv.contains("100")); // carry
        assert!(csv.contains("500")); // rates
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_to_json() {
        let total = Money::new(1000.0, Currency::USD);
        let attribution = PnlAttribution::new(
            total,
            "BOND-001",
            date!(2025 - 01 - 15),
            date!(2025 - 01 - 16),
            AttributionMethod::Parallel,
        );

        let json = attribution.to_json().unwrap();
        assert!(json.contains("BOND-001"));
        assert!(json.contains("total_pnl"));
    }
}
