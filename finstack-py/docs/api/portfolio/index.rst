Portfolio API
=============

The portfolio module provides position tracking, aggregation, and optimization.

.. toctree::
   :maxdepth: 2

   core
   valuation
   aggregation
   margin
   optimization

Overview
--------

The portfolio module includes:

* **Portfolio Builder**: Entity-based position management
* **Valuation**: Cross-currency aggregation with explicit FX
* **Aggregation**: Group by entity, attribute, book
* **Margin**: Netting and collateral calculations
* **Optimization**: Constrained portfolio construction

Core Types
----------

.. automodule:: finstack.portfolio
   :members: Portfolio, PortfolioBuilder, Entity, Position
   :undoc-members:
   :show-inheritance:

Build portfolios with entities and positions.

Example:

.. code-block:: python

   from finstack import PortfolioBuilder, Entity

   # Create entity
   fund = Entity(id="FUND001", name="Alpha Fund")
   
   # Build portfolio
   builder = PortfolioBuilder()
   builder.entity(fund)
   builder.base_ccy(Currency.from_code("USD"))
   builder.as_of(Date(2024, 1, 15))
   
   # Add positions
   builder.position_from_instrument(
       position_id="POS_BOND",
       instrument=bond,
       quantity=10.0,
       entity_id="FUND001"
   )
   
   portfolio = builder.build()

Valuation
---------

.. automodule:: finstack.portfolio
   :members: value_portfolio, PortfolioValuation, PositionValue
   :undoc-members:
   :show-inheritance:

Value portfolios with cross-currency aggregation.

Example:

.. code-block:: python

   from finstack import value_portfolio

   valuation = value_portfolio(portfolio, market, None)
   
   # Total value
   print(f"Total: {valuation.total_value.amount:,.2f}")
   
   # By entity
   for entity_id, value in valuation.by_entity.items():
       print(f"{entity_id}: {value.amount:,.2f}")
   
   # Export to DataFrame
   df = valuation.to_polars()

Aggregation
-----------

.. automodule:: finstack.portfolio
   :members: aggregate_by_attribute, aggregate_by_book, aggregate_metrics
   :undoc-members:
   :show-inheritance:

Flexible grouping and aggregation.

**By Attribute**:

.. code-block:: python

   from finstack import aggregate_by_attribute

   # Group by rating
   by_rating = aggregate_by_attribute(
       valuation,
       portfolio.positions,
       "rating",
       Currency.from_code("USD")
   )
   # { "AAA": Money(...), "BBB": Money(...) }

**By Book**:

.. code-block:: python

   from finstack import aggregate_by_book

   # Hierarchical book aggregation
   by_book = aggregate_by_book(
       valuation,
       portfolio,
       Currency.from_code("USD")
   )
   # Recursive rollup: child books → parent books

**Metrics**:

.. code-block:: python

   from finstack import aggregate_metrics

   # Aggregate DV01, CS01, Greeks
   metrics = aggregate_metrics(
       valuation,
       Currency.from_code("USD"),
       fx_matrix,
       Date(2024, 1, 15)
   )
   
   print(f"Portfolio DV01: {metrics.dv01}")
   print(f"Portfolio Delta: {metrics.delta}")

Margin
------

.. automodule:: finstack.portfolio
   :members: NettingSet, NettingSetManager, PortfolioMarginAggregator
   :undoc-members:
   :show-inheritance:

Margin and collateral calculations with netting sets.

Example:

.. code-block:: python

   from finstack import NettingSetManager, NettingSetId

   # Define netting sets
   manager = NettingSetManager()
   manager.add_set(NettingSetId.bilateral("COUNTERPARTY_A", "USD"))
   manager.add_set(NettingSetId.cleared("CLEARING_HOUSE", "EUR"))
   
   # Assign positions to netting sets
   manager.assign_position("POS_SWAP1", "COUNTERPARTY_A")
   
   # Calculate margin
   from finstack import PortfolioMarginAggregator
   
   aggregator = PortfolioMarginAggregator(manager, csa_terms)
   margin_result = aggregator.calculate_margin(valuation, market)
   
   # Access results
   for set_id, margin in margin_result.by_set.items():
       print(f"{set_id}: VM={margin.variation_margin}, IM={margin.initial_margin}")

Optimization
------------

.. automodule:: finstack.portfolio
   :members: PortfolioOptimizationProblem, Constraint, Objective, TradeUniverse
   :undoc-members:
   :show-inheritance:

Constrained portfolio construction and rebalancing.

Example:

.. code-block:: python

   from finstack import (
       PortfolioOptimizationProblem,
       Constraint, Objective, TradeUniverse
   )
   
   # Define universe
   universe = TradeUniverse()
   universe.add_candidate("BOND_A", bond_a, 100.0)
   universe.add_candidate("BOND_B", bond_b, 95.0)
   
   # Build problem
   problem = PortfolioOptimizationProblem()
   problem.universe(universe)
   problem.objective(Objective.maximize_yield())
   
   # Add constraints
   problem.add_constraint(Constraint.tag_exposure_limit("rating", "CCC", 0.10))
   problem.add_constraint(Constraint.weight_bounds(0.0, 0.05))
   problem.add_constraint(Constraint.budget(10_000_000))
   
   # Solve
   result = problem.solve()
   
   # Extract trades
   for trade in result.trades:
       print(f"{trade.position_id}: {trade.quantity} @ {trade.price}")

See Also
--------

* :doc:`../../tutorials/beginner/07_portfolio_basics`
* :doc:`../../tutorials/intermediate/05_portfolio_optimization`
* :doc:`../../examples/portfolio/index`
