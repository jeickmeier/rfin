# Summary

[Introduction](introduction.md)

---

# Getting Started

- [Overview](getting-started/README.md)
  - [Installation](getting-started/installation.md)
  - [Quick Start — Python](getting-started/quickstart-python.md)
  - [Quick Start — Rust](getting-started/quickstart-rust.md)
  - [Quick Start — WASM](getting-started/quickstart-wasm.md)

---

# Architecture

- [Overview](architecture/README.md)
  - [Core Primitives](architecture/core-primitives/README.md)
    - [Currency & Money](architecture/core-primitives/currency-money.md)
    - [Dates & Calendars](architecture/core-primitives/dates-calendars.md)
    - [Schedules & Periods](architecture/core-primitives/schedules-periods.md)
    - [Configuration](architecture/core-primitives/config.md)
  - [Market Data](architecture/market-data/README.md)
    - [Discount Curves](architecture/market-data/discount-curves.md)
    - [Forward Curves](architecture/market-data/forward-curves.md)
    - [Hazard Curves](architecture/market-data/hazard-curves.md)
    - [Volatility Surfaces](architecture/market-data/volatility-surfaces.md)
    - [FX Rates](architecture/market-data/fx-rates.md)
  - [Instruments](architecture/instruments/README.md)
    - [Fixed Income](architecture/instruments/fixed_income.md)
    - [Commodity](architecture/instruments/commodity.md)
    - [Rates](architecture/instruments/rates.md)
    - [Credit](architecture/instruments/credit.md)
    - [Equity](architecture/instruments/equity.md)
    - [FX](architecture/instruments/fx.md)
    - [Exotic](architecture/instruments/exotic.md)
  - [Risk](architecture/risk/README.md)
    - [Metrics](architecture/risk/metrics.md)
    - [Attribution](architecture/risk/attribution.md)
    - [Scenarios](architecture/risk/scenarios.md)
  - [Portfolio](architecture/portfolio/README.md)
    - [Valuation](architecture/portfolio/valuation.md)
    - [Grouping](architecture/portfolio/grouping.md)
    - [Optimization](architecture/portfolio/optimization.md)
  - [Statements](architecture/statements/README.md)
    - [Waterfalls](architecture/statements/waterfalls.md)
    - [Covenants](architecture/statements/covenants.md)
    - [Forecasting](architecture/statements/forecasting.md)
  - [Analytics](architecture/analytics/README.md)
    - [Expressions](architecture/analytics/expressions.md)
  - [Monte Carlo](architecture/monte-carlo/README.md)
    - [Path Generation](architecture/monte-carlo/path-generation.md)
    - [Pricing](architecture/monte-carlo/pricing.md)
  - [Binding Layer](architecture/binding-layer/README.md)
    - [Python Bindings](architecture/binding-layer/python-bindings.md)
    - [WASM Bindings](architecture/binding-layer/wasm-bindings.md)

---

# Cookbooks

- [Overview](cookbooks/README.md)
  - [Curve Building](cookbooks/curve-building.md)
  - [Bond Pricing](cookbooks/bond-pricing.md)
  - [Swap Pricing](cookbooks/swap-pricing.md)
  - [Options Pricing](cookbooks/options-pricing.md)
  - [Credit Analysis](cookbooks/credit-analysis.md)
  - [Portfolio Valuation](cookbooks/portfolio-valuation.md)
  - [Scenario Analysis](cookbooks/scenario-analysis.md)
  - [Statement Modeling](cookbooks/statement-modeling.md)
  - [Monte Carlo](cookbooks/monte-carlo.md)
  - [P&L Attribution](cookbooks/pnl-attribution.md)
  - [Exotic Options](cookbooks/exotic-options.md)
  - [Margin & Netting](cookbooks/margin-netting.md)

---

# Extending Finstack

- [Overview](extending/README.md)
  - [Add an Instrument](extending/add-instrument.md)
  - [Add a Pricer](extending/add-pricer.md)
  - [Add a Python Binding](extending/add-python-binding.md)
  - [Add a WASM Binding](extending/add-wasm-binding.md)
  - [Add a Metric](extending/add-metric.md)
  - [Add Market Data](extending/add-market-data.md)

---

# Conventions

- [Overview](conventions/README.md)
  - [Naming](conventions/naming.md)
  - [Error Handling](conventions/error-handling.md)
  - [Testing](conventions/testing.md)
  - [Documentation](conventions/documentation.md)

---

# Reference

- [Overview](reference/README.md)
  - [Crate Index](reference/crate-index.md)
  - [Metric Keys](reference/metric-keys.md)
  - [Market Conventions](reference/market-conventions.md)
  - [Error Catalog](reference/error-catalog.md)

---

# Notebooks

- [Overview](notebooks/README.md)

<!-- Notebook entries require the mdbook-jupyter preprocessor.
     Uncomment [preprocessor.jupyter] in book.toml and the entries below:

  - [Core: Currency, Money & Config](notebooks/core/01_core_basics_currency_money_config.ipynb)
  - [Core: Dates, Calendars & Schedules](notebooks/core/02_core_dates_calendars_daycounts_schedules.ipynb)
  - [Core: Market Data & Curves](notebooks/core/03_core_market_data_curves_fx.ipynb)
  - [Core: Cashflows & Math](notebooks/core/04_core_cashflows_xirr_math.ipynb)
  - [Core: End-to-End Workflow](notebooks/core/05_core_end_to_end_workflow.ipynb)
  - [Core: Analytics Expressions](notebooks/core/06_core_analytics_expressions.ipynb)
  - [Valuations: Intro & Pricer Registry](notebooks/valuations/01_valuations_intro_pricer_registry.ipynb)
  - [Valuations: Market Data & Curves](notebooks/valuations/02_valuations_market_data_and_curves.ipynb)
  - [Valuations: Cashflows & Schedules](notebooks/valuations/03_valuations_cashflows_and_schedules.ipynb)
  - [Valuations: Risk & Attribution](notebooks/valuations/04_valuations_risk_and_attribution.ipynb)
  - [Valuations: Bonds](notebooks/valuations/05_valuations_bonds.ipynb)
  - [Valuations: Swaps & Basis](notebooks/valuations/06_valuations_swaps_and_basis.ipynb)
  - [Valuations: Caps, Floors & Swaptions](notebooks/valuations/07_valuations_caps_floors_swaptions.ipynb)
  - [Valuations: Credit Derivatives](notebooks/valuations/08_valuations_credit_derivatives.ipynb)
  - [Valuations: Structured Credit](notebooks/valuations/09_valuations_structured_credit.ipynb)
  - [Valuations: Equity Derivatives](notebooks/valuations/10_valuations_equity_derivatives.ipynb)
  - [Valuations: FX Derivatives](notebooks/valuations/11_valuations_fx_derivatives.ipynb)
  - [Valuations: Path-Dependent Options](notebooks/valuations/12_valuations_path_dependent_options.ipynb)
  - [Valuations: Barrier & Autocallable](notebooks/valuations/13_valuations_barrier_and_autocallable.ipynb)
  - [Valuations: Variance & Quanto](notebooks/valuations/14_valuations_variance_and_quanto.ipynb)
  - [Valuations: Private Credit](notebooks/valuations/15_valuations_private_credit.ipynb)
  - [Valuations: Convertibles & Hybrids](notebooks/valuations/16_valuations_convertibles_and_hybrids.ipynb)
  - [Valuations: Monte Carlo Deep Dive](notebooks/valuations/17_valuations_monte_carlo_deep_dive.ipynb)
  - [Valuations: Calibration](notebooks/valuations/18_valuations_calibration.ipynb)
  - [Statements: Example](notebooks/statements/statements_example.ipynb)
  - [Statements: Adjustments](notebooks/statements/adjustments_demo.ipynb)
  - [Statements: Advanced Covenants](notebooks/statements/advanced_covenants_demo.ipynb)
  - [Statements: Dynamic Waterfall](notebooks/statements/dynamic_waterfall_example.ipynb)
  - [Scenarios: Introduction](notebooks/scenarios/01_scenarios_intro.ipynb)
  - [Portfolio: Introduction](notebooks/portfolio/01_portfolio_intro.ipynb)
  - [Portfolio: Optimization](notebooks/portfolio/02_portfolio_optimization.ipynb)
-->
