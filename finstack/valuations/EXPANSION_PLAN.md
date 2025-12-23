# Finstack Valuation Library Expansion Plan

This document outlines the roadmap for adding new instruments, pricing models, and risk metrics to the `finstack-valuations` crate. The focus is on supporting **Asset Managers**, **Pension Funds (LDI)**, and **Hedge Funds**.

---

## 1. New Instruments

### 1.1 Inflation Cap/Floor (Rates)
**Target Audience:** Pension Funds, Insurance (LDI Strategies)  
**Location:** `valuations/src/instruments/rates/inflation_cap_floor/`

Inflation options are used to hedge liability inflation linkage (e.g., COLA adjustments).

*   **Structure:**
    *   `InflationCapFloor` struct.
    *   **Underlying:** Inflation Index (e.g., CPI-U, UK RPI, Euro HICP).
    *   **Payoff:** `max(Index_t / Index_{t-1} - 1 - Strike, 0)` for YoY caps.
    *   **Parameters:** Observation Lag (e.g., 3M), Interpolation (Linear/Flat).
*   **Pricing Model:**
    *   **Black-76 / Bachelier** on the forward inflation index.
    *   Requires `InflationVolatilitySurface` (Strike vs Term).

### 1.2 FX Variance Swap (FX)
**Target Audience:** Hedge Funds, Macro Funds  
**Location:** `valuations/src/instruments/fx/fx_variance_swap/`

Pure volatility trading product allowing exposure to FX vol without delta hedging.

*   **Structure:**
    *   `FxVarianceSwap` struct.
    *   **Payoff:** `VarianceNotional * (RealizedVol^2 - StrikeVol^2)`.
    *   **Drift Handling:** Must correctly account for domestic vs. foreign interest rate differential in the forward expectation.
*   **Pricing Model:**
    *   **Replication Portfolio:** Integrating over the strip of OTM FX Options (Calls and Puts).
    *   Requires `FxVolatilitySurface` (Smile/Smirk).

### 1.3 Commodity Option (Commodities)
**Target Audience:** CTA / Macro Funds, Corporate Hedgers  
**Location:** `valuations/src/instruments/commodity/commodity_option/`

*   **Structure:**
    *   `CommodityOption` struct.
    *   **Underlying:** Commodity Future (or Spot).
    *   **Style:** European (vanilla) and American (early exercise for futures).
*   **Pricing Model:**
    *   **Black-76** (European).
    *   **Barone-Adesi Whaley** or **Binomial Tree** (American).

### 1.4 Real Estate Asset (Private Markets)
**Target Audience:** Pension Funds, Endowments  
**Location:** `valuations/src/instruments/real_estate/`

Physical asset valuation for illiquid portfolio components.

*   **Structure:**
    *   `RealEstateAsset` struct.
    *   **Inputs:** `NetOperatingIncome` (NOI) schedule, `CapRate` (Capitalization Rate), `AppraisalValue`.
*   **Pricing Model:**
    *   **Discounted Cash Flow (DCF):** Discounting projected NOI and terminal value.
    *   **Direct Capitalization:** `NOI / CapRate`.

---

## 2. New Risk Metrics

### 2.1 Bond Convexity
**Type:** Interest Rate Risk (Second Order)  
**Location:** `valuations/src/instruments/fixed_income/bond/metrics/risk/convexity.rs`

Crucial for LDI portfolios with long durations where `Duration` linear approximation fails.

*   **Formula:** `(1 / Price) * (d²Price / dYield²)`
*   **Implementation:** Closed-form derivative of the bond pricing formula (reusing the cashflow engine).

### 2.2 G-Spread (Government Spread)
**Type:** Relative Value / Credit Risk  
**Location:** `valuations/src/instruments/fixed_income/bond/metrics/price_yield_spread/g_spread.rs`

Spread over the interpolated government bond curve (e.g., UST, Bunds) matching the bond's maturity.

*   **Requirements:**
    *   Input: `BenchmarkCurveId` in `MetricContext` (identifying the Gov curve).
    *   Logic: Interpolate Gov Yield at `T_maturity`, calculate `BondYield - GovYield`.

### 2.3 Option Greeks (First-Class Support)
**Type:** Sensitivity  
**Location:** `valuations/src/metrics/sensitivities/`

Formalize the generic FD calculators into registered metrics for easy access.

*   **Gamma:** `d²V/dS²` (already implemented in generic FD, needs registration).
*   **Vanna:** `d²V/dS dVol` (Spot-Vol cross sensitivity).
*   **Volga:** `d²V/dVol²` (Vol convexity).

### 2.4 Expected Exposure (EE/PFE)
**Type:** Counterparty Credit Risk (CCR)  
**Location:** `valuations/src/metrics/risk/exposure.rs`

Profile of exposure over time for CVA (Credit Valuation Adjustment).

*   **Implementation:**
    *   Requires **Monte Carlo** simulation of market factors.
    *   Returns a time-series vector: `[EE(t_0), EE(t_1), ... EE(t_n)]`.
    *   `EE(t) = Average( max(NPV(t), 0) )` across paths.

---

## 3. Implementation Roadmap

### Phase 1: Core Rates & LDI (High Priority)
1.  Implement **Bond Convexity** metric (low effort, high value).
2.  Implement **Inflation Cap/Floor** instrument and pricing logic.
3.  Implement **G-Spread** metric.

### Phase 2: Volatility & Hedge Fund Tools
1.  Implement **FX Variance Swap**.
2.  Register **Gamma** and **Vanna** metrics in the central registry.
3.  Implement **Commodity Option**.

### Phase 3: Private Markets
1.  Implement **Real Estate** instrument structure.

### Phase 4: Advanced Risk
2.  Prototype **Expected Exposure (EE)** metric framework (requires MC engine integration).