pub mod distributions;
pub mod integration;
pub mod linalg;
pub mod probability;
pub mod random;
pub mod solver;
pub mod special_functions;
pub mod stats;
pub mod summation;

// Distributions
pub use distributions::{
    binomial_distribution_js as binomialDistribution, binomial_probability_js as binomialProbability,
    chi_squared_cdf_js as chiSquaredCdf, chi_squared_pdf_js as chiSquaredPdf,
    chi_squared_quantile_js as chiSquaredQuantile, exponential_cdf_js as exponentialCdf,
    exponential_pdf_js as exponentialPdf, exponential_quantile_js as exponentialQuantile,
    log_binomial_coefficient_js as logBinomialCoefficient, log_factorial_js as logFactorial,
    lognormal_cdf_js as lognormalCdf, lognormal_pdf_js as lognormalPdf,
    lognormal_quantile_js as lognormalQuantile,
};

// Integration
pub use integration::{
    adaptive_simpson as adaptiveSimpson, gauss_legendre_integrate as gaussLegendreIntegrate,
    gauss_legendre_integrate_adaptive as gaussLegendreIntegrateAdaptive,
    gauss_legendre_integrate_composite as gaussLegendreIntegrateComposite,
    simpson_rule as simpsonRule, trapezoidal_rule as trapezoidalRule,
    JsGaussHermiteQuadrature as GaussHermiteQuadrature,
};

// Linear Algebra
pub use linalg::{
    apply_correlation_js as applyCorrelation,
    build_correlation_matrix_js as buildCorrelationMatrix,
    cholesky_decomposition_js as choleskyDecomposition,
    validate_correlation_matrix_js as validateCorrelationMatrix,
};

// Probability
pub use probability::{
    correlation_bounds_js as correlationBounds, joint_probabilities_js as jointProbabilities,
    JsCorrelatedBernoulli as CorrelatedBernoulliDist,
};

// Random
pub use random::{box_muller_transform_js as boxMullerTransform, JsRng as Rng};

// Solvers
pub use solver::{JsBrentSolver as BrentSolver, JsNewtonSolver as NewtonSolver};

// Special Functions
pub use special_functions::{
    erf_js as erf, norm_cdf_js as normCdf, norm_inv_cdf_js as normInvCdf, norm_pdf_js as normPdf,
    student_t_cdf_js as studentTCdf, student_t_inv_cdf_js as studentTInvCdf,
};

// Statistics
pub use stats::{
    correlation_js as correlation, covariance_js as covariance, mean_js as mean,
    variance_js as variance,
};

// Summation
pub use summation::{
    kahan_sum_js as kahanSum, neumaier_sum_js as neumaierSum,
    JsNeumaierAccumulator as SumAccumulator,
};
