//! Centralized argument extraction types for Python bindings.
//!
//! This module provides `FromPyObject` implementations for common financial types,
//! enabling flexible argument parsing across instrument builders and other bindings.
//!
//! ## WASM Parity Note
//!
//! All logic must stay in Rust to ensure WASM bindings can share the same functionality.
//! These types only handle type conversion - no business logic or validation beyond
//! type parsing belongs here.

use crate::core::common::labels::normalize_label;
use crate::core::currency::PyCurrency;
use crate::core::dates::schedule::{PyFrequency, PyStubKind};
use crate::core::dates::PyDayCount;
use crate::core::math::interp::{PyExtrapolationPolicy, PyInterpStyle};
use crate::errors::{unknown_business_day_convention, unknown_rounding_mode};
use finstack_core::config::RoundingMode;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::dates::{BusinessDayConvention, StubKind, Tenor};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyDict};
use pyo3::FromPyObject;
use std::str::FromStr;

pub struct CurrencyArg(pub Currency);

impl<'a, 'py> FromPyObject<'a, 'py> for CurrencyArg {
    type Error = PyErr;
    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(ccy) = obj.extract::<PyRef<PyCurrency>>() {
            return Ok(CurrencyArg(ccy.inner));
        }
        if let Ok(code) = obj.extract::<&str>() {
            return Currency::from_str(code)
                .map(CurrencyArg)
                .map_err(|_| crate::errors::unknown_currency(code));
        }
        Err(PyTypeError::new_err(
            "Expected Currency instance or ISO currency code string",
        ))
    }
}

pub struct RoundingModeArg(pub RoundingMode);

impl<'a, 'py> FromPyObject<'a, 'py> for RoundingModeArg {
    type Error = PyErr;
    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(mode) = obj.extract::<PyRef<crate::core::config::PyRoundingMode>>() {
            return Ok(RoundingModeArg(mode.inner));
        }
        if let Ok(name) = obj.extract::<&str>() {
            let n = normalize_label(name);
            let m = match n.as_str() {
                "bankers" | "banker" => RoundingMode::Bankers,
                "away_from_zero" | "awayfromzero" => RoundingMode::AwayFromZero,
                "toward_zero" | "towards_zero" => RoundingMode::TowardZero,
                "floor" => RoundingMode::Floor,
                "ceil" | "ceiling" => RoundingMode::Ceil,
                other => return Err(unknown_rounding_mode(other)),
            };
            return Ok(RoundingModeArg(m));
        }
        Err(PyTypeError::new_err(
            "Expected RoundingMode or string identifier",
        ))
    }
}

pub struct BusinessDayConventionArg(pub BusinessDayConvention);

impl<'a, 'py> FromPyObject<'a, 'py> for BusinessDayConventionArg {
    type Error = PyErr;
    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(conv) =
            obj.extract::<PyRef<crate::core::dates::calendar::PyBusinessDayConvention>>()
        {
            return Ok(BusinessDayConventionArg(conv.inner));
        }
        if let Ok(name) = obj.extract::<&str>() {
            return BusinessDayConvention::from_str(name)
                .map(BusinessDayConventionArg)
                .map_err(|_| unknown_business_day_convention(name));
        }
        Err(PyTypeError::new_err(
            "Expected BusinessDayConvention or string identifier",
        ))
    }
}

pub struct DayCountArg(pub DayCount);

impl<'a, 'py> FromPyObject<'a, 'py> for DayCountArg {
    type Error = PyErr;
    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(dc) = obj.extract::<PyRef<PyDayCount>>() {
            return Ok(DayCountArg(dc.inner));
        }
        if let Ok(name) = obj.extract::<&str>() {
            let n = normalize_label(name);
            let v = match n.as_str() {
                "act/360" | "act_360" | "actual/360" => DayCount::Act360,
                "act/365f" | "act_365f" | "actual/365f" => DayCount::Act365F,
                "act/365l" | "act_365l" | "actual/365l" | "act/365afb" => DayCount::Act365L,
                "30/360" | "30_360" | "thirty/360" | "30u/360" => DayCount::Thirty360,
                "30e/360" | "30e_360" | "30/360e" => DayCount::ThirtyE360,
                "act/act" | "act_act" | "actual/actual" | "act/act isda" => DayCount::ActAct,
                "act/act isma" | "act_act_isma" | "icma" => DayCount::ActActIsma,
                "bus/252" | "bus_252" | "business/252" => DayCount::Bus252,
                other => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unknown day-count convention: {other}"
                    )))
                }
            };
            return Ok(DayCountArg(v));
        }
        Err(PyTypeError::new_err(
            "Expected DayCount or string identifier",
        ))
    }
}

pub struct InterpStyleArg(pub InterpStyle);

impl<'a, 'py> FromPyObject<'a, 'py> for InterpStyleArg {
    type Error = PyErr;
    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(py) = obj.extract::<PyRef<PyInterpStyle>>() {
            return Ok(InterpStyleArg(py.inner));
        }
        if let Ok(name) = obj.extract::<&str>() {
            let n = normalize_label(name);
            let v = match n.as_str() {
                "linear" => InterpStyle::Linear,
                "log_linear" => InterpStyle::LogLinear,
                "monotone_convex" => InterpStyle::MonotoneConvex,
                "cubic_hermite" => InterpStyle::CubicHermite,
                "piecewise_quadratic_forward" => InterpStyle::PiecewiseQuadraticForward,
                "flat_fwd" => InterpStyle::LogLinear,
                other => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unknown interpolation style: {other}"
                    )))
                }
            };
            return Ok(InterpStyleArg(v));
        }
        Err(PyTypeError::new_err(
            "Expected InterpStyle or string identifier",
        ))
    }
}

pub struct ExtrapolationPolicyArg(pub ExtrapolationPolicy);

impl<'a, 'py> FromPyObject<'a, 'py> for ExtrapolationPolicyArg {
    type Error = PyErr;
    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(py) = obj.extract::<PyRef<PyExtrapolationPolicy>>() {
            return Ok(ExtrapolationPolicyArg(py.inner));
        }
        if let Ok(name) = obj.extract::<&str>() {
            let n = normalize_label(name);
            let v = match n.as_str() {
                "flat_zero" => ExtrapolationPolicy::FlatZero,
                "flat_forward" => ExtrapolationPolicy::FlatForward,
                other => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unknown extrapolation policy: {other}"
                    )))
                }
            };
            return Ok(ExtrapolationPolicyArg(v));
        }
        Err(PyTypeError::new_err(
            "Expected ExtrapolationPolicy or string identifier",
        ))
    }
}

/// Flexible tenor/frequency argument extraction.
///
/// Accepts:
/// - `Frequency` (PyFrequency) instance
/// - String labels: "annual", "semi_annual", "quarterly", "monthly", "biweekly", "weekly", "daily"
/// - Tenor shorthand: "1y", "6m", "3m", "1m", "2w", "1w", "1d"
/// - Integer payments per year: 1, 2, 4, 12, etc.
pub struct TenorArg(pub Tenor);

impl<'a, 'py> FromPyObject<'a, 'py> for TenorArg {
    type Error = PyErr;
    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        // Try Frequency wrapper first
        if let Ok(py_freq) = obj.extract::<PyRef<PyFrequency>>() {
            return Ok(TenorArg(py_freq.inner));
        }
        // Try string parsing
        if let Ok(name) = obj.extract::<&str>() {
            let normalized = normalize_label(name);
            if let Ok(payments) = normalized.parse::<u32>() {
                return Tenor::from_payments_per_year(payments)
                    .map(TenorArg)
                    .map_err(|msg| pyo3::exceptions::PyValueError::new_err(msg.to_string()));
            }
            let tenor = match normalized.as_str() {
                "annual" | "1y" | "yearly" => Tenor::annual(),
                "semiannual" | "semi_annual" | "6m" | "semi" => Tenor::semi_annual(),
                "quarterly" | "qtr" | "3m" => Tenor::quarterly(),
                "monthly" | "1m" => Tenor::monthly(),
                "biweekly" | "2w" => Tenor::biweekly(),
                "weekly" | "1w" => Tenor::weekly(),
                "daily" | "1d" => Tenor::daily(),
                other => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unknown frequency/tenor: {other}"
                    )));
                }
            };
            return Ok(TenorArg(tenor));
        }
        // Try integer payments per year
        if let Ok(payments) = obj.extract::<u32>() {
            return Tenor::from_payments_per_year(payments)
                .map(TenorArg)
                .map_err(|msg| pyo3::exceptions::PyValueError::new_err(msg.to_string()));
        }
        Err(PyTypeError::new_err(
            "Expected Frequency, string identifier, or payments per year (int)",
        ))
    }
}

/// Flexible stub kind argument extraction.
///
/// Accepts:
/// - `StubKind` (PyStubKind) instance
/// - String labels: "none", "short_front", "short_back", "long_front", "long_back"
pub struct StubKindArg(pub StubKind);

impl<'a, 'py> FromPyObject<'a, 'py> for StubKindArg {
    type Error = PyErr;
    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        // Try StubKind wrapper first
        if let Ok(py_stub) = obj.extract::<PyRef<PyStubKind>>() {
            return Ok(StubKindArg(py_stub.inner));
        }
        // Try string parsing
        if let Ok(name) = obj.extract::<&str>() {
            let normalized = normalize_label(name);
            let stub = match normalized.as_str() {
                "none" => StubKind::None,
                "short_front" | "shortfront" => StubKind::ShortFront,
                "short_back" | "shortback" => StubKind::ShortBack,
                "long_front" | "longfront" => StubKind::LongFront,
                "long_back" | "longback" => StubKind::LongBack,
                other => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unknown stub kind: {other}"
                    )));
                }
            };
            return Ok(StubKindArg(stub));
        }
        Err(PyTypeError::new_err(
            "Expected StubKind or string identifier",
        ))
    }
}

pub fn extract_float_pairs(obj: &Bound<'_, PyAny>) -> PyResult<Vec<(f64, f64)>> {
    // 1. Try direct extraction as list of tuples
    if let Ok(vec) = obj.extract::<Vec<(f64, f64)>>() {
        return Ok(vec);
    }

    // 2. Try dict (key=float, value=float)
    if let Some(vec) = extract_from_dict(obj)? {
        return Ok(vec);
    }

    // 3. Pandas Series support (index=time, value=rate)
    if let Some(vec) = extract_from_pandas(obj)? {
        return Ok(vec);
    }

    // 4. Try iterating (works for list of lists, list of tuples, numpy 2D array, etc.)
    if let Some(vec) = extract_from_sequence(obj)? {
        return Ok(vec);
    }

    Err(PyTypeError::new_err(
        "Expected list of pairs, dict, or pandas Series (float index)",
    ))
}

fn extract_from_dict(obj: &Bound<'_, PyAny>) -> PyResult<Option<Vec<(f64, f64)>>> {
    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut results = Vec::new();
        for (k, v) in dict {
            let key = k.extract::<f64>()?;
            let val = v.extract::<f64>()?;
            results.push((key, val));
        }
        // Sort by key (time) as dicts are unordered
        //
        // NOTE: `partial_cmp` returns None for NaN, which would panic if unwrapped.
        // We use a total ordering here to avoid hard-crashing the Python extension.
        results.sort_by(|a, b| a.0.total_cmp(&b.0));
        return Ok(Some(results));
    }
    Ok(None)
}

fn extract_from_pandas(obj: &Bound<'_, PyAny>) -> PyResult<Option<Vec<(f64, f64)>>> {
    // Check for "items" method which returns iterator of (index, value)
    let Ok(items_method) = obj.getattr("items") else {
        return Ok(None);
    };
    let Ok(iter) = items_method.call0()?.try_iter() else {
        return Ok(None);
    };

    let mut results = Vec::new();
    for item in iter {
        let item = item?;
        let pair = item
            .extract::<(f64, f64)>()
            .map_err(|_| PyTypeError::new_err("Expected pair from items() iterator"))?;
        results.push(pair);
    }
    Ok(Some(results))
}

fn extract_from_sequence(obj: &Bound<'_, PyAny>) -> PyResult<Option<Vec<(f64, f64)>>> {
    if let Ok(iter) = obj.try_iter() {
        let mut results = Vec::new();
        for item in iter {
            let item = item?;
            // Try to extract as tuple or list of 2 floats
            if let Ok((a, b)) = item.extract::<(f64, f64)>() {
                results.push((a, b));
            } else if let Ok(list) = item.extract::<Vec<f64>>() {
                if list.len() == 2 {
                    results.push((list[0], list[1]));
                } else {
                    return Err(PyTypeError::new_err(format!(
                        "Expected pair of floats, got list of length {}",
                        list.len()
                    )));
                }
            } else {
                return Err(PyTypeError::new_err("Expected pair of floats"));
            }
        }
        return Ok(Some(results));
    }
    Ok(None)
}
