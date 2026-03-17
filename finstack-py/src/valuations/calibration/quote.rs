use crate::core::common::args::CurrencyArg;
use crate::core::dates::schedule::PyFrequency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::dates::PyTenor;
use finstack_core::dates::Tenor;
use finstack_core::types::{CurveId, UnderlyingId};
use finstack_valuations::instruments::OptionType;
use finstack_valuations::market::conventions::ids::{
    CdsConventionKey, CdsDocClause, IndexId, InflationSwapConventionId, IrFutureContractId,
    OptionConventionId, SwaptionConventionId,
};
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::cds_tranche::CDSTrancheQuote;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::inflation::InflationQuote;
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use finstack_valuations::market::quotes::rates::RateQuote as RatesQuote;
use finstack_valuations::market::quotes::vol::VolQuote;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyType};
use pyo3::{Bound, PyRef};
use std::str::FromStr;

fn parse_pillar(obj: Bound<'_, PyAny>) -> PyResult<Pillar> {
    if let Ok(tenor) = obj.extract::<PyRef<'_, PyTenor>>() {
        return Ok(Pillar::Tenor(tenor.inner));
    }
    if let Ok(text) = obj.extract::<&str>() {
        if let Ok(tenor) = Tenor::parse(text) {
            return Ok(Pillar::Tenor(tenor));
        }
    }
    py_to_date(&obj).map(Pillar::Date)
}

fn parse_index(obj: Bound<'_, PyAny>) -> PyResult<IndexId> {
    if let Ok(text) = obj.extract::<&str>() {
        return Ok(IndexId::new(text));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "Expected index id string like 'USD-SOFR-3M'",
    ))
}

fn parse_doc_clause(text: &str) -> PyResult<CdsDocClause> {
    CdsDocClause::from_str(text).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

fn parse_option_type(text: &str) -> PyResult<OptionType> {
    OptionType::from_str(text).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "RatesQuote",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRatesQuote {
    pub(crate) inner: RatesQuote,
}

impl PyRatesQuote {
    pub(crate) fn new(inner: RatesQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRatesQuote {
    #[classmethod]
    #[pyo3(
        signature = (id, expiry, price, *, contract=None, convexity_adjustment=None, vol_surface_id=None),
        text_signature = "(cls, id, expiry, price, *, contract=None, convexity_adjustment=None, vol_surface_id=None)"
    )]
    fn future(
        _cls: &Bound<'_, PyType>,
        id: &str,
        expiry: Bound<'_, PyAny>,
        price: f64,
        contract: Option<&str>,
        convexity_adjustment: Option<f64>,
        vol_surface_id: Option<&str>,
    ) -> PyResult<Self> {
        let expiry_date = py_to_date(&expiry)?;
        let contract_id = IrFutureContractId::new(contract.unwrap_or("UNKNOWN"));
        Ok(Self::new(RatesQuote::Futures {
            id: QuoteId::new(id),
            contract: contract_id,
            expiry: expiry_date,
            price,
            convexity_adjustment,
            vol_surface_id: vol_surface_id.map(CurveId::new),
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (id, index, maturity, rate),
        text_signature = "(cls, id, index, maturity, rate)"
    )]
    fn deposit(
        _cls: &Bound<'_, PyType>,
        id: &str,
        index: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        rate: f64,
    ) -> PyResult<Self> {
        let index_id = parse_index(index)?;
        let pillar = parse_pillar(maturity)?;
        Ok(Self::new(RatesQuote::Deposit {
            id: QuoteId::new(id),
            index: index_id,
            pillar,
            rate,
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (id, index, start, end, rate),
        text_signature = "(cls, id, index, start, end, rate)"
    )]
    fn fra(
        _cls: &Bound<'_, PyType>,
        id: &str,
        index: Bound<'_, PyAny>,
        start: Bound<'_, PyAny>,
        end: Bound<'_, PyAny>,
        rate: f64,
    ) -> PyResult<Self> {
        let index_id = parse_index(index)?;
        let start_pillar = parse_pillar(start)?;
        let end_pillar = parse_pillar(end)?;
        Ok(Self::new(RatesQuote::Fra {
            id: QuoteId::new(id),
            index: index_id,
            start: start_pillar,
            end: end_pillar,
            rate,
        }))
    }

    #[classmethod]
    #[pyo3(
        signature = (id, index, maturity, rate, *, spread_decimal=None),
        text_signature = "(cls, id, index, maturity, rate, *, spread_decimal=None)"
    )]
    /// Create a swap rate quote.
    ///
    /// Args:
    ///     id: Quote identifier.
    ///     index: Rate index (e.g., "USD-SOFR-3M").
    ///     maturity: Swap maturity as Tenor or Date.
    ///     rate: Par swap rate (decimal, e.g., 0.05 for 5%).
    ///     spread_decimal: Optional spread in decimal format (e.g., 0.0010 for 10bp).
    ///                     Note: This is in decimal, not basis points.
    ///
    /// Returns:
    ///     RatesQuote: Swap rate quote.
    ///
    /// Examples:
    ///     >>> # 5Y swap at 5% with 10bp spread:
    ///     >>> quote = RatesQuote.swap(
    ///     ...     "swap_5y",
    ///     ...     "USD-SOFR-3M",
    ///     ...     "5Y",
    ///     ...     0.05,
    ///     ...     spread_decimal=0.0010
    ///     ... )
    fn swap(
        _cls: &Bound<'_, PyType>,
        id: &str,
        index: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        rate: f64,
        spread_decimal: Option<f64>,
    ) -> PyResult<Self> {
        let index_id = parse_index(index)?;
        let pillar = parse_pillar(maturity)?;
        Ok(Self::new(RatesQuote::Swap {
            id: QuoteId::new(id),
            index: index_id,
            pillar,
            rate,
            spread_decimal,
        }))
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            RatesQuote::Deposit { .. } => "deposit",
            RatesQuote::Fra { .. } => "fra",
            RatesQuote::Futures { .. } => "futures",
            RatesQuote::Swap { .. } => "swap",
        }
    }

    fn to_market_quote(&self) -> PyMarketQuote {
        PyMarketQuote::new(MarketQuote::Rates(self.inner.clone()))
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        match &self.inner {
            RatesQuote::Deposit {
                id, pillar, rate, ..
            } => Ok(format!(
                "RatesQuote.deposit(id='{}', pillar='{}', rate={:.6})",
                id, pillar, rate
            )),
            RatesQuote::Fra {
                id,
                start,
                end,
                rate,
                ..
            } => Ok(format!(
                "RatesQuote.fra(id='{}', start='{}', end='{}', rate={:.6})",
                id, start, end, rate
            )),
            RatesQuote::Futures {
                id, expiry, price, ..
            } => {
                let expiry_py = date_to_py(py, *expiry)?;
                Ok(format!(
                    "RatesQuote.futures(id='{}', expiry={}, price={:.6})",
                    id, expiry_py, price
                ))
            }
            RatesQuote::Swap {
                id,
                pillar,
                rate,
                spread_decimal,
                ..
            } => Ok(format!(
                "RatesQuote.swap(id='{}', pillar='{}', rate={:.6}, spread_decimal={:?})",
                id, pillar, rate, spread_decimal
            )),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "CreditQuote",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCreditQuote {
    pub(crate) inner: MarketQuote,
}

impl PyCreditQuote {
    pub(crate) fn new(inner: MarketQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCreditQuote {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, id, entity, pillar, spread_bp, recovery_rate, currency, doc_clause='XR14')"
    )]
    #[allow(clippy::too_many_arguments)]
    fn cds_par_spread(
        _cls: &Bound<'_, PyType>,
        id: &str,
        entity: &str,
        pillar: Bound<'_, PyAny>,
        spread_bp: f64,
        recovery_rate: f64,
        currency: Bound<'_, PyAny>,
        doc_clause: Option<&str>,
    ) -> PyResult<Self> {
        let CurrencyArg(ccy) = currency.extract::<CurrencyArg>()?;
        let doc = parse_doc_clause(doc_clause.unwrap_or("XR14"))?;
        let pillar = parse_pillar(pillar)?;
        Ok(Self::new(MarketQuote::Cds(CdsQuote::CdsParSpread {
            id: QuoteId::new(id),
            entity: entity.to_string(),
            convention: CdsConventionKey {
                currency: ccy,
                doc_clause: doc,
            },
            pillar,
            spread_bp,
            recovery_rate,
        })))
    }

    #[classmethod]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "(cls, id, entity, pillar, upfront_pct, running_spread_bp, recovery_rate, currency, doc_clause='XR14')"
    )]
    fn cds_upfront(
        _cls: &Bound<'_, PyType>,
        id: &str,
        entity: &str,
        pillar: Bound<'_, PyAny>,
        upfront_pct: f64,
        running_spread_bp: f64,
        recovery_rate: f64,
        currency: Bound<'_, PyAny>,
        doc_clause: Option<&str>,
    ) -> PyResult<Self> {
        let CurrencyArg(ccy) = currency.extract::<CurrencyArg>()?;
        let doc = parse_doc_clause(doc_clause.unwrap_or("XR14"))?;
        let pillar = parse_pillar(pillar)?;
        Ok(Self::new(MarketQuote::Cds(CdsQuote::CdsUpfront {
            id: QuoteId::new(id),
            entity: entity.to_string(),
            convention: CdsConventionKey {
                currency: ccy,
                doc_clause: doc,
            },
            pillar,
            running_spread_bp,
            upfront_pct,
            recovery_rate,
        })))
    }

    #[classmethod]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        text_signature = "(cls, id, index, attachment, detachment, maturity, upfront_pct, running_spread_bp, currency, doc_clause='XR14')"
    )]
    fn cds_tranche(
        _cls: &Bound<'_, PyType>,
        id: &str,
        index: &str,
        attachment: f64,
        detachment: f64,
        maturity: Bound<'_, PyAny>,
        upfront_pct: f64,
        running_spread_bp: f64,
        currency: Bound<'_, PyAny>,
        doc_clause: Option<&str>,
    ) -> PyResult<Self> {
        let maturity_date = py_to_date(&maturity)?;
        let CurrencyArg(ccy) = currency.extract::<CurrencyArg>()?;
        let doc = parse_doc_clause(doc_clause.unwrap_or("XR14"))?;
        Ok(Self::new(MarketQuote::CDSTranche(
            CDSTrancheQuote::CDSTranche {
                id: QuoteId::new(id),
                index: index.to_string(),
                attachment,
                detachment,
                maturity: maturity_date,
                upfront_pct,
                running_spread_bp,
                convention: CdsConventionKey {
                    currency: ccy,
                    doc_clause: doc,
                },
            },
        )))
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            MarketQuote::Cds(cds) => match cds {
                CdsQuote::CdsParSpread { .. } => "cds_par_spread",
                CdsQuote::CdsUpfront { .. } => "cds_upfront",
            },
            MarketQuote::CDSTranche(_) => "cds_tranche",
            _ => "unknown",
        }
    }

    fn to_market_quote(&self) -> PyMarketQuote {
        PyMarketQuote::new(self.inner.clone())
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            MarketQuote::Cds(cds) => match cds {
                CdsQuote::CdsParSpread { entity, spread_bp, .. } => {
                    format!(
                        "CreditQuote.cds_par_spread(entity='{}', spread_bp={:.6})",
                        entity, spread_bp
                    )
                }
                CdsQuote::CdsUpfront {
                    entity,
                    upfront_pct,
                    running_spread_bp,
                    ..
                } => format!(
                    "CreditQuote.cds_upfront(entity='{}', upfront_pct={:.6}, running_spread_bp={:.6})",
                    entity, upfront_pct, running_spread_bp
                ),
            },
            MarketQuote::CDSTranche(CDSTrancheQuote::CDSTranche {
                index,
                attachment,
                detachment,
                ..
            }) => format!(
                "CreditQuote.cds_tranche(index='{}', attachment={:.4}, detachment={:.4})",
                index, attachment, detachment
            ),
            _ => "CreditQuote(unsupported)".to_string(),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "VolQuote",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyVolQuote {
    pub(crate) inner: VolQuote,
}

impl PyVolQuote {
    pub(crate) fn new(inner: VolQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolQuote {
    #[classmethod]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(text_signature = "(cls, underlying, expiry, strike, vol, option_type, convention)")]
    fn option_vol(
        _cls: &Bound<'_, PyType>,
        underlying: &str,
        expiry: Bound<'_, PyAny>,
        strike: f64,
        vol: f64,
        option_type: &str,
        convention: &str,
    ) -> PyResult<Self> {
        let expiry_date = py_to_date(&expiry)?;
        let option_type = parse_option_type(option_type)?;
        Ok(Self::new(VolQuote::OptionVol {
            underlying: UnderlyingId::new(underlying),
            expiry: expiry_date,
            strike,
            vol,
            option_type,
            convention: OptionConventionId::new(convention),
        }))
    }

    #[classmethod]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(text_signature = "(cls, expiry, maturity, strike, vol, quote_type, convention)")]
    fn swaption_vol(
        _cls: &Bound<'_, PyType>,
        expiry: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        strike: f64,
        vol: f64,
        quote_type: &str,
        convention: &str,
    ) -> PyResult<Self> {
        let expiry_date = py_to_date(&expiry)?;
        let maturity_date = py_to_date(&maturity)?;
        Ok(Self::new(VolQuote::SwaptionVol {
            expiry: expiry_date,
            maturity: maturity_date,
            strike,
            vol,
            quote_type: quote_type.to_string(),
            convention: SwaptionConventionId::new(convention),
        }))
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            VolQuote::OptionVol { .. } => "option",
            VolQuote::SwaptionVol { .. } => "swaption",
        }
    }

    fn to_market_quote(&self) -> PyMarketQuote {
        PyMarketQuote::new(MarketQuote::Vol(self.inner.clone()))
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            VolQuote::OptionVol {
                underlying, strike, ..
            } => format!(
                "VolQuote.option_vol(underlying='{}', strike={:.6})",
                underlying, strike
            ),
            VolQuote::SwaptionVol {
                strike, quote_type, ..
            } => format!(
                "VolQuote.swaption_vol(strike={:.6}, quote_type='{}')",
                strike, quote_type
            ),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "InflationQuote",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyInflationQuote {
    pub(crate) inner: InflationQuote,
}

impl PyInflationQuote {
    pub(crate) fn new(inner: InflationQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInflationQuote {
    #[classmethod]
    #[pyo3(text_signature = "(cls, maturity, rate, index, convention)")]
    fn inflation_swap(
        _cls: &Bound<'_, PyType>,
        maturity: Bound<'_, PyAny>,
        rate: f64,
        index: &str,
        convention: &str,
    ) -> PyResult<Self> {
        let maturity_date = py_to_date(&maturity)?;
        Ok(Self::new(InflationQuote::InflationSwap {
            maturity: maturity_date,
            rate,
            index: index.to_string(),
            convention: InflationSwapConventionId::new(convention),
        }))
    }

    #[classmethod]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(text_signature = "(cls, maturity, rate, index, frequency, convention)")]
    fn yoy_inflation_swap(
        _cls: &Bound<'_, PyType>,
        maturity: Bound<'_, PyAny>,
        rate: f64,
        index: &str,
        frequency: PyRef<PyFrequency>,
        convention: &str,
    ) -> PyResult<Self> {
        let maturity_date = py_to_date(&maturity)?;
        Ok(Self::new(InflationQuote::YoYInflationSwap {
            maturity: maturity_date,
            rate,
            index: index.to_string(),
            frequency: frequency.inner,
            convention: InflationSwapConventionId::new(convention),
        }))
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            InflationQuote::InflationSwap { .. } => "inflation_swap",
            InflationQuote::YoYInflationSwap { .. } => "yoy_inflation_swap",
        }
    }

    fn to_market_quote(&self) -> PyMarketQuote {
        PyMarketQuote::new(MarketQuote::Inflation(self.inner.clone()))
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            InflationQuote::InflationSwap { index, rate, .. } => format!(
                "InflationQuote.inflation_swap(index='{}', rate={:.6})",
                index, rate
            ),
            InflationQuote::YoYInflationSwap { index, rate, .. } => format!(
                "InflationQuote.yoy_inflation_swap(index='{}', rate={:.6})",
                index, rate
            ),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "MarketQuote",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyMarketQuote {
    pub(crate) inner: MarketQuote,
}

impl PyMarketQuote {
    pub(crate) fn new(inner: MarketQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarketQuote {
    #[classmethod]
    fn from_rates(_cls: &Bound<'_, PyType>, quote: PyRef<PyRatesQuote>) -> Self {
        Self::new(MarketQuote::Rates(quote.inner.clone()))
    }

    #[classmethod]
    fn from_credit(_cls: &Bound<'_, PyType>, quote: PyRef<PyCreditQuote>) -> Self {
        Self::new(quote.inner.clone())
    }

    #[classmethod]
    fn from_vol(_cls: &Bound<'_, PyType>, quote: PyRef<PyVolQuote>) -> Self {
        Self::new(MarketQuote::Vol(quote.inner.clone()))
    }

    #[classmethod]
    fn from_inflation(_cls: &Bound<'_, PyType>, quote: PyRef<PyInflationQuote>) -> Self {
        Self::new(MarketQuote::Inflation(quote.inner.clone()))
    }

    #[getter]
    fn kind(&self) -> &'static str {
        match &self.inner {
            MarketQuote::Bond(_) => "bond",
            MarketQuote::Rates(_) => "rates",
            MarketQuote::Cds(_) => "cds",
            MarketQuote::CDSTranche(_) => "cds_tranche",
            MarketQuote::Fx(_) => "fx",
            MarketQuote::Vol(_) => "vol",
            MarketQuote::Inflation(_) => "inflation",
            MarketQuote::Xccy(_) => "xccy",
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            MarketQuote::Bond(_) => "MarketQuote.from_bond(bond)".to_string(),
            MarketQuote::Rates(q) => match q {
                RatesQuote::Deposit { .. } => "MarketQuote.from_rates(deposit)".to_string(),
                RatesQuote::Fra { .. } => "MarketQuote.from_rates(fra)".to_string(),
                RatesQuote::Futures { .. } => "MarketQuote.from_rates(futures)".to_string(),
                RatesQuote::Swap { .. } => "MarketQuote.from_rates(swap)".to_string(),
            },
            MarketQuote::Cds(q) => match q {
                CdsQuote::CdsParSpread { .. } => {
                    "MarketQuote.from_credit(cds_par_spread)".to_string()
                }
                CdsQuote::CdsUpfront { .. } => "MarketQuote.from_credit(cds_upfront)".to_string(),
            },
            MarketQuote::CDSTranche(_) => "MarketQuote.from_credit(cds_tranche)".to_string(),
            MarketQuote::Fx(_) => "MarketQuote.from_fx(fx)".to_string(),
            MarketQuote::Vol(q) => match q {
                VolQuote::OptionVol { .. } => "MarketQuote.from_vol(option)".to_string(),
                VolQuote::SwaptionVol { .. } => "MarketQuote.from_vol(swaption)".to_string(),
            },
            MarketQuote::Inflation(q) => match q {
                InflationQuote::InflationSwap { .. } => {
                    "MarketQuote.from_inflation(inflation_swap)".to_string()
                }
                InflationQuote::YoYInflationSwap { .. } => {
                    "MarketQuote.from_inflation(yoy_inflation_swap)".to_string()
                }
            },
            MarketQuote::Xccy(_) => "MarketQuote.from_xccy(xccy)".to_string(),
        }
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyRatesQuote>()?;
    module.add_class::<PyCreditQuote>()?;
    module.add_class::<PyVolQuote>()?;
    module.add_class::<PyInflationQuote>()?;
    module.add_class::<PyMarketQuote>()?;
    Ok(vec![
        "RatesQuote",
        "CreditQuote",
        "VolQuote",
        "InflationQuote",
        "MarketQuote",
    ])
}

#[cfg(test)]
mod tests {
    use super::parse_option_type;
    use finstack_valuations::instruments::OptionType;

    #[test]
    fn parse_option_type_accepts_supported_aliases() {
        assert!(matches!(parse_option_type("call"), Ok(OptionType::Call)));
        assert!(matches!(
            parse_option_type("sell_protection"),
            Ok(OptionType::Put)
        ));
    }

    #[test]
    fn parse_option_type_rejects_unknown_labels() {
        assert!(parse_option_type("straddle").is_err());
    }
}
