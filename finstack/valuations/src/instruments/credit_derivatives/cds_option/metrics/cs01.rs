//! CDS Option CS01 (Hazard Rate Sensitivity) metric calculator.
//!
//! **Important**: This metric computes the PV sensitivity to a 1bp parallel
//! shift in the **hazard rate curve**, not the par spread. For constant recovery
//! R, the relationship is approximately `dh/dS ≈ 1/(1-R)`, so at R=40% the
//! hazard-rate CS01 is approximately 60% of the true spread-based CS01.
//!
//! For spread-based CS01 (matching Bloomberg CDSW), one would need to
//! recalibrate hazard rates from bumped par spreads. This implementation
//! uses direct hazard curve bumping for computational efficiency.
//!
//! # Formula
//!
//! CS01 = (PV_bumped - PV_base) / 1bp
//!
//! where the bump is applied to the hazard curve (parallel shift).
//!
//! # Naming Convention
//!
//! Despite the "CS01" name, this is technically a "Hazard01" metric.
//! Users requiring spread-based CS01 should apply the `1/(1-R)` scaling
//! factor to approximate the spread sensitivity.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds_option::CDSOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::Result;

/// Standard credit spread bump: 1 basis point (0.0001 in decimal).
const CS01_BUMP_BP: f64 = 1.0;

/// CS01 (hazard rate sensitivity) calculator for CDS Option instruments.
///
/// Computes sensitivity by bumping the hazard curve by 1bp and repricing.
/// Note: this bumps hazard rates directly, not par spreads. The result is
/// approximately `(1-R)` times the true spread-based CS01. See module docs
/// for details on the distinction.
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds_option: &CDSOption = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= cds_option.expiry {
            tracing::debug!(
                instrument_id = %cds_option.id,
                as_of = %as_of,
                expiry = %cds_option.expiry,
                "CDS Option CS01: Instrument already expired, returning 0.0"
            );
            return Ok(0.0);
        }

        // Base PV
        let base_pv = context.base_value.amount();

        // Get the hazard curve and bump it
        let hazard = context.curves.get_hazard(&cds_option.credit_curve_id)?;

        // Bump hazard curve by 1bp (convert bp to decimal: 1bp = 0.0001)
        // with_parallel_bump expects a decimal shift, so 1bp = 0.0001
        let bump_decimal = CS01_BUMP_BP * 1e-4;
        let temp_bumped = hazard.with_parallel_bump(bump_decimal)?;

        // Rebuild with the original ID so it replaces in the context
        let bumped_hazard = rebuild_hazard_with_id(&temp_bumped, &cds_option.credit_curve_id)?;

        // Create bumped market context
        let bumped_curves = context.curves.as_ref().clone().insert_hazard(bumped_hazard);

        // Reprice with bumped curve
        let pv_bumped = cds_option.value(&bumped_curves, as_of)?.amount();

        // CS01 = (PV_bumped - PV_base) / bump_size
        // Note: We report CS01 per 1bp, so divide by the bump size in bp
        let cs01 = (pv_bumped - base_pv) / CS01_BUMP_BP;

        Ok(cs01)
    }

    fn dependencies(&self) -> &[MetricId] {
        // No dependencies - we compute CS01 independently via finite differences
        &[]
    }
}

/// Rebuild a hazard curve with a new ID, preserving all metadata.
fn rebuild_hazard_with_id(
    curve: &HazardCurve,
    new_id: &finstack_core::types::CurveId,
) -> Result<HazardCurve> {
    curve
        .to_builder_with_id(new_id.clone())
        .build()
        .map_err(|_| finstack_core::Error::Internal)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::credit_derivatives::cds_option::parameters::CDSOptionParams;
    use crate::instruments::credit_derivatives::cds_option::CDSOption;
    use crate::instruments::CreditParams;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
    use finstack_core::money::Money;
    use time::macros::date;

    /// Create a standard test market with discount and hazard curves.
    fn test_market(as_of: Date) -> MarketContext {
        let rate: f64 = 0.03;
        let df1 = (-rate).exp();
        let df5 = (-rate * 5.0).exp();
        let df10 = (-rate * 10.0).exp();

        let disc = DiscountCurve::builder("USD_OIS")
            .base_date(as_of)
            .knots([(0.0, 1.0), (1.0, df1), (5.0, df5), (10.0, df10)])
            .build()
            .expect("Valid discount curve");

        let hazard_rate = 0.02;
        let recovery = 0.4;
        let par = hazard_rate * 10000.0 * (1.0 - recovery);
        let hazard = HazardCurve::builder("CORP_HAZARD")
            .base_date(as_of)
            .recovery_rate(recovery)
            .knots([(1.0, hazard_rate), (5.0, hazard_rate), (10.0, hazard_rate)])
            .par_spreads([(1.0, par), (5.0, par), (10.0, par)])
            .build()
            .expect("Valid hazard curve");

        MarketContext::new()
            .insert_discount(disc)
            .insert_hazard(hazard)
    }

    #[test]
    fn test_cs01_finite_diff_for_call() {
        let as_of = date!(2024 - 01 - 01);
        let market = test_market(as_of);

        let option_params = CDSOptionParams::call(
            rust_decimal::Decimal::new(1, 2), // 0.01 = 100bp
            date!(2025 - 01 - 01),
            date!(2029 - 01 - 01),
            Money::new(10_000_000.0, Currency::USD),
        )
        .expect("Valid CDS option parameters");
        let credit_params = CreditParams::corporate_standard("CORP", "CORP_HAZARD");
        let mut option = CDSOption::new(
            "TEST_CDSOPT",
            &option_params,
            &credit_params,
            "USD_OIS",
            "CDS_VOL",
        )
        .expect("Valid CDS option");
        // Set implied vol override since we don't have a vol surface
        option.pricing_overrides.market_quotes.implied_volatility = Some(0.30);

        // Get base value
        let base_value = option
            .value(&market, as_of)
            .expect("Pricing should succeed");

        let mut context = MetricContext::new(
            std::sync::Arc::new(option),
            std::sync::Arc::new(market),
            as_of,
            base_value,
            MetricContext::default_config(),
        );

        let calculator = Cs01Calculator;
        let result = calculator.calculate(&mut context);

        assert!(result.is_ok(), "CS01 calculation should succeed");
        let cs01 = result.expect("should succeed");

        // CS01 can be positive or negative depending on option position
        // For a call option on spreads, CS01 should be finite
        assert!(cs01.is_finite(), "CS01 should be finite");
        assert!(
            cs01.abs() < 1_000_000.0,
            "CS01 should be reasonable in magnitude: {}",
            cs01
        );
    }
}
