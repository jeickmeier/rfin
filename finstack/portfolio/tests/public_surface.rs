use finstack_portfolio::factor_model::{FactorAssignmentReport, PositionChange, UnmatchedEntry};
use finstack_portfolio::{FactorModel, FactorModelBuilder, RiskDecomposition};

#[test]
fn portfolio_root_and_factor_model_module_exports_compile() {
    fn assert_type<T>() {
        let _ = std::mem::size_of::<T>();
    }

    assert_type::<FactorModel>();
    assert_type::<FactorModelBuilder>();
    assert_type::<RiskDecomposition>();
    assert_type::<FactorAssignmentReport>();
    assert_type::<PositionChange>();
    assert_type::<UnmatchedEntry>();
}
