use model_go::system2_verifier::*;

#[test]
fn parse_valid_ast() {
    let draft = "32|1337|test";
    let ast = System2Verifier::parse_and_verify(draft).unwrap();
    assert_eq!(ast.opcode, 32);
    assert_eq!(ast.payload_id, 1337);
    assert_eq!(ast.arguments, vec!["test"]);
}

#[test]
fn parse_rejects_opcode_zero() {
    let draft = "0|1337|test";
    assert!(System2Verifier::parse_and_verify(draft).is_err());
}

#[test]
fn parse_rejects_invalid_format() {
    let draft = "32|invalid_id|test";
    assert!(System2Verifier::parse_and_verify(draft).is_err());
}

#[test]
fn rejection_sampling_succeeds_within_retries() {
    let result = System2Verifier::execute_with_rejection_sampling("test prompt", 3);
    assert!(result.is_ok());
}

#[test]
fn rejection_sampling_fails_with_zero_retries() {
    let result = System2Verifier::execute_with_rejection_sampling("test prompt", 0);
    assert!(result.is_err());
}
