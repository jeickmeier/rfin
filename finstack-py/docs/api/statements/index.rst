Statements API
==============

The statements module provides financial statement modeling and evaluation.

.. toctree::
   :maxdepth: 2

   builder
   evaluator
   forecasts
   registry
   extensions

Overview
--------

The statements module includes:

* **ModelBuilder**: Declarative model construction
* **Evaluator**: Period-by-period evaluation with precedence rules
* **Forecasts**: Deterministic and statistical methods
* **Registry**: Reusable metric definitions
* **Extensions**: Corkscrew validation, credit scorecards

Model Builder
-------------

.. automodule:: finstack.statements
   :members: ModelBuilder, NodeSpec, NodeType
   :undoc-members:
   :show-inheritance:

Build financial models declaratively with type-state enforcement.

Example:

.. code-block:: python

   from finstack import ModelBuilder, ForecastSpec

   builder = ModelBuilder()
   builder.id("my_model")
   builder.periods(start, end, Tenor.Quarterly())

   # Add node with forecast
   builder.node(
       node_id="revenue",
       node_type=NodeType.Revenue,
       forecast=ForecastSpec.growth_pct(rate=0.10)
   )

   # Add node with formula
   builder.node(
       node_id="gross_profit",
       node_type=NodeType.GrossProfit,
       formula="revenue - cogs"
   )

   model = builder.build()

Evaluator
---------

.. automodule:: finstack.statements
   :members: Evaluator, Results
   :undoc-members:
   :show-inheritance:

Evaluate models with precedence rules: **Value > Forecast > Formula**.

Example:

.. code-block:: python

   from finstack import Evaluator

   evaluator = Evaluator()
   results = evaluator.evaluate(model)

   # Access results
   revenue = results.get_node("revenue")
   for period, value in revenue.items():
       print(f"{period}: {value}")

   # Export to DataFrame
   df = results.to_polars()

Forecasts
---------

.. automodule:: finstack.statements
   :members: ForecastSpec, ForecastMethod
   :undoc-members:
   :show-inheritance:

Generate period values from forecasts.

**Available Methods**:

* **Deterministic**: ForwardFill, GrowthPercentage, CurvePercentage
* **Statistical**: Normal, LogNormal (with explicit seed)
* **Override**: Base forecast with sparse period overrides
* **TimeSeries**: External data reference
* **Seasonal**: Repeating patterns with optional growth

Example:

.. code-block:: python

   from finstack import ForecastSpec

   # Growth forecast
   growth = ForecastSpec.growth_pct(rate=0.05)

   # Seasonal forecast
   seasonal = ForecastSpec.seasonal(
       pattern=[1.0, 1.2, 0.9, 1.1],  # Q1, Q2, Q3, Q4
       mode="multiplicative",
       growth_rate=0.03
   )

   # Statistical forecast
   lognormal = ForecastSpec.lognormal(mean=100000, std_dev=10000, seed=42)

Registry
--------

.. automodule:: finstack.statements
   :members: Registry, MetricDefinition
   :undoc-members:
   :show-inheritance:

Reusable metric definitions with inter-metric dependencies.

Example:

.. code-block:: python

   from finstack import Registry

   # Load built-in metrics
   registry = Registry.load_builtins()

   # List available metrics
   metrics = registry.list_metrics()

   # Add metric to model from registry
   builder.add_metric_from_registry("fin.ebitda", registry)

   # Load custom metrics from JSON
   custom_registry = Registry.load_from_json(json_str)

Extensions
----------

.. automodule:: finstack.statements
   :members: CorkscrewExtension, CreditScorecardExtension, CorkscrewConfig, ScorecardConfig
   :undoc-members:
   :show-inheritance:

Post-evaluation validation and augmentation.

**Corkscrew Extension**: Validate balance sheet roll-forward.

.. code-block:: python

   from finstack import CorkscrewExtension, CorkscrewConfig, AccountType

   config = CorkscrewConfig()
   config.add_account("cash", AccountType.Asset, ["revenue", "expenses"])
   config.tolerance(0.01)

   extension = CorkscrewExtension(config)

   # Apply to results
   validated = extension.execute(results, model)

**Credit Scorecard Extension**: Assign credit ratings.

.. code-block:: python

   from finstack import CreditScorecardExtension, ScorecardConfig, ScorecardMetric

   config = ScorecardConfig()
   config.add_metric(
       name="leverage",
       formula="debt / ebitda",
       weight=0.4,
       thresholds={"AAA": 2.0, "BBB": 4.0}
   )

   extension = CreditScorecardExtension(config)
   rating_results = extension.execute(results, model)

See Also
--------

* :doc:`../../tutorials/intermediate/index`
* :doc:`../../examples/statements/index`
