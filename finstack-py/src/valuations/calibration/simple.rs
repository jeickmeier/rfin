use super::config::{PyCalibrationConfig, PyMultiCurveConfig};
use super::quote::PyMarketQuote;
use super::report::PyCalibrationReport;
use crate::core::common::args::CurrencyArg;
use crate::core::error::core_to_py;
use crate::core::market_data::PyMarketContext;
use crate::core::utils::py_to_date;
use finstack_core::market_data::term_structures::Seniority;
use finstack_valuations::calibration::simple_calibration::SimpleCalibration;
use finstack_valuations::calibration::CalibrationConfig;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule};
use pyo3::{Bound, Py, PyRef};
use std::collections::HashMap;
use time::Date;

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "SimpleCalibration",
    unsendable
)]
pub struct PySimpleCalibration {
    base_date: Date,
    base_currency: finstack_core::prelude::Currency,
    config: CalibrationConfig,
    entity_seniority: HashMap<String, Seniority>,
}

impl PySimpleCalibration {
    fn new_internal(
        base_date: Date,
        base_currency: finstack_core::prelude::Currency,
        config: CalibrationConfig,
        entity_seniority: HashMap<String, Seniority>,
    ) -> Self {
        Self {
            base_date,
            base_currency,
            config,
            entity_seniority,
        }
    }

    fn sync_config_from_entity(&mut self) {
        self.config.entity_seniority.clear();
        for (entity, seniority) in &self.entity_seniority {
            self.config
                .entity_seniority
                .insert(entity.clone(), *seniority);
        }
    }

    fn sync_entity_from_config(&mut self) {
        self.entity_seniority.clear();
        for (entity, seniority) in self.config.entity_seniority.iter() {
            self.entity_seniority.insert(entity.clone(), *seniority);
        }
    }

    fn parse_seniority(value: &str) -> PyResult<Seniority> {
        value
            .parse()
            .map_err(|e: String| pyo3::exceptions::PyValueError::new_err(e))
    }

    fn build(&self) -> SimpleCalibration {
        let mut config = self.config.clone();
        config.entity_seniority.clear();
        for (entity, seniority) in &self.entity_seniority {
            config.entity_seniority.insert(entity.clone(), *seniority);
        }
        let mut builder =
            SimpleCalibration::new(self.base_date, self.base_currency).with_config(config);
        for (entity, seniority) in &self.entity_seniority {
            builder = builder.with_entity_seniority(entity.clone(), *seniority);
        }
        builder
    }

    fn map_from_dict(dict: Option<Bound<'_, PyDict>>) -> PyResult<HashMap<String, Seniority>> {
        let mut map = HashMap::new();
        if let Some(py_dict) = dict {
            for (key, value) in py_dict.iter() {
                let entity = key.extract::<String>()?;
                let label = value.extract::<String>()?;
                map.insert(entity, Self::parse_seniority(&label)?);
            }
        }
        Ok(map)
    }
}

#[pymethods]
impl PySimpleCalibration {
    #[new]
    #[pyo3(signature = (base_date, base_currency, *, config=None, entity_seniority=None))]
    fn ctor(
        base_date: Bound<'_, PyAny>,
        base_currency: Bound<'_, PyAny>,
        config: Option<PyRef<PyCalibrationConfig>>,
        entity_seniority: Option<Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let date = py_to_date(&base_date)?;
        let CurrencyArg(currency) = CurrencyArg::extract_bound(&base_currency)?;
        let cfg = config
            .map(|c| c.inner.clone())
            .unwrap_or_else(CalibrationConfig::default);
        let mut seniority_map = Self::map_from_dict(entity_seniority)?;
        for (entity, seniority) in cfg.entity_seniority.iter() {
            seniority_map.insert(entity.clone(), *seniority);
        }
        let mut instance = Self::new_internal(date, currency, cfg, seniority_map);
        instance.sync_config_from_entity();
        Ok(instance)
    }

    #[getter]
    fn base_currency(&self) -> crate::core::currency::PyCurrency {
        crate::core::currency::PyCurrency::new(self.base_currency)
    }

    #[getter]
    fn base_date(&self, py: Python<'_>) -> PyResult<PyObject> {
        crate::core::utils::date_to_py(py, self.base_date)
    }

    #[getter]
    fn config(&self) -> PyCalibrationConfig {
        PyCalibrationConfig::new(self.config.clone())
    }

    fn set_config(&mut self, config: PyRef<PyCalibrationConfig>) {
        self.config = config.inner.clone();
        self.sync_entity_from_config();
    }

    fn set_multi_curve_config(&mut self, cfg: PyRef<PyMultiCurveConfig>) {
        self.config.multi_curve = cfg.inner.clone();
    }

    fn set_entity_seniority(&mut self, mapping: Bound<'_, PyDict>) -> PyResult<()> {
        let map = Self::map_from_dict(Some(mapping))?;
        self.entity_seniority = map;
        self.sync_config_from_entity();
        Ok(())
    }

    fn add_entity_seniority(&mut self, entity: &str, seniority: &str) -> PyResult<()> {
        let value = Self::parse_seniority(seniority)?;
        self.entity_seniority.insert(entity.to_string(), value);
        self.sync_config_from_entity();
        Ok(())
    }

    #[pyo3(text_signature = "(self, quotes)")]
    fn calibrate(
        &self,
        quotes: Vec<Py<PyMarketQuote>>,
    ) -> PyResult<(PyMarketContext, PyCalibrationReport)> {
        let rust_quotes = Python::with_gil(|py| -> PyResult<Vec<_>> {
            quotes
                .into_iter()
                .map(|quote| {
                    let bound = quote.bind(py);
                    Ok(bound.borrow().inner.clone())
                })
                .collect()
        })?;

        let calibrator = self.build();
        let (market, report) = calibrator.calibrate(&rust_quotes).map_err(core_to_py)?;
        Ok((
            PyMarketContext { inner: market },
            PyCalibrationReport::new(report),
        ))
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PySimpleCalibration>()?;
    Ok(vec!["SimpleCalibration"])
}
