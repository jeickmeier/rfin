/// Solver traits for bootstrap and global optimization.
pub mod traits;
pub mod bootstrap;
pub mod global;

pub use traits::{BootstrapTarget, GlobalSolveTarget};
pub use bootstrap::SequentialBootstrapper;
pub use global::GlobalOptimizer;

