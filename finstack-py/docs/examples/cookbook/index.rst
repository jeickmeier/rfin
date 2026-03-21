Cookbook Examples
=================

Real-world patterns and workflows for quantitative finance.

.. toctree::
   :maxdepth: 1

   build_portfolio
   stress_testing
   risk_reporting
   term_loan_modeling
   exotic_options
   curve_calibration
   monte_carlo_pricing
   portfolio_optimization

Overview
--------

The cookbook provides battle-tested patterns for common quantitative finance tasks:

1. **Build Portfolio** - Multi-asset portfolio construction
2. **Stress Testing** - Scenario analysis and composition
3. **Risk Reporting** - DV01/CS01 reports with bucketing
4. **Term Loan Modeling** - Covenant tracking and cashflow waterfalls
5. **Exotic Options** - Barrier, Asian, lookback pricing
6. **Curve Calibration** - Bootstrap curves from market quotes
7. **Monte Carlo Pricing** - Variance reduction and LSMC
8. **Portfolio Optimization** - Constrained construction

Quick Reference
---------------

Build a Multi-Asset Portfolio
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   from finstack import PortfolioBuilder, Entity

   builder = PortfolioBuilder()
   builder.entity(Entity(id="FUND", name="Alpha Fund"))
   builder.base_ccy(Currency.from_code("USD"))

   # Add bonds
   for bond in bonds:
       builder.position_from_instrument(
           position_id=f"POS_{bond.id}",
           instrument=bond,
           quantity=10.0,
           entity_id="FUND",
           tags={"asset_class": "fixed_income", "rating": bond.rating}
       )

   # Add equities
   for option in options:
       builder.position_from_instrument(
           position_id=f"POS_{option.id}",
           instrument=option,
           quantity=100.0,
           entity_id="FUND",
           tags={"asset_class": "equity_options"}
       )

   portfolio = builder.build()

Run Stress Test
~~~~~~~~~~~~~~~

.. code-block:: python

   from datetime import date
   from finstack.scenarios import ExecutionContext, ScenarioEngine
   from finstack.scenarios.dsl import from_dsl
   from finstack.statements.types import FinancialModelSpec

   # Define scenario
   scenario = from_dsl("""
       # Rates shock
       shift USD.OIS +50bp
       shift EUR.OIS +40bp

       # Equity crash
       shift equities -20%

       # FX stress
       shift fx USD/EUR +5%
   """)

   # Apply and revalue
   ctx = ExecutionContext(market, FinancialModelSpec("empty", []), date.today())
   engine = ScenarioEngine()
   report = engine.apply(scenario, ctx)
   shocked_market = ctx.market
   stressed_valuation = value_portfolio(portfolio, shocked_market, None)

   # Calculate P&L
   base_pv = base_valuation.total_value.amount
   stressed_pv = stressed_valuation.total_value.amount
   pnl = stressed_pv - base_pv

Generate Risk Report
~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   # Price with metrics
   from finstack.valuations.pricer import standard_registry

   registry = standard_registry()
   results = []

   for position in portfolio.positions:
       result = registry.price_with_metrics(
           position.instrument,
           "discounting",
           market,
           ["dv01", "cs01", "theta", "delta"]
       )
       results.append({
           "position_id": position.id,
           "pv": result.value.amount,
           "dv01": result.measures["dv01"],
           "cs01": result.measures["cs01"],
           "theta": result.measures["theta"],
           "delta": result.measures["delta"],
       })

   # Export to DataFrame
   import polars as pl
   df = pl.DataFrame(results)
   df.write_csv("risk_report.csv")

Calibrate Curves
~~~~~~~~~~~~~~~~

.. code-block:: python

   from finstack import execute_calibration, RatesQuote

   quotes = [
       RatesQuote.deposit_rate("3M", 0.045),
       RatesQuote.swap_rate("1Y", 0.048),
       RatesQuote.swap_rate("5Y", 0.050),
       RatesQuote.swap_rate("10Y", 0.052),
   ]

   plan = {
       "steps": [{
           "kind": "discount",
           "curve_id": "USD.OIS",
           "base_date": "2024-01-15",
           "quotes": [q.to_json() for q in quotes]
       }]
   }

   result = execute_calibration(plan, {})
   curve = result.curves["USD.OIS"]

Optimize Portfolio
~~~~~~~~~~~~~~~~~~

.. code-block:: python

   from finstack import (
       PortfolioOptimizationProblem,
       Constraint, Objective, TradeUniverse
   )

   # Define universe
   universe = TradeUniverse()
   for bond in candidate_bonds:
       universe.add_candidate(bond.id, bond, bond.price)

   # Build problem
   problem = PortfolioOptimizationProblem()
   problem.universe(universe)
   problem.objective(Objective.maximize_yield())

   # Constraints
   problem.add_constraint(Constraint.budget(10_000_000))
   problem.add_constraint(Constraint.tag_exposure_limit("rating", "CCC", 0.10))
   problem.add_constraint(Constraint.weight_bounds(0.0, 0.05))

   # Solve
   result = problem.solve()

Full Examples
-------------

For complete, runnable examples with market data and validation:

* See ``finstack-py/examples/cookbook/`` directory
* Run with: ``python examples/cookbook/build_portfolio.py``
* All examples include error handling and output formatting

User Personas
-------------

Examples are organized by user persona:

* **Quantitative Analysts**: Exotic pricing, Monte Carlo, calibration
* **Credit/Equity Analysts**: Portfolio construction, covenant modeling
* **Risk Managers**: Stress testing, margin, VaR
* **Portfolio Managers**: Optimization, rebalancing, attribution
* **Data Engineers**: DataFrame I/O, batch processing, pipelines

Next Steps
----------

* Review full examples in the repository
* Adapt patterns to your specific use case
* Check the :doc:`../../api/index` for detailed API reference
* Ask questions on GitHub Discussions
