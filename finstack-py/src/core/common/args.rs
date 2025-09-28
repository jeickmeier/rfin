use crate::core::common::labels::normalize_label;
use crate::core::currency::PyCurrency;
use crate::core::error::{unknown_business_day_convention, unknown_rounding_mode};
use crate::core::dates::PyDayCount;
use finstack_core::config::RoundingMode;
use finstack_core::currency::Currency;
use finstack_core::dates::BusinessDayConvention;
use finstack_core::dates::DayCount;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::FromPyObject;
use std::str::FromStr;

pub struct CurrencyArg(pub Currency);

impl<'py> FromPyObject<'py> for CurrencyArg {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(ccy) = obj.extract::<PyRef<PyCurrency>>() { return Ok(CurrencyArg(ccy.inner)); }
        if let Ok(code) = obj.extract::<&str>() {
            return Currency::from_str(code)
                .map(CurrencyArg)
                .map_err(|_| crate::core::error::unknown_currency(code));
        }
        Err(PyTypeError::new_err("Expected Currency instance or ISO currency code string"))
    }
}

pub struct RoundingModeArg(pub RoundingMode);

impl<'py> FromPyObject<'py> for RoundingModeArg {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
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
        Err(PyTypeError::new_err("Expected RoundingMode or string identifier"))
    }
}

pub struct BusinessDayConventionArg(pub BusinessDayConvention);

impl<'py> FromPyObject<'py> for BusinessDayConventionArg {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(conv) = obj.extract::<PyRef<crate::core::dates::calendar::PyBusinessDayConvention>>() {
            return Ok(BusinessDayConventionArg(conv.inner));
        }
        if let Ok(name) = obj.extract::<&str>() {
            let n = normalize_label(name);
            let v = match n.as_str() {
                "unadjusted" => BusinessDayConvention::Unadjusted,
                "following" => BusinessDayConvention::Following,
                "modified_following" => BusinessDayConvention::ModifiedFollowing,
                "preceding" => BusinessDayConvention::Preceding,
                "modified_preceding" => BusinessDayConvention::ModifiedPreceding,
                other => return Err(unknown_business_day_convention(other)),
            };
            return Ok(BusinessDayConventionArg(v));
        }
        Err(PyTypeError::new_err("Expected BusinessDayConvention or string identifier"))
    }
}

#[allow(dead_code)]
pub struct DayCountArg(pub DayCount);

impl<'py> FromPyObject<'py> for DayCountArg {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(dc) = obj.extract::<PyRef<PyDayCount>>() { return Ok(DayCountArg(dc.inner)); }
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
                other => return Err(pyo3::exceptions::PyValueError::new_err(format!("Unknown day-count convention: {other}"))),
            };
            return Ok(DayCountArg(v));
        }
        Err(PyTypeError::new_err("Expected DayCount or string identifier"))
    }
}

#[allow(dead_code)]
pub struct InterpStyleArg(pub InterpStyle);

impl<'py> FromPyObject<'py> for InterpStyleArg {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(py) = obj.extract::<PyRef<crate::core::market_data::interp::PyInterpStyle>>() {
            return Ok(InterpStyleArg(py.inner));
        }
        if let Ok(name) = obj.extract::<&str>() {
            let n = normalize_label(name);
            let v = match n.as_str() {
                "linear" => InterpStyle::Linear,
                "log_linear" => InterpStyle::LogLinear,
                "monotone_convex" => InterpStyle::MonotoneConvex,
                "cubic_hermite" => InterpStyle::CubicHermite,
                "flat_fwd" => InterpStyle::FlatFwd,
                other => return Err(pyo3::exceptions::PyValueError::new_err(format!("Unknown interpolation style: {other}"))),
            };
            return Ok(InterpStyleArg(v));
        }
        Err(PyTypeError::new_err("Expected InterpStyle or string identifier"))
    }
}

#[allow(dead_code)]
pub struct ExtrapolationPolicyArg(pub ExtrapolationPolicy);

impl<'py> FromPyObject<'py> for ExtrapolationPolicyArg {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(py) = obj.extract::<PyRef<crate::core::market_data::interp::PyExtrapolationPolicy>>() {
            return Ok(ExtrapolationPolicyArg(py.inner));
        }
        if let Ok(name) = obj.extract::<&str>() {
            let n = normalize_label(name);
            let v = match n.as_str() {
                "flat_zero" => ExtrapolationPolicy::FlatZero,
                "flat_forward" => ExtrapolationPolicy::FlatForward,
                other => return Err(pyo3::exceptions::PyValueError::new_err(format!("Unknown extrapolation policy: {other}"))),
            };
            return Ok(ExtrapolationPolicyArg(v));
        }
        Err(PyTypeError::new_err("Expected ExtrapolationPolicy or string identifier"))
    }
}

