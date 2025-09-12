#!/usr/bin/env python3
"""
Example demonstrating solver configuration serialization.

This shows how to save and load solver configurations for reproducible computations.
"""

import json
from typing import Dict, Any


def demo_solver_configs():
    """Demonstrate solver configuration serialization (JSON representation)."""
    
    # Newton Solver configuration
    newton_config = {
        "tolerance": 1e-12,
        "max_iterations": 50,
        "fd_step": 1e-8
    }
    
    # Brent Solver configuration
    brent_config = {
        "tolerance": 1e-10,
        "max_iterations": 100,
        "bracket_expansion": 2.0,
        "initial_bracket_size": 0.1
    }
    
    # Hybrid Solver configuration (contains both Newton and Brent)
    hybrid_config = {
        "newton": {
            "tolerance": 1e-12,
            "max_iterations": 50,
            "fd_step": 1e-8
        },
        "brent": {
            "tolerance": 1e-12,
            "max_iterations": 50,
            "bracket_expansion": 2.0,
            "initial_bracket_size": None
        }
    }
    
    # Save configurations to file
    configs = {
        "newton_solver": newton_config,
        "brent_solver": brent_config,
        "hybrid_solver": hybrid_config
    }
    
    # Pretty print the configurations
    print("Solver Configurations:")
    print(json.dumps(configs, indent=2))
    
    # Save to file for later use
    with open("solver_configs.json", "w") as f:
        json.dump(configs, f, indent=2)
    print("\nConfigurations saved to solver_configs.json")
    
    # Load and verify
    with open("solver_configs.json", "r") as f:
        loaded_configs = json.load(f)
    
    print("\nLoaded configurations match original:", configs == loaded_configs)
    
    # Clean up
    import os
    os.remove("solver_configs.json")


def demo_quadrature_serialization():
    """Demonstrate Gauss-Hermite quadrature serialization."""
    
    # Quadrature configurations - only the order is serialized
    # since the points and weights are static data
    quadrature_configs = [
        {"order": 5},   # 5-point quadrature
        {"order": 7},   # 7-point quadrature
        {"order": 10},  # 10-point quadrature
    ]
    
    print("\n\nGauss-Hermite Quadrature Configurations:")
    for config in quadrature_configs:
        print(f"  Order {config['order']}: {config['order']}-point Gauss-Hermite quadrature")
    
    # These configurations can be used to reconstruct the appropriate
    # quadrature instance when deserializing
    print("\nNote: The actual quadrature points and weights are static data")
    print("      and are reconstructed based on the order during deserialization.")


def demo_rng_serialization():
    """Demonstrate RNG state serialization for reproducible Monte Carlo."""
    
    # SimpleRng state
    rng_state = {
        "state": 1234567890  # Internal LCG state
    }
    
    print("\n\nRandom Number Generator State:")
    print(json.dumps(rng_state, indent=2))
    
    print("\nThis allows for:")
    print("  - Checkpointing long-running Monte Carlo simulations")
    print("  - Reproducible random number sequences")
    print("  - Parallel simulation with different seeds")
    
    # Example of multiple RNG states for parallel simulations
    parallel_seeds = [
        {"state": 42},
        {"state": 1337},
        {"state": 9999},
        {"state": 2024},
    ]
    
    print("\nParallel simulation seeds:")
    for i, seed in enumerate(parallel_seeds):
        print(f"  Worker {i}: state = {seed['state']}")


def main():
    """Run all serialization demonstrations."""
    print("=" * 60)
    print("Mathematical Solver Serialization Examples")
    print("=" * 60)
    
    demo_solver_configs()
    demo_quadrature_serialization()
    demo_rng_serialization()
    
    print("\n" + "=" * 60)
    print("Summary:")
    print("  - Solver configurations can be saved/loaded for reproducibility")
    print("  - Quadrature order is sufficient to reconstruct the full quadrature")
    print("  - RNG state allows checkpointing and parallel simulations")
    print("=" * 60)


if __name__ == "__main__":
    main()
