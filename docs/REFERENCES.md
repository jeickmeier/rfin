# Finstack Documentation References

This file provides stable anchors for canonical references used across the
`finstack` crates. Public Rust, Python, and WASM documentation should link here
when an API implements a market convention, pricing model, numerical method, or
risk calculation with a standard reference.

## Usage

- Prefer links of the form `docs/REFERENCES.md#anchor-name` in rustdoc
  `# References` sections.
- Use the most specific anchor that matches the algorithm or convention.
- If a public API relies on market practice rather than a single paper, cite the
  closest industry standard first, then a practitioner text if needed.

## Day Count And Business-Day Conventions

<a id="isda-2006-definitions"></a>

### ISDA 2006 Definitions

- International Swaps and Derivatives Association. *2006 ISDA Definitions*.
  Sections covering day-count fractions, business-day conventions, and schedule
  adjustments.

<a id="icma-rule-book"></a>

### ICMA Rule Book

- International Capital Market Association. *ICMA Rule Book*. Bond-market
  conventions for accrued interest and irregular coupon handling, including
  Actual/Actual (ICMA/ISMA) style calculations.

<a id="iso-8601"></a>

### ISO 8601

- International Organization for Standardization. *ISO 8601 Date and Time
  Format*. Canonical reference for calendar, week-date, and period notation.

## Curves, Discounting, And Interest Rates

<a id="hull-options-futures"></a>

### Hull Options Futures

- Hull, J. C. *Options, Futures, and Other Derivatives*. Standard reference for
  discounting, forwards, swaps, and foundational derivatives pricing.

<a id="andersen-piterbarg-interest-rate-modeling"></a>

### Andersen Piterbarg Interest Rate Modeling

- Andersen, L. B. G., and Piterbarg, V. V. *Interest Rate Modeling*. Multi-curve
  discounting, term-structure construction, and interest-rate modeling
  conventions.

<a id="hagan-west-monotone-convex"></a>

### Hagan West Monotone Convex

- Hagan, P. S., and West, G. "Interpolation Methods for Curve Construction."
  Canonical reference for monotone-convex interpolation used in yield-curve
  construction.

<a id="tuckman-serrat-fixed-income"></a>

### Tuckman Serrat Fixed Income

- Tuckman, B., and Serrat, A. *Fixed Income Securities*. Standard text for
  key-rate risk, DV01, and fixed-income hedging intuition.

## Credit, Correlation, And Portfolio Risk

<a id="isda-cds-standard-model"></a>

### ISDA CDS Standard Model

- ISDA CDS Standard Model documentation and related ISDA credit-derivatives
  conventions. Use for hazard-rate, survival-probability, and CDS-style
  accrual/settlement references.

<a id="mcneil-frey-embrechts-qrm"></a>

### McNeil Frey Embrechts QRM

- McNeil, A. J., Frey, R., and Embrechts, P. *Quantitative Risk Management*.
  Canonical reference for VaR, Expected Shortfall, and portfolio risk
  interpretation.

<a id="meucci-risk-and-asset-allocation"></a>

### Meucci Risk And Asset Allocation

- Meucci, A. *Risk and Asset Allocation*. Reference for factor models, covariance
  aggregation, and exposure-based portfolio risk decomposition.

## Volatility, Options, And Smile Models

<a id="black-1976"></a>

### Black 1976

- Black, F. "The Pricing of Commodity Contracts." The standard reference for the
  Black (1976) forward-style option pricing model.

<a id="bachelier-1900"></a>

### Bachelier 1900

- Bachelier, L. *The Theory of Speculation*. Canonical reference for normal-model
  option pricing.

<a id="gatheral-volatility-surface"></a>

### Gatheral Volatility Surface

- Gatheral, J. *The Volatility Surface*. Canonical reference for implied-volatility
  parameterizations, total variance, and smile dynamics.

<a id="gatheral-2004-svi"></a>

### Gatheral 2004 SVI

- Gatheral, J. "A Parsimonious Arbitrage-Free Implied Volatility
  Parameterization." Standard SVI slice reference.

<a id="gatheral-jacquier-2014-svi"></a>

### Gatheral Jacquier 2014 SVI

- Gatheral, J., and Jacquier, A. "Arbitrage-Free SVI Volatility Surfaces."
  Follow-on reference for SVI no-arbitrage conditions.

<a id="hagan-2002-sabr"></a>

### Hagan 2002 SABR

- Hagan, P. S., Kumar, D., Lesniewski, A., and Woodward, D. "Managing Smile
  Risk." Canonical SABR reference.

<a id="heston-1993"></a>

### Heston 1993

- Heston, S. L. "A Closed-Form Solution for Options with Stochastic Volatility."
  Canonical Heston-model reference.

<a id="clark-fx-options"></a>

### Clark FX Options

- Clark, I. *Foreign Exchange Option Pricing*. Reference for FX volatility
  conventions and smile construction.

<a id="wystup-fx-options"></a>

### Wystup FX Options

- Wystup, U. *FX Options and Structured Products*. Reference for delta-based FX
  volatility quoting and smile construction.

## Numerical Methods, Statistics, And Randomness

<a id="higham-accuracy-and-stability"></a>

### Higham Accuracy And Stability

- Higham, N. J. *Accuracy and Stability of Numerical Algorithms*. Canonical
  reference for floating-point error analysis and numerically stable algorithms.

<a id="press-numerical-recipes"></a>

### Press Numerical Recipes

- Press, W. H. et al. *Numerical Recipes*. Practical reference for root finding,
  integration, interpolation, and Monte Carlo techniques.

<a id="welford-1962"></a>

### Welford 1962

- Welford, B. P. "Note on a Method for Calculating Corrected Sums of Squares and
  Products." Canonical one-pass variance reference.

<a id="kahan-1965"></a>

### Kahan 1965

- Kahan, W. "Further Remarks on Reducing Truncation Errors." Canonical reference
  for compensated summation.
