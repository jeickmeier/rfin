#!/usr/bin/env python3
"""
Enhanced SABR Model Example

This example demonstrates the enhanced SABR model implementation with:
1. Standard Hagan et al. approximation formulas with numerical stability
2. Robust near-the-money (ATM) case handling
3. Shifted SABR for negative interest rate environments
4. Free-boundary SABR for cross-zero scenarios

The enhanced implementation addresses the key requirements:
- Proper Hagan et al. formulas with improved numerical stability
- ATM case robustness to prevent numerical instabilities
- Support for negative forward rates through shifting or free-boundary approaches
"""

import numpy as np
import matplotlib.pyplot as plt
from datetime import date, timedelta

# Note: This is a conceptual example. The actual Python bindings would expose
# the enhanced SABR functionality implemented in Rust

def enhanced_sabr_demo():
    """
    Demonstrate enhanced SABR model features for different rate environments
    """
    
    print("=== Enhanced SABR Model Demonstration ===")
    print()
    
    # 1. Standard positive rate environment
    print("1. Standard SABR (Positive Rates)")
    print("-" * 40)
    
    forward_rate = 0.03  # 3%
    strikes = np.array([0.01, 0.02, 0.03, 0.04, 0.05])  # 1% to 5%
    time_to_expiry = 2.0
    
    # Standard SABR parameters
    alpha = 0.2   # Initial volatility
    beta = 0.5    # CEV exponent (0.5 common for rates)
    nu = 0.3      # Vol of vol
    rho = -0.2    # Correlation (negative for rates)
    
    print(f"Forward: {forward_rate:.3%}")
    print(f"SABR params: α={alpha:.3f}, β={beta:.1f}, ν={nu:.3f}, ρ={rho:.3f}")
    print()
    
    # Calculate implied volatilities (conceptual - actual implementation in Rust)
    vols_standard = []
    for strike in strikes:
        # This would call finstack.SABRModel.implied_volatility() in actual Python bindings
        vol = sabr_implied_vol_conceptual(forward_rate, strike, time_to_expiry, alpha, beta, nu, rho)
        vols_standard.append(vol)
    
    print("Strike    | Implied Vol")
    print("----------|------------")
    for i, strike in enumerate(strikes):
        print(f"{strike:.3%}     | {vols_standard[i]:.2%}")
    print()
    
    # 2. Negative rate environment with Shifted SABR
    print("2. Shifted SABR (Negative Rates)")
    print("-" * 40)
    
    forward_negative = -0.005  # -50bps (negative rate environment)
    strikes_negative = np.array([-0.01, -0.005, 0.0, 0.005, 0.01])
    shift = 0.02  # 200bps shift to make all rates positive
    
    print(f"Forward: {forward_negative:.3%} (negative)")
    print(f"Shift: {shift:.3%} (to handle negative rates)")
    print(f"Effective forward: {forward_negative + shift:.3%}")
    print()
    
    # Shifted rates become positive for SABR calculation
    forward_shifted = forward_negative + shift
    strikes_shifted = strikes_negative + shift
    
    vols_shifted = []
    for strike_shifted in strikes_shifted:
        # This would call finstack.SABRModel.implied_volatility() with shifted parameters
        vol = sabr_implied_vol_conceptual(forward_shifted, strike_shifted, time_to_expiry, alpha, beta, nu, rho)
        vols_shifted.append(vol)
    
    print("Original Strike | Shifted Strike | Implied Vol")
    print("---------------|----------------|------------")
    for i, (orig_strike, shifted_strike) in enumerate(zip(strikes_negative, strikes_shifted)):
        print(f"{orig_strike:7.3%}       | {shifted_strike:7.3%}      | {vols_shifted[i]:.2%}")
    print()
    
    # 3. ATM case demonstration
    print("3. Enhanced ATM Stability")
    print("-" * 40)
    
    forward_atm = 0.025  # 2.5%
    # Very close strikes to test ATM stability
    strikes_atm = np.array([
        forward_atm - 1e-6,  # Tiny difference
        forward_atm - 1e-8,  # Even smaller
        forward_atm,         # Exact ATM
        forward_atm + 1e-8,  # Tiny positive
        forward_atm + 1e-6   # Small positive
    ])
    
    print(f"Forward: {forward_atm:.5%}")
    print("Testing numerical stability near ATM...")
    print()
    
    vols_atm = []
    for strike in strikes_atm:
        vol = sabr_implied_vol_conceptual(forward_atm, strike, time_to_expiry, alpha, beta, nu, rho)
        vols_atm.append(vol)
    
    print("Strike Diff  | Implied Vol | ATM Check")
    print("-------------|-------------|----------")
    for i, strike in enumerate(strikes_atm):
        diff = strike - forward_atm
        is_atm = abs(diff) < 1e-8 or abs(diff/forward_atm) < 1e-8
        print(f"{diff:+.2e}     | {vols_atm[i]:.4%}     | {'ATM' if is_atm else 'Off-ATM'}")
    
    vol_range = max(vols_atm) - min(vols_atm)
    print(f"\nVolatility range: {vol_range:.2e} (should be very small)")
    print()
    
    # 4. Free-boundary SABR for cross-zero scenarios
    print("4. Free-Boundary SABR (Cross-Zero)")
    print("-" * 40)
    
    forward_cross = -0.001  # Negative forward
    strikes_cross = np.array([-0.003, -0.001, 0.0, 0.002, 0.004])
    
    print(f"Forward: {forward_cross:.3%} (negative)")
    print("Cross-zero strikes (negative to positive)...")
    print()
    
    # Free-boundary SABR uses absolute values with cross-zero correction
    vols_fb = []
    for strike in strikes_cross:
        # This would call finstack.SABRModel.implied_volatility_free_boundary()
        vol = sabr_free_boundary_conceptual(forward_cross, strike, time_to_expiry, alpha, beta, nu, rho)
        vols_fb.append(vol)
    
    print("Strike    | Abs Strike | Implied Vol | Cross-Zero")
    print("----------|------------|-------------|----------")
    for i, strike in enumerate(strikes_cross):
        abs_strike = abs(strike)
        is_cross = np.sign(forward_cross) != np.sign(strike) if strike != 0 else False
        print(f"{strike:7.3%}   | {abs_strike:7.3%}    | {vols_fb[i]:.2%}       | {'Yes' if is_cross else 'No'}")
    print()

def sabr_implied_vol_conceptual(forward, strike, time_to_expiry, alpha, beta, nu, rho):
    """
    Conceptual SABR implementation for demonstration
    (Real implementation is in Rust with enhanced numerical stability)
    """
    # ATM detection with enhanced stability
    abs_diff = abs(forward - strike)
    rel_diff = abs_diff / max(forward, strike)
    
    if abs_diff < 1e-8 or rel_diff < 1e-8:
        # ATM case with enhanced formula
        if beta == 0.0:
            # Normal SABR
            vol = alpha * (1 + time_to_expiry * (2 - 3 * rho**2) / 24 * nu**2)
        elif beta == 1.0:
            # Lognormal SABR
            alpha_term = alpha**2 / (24 * forward**2)
            rho_term = 0.25 * rho * nu * alpha / forward
            nu_term = (2 - 3 * rho**2) / 24 * nu**2
            vol = alpha / forward * (1 + time_to_expiry * (alpha_term + rho_term + nu_term))
        else:
            # General beta case
            f_beta = forward**beta
            alpha_term = (1 - beta)**2 / 24 * alpha**2 / forward**(2 * (1 - beta))
            rho_term = 0.25 * rho * beta * nu * alpha / f_beta
            nu_term = (2 - 3 * rho**2) / 24 * nu**2
            vol = alpha / f_beta * (1 + time_to_expiry * (alpha_term + rho_term + nu_term))
        return vol
    
    # Off-ATM case with standard Hagan formula
    f_mid = np.sqrt(forward * strike)
    f_mid_beta = f_mid**beta
    
    if beta == 1.0:
        z = (nu / alpha) * np.log(forward / strike)
    elif beta == 0.0:
        z = (nu / alpha) * (forward - strike)
    else:
        z = (nu / alpha) * (forward**(1 - beta) - strike**(1 - beta)) / (1 - beta)
    
    if abs(z) < 1e-8:
        return sabr_implied_vol_conceptual(forward, forward, time_to_expiry, alpha, beta, nu, rho)
    
    # Chi function with enhanced stability
    discriminant = 1 - 2 * rho * z + z**2
    sqrt_disc = np.sqrt(discriminant)
    
    if abs(1 - rho) < 1e-12:
        x = z / (1 + z)
    else:
        x = np.log((sqrt_disc + z - rho) / (1 - rho))
    
    # SABR formula factors
    correction = 1 + (1 - beta)**2 / 24 * np.log(forward / strike)**2
    factor1 = alpha / (f_mid_beta * correction)
    factor2 = z / x
    
    time_correction = ((1 - beta)**2 / 24 * alpha**2 / f_mid**(2 * (1 - beta))
                      + 0.25 * rho * beta * nu * alpha / f_mid_beta
                      + (2 - 3 * rho**2) / 24 * nu**2)
    factor3 = 1 + time_to_expiry * time_correction
    
    return factor1 * factor2 * factor3

def sabr_free_boundary_conceptual(forward, strike, time_to_expiry, alpha, beta, nu, rho):
    """
    Conceptual Free-boundary SABR for cross-zero scenarios
    """
    abs_forward = abs(forward)
    abs_strike = abs(strike)
    
    # Use standard SABR with absolute values
    vol = sabr_implied_vol_conceptual(abs_forward, abs_strike, time_to_expiry, alpha, beta, nu, rho)
    
    # Apply cross-zero correction if needed
    if np.sign(forward) != np.sign(strike) and strike != 0:
        cross_correction = 1 + 0.1 * abs(forward - strike) / (abs_forward + abs_strike)
        vol *= cross_correction
    
    return vol

if __name__ == "__main__":
    enhanced_sabr_demo()
