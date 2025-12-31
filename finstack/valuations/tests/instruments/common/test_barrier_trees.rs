use finstack_valuations::instruments::common::models::BinomialTree;
use finstack_valuations::instruments::OptionMarketParams;

#[test]
fn test_up_and_out_call_reduces_value_with_low_barrier() {
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 1.0);
    let tree = BinomialTree::crr(150);

    let vanilla = tree.price_european(&market).unwrap();
    // Low barrier near spot: likely to hit, so value should move toward rebate (0)
    let up_and_out = tree
        .price_barrier_out(&market, Some(101.0), None, 0.0)
        .unwrap();

    assert!(up_and_out <= vanilla);
}

#[test]
fn test_in_out_parity_single_barrier() {
    let market = OptionMarketParams::put(100.0, 95.0, 0.03, 0.25, 0.5);
    let tree = BinomialTree::crr(200);

    let vanilla = tree.price_european(&market).unwrap();
    let down_and_out = tree
        .price_barrier_out(&market, None, Some(99.0), 0.0)
        .unwrap();
    let down_and_in = tree
        .price_barrier_in(&market, None, Some(99.0), 0.0)
        .unwrap();

    let parity_diff = (vanilla - (down_and_out + down_and_in)).abs();
    assert!(parity_diff < 1e-6);
}

#[test]
fn test_american_knock_in_put_ge_european() {
    let market = OptionMarketParams::put(100.0, 105.0, 0.03, 0.25, 1.0);
    let tree = BinomialTree::crr(200);

    let european = tree
        .price_barrier_in(&market, None, Some(95.0), 0.0)
        .unwrap();
    let american = tree
        .price_barrier_in_american(&market, None, Some(95.0), 0.0)
        .unwrap();

    assert!(
        american + 1e-8 >= european,
        "American knock-in should not be less than European"
    );
}
