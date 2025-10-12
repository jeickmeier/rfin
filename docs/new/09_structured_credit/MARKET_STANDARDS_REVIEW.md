# Structured Credit Instrument: Market Standards Review

This document provides a review of the `structured_credit` instrument implementation against common market standards and best practices in financial modeling.

## 1. Overall Architecture

### :+1: Commendation: Unified Instrument Model

The single most significant architectural strength is the unification of the previously separate `ABS`, `CLO`, `CMBS`, and `RMBS` modules into a single `StructuredCredit` instrument.

-   **Reduced Duplication**: This refactoring successfully eliminated an estimated 1,400 lines of redundant code.
-   **Improved Maintainability**: Logic for cashflow generation, pricing, and risk calculation is now centralized, making it vastly easier to maintain and extend.
-   **Composability**: The use of `DealType` enums and composed structs (`AssetPool`, `TrancheStructure`, `WaterfallEngine`) provides a clean, flexible, and extensible design. Deal-specific variations are handled through configuration and composition rather than inheritance or code duplication.

This is a best-in-class design for representing related financial products and serves as a strong foundation for the library.

---

## 2. Adherence to Market Standards

The implementation correctly models many of the core concepts of structured finance.

### 2.1. Core Components

| Component           | Status                               | Notes                                                                                                                                                                                                                                                                              |
| ------------------- | ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Asset Pool**      | :white_check_mark: **Mostly Compliant**  | `PoolAsset` is well-structured. Calculations for WAC (Weighted Average Coupon) and Diversity Score are standard. The distinction between WAL (Weighted Average Life) and WAM (Weighted Average Maturity) is correctly identified.                                                        |
| **Tranches**        | :white_check_mark: **Compliant**         | The `Tranche` and `TrancheStructure` components correctly model the capital stack using attachment/detachment points and seniority. The waterfall loss allocation logic (`loss_allocation`) correctly impairs tranches from the bottom up.                                            |
| **Waterfall**       | :white_check_mark: **Compliant**         | The sequential "priority of payments" is standard (Fees → Interest → Principal → Equity). The implementation of coverage test triggers (`OC`/`IC`) that divert cashflow from junior to senior tranches upon a breach is a correct and critical feature for accurate modeling. |
| **Behavioral Models** | :white_check_mark: **Compliant**         | The use of standard models like **PSA** (Public Securities Association) for RMBS prepayments and **SDA** (Standard Default Assumption) for defaults is correct. The formulas for converting between annualized (CPR/CDR) and monthly (SMM/MDR) rates are accurate.            |
| **Risk Metrics**    | :white_check_mark: **Compliant**         | The numerical, market-standard methods for calculating key risk metrics are correctly implemented: <br> - **CS01** (Credit Spread DV01) via a 1bp spread bump. <br> - **Modified Duration** via a 1bp yield bump. <br> - **Z-Spread** via a root-finding solver.          |
| **CLO Metrics**     | :white_check_mark: **Compliant**         | The implementation of **WAS** (Weighted Average Spread) correctly prioritizes the "spread component only" for floating-rate assets, which is the market convention. The **WARF** (Weighted Average Rating Factor) calculation correctly uses the Moody's factor table.    |

### 2.2. Areas for Review & Improvement

While the core logic is sound, a few areas deviate from strict market standards or could be enhanced for greater accuracy.

#### 1. WAL Calculation Methodology

-   **Issue**: The generic `WalCalculator` in `metrics/pricing/wal.rs` treats all positive cashflows as principal.
-   **Market Standard**: Weighted Average Life (WAL) should be calculated using **only the principal components** of cashflows.
-   **Suggestion**: The cashflow generation engine already produces a `TrancheCashflowResult` which correctly separates `principal_flows` from `interest_flows`. The `WalCalculator` should be updated to consume this richer data structure (or the underlying `detailed_flows`) instead of a simple `DatedFlows` vector. This will allow it to compute a precise, market-standard WAL by weighting only the principal payments. The existing `calculate_tranche_wal` function in `tranche_valuation.rs` already does this correctly and can serve as a model.

#### 2. Hardcoded Behavioral Assumptions

-   **Issue**: Several places in the code use hardcoded rates for prepayment and default, particularly in `specs.rs` under the `AssetDefault` variants and in the `new_*` constructors in `types.rs`.
-   **Market Standard**: While defaults are useful, behavioral assumptions in a production system should be configurable and driven by data (e.g., market data services, user-defined scenario inputs).
-   **Suggestion**: Transition these hardcoded values into the `DealConfig` structure or a new `MarketAssumptions` struct. This would allow users to easily override default assumptions for a specific deal or scenario without changing the core library code. The `config.rs` file is a good place to centralize the *default* values, but the instrument itself should be configurable.

---

## 3. Code Quality & Refactoring

The codebase is generally high-quality. The recent refactoring to a unified `StructuredCredit` type is a major improvement.

### 3.1. Completed Refactoring

-   **Duplication in `instrument_trait.rs`**: The significant code duplication between `generate_tranche_cashflows` and `generate_specific_tranche_cashflows` has been successfully **resolved**. The logic is now centralized in a single `run_full_simulation` method, improving maintainability.

### 3.2. Further Opportunities

#### 1. Inconsistent Accrued Interest Calculation

-   **Issue**: There is a sophisticated `AccruedCalculator` in `metrics/pricing/accrued.rs` that attempts to reverse-engineer the accrual period from a list of cashflows. However, the primary entry point for tranche-level metrics (`value_tranche_with_metrics` in `types.rs`) currently hardcodes accrued interest to zero.
-   **Suggestion**: Simplify and unify this. The most robust way to calculate accrued interest is from first principles: `(As Of Date - Last Payment Date) / (Next Payment Date - Last Payment Date) * Next Interest Payment`. This requires tracking the payment schedule. Since the simulation already generates a schedule, this information could be passed to the metric calculation context. The current implementation in `AccruedCalculator` is a reasonable proxy but could be made more robust. The hardcoded zero in `value_tranche_with_metrics` should be replaced with a call to the proper calculation logic.

#### 2. Incomplete IC Test Implementation

-   **Issue**: The waterfall diversion logic in `waterfall.rs` contains a placeholder for the Interest Coverage (IC) test, stating `// For simplicity, skip IC for now`.
-   **Suggestion**: Implement the IC test check. The `coverage_tests.rs` module already contains the correct calculation logic for an IC test. This just needs to be wired into the `check_diversion_triggers_active` function in the waterfall. This would involve passing `interest_collections` into the `TestContext` to complete the implementation.

## 4. Summary & Recommendations

The `structured_credit` instrument is well-designed, modern, and largely compliant with market standards. The recent architectural unification is a major success.

**High-Priority Recommendations:**

1.  **Fix WAL Calculation**: Update the primary `WalCalculator` to use principal-only cashflows to align with the strict market definition.
2.  **Implement IC Test Trigger**: Complete the waterfall diversion logic by implementing the check for the Interest Coverage (IC) test.

**Medium-Priority Recommendations:**

3.  **Externalize Assumptions**: Move hardcoded behavioral assumptions (CPR, CDR, etc.) from `specs.rs` and `types.rs` into a configurable struct.
4.  **Unify Accrued Interest**: Replace the hardcoded zero for accrued interest in `value_tranche_with_metrics` with a consistent and robust calculation.

By addressing these points, the `structured_credit` module can further enhance its accuracy, flexibility, and alignment with institutional financial modeling practices.
