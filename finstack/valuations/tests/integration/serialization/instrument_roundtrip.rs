//! Round-trip all instrument examples through JSON.
//!
//! For each instrument with an example(), we:
//! - Wrap it in InstrumentEnvelope
//! - Serialize to JSON
//! - Deserialize back via InstrumentEnvelope::from_str (using the manual enum deserializer)
//! - Assert the resulting boxed instrument has the same id
//!
//! This confirms that our JSON format is lossless for construction.
//
use finstack_valuations::instruments::*;
//
fn assert_roundtrip(expected_id: &str, json: json_loader::InstrumentJson) {
    let original = json_loader::InstrumentEnvelope {
        schema: "finstack.instrument/1".to_string(),
        instrument: json,
    };
    let s = serde_json::to_string_pretty(&original).unwrap();
    // First deserialize back to an envelope to surface any serde errors clearly
    let envelope: json_loader::InstrumentEnvelope =
        serde_json::from_str(&s).expect("Envelope serde round-trip failed");
    // Then construct the runtime instrument
    let boxed = envelope
        .instrument
        .into_boxed()
        .expect("into_boxed() failed for round-tripped instrument");
    assert_eq!(boxed.id(), expected_id);
}
//
#[test]
fn all_examples_roundtrip() {
    // Fixed Income
    let ex = Bond::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::Bond(ex));
    let ex = ConvertibleBond::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::ConvertibleBond(ex));
    let ex = InflationLinkedBond::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::InflationLinkedBond(ex));
    let ex = TermLoan::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::TermLoan(ex));
    //
    // Rates
    let ex = InterestRateSwap::example().expect("Example should construct");
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::InterestRateSwap(ex));
    let ex = InflationSwap::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::InflationSwap(ex));
    let ex = ForwardRateAgreement::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::ForwardRateAgreement(ex));
    let ex = Swaption::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::Swaption(ex));
    let ex = InterestRateFuture::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::InterestRateFuture(ex));
    let ex = CmsOption::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::CmsOption(ex));
    let ex = Deposit::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::Deposit(ex));
    let ex = Repo::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::Repo(ex));
    //
    // Credit
    let ex = CreditDefaultSwap::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::CreditDefaultSwap(ex));
    let ex = CDSIndex::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::CDSIndex(ex));
    let ex = CDSTranche::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::CDSTranche(ex));
    let ex = CDSOption::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::CDSOption(ex));
    //
    // Equity
    let ex = Equity::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::Equity(ex));
    let ex = EquityOption::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::EquityOption(ex));
    let ex = AsianOption::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::AsianOption(ex));
    let ex = BarrierOption::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::BarrierOption(ex));
    let ex = LookbackOption::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::LookbackOption(ex));
    let ex = VarianceSwap::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::VarianceSwap(ex));
    //
    // FX
    let ex = FxSwap::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::FxSwap(ex));
    let ex = FxOption::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::FxOption(ex));
    let ex = FxBarrierOption::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::FxBarrierOption(ex));
    let ex = QuantoOption::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::QuantoOption(ex));
    //
    // Exotic Options
    let ex = Autocallable::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::Autocallable(ex));
    let ex = CliquetOption::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::CliquetOption(ex));
    let ex = RangeAccrual::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::RangeAccrual(ex));
    //
    // TRS
    let ex = EquityTotalReturnSwap::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::TrsEquity(ex));
    let ex = FIIndexTotalReturnSwap::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::TrsFixedIncomeIndex(ex));
    //
    // Other
    let ex = Basket::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::Basket(ex));
    let ex = PrivateMarketsFund::example();
    let id = ex.id.as_str().to_string();
    assert_roundtrip(&id, json_loader::InstrumentJson::PrivateMarketsFund(ex));
}
