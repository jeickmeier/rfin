use std::fs;
use std::path::PathBuf;

fn index_dts() -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(manifest_dir.join("index.d.ts")).expect("read finstack-wasm/index.d.ts")
}

fn benchmark_script() -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(manifest_dir.join("benchmarks/bench.mjs"))
        .expect("read finstack-wasm/benchmarks/bench.mjs")
}

#[test]
fn analytics_dts_matches_runtime_hotspots() {
    let dts = index_dts();

    assert!(dts.contains("dates: string[];"));
    assert!(dts.contains("rollingGreeks(returns: number[], benchmark: number[], dates: string[], window: number, annFactor: number): RollingGreeksResult;"));
    assert!(
        dts.contains("classifyBreaches(varForecasts: number[], realizedPnl: number[]): boolean[];")
    );
    assert!(dts.contains("rollingVarForecasts(returns: number[], lookback: number, confidence: number, method: string): [number[], number[]];"));
    assert!(dts.contains("compareVarBacktests(models: [string, number[]][], realizedPnl: number[], confidence: number, windowSize: number): MultiModelComparisonJson;"));
    assert!(dts
        .contains("excessReturns(returns: number[], rf: number[], nperiods?: number): number[];"));
    assert!(dts.contains("martinRatio(cagr: number, ulcer: number): number;"));
    assert!(dts.contains("The WASM analytics namespace intentionally exposes pure functions"));
}

#[test]
fn cashflows_dts_matches_json_bridge_surface() {
    let dts = index_dts();

    assert!(dts.contains("export interface CashflowsNamespace"));
    assert!(dts
        .contains("buildCashflowSchedule(specJson: string, marketJson?: string | null): string;"));
    assert!(dts.contains("validateCashflowSchedule(scheduleJson: string): string;"));
    assert!(dts.contains("datedFlows(scheduleJson: string): string;"));
    assert!(dts.contains("accruedInterest("));
    assert!(dts.contains("bondFromCashflows("));
    assert!(dts.contains("export declare const cashflows: CashflowsNamespace;"));
}

#[test]
fn valuations_dts_exposes_direct_fx_instruments() {
    let dts = index_dts();

    assert!(dts.contains("export interface FxNamespace"));
    assert!(dts.contains("FxSpot: FxInstrumentConstructor<FxInstrument>;"));
    assert!(dts.contains("FxForward: FxInstrumentConstructor<FxInstrument>;"));
    assert!(dts.contains("FxSwap: FxInstrumentConstructor<FxInstrument>;"));
    assert!(dts.contains("Ndf: FxInstrumentConstructor<FxInstrument>;"));
    assert!(dts.contains("FxOption: FxInstrumentConstructor<FxOptionInstrument>;"));
    assert!(dts.contains("FxBarrierOption: FxInstrumentConstructor<FxOptionInstrument>;"));
    assert!(dts.contains("QuantoOption: FxInstrumentConstructor<FxOptionInstrument>;"));
    assert!(dts.contains("fx: FxNamespace;"));
    assert!(dts
        .contains("foreignRho(marketJson: string, asOf: string, model?: string | null): number;"));
    assert!(dts.contains("greeks(\n    marketJson: string,\n    asOf: string,\n    model?: string | null\n  ): Record<string, number>;"));
}

#[test]
fn portfolio_cashflow_api_uses_full_cashflow_name_everywhere() {
    let dts = index_dts();
    let bench = benchmark_script();

    assert!(dts.contains("aggregateFullCashflows(specJson: string, marketJson: string): string;"));
    assert!(!dts.contains("aggregateCashflows("));
    assert!(bench.contains("aggregateFullCashflows"));
    assert!(!bench.contains("aggregateCashflows"));
}

#[test]
fn portfolio_dts_exposes_reference_price_for_almgren_chriss() {
    let dts = index_dts();

    assert!(dts.contains("referencePrice?: number | null"));
}

#[test]
fn core_daycount_dts_exposes_context_for_context_dependent_conventions() {
    let dts = index_dts();

    assert!(dts.contains("export interface DayCountContext"));
    assert!(dts.contains("yearFractionWithContext(startEpochDays: number, endEpochDays: number, ctx: DayCountContext): number;"));
    assert!(dts.contains("DayCountContext: DayCountContextConstructor;"));
}

#[test]
fn statements_dts_matches_runtime_exports() {
    let dts = index_dts();

    assert!(dts.contains("export interface StatementsNamespace"));
    assert!(dts.contains("validateFinancialModelJson(json: string): string;"));
    assert!(dts.contains("modelNodeIds(json: string): string[];"));
    assert!(dts.contains("validateCheckSuiteSpec(json: string): string;"));
    assert!(dts.contains("export declare const statements: StatementsNamespace;"));
}
