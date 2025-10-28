//! Multi-Level Monte Carlo (MLMC) framework.
//!
//! Implements variance reduction through level hierarchy with different
//! time step sizes, achieving O(ε^{-2}) complexity vs O(ε^{-3}) for standard MC.
//!
//! # Key Idea
//!
//! Instead of estimating E[P_h] directly with fine step size h, use:
//!
//! ```text
//! E[P_h] = E[P_h0] + Σ_{ℓ=1}^L E[P_hℓ - P_hℓ₋₁]
//! ```
//!
//! where hℓ = h0/2^ℓ (telescoping sum).
//!
//! Each correction E[P_hℓ - P_hℓ₋₁] uses **coupled paths** (same Brownian
//! increments) to reduce variance, and we allocate paths optimally:
//!
//! ```text
//! N_ℓ ∝ √(V_ℓ / C_ℓ)
//! ```
//!
//! Reference: Giles (2008) - "Multilevel Monte Carlo Path Simulation"

pub mod level;
pub mod engine;
pub mod estimator;

pub use level::MlmcLevel;
pub use engine::{MlmcEngine, MlmcConfig};
pub use estimator::{MlmcEstimate, optimal_allocation};

