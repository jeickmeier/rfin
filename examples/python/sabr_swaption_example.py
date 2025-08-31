#!/usr/bin/env python3
"""
Example demonstrating SABR model for swaption pricing.

This example shows:
1. Creating swaptions with different strikes
2. Using SABR model for volatility smile
3. Calibrating SABR parameters to market data
4. Comparing Black vs SABR pricing
"""

import numpy as np
import matplotlib.pyplot as plt
from datetime import date, timedelta

# Note: This is a conceptual example. The actual Python bindings for SABR
# would need to be implemented in finstack-py


def sabr_implied_vol(forward, strike, time_to_expiry, alpha, beta, nu, rho):
    """
    Calculate SABR implied volatility using Hagan's approximation.

    Parameters:
    - forward: Forward rate
    - strike: Strike rate
    - time_to_expiry: Time to expiry in years
    - alpha: Initial volatility
    - beta: CEV exponent (0 to 1)
    - nu: Volatility of volatility
    - rho: Correlation between asset and volatility
    """
    if abs(forward - strike) < 1e-12:
        # ATM case
        f_beta = forward**beta
        vol = (
            alpha
            / f_beta
            * (
                1
                + time_to_expiry
                * (
                    (1 - beta) ** 2 / 24 * alpha**2 / forward ** (2 * (1 - beta))
                    + 0.25 * rho * beta * nu * alpha / f_beta
                    + (2 - 3 * rho**2) / 24 * nu**2
                )
            )
        )
        return vol

    # Off-ATM case
    f_mid = np.sqrt(forward * strike)
    f_mid_beta = f_mid**beta

    if beta == 1.0:
        z = (nu / alpha) * np.log(forward / strike)
    else:
        z = (nu / alpha) * (forward ** (1 - beta) - strike ** (1 - beta)) / (1 - beta)

    if abs(z) < 1e-10:
        return sabr_implied_vol(forward, forward, time_to_expiry, alpha, beta, nu, rho)

    # Calculate chi(z)
    sqrt_disc = np.sqrt(1 - 2 * rho * z + z**2)
    if abs(1 - rho) < 1e-12:
        x = z / (1 + z)
    else:
        x = np.log((sqrt_disc + z - rho) / (1 - rho))

    # First factor
    factor1 = alpha / (
        f_mid_beta
        * (
            1
            + (1 - beta) ** 2 / 24 * np.log(forward / strike) ** 2
            + (1 - beta) ** 4 / 1920 * np.log(forward / strike) ** 4
        )
    )

    # Second factor
    factor2 = z / x

    # Third factor (time correction)
    factor3 = 1 + time_to_expiry * (
        (1 - beta) ** 2 / 24 * alpha**2 / f_mid ** (2 * (1 - beta))
        + 0.25 * rho * beta * nu * alpha / f_mid_beta
        + (2 - 3 * rho**2) / 24 * nu**2
    )

    return factor1 * factor2 * factor3


def black_swaption_price(
    forward, strike, annuity, time_to_expiry, volatility, is_payer=True
):
    """
    Price swaption using Black's model.

    Parameters:
    - forward: Forward swap rate
    - strike: Strike rate
    - annuity: Swap annuity factor
    - time_to_expiry: Time to expiry in years
    - volatility: Black volatility
    - is_payer: True for payer swaption, False for receiver
    """
    from scipy.stats import norm

    if time_to_expiry <= 0:
        return 0.0

    variance = volatility**2 * time_to_expiry
    d1 = (np.log(forward / strike) + 0.5 * variance) / np.sqrt(variance)
    d2 = d1 - np.sqrt(variance)

    if is_payer:
        # Payer swaption (call on swap rate)
        price = annuity * (forward * norm.cdf(d1) - strike * norm.cdf(d2))
    else:
        # Receiver swaption (put on swap rate)
        price = annuity * (strike * norm.cdf(-d2) - forward * norm.cdf(-d1))

    return price


def calibrate_sabr_beta_fixed(forward, strikes, market_vols, time_to_expiry, beta=0.5):
    """
    Calibrate SABR parameters with fixed beta.

    This is a simplified calibration - in practice would use optimization.
    """
    # Initial guess
    atm_idx = np.argmin(np.abs(strikes - forward))
    atm_vol = market_vols[atm_idx]

    alpha = atm_vol * forward ** (1 - beta)
    nu = 0.3  # Typical value
    rho = 0.0  # Start with zero correlation

    # Simple gradient descent (in practice use scipy.optimize)
    learning_rate = 0.01
    for _ in range(100):
        # Calculate model vols
        model_vols = [
            sabr_implied_vol(forward, k, time_to_expiry, alpha, beta, nu, rho)
            for k in strikes
        ]

        # Calculate errors
        errors = np.array(model_vols) - np.array(market_vols)
        total_error = np.sum(errors**2)

        if total_error < 1e-6:
            break

        # Update parameters (simplified)
        alpha -= learning_rate * np.sum(errors) * 0.1
        nu -= learning_rate * np.sum(errors * (strikes - forward)) * 0.01
        rho -= learning_rate * np.sum(errors * np.sign(strikes - forward)) * 0.1

        # Keep parameters in valid ranges
        alpha = max(0.001, alpha)
        nu = max(0.0, min(2.0, nu))
        rho = max(-0.99, min(0.99, rho))

    return alpha, nu, rho


def main():
    """Demonstrate SABR model for swaption pricing."""

    # Market data
    forward_rate = 0.03  # 3% forward swap rate
    time_to_expiry = 2.0  # 2 years to expiry
    annuity = 4.5  # Swap annuity factor
    notional = 10_000_000  # $10M notional

    # Strike grid
    strikes = np.linspace(0.015, 0.045, 11)  # 1.5% to 4.5%

    # SABR parameters (typically calibrated to market)
    alpha = 0.01
    beta = 0.5  # Common choice for rates
    nu = 0.3
    rho = -0.2  # Negative correlation typical for rates

    # Calculate SABR implied volatilities
    sabr_vols = [
        sabr_implied_vol(forward_rate, k, time_to_expiry, alpha, beta, nu, rho)
        for k in strikes
    ]

    # For comparison, flat volatility (no smile)
    flat_vol = 0.20  # 20% flat vol
    flat_vols = [flat_vol] * len(strikes)

    # Price swaptions at different strikes
    print("Swaption Prices (Payer, $10M notional):")
    print("Strike | Black Price | SABR Price | Difference")
    print("-" * 50)

    for i, strike in enumerate(strikes):
        black_price = (
            black_swaption_price(
                forward_rate, strike, annuity, time_to_expiry, flat_vol
            )
            * notional
        )

        sabr_price = (
            black_swaption_price(
                forward_rate, strike, annuity, time_to_expiry, sabr_vols[i]
            )
            * notional
        )

        diff = sabr_price - black_price
        print(f"{strike:.1%} | ${black_price:,.0f} | ${sabr_price:,.0f} | ${diff:,.0f}")

    # Plot volatility smile
    plt.figure(figsize=(12, 5))

    # Volatility smile
    plt.subplot(1, 2, 1)
    plt.plot(strikes * 100, np.array(sabr_vols) * 100, "b-", label="SABR", linewidth=2)
    plt.plot(
        strikes * 100,
        np.array(flat_vols) * 100,
        "r--",
        label="Flat (Black)",
        linewidth=2,
    )
    plt.axvline(forward_rate * 100, color="gray", linestyle=":", alpha=0.5, label="ATM")
    plt.xlabel("Strike (%)")
    plt.ylabel("Implied Volatility (%)")
    plt.title("SABR Volatility Smile")
    plt.legend()
    plt.grid(True, alpha=0.3)

    # Price comparison
    plt.subplot(1, 2, 2)
    black_prices = [
        black_swaption_price(forward_rate, k, annuity, time_to_expiry, flat_vol)
        * notional
        for k in strikes
    ]
    sabr_prices = [
        black_swaption_price(forward_rate, k, annuity, time_to_expiry, v) * notional
        for k, v in zip(strikes, sabr_vols)
    ]

    plt.plot(
        strikes * 100, np.array(black_prices) / 1000, "r--", label="Black", linewidth=2
    )
    plt.plot(
        strikes * 100, np.array(sabr_prices) / 1000, "b-", label="SABR", linewidth=2
    )
    plt.axvline(forward_rate * 100, color="gray", linestyle=":", alpha=0.5, label="ATM")
    plt.xlabel("Strike (%)")
    plt.ylabel("Swaption Price ($000s)")
    plt.title("Swaption Prices: Black vs SABR")
    plt.legend()
    plt.grid(True, alpha=0.3)

    plt.tight_layout()
    plt.savefig("sabr_swaption_smile.png", dpi=150)
    plt.show()

    # Demonstrate calibration
    print("\n" + "=" * 50)
    print("SABR Calibration Example")
    print("=" * 50)

    # Synthetic market data (from SABR model)
    market_strikes = np.array([0.02, 0.025, 0.03, 0.035, 0.04])
    true_params = (alpha, nu, rho)
    market_vols = [
        sabr_implied_vol(forward_rate, k, time_to_expiry, alpha, beta, nu, rho)
        for k in market_strikes
    ]

    # Add some noise to simulate market quotes
    market_vols = np.array(market_vols) + np.random.normal(0, 0.005, len(market_vols))

    # Calibrate
    cal_alpha, cal_nu, cal_rho = calibrate_sabr_beta_fixed(
        forward_rate, market_strikes, market_vols, time_to_expiry, beta
    )

    print(f"True parameters:       α={alpha:.4f}, ν={nu:.4f}, ρ={rho:.4f}")
    print(f"Calibrated parameters: α={cal_alpha:.4f}, ν={cal_nu:.4f}, ρ={cal_rho:.4f}")

    # Compare calibrated vs market vols
    cal_vols = [
        sabr_implied_vol(
            forward_rate, k, time_to_expiry, cal_alpha, beta, cal_nu, cal_rho
        )
        for k in market_strikes
    ]

    print("\nCalibration fit:")
    print("Strike | Market Vol | Model Vol | Error (bps)")
    print("-" * 45)
    for k, mv, cv in zip(market_strikes, market_vols, cal_vols):
        error_bps = (cv - mv) * 10000
        print(f"{k:.1%} | {mv:.2%} | {cv:.2%} | {error_bps:+.1f}")

    # Calculate smile metrics
    print("\n" + "=" * 50)
    print("Smile Metrics at ATM")
    print("=" * 50)

    # Skew (first derivative)
    bump = 0.0001
    vol_up = sabr_implied_vol(
        forward_rate, forward_rate + bump, time_to_expiry, alpha, beta, nu, rho
    )
    vol_down = sabr_implied_vol(
        forward_rate, forward_rate - bump, time_to_expiry, alpha, beta, nu, rho
    )
    skew = (vol_up - vol_down) / (2 * bump)

    # Curvature (second derivative)
    vol_center = sabr_implied_vol(
        forward_rate, forward_rate, time_to_expiry, alpha, beta, nu, rho
    )
    curvature = (vol_up - 2 * vol_center + vol_down) / bump**2

    print(f"ATM Volatility: {vol_center:.2%}")
    print(f"Skew (∂σ/∂K):   {skew:.4f}")
    print(f"Curvature:      {curvature:.4f}")

    # Risk reversal and butterfly
    k_25d_put = forward_rate * 0.9  # Approximate 25-delta put
    k_25d_call = forward_rate * 1.1  # Approximate 25-delta call

    vol_25d_put = sabr_implied_vol(
        forward_rate, k_25d_put, time_to_expiry, alpha, beta, nu, rho
    )
    vol_25d_call = sabr_implied_vol(
        forward_rate, k_25d_call, time_to_expiry, alpha, beta, nu, rho
    )

    risk_reversal = vol_25d_call - vol_25d_put
    butterfly = 0.5 * (vol_25d_call + vol_25d_put) - vol_center

    print(f"\n25-Delta Risk Reversal: {risk_reversal*100:.2f}%")
    print(f"25-Delta Butterfly:      {butterfly*100:.2f}%")


if __name__ == "__main__":
    main()
