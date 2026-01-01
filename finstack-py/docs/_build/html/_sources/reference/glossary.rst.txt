Glossary
========

Financial Terms
---------------

.. glossary::

   Accrued Interest
      Interest that has accumulated on a bond since the last coupon payment date.

   Basis Point (bp)
      One hundredth of one percent (0.01%). Used for quoting interest rate changes.

   Business Day Convention (BDC)
      Rule for adjusting dates that fall on weekends or holidays (e.g., ModifiedFollowing).

   Cashflow
      Payment of money at a specific date (coupon, principal, fee, etc.).

   Clean Price
      Bond price excluding accrued interest. Quoted in financial markets.

   Convexity
      Second-order sensitivity of bond price to interest rate changes.

   Credit Default Swap (CDS)
      Derivative contract providing insurance against credit default.

   CS01
      Credit spread sensitivity: change in value per 1bp shift in credit spread.

   Day Count Convention
      Method for calculating year fraction between two dates (e.g., Act/360, 30/360).

   Dirty Price
      Bond price including accrued interest. Actual settlement price.

   Discount Curve
      Term structure of discount factors for present value calculations.

   Discount Factor
      Present value of $1 received at a future date.

   Duration
      Weighted average time to receive cashflows. Macaulay or Modified.

   DV01
      Dollar value of 1bp: change in value per 1bp shift in discount curve.

   Forward Curve
      Term structure of forward rates (e.g., LIBOR, SOFR).

   FX (Foreign Exchange)
      Currency exchange rate (e.g., USD/EUR = 0.92).

   Greeks
      Sensitivities of option value: Delta, Gamma, Vega, Theta, Rho.

   Hazard Curve
      Credit curve representing default intensity (survival probability).

   Inflation Curve
      Term structure of inflation rates or CPI levels.

   Interpolation
      Method for estimating curve values between known points (linear, log-linear, cubic).

   ISDA
      International Swaps and Derivatives Association. Defines standard conventions.

   Notional
      Face value or principal amount of an instrument.

   Par Rate
      Coupon rate that makes a bond trade at par (100).

   Present Value (PV)
      Current value of future cashflows discounted to today.

   Spread
      Yield difference or premium over a benchmark (e.g., credit spread over Treasury).

   Tenor
      Time period (e.g., 3M, 1Y, 5Y).

   Volatility Surface
      Term structure and strike structure of implied volatilities.

   Yield
      Internal rate of return of a bond's cashflows.

   Yield to Maturity (YTM)
      Discount rate that equates bond price to present value of cashflows.

   Zero Rate
      Yield on a zero-coupon bond.

Technical Terms
---------------

.. glossary::

   Analytical Pricing
      Closed-form mathematical formula for pricing (e.g., Black-Scholes).

   Bootstrapping
      Iterative curve construction from market quotes.

   Calibration
      Fitting model parameters to match market prices.

   Determinism
      Property of producing identical results for identical inputs.

   GIL (Global Interpreter Lock)
      Python threading limitation. finstack releases GIL for parallelism.

   Monte Carlo
      Simulation-based pricing using random scenarios.

   Netting Set
      Group of positions for which exposure is netted (reduced by offsetting).

   Parity Test
      Test verifying Python and Rust produce identical results.

   Polars
      Fast DataFrame library for Python (Rust-backed).

   PyO3
      Rust library for building Python bindings.

   Rust
      Systems programming language. finstack's core is written in Rust.

   Scenario
      Hypothetical market state for stress testing.

   Serde
      Rust serialization/deserialization framework.

   WASM (WebAssembly)
      Binary instruction format for browsers. finstack has WASM bindings.

finstack-Specific Terms
------------------------

.. glossary::

   Currency Safety
      finstack's enforcement of no implicit cross-currency arithmetic.

   FX Policy
      Strategy for converting between currencies (e.g., triangulation, direct).

   Model Key
      Enum identifying a pricing model (e.g., BarrierBSContinuous).

   Precedence Rule
      Statement evaluation order: Value > Forecast > Formula.

   Pricer Registry
      Type-safe mapping from (InstrumentType, ModelKey) to pricers.

   Results Metadata
      Information stamped on results: rounding context, FX policy, parallel flag.

   Rounding Context
      Active rounding mode and decimal places for an operation.

   Wire Format
      JSON serialization schema. Stable across versions.

Abbreviations
-------------

.. glossary::

   bp
      Basis point (0.01%).

   BDC
      Business Day Convention.

   CDS
      Credit Default Swap.

   CLO
      Collateralized Loan Obligation.

   CPI
      Consumer Price Index (inflation measure).

   CSA
      Credit Support Annex (margin agreement).

   CVA
      Credit Valuation Adjustment.

   DVA
      Debit Valuation Adjustment.

   FRA
      Forward Rate Agreement.

   FVA
      Funding Valuation Adjustment.

   IM
      Initial Margin.

   IMM
      International Monetary Market (IMM dates: 3rd Wednesday).

   ISDA
      International Swaps and Derivatives Association.

   MBS
      Mortgage-Backed Security.

   MC
      Monte Carlo.

   NDF
      Non-Deliverable Forward.

   NPV
      Net Present Value.

   OIS
      Overnight Index Swap.

   PV
      Present Value.

   SIMM
      Standard Initial Margin Model.

   TRS
      Total Return Swap.

   VM
      Variation Margin.

   xVA
      Collective term for valuation adjustments (CVA, DVA, FVA, etc.).

   YTM
      Yield to Maturity.

See Also
--------

* :doc:`../tutorials/core_concepts`
* :doc:`../api/index`
