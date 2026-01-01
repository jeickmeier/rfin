Advanced Tutorials
==================

Master advanced topics for production quantitative finance workflows.

.. toctree::
   :maxdepth: 1

   01_monte_carlo_valuation
   02_structured_products
   03_xva_calculations
   04_backtesting_strategies
   05_performance_optimization
   06_custom_pricers
   07_regulatory_reporting
   08_production_deployment

Prerequisites
-------------

Before starting these tutorials, you should:

* Complete all :doc:`../intermediate/index` tutorials
* Have strong quantitative finance background
* Understand Monte Carlo methods and numerical methods
* Be familiar with production system requirements
* Have experience with performance profiling

Tutorial Overview
-----------------

1. **Monte Carlo Valuation** - Advanced MC techniques (variance reduction, LSMC, xVA)
2. **Structured Products** - Callable bonds, CLOs, MBS, waterfall structures
3. **xVA Calculations** - CVA, DVA, FVA calculation and exposure profiles
4. **Backtesting Strategies** - Time-series portfolio revaluation and PnL attribution
5. **Performance Optimization** - Profiling, batching, parallelism, zero-copy
6. **Custom Pricers** - Implement new pricing models and register in PricerRegistry
7. **Regulatory Reporting** - FRTB, SA-CCR, SIMM margin calculations
8. **Production Deployment** - Benchmarking, testing, deployment patterns

Learning Outcomes
-----------------

After completing these tutorials, you will:

* Build production-grade valuation systems
* Implement custom pricing models efficiently
* Optimize computation for large-scale portfolios
* Generate regulatory capital and margin reports
* Deploy finstack in production environments
* Contribute back to the finstack ecosystem

Time Investment
---------------

Each tutorial takes approximately **1-3 hours**. Total time: ~15-20 hours.

Real-World Applications
-----------------------

These tutorials draw from real production use cases:

* **Hedge funds**: Multi-strategy risk aggregation
* **Investment banks**: Exotic derivatives trading desks
* **Asset managers**: Fixed income portfolio optimization
* **Regulators**: Capital adequacy monitoring
* **Prop trading**: High-frequency backtesting

Next: :doc:`01_monte_carlo_valuation`
