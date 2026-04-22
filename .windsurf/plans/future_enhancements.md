# asian_option

- High: Add discrete-monitoring bias correction for arithmetic averaging and better control-variate calibration.
- High: Support local-vol/stochastic-vol smile adjustments beyond flat/clamped surfaces.
- Moderate: Add early-exercise/Bermudan-style averaging support with tree/LSMC pricing.

# autocallable

- High: Support stochastic volatility/jumps and quanto effects for cross-ccy underlyings.
- Moderate: Add closed-form or semi-analytic approximations for speed when barriers are far OTM/ITM.
- Moderate: Enrich reporting with per-observation call probabilities and digital-Greek decompositions.

# barrier_option

- High: Add analytical discrete-monitoring corrections (Broadie–Glasserman) for tighter parity with exchange pricing.
- High: Support stochastic/local volatility smile adjustments and jump-diffusion tails.
- Moderate: Expand rebate handling to include delayed/continuous rebate payment timing.

# basis_swap

- High: Add funding/CSA basis adjustments and convexity corrections for long-dated tenors.
- High: Support stochastic basis modeling and curve-consistent bootstrapping aids.
- Moderate: Include spread-attribution and carry/roll analytics in the metrics set.

# basket

- High: Support scheduled rebalancing rules and turnover costs.
- High: Provide built-in stress reporting (single-name shocks, FX shocks) with cached constituent impacts.
- Moderate: Add drift/vol attribution and tracking-error style diagnostics.

# bond

- High: Add full callable/putable amortizing parity with more tree/PDE models and stochastic rates.
- High: Expand risk to include curve-shift scenarios (non-parallel) and callable bond Greeks.

# cap_floor

- High: Add Bachelier/normal and displaced-diffusion pricing paths for low-rate regimes.
- High: Support SABR/shifted-lognormal smile integration for more accurate vol skews.
- Moderate: Include gamma/volga analytics and callable-cap style optionality extensions.

# cds

- High: Add stochastic recovery and correlation hooks; richer accrual-on-default conventions (market fallbacks).

# cds_index

- High: Support stochastic spread simulation and correlation for scenario analytics.
- Moderate: Provide roll mechanics and curve-building helpers around series rolls.

# cds_option

- High: Incorporate stochastic recovery and smile surfaces beyond flat vol inputs.
- High: Provide callable/compound CDS option scaffolding and early-exercise approximations.
- Moderate: Add normal/Bachelier spread model and displaced-diffusion support for deep OTM/ITM quotes.

# cliquet_option

- High: Support stochastic volatility and jump diffusion for equity-linked structures.
- Moderate: Add semi-analytic approximations for additive cliquets to reduce MC runtime.
- Moderate: Provide gradient-based Greeks (pathwise/LR) for lower variance in MC mode.

# cms_option

- High: Add SABR/LMM-based convexity adjustments for long-tenor CMS instruments.
- High: Support Bermudan CMS caps/floors and callable CMS structures.
- Moderate: Introduce smile-consistent vol sourcing and interpolation diagnostics.

# convertible

- High: Add finite-difference/PDE and Monte Carlo hybrid methods for complex conversion triggers.
- High: Support stochastic credit/equity correlation and jump processes.
- Moderate: Improve calibration helpers for implied volatility/credit from market CB quotes.

# dcf

- High: Add probabilistic/scenario-weighted DCF paths and tax/CapEx/depreciation schedules.
- High: Support multi-stage growth/discount curves and mid-year discounting options.
- Moderate: Provide built-in sensitivity tables (WACC/g matrices) and Monte Carlo on key drivers.

# deposit

- Moderate: Add support for compounding/linear vs ACT/360 accrual toggles and holiday-adjusted start/end shifts.
- Low: Include callable/extendable deposit variants if needed.

# equity

- High: Add borrow cost/financing spread modeling for short/levered positions.
- High: Support corporate action adjustments (splits/dividends) through convenience helpers.
- Moderate: Provide richer risk decomposition (beta attribution, factor exposures) via integration hooks.

# equity_option

- High: Add explicit Bermudan exercise schedule support and early-exercise policy controls.
- High: Support local/stochastic volatility smile models and jump diffusion variants.
- Moderate: Provide American option greeks via lattice differentiation or Barone-Adesi/Whaley approximations.

# fra

- High: Add convexity adjustment utilities for futures vs FRA comparison.
- Moderate: Support multi-period FRA strips and averaging constructs.
- Moderate: Provide bucketed curve sensitivities and scenario stress helpers out of the box.

# fx_barrier_option

- High: Add discrete-monitoring corrections and barrier smoothing techniques for FX calendar specifics.
- High: Support stochastic/local vol and jumps; quanto adjustments for cross-currency settlements.
- Moderate: Include early-exercise/windowed barrier styles if demanded by products.

# fx_option

- High: Support smile-consistent local-vol/stochastic-vol models and skew-aware greeks.
- High: Add American/barrier-style adjustments or link to barrier pricers for hybrids.
- Moderate: Provide quanto adjustments and proxy hedging analytics for cross-ccy exposures.

# fx_spot

- High: Add forward/points support and broken-date interpolation for delivery beyond spot.
- Moderate: Include bid/ask spread and transaction-cost modeling.
- Moderate: Provide settlement netting and counterparty exposure hooks.

# fx_swap

- High: Add CSA/basis spread handling and discount-curve alignment diagnostics.
- High: Support broken-date interpolation for near/far beyond standard tenors.
- Moderate: Provide FX swaption hooks or optional early termination features.

# inflation_linked_bond

- High: Add seasonality decomposition and explicit seasonality-adjusted interpolation.
- High: Support stochastic inflation and correlation with rates for risk scenarios.
- Moderate: Include callable linker features and convexity adjustments vs nominal curve.

# inflation_swap

- High: Add couponized (periodic) inflation swap support with per-period accrual and payment schedules.
- High: Include seasonality and convexity adjustments vs nominal/real curves.
- High: Support stochastic inflation models and correlation with rates for stress testing.

# ir_future

- High: Add exchange-specific delivery options and cheapest-to-deliver style adjustments where applicable.
- Moderate: Support normal/Bachelier modeling for low-rate environments and alt convexity models.
- High: Provide margining P&L simulation hooks and daily settlement impact analytics.

# irs

- High: Support Bermudan/cancellable swap optionality directly in-module or via swaption interop.
- High: Provide stochastic-rate (HW/LMM) pricing pathways for long-dated exotic compounding.

# lookback_option

- High: Add discrete-monitoring bias corrections and analytical approximations for seasoned paths.
- High: Support early-exercise/lookback American features via tree/LSMC methods.
- Moderate: Incorporate stochastic/local volatility and jump processes for better tail behavior.

# private_markets_fund

- High: Add scenario engines for deal-level performance (probabilistic proceeds/distributions).
- High: Support multi-currency funds with embedded FX treatment and hedging hooks.
- Moderate: Include clawback/escrow mechanics and recycling provisions in waterfall spec.

# quanto_option

- High: Add stochastic correlation and local/stochastic vol coupling between equity and FX.
- High: Support early-exercise quanto options and barrier-style quanto hybrids.
- Moderate: Provide calibration helpers for quanto drift adjustments from observed markets.

# range_accrual

- High: Support stochastic volatility/jumps and correlated multi-asset ranges.
- Moderate: Add analytical approximations for narrow ranges to reduce MC runtime.
- Moderate: Provide gradient/adjoint Greeks for lower-variance sensitivity estimates.

# repo

- High: Support triparty eligibility schedules and collateral substitution events.
- High: Include fail/recall penalties and optional early termination features.

# revolving_credit

- Critical: Add GAAP/IFRS effective interest treatment and CECL/expected-loss hooks.
- High: Enrich stochastic engine with jump/regime processes and multi-currency support with FX hedging.
- High: Provide prebuilt stress packs (utilization/rate/credit) and visualization for drawdown/liquidity analytics.

# structured_credit

- High: Support base/curvature OAS grids and callable step-up tranches.

# swaption

- High: Support stochastic rate models (HW/LMM) and smile-consistent pricing beyond SABR interpolation.
- High: Provide callable CMS/INF structures interop and more settlement-style options.

# term_loan

- Critical: Add GAAP/IFRS reporting and OID with EIR amortization (OidEirSpec).
- High: Add CECL/expected credit loss provisioning hooks.
- High: Revolver integration combining DDTL with revolving credit features.
- Moderate: Advanced PIK schedules with time-varying fractions (PikSpec).

# trs

- High: Add margining and collateral modeling, plus resettable notionals and pathwise financing accrual.
- High: Support stochastic equity/credit processes for total-return legs and correlation to financing leg.
- Moderate: Provide coupon reinvestment/fee modeling and early termination options.

# variance_swap

- High: Add corridor/conditional variance features and gamma swaps.
- High: Support stochastic volatility models for forward variance projection and fair strike estimation.
- Moderate: Provide realized path builders and corporate-action aware return cleaners.
