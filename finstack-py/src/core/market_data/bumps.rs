use crate::core::currency::PyCurrency;
use crate::core::dates::utils::py_to_date;
use crate::core::types::PyCurveId;
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;

#[pyclass(module = "finstack.core.market_data.bumps", name = "BumpMode", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyBumpMode {
    pub(crate) inner: BumpMode,
}

#[pymethods]
impl PyBumpMode {
    #[classattr]
    const ADDITIVE: Self = Self {
        inner: BumpMode::Additive,
    };

    #[classattr]
    const MULTIPLICATIVE: Self = Self {
        inner: BumpMode::Multiplicative,
    };

    fn __repr__(&self) -> &'static str {
        match self.inner {
            BumpMode::Additive => "BumpMode.ADDITIVE",
            BumpMode::Multiplicative => "BumpMode.MULTIPLICATIVE",
            _ => "BumpMode.UNKNOWN_VARIANT",
        }
    }
}

#[pyclass(module = "finstack.core.market_data.bumps", name = "BumpUnits", frozen)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyBumpUnits {
    pub(crate) inner: BumpUnits,
}

#[pymethods]
impl PyBumpUnits {
    #[classattr]
    const RATE_BP: Self = Self {
        inner: BumpUnits::RateBp,
    };

    #[classattr]
    const PERCENT: Self = Self {
        inner: BumpUnits::Percent,
    };

    #[classattr]
    const FRACTION: Self = Self {
        inner: BumpUnits::Fraction,
    };

    #[classattr]
    const FACTOR: Self = Self {
        inner: BumpUnits::Factor,
    };

    fn __repr__(&self) -> &'static str {
        match self.inner {
            BumpUnits::RateBp => "BumpUnits.RATE_BP",
            BumpUnits::Percent => "BumpUnits.PERCENT",
            BumpUnits::Fraction => "BumpUnits.FRACTION",
            BumpUnits::Factor => "BumpUnits.FACTOR",
            _ => "BumpUnits.UNKNOWN_VARIANT",
        }
    }
}

#[pyclass(module = "finstack.core.market_data.bumps", name = "BumpType", frozen)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PyBumpType {
    pub(crate) inner: BumpType,
}

#[pymethods]
impl PyBumpType {
    #[classattr]
    const PARALLEL: Self = Self {
        inner: BumpType::Parallel,
    };

    #[staticmethod]
    #[pyo3(text_signature = "(time_years)")]
    fn key_rate(time_years: f64) -> Self {
        Self {
            inner: BumpType::KeyRate { time_years },
        }
    }

    #[getter]
    fn is_key_rate(&self) -> bool {
        matches!(self.inner, BumpType::KeyRate { .. })
    }

    #[getter]
    fn time_years(&self) -> Option<f64> {
        match self.inner {
            BumpType::KeyRate { time_years } => Some(time_years),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            BumpType::Parallel => "BumpType.PARALLEL".to_string(),
            BumpType::KeyRate { time_years } => format!("BumpType.KeyRate({time_years})"),
        }
    }
}

#[pyclass(module = "finstack.core.market_data.bumps", name = "BumpSpec", frozen)]
#[derive(Clone, Debug)]
pub struct PyBumpSpec {
    pub(crate) inner: BumpSpec,
}

#[pymethods]
impl PyBumpSpec {
    #[new]
    #[pyo3(signature = (mode, units, value, bump_type=None))]
    fn ctor(
        mode: PyRef<'_, PyBumpMode>,
        units: PyRef<'_, PyBumpUnits>,
        value: f64,
        bump_type: Option<PyRef<'_, PyBumpType>>,
    ) -> Self {
        Self {
            inner: BumpSpec {
                mode: mode.inner,
                units: units.inner,
                value,
                bump_type: bump_type.map(|b| b.inner).unwrap_or(BumpType::Parallel),
            },
        }
    }

    #[staticmethod]
    fn parallel_bp(bump_bp: f64) -> Self {
        Self {
            inner: BumpSpec::parallel_bp(bump_bp),
        }
    }

    #[staticmethod]
    fn key_rate_bp(time_years: f64, bump_bp: f64) -> Self {
        Self {
            inner: BumpSpec::key_rate_bp(time_years, bump_bp),
        }
    }

    #[staticmethod]
    fn multiplier(factor: f64) -> Self {
        Self {
            inner: BumpSpec::multiplier(factor),
        }
    }

    #[staticmethod]
    fn inflation_shift_pct(bump_pct: f64) -> Self {
        Self {
            inner: BumpSpec::inflation_shift_pct(bump_pct),
        }
    }

    #[staticmethod]
    fn correlation_shift_pct(bump_pct: f64) -> Self {
        Self {
            inner: BumpSpec::correlation_shift_pct(bump_pct),
        }
    }

    #[getter]
    fn mode(&self) -> PyBumpMode {
        PyBumpMode {
            inner: self.inner.mode,
        }
    }

    #[getter]
    fn units(&self) -> PyBumpUnits {
        PyBumpUnits {
            inner: self.inner.units,
        }
    }

    #[getter]
    fn value(&self) -> f64 {
        self.inner.value
    }

    #[getter]
    fn bump_type(&self) -> PyBumpType {
        PyBumpType {
            inner: self.inner.bump_type,
        }
    }

    fn __repr__(&self) -> String {
        let mode = PyBumpMode {
            inner: self.inner.mode,
        };
        let units = PyBumpUnits {
            inner: self.inner.units,
        };
        let bump_type = PyBumpType {
            inner: self.inner.bump_type,
        };
        format!(
            "BumpSpec(mode={}, units={}, value={}, bump_type={})",
            mode.__repr__(),
            units.__repr__(),
            self.inner.value,
            bump_type.__repr__()
        )
    }
}

#[pyclass(
    module = "finstack.core.market_data.bumps",
    name = "MarketBump",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyMarketBump {
    pub(crate) inner: MarketBump,
}

#[pymethods]
impl PyMarketBump {
    #[classmethod]
    #[pyo3(text_signature = "(cls, curve_id, spec)")]
    fn curve(_cls: &Bound<'_, PyType>, curve_id: &PyCurveId, spec: &PyBumpSpec) -> Self {
        Self {
            inner: MarketBump::Curve {
                id: curve_id.inner.clone(),
                spec: spec.inner,
            },
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, base_currency, quote_currency, pct, as_of)")]
    fn fx_pct(
        _cls: &Bound<'_, PyType>,
        base_currency: PyRef<'_, PyCurrency>,
        quote_currency: PyRef<'_, PyCurrency>,
        pct: f64,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let date = py_to_date(&as_of)?;
        Ok(Self {
            inner: MarketBump::FxPct {
                base: base_currency.inner,
                quote: quote_currency.inner,
                pct,
                as_of: date,
            },
        })
    }

    #[classmethod]
    #[pyo3(signature = (surface_id, pct, expiries=None, strikes=None))]
    fn vol_bucket_pct(
        _cls: &Bound<'_, PyType>,
        surface_id: &PyCurveId,
        pct: f64,
        expiries: Option<Vec<f64>>,
        strikes: Option<Vec<f64>>,
    ) -> Self {
        Self {
            inner: MarketBump::VolBucketPct {
                surface_id: surface_id.inner.clone(),
                expiries,
                strikes,
                pct,
            },
        }
    }

    #[classmethod]
    #[pyo3(signature = (surface_id, points, detachments=None))]
    fn base_corr_bucket_pts(
        _cls: &Bound<'_, PyType>,
        surface_id: &PyCurveId,
        points: f64,
        detachments: Option<Vec<f64>>,
    ) -> Self {
        Self {
            inner: MarketBump::BaseCorrBucketPts {
                surface_id: surface_id.inner.clone(),
                detachments,
                points,
            },
        }
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner {
            MarketBump::Curve { .. } => "curve",
            MarketBump::FxPct { .. } => "fx_pct",
            MarketBump::VolBucketPct { .. } => "vol_bucket_pct",
            MarketBump::BaseCorrBucketPts { .. } => "base_corr_bucket_pts",
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            MarketBump::Curve { id, .. } => format!("MarketBump.Curve(id='{}')", id),
            MarketBump::FxPct {
                base, quote, pct, ..
            } => format!(
                "MarketBump.FxPct(base={:?}, quote={:?}, pct={})",
                base, quote, pct
            ),
            MarketBump::VolBucketPct {
                surface_id, pct, ..
            } => format!(
                "MarketBump.VolBucketPct(surface='{}', pct={})",
                surface_id, pct
            ),
            MarketBump::BaseCorrBucketPts {
                surface_id, points, ..
            } => format!(
                "MarketBump.BaseCorrBucketPts(surface='{}', points={})",
                surface_id, points
            ),
        }
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "bumps")?;
    module.setattr(
        "__doc__",
        "Bump specifications and market bump helpers for scenario generation.",
    )?;
    module.add_class::<PyBumpMode>()?;
    module.add_class::<PyBumpUnits>()?;
    module.add_class::<PyBumpType>()?;
    module.add_class::<PyBumpSpec>()?;
    module.add_class::<PyMarketBump>()?;

    let exports = [
        "BumpMode",
        "BumpUnits",
        "BumpType",
        "BumpSpec",
        "MarketBump",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
