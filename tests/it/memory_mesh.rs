use model_go::memory_mesh::*;

#[test]
fn test_cache_insertion_and_lookup() {
    let config = model_go::config::EngineConfig::default();
    let mesh = MemoryMesh::new(&config).unwrap();
    let hash = 0x1234_5678;
    let state = "{\"action\": \"test\"}";
    
    assert_eq!(mesh.get_cached_intent(hash), None);
    
    mesh.cache_intent_success(hash, state.to_string());
    
    // Block and explicitly flush the wait-free TLS buffers to the Daemon
    mesh.cache.sync();
    
    assert_eq!(mesh.get_cached_intent(hash).unwrap(), state);
}

#[test]
fn test_temporal_state_persistence() {
    let config = model_go::config::EngineConfig::default();
    let mesh = MemoryMesh::new(&config).unwrap();
    let workflow_id = 42;
    
    let state_epoch_1 = vec![1, 2, 3, 4];
    let state_epoch_3 = vec![5, 6, 7, 8];
    
    mesh.persist_temporal_state(workflow_id, 1, state_epoch_1.clone());
    mesh.persist_temporal_state(workflow_id, 3, state_epoch_3.clone());
    
    // Let background queue process
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    assert_eq!(mesh.get_temporal_state(workflow_id, 1).unwrap(), state_epoch_1);
    assert_eq!(mesh.get_temporal_state(workflow_id, 3).unwrap(), state_epoch_3);
    assert_eq!(mesh.get_temporal_state(workflow_id, 2), None);
}
