//! DataFrame export utilities for P&L attribution.
//!
//! Provides methods to export attribution results to structured formats for
//! analysis and reporting.
//!
//! Note: Full Polars DataFrame integration pending. Current implementation
//! provides JSON-based exports.

use super::types::*;

impl PnlAttribution {
    /// Export attribution summary as JSON.
    ///
    /// Returns a JSON object with all attribution factors.
    ///
    /// # Errors
    ///
    /// Returns error if JSON serialization fails.
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
    /// - instrument_id, currency, total, carry, carry_theta, carry_roll_down,
    ///   rates_curves, credit_curves, inflation_curves, correlations, fx, vol,
    ///   model_params, market_scalars, residual, residual_pct
    pub fn to_csv(&self) -> String {
        let mut lines = Vec::new();

        // Header
        lines.push(
            "instrument_id,currency,total,carry,carry_theta,carry_roll_down,\
             rates_curves,credit_curves,\
             inflation_curves,correlations,fx,vol,model_params,\
             market_scalars,residual,residual_pct"
                .to_string(),
        );

        let theta_str = self
            .carry_detail
            .as_ref()
            .and_then(|d| d.theta.as_ref())
            .map_or(String::new(), |m| m.amount().to_string());

        let roll_down_str = self
            .carry_detail
            .as_ref()
            .and_then(|d| d.roll_down.as_ref())
            .map_or(String::new(), |m| m.amount().to_string());

        // Data row
        lines.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            self.meta.instrument_id,
            self.total_pnl.currency(),
            self.total_pnl.amount(),
            self.carry.amount(),
            theta_str,
            roll_down_str,
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
    /// Returns CSV with columns: instrument_id, curve_id, tenor, pnl, currency
    ///
    /// Returns None if no rates detail available.
    pub fn rates_detail_to_csv(&self) -> Option<String> {
        self.rates_detail.as_ref().map(|detail| {
            let mut lines = Vec::new();

            // Header
            lines.push("instrument_id,curve_id,tenor,pnl,currency".to_string());

            // Per-curve aggregates (sorted by curve_id for determinism)
            let mut curve_entries: Vec<_> = detail.by_curve.iter().collect();
            curve_entries.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));

            for (curve_id, pnl) in curve_entries {
                lines.push(format!(
                    "{},{},{},{},{}",
                    self.meta.instrument_id,
                    curve_id.as_str(),
                    "",
                    pnl.amount(),
                    pnl.currency()
                ));
            }

            // Per-tenor details (sorted by curve_id then tenor)
            let mut tenor_entries: Vec<_> = detail.by_tenor.iter().collect();
            tenor_entries.sort_by(|a, b| {
                let cmp_curve = a.0 .0.as_str().cmp(b.0 .0.as_str());
                if cmp_curve == std::cmp::Ordering::Equal {
                    a.0 .1.cmp(&b.0 .1)
                } else {
                    cmp_curve
                }
            });

            for ((curve_id, tenor), pnl) in tenor_entries {
                lines.push(format!(
                    "{},{},{},{},{}",
                    self.meta.instrument_id,
                    curve_id.as_str(),
                    tenor,
                    pnl.amount(),
                    pnl.currency()
                ));
            }

            lines.join("\n")
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
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
        attribution
            .compute_residual()
            .expect("Residual computation should succeed in test");

        let csv = attribution.to_csv();
        assert!(csv.contains("instrument_id"));
        assert!(csv.contains("currency"));
        assert!(csv.contains("BOND-001"));
        assert!(csv.contains("USD"));
        assert!(csv.contains("100")); // carry
        assert!(csv.contains("500")); // rates
    }

    #[test]
    fn test_to_json() {
        let total = Money::new(1000.0, Currency::USD);
        let attribution = PnlAttribution::new(
            total,
            "BOND-001",
            date!(2025 - 01 - 15),
            date!(2025 - 01 - 16),
            AttributionMethod::Parallel,
        );

        let json = attribution
            .to_json()
            .expect("JSON serialization should succeed in test");
        assert!(json.contains("BOND-001"));
        assert!(json.contains("total_pnl"));
    }

    #[test]
    fn test_csv_currency_column() {
        // Test that currency is properly exported
        let total = Money::new(5000.0, Currency::EUR);
        let mut attribution = PnlAttribution::new(
            total,
            "EUR-BOND-001",
            date!(2025 - 01 - 15),
            date!(2025 - 01 - 16),
            AttributionMethod::Parallel,
        );

        attribution.carry = Money::new(200.0, Currency::EUR);
        attribution.rates_curves_pnl = Money::new(300.0, Currency::EUR);
        attribution
            .compute_residual()
            .expect("Residual computation should succeed in test");

        let csv = attribution.to_csv();

        // Check header includes currency
        assert!(csv.contains("currency"));

        // Check data row includes EUR
        assert!(csv.contains("EUR"));

        // Check amounts are correct
        assert!(csv.contains("5000"));
        assert!(csv.contains("200"));
        assert!(csv.contains("300"));
    }

    #[test]
    fn test_rates_detail_csv_ordering() {
        use finstack_core::types::CurveId;
        use indexmap::IndexMap;

        let total = Money::new(1000.0, Currency::USD);
        let mut attribution = PnlAttribution::new(
            total,
            "BOND-001",
            date!(2025 - 01 - 15),
            date!(2025 - 01 - 16),
            AttributionMethod::Parallel,
        );

        // Create rates detail with multiple curves in non-alphabetical order
        let mut by_curve = IndexMap::new();
        by_curve.insert(CurveId::from("USD-SOFR"), Money::new(100.0, Currency::USD));
        by_curve.insert(CurveId::from("EUR-OIS"), Money::new(50.0, Currency::USD));
        by_curve.insert(CurveId::from("GBP-SONIA"), Money::new(75.0, Currency::USD));

        let mut by_tenor = IndexMap::new();
        by_tenor.insert(
            (CurveId::from("USD-SOFR"), "5Y".to_string()),
            Money::new(40.0, Currency::USD),
        );
        by_tenor.insert(
            (CurveId::from("EUR-OIS"), "2Y".to_string()),
            Money::new(30.0, Currency::USD),
        );

        attribution.rates_detail = Some(RatesCurvesAttribution {
            by_curve,
            by_tenor,
            discount_total: Money::new(150.0, Currency::USD),
            forward_total: Money::new(75.0, Currency::USD),
        });

        let csv = attribution
            .rates_detail_to_csv()
            .expect("CSV generation should succeed in test");

        // Verify header includes currency
        assert!(csv.contains("currency"));

        // Parse lines to check ordering
        let lines: Vec<&str> = csv.lines().collect();
        assert!(lines.len() > 1); // Header + data

        // Check that EUR-OIS comes before USD-SOFR (alphabetical)
        let eur_pos = csv.find("EUR-OIS").expect("EUR-OIS should be found in CSV");
        let usd_pos = csv
            .find("USD-SOFR")
            .expect("USD-SOFR should be found in CSV");
        assert!(eur_pos < usd_pos, "Curves should be alphabetically ordered");
    }
}
