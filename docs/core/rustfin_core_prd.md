# **Product Requirements Document — rustfin-core v1.0**

| Doc ID | RF-CORE-PRD-1.0-FINAL |
| :---- | :---- |
| **Status** | **Approved for Implementation** |
| **Date** | **29 June 2025** |
| **Authors** | Core Architecture Group |
| **Stakeholders** | Quant Eng, Trading & Risk, Structured-Credit Desk, Treasury, IT Ops, Dev Tooling |

**Scope Note:** This document consolidates all preceding design documents and PRDs for rustfin-core. It represents the single source of truth for the v1.0 release, superseding all previous versions.

## **1\. Executive Summary**

rustfin-core is the foundational analytics engine for the RustFin suite. It provides a comprehensive, high-performance, and memory-safe toolkit for financial computation. The core library supplies all common abstractions required for pricing and risk management, including: robust date and calendar logic, detailed cash flow representations, advanced curve and volatility surface modeling, a wide range of financial instruments, and a sophisticated risk metrics engine.

Designed to be composable and portable, rustfin-core ensures analytical consistency and performance across all applications, with stable bindings for Python and WebAssembly to support both research and production workflows.

## **2\. Goals & Success Metrics**

| \# | Goal | Acceptance Metric |
| :---- | :---- | :---- |
| **G1** | **Complete & Correct Foundations** | All public APIs for dates, calendars, cash flows, and curves are implemented as specified in the TDD. Golden-vector tests against QuantLib and other benchmarks pass within a 1e-9 tolerance. |
| **G2** | **High-Performance Analytics** | Valuation and risk calculations meet or exceed performance targets: \- **PV:** 100k vanilla swaps in \< 500 ms (16 cores). \- **Bootstrap:** 100-quote multi-curve solve in \< 3 ms. \- **Risk:** Full first-order risk vector on a 50k trade portfolio in \< 400 ms. |
| **G3** | **Comprehensive Instrument Coverage** | All instruments specified in section 4.5 are implemented and can be priced using the Priced trait. This includes money market instruments, swaps, bonds, caps/floors, and swaptions. |
| **G4** | **Advanced Risk Engine** | A unified RiskEngine provides first- and second-order sensitivities (Delta, Gamma, Vega, DV01) via Analytic, Adjoint (AAD), and Finite-Difference methods. AAD Gamma calculation time is ≤ 1.3x the PV calculation time. |
| **G5** | **Robust Serialization & Portability** | All public types implement serde::Serialize and serde::Deserialize with stable, versioned schemas. Identical binary (CBOR) representations are produced across Rust, Python, and WASM environments, verified by CI snapshot tests. |
| **G6** | **Uncompromising Safety & Stability** | The public API is 100% unsafe-free, verified by cargo geiger. The library achieves v1.0 stability with a clear semantic versioning policy for any future breaking changes. |

## **3\. Scope**

| Included (v1.0) | Deferred (v1.1+) |
| :---- | :---- |
| All capabilities listed in Section 4\. | Exotic path-dependent instruments (e.g., Bermudans, Asian options). |
| Foundational primitives, dates, and calendars. | Full Monte-Carlo simulation engine. |
| Cash flow generation and NPV. | Cross-gamma risk calculations (e.g., dVega/dRate). |
| Advanced curve & surface modeling and calibration. | XVA (CVA, DVA, FVA) and margin simulation. |
| A comprehensive set of vanilla and semi-exotic instruments. | GUI dashboards and end-user applications. |
| First- and second-order risk metrics via Analytic/AAD/FD. | Direct market data feed integration. |
| Private-credit bond extensions (PIK, step-ups). |  |
| Python and WASM bindings for all core functionality. |  |

## **4\. Functional Requirements**

### **4.1. Primitives (C-PRIM)**

| ID | Capability |
| :---- | :---- |
| **C-PRIM-01** | Provide a Currency enum based on ISO-4217 numeric codes with robust string parsing. |
| **C-PRIM-02** | Provide a generic Money\<F\> struct for currency-aware arithmetic, supporting f64 and rust\_decimal::Decimal. |
| **C-PRIM-03** | Establish a unified, crate-wide Error enum with Input, Calendar, and Internal variants. |
| **C-PRIM-04** | Define and export foundational enums: DayCount, Frequency, and BusDayConv. |
| **C-PRIM-05** | Define a PeriodKey struct for efficient caching of date-based calculations. |

### **4.2. Dates & Calendars (C-DATE, C-CAL)**

| ID | Capability |
| :---- | :---- |
| **C-DATE-01** | Provide a memory-efficient Date struct with robust arithmetic (days, weeks, months, years). |
| **C-DATE-02** | Implement a Schedule builder for generating cash flow dates with support for frequency, stubs, and business day conventions. |
| **C-DATE-03** | Provide year-fraction calculation helpers for all standard day-count conventions. |
| **C-CAL-01** | Provide a HolidayCalendar trait and a HolidaySet implementation for defining financial calendars. |
| **C-CAL-02** | Support composite calendars (union/intersection of multiple calendars). |
| **C-CAL-03** | Include a build-time pipeline to parse iCalendar (\*.ics) files into an efficient, embedded binary format. |
| **C-CAL-04** | Provide built-in calendars for major financial centers (TARGET2, NYSE, LSE, CME, etc.). |
| **C-CAL-05** | Implement IMM date calculation helpers for futures and FRA contracts. |

### **4.3. Cash Flows (C-CF)**

| ID | Capability |
| :---- | :---- |
| **C-CF-01** | Define a CashFlow struct (date, amount, kind) and a CashFlowLeg container. |
| **C-CF-02** | Implement a Discountable trait with an npv method that uses a DiscountCurve. |
| **C-CF-03** | Support notional amortization schedules (linear, step) and stub periods for irregular coupons. |
| **C-CF-04** | Implement a cache for accrual factors to optimize large portfolio calculations. |
| **C-CF-05** | Provide builders for both fixed-rate and floating-rate legs, including support for spreads, gearing, and reset lags. |

### **4.4. Curves & Calibration (C-CURVE, C-CALIB)**

| ID | Capability |
| :---- | :---- |
| **C-CURVE-01** | Define core traits: Curve (for generic term structures) and DiscountCurve (for discounting). |
| **C-CURVE-02** | Implement a flexible numeric precision layer, aliasing F to f64 or Decimal via feature flag. |
| **C-CURVE-03** | Provide concrete curve implementations: YieldCurve, ForwardIndexCurve, HazardCurve, and InflationCurve. |
| **C-CURVE-04** | Provide VolSurface (2D) and SwaptionSurface3D for volatility modeling. |
| **C-CURVE-05** | Implement a CurveSet container for managing multi-curve contexts (e.g., separate discount and forecast curves). |
| **C-CURVE-06** | Support a selection of pluggable interpolation policies (e.g., Log-Linear, Monotone Convex). |
| **C-CALIB-01** | Define a Bootstrappable trait for calibrating curves from market quotes. |
| **C-CALIB-02** | Implement bootstrappers for yield curves, hazard curves, and inflation curves. |
| **C-CALIB-03** | Implement a coupled multi-curve solver for simultaneously calibrating discount and forecast curves. |
| **C-CALIB-04** | Provide SABR calibration helpers for volatility surfaces. |

### **4.5. Instruments (C-INSTR)**

| ID | Capability |
| :---- | :---- |
| **C-INSTR-01** | Define a universal Priced trait with a pv method as the standard valuation interface. |
| **C-INSTR-02** | Implement SpotAsset for cash and other spot-settling assets. |
| **C-INSTR-03** | Implement Money Market instruments: Deposit, FRA, and InterestRateFuture. |
| **C-INSTR-04** | Implement InterestRateSwap with support for fixed vs. floating and multi-curve discounting. |
| **C-INSTR-05** | Implement CapFloor and Swaption using Black's model and volatility surfaces. |
| **C-INSTR-06** | Implement Bond instruments: Fixed Rate, Floating Rate, and Callable. |
| **C-INSTR-07** | Implement private-credit bond extensions (e.g., step-up coupons, PIK) under a feature flag. |
| **C-INSTR-08** | Implement deferred stubs for future instrument support (FX, Credit, Equity derivatives). |
| **C-INSTR-09** | Provide fluent Builder patterns for ergonomic and safe instrument construction. |

### **4.6. Risk (C-RISK)**

| ID | Capability |
| :---- | :---- |
| **C-RISK-01** | Define a RiskEngine trait as the single entry point for all sensitivity calculations. |
| **C-RISK-02** | Implement multiple risk calculation modes: Analytic, Adjoint (AAD), and FiniteDifference. |
| **C-RISK-03** | Define a comprehensive RiskFactor taxonomy covering rates, FX, volatility, credit, and equity. |
| **C-RISK-04** | Produce a sparse RiskReport containing PV and key sensitivities (Delta, Gamma, Vega, Theta, DV01). |
| **C-RISK-05** | Implement helpers for generating bucketed risk vectors (key-rate DV01, Vega ladders). |
| **C-RISK-06** | Provide a reusable, thread-safe BumpCache to accelerate repeated scenario evaluations. |
| **C-RISK-07** | Implement a MarketSnapshot trait for applying what-if scenario shocks without rebuilding curves. |

## **5\. Non-Functional Requirements**

### **5.1. Performance**

* All performance targets listed in Success Metric G2 must be met in CI benchmarks.  
* Memory usage for bootstrapping a standard curve set shall not exceed 150 MB.  
* The library must demonstrate linear scaling of parallel computations up to at least 16 cores.

### **5.2. Safety & Security**

* The public API must be 100% free of unsafe code.  
* All dependencies will be audited for security vulnerabilities via cargo auditable.  
* The library must be free of panics in its public API; all recoverable errors must be returned as Result.

### **5.3. Serialization & Stability**

* All public types must derive Serialize and Deserialize with \#\[serde(version \= 1)\].  
* Serialization formats (JSON, CBOR) must be backwards-compatible for N-1 versions.  
* The public API will adhere strictly to semantic versioning after the v1.0.0 release.

### **5.4. Packaging & Portability**

* The default compiled Python wheel must be less than 6 MB (stripped).  
* The library must compile and pass all tests on stable Rust (MSRV 1.78) for Linux, macOS, and Windows.  
* WASM builds must be supported and tested.

## **6\. Milestones & Implementation Order**

| Phase | Duration | Dependencies | Key Deliverables (Capabilities) |
| :---- | :---- | :---- | :---- |
| **M0: Foundations** | 2 weeks | \- | C-PRIM complete. Basic Date struct (C-DATE-01). Error handling framework. |
| **M1: Dates & Calendars** | 3 weeks | M0 | Remainder of C-DATE and all C-CAL. Schedule generation & holiday engine. |
| **M2: Cash Flows & Curves** | 4 weeks | M1 | C-CF and C-CURVE complete. Leg builders, NPV helpers, curve types, CurveSet. |
| **M3: Calibration** | 3 weeks | M2 | All C-CALIB. Yield curve and multi-curve bootstrappers. |
| **M4: Core Instruments** | 3 weeks | M3 | C-INSTR-01 to C-INSTR-06. Money markets, swaps, caps/floors, swaptions, bonds. |
| **M5: Risk Engine** | 4 weeks | M4 | All C-RISK. Risk factor taxonomy, Analytic/AAD/FD engines, RiskReport. |
| **M6: Hardening & Docs** | 3 weeks | M5 | C-INSTR-07 (Private Credit). Finalize all docs, run fuzz tests, lock API for v1.0. |
| **GA (v1.0.0)** | \- | M6 | Tag and publish official v1.0.0 release to crates.io and other repositories. |
| **Total Duration** | **22 weeks** |  |  |

## **7\. Risks & Mitigation**

| Risk | Mitigation |
| :---- | :---- |
| **Bootstrapping Convergence** | The calibration engine will include robust fallback solvers (e.g., Brent's method) and emit detailed warnings on failure, rather than panicking. Monotonicity checks will be enforced. |
| **Performance Regression** | A comprehensive benchmark suite will run in CI on every pull request. Regressions greater than 5% will block merging. |
| **Calendar Data Discrepancies** | A golden set of calendar data will be maintained, and the CI pipeline will run a diff against it, alerting on any changes. The binary calendar format will be hashed to ensure reproducibility. |
| **Precision Drift (decimal128)** | The high-precision mode will be opt-in via feature flag. CI will run a separate test suite in decimal128 mode to ensure correctness and prevent precision-related bugs. |

