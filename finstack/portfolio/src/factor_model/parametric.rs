//! Parametric factor risk decomposition using covariance-based Euler allocation.

use super::traits::RiskDecomposer;
use super::types::{FactorContribution, PositionFactorContribution, RiskDecomposition};
use crate::PositionId;
use finstack_core::factor_model::{FactorCovarianceMatrix, RiskMeasure};

use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;

/// Covariance-based decomposer for linear factor risk measures.
///
/// `ParametricDecomposer` assumes the incoming [`SensitivityMatrix`] rows are already
/// position-weighted by the upstream sensitivity engine. Portfolio exposures are therefore
/// just the column sums of the matrix, and Euler allocations are computed directly from those
/// weighted exposures.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParametricDecomposer;

impl ParametricDecomposer {
    const VARIANCE_TOLERANCE: f64 = 1e-12;

    fn validate_factor_axes(
        sensitivities: &SensitivityMatrix,
        covariance: &FactorCovarianceMatrix,
    ) -> finstack_core::Result<()> {
        if sensitivities.n_factors() != covariance.n_factors() {
            return Err(finstack_core::Error::Validation(
                "SensitivityMatrix and FactorCovarianceMatrix factor dimensions do not match"
                    .to_string(),
            ));
        }

        if sensitivities.factor_ids() != covariance.factor_ids() {
            return Err(finstack_core::Error::Validation(
                "SensitivityMatrix and FactorCovarianceMatrix factor order does not match"
                    .to_string(),
            ));
        }

        if covariance.as_slice().iter().any(|entry| !entry.is_finite()) {
            return Err(finstack_core::Error::Validation(
                "Covariance matrix entries must be finite".to_string(),
            ));
        }

        let _validated_covariance = FactorCovarianceMatrix::new(
            covariance.factor_ids().to_vec(),
            covariance.as_slice().to_vec(),
        )?;

        Ok(())
    }

    fn portfolio_exposures(sensitivities: &SensitivityMatrix) -> Vec<f64> {
        let mut exposures = vec![0.0; sensitivities.n_factors()];

        for row in sensitivities
            .as_slice()
            .chunks_exact(sensitivities.n_factors())
        {
            for (exposure, delta) in exposures.iter_mut().zip(row.iter()) {
                *exposure += *delta;
            }
        }

        exposures
    }

    fn covariance_times_exposures(
        covariance: &FactorCovarianceMatrix,
        exposures: &[f64],
    ) -> Vec<f64> {
        let mut cov_times_exposure = vec![0.0; covariance.n_factors()];

        for (result, row) in cov_times_exposure
            .iter_mut()
            .zip(covariance.as_slice().chunks_exact(covariance.n_factors()))
        {
            *result = row
                .iter()
                .zip(exposures.iter())
                .map(|(entry, exposure)| entry * exposure)
                .sum();
        }

        cov_times_exposure
    }

    fn scale_for_measure(
        measure: &RiskMeasure,
        variance: f64,
    ) -> finstack_core::Result<(f64, f64)> {
        measure.validate()?;
        let variance = Self::validated_variance(variance)?;
        let sigma = variance.sqrt();

        let scaled = match measure {
            RiskMeasure::Variance => (variance, 1.0),
            RiskMeasure::Volatility => {
                if sigma > 0.0 {
                    (sigma, sigma.recip())
                } else {
                    (0.0, 0.0)
                }
            }
            RiskMeasure::VaR { confidence } => {
                let z_score = Self::normal_quantile(*confidence);
                if sigma > 0.0 {
                    (sigma * z_score, z_score * sigma.recip())
                } else {
                    (0.0, 0.0)
                }
            }
            RiskMeasure::ExpectedShortfall { confidence } => {
                let z_score = Self::normal_quantile(*confidence);
                let es_multiplier = Self::normal_pdf(z_score) / (1.0 - confidence);
                if sigma > 0.0 {
                    (sigma * es_multiplier, es_multiplier * sigma.recip())
                } else {
                    (0.0, 0.0)
                }
            }
        };

        Ok(scaled)
    }

    fn validated_variance(variance: f64) -> finstack_core::Result<f64> {
        if variance < -Self::VARIANCE_TOLERANCE {
            Err(finstack_core::Error::Validation(format!(
                "Portfolio variance must be non-negative, got {variance}"
            )))
        } else {
            Ok(variance.max(0.0))
        }
    }

    fn normal_quantile(probability: f64) -> f64 {
        const A1: f64 = -3.969_683_028_665_376e1;
        const A2: f64 = 2.209_460_984_245_205e2;
        const A3: f64 = -2.759_285_104_469_687e2;
        const A4: f64 = 1.383_577_518_672_69e2;
        const A5: f64 = -3.066_479_806_614_716e1;
        const A6: f64 = 2.506_628_277_459_239;
        const B1: f64 = -5.447_609_879_822_406e1;
        const B2: f64 = 1.615_858_368_580_409e2;
        const B3: f64 = -1.556_989_798_598_866e2;
        const B4: f64 = 6.680_131_188_771_972e1;
        const B5: f64 = -1.328_068_155_288_572e1;
        const C1: f64 = -7.784_894_002_430_293e-3;
        const C2: f64 = -3.223_964_580_411_365e-1;
        const C3: f64 = -2.400_758_277_161_838;
        const C4: f64 = -2.549_732_539_343_734;
        const C5: f64 = 4.374_664_141_464_968;
        const C6: f64 = 2.938_163_982_698_783;
        const D1: f64 = 7.784_695_709_041_462e-3;
        const D2: f64 = 3.224_671_290_700_398e-1;
        const D3: f64 = 2.445_134_137_142_996;
        const D4: f64 = 3.754_408_661_907_416;
        const P_LOW: f64 = 0.024_25;
        const P_HIGH: f64 = 1.0 - P_LOW;

        if probability < P_LOW {
            let q = (-2.0 * probability.ln()).sqrt();
            (((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6)
                / ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0)
        } else if probability > P_HIGH {
            let q = (-2.0 * (1.0 - probability).ln()).sqrt();
            -(((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6)
                / ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0)
        } else {
            let q = probability - 0.5;
            let r = q * q;
            (((((A1 * r + A2) * r + A3) * r + A4) * r + A5) * r + A6) * q
                / (((((B1 * r + B2) * r + B3) * r + B4) * r + B5) * r + 1.0)
        }
    }

    fn normal_pdf(x: f64) -> f64 {
        (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
    }
}

impl RiskDecomposer for ParametricDecomposer {
    fn decompose(
        &self,
        sensitivities: &SensitivityMatrix,
        covariance: &FactorCovarianceMatrix,
        measure: &RiskMeasure,
    ) -> finstack_core::Result<RiskDecomposition> {
        Self::validate_factor_axes(sensitivities, covariance)?;
        measure.validate()?;

        if sensitivities.n_factors() == 0 {
            return Ok(RiskDecomposition {
                total_risk: 0.0,
                measure: *measure,
                factor_contributions: Vec::new(),
                residual_risk: 0.0,
                position_factor_contributions: Vec::new(),
            });
        }

        let exposures = Self::portfolio_exposures(sensitivities);
        let cov_times_exposure = Self::covariance_times_exposures(covariance, &exposures);
        let component_variance: Vec<f64> = exposures
            .iter()
            .zip(cov_times_exposure.iter())
            .map(|(exposure, covariance_exposure)| exposure * covariance_exposure)
            .collect();
        let variance: f64 = component_variance.iter().sum();

        let (total_risk, scale) = Self::scale_for_measure(measure, variance)?;
        let factor_ids = covariance.factor_ids();

        let factor_contributions = factor_ids
            .iter()
            .zip(component_variance.iter().zip(cov_times_exposure.iter()))
            .map(
                |(factor_id, (factor_component_variance, marginal_component_variance))| {
                    let absolute_risk = factor_component_variance * scale;
                    let relative_risk = if total_risk.abs() > 0.0 {
                        absolute_risk / total_risk
                    } else {
                        0.0
                    };
                    let marginal_risk = marginal_component_variance * scale;

                    FactorContribution {
                        factor_id: factor_id.clone(),
                        absolute_risk,
                        relative_risk,
                        marginal_risk,
                    }
                },
            )
            .collect();

        let position_factor_contributions = sensitivities
            .position_ids()
            .iter()
            .zip(
                sensitivities
                    .as_slice()
                    .chunks_exact(sensitivities.n_factors()),
            )
            .flat_map(|(position_id, row)| {
                factor_ids
                    .iter()
                    .zip(row.iter().zip(cov_times_exposure.iter()))
                    .map(move |(factor_id, (delta, covariance_exposure))| {
                        PositionFactorContribution {
                            position_id: PositionId::from(position_id.clone()),
                            factor_id: factor_id.clone(),
                            risk_contribution: delta * covariance_exposure * scale,
                        }
                    })
            })
            .collect();

        Ok(RiskDecomposition {
            total_risk,
            measure: *measure,
            factor_contributions,
            residual_risk: 0.0,
            position_factor_contributions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ParametricDecomposer;
    use crate::factor_model::RiskDecomposer;
    use crate::PositionId;
    use finstack_core::factor_model::{FactorCovarianceMatrix, FactorId, RiskMeasure};
    use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;

    type TestResult = finstack_core::Result<()>;

    fn test_setup() -> finstack_core::Result<(SensitivityMatrix, FactorCovarianceMatrix)> {
        let mut sensitivities = SensitivityMatrix::zeros(
            vec!["pos-A".into(), "pos-B".into()],
            vec![FactorId::new("Rates"), FactorId::new("Credit")],
        );
        sensitivities.set_delta(0, 0, 100.0);
        sensitivities.set_delta(0, 1, 0.0);
        sensitivities.set_delta(1, 0, 0.0);
        sensitivities.set_delta(1, 1, 50.0);

        let covariance = FactorCovarianceMatrix::new(
            vec![FactorId::new("Rates"), FactorId::new("Credit")],
            vec![0.04, 0.03, 0.03, 0.09],
        )?;

        Ok((sensitivities, covariance))
    }

    #[test]
    fn test_parametric_variance_decomposition_uses_weighted_sensitivities_directly() -> TestResult {
        let (sensitivities, covariance) = test_setup()?;
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(&sensitivities, &covariance, &RiskMeasure::Variance)?;

        assert!((result.total_risk - 925.0).abs() < 1e-10);

        let Some(rates) = result
            .factor_contributions
            .iter()
            .find(|contribution| contribution.factor_id == FactorId::new("Rates"))
        else {
            return Err(finstack_core::Error::Validation(
                "rates contribution must exist".to_string(),
            ));
        };
        let Some(credit) = result
            .factor_contributions
            .iter()
            .find(|contribution| contribution.factor_id == FactorId::new("Credit"))
        else {
            return Err(finstack_core::Error::Validation(
                "credit contribution must exist".to_string(),
            ));
        };

        assert!((rates.absolute_risk - 550.0).abs() < 1e-10);
        assert!((credit.absolute_risk - 375.0).abs() < 1e-10);
        assert!((rates.relative_risk - (550.0 / 925.0)).abs() < 1e-10);
        assert!((credit.relative_risk - (375.0 / 925.0)).abs() < 1e-10);
        assert!((rates.marginal_risk - 5.5).abs() < 1e-10);
        assert!((credit.marginal_risk - 7.5).abs() < 1e-10);
        assert!((result.residual_risk).abs() < 1e-12);

        let sum_relative: f64 = result
            .factor_contributions
            .iter()
            .map(|contribution| contribution.relative_risk)
            .sum();
        assert!((sum_relative - 1.0).abs() < 1e-12);

        let expected_rows = vec![
            (PositionId::new("pos-A"), FactorId::new("Rates"), 550.0),
            (PositionId::new("pos-A"), FactorId::new("Credit"), 0.0),
            (PositionId::new("pos-B"), FactorId::new("Rates"), 0.0),
            (PositionId::new("pos-B"), FactorId::new("Credit"), 375.0),
        ];

        assert_eq!(
            result.position_factor_contributions.len(),
            expected_rows.len()
        );

        for (position_id, factor_id, expected_contribution) in expected_rows {
            let Some(row) = result
                .position_factor_contributions
                .iter()
                .find(|contribution| {
                    contribution.position_id == position_id && contribution.factor_id == factor_id
                })
            else {
                return Err(finstack_core::Error::Validation(
                    "position-factor contribution must exist".to_string(),
                ));
            };
            assert!((row.risk_contribution - expected_contribution).abs() < 1e-10);
        }

        Ok(())
    }

    #[test]
    fn test_parametric_volatility_decomposition_scales_component_variance() -> TestResult {
        let (sensitivities, covariance) = test_setup()?;
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(&sensitivities, &covariance, &RiskMeasure::Volatility)?;

        let sigma = 925.0_f64.sqrt();
        assert!((result.total_risk - sigma).abs() < 1e-10);

        let Some(rates) = result
            .factor_contributions
            .iter()
            .find(|contribution| contribution.factor_id == FactorId::new("Rates"))
        else {
            return Err(finstack_core::Error::Validation(
                "rates contribution must exist".to_string(),
            ));
        };
        let Some(credit) = result
            .factor_contributions
            .iter()
            .find(|contribution| contribution.factor_id == FactorId::new("Credit"))
        else {
            return Err(finstack_core::Error::Validation(
                "credit contribution must exist".to_string(),
            ));
        };

        assert!((rates.absolute_risk - (550.0 / sigma)).abs() < 1e-10);
        assert!((credit.absolute_risk - (375.0 / sigma)).abs() < 1e-10);
        assert!((rates.marginal_risk - (5.5 / sigma)).abs() < 1e-10);
        assert!((credit.marginal_risk - (7.5 / sigma)).abs() < 1e-10);

        Ok(())
    }

    #[test]
    fn test_parametric_var_decomposition_uses_validated_confidence_scaling() -> TestResult {
        let (sensitivities, covariance) = test_setup()?;
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(
            &sensitivities,
            &covariance,
            &RiskMeasure::VaR { confidence: 0.99 },
        )?;

        let sigma = 925.0_f64.sqrt();
        let z_99 = 2.326_347_874_040_840_8;
        let expected_var = sigma * z_99;
        assert!((result.total_risk - expected_var).abs() < 1e-6);

        let Some(rates) = result
            .factor_contributions
            .iter()
            .find(|contribution| contribution.factor_id == FactorId::new("Rates"))
        else {
            return Err(finstack_core::Error::Validation(
                "rates contribution must exist".to_string(),
            ));
        };
        assert!((rates.absolute_risk - ((550.0 / sigma) * z_99)).abs() < 1e-6);

        Ok(())
    }

    #[test]
    fn test_parametric_expected_shortfall_scales_component_variance() -> TestResult {
        let (sensitivities, covariance) = test_setup()?;
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(
            &sensitivities,
            &covariance,
            &RiskMeasure::ExpectedShortfall { confidence: 0.99 },
        )?;

        let sigma = 925.0_f64.sqrt();
        let z_99 = 2.326_347_874_040_840_8;
        let pdf = (-0.5_f64 * z_99 * z_99).exp() / (2.0_f64 * std::f64::consts::PI).sqrt();
        let es_multiplier = pdf / 0.01;

        assert!((result.total_risk - (sigma * es_multiplier)).abs() < 1e-6);

        let Some(rates) = result
            .factor_contributions
            .iter()
            .find(|contribution| contribution.factor_id == FactorId::new("Rates"))
        else {
            return Err(finstack_core::Error::Validation(
                "rates contribution must exist".to_string(),
            ));
        };
        assert!((rates.absolute_risk - ((550.0 / sigma) * es_multiplier)).abs() < 1e-6);

        Ok(())
    }

    #[test]
    fn test_parametric_zero_risk_portfolio_has_no_nan_relative_risk() -> TestResult {
        let sensitivities = SensitivityMatrix::zeros(
            vec!["cash".into()],
            vec![FactorId::new("Rates"), FactorId::new("Credit")],
        );
        let covariance = FactorCovarianceMatrix::new(
            vec![FactorId::new("Rates"), FactorId::new("Credit")],
            vec![0.04, 0.0, 0.0, 0.09],
        )?;
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(&sensitivities, &covariance, &RiskMeasure::Variance)?;

        assert!((result.total_risk).abs() < 1e-12);
        for contribution in &result.factor_contributions {
            assert!((contribution.absolute_risk).abs() < 1e-12);
            assert!((contribution.relative_risk).abs() < 1e-12);
            assert!(!contribution.relative_risk.is_nan());
        }

        Ok(())
    }

    #[test]
    fn test_parametric_rejects_factor_axis_order_mismatch() -> TestResult {
        let (sensitivities, _) = test_setup()?;
        let covariance = FactorCovarianceMatrix::new(
            vec![FactorId::new("Credit"), FactorId::new("Rates")],
            vec![0.09, 0.03, 0.03, 0.04],
        )?;
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(&sensitivities, &covariance, &RiskMeasure::Variance);

        assert!(result.is_err());
        let Err(error) = result else {
            return Err(finstack_core::Error::Validation(
                "factor order mismatch should be rejected".to_string(),
            ));
        };
        assert!(format!("{error}").contains("factor"));

        Ok(())
    }

    #[test]
    fn test_parametric_zero_factor_axes_return_empty_zero_risk_decomposition() -> TestResult {
        let sensitivities = SensitivityMatrix::zeros(vec!["cash".into()], vec![]);
        let covariance = FactorCovarianceMatrix::new(vec![], vec![])?;
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(&sensitivities, &covariance, &RiskMeasure::Variance)?;

        assert!((result.total_risk).abs() < 1e-12);
        assert!(result.factor_contributions.is_empty());
        assert!(result.position_factor_contributions.is_empty());
        assert!((result.residual_risk).abs() < 1e-12);

        Ok(())
    }

    #[test]
    fn test_parametric_rejects_negative_portfolio_variance_from_unchecked_covariance() {
        let mut sensitivities =
            SensitivityMatrix::zeros(vec!["pos-A".into()], vec![FactorId::new("Rates")]);
        sensitivities.set_delta(0, 0, 10.0);

        let covariance =
            FactorCovarianceMatrix::new_unchecked(vec![FactorId::new("Rates")], vec![-1.0]);
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(&sensitivities, &covariance, &RiskMeasure::Variance);

        assert!(result.is_err());
    }

    #[test]
    fn test_parametric_rejects_non_finite_unchecked_covariance() {
        let mut sensitivities =
            SensitivityMatrix::zeros(vec!["pos-A".into()], vec![FactorId::new("Rates")]);
        sensitivities.set_delta(0, 0, 10.0);

        let covariance =
            FactorCovarianceMatrix::new_unchecked(vec![FactorId::new("Rates")], vec![f64::NAN]);
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(&sensitivities, &covariance, &RiskMeasure::Variance);

        assert!(result.is_err());
    }

    #[test]
    fn test_parametric_rejects_asymmetric_unchecked_covariance() {
        let mut sensitivities = SensitivityMatrix::zeros(
            vec!["pos-A".into()],
            vec![FactorId::new("Rates"), FactorId::new("Credit")],
        );
        sensitivities.set_delta(0, 0, 10.0);
        sensitivities.set_delta(0, 1, 5.0);

        let covariance = FactorCovarianceMatrix::new_unchecked(
            vec![FactorId::new("Rates"), FactorId::new("Credit")],
            vec![1.0, 0.25, 0.10, 1.0],
        );
        let decomposer = ParametricDecomposer;
        let result = decomposer.decompose(&sensitivities, &covariance, &RiskMeasure::Variance);

        assert!(result.is_err());
    }
}
