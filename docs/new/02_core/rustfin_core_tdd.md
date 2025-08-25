# **Technical Design Document — rustfin-core v1.0**

| Doc ID | RF-CORE-TDD-1.0-FINAL |
| :---- | :---- |
| **Status** | **Approved for Implementation** |
| **Date** | **29 June 2025** |
| **Authors** | Core Architecture Group |

**Scope Note:** This TDD translates the consolidated rustfin-core PRD into a concrete engineering blueprint. It is the single source of truth for all architectural decisions, data structures, and algorithms for the v1.0 release.

## **1\. High-Level Architecture**

rustfin-core is a pure library crate designed with a layered architecture to ensure separation of concerns, testability, and composability.

### **1.1. System Context Diagram**

```mermaid
graph TD
    subgraph RustFin Ecosystem
        direction LR
        A[External Apps e.g., Python/WASM] --> Binds
        subgraph rustfin-workspace
            direction TB
            Portfolio[rustfin-portfolio] --> Core
            Scenario[rustfin-scenario] --> Core
            Binds[rustfin-binds] --> Core
        end
    end
    A -- Consumes --> Binds
    subgraph rustfin-core
        direction TB
        L4[L4: Instruments] --> L3
        L3[L3: Engines (Risk/Calibration)] --> L2
        L2[L2: Domain Types (Cashflow/Curves)] --> L1
        L1[L1: Foundations (Dates/Calendar)] --> L0
        L0[L0: Primitives (Currency/Error)]
    end

    Core -- Is --> rustfin-core

    classDef lib fill:#e6f3ff,stroke:#367d91,stroke-width:2px;
    classDef app fill:#f5f5f5,stroke:#333;
    class rustfin-core,Portfolio,Scenario,Binds,A,Core lib
    class App app
```

### **1.2. Layer Responsibilities**

| Layer | Crates/Modules | Responsibility | Key Abstractions |
| :---- | :---- | :---- | :---- |
| **L0: Primitives** | primitives | Core, non-financial types, error handling, and shared enums. | Currency, Money\<F\>, Error |
| **L1: Foundations** | dates, calendar | Time and business day logic. | Date, Schedule, HolidayCalendar |
| **L2: Domain Types** | cashflow, curves | Core financial concepts and data structures. | CashFlowLeg, Curve, VolSurface |
| **L3: Engines** | calibration, risk | Complex analytical processing. | Bootstrappable, RiskEngine |
| **L4: Instruments** | instr | Concrete financial products and their valuation. | Priced, Instrument Builders |

## **2\. Cross-Cutting Concerns**

### **2.1. Numeric Precision**

All floating-point calculations will use a type alias `pub type F`. This provides a single point of control for precision throughout the library.

```rust
// In primitives/src/lib.rs
cfg_if::cfg_if! {
    if #[cfg(feature = "decimal128")] {
        pub type F = rust_decimal::Decimal;
    } else {
        pub type F = f64;
    }
}
```

* **Default:** f64 for maximum performance and compatibility.  
* **decimal128 feature:** rust\_decimal::Decimal for high-precision applications, at a performance cost.

### **2.2. Error Handling**

A single, unified Error enum, defined in the primitives module, will be used across the entire crate. This ensures consistent error handling and simplifies bubbling errors up to the caller.

```rust
// In primitives/error.rs
#[derive(thiserror::Error, Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    #[error("Input Validation Error: {0}")]
    Input(String),

    #[error("Calendar Data Error: {0}")]
    Calendar(String),

    #[error("Pricing or Convergence Error: {0}")]
    Pricing(String),

    #[error("Internal Logic Error: {0}")]
    Internal(String),
}
```

Module-specific errors will implement From\<MyError\> for Error to facilitate conversion. No public API will ever panic\!.

### **2.3. Serialization**

All public structs and enums will derive serde::{Serialize, Deserialize} behind the serde feature flag. To ensure long-term stability and backwards compatibility:

* Structs and enums will use \#\[serde(tag \= "type", content \= "data")\] for enums and versioning attributes for structs where appropriate.  
* New optional fields will use \#\[serde(default)\].  
* CI will include snapshot tests (cargo-insta) to prevent unintentional changes to serialized formats.

### **2.4. Concurrency & Parallelism**

* All shareable data structures (e.g., HolidaySet, YieldCurve, CashFlowLeg) will be Send \+ Sync.  
* Large, immutable data will be wrapped in Arc\<T\> to allow for cheap, thread-safe cloning.  
* Computationally intensive loops (e.g., risk scenarios, portfolio valuation) will be parallelized using the rayon crate, enabled by the parallel feature flag.

### **2.5. Centralized Configuration (RuntimeConfig)**

To allow runtime control over model parameters, a centralized, hierarchical configuration system will be implemented. This allows users to change calculation settings without recompiling the code.

#### **2.5.1. Design Principles**

* **Runtime-Modifiable:** The configuration is loaded at runtime and passed into all major functions.  
* **Serializable:** The config struct derives serde and can be loaded from human-readable files (e.g., TOML, JSON).  
* **Default-Driven:** Users only need to specify parameters they wish to override. The system provides sensible defaults for all settings.  
* **Thread-Safe & Sharable:** The configuration object is immutable and wrapped in an Arc for efficient sharing across threads.

#### **2.5.2. Data Structure**

The configuration will be organized into a hierarchy of structs, mirroring the library's modules.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub curves: CurveConfig,
    #[serde(default)]
    pub calibration: CalibrationConfig,
    #[serde(default)]
    pub risk: RiskConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            curves: CurveConfig::default(),
            calibration: CalibrationConfig::default(),
            risk: RiskConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveConfig {
    pub default_interpolation_policy: InterpPolicy,
    pub allow_extrapolation: bool,
}

impl Default for CurveConfig {
    fn default() -> Self {
        Self {
            default_interpolation_policy: InterpPolicy::LogDf, // Sensible default
            allow_extrapolation: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationConfig {
    pub solver_tolerance: F,
    pub max_iterations: u32,
    pub default_solver: SolverType, // e.g., enum { Newton, Brent }
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            solver_tolerance: 1e-12 as F,
            max_iterations: 100,
            default_solver: SolverType::Newton,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub default_fd_bump_size: F,
    pub default_fd_bump_type: BumpType, // e.g., enum { Additive, Relative }
    pub default_fd_stencil: FDStencil,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            default_fd_bump_size: 1e-4 as F,
            default_fd_bump_type: BumpType::Additive,
            default_fd_stencil: FDStencil::TwoSided,
        }
    }
}
```

*Note (updated 2025-07-12): `InterpPolicy` has been removed — use the
`Interpolator` enum's smart constructors instead.*

### **2.6. The ValuationContext Object**

`ValuationContext` lives **for now** in the `primitives` crate (module `primitives::context`) so that every other crate can depend on it without cyclic links.  A future refactor may move it to a dedicated shared-runtime crate, but until then all design and PR references point to the primitives location.

To avoid passing numerous arguments (val_date, market_data, config) to every function, these will be consolidated into a single ValuationContext object. This simplifies API signatures and provides a single, consistent source of contextual data for all calculations.

#### **2.6.1. Data Structure**

```rust
use std::sync::Arc;

pub struct ValuationContext {
    pub val_date: Date,
    pub market_data: Arc<dyn MarketDataProvider + Send + Sync>,
    pub config: Arc<RuntimeConfig>,
}

// The MarketDataProvider trait will provide access to curves, surfaces, etc.
pub trait MarketDataProvider {
    fn get_discount_curve(&self, id: &CurveId) -> Result<Arc<dyn DiscountCurve + Send + Sync>, Error>;
    // ... other getters for forecast curves, vol surfaces etc.
}

// The context itself can implement the CurveProvider trait for convenience.
impl CurveProvider for ValuationContext {
    fn discount(&self, id: &str) -> Result<&dyn DiscountCurve, Error> {
        // delegates to self.market_data
    }
    // ...
}
```

#### **2.6.2. API Integration**

All major entry-point functions will now accept a reference to a ValuationContext.

* **Old Signature:** fn pv\<C: CurveProvider\>(\&self, curves: \&C, val\_date: Date) \-\> Result\<...\>  
* **New Signature:** fn pv(\&self, context: \&ValuationContext) \-\> Result\<...\>

This pattern provides a cleaner, more extensible, and more powerful interface for the entire library.

### **2.7. Feature Flag Matrix**

| Flag | Purpose | Mutually Exclusive With | Notes |
| :---- | :---- | :---- | :---- |
| serde | Enables serialization/deserialization on all public types. |  | Also enables serialization for RuntimeConfig. |
| decimal128 | Switches the core numeric type F from f64 to Decimal. |  |  |
| parallel | Enables Rayon-based parallel computation. |  | Default for server-side builds. |
| index | Enables floating-rate leg builder (requires forward index curves). |  | Added for cashflow & curves crates. |
| python | Enables build configuration for Python bindings (PyO3). |  |  |
| wasm | Enables build configuration for WebAssembly targets. |  | May require wee\_alloc. |

## **3\. Module-Level Design (Updated Signatures)**

### **3.1. primitives**

* **Responsibility:** Foundational, non-domain-specific types.  
* **Core Structs/Enums:**  
  * Currency: \#\[repr(u16)\] enum based on ISO-4217 numeric codes.  
  * Money\<F\>: Struct containing amount: F and ccy: Currency. Implements checked arithmetic.  
  * Error: See Section 2.2.  
  * DayCount, Frequency, BusDayConv: \#\[repr(u8)\] enums for compact storage.

### **3.2. dates & calendar**

* **Responsibility:** All logic related to time and business conventions.  
* **Data Structures:**  
  * Date: A struct wrapping days\_since\_epoch: i32 for efficiency and safety. All constructors are checked.  
  * HolidaySet: Contains holidays: &'static \[i32\] (a sorted slice of epoch days) and a WeekendRule. The holiday data is loaded from a binary blob at compile time via include\_bytes\!. The ical parser is a standard dependency.  
  * CompositeCalendar\<'a\>: Holds a SmallVec\<\[&'a HolidaySet; 4\]\> to merge calendars on the stack without allocation for common cases.  
* **Algorithms:**  
  * **Date Arithmetic:** Implemented via integer math on the epoch day count.  
  * **Business Day Adjustment:** An iterative loop (max 7 iterations) that calls is\_business\_day.  
  * **is\_business\_day:** \!is\_weekend(date) && holidays.binary\_search(\&date\_epoch\_day).is\_err().

### **3.3. cashflow & curves**

* **Responsibility:** Representing payments and the term structures used to value them.  
* **Data Structures:**  
  * CashFlowLeg: Wraps an Arc\<\[CashFlow\]\> to make cloning cheap. Stores leg-level metadata like day count and notional.  
  * YieldCurve: Stores knots and values in contiguous Box\<\[F\]\> slices for cache-friendly access. An interp: Box\<dyn Interpolator\> field holds the interpolation strategy.  
  * VolSurface, SwaptionSurface3D: Uses ndarray::Array2\<F\> and ndarray::Array3\<F\> for grid data, combined with interpolators for each axis.  
  * CurveSet: A struct of HashMap\<CurveId, Arc\<dyn ...\>\> to store and retrieve different curve types (discount, forecast, hazard, etc.).  
* **Algorithms:**  
  * **NPV:** A simple loop over cash flows, calling curve.df(date). Parallelized with Rayon for slices of legs.  
  * **Interpolation:** The Interpolator trait object dispatches to a specific implementation (e.g., Log-Linear DF, Monotone Convex). Binary search is used to find the relevant knot segment.

### **3.4. calibration**

* **Responsibility:** Calibrating curve parameters to match market quotes.  
* **Core Trait:**  
  * Bootstrappable: fn calibrate(\&mut self, context: \&ValuationContext) \-\> Result\<(), Error\>;  
* **Algorithms:**  
  * **Single Curve Bootstrap:** A sequential process. For each instrument (sorted by maturity), a root-finding solver (configured via context.config) finds the discount factor at the new knot that makes the instrument's PV equal to its price.  
  * **Multi-Curve Solver:** An iterative projection algorithm, configured and driven by the ValuationContext.

### **3.5. instr**

* **Responsibility:** Defining concrete financial instruments and their valuation logic.  
* **Core Trait:**  
  * Priced: fn pv(\&self, context: \&ValuationContext) \-\> Result\<Self::PVOutput, Error\>;  
* **Design Pattern:**  
  * Each instrument struct contains only its defining data. Private credit features like PIK and step-up coupons are standard fields on the Bond struct.  
  * The Priced implementation contains the valuation logic, using the provided ValuationContext to access market data and configuration.  
  * A fluent Builder is provided for each instrument.

### **3.6. risk**

* **Responsibility:** Calculating market risk sensitivities (Greeks).  
* **Core Trait & Structs:**  
  * RiskEngine: fn compute\<I: Priced\>(\&self, instr: \&I, context: \&ValuationContext) \-\> Result\<RiskReport, Error\>;  
  * RiskFactor: A comprehensive enum identifying what was shocked.  
  * RiskReport: A sparse struct containing the base PV and sensitivity vectors.  
* **Algorithms:**  
  * **Finite Difference (FD):** Bumps a RiskFactor within a copy of the ValuationContext, re-prices, and calculates sensitivity. Parameters like bump size are taken from context.config.risk.  
  * **Adjoint Algorithmic Differentiation (AAD):** The tape-based reverse mode, which also sources its configuration from the context.  
  * **Bump Cache:** A cache keyed by RiskFactor and config hash to store and reuse shocked ValuationContext components.

## **4\. Testing & CI Strategy**

1. **Unit Tests:** Each module will have comprehensive unit tests (\#\[cfg(test)\]).  
2. **Golden Vector Tests:** Key calculations will be tested against golden data sets, with separate tests for different RuntimeConfig files.  
3. **Property-Based Tests:** proptest will be used to test invariants under various configurations.  
4. **Fuzzing:** cargo-fuzz will be used on parsers and algorithms.  
5. **Benchmarks:** cargo-criterion will be used to benchmark critical performance paths.  
6. **CI Pipeline (GitHub Actions):** Will be updated to run tests with multiple configuration files to ensure parameter changes work as expected.