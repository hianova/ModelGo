use model_go::decision_engine::*;
use rkyv::Deserialize;

#[test]
fn test_decision_engine_serialization() {
    let decision = DecisionResult {
        pros: vec!["Fast".to_string()],
        cons: vec!["Expensive".to_string()],
        risk_rating: RiskRating::Medium,
    };

    // Serialize
    let bytes = rkyv::to_bytes::<_, 256>(&decision).unwrap();

    // Deserialize & Validate via bytecheck
    let archived = rkyv::check_archived_root::<DecisionResult>(&bytes).unwrap();
    
    let deserialized: DecisionResult = archived.deserialize(&mut rkyv::Infallible).unwrap();
    
    assert_eq!(deserialized.pros.len(), 1);
    assert_eq!(deserialized.pros[0].as_str(), "Fast");
    assert_eq!(deserialized.risk_rating, RiskRating::Medium);
}
