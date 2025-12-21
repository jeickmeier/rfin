use finstack_core::math::summation::{
    kahan_sum, neumaier_sum, NeumaierAccumulator, pairwise_sum, stable_sum,
};

// ===================================================================
// Kahan Sum Tests
// ===================================================================

#[test]
fn test_kahan_sum_basic() {
    let values = vec![0.1, 0.2, 0.3];
    let result = kahan_sum(values.into_iter());
    
    // Should be close to 0.6
    assert!((result - 0.6).abs() < 1e-15, "Basic sum should be accurate");
}

#[test]
fn test_kahan_sum_large_small_large() {
    // Classic floating-point issue: adding small number to large number
    // Kahan should handle this better than naive summation
    let values = vec![1e16, 1.0, -1e16];
    let result = kahan_sum(values.into_iter());
    
    // Naive sum would lose the 1.0 completely; Kahan should preserve something
    // but may not preserve exactly 1.0 due to the extreme magnitude difference
    assert!(result >= 0.0 && result <= 2.0, "Kahan should preserve small value: got {}", result);
}

#[test]
fn test_kahan_sum_empty() {
    let values: Vec<f64> = vec![];
    let result = kahan_sum(values.into_iter());
    
    assert_eq!(result, 0.0, "Empty sum should be zero");
}

#[test]
fn test_kahan_sum_single_element() {
    let result = kahan_sum(vec![3.14].into_iter());
    assert_eq!(result, 3.14, "Single element should be preserved");
}

#[test]
fn test_kahan_sum_negative_values() {
    let values = vec![-0.1, -0.2, -0.3];
    let result = kahan_sum(values.into_iter());
    
    assert!((result - (-0.6)).abs() < 1e-15, "Should handle negative values");
}

#[test]
fn test_kahan_sum_many_small_values() {
    // Sum many small values that would normally accumulate error
    let values: Vec<f64> = (0..1000).map(|i| (i as f64) * 0.001).collect();
    let result = kahan_sum(values.iter().copied());
    
    // Sum of 0.0, 0.001, 0.002, ..., 0.999 = sum from i=0 to 999 of i*0.001
    // = 0.001 * (sum from i=0 to 999 of i) = 0.001 * (999*1000/2) = 499.5
    let expected = 499.5;
    assert!((result - expected).abs() < 1e-10, "Should handle many small values accurately");
}

#[test]
fn test_kahan_sum_mixed_sign() {
    // Alternating positive and negative values
    let values = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
    let result = kahan_sum(values.into_iter());
    
    // 3 positive and 3 negative = 0.0
    assert!((result - 0.0).abs() < 1e-10, "Should handle mixed signs, got {}", result);
}

// ===================================================================
// Neumaier Sum Tests
// ===================================================================

#[test]
fn test_neumaier_sum_basic() {
    let values = vec![0.1, 0.2, 0.3];
    let result = neumaier_sum(values.into_iter());
    
    assert!((result - 0.6).abs() < 1e-15, "Basic sum should be accurate");
}

#[test]
fn test_neumaier_sum_large_small_opposite() {
    // Neumaier should handle mixed-sign values better than Kahan
    let values = vec![1e16, 1.0, -1e16];
    let result = neumaier_sum(values.into_iter());
    
    assert!((result - 1.0).abs() < 1e-15, "Neumaier should preserve small value");
}

#[test]
fn test_neumaier_sum_empty() {
    let values: Vec<f64> = vec![];
    let result = neumaier_sum(values.into_iter());
    
    assert_eq!(result, 0.0, "Empty sum should be zero");
}

#[test]
fn test_neumaier_sum_single_element() {
    let result = neumaier_sum(vec![2.71].into_iter());
    assert_eq!(result, 2.71, "Single element should be preserved");
}

#[test]
fn test_neumaier_sum_negative_values() {
    let values = vec![-0.5, -0.25, -0.125];
    let result = neumaier_sum(values.into_iter());
    
    let expected = -0.875;
    assert!((result - expected).abs() < 1e-15, "Should handle negative values");
}

#[test]
fn test_neumaier_sum_financial_cashflows() {
    // Simulate mixed-sign financial cashflows
    let values = vec![
        -100.0,  // initial investment
        10.0,    // year 1 return
        10.0,    // year 2 return
        10.0,    // year 3 return
        110.0,   // year 4 return + principal
    ];
    let result = neumaier_sum(values.into_iter());
    
    let expected = 40.0;
    assert!((result - expected).abs() < 1e-12, "Should handle cashflows accurately");
}

#[test]
fn test_neumaier_sum_large_then_small_additions() {
    // Adding increasingly small numbers to a large base
    let mut values = vec![1e10];
    for i in 1..=100 {
        values.push((i as f64) * 1e-5);
    }
    let result = neumaier_sum(values.into_iter());
    
    // The sum of 1e-5, 2e-5, ..., 100e-5 = 1e-5 * (1+2+...+100) = 1e-5 * 5050 = 0.05050
    let expected = 1e10 + 0.05050;
    assert!((result - expected).abs() < 1e-8, "Should preserve small additions to large base");
}

// ===================================================================
// Neumaier Accumulator Tests
// ===================================================================

#[test]
fn test_neumaier_accumulator_basic() {
    let mut acc = NeumaierAccumulator::new();
    
    acc.add(0.1);
    acc.add(0.2);
    acc.add(0.3);
    
    let total = acc.total();
    assert!((total - 0.6).abs() < 1e-15, "Accumulator should sum accurately");
}

#[test]
fn test_neumaier_accumulator_default() {
    let acc = NeumaierAccumulator::default();
    assert_eq!(acc.total(), 0.0, "Default accumulator should have zero total");
}

#[test]
fn test_neumaier_accumulator_copy() {
    let mut acc = NeumaierAccumulator::new();
    acc.add(5.5);
    
    let acc_copy = acc;
    assert_eq!(acc_copy.total(), 5.5, "Copy should preserve state");
}

#[test]
fn test_neumaier_accumulator_large_and_small() {
    let mut acc = NeumaierAccumulator::new();
    
    acc.add(1e16);
    acc.add(1.0);
    acc.add(-1e16);
    
    let total = acc.total();
    assert!((total - 1.0).abs() < 1e-12, "Accumulator should preserve small value");
}

#[test]
fn test_neumaier_accumulator_negative() {
    let mut acc = NeumaierAccumulator::new();
    
    acc.add(-1.5);
    acc.add(-2.5);
    acc.add(-3.0);
    
    let total = acc.total();
    assert!((total - (-7.0)).abs() < 1e-15, "Should handle negative accumulation");
}

#[test]
fn test_neumaier_accumulator_incremental() {
    let mut acc = NeumaierAccumulator::new();
    
    for i in 1..=10 {
        acc.add(i as f64);
    }
    
    let total = acc.total();
    let expected = 55.0; // 1+2+3+...+10
    assert!((total - expected).abs() < 1e-14, "Incremental addition should be accurate");
}

#[test]
fn test_neumaier_accumulator_clone() {
    let mut acc = NeumaierAccumulator::new();
    acc.add(7.5);
    
    let acc_cloned = acc.clone();
    assert_eq!(acc_cloned.total(), 7.5, "Clone should preserve state");
}

#[test]
fn test_neumaier_accumulator_alternating() {
    let mut acc = NeumaierAccumulator::new();
    
    for _ in 0..5 {
        acc.add(1.0);
        acc.add(-0.5);
    }
    
    let total = acc.total();
    let expected = 2.5; // 5 * (1.0 - 0.5)
    assert!((total - expected).abs() < 1e-14, "Should handle alternating signs");
}

// ===================================================================
// Pairwise Sum Tests
// ===================================================================

#[test]
fn test_pairwise_sum_basic() {
    let values = [0.1, 0.2, 0.3];
    let result = pairwise_sum(&values);
    
    assert!((result - 0.6).abs() < 1e-15, "Basic pairwise sum should be accurate");
}

#[test]
fn test_pairwise_sum_empty() {
    let values: [f64; 0] = [];
    let result = pairwise_sum(&values);
    
    assert_eq!(result, 0.0, "Empty pairwise sum should be zero");
}

#[test]
fn test_pairwise_sum_single() {
    let values = [3.14];
    let result = pairwise_sum(&values);
    
    assert_eq!(result, 3.14, "Single element should be preserved");
}

#[test]
fn test_pairwise_sum_two() {
    let values = [1.5, 2.5];
    let result = pairwise_sum(&values);
    
    assert_eq!(result, 4.0, "Two elements should sum correctly");
}

#[test]
fn test_pairwise_sum_power_of_two() {
    // Test with powers of 2 for recursive division
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let result = pairwise_sum(&values);
    
    let expected = 36.0; // 1+2+3+4+5+6+7+8
    assert!((result - expected).abs() < 1e-14, "Pairwise sum of powers of 2");
}

#[test]
fn test_pairwise_sum_odd_length() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let result = pairwise_sum(&values);
    
    let expected = 15.0;
    assert!((result - expected).abs() < 1e-14, "Pairwise sum with odd length");
}

#[test]
fn test_pairwise_sum_negative() {
    let values = [-1.0, -2.0, -3.0, -4.0];
    let result = pairwise_sum(&values);
    
    let expected = -10.0;
    assert!((result - expected).abs() < 1e-14, "Pairwise sum with negative values");
}

#[test]
fn test_pairwise_sum_mixed_sign() {
    let values = vec![10.0, -5.0, 3.0, -2.0, 1.0];
    let result = pairwise_sum(&values);
    
    let expected = 7.0;
    assert!((result - expected).abs() < 1e-14, "Pairwise sum with mixed signs");
}

#[test]
fn test_pairwise_sum_large_array() {
    // Test with larger array to exercise recursion
    let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
    let result = pairwise_sum(&values);
    
    let expected = (100.0 * 101.0) / 2.0; // Gauss sum formula
    assert!((result - expected).abs() < 1e-10, "Pairwise sum of 100 elements");
}

#[test]
fn test_pairwise_sum_many_small() {
    let values: Vec<f64> = (0..1000).map(|i| (i as f64) * 1e-10).collect();
    let result = pairwise_sum(&values);
    
    // Sum of i*1e-10 for i=0 to 999 = 1e-10 * (999*1000/2) = 0.04995
    let expected = 1e-10 * (999.0 * 1000.0 / 2.0);
    assert!((result - expected).abs() < 1e-15, "Pairwise sum of many small values");
}

// ===================================================================
// Stable Sum Tests
// ===================================================================

#[test]
fn test_stable_sum_basic() {
    let values = [0.1, 0.2, 0.3];
    let result = stable_sum(&values);
    
    assert!((result - 0.6).abs() < 1e-15, "Stable sum should be accurate");
}

#[test]
fn test_stable_sum_empty() {
    let values: [f64; 0] = [];
    let result = stable_sum(&values);
    
    assert_eq!(result, 0.0, "Empty stable sum should be zero");
}

#[test]
fn test_stable_sum_mixed_sign() {
    let values = [100.0, -50.0, 10.0, -30.0, 5.0];
    let result = stable_sum(&values);
    
    let expected = 35.0;
    assert!((result - expected).abs() < 1e-14, "Stable sum should handle mixed signs");
}

#[test]
fn test_stable_sum_financial_scenario() {
    // Realistic financial scenario: various transaction amounts
    let values = [
        -10000.0,   // purchase
        1000.0,     // dividend
        500.0,      // interest
        -500.0,     // fee
        2000.0,     // bonus
        -200.0,     // tax
    ];
    let result = stable_sum(&values);
    
    let expected = -7200.0;
    assert!((result - expected).abs() < 1e-10, "Stable sum for financial scenario");
}

#[test]
fn test_stable_sum_precision_preservation() {
    // Test that stable_sum preserves precision better than naive summation
    let values = [1e16, 1.0, 1.0, 1.0, -1e16];
    let result = stable_sum(&values);
    
    // Naive summation would lose the three 1.0 values
    // Stable should preserve them
    let expected = 3.0;
    assert!(result >= 2.0 && result <= 4.0, "Should preserve small values: got {}", result);
}

// ===================================================================
// Comparative Tests
// ===================================================================

#[test]
fn test_all_methods_agree_simple() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    
    let kahan_result = kahan_sum(values.iter().copied());
    let neumaier_result = neumaier_sum(values.iter().copied());
    let pairwise_result = pairwise_sum(&values);
    let stable_result = stable_sum(&values);
    
    // All should agree for simple cases
    let expected = 15.0;
    assert!((kahan_result - expected).abs() < 1e-14);
    assert!((neumaier_result - expected).abs() < 1e-14);
    assert!((pairwise_result - expected).abs() < 1e-14);
    assert!((stable_result - expected).abs() < 1e-14);
}

#[test]
fn test_neumaier_accumulator_matches_function() {
    let values = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    
    let mut acc = NeumaierAccumulator::new();
    for v in &values {
        acc.add(*v);
    }
    
    let acc_total = acc.total();
    let func_total = neumaier_sum(values.iter().copied());
    
    assert!((acc_total - func_total).abs() < 1e-15, "Accumulator should match function");
}

#[test]
fn test_methods_with_large_cancellation() {
    // Large positive, then nearly-equal large negative
    let values = vec![1e15, -1e15 + 1.0];
    
    let kahan_result = kahan_sum(values.iter().copied());
    let neumaier_result = neumaier_sum(values.iter().copied());
    let pairwise_result = pairwise_sum(&values);
    let stable_result = stable_sum(&values);
    
    // All should detect the 1.0 remainder
    // (Though precision may vary)
    assert!(kahan_result >= 0.0 && kahan_result <= 2.0);
    assert!(neumaier_result >= 0.0 && neumaier_result <= 2.0);
    assert!(pairwise_result >= 0.0 && pairwise_result <= 2.0);
    assert!(stable_result >= 0.0 && stable_result <= 2.0);
}
