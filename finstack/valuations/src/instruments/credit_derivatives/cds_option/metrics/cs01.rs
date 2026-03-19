//! CDS Option CS01 metric calculators.
//!
//! Provides two CS01 variants:
//!
//! - **`Cs01Calculator`** (registered as `MetricId::Cs01`): Par-spread CS01
//!   using the generic par-spread bump + re-bootstrap machinery via central
//!   differencing. This is the default and matches Bloomberg CDSW conventions.
//!
//! - **`Cs01HazardCalculator`** (registered as `MetricId::Cs01Hazard`): Direct
//!   hazard-rate bump CS01. Directly shifts hazard rates without re-bootstrapping.
//!   For constant recovery R, the result is approximately `(1-R)` times the
//!   par-spread CS01.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds_option::CDSOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::Result;

/// Standard credit spread bump: 1 basis point (0.0001 in decimal).
const CS01_BUMP_BP: f64 = 1.0;

/// Par-spread CS01 calculator for CDS Option instruments.
///
/// Delegates to the generic par-spread CS01 machinery which bumps par spreads,
/// re-bootstraps the hazard curve, and uses central differencing.
pub type Cs01Calculator = crate::metrics::GenericParallelCs01<CDSOption>;

/// CS01 Hazard (direct hazard-rate sensitivity) calculator for CDS Option instruments.
///
/// Computes sensitivity by bumping the hazard curve by 1bp (central difference)
/// and repricing. This bumps hazard rates directly, not par spreads.
pub struct Cs01HazardCalculator;

impl MetricCalculator for Cs01HazardCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds_option: &CDSOption = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= cds_option.expiry {
            tracing::debug!(
                instrument_id = %cds_option.id,
                as_of = %as_of,
                expiry = %cds_option.expiry,
                "CDS Option CS01Hazard: Instrument already expired, returning 0.0"
            );
            return Ok(0.0);
        }

        let hazard = context.curves.get_hazard(&cds_option.credit_curve_id)?;

        let bump_decimal = CS01_BUMP_BP * 1e-4;
        let temp_bumped_up = hazard.with_parallel_bump(bump_decimal)?;
        let bumped_hazard_up =
            rebuild_hazard_with_id(&temp_bumped_up, &cds_option.credit_curve_id)?;

        let temp_bumped_down = hazard.with_parallel_bump(-bump_decimal)?;
        let bumped_hazard_down =
            rebuild_hazard_with_id(&temp_bumped_down, &cds_option.credit_curve_id)?;

        let ctx_up = context.curves.as_ref().clone().insert(bumped_hazard_up);
        let pv_up = cds_option.value(&ctx_up, as_of)?.amount();

        let ctx_down = context.curves.as_ref().clone().insert(bumped_hazard_down);
        let pv_down = cds_option.value(&ctx_down, as_of)?.amount();

        let cs01 = (pv_up - pv_down) / (2.0 * CS01_BUMP_BP);

        Ok(cs01)
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
        .map_err(|_| finstack_core::Error::internal("failed to rebuild hazard curve with new id"))
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

        MarketContext::new().insert(disc).insert(hazard)
    }

    #[test]
    fn test_cs01_hazard_finite_diff_for_call() {
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
        option.pricing_overrides.market_quotes.implied_volatility = Some(0.30);

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

        let calculator = Cs01HazardCalculator;
        let result = calculator.calculate(&mut context);

        assert!(result.is_ok(), "CS01Hazard calculation should succeed");
        let cs01 = result.expect("should succeed");

        assert!(cs01.is_finite(), "CS01 should be finite");
        assert!(
            cs01.abs() < 1_000_000.0,
            "CS01 should be reasonable in magnitude: {}",
            cs01
        );
    }
}
