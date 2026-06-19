use model_go::router::*;
use union_code::CompressedIntent;

#[test]
fn test_union_code_engine_route() {
    let engine = UnionCodeEngine::new();
    
    // Test basic valid route that should be handled by the fast path (FST / Cache)
    let result = engine.route("請幫我拿咖啡".as_bytes());
    assert_eq!(
        result,
        Ok((CompressedIntent { opcode: 0x20, payload_id: 0x0A42 }, None))
    );
    
    // Test fallback error for unmapped intents
    let miss_result = engine.route("一個完全沒有見過的未知指令".as_bytes());
    assert_eq!(miss_result, Err(0x06)); // 0x06 is NotFound
}
