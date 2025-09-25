use finstack_valuations::instruments::common::models::{BinomialTree, TreeType};
use finstack_valuations::instruments::common::parameters::OptionMarketParams;

#[test]
fn test_up_and_out_call_reduces_value_with_low_barrier() {
    let market = OptionMarketParams::call(100.0, 100.0, 0.05, 0.20, 1.0);
    let tree = BinomialTree::crr(150);

    let vanilla = tree.price_european(&market).unwrap();
    // Low barrier near spot: likely to hit, so value should move toward rebate (0)
    let up_and_out = tree.price_up_and_out(&market, 101.0, 0.0).unwrap();

    assert!(up_and_out <= vanilla);
}

#[test]
fn test_in_out_parity_single_barrier() {
    let market = OptionMarketParams::put(100.0, 95.0, 0.03, 0.25, 0.5);
    let tree = BinomialTree::crr(200);

    let vanilla = tree.price_european(&market).unwrap();
    let down_and_out = tree.price_down_and_out(&market, 99.0, 0.0).unwrap();
    let down_and_in = tree.price_down_and_in(&market, 99.0, 0.0).unwrap();

    let parity_diff = (vanilla - (down_and_out + down_and_in)).abs();
    assert!(parity_diff < 1e-6);
}

