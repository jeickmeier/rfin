# Implementation Plan for Critical Missing Components

## Overview
This document provides detailed implementation plans for the 5 critical components identified as missing or incomplete when comparing Finstack to FinancePy's methodologies.

---

## 3. American Option Pricing (Binomial Trees)

### Objective
Implement American option pricing using the Leisen-Reimer binomial tree model, which provides better convergence properties than standard Cox-Ross-Rubinstein (CRR).

### Mathematical Foundation
The Leisen-Reimer model uses:
- Peizer-Pratt inversion for probability calculations
- Better convergence to Black-Scholes for European options
- Optimal exercise boundary detection for American options

### Implementation Steps

#### Step 1: Create Binomial Tree Module
**File**: `finstack/valuations/src/instruments/options/binomial_tree.rs`

```rust
use finstack_core::{F, Result};
use crate::instruments::options::{OptionType, ExerciseStyle};

/// Binomial tree types
#[derive(Clone, Copy, Debug)]
pub enum TreeType {
    /// Cox-Ross-Rubinstein
    CRR,
    /// Jarrow-Rudd
    JR,
    /// Leisen-Reimer (most accurate)
    LeisenReimer,
    /// Tian
    Tian,
}

/// Binomial tree for option pricing
pub struct BinomialTree {
    /// Number of time steps
    pub steps: usize,
    /// Tree type
    pub tree_type: TreeType,
    /// Cache tree nodes for efficiency
    pub use_cache: bool,
}

impl BinomialTree {
    /// Create new binomial tree
    pub fn new(steps: usize, tree_type: TreeType) -> Self {
        Self {
            steps,
            tree_type,
            use_cache: true,
        }
    }
    
    /// Peizer-Pratt inversion function (for Leisen-Reimer)
    fn peizer_pratt_inversion(&self, z: F, n: usize) -> F {
        let n_f = n as f64;
        if z.abs() < 1e-10 {
            return 0.5;
        }
        
        let sign = if z > 0.0 { 1.0 } else { -1.0 };
        let z = z.abs();
        
        let a = z / (n_f + 1.0/3.0 + 0.1/(n_f + 1.0)).sqrt();
        let b = 1.0 + a * a * (1.0/4.0 + a * a * (3.0/28.0 + a * a * (23.0/240.0)));
        
        0.5 + sign * 0.5 * (1.0 - (-a * a * b).exp()).sqrt()
    }
    
    /// Calculate tree parameters based on type
    fn calculate_parameters(
        &self,
        spot: F,
        strike: F,
        r: F,
        sigma: F,
        t: F,
        q: F,
    ) -> (F, F, F) {
        let dt = t / self.steps as f64;
        
        match self.tree_type {
            TreeType::LeisenReimer => {
                // Leisen-Reimer parameters
                let d1 = ((spot / strike).ln() + (r - q + 0.5 * sigma * sigma) * t) 
                    / (sigma * t.sqrt());
                let d2 = d1 - sigma * t.sqrt();
                
                let p = self.peizer_pratt_inversion(d2, self.steps);
                let p_star = self.peizer_pratt_inversion(d1, self.steps);
                
                let df = (-r * dt).exp();
                let u = df * p_star / p;
                let d = df * (1.0 - p_star) / (1.0 - p);
                
                (u, d, p)
            },
            TreeType::CRR => {
                // Cox-Ross-Rubinstein parameters
                let u = (sigma * dt.sqrt()).exp();
                let d = 1.0 / u;
                let p = (((r - q) * dt).exp() - d) / (u - d);
                
                (u, d, p)
            },
            _ => {
                // Default to CRR for now
                let u = (sigma * dt.sqrt()).exp();
                let d = 1.0 / u;
                let p = (((r - q) * dt).exp() - d) / (u - d);
                
                (u, d, p)
            }
        }
    }
    
    /// Price American option using binomial tree
    pub fn price_american(
        &self,
        spot: F,
        strike: F,
        r: F,
        sigma: F,
        t: F,
        q: F,
        option_type: OptionType,
    ) -> Result<F> {
        let (u, d, p) = self.calculate_parameters(spot, strike, r, sigma, t, q);
        let dt = t / self.steps as f64;
        let df = (-r * dt).exp();
        
        // Build terminal values
        let mut values = Vec::with_capacity(self.steps + 1);
        for i in 0..=self.steps {
            let spot_t = spot * u.powi(i as i32) * d.powi((self.steps - i) as i32);
            let payoff = match option_type {
                OptionType::Call => (spot_t - strike).max(0.0),
                OptionType::Put => (strike - spot_t).max(0.0),
            };
            values.push(payoff);
        }
        
        // Backward induction with early exercise
        for step in (0..self.steps).rev() {
            for i in 0..=step {
                // Continuation value
                let continuation = df * (p * values[i + 1] + (1.0 - p) * values[i]);
                
                // Exercise value
                let spot_t = spot * u.powi(i as i32) * d.powi((step - i) as i32);
                let exercise = match option_type {
                    OptionType::Call => (spot_t - strike).max(0.0),
                    OptionType::Put => (strike - spot_t).max(0.0),
                };
                
                // American option: max of continuation and exercise
                values[i] = continuation.max(exercise);
            }
            values.pop(); // Remove last element as tree shrinks
        }
        
        Ok(values[0])
    }
}
```

#### Step 2: Integrate with Equity Option
**File**: Update `finstack/valuations/src/instruments/options/equity_option/mod.rs`

```rust
impl EquityOption {
    /// Price option based on exercise style
    pub fn price(
        &self,
        spot: F,
        r: F,
        sigma: F,
        t: F,
        q: F,
    ) -> Result<Money> {
        let price = match self.exercise_style {
            ExerciseStyle::European => {
                self.black_scholes_price(spot, r, sigma, t, q)?
            },
            ExerciseStyle::American => {
                // Use binomial tree for American options
                let tree = BinomialTree::new(100, TreeType::LeisenReimer);
                let price = tree.price_american(
                    spot,
                    self.strike.amount(),
                    r,
                    sigma,
                    t,
                    q,
                    self.option_type,
                )?;
                Money::new(price * self.contract_size, self.strike.currency())
            },
            ExerciseStyle::Bermudan => {
                return Err(Error::Internal); // Not yet implemented
            }
        };
        
        Ok(price)
    }
}
```

---

## 5. Implied Volatility Solver

### Objective
Implement a robust implied volatility solver using Brenner-Subrahmanyam initial guess and Halley's method for fast convergence.

### Implementation Steps

#### Step 1: Create Implied Volatility Module
**File**: `finstack/valuations/src/instruments/options/implied_vol.rs`

```rust
use finstack_core::{F, Result, Error};
use crate::instruments::options::{OptionType, black_scholes_common};
use std::f64::consts::PI;

/// Configuration for implied volatility solver
#[derive(Clone, Debug)]
pub struct ImpliedVolConfig {
    /// Tolerance for convergence
    pub tolerance: F,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Minimum volatility bound
    pub min_vol: F,
    /// Maximum volatility bound
    pub max_vol: F,
    /// Use Halley's method (vs Newton)
    pub use_halley: bool,
}

impl Default for ImpliedVolConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-10,
            max_iterations: 20,
            min_vol: 0.001,  // 0.1%
            max_vol: 5.0,     // 500%
            use_halley: true, // Halley converges faster for implied vol
        }
    }
}

/// Implied volatility solver
pub struct ImpliedVolSolver {
    config: ImpliedVolConfig,
}

impl ImpliedVolSolver {
    /// Create new solver with default config
    pub fn new() -> Self {
        Self {
            config: ImpliedVolConfig::default(),
        }
    }
    
    /// Create solver with custom config
    pub fn with_config(config: ImpliedVolConfig) -> Self {
        Self { config }
    }
    
    /// Brenner-Subrahmanyam initial volatility guess
    /// This approximation works well for near-ATM options
    fn initial_guess(
        &self,
        market_price: F,
        spot: F,
        strike: F,
        r: F,
        t: F,
        q: F,
        option_type: OptionType,
    ) -> F {
        // Calculate intrinsic value
        let intrinsic = match option_type {
            OptionType::Call => (spot * (-(q * t)).exp() - strike * (-(r * t)).exp()).max(0.0),
            OptionType::Put => (strike * (-(r * t)).exp() - spot * (-(q * t)).exp()).max(0.0),
        };
        
        // Time value
        let time_value = (market_price - intrinsic).max(0.01);
        
        // Brenner-Subrahmanyam approximation
        let forward = spot * ((r - q) * t).exp();
        (2.0 * PI / t).sqrt() * time_value / forward
    }
    
    /// Calculate option price, vega, and volga for given volatility
    fn price_vega_volga(
        &self,
        vol: F,
        spot: F,
        strike: F,
        r: F,
        t: F,
        q: F,
        option_type: OptionType,
    ) -> (F, F, F) {
        if vol <= 0.0 || t <= 0.0 {
            return (0.0, 0.0, 0.0);
        }
        
        let sqrt_t = t.sqrt();
        let d1 = ((spot / strike).ln() + (r - q + 0.5 * vol * vol) * t) / (vol * sqrt_t);
        let d2 = d1 - vol * sqrt_t;
        
        let nd1 = black_scholes_common::norm_cdf(d1);
        let nd2 = black_scholes_common::norm_cdf(d2);
        let npd1 = black_scholes_common::norm_pdf(d1);
        
        let exp_qt = (-q * t).exp();
        let exp_rt = (-r * t).exp();
        
        // Price
        let price = match option_type {
            OptionType::Call => spot * exp_qt * nd1 - strike * exp_rt * nd2,
            OptionType::Put => strike * exp_rt * (1.0 - nd2) - spot * exp_qt * (1.0 - nd1),
        };
        
        // Vega
        let vega = spot * exp_qt * npd1 * sqrt_t;
        
        // Volga (dvega/dvol)
        let volga = vega * d1 * d2 / vol;
        
        (price, vega, volga)
    }
    
    /// Solve for implied volatility given market price
    pub fn solve(
        &self,
        market_price: F,
        spot: F,
        strike: F,
        r: F,
        t: F,
        q: F,
        option_type: OptionType,
    ) -> Result<F> {
        // Handle edge cases
        if t <= 0.0 {
            return Err(Error::Internal);
        }
        
        // Check if price is below intrinsic value
        let intrinsic = match option_type {
            OptionType::Call => {
                (spot * (-(q * t)).exp() - strike * (-(r * t)).exp()).max(0.0)
            },
            OptionType::Put => {
                (strike * (-(r * t)).exp() - spot * (-(q * t)).exp()).max(0.0)
            }
        };
        
        if market_price < intrinsic {
            return Err(Error::Internal); // No solution exists
        }
        
        // Initial guess
        let mut vol = self.initial_guess(
            market_price,
            spot,
            strike,
            r,
            t,
            q,
            option_type,
        );
        
        // Clamp initial guess to bounds
        vol = vol.clamp(self.config.min_vol, self.config.max_vol);
        
        // Iteration
        for i in 0..self.config.max_iterations {
            let (price, vega, volga) = self.price_vega_volga(
                vol,
                spot,
                strike,
                r,
                t,
                q,
                option_type,
            );
            
            let diff = price - market_price;
            
            // Check convergence
            if diff.abs() < self.config.tolerance {
                return Ok(vol);
            }
            
            // Check if vega is too small
            if vega.abs() < 1e-10 {
                // Try bisection as fallback
                return self.bisection_fallback(
                    market_price,
                    spot,
                    strike,
                    r,
                    t,
                    q,
                    option_type,
                );
            }
            
            // Update volatility
            if self.config.use_halley {
                // Halley's method (third-order convergence)
                let h = diff / vega;
                vol -= h * (1.0 + 0.5 * h * volga / vega);
            } else {
                // Newton's method (second-order convergence)
                vol -= diff / vega;
            }
            
            // Clamp to bounds
            vol = vol.clamp(self.config.min_vol, self.config.max_vol);
        }
        
        Err(Error::Internal) // Failed to converge
    }
    
    /// Bisection method as fallback
    fn bisection_fallback(
        &self,
        market_price: F,
        spot: F,
        strike: F,
        r: F,
        t: F,
        q: F,
        option_type: OptionType,
    ) -> Result<F> {
        let mut low = self.config.min_vol;
        let mut high = self.config.max_vol;
        
        for _ in 0..50 {
            let mid = (low + high) / 2.0;
            let (price, _, _) = self.price_vega_volga(
                mid,
                spot,
                strike,
                r,
                t,
                q,
                option_type,
            );
            
            if (price - market_price).abs() < self.config.tolerance {
                return Ok(mid);
            }
            
            if price < market_price {
                low = mid;
            } else {
                high = mid;
            }
        }
        
        Err(Error::Internal)
    }
}
```

---

## 6. Enhanced CDS Pricing with Accrual-on-Default

### Objective
Enhance CDS pricing to include accrual-on-default and use finer discretization for more accurate valuation.

### Implementation Steps

#### Step 1: Update CDS Module
**File**: Update `finstack/valuations/src/instruments/cds/mod.rs`

```rust
impl CreditDefaultSwap {
    /// Calculate protection leg PV with accrual-on-default
    pub fn pv_protection_leg_enhanced(
        &self,
        disc: &dyn Discount,
        credit: &CreditCurve,
    ) -> Result<Money> {
        let start = self.premium.start;
        let end = self.premium.end;
        let lgd = 1.0 - self.protection.recovery_rate;
        
        // Use finer discretization (FinancePy default: 500 steps)
        let num_steps = 500;
        let dt = (end - start).whole_days() as f64 / 365.25 / num_steps as f64;
        
        let mut pv = 0.0;
        
        // Simpson's rule for integration
        for i in 0..num_steps {
            let t = i as f64 * dt;
            let t_next = (i + 1) as f64 * dt;
            let t_mid = (t + t_next) / 2.0;
            
            // Function values at three points
            let f_t = self.protection_integrand(t, disc, credit, lgd);
            let f_mid = self.protection_integrand(t_mid, disc, credit, lgd);
            let f_next = self.protection_integrand(t_next, disc, credit, lgd);
            
            // Simpson's rule: (dt/6) * (f(t) + 4*f(t_mid) + f(t_next))
            pv += (dt / 6.0) * (f_t + 4.0 * f_mid + f_next);
        }
        
        // Add accrual on default
        let accrual = self.calculate_accrual_on_default(disc, credit)?;
        
        Ok(Money::new(
            pv * self.notional.amount() + accrual.amount(),
            self.notional.currency(),
        ))
    }
    
    /// Protection leg integrand
    fn protection_integrand(
        &self,
        t: F,
        disc: &dyn Discount,
        credit: &CreditCurve,
        lgd: F,
    ) -> F {
        let sp = credit.survival_probability(self.premium.start.add_years(t));
        let df = disc.df(t);
        let hazard = credit.hazard_rate(t);
        
        lgd * hazard * sp * df
    }
    
    /// Calculate accrual on default
    pub fn calculate_accrual_on_default(
        &self,
        disc: &dyn Discount,
        credit: &CreditCurve,
    ) -> Result<Money> {
        let schedule = self.build_premium_schedule(&CurveSet::new(), Date::today())?;
        let mut accrual = 0.0;
        
        for i in 0..schedule.len() - 1 {
            let (start_date, _) = schedule[i];
            let (end_date, coupon) = schedule[i + 1];
            
            // Time to period start and end
            let t_start = self.year_fraction(self.premium.start, start_date);
            let t_end = self.year_fraction(self.premium.start, end_date);
            
            // Average survival probability and discount factor
            let sp_avg = (credit.survival_probability(start_date) 
                + credit.survival_probability(end_date)) / 2.0;
            let df_avg = (disc.df(t_start) + disc.df(t_end)) / 2.0;
            
            // Default probability in period
            let default_prob = credit.survival_probability(start_date) 
                - credit.survival_probability(end_date);
            
            // Accrual factor (assuming default happens mid-period)
            let accrual_factor = 0.5;
            
            accrual += coupon.amount() * default_prob * df_avg * accrual_factor;
        }
        
        Ok(Money::new(accrual, self.notional.currency()))
    }
    
    /// Enhanced par spread calculation
    pub fn par_spread_enhanced(
        &self,
        disc: &dyn Discount,
        credit: &CreditCurve,
    ) -> Result<F> {
        let protection_pv = self.pv_protection_leg_enhanced(disc, credit)?;
        let risky_annuity = self.risky_annuity_enhanced(disc, credit)?;
        
        // Par spread = Protection PV / Risky Annuity
        Ok(protection_pv.amount() / risky_annuity * 10000.0) // Convert to basis points
    }
    
    /// Enhanced risky annuity calculation
    pub fn risky_annuity_enhanced(
        &self,
        disc: &dyn Discount,
        credit: &CreditCurve,
    ) -> Result<F> {
        let schedule = self.build_premium_schedule(&CurveSet::new(), Date::today())?;
        let mut annuity = 0.0;
        
        for i in 0..schedule.len() - 1 {
            let (start_date, _) = schedule[i];
            let (end_date, _) = schedule[i + 1];
            
            let t_start = self.year_fraction(self.premium.start, start_date);
            let t_end = self.year_fraction(self.premium.start, end_date);
            let dcf = self.premium.dc.year_fraction(start_date, end_date)?;
            
            // Survival probability at payment date
            let sp = credit.survival_probability(end_date);
            let df = disc.df(t_end);
            
            annuity += dcf * sp * df;
            
            // Add accrual component
            let default_prob = credit.survival_probability(start_date) - sp;
            let df_mid = disc.df((t_start + t_end) / 2.0);
            annuity += 0.5 * dcf * default_prob * df_mid;
        }
        
        Ok(annuity * self.notional.amount())
    }
    
    fn year_fraction(&self, from: Date, to: Date) -> F {
        self.premium.dc.year_fraction(from, to).unwrap_or(0.0)
    }
}
```

---

## 7. SABR Volatility Model

### Objective
Implement the SABR (Stochastic Alpha, Beta, Rho) model for accurate volatility smile modeling in interest rate and FX options.

### Implementation Steps

#### Step 1: Create SABR Module
**File**: `finstack/valuations/src/models/sabr.rs`

```rust
use finstack_core::{F, Result};
use std::f64::consts::PI;

/// SABR model parameters
#[derive(Clone, Debug)]
pub struct SABRParameters {
    /// Initial volatility
    pub alpha: F,
    /// CEV exponent (0 = normal, 1 = lognormal)
    pub beta: F,
    /// Correlation between asset and volatility
    pub rho: F,
    /// Volatility of volatility
    pub nu: F,
}

impl SABRParameters {
    /// Create new SABR parameters
    pub fn new(alpha: F, beta: F, rho: F, nu: F) -> Result<Self> {
        // Validate parameters
        if alpha <= 0.0 {
            return Err(Error::Internal);
        }
        if beta < 0.0 || beta > 1.0 {
            return Err(Error::Internal);
        }
        if rho < -1.0 || rho > 1.0 {
            return Err(Error::Internal);
        }
        if nu < 0.0 {
            return Err(Error::Internal);
        }
        
        Ok(Self { alpha, beta, rho, nu })
    }
}

/// SABR volatility model
pub struct SABRModel {
    params: SABRParameters,
}

impl SABRModel {
    /// Create new SABR model
    pub fn new(params: SABRParameters) -> Self {
        Self { params }
    }
    
    /// Calculate implied volatility using SABR formula
    /// Based on Hagan et al. (2002) "Managing Smile Risk"
    pub fn implied_volatility(
        &self,
        forward: F,
        strike: F,
        expiry: F,
    ) -> F {
        let alpha = self.params.alpha;
        let beta = self.params.beta;
        let rho = self.params.rho;
        let nu = self.params.nu;
        
        // Handle ATM case
        if (strike - forward).abs() < 1e-10 {
            return self.atm_volatility(forward, expiry);
        }
        
        // Calculate intermediate values
        let fk_beta = (forward * strike).powf((1.0 - beta) / 2.0);
        let log_fk = (forward / strike).ln();
        let z = nu / alpha * fk_beta * log_fk;
        
        // Calculate x(z)
        let x = if (1.0 - 2.0 * rho * z + z * z).sqrt() + z - rho > 0.0 {
            ((1.0 - 2.0 * rho * z + z * z).sqrt() + z - rho) / (1.0 - rho)
        } else {
            // Handle numerical issues
            1.0
        }.ln();
        
        // Main SABR formula components
        let factor1 = alpha / fk_beta;
        
        let factor2 = 1.0 + (1.0 - beta).powi(2) / 24.0 * log_fk.powi(2)
            + (1.0 - beta).powi(4) / 1920.0 * log_fk.powi(4);
        
        let factor3 = if x.abs() > 1e-10 {
            z / x
        } else {
            1.0 // Limit as x -> 0
        };
        
        let factor4 = 1.0 + expiry * (
            (1.0 - beta).powi(2) * alpha.powi(2) / (24.0 * fk_beta.powi(2))
            + 0.25 * rho * beta * nu * alpha / fk_beta
            + (2.0 - 3.0 * rho.powi(2)) * nu.powi(2) / 24.0
        );
        
        factor1 / factor2 * factor3 * factor4
    }
    
    /// ATM volatility approximation
    fn atm_volatility(&self, forward: F, expiry: F) -> F {
        let alpha = self.params.alpha;
        let beta = self.params.beta;
        let rho = self.params.rho;
        let nu = self.params.nu;
        
        let f_beta = forward.powf(1.0 - beta);
        
        let vol_atm = alpha / f_beta;
        
        let correction = 1.0 + expiry * (
            (1.0 - beta).powi(2) * alpha.powi(2) / (24.0 * forward.powf(2.0 - 2.0 * beta))
            + 0.25 * rho * beta * nu * alpha / f_beta
            + (2.0 - 3.0 * rho.powi(2)) * nu.powi(2) / 24.0
        );
        
        vol_atm * correction
    }
    
    /// Calibrate SABR parameters to market volatilities
    pub fn calibrate(
        forward: F,
        expiry: F,
        strikes: &[F],
        market_vols: &[F],
        beta: F,  // Beta is usually fixed
    ) -> Result<SABRParameters> {
        use finstack_core::math::optimization::LevenbergMarquardt;
        
        // Initial guess for parameters
        let atm_vol = market_vols[strikes.len() / 2];
        let initial_alpha = atm_vol * forward.powf(1.0 - beta);
        let initial_rho = 0.0;
        let initial_nu = 0.3;
        
        // Objective function: sum of squared errors
        let objective = |params: &[F]| -> F {
            let alpha = params[0];
            let rho = params[1];
            let nu = params[2];
            
            let test_params = SABRParameters::new(alpha, beta, rho, nu);
            if test_params.is_err() {
                return 1e10; // Penalty for invalid parameters
            }
            
            let model = SABRModel::new(test_params.unwrap());
            
            let mut error = 0.0;
            for (i, &strike) in strikes.iter().enumerate() {
                let model_vol = model.implied_volatility(forward, strike, expiry);
                let diff = model_vol - market_vols[i];
                error += diff * diff;
            }
            
            error
        };
        
        // Optimize using Levenberg-Marquardt
        let optimizer = LevenbergMarquardt::new();
        let result = optimizer.minimize(
            objective,
            &[initial_alpha, initial_rho, initial_nu],
            &[0.001, -0.999, 0.001],  // Lower bounds
            &[10.0, 0.999, 2.0],       // Upper bounds
        )?;
        
        SABRParameters::new(result[0], beta, result[1], result[2])
    }
}

/// SABR volatility cube for swaptions
pub struct SABRVolatilityCube {
    /// Expiry tenors
    pub expiries: Vec<F>,
    /// Swap tenors
    pub swap_tenors: Vec<F>,
    /// SABR parameters for each expiry-tenor pair
    pub parameters: Vec<Vec<SABRParameters>>,
}

impl SABRVolatilityCube {
    /// Get implied volatility for given expiry, tenor, and strike
    pub fn implied_vol(
        &self,
        expiry: F,
        swap_tenor: F,
        forward: F,
        strike: F,
    ) -> Result<F> {
        // Find appropriate parameters using bilinear interpolation
        let params = self.interpolate_parameters(expiry, swap_tenor)?;
        let model = SABRModel::new(params);
        Ok(model.implied_volatility(forward, strike, expiry))
    }
    
    fn interpolate_parameters(
        &self,
        expiry: F,
        swap_tenor: F,
    ) -> Result<SABRParameters> {
        // Bilinear interpolation of SABR parameters
        // ... implementation details
        todo!()
    }
}
```

---

## 8. Enhanced Greeks Calculation

### Objective
Improve Greeks calculation using adaptive finite differences and Richardson extrapolation for higher accuracy.

### Implementation Steps

#### Step 1: Create Enhanced Greeks Module
**File**: `finstack/valuations/src/instruments/options/greeks_enhanced.rs`

```rust
use finstack_core::{F, Result};
use super::Greeks;

/// Enhanced Greeks calculator with adaptive methods
pub struct EnhancedGreeksCalculator {
    /// Base bump size multiplier
    pub bump_multiplier: F,
    /// Use Richardson extrapolation
    pub use_richardson: bool,
    /// Adaptive bump sizing
    pub adaptive_bumps: bool,
}

impl Default for EnhancedGreeksCalculator {
    fn default() -> Self {
        Self {
            bump_multiplier: 1.0,
            use_richardson: true,
            adaptive_bumps: true,
        }
    }
}

impl EnhancedGreeksCalculator {
    /// Calculate optimal bump size based on FinancePy methodology
    fn optimal_bump_size(&self, value: F, is_percentage: bool) -> F {
        if self.adaptive_bumps {
            // FinancePy-style optimal bump
            let base = value.abs().max(1.0);
            let epsilon = f64::EPSILON.sqrt();
            
            if is_percentage {
                // For rates/volatilities
                epsilon * 100.0 * self.bump_multiplier
            } else {
                // For spot prices
                epsilon * base * self.bump_multiplier
            }
        } else {
            // Fixed bump sizes
            if is_percentage {
                0.01 * self.bump_multiplier  // 1bp for rates
            } else {
                0.01 * value.abs().max(1.0) * self.bump_multiplier
            }
        }
    }
    
    /// Calculate delta with Richardson extrapolation
    pub fn calculate_delta<P>(
        &self,
        price_fn: P,
        spot: F,
    ) -> F
    where
        P: Fn(F) -> F,
    {
        let h = self.optimal_bump_size(spot, false);
        
        if self.use_richardson {
            // Richardson extrapolation for O(h^4) accuracy
            let h1 = h;
            let h2 = h / 2.0;
            
            let delta1 = (price_fn(spot + h1) - price_fn(spot - h1)) / (2.0 * h1);
            let delta2 = (price_fn(spot + h2) - price_fn(spot - h2)) / (2.0 * h2);
            
            // Richardson extrapolation formula
            (4.0 * delta2 - delta1) / 3.0
        } else {
            // Standard central difference
            (price_fn(spot + h) - price_fn(spot - h)) / (2.0 * h)
        }
    }
    
    /// Calculate gamma with Richardson extrapolation
    pub fn calculate_gamma<P>(
        &self,
        price_fn: P,
        spot: F,
    ) -> F
    where
        P: Fn(F) -> F,
    {
        let h = self.optimal_bump_size(spot, false);
        
        if self.use_richardson {
            let h1 = h;
            let h2 = h / 2.0;
            
            // Second derivative using central differences
            let gamma1 = (price_fn(spot + h1) - 2.0 * price_fn(spot) + price_fn(spot - h1)) 
                / (h1 * h1);
            let gamma2 = (price_fn(spot + h2) - 2.0 * price_fn(spot) + price_fn(spot - h2)) 
                / (h2 * h2);
            
            // Richardson extrapolation
            (4.0 * gamma2 - gamma1) / 3.0
        } else {
            (price_fn(spot + h) - 2.0 * price_fn(spot) + price_fn(spot - h)) / (h * h)
        }
    }
    
    /// Calculate vega with optimal bump
    pub fn calculate_vega<P>(
        &self,
        price_fn: P,
        vol: F,
    ) -> F
    where
        P: Fn(F) -> F,
    {
        let h = self.optimal_bump_size(vol, true);
        
        if self.use_richardson {
            let h1 = h;
            let h2 = h / 2.0;
            
            let vega1 = (price_fn(vol + h1) - price_fn(vol - h1)) / (2.0 * h1);
            let vega2 = (price_fn(vol + h2) - price_fn(vol - h2)) / (2.0 * h2);
            
            (4.0 * vega2 - vega1) / 3.0
        } else {
            (price_fn(vol + h) - price_fn(vol - h)) / (2.0 * h)
        }
    }
    
    /// Calculate cross-Greeks (vanna, volga, charm, etc.)
    pub fn calculate_cross_greeks<P>(
        &self,
        price_fn: P,
        spot: F,
        vol: F,
        t: F,
        r: F,
    ) -> CrossGreeks
    where
        P: Fn(F, F, F, F) -> F,
    {
        let h_spot = self.optimal_bump_size(spot, false);
        let h_vol = self.optimal_bump_size(vol, true);
        let h_time = 1.0 / 365.25; // 1 day
        
        // Vanna: ∂²V/∂S∂σ
        let vanna = if self.use_richardson {
            self.calculate_vanna_richardson(&price_fn, spot, vol, t, r, h_spot, h_vol)
        } else {
            self.calculate_vanna_standard(&price_fn, spot, vol, t, r, h_spot, h_vol)
        };
        
        // Volga: ∂²V/∂σ²
        let volga = (price_fn(spot, vol + h_vol, t, r) 
            - 2.0 * price_fn(spot, vol, t, r) 
            + price_fn(spot, vol - h_vol, t, r)) / (h_vol * h_vol);
        
        // Charm: ∂²V/∂S∂t
        let charm = if t > h_time {
            let delta_now = self.calculate_delta(|s| price_fn(s, vol, t, r), spot);
            let delta_later = self.calculate_delta(|s| price_fn(s, vol, t - h_time, r), spot);
            -(delta_now - delta_later) / h_time * 365.25
        } else {
            0.0
        };
        
        CrossGreeks {
            vanna,
            volga,
            charm,
            color: 0.0,  // Calculate similarly
            speed: 0.0,  // Calculate similarly
        }
    }
    
    fn calculate_vanna_richardson<P>(
        &self,
        price_fn: &P,
        spot: F,
        vol: F,
        t: F,
        r: F,
        h_spot: F,
        h_vol: F,
    ) -> F
    where
        P: Fn(F, F, F, F) -> F,
    {
        // Four-point formula for mixed derivative with Richardson
        let h1_s = h_spot;
        let h1_v = h_vol;
        let h2_s = h_spot / 2.0;
        let h2_v = h_vol / 2.0;
        
        // First approximation
        let vanna1 = (
            price_fn(spot + h1_s, vol + h1_v, t, r)
            - price_fn(spot + h1_s, vol - h1_v, t, r)
            - price_fn(spot - h1_s, vol + h1_v, t, r)
            + price_fn(spot - h1_s, vol - h1_v, t, r)
        ) / (4.0 * h1_s * h1_v);
        
        // Second approximation with smaller step
        let vanna2 = (
            price_fn(spot + h2_s, vol + h2_v, t, r)
            - price_fn(spot + h2_s, vol - h2_v, t, r)
            - price_fn(spot - h2_s, vol + h2_v, t, r)
            + price_fn(spot - h2_s, vol - h2_v, t, r)
        ) / (4.0 * h2_s * h2_v);
        
        // Richardson extrapolation
        (4.0 * vanna2 - vanna1) / 3.0
    }
    
    fn calculate_vanna_standard<P>(
        &self,
        price_fn: &P,
        spot: F,
        vol: F,
        t: F,
        r: F,
        h_spot: F,
        h_vol: F,
    ) -> F
    where
        P: Fn(F, F, F, F) -> F,
    {
        (
            price_fn(spot + h_spot, vol + h_vol, t, r)
            - price_fn(spot + h_spot, vol - h_vol, t, r)
            - price_fn(spot - h_spot, vol + h_vol, t, r)
            + price_fn(spot - h_spot, vol - h_vol, t, r)
        ) / (4.0 * h_spot * h_vol)
    }
}

/// Container for cross-Greeks
#[derive(Clone, Debug)]
pub struct CrossGreeks {
    pub vanna: F,
    pub volga: F,
    pub charm: F,
    pub color: F,
    pub speed: F,
}
```

---

## Testing Strategy

### Unit Tests
Each component should have comprehensive unit tests comparing against:
1. FinancePy reference values
2. Known analytical solutions
3. Market standard benchmarks

### Integration Tests
Create end-to-end tests that:
1. Price complex instruments
2. Calculate full Greeks suite
3. Verify consistency across methods

### Performance Benchmarks
Benchmark against:
1. FinancePy execution times
2. Industry standard libraries
3. Analytical solutions where available

## Implementation Priority

1. **Implied Volatility Solver** (Critical for all options)
2. **American Options** (Major gap in functionality)
3. **Enhanced CDS Pricing** (Improves accuracy significantly)
4. **SABR Model** (Important for volatility smile)
5. **Enhanced Greeks** (Nice-to-have improvement)

## Success Metrics

- All test cases pass with <1bp difference from FinancePy
- Performance within 2x of FinancePy (accounting for Python/Rust differences)
- Zero convergence failures in standard market conditions
- Support for all major option types and exercise styles
