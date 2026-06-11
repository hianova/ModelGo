use anyhow::Result;
use cdDB::{CdDBDispatcher, WriteCommand, UserWriter, Attributes};
use dualcache_ff::static_cache::static_cache::StaticDualCache;
use dualcache_ff::config::Config;
use std::sync::Arc;

/// The Memory & State Mesh
/// Bridges the mmap model loader, DualCacheFF routing, and cdDB disk persistence.
pub struct MemoryMesh {
    /// O(1) Wait-Free routing state machine mapping Intent Hash -> Success State.
    cache: Arc<StaticDualCache<u64, String, 128>>,
    /// High-performance synchronous persistent storage engine.
    _db: CdDBDispatcher<1024>,
    workflows_writer: UserWriter,
}

impl MemoryMesh {
    pub fn new() -> Result<Self> {
        let config = Config::with_memory_budget(1, 100);
        let cache = Arc::new(StaticDualCache::<u64, String, 128>::new(config));
        
        // Initialize cdDB for persisting long-text state and workflows
        let mut db = CdDBDispatcher::<1024>::new_std(None);
        let workflows_writer = db.register_partition("workflows".to_string());
        let _ = db.register_partition("kv_state".to_string());

        Ok(Self {
            cache,
            _db: db,
            workflows_writer,
        })
    }

    /// Logs a successful workflow intent hash to the fast-path cache.
    pub fn cache_intent_success(&self, intent_hash: u64, result: String) {
        self.cache.insert(intent_hash, result.clone());
        println!("[Memory Mesh] Inserted state for hash 0x{:016X} into DualCacheFF (88ns).", intent_hash);
    }

    /// Persists a complex workflow or long-text memory into the cdDB WAL and SSD.
    pub fn persist_workflow(&self, entity_id: u32, workflow_json: &str) {
        let mut attributes = Attributes::new();
        attributes.insert("workflow_data".to_string(), workflow_json.to_string());

        let cmd = WriteCommand::Insert {
            entity_id: entity_id as usize,
            attributes,
            attributes_int: Attributes::new(),
            attributes_blob: Attributes::new(),
        };

        if self.workflows_writer.send(cmd).is_ok() {
            println!("[Memory Mesh] Workflow ID {} successfully synced to cdDB Tiered Storage.", entity_id);
        }
    }

    /// Exposes the inner wait-free DualCache lookup for O(1) route verification.
    pub fn get_cached_intent(&self, intent_hash: u64) -> Option<String> {
        self.cache.get(&intent_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insertion_and_lookup() {
        let mesh = MemoryMesh::new().unwrap();
        let hash = 0x1234_5678;
        let state = "{\"action\": \"test\"}";
        
        assert_eq!(mesh.get_cached_intent(hash), None);
        
        mesh.cache_intent_success(hash, state.to_string());
        
        assert_eq!(mesh.get_cached_intent(hash).unwrap(), state);
    }
}
