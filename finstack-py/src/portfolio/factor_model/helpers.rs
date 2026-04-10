use crate::errors::core_to_py;
use finstack_core::currency::Currency;
use finstack_core::factor_model::{
    CurveType, DependencyType, FactorCovarianceMatrix, FactorModelConfig, FactorType, PricingMode,
    RiskMeasure, UnmatchedPolicy,
};
use finstack_core::market_data::bumps::BumpUnits;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyString};
use pyo3::Bound;
use pythonize::depythonize;
use std::str::FromStr;

pub(super) fn normalized_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !matches!(ch, '_' | '-' | ' '))
        .flat_map(char::to_lowercase)
        .collect()
}

pub(super) fn parse_factor_type(value: &str) -> PyResult<FactorType> {
    FactorType::from_str(value).map_err(|_| {
        PyValueError::new_err(format!(
            "Unsupported factor_type '{value}'. Expected Rates, Credit, Equity, FX, Volatility, Commodity, Inflation, or custom:<name>"
        ))
    })
}

pub(super) fn factor_type_to_string(value: &FactorType) -> String {
    match value {
        FactorType::Rates => "Rates".to_string(),
        FactorType::Credit => "Credit".to_string(),
        FactorType::Equity => "Equity".to_string(),
        FactorType::FX => "FX".to_string(),
        FactorType::Volatility => "Volatility".to_string(),
        FactorType::Commodity => "Commodity".to_string(),
        FactorType::Inflation => "Inflation".to_string(),
        FactorType::Custom(name) => format!("Custom:{name}"),
    }
}

pub(super) fn parse_pricing_mode(value: &str) -> PyResult<PricingMode> {
    PricingMode::from_str(value).map_err(|_| {
        PyValueError::new_err(format!(
            "Unsupported pricing_mode '{value}'. Expected DeltaBased or FullRepricing"
        ))
    })
}

pub(super) fn pricing_mode_to_string(value: PricingMode) -> String {
    match value {
        PricingMode::DeltaBased => "DeltaBased".to_string(),
        PricingMode::FullRepricing => "FullRepricing".to_string(),
    }
}

pub(super) fn parse_unmatched_policy(value: &str) -> PyResult<UnmatchedPolicy> {
    UnmatchedPolicy::from_str(value).map_err(|_| {
        PyValueError::new_err(format!(
            "Unsupported unmatched_policy '{value}'. Expected Strict, Residual, or Warn"
        ))
    })
}

pub(super) fn unmatched_policy_to_string(value: UnmatchedPolicy) -> String {
    match value {
        UnmatchedPolicy::Strict => "Strict".to_string(),
        UnmatchedPolicy::Residual => "Residual".to_string(),
        UnmatchedPolicy::Warn => "Warn".to_string(),
    }
}

pub(super) fn parse_dependency_type(value: &str) -> PyResult<DependencyType> {
    if normalized_name(value) == "hazard" {
        return Err(PyValueError::new_err(
            "Hazard is a CurveType, not a DependencyType. Use dependency_type='Credit' and curve_type='Hazard'",
        ));
    }
    DependencyType::from_str(value)
        .map_err(|_| PyValueError::new_err(format!("Unsupported dependency_type '{value}'")))
}

pub(super) fn dependency_type_to_string(value: DependencyType) -> String {
    match value {
        DependencyType::Discount => "Discount".to_string(),
        DependencyType::Forward => "Forward".to_string(),
        DependencyType::Credit => "Credit".to_string(),
        DependencyType::Spot => "Spot".to_string(),
        DependencyType::Vol => "Vol".to_string(),
        DependencyType::Fx => "Fx".to_string(),
        DependencyType::Series => "Series".to_string(),
    }
}

pub(super) fn parse_curve_type(value: &str) -> PyResult<CurveType> {
    CurveType::from_str(value)
        .map_err(|_| PyValueError::new_err(format!("Unsupported curve_type '{value}'")))
}

pub(super) fn curve_type_to_string(value: CurveType) -> String {
    match value {
        CurveType::Discount => "Discount".to_string(),
        CurveType::Forward => "Forward".to_string(),
        CurveType::Hazard => "Hazard".to_string(),
        CurveType::Inflation => "Inflation".to_string(),
        CurveType::BaseCorrelation => "BaseCorrelation".to_string(),
    }
}

pub(super) fn parse_bump_units(value: &str) -> PyResult<BumpUnits> {
    match normalized_name(value).as_str() {
        "bp" | "bps" | "ratebp" => Ok(BumpUnits::RateBp),
        "percent" | "pct" => Ok(BumpUnits::Percent),
        "fraction" => Ok(BumpUnits::Fraction),
        "factor" => Ok(BumpUnits::Factor),
        _ => Err(PyValueError::new_err(format!(
            "Unsupported bump units '{value}'. Expected bp, percent, fraction, or factor"
        ))),
    }
}

pub(super) fn bump_units_to_string(value: BumpUnits) -> String {
    match value {
        BumpUnits::RateBp => "bp".to_string(),
        BumpUnits::Percent => "percent".to_string(),
        BumpUnits::Fraction => "fraction".to_string(),
        BumpUnits::Factor => "factor".to_string(),
        _ => "unknown".to_string(),
    }
}

pub(super) fn parse_risk_measure(value: Option<&Bound<'_, PyAny>>) -> PyResult<RiskMeasure> {
    let Some(value) = value else {
        return Ok(RiskMeasure::Variance);
    };

    if let Ok(text) = value.extract::<String>() {
        return match normalized_name(&text).as_str() {
            "variance" => Ok(RiskMeasure::Variance),
            "volatility" => Ok(RiskMeasure::Volatility),
            "var" => Err(PyValueError::new_err(
                "VaR requires a confidence payload, for example {'var': {'confidence': 0.99}}",
            )),
            "expectedshortfall" => Err(PyValueError::new_err(
                "ExpectedShortfall requires a confidence payload, for example {'expected_shortfall': {'confidence': 0.975}}",
            )),
            _ => Err(PyValueError::new_err(format!(
                "Unsupported risk_measure '{text}'"
            ))),
        };
    }

    let json_value: serde_json::Value = depythonize(value)
        .map_err(|err| PyValueError::new_err(format!("Failed to parse risk_measure: {err}")))?;
    serde_json::from_value(json_value).map_err(|err| PyValueError::new_err(err.to_string()))
}

pub(super) fn risk_measure_to_py(py: Python<'_>, value: &RiskMeasure) -> PyResult<Py<PyAny>> {
    match value {
        RiskMeasure::Variance => Ok(PyString::new(py, "Variance").into_any().unbind()),
        RiskMeasure::Volatility => Ok(PyString::new(py, "Volatility").into_any().unbind()),
        RiskMeasure::VaR { confidence } => {
            let dict = PyDict::new(py);
            let nested = PyDict::new(py);
            nested.set_item("confidence", *confidence)?;
            dict.set_item("var", nested)?;
            Ok(dict.into())
        }
        RiskMeasure::ExpectedShortfall { confidence } => {
            let dict = PyDict::new(py);
            let nested = PyDict::new(py);
            nested.set_item("confidence", *confidence)?;
            dict.set_item("expected_shortfall", nested)?;
            Ok(dict.into())
        }
    }
}

pub(super) fn build_validated_config(config: FactorModelConfig) -> PyResult<FactorModelConfig> {
    config.risk_measure.validate().map_err(core_to_py)?;
    let factor_ids = config.covariance.factor_ids().to_vec();
    let data = config.covariance.as_slice().to_vec();
    let covariance = FactorCovarianceMatrix::new(factor_ids, data).map_err(core_to_py)?;
    Ok(FactorModelConfig {
        covariance,
        ..config
    })
}

pub(super) fn mapping_to_json<T: serde::Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string_pretty(value).map_err(|err| PyValueError::new_err(err.to_string()))
}

pub(super) fn currency_pair_string(base: Currency, quote: Currency) -> String {
    format!("{base}/{quote}")
}
