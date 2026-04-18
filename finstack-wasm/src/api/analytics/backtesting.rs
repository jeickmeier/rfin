use crate::utils::to_js_err;
use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

fn parse_var_method(method: &str) -> Result<fa::backtesting::VarMethod, JsValue> {
    match method.to_ascii_lowercase().as_str() {
        "historical" | "hist" => Ok(fa::backtesting::VarMethod::Historical),
        "parametric" | "gaussian" | "normal" => Ok(fa::backtesting::VarMethod::Parametric),
        "cornishfisher" | "cornish_fisher" | "cornish-fisher" | "cf" => {
            Ok(fa::backtesting::VarMethod::CornishFisher)
        }
        other => Err(JsValue::from_str(&format!(
            "unknown VaR method '{other}' (expected Historical, Parametric, or CornishFisher)"
        ))),
    }
}

fn run_rolling_var(
    returns: &[f64],
    lookback: usize,
    confidence: f64,
    method: fa::backtesting::VarMethod,
) -> (Vec<f64>, Vec<f64>) {
    match method {
        fa::backtesting::VarMethod::Historical => fa::backtesting::rolling_var_forecasts(
            returns,
            lookback,
            confidence,
            |window, level| fa::risk_metrics::value_at_risk(window, level, None),
        ),
        fa::backtesting::VarMethod::Parametric => fa::backtesting::rolling_var_forecasts(
            returns,
            lookback,
            confidence,
            |window, level| fa::risk_metrics::parametric_var(window, level, None),
        ),
        fa::backtesting::VarMethod::CornishFisher => fa::backtesting::rolling_var_forecasts(
            returns,
            lookback,
            confidence,
            |window, level| fa::risk_metrics::cornish_fisher_var(window, level, None),
        ),
    }
}

#[wasm_bindgen(js_name = classifyBreaches)]
pub fn classify_breaches(
    var_forecasts: JsValue,
    realized_pnl: JsValue,
) -> Result<JsValue, JsValue> {
    let var: Vec<f64> = serde_wasm_bindgen::from_value(var_forecasts).map_err(to_js_err)?;
    let pnl: Vec<f64> = serde_wasm_bindgen::from_value(realized_pnl).map_err(to_js_err)?;
    let breaches = fa::backtesting::classify_breaches(&var, &pnl);
    let bools: Vec<bool> = breaches
        .iter()
        .map(|b| *b == fa::backtesting::Breach::Hit)
        .collect();
    serde_wasm_bindgen::to_value(&bools).map_err(to_js_err)
}

#[wasm_bindgen(js_name = kupiecTest)]
pub fn kupiec_test(breach_count: usize, n: usize, confidence: f64) -> Result<JsValue, JsValue> {
    let result = fa::backtesting::kupiec_test(breach_count, n, confidence);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

#[wasm_bindgen(js_name = christoffersenTest)]
pub fn christoffersen_test(
    breach_indicators: JsValue,
    confidence: f64,
) -> Result<JsValue, JsValue> {
    let indicators: Vec<bool> =
        serde_wasm_bindgen::from_value(breach_indicators).map_err(to_js_err)?;
    let seq: Vec<fa::backtesting::Breach> = indicators
        .into_iter()
        .map(|b| {
            if b {
                fa::backtesting::Breach::Hit
            } else {
                fa::backtesting::Breach::Miss
            }
        })
        .collect();
    let result = fa::backtesting::christoffersen_test(&seq, confidence);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

#[wasm_bindgen(js_name = trafficLight)]
pub fn traffic_light(exceptions: usize, n: usize, confidence: f64) -> Result<JsValue, JsValue> {
    let result = fa::backtesting::traffic_light(exceptions, n, confidence);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

#[wasm_bindgen(js_name = runBacktest)]
pub fn run_backtest(
    var_forecasts: JsValue,
    realized_pnl: JsValue,
    confidence: f64,
    window_size: usize,
) -> Result<JsValue, JsValue> {
    let var: Vec<f64> = serde_wasm_bindgen::from_value(var_forecasts).map_err(to_js_err)?;
    let pnl: Vec<f64> = serde_wasm_bindgen::from_value(realized_pnl).map_err(to_js_err)?;
    let cfg = fa::backtesting::VarBacktestConfig::new()
        .with_confidence(confidence)
        .with_window_size(window_size);
    let result = fa::backtesting::run_backtest(&var, &pnl, &cfg);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

#[wasm_bindgen(js_name = rollingVarForecasts)]
pub fn rolling_var_forecasts(
    returns: JsValue,
    lookback: usize,
    confidence: f64,
    method: &str,
) -> Result<JsValue, JsValue> {
    let returns: Vec<f64> = serde_wasm_bindgen::from_value(returns).map_err(to_js_err)?;
    let method = parse_var_method(method)?;
    let result = run_rolling_var(&returns, lookback, confidence, method);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

#[wasm_bindgen(js_name = compareVarBacktests)]
pub fn compare_var_backtests(
    models: JsValue,
    realized_pnl: JsValue,
    confidence: f64,
    window_size: usize,
) -> Result<JsValue, JsValue> {
    let models: Vec<(String, Vec<f64>)> =
        serde_wasm_bindgen::from_value(models).map_err(to_js_err)?;
    let realized_pnl: Vec<f64> = serde_wasm_bindgen::from_value(realized_pnl).map_err(to_js_err)?;
    let parsed_models: Vec<(fa::backtesting::VarMethod, Vec<f64>)> = models
        .into_iter()
        .map(|(method, forecasts)| Ok((parse_var_method(&method)?, forecasts)))
        .collect::<Result<_, JsValue>>()?;
    let refs: Vec<(fa::backtesting::VarMethod, &[f64])> = parsed_models
        .iter()
        .map(|(method, forecasts)| (*method, forecasts.as_slice()))
        .collect();
    let cfg = fa::backtesting::VarBacktestConfig::new()
        .with_confidence(confidence)
        .with_window_size(window_size);
    let result = fa::backtesting::compare_var_backtests(&refs, &realized_pnl, &cfg);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

#[wasm_bindgen(js_name = pnlExplanation)]
pub fn pnl_explanation(
    hypothetical_pnl: JsValue,
    risk_theoretical_pnl: JsValue,
    var: JsValue,
) -> Result<JsValue, JsValue> {
    let hypothetical_pnl: Vec<f64> =
        serde_wasm_bindgen::from_value(hypothetical_pnl).map_err(to_js_err)?;
    let risk_theoretical_pnl: Vec<f64> =
        serde_wasm_bindgen::from_value(risk_theoretical_pnl).map_err(to_js_err)?;
    let var: Vec<f64> = serde_wasm_bindgen::from_value(var).map_err(to_js_err)?;
    let result = fa::backtesting::pnl_explanation(&hypothetical_pnl, &risk_theoretical_pnl, &var);
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}
