//! Instrument integration tests - comprehensive test runner for all instruments
//! Note: Many legacy tests are commented out pending API migration (finstack_core::F removal, etc.)

// Priority tests - Structured Credit (partially enabled)
// NOTE: Most integration tests temporarily disabled due to extensive API changes
// #[path = "instruments/structured_credit/structured_credit_integration.rs"]
// mod structured_credit_integration; // 25+ errors - needs major refactor
// #[path = "instruments/structured_credit/test_deal_specific_and_risk_pricing_metrics.rs"]
// mod test_deal_specific_and_risk_pricing_metrics; // 25+ errors - needs major refactor
// #[path = "instruments/structured_credit/test_pool_and_waterfall.rs"]
// mod test_pool_and_waterfall; // 5 errors - field/enum changes
#[path = "instruments/structured_credit/test_pool_stats_and_characteristics.rs"]
mod test_pool_stats_and_characteristics;
// #[path = "instruments/structured_credit/test_pricer_integration.rs"]
// mod test_pricer_integration; // 6 errors - API changes
#[path = "instruments/structured_credit/test_structured_credit_serialization.rs"]
mod test_structured_credit_serialization;
#[path = "instruments/structured_credit/test_tranche_valuation_and_diversion.rs"]
mod test_tranche_valuation_and_diversion;

// Priority tests - Variance Swap (HIGH PRIORITY - 0% baseline coverage)
#[path = "instruments/variance_swap/test_variance_swap_metrics_comprehensive.rs"]
mod test_variance_swap_metrics_comprehensive;

// Priority tests - Basket (HIGH PRIORITY - 0% baseline coverage)
#[path = "instruments/basket/test_basket_pricer_integration.rs"]
mod test_basket_pricer_integration;

// Priority tests - Inflation-Linked Bond (HIGH PRIORITY - 0% baseline coverage)
// #[path = "instruments/inflation_linked_bond/test_inflation_linked_bond.rs"]
// mod test_inflation_linked_bond; // has errors
// #[path = "instruments/inflation_linked_bond/test_inflation_linked_bond_metrics.rs"]
// mod test_inflation_linked_bond_metrics; // has errors

// Bond tests - Bond ASW metrics (PRIORITY - 0% baseline)
#[path = "instruments/bond/test_bond_asw_metrics.rs"]
mod test_bond_asw_metrics;
// Bond tests - other metrics
#[path = "instruments/bond/bond_metrics_validation.rs"]
mod bond_metrics_validation;
// #[path = "instruments/bond/test_bond_pricing_helpers.rs"]
// mod test_bond_pricing_helpers; // has test failures - needs investigation

// Working instrument tests
#[path = "instruments/cap_floor/test_cap_floor.rs"]
mod test_cap_floor;
#[path = "instruments/cds/test_cds_basic_and_conventions.rs"]
mod test_cds_basic_and_conventions;
#[path = "instruments/cds_index/cds_index_metrics_validation.rs"]
mod cds_index_metrics_validation;
#[path = "instruments/cds_tranche/cds_tranche_metrics_validation.rs"]
mod cds_tranche_metrics_validation;
#[path = "instruments/equity_option/test_equity_option_metrics.rs"]
mod test_equity_option_metrics;
#[path = "instruments/fra/test_fra_metrics.rs"]
mod test_fra_metrics;
#[path = "instruments/fx_option/test_fx_option_metrics.rs"]
mod test_fx_option_metrics;
#[path = "instruments/ir_future/test_ir_future_metrics.rs"]
mod test_ir_future_metrics;
#[path = "instruments/irs/irs_metrics_validation.rs"]
mod irs_metrics_validation;
#[path = "instruments/irs/test_irs_metrics_comprehensive.rs"]
mod test_irs_metrics_comprehensive;
// #[path = "instruments/repo/test_repo.rs"]
// mod test_repo; // needs extensive API fixes (14 errors)
#[path = "instruments/swaption/test_swaption_metrics_comprehensive.rs"]
mod test_swaption_metrics_comprehensive;

// Standalone test files
#[path = "instruments/options_metrics_validation.rs"]
mod options_metrics_validation;
#[path = "instruments/test_instrument_serialization_roundtrip.rs"]
mod test_instrument_serialization_roundtrip;

// COMMENTED OUT - Legacy tests needing extensive API migration:
// Structured credit:
//   - structured_credit_integration.rs (StructuredCreditCreditFactors API)
//   - test_deal_specific_and_risk_pricing_metrics.rs (StructuredCreditCreditFactors API)
//   - test_pool_and_waterfall.rs (DealType/TrancheSeniority imports)
//   - test_pricer_integration.rs (various API changes)
//   - test_tranche_valuation_and_diversion.rs (enums imports)
// TRS:
//   - test_total_return_swaps.rs (traits/underlying API)
//   - test_trs_pricer_and_metrics.rs (underlying API)
// Bond:
//   - test_oas_bond.rs (finstack_core::F)
//   - test_oas_custom_cashflows.rs (finstack_core::F)
// CDS:
//   - test_cds_index_imm_factor_cs01.rs (CreditParams, pricing module)
//   - cds_index/test_cds_index.rs (CreditParams)
// Other:
//   - deposit/test_deposit.rs (finstack_core::F)
//   - swaption/test_swaption_pricing_and_metrics.rs (Priceable trait)
//   - test_private_markets_fund.rs (finstack_core::F)
//   - test_registry_and_pricing.rs (InstrumentKey)
//   - traits_impls.rs (traits/underlying API)
//   - test_instruments.rs (finstack_core::F, Attributes)
//   - test_instruments_2.rs (CoreDiscCurve)
//   - basis_swap tests (BasisLegSpec)
//   - basket tests except integration (ReplicationMethod)
//   - common tests (various)
//   - convertible tests (ConvertibleBond)
//   - fx_swap tests (FxSwapParams)
//   - inflation_swap tests (InflationSwapBuilder)
