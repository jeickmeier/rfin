

// Scenario Manager & Stress Testing
Current State: corporate.rs has basic sensitivity analysis (WACC +/- 1%), but analysts need full multi-variable scenarios. Recommendation: Implement a ScenarioManager.

Features:
Case Management: Define "Base Case", "Sponsor Case", "Downside/Stress Case", and "Upside Case".
Variable Overrides: Allow overriding growth rates, margins, and interest rate curves per scenario.
Side-by-Side Comparison: Generate reports that show key credit stats (Leverage, Interest Coverage) across all scenarios in a single view.

// Advanced Covenant Modeling
Current State: covenants.rs forecasts breaches based on simple thresholds. Recommendation: Enhance support for complex credit agreement structures.

Features:
Maintenance vs. Incurrence: Distinguish between covenants that are tested quarterly vs. those tested only upon taking an action (debt incurrence).
Springing Covenants: Logic for covenants that only activate when the Revolver is utilized > X%.
Baskets & Headroom: Track "Available Baskets" (e.g., "General Debt Basket", "Permitted Acquisitions") and calculate "Headroom" (how much EBITDA can drop before a breach).

// Deal Sizing & LBO Solver
Current State: goal_seek.rs solves for single variables. Recommendation: A specialized DealStructurer or LBOSolver.

Features:
Multi-Constraint Solving: "Maximize Total Debt" subject to: Leverage < 4.5x AND FCCR > 1.2x.
Returns Analysis: Calculate Sponsor IRR and MOIC (Multiple on Invested Capital) based on entry/exit multiples and leverage.
Sources & Uses: Automatically generate a balanced "Sources and Uses" table for the transaction.

// Unit Economics & KPI Drivers
Current State: Forecasting seems generic (ForecastSpec::growth). Recommendation: Add a UnitEconomics driver module.

Features:
Price x Volume: Model revenue as Price * Volume rather than just a growth rate.
SaaS Metrics: For tech deals, explicitly model ARR Waterfall (New Bookings, Churn, Upsell, Downsell).
Cohort Analysis: If data permits, forecast based on customer cohorts.

// Peer Benchmarking
Current State: Analysis is isolated to the single company. Recommendation: A Benchmarking module.

Features:
Industry Comparison: Compare the target's margins, leverage, and growth against a dataset of industry peers or indices.
Quartile Analysis: "Is this company in the top quartile for Gross Margin but bottom quartile for Cash Conversion?"