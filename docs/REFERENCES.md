# References (Canonical)

This file centralizes **canonical, stable** references used throughout the Finstack documentation
(Rust docs, Python `.pyi` docstrings, WASM/TypeScript docs).

Guidelines:

- Prefer **primary sources** (original paper, official standard/spec) or widely accepted reference texts.
- When a docstring cites a reference, prefer linking to an **anchor in this file** so citations remain stable.

---

## black1976

Fischer Black (1976). *The Pricing of Commodity Contracts*. **Journal of Financial Economics**, 3(1–2), 167–179.
Often referenced for the “Black ’76” lognormal model for options on **forwards/futures**.

- Link: [ScienceDirect landing page](https://www.sciencedirect.com/science/article/pii/0304405X76900246)

## blackScholes1973

Fischer Black and Myron Scholes (1973). *The Pricing of Options and Corporate Liabilities*. **Journal of Political Economy**, 81(3), 637–654.

- DOI: [`10.1086/260062`](https://doi.org/10.1086/260062)

## merton1973

Robert C. Merton (1973). *Theory of Rational Option Pricing*. **The Bell Journal of Economics and Management Science**, 4(1), 141–183.
Extends the Black–Scholes framework and is commonly cited for the continuous-dividend-yield form.

- DOI: [`10.2307/3003143`](https://doi.org/10.2307/3003143)

## bachelier1900

Louis Bachelier (1900). *Théorie de la spéculation*.
Foundational work for the **normal (Bachelier)** option pricing model.

- Online scan (archival): [Gallica (BnF)](https://gallica.bnf.fr/ark:/12148/bpt6k4337026)

## isda2006Definitions

ISDA (2006). *2006 ISDA Definitions*.
Canonical industry reference for many **interest rate derivative** conventions and day count conventions.

- Product page: [ISDA Bookstore](https://www.isda.org/book/2006-isda-definitions/)

## isdaDayCount

ISDA day count / accrual conventions are described in the ISDA Definitions and related market practice documents.
In Finstack docstrings, “ISDA day count” citations generally refer to:

- ISDA (2006). *2006 ISDA Definitions* (see [`isda2006Definitions`](#isda2006definitions))

## hullOptionsFuturesDerivatives

John C. Hull. *Options, Futures, and Other Derivatives* (various editions).
Widely used reference text covering Black–Scholes(-Merton), Black ’76, caps/floors, and swaptions at a
practitioner-friendly level.

## abramowitzStegun1964

Milton Abramowitz and Irene A. Stegun (1964). *Handbook of Mathematical Functions with Formulas, Graphs, and Mathematical Tables*.
Classic reference for special functions and distribution definitions (and many approximations).

- Online (NIST mirror): [NIST Digital Library of Mathematical Functions (DLMF)](https://dlmf.nist.gov/)

## devroye1986

Luc Devroye (1986). *Non-Uniform Random Variate Generation*.
Canonical reference for random variate generation (sampling) methods.

- Online (author-hosted): [PDF](https://luc.devroye.org/rnbookindex.html)

## garmanKohlhagen1983

Mark B. Garman and Steven W. Kohlhagen (1983). *Foreign Currency Option Values*. **Journal of International Money and Finance**, 2(3), 231–237.
Standard reference for the Black–Scholes-style model for FX options with domestic/foreign rates.

## dupire1994

Bruno Dupire (1994). *Pricing with a Smile*. **Risk**, 7(1), 18–20.
Common reference for local volatility (Dupire) models and the implied/local vol relationship.

## heston1993

Steven L. Heston (1993). *A Closed-Form Solution for Options with Stochastic Volatility with Applications to Bond and Currency Options*. **The Review of Financial Studies**, 6(2), 327–343.

## haganSABR2002

Patrick S. Hagan, Deep Kumar, Andrew Lesniewski, and Diana Woodward (2002). *Managing Smile Risk*. **Wilmott Magazine**.
Canonical reference for the SABR model and widely used implied vol approximations.

## demeterfiVarianceSwaps1999

K. Demeterfi, E. Derman, M. Kamal, and J. Zou (1999). *More Than You Ever Wanted to Know About Volatility Swaps*.
Canonical practitioner reference for variance/volatility swap replication and conventions.

## brigoMercurio2006

Damiano Brigo and Fabio Mercurio (2006). *Interest Rate Models — Theory and Practice* (2nd ed.).
Standard reference for IR modelling, curve construction concepts, and IR derivatives.

## okane2008

Dominic O’Kane (2008). *Modelling Single-name and Multi-name Credit Derivatives*.
Widely cited reference text for CDS pricing, hazard rates, and credit derivative conventions.

## liGaussianCopula2000

David X. Li (2000). *On Default Correlation: A Copula Function Approach*. **Journal of Fixed Income**, 9(4), 43–54.
Often cited for Gaussian copula approaches used in structured credit (e.g., CDO tranches).

## gobet2009BarrierMC

Emmanuel Gobet (2009). *Advanced Monte Carlo Methods for Barrier and Related Exotic Options*. In: **Handbook of Numerical Analysis**, Vol. 15, 497–528.
Survey-style reference for practical barrier option simulation techniques (e.g., Brownian bridge corrections).

## gobetMiri2014AveragedDiffusions

Emmanuel Gobet and Mohammed Miri (2014). *Weak Approximation of Averaged Diffusion Processes*. **Stochastic Processes and their Applications**, 124(1), 475–504.
Reference for weak approximation / expansion approaches often used for average-based payoffs (Asian/basket-type structures).

## damodaranInvestmentValuation

Aswath Damodaran. *Investment Valuation: Tools and Techniques for Determining the Value of Any Asset* (various editions).
Widely cited practitioner reference for corporate DCF valuation (WACC discounting, terminal value via perpetuity).

---

## sharpe1966

William F. Sharpe (1966). *Mutual Fund Performance*. **Journal of Business**, 39(1), 119–138.
Original paper introducing the reward-to-variability ratio, now known as the Sharpe ratio.

## sortinoVanDerMeer1991

Frank A. Sortino and Robert van der Meer (1991). *Downside Risk*. **Journal of Portfolio Management**, 17(4), 27–31.
Introduces the Sortino ratio, which penalises only downside volatility below a minimum acceptable return.

## youngCalmar1991

Terry W. Young (1991). *Calmar Ratio: A Smoother Tool*. **Futures**, 20(1), 40.
Defines the Calmar ratio as CAGR divided by maximum drawdown, used to assess risk-adjusted trend-following performance.

## kelly1956

John L. Kelly Jr. (1956). *A New Interpretation of Information Rate*. **Bell System Technical Journal**, 35(4), 917–926.
Derives the Kelly criterion: the fraction of capital to wager that maximises long-run wealth growth.

## artzner1999CoherentRisk

Philippe Artzner, Freddy Delbaen, Jean-Marc Eber, and David Heath (1999). *Coherent Measures of Risk*. **Mathematical Finance**, 9(3), 203–228.
Foundational axiomatization of coherent risk measures; establishes Expected Shortfall (CVaR) as a coherent alternative to VaR.

- DOI: [`10.1111/1467-9965.00068`](https://doi.org/10.1111/1467-9965.00068)

## jpmorgan1996RiskMetrics

J.P. Morgan / Reuters (1996). *RiskMetrics™ — Technical Document* (4th ed.).
Industry standard reference for historical and parametric Value-at-Risk (VaR) methodology.

- Online: [RiskMetrics Technical Document](https://www.msci.com/documents/10199/5915b101-4206-4ba0-aee2-3449d5c7e95a)

## martinUlcer1987

Peter G. Martin (1987). *The Investor's Guide to Fidelity Funds*.
Introduces the Ulcer Index as the root-mean-square of drawdown depths, measuring investor distress.

## grinoldKahn1999ActivePortfolio

Richard C. Grinold and Ronald N. Kahn (1999). *Active Portfolio Management* (2nd ed.). McGraw-Hill.
Standard practitioner reference for information ratio, tracking error, and the fundamental law of active management.

---

## Notes on use in docstrings

In Python `.pyi` docstrings, prefer:

- “Sources” section with bullet points that link to anchors here, e.g.
  - `- Black (1976): see docs/REFERENCES.md#black1976`
