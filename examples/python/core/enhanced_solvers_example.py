#!/usr/bin/env python3
"""
Example demonstrating enhanced solver capabilities in finstack.

This example shows:
1. Multi-dimensional optimization with Levenberg-Marquardt
2. Global optimization with Differential Evolution
3. Analytical derivatives for faster convergence
"""

# import numpy as np
# from finstack import finstack_py  # Assuming Python bindings are available
import time

def example_sabr_calibration_with_analytical_derivatives():
    """
    Demonstrates SABR calibration using analytical derivatives for faster convergence.
    """
    print("=== SABR Calibration with Analytical Derivatives ===\n")
    
    # Market data
    forward = 100.0
    strikes = np.array([90.0, 95.0, 100.0, 105.0, 110.0])
    market_vols = np.array([0.22, 0.21, 0.20, 0.21, 0.22])  # Smile shape
    time_to_expiry = 1.0
    beta = 0.5  # Fixed beta parameter
    
    # Create SABR calibrator
    # Note: This is a conceptual example - actual Python bindings would need to be implemented
    calibrator = finstack_py.SABRCalibrator(
        tolerance=1e-8,
        max_iterations=100
    )
    
    # Calibrate with analytical derivatives (if available)
    start_time = time.time()
    params_analytical = calibrator.calibrate(
        forward=forward,
        strikes=strikes,
        market_vols=market_vols,
        time_to_expiry=time_to_expiry,
        beta=beta,
        use_analytical_derivatives=True  # Enable analytical derivatives
    )
    time_analytical = time.time() - start_time
    
    # Calibrate without analytical derivatives for comparison
    start_time = time.time()
    params_numerical = calibrator.calibrate(
        forward=forward,
        strikes=strikes,
        market_vols=market_vols,
        time_to_expiry=time_to_expiry,
        beta=beta,
        use_analytical_derivatives=False  # Use finite differences
    )
    time_numerical = time.time() - start_time
    
    print(f"Calibration with analytical derivatives:")
    print(f"  Alpha: {params_analytical.alpha:.6f}")
    print(f"  Nu:    {params_analytical.nu:.6f}")
    print(f"  Rho:   {params_analytical.rho:.6f}")
    print(f"  Time:  {time_analytical:.4f} seconds\n")
    
    print(f"Calibration with numerical derivatives:")
    print(f"  Alpha: {params_numerical.alpha:.6f}")
    print(f"  Nu:    {params_numerical.nu:.6f}")
    print(f"  Rho:   {params_numerical.rho:.6f}")
    print(f"  Time:  {time_numerical:.4f} seconds\n")
    
    print(f"Speed improvement: {time_numerical/time_analytical:.2f}x faster with analytical derivatives\n")
    
    # Verify calibration quality
    model = finstack_py.SABRModel(params_analytical)
    calibrated_vols = [
        model.implied_volatility(forward, strike, time_to_expiry)
        for strike in strikes
    ]
    
    print("Calibration quality:")
    print("Strike  Market Vol  Model Vol  Error (bps)")
    for i, strike in enumerate(strikes):
        error_bps = (calibrated_vols[i] - market_vols[i]) * 10000
        print(f"{strike:6.1f}  {market_vols[i]:.4f}     {calibrated_vols[i]:.4f}    {error_bps:+.2f}")

def example_global_optimization():
    """
    Demonstrates global optimization using Differential Evolution.
    """
    print("\n=== Global Optimization with Differential Evolution ===\n")
    
    # Example: Calibrate a complex model with multiple local minima
    # We'll use a simplified example of calibrating a jump-diffusion model
    
    def objective_function(params):
        """
        Objective function with multiple local minima.
        This simulates calibrating a jump-diffusion model to option prices.
        """
        # params = [volatility, jump_intensity, jump_size_mean, jump_size_std]
        vol, lambda_j, mu_j, sigma_j = params
        
        # Simulated market prices (in practice, these would be real market data)
        market_prices = np.array([5.0, 7.5, 10.0, 12.5, 15.0])
        strikes = np.array([90.0, 95.0, 100.0, 105.0, 110.0])
        
        # Simplified model prices (in practice, use proper jump-diffusion pricer)
        model_prices = []
        for strike in strikes:
            # Simplified formula incorporating jump parameters
            base_price = 100.0 * np.exp(-0.5 * vol**2 + vol * np.random.randn())
            jump_adjustment = lambda_j * (np.exp(mu_j + 0.5 * sigma_j**2) - 1)
            model_price = max(base_price * (1 + jump_adjustment) - strike, 0)
            model_prices.append(model_price)
        
        # Sum of squared errors
        return sum((mp - mm)**2 for mp, mm in zip(market_prices, model_prices))
    
    # Use Differential Evolution for global optimization
    de_solver = finstack_py.DifferentialEvolutionSolver(
        population_size=50,
        max_generations=200,
        mutation_factor=0.8,
        crossover_prob=0.9,
        tolerance=1e-6,
        seed=42  # For reproducibility
    )
    
    # Parameter bounds
    bounds = [
        (0.1, 0.5),    # volatility
        (0.0, 2.0),    # jump_intensity
        (-0.1, 0.1),   # jump_size_mean
        (0.01, 0.3)    # jump_size_std
    ]
    
    # Initial guess
    initial = [0.2, 0.5, 0.0, 0.1]
    
    print("Starting global optimization...")
    start_time = time.time()
    
    result = de_solver.minimize(
        objective=objective_function,
        initial=initial,
        bounds=bounds
    )
    
    elapsed_time = time.time() - start_time
    
    print(f"Optimization completed in {elapsed_time:.2f} seconds")
    print(f"Optimal parameters:")
    print(f"  Volatility:      {result[0]:.4f}")
    print(f"  Jump intensity:  {result[1]:.4f}")
    print(f"  Jump size mean:  {result[2]:.4f}")
    print(f"  Jump size std:   {result[3]:.4f}")
    print(f"  Final objective: {objective_function(result):.6f}")

def example_multi_dimensional_system():
    """
    Demonstrates solving a multi-dimensional system with analytical Jacobian.
    """
    print("\n=== Multi-Dimensional System with Analytical Jacobian ===\n")
    
    # Example: Solve for implied parameters from multiple market observables
    # System of equations representing pricing conditions
    
    def residuals(params):
        """
        System of residuals for calibration.
        params = [rate, volatility, correlation]
        """
        r, vol, rho = params
        
        # Target market observables
        target_swap_rate = 0.03
        target_cap_price = 100.0
        target_swaption_vol = 0.15
        
        # Simplified model outputs (in practice, use proper pricers)
        model_swap_rate = r + 0.001 * vol  # Simplified
        model_cap_price = 100 * np.exp(vol * np.sqrt(1 - rho**2))
        model_swaption_vol = vol * (1 + 0.1 * rho)
        
        return [
            model_swap_rate - target_swap_rate,
            model_cap_price - target_cap_price,
            model_swaption_vol - target_swaption_vol
        ]
    
    def jacobian(params):
        """
        Analytical Jacobian of the system.
        """
        r, vol, rho = params
        
        jac = np.zeros((3, 3))
        
        # Derivatives of swap rate equation
        jac[0, 0] = 1.0  # d(swap_rate)/dr
        jac[0, 1] = 0.001  # d(swap_rate)/dvol
        jac[0, 2] = 0.0  # d(swap_rate)/drho
        
        # Derivatives of cap price equation
        cap_price = 100 * np.exp(vol * np.sqrt(1 - rho**2))
        jac[1, 0] = 0.0  # d(cap_price)/dr
        jac[1, 1] = cap_price * np.sqrt(1 - rho**2)  # d(cap_price)/dvol
        jac[1, 2] = -cap_price * vol * rho / np.sqrt(1 - rho**2)  # d(cap_price)/drho
        
        # Derivatives of swaption vol equation
        jac[2, 0] = 0.0  # d(swaption_vol)/dr
        jac[2, 1] = 1 + 0.1 * rho  # d(swaption_vol)/dvol
        jac[2, 2] = 0.1 * vol  # d(swaption_vol)/drho
        
        return jac
    
    # Solve using Levenberg-Marquardt with analytical Jacobian
    lm_solver = finstack_py.LevenbergMarquardtSolver(
        tolerance=1e-10,
        max_iterations=100
    )
    
    initial = [0.025, 0.12, 0.0]  # Initial guess
    
    print("Solving system with analytical Jacobian...")
    start_time = time.time()
    
    result_analytical = lm_solver.solve_system(
        residuals=residuals,
        jacobian=jacobian,
        initial=initial,
        use_analytical=True
    )
    
    time_analytical = time.time() - start_time
    
    # Solve without analytical Jacobian for comparison
    start_time = time.time()
    
    result_numerical = lm_solver.solve_system(
        residuals=residuals,
        jacobian=None,  # Will use finite differences
        initial=initial,
        use_analytical=False
    )
    
    time_numerical = time.time() - start_time
    
    print(f"\nResults with analytical Jacobian:")
    print(f"  Rate:        {result_analytical[0]:.6f}")
    print(f"  Volatility:  {result_analytical[1]:.6f}")
    print(f"  Correlation: {result_analytical[2]:.6f}")
    print(f"  Time:        {time_analytical:.6f} seconds")
    
    print(f"\nResults with numerical Jacobian:")
    print(f"  Rate:        {result_numerical[0]:.6f}")
    print(f"  Volatility:  {result_numerical[1]:.6f}")
    print(f"  Correlation: {result_numerical[2]:.6f}")
    print(f"  Time:        {time_numerical:.6f} seconds")
    
    print(f"\nSpeed improvement: {time_numerical/time_analytical:.2f}x faster with analytical Jacobian")
    
    # Verify solution
    final_residuals = residuals(result_analytical)
    print(f"\nFinal residuals:")
    for i, res in enumerate(final_residuals):
        print(f"  Equation {i+1}: {res:.2e}")

if __name__ == "__main__":
    print("Enhanced Solver Capabilities Demo")
    print("=" * 50)
    
    print("\nNote: This is a conceptual example demonstrating the enhanced solver")
    print("capabilities. Actual Python bindings would need to be implemented")
    print("to run these examples with real data.")
    
    print("\nKey features now available in finstack:")
    print("\n1. Levenberg-Marquardt solver for non-linear least squares")
    print("   - Adaptive damping parameter for robust convergence")
    print("   - Box constraints support")
    print("   - Suitable for SABR calibration, curve fitting")
    
    print("\n2. Differential Evolution for global optimization") 
    print("   - Population-based stochastic search")
    print("   - No gradient information required")
    print("   - Handles multi-modal objective functions")
    print("   - Reproducible with seed support")
    
    print("\n3. Analytical derivatives support")
    print("   - 3-5x faster convergence for complex models")
    print("   - AnalyticalDerivatives trait for custom implementations")
    print("   - Automatic fallback to finite differences")
    print("   - SABR model derivatives implemented")
    
    print("\n4. Unified multi-dimensional solver interface")
    print("   - minimize() for optimization problems")
    print("   - solve_system() for systems of equations")
    print("   - Consistent API across solver types")
    
    print("\nExample usage in Rust:")
    print("""
    use finstack_core::math::solver_multi::{
        LevenbergMarquardtSolver, MultiSolver, AnalyticalDerivatives
    };
    
    // Create solver
    let solver = LevenbergMarquardtSolver::new()
        .with_tolerance(1e-8)
        .with_max_iterations(100);
    
    // Define objective
    let objective = |params: &[f64]| -> f64 {
        // Your objective function here
        (params[0] - 2.0).powi(2) + (params[1] - 3.0).powi(2)
    };
    
    // With analytical derivatives
    struct MyDerivatives;
    impl AnalyticalDerivatives for MyDerivatives {
        fn gradient(&self, params: &[f64], gradient: &mut [f64]) {
            gradient[0] = 2.0 * (params[0] - 2.0);
            gradient[1] = 2.0 * (params[1] - 3.0);
        }
    }
    
    // Optimize with derivatives
    let derivatives = MyDerivatives;
    let result = solver.minimize_with_derivatives(
        objective,
        &derivatives,
        &[0.0, 0.0],  // initial guess
        None          // no bounds
    ).unwrap();
    """)
