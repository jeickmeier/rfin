# Statements Documentation Structure

**Last updated:** 2025-09-30

---

## Overview

The statements documentation has been restructured from a single 4500+ line document into a focused, modular documentation set. This improves navigability, maintainability, and makes it easier to find specific information.

---

## New Structure

```
docs/new/04_statements/
├── 04_statements_prd.md              # Product requirements (unchanged)
├── 04_statements_tdd.md              # Technical design doc (unchanged)
├── statement_plan_ARCHIVE.md         # Original plan (archived for reference)
├── DOCUMENTATION_STRUCTURE.md        # This file
└── statements/                        # ⭐ New structured docs
    ├── README.md                      # Executive summary & quick start
    ├── ARCHITECTURE.md                # High-level design
    ├── IMPLEMENTATION_PLAN.md         # Phased rollout strategy
    ├── API_REFERENCE.md               # Wire types & DSL syntax
    ├── CAPITAL_STRUCTURE.md           # Debt/equity integration
    ├── TESTING_STRATEGY.md            # Testing approach
    └── examples/
        ├── basic_pl_statement.md      # Simple P&L example
        ├── forecasting_methods.md     # All forecast types
        └── [more examples...]
```

---

## Document Purpose

| Document | Purpose | Audience |
|----------|---------|----------|
| **README.md** | Quick start, overview, feature summary | All users |
| **ARCHITECTURE.md** | System design, integration points, design decisions | Engineers implementing |
| **IMPLEMENTATION_PLAN.md** | Phased development strategy, PR breakdown | Engineers implementing |
| **API_REFERENCE.md** | Complete API documentation (types, DSL, functions) | All users |
| **CAPITAL_STRUCTURE.md** | Debt/equity modeling guide | Users needing capital structure |
| **TESTING_STRATEGY.md** | Test organization and requirements | Engineers implementing |
| **examples/** | Working code examples | All users |

---

## Quick Navigation

### I want to...

**...understand what this crate does**  
→ [README.md](./statements/README.md)

**...build my first model**  
→ [README.md#quick-start](./statements/README.md#-quick-start)  
→ [examples/basic_pl_statement.md](./statements/examples/basic_pl_statement.md)

**...understand the architecture**  
→ [ARCHITECTURE.md](./statements/ARCHITECTURE.md)

**...see the implementation timeline**  
→ [IMPLEMENTATION_PLAN.md](./statements/IMPLEMENTATION_PLAN.md)

**...look up DSL syntax**  
→ [API_REFERENCE.md#2-dsl-syntax](./statements/API_REFERENCE.md#2-dsl-syntax)

**...integrate debt instruments**  
→ [CAPITAL_STRUCTURE.md](./statements/CAPITAL_STRUCTURE.md)

**...write tests**  
→ [TESTING_STRATEGY.md](./statements/TESTING_STRATEGY.md)

**...see examples**  
→ [examples/](./statements/examples/)
