use std::fs;
use std::path::PathBuf;

fn index_dts() -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(manifest_dir.join("index.d.ts")).expect("read finstack-wasm/index.d.ts")
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
fn core_daycount_dts_exposes_context_for_context_dependent_conventions() {
    let dts = index_dts();

    assert!(dts.contains("export interface DayCountContext"));
    assert!(dts.contains("yearFractionWithContext(startEpochDays: number, endEpochDays: number, ctx: DayCountContext): number;"));
    assert!(dts.contains("DayCountContext: DayCountContextConstructor;"));
}
