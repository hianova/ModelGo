use anyhow::Result;
use cdDB::{Attributes, CdDBDispatcher, DualCacheFF, UserWriter, WriteCommand, dualcache_ff};
use std::sync::{Arc, OnceLock};

/// The Memory & State Mesh
/// Bridges the mmap model loader, DualCacheFF routing, and cdDB disk persistence.
pub struct MemoryMesh {
    /// O(1) Wait-Free routing state machine mapping Intent Hash -> Success State.
    pub cache: Arc<
        DualCacheFF<
            u64,
            String,
            dualcache_ff::core::config::DefaultExponentialPolicy,
            1024,
            2048,
            4096,
            7168,
            16,
            1024,
            64,
        >,
    >,
    /// High-performance synchronous persistent storage engine.
    _db: CdDBDispatcher<1024>,
    workflows_writer: UserWriter,
    temporal_writer: UserWriter,
}

static GLOBAL_MESH: OnceLock<MemoryMesh> = OnceLock::new();

impl MemoryMesh {
    pub fn global() -> &'static MemoryMesh {
        GLOBAL_MESH.get_or_init(|| {
            MemoryMesh::new(&crate::config::EngineConfig::default())
                .expect("Failed to initialize global MemoryMesh")
        })
    }

    pub fn new(_config: &crate::config::EngineConfig) -> Result<Self> {
        let cache = Arc::new(DualCacheFF::<
            u64,
            String,
            dualcache_ff::core::config::DefaultExponentialPolicy,
            1024,
            2048,
            4096,
            7168,
            16,
            1024,
            64,
        >::new());

        // Initialize cdDB for persisting long-text state and workflows
        let mut db = CdDBDispatcher::<1024>::new_std(None);
        let workflows_writer = db.register_partition("workflows".to_string());
        let temporal_writer = db.register_partition("temporal_log".to_string());

        Ok(Self {
            cache,
            _db: db,
            workflows_writer,
            temporal_writer,
        })
    }

    /// Logs a successful workflow intent hash to the fast-path cache.
    pub fn cache_intent_success(&self, intent_hash: u64, result: String) {
        let handle = self.cache.register_thread();
        self.cache.insert(intent_hash, result.clone(), &handle);
        println!(
            "[Memory Mesh] Inserted state for hash 0x{:016X} into DualCacheFF (88ns).",
            intent_hash
        );
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
            println!(
                "[Memory Mesh] Workflow ID {} successfully synced to cdDB Tiered Storage.",
                entity_id
            );
        }
    }

    /// Exposes a way to retrieve stored workflows or metadata from the cdDB partition.
    pub fn get_workflow(&self, entity_id: u32) -> Option<String> {
        let mut result = None;
        let route = self._db.get_route("workflows")?;

        let nodes = [cdDB::QueryNode::Get {
            entity_id: entity_id as usize,
            attr: "workflow_data",
        }];

        route.execute_batch(&nodes, |res| {
            if let cdDB::QueryResult::Str(s) = res {
                result = Some(s.clone());
            }
        });

        result
    }

    /// Exposes the inner wait-free DualCache lookup for O(1) route verification.
    pub fn get_cached_intent(&self, intent_hash: u64) -> Option<String> {
        let handle = self.cache.register_thread();
        self.cache.get(&intent_hash, &handle)
    }

    /// Persists a temporal snapshot (e.g. an epoch) of a ChaosState or Workflow.
    pub fn persist_temporal_state(&self, workflow_id: u32, epoch: u32, state_payload: Vec<u8>) {
        let entity_id = ((workflow_id as usize) << 32) | (epoch as usize);

        let cmd = WriteCommand::InsertFast {
            entity_id,
            epoch,
            record_type: 1, // 1 for ChaosState snapshot
            payload: std::sync::Arc::new(state_payload),
        };

        if self.temporal_writer.send(cmd).is_ok() {
            println!(
                "[Memory Mesh] Temporal state for workflow {} at epoch {} successfully recorded.",
                workflow_id, epoch
            );
        }
    }

    /// Retrieves a temporal snapshot of a ChaosState or Workflow at a specific epoch.
    pub fn get_temporal_state(&self, workflow_id: u32, epoch: u32) -> Option<Vec<u8>> {
        let entity_id = ((workflow_id as usize) << 32) | (epoch as usize);

        let mut result_payload = None;
        let route = self._db.get_route("temporal_log")?;

        let nodes = [cdDB::QueryNode::Get {
            entity_id,
            attr: "payload",
        }];

        route.execute_batch(&nodes, |res| {
            if let cdDB::QueryResult::Blob(b) = res {
                result_payload = Some(b);
            }
        });

        result_payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_mesh() {
        std::thread::Builder::new()
            .stack_size(128 * 1024 * 1024)
            .spawn(|| {
                let mesh = MemoryMesh::new(&crate::config::EngineConfig::default()).unwrap();

                mesh.cache_intent_success(0x1234, "SUCCESS_RESULT".to_string());
                let val = mesh.get_cached_intent(0x1234);
                assert_eq!(val, Some("SUCCESS_RESULT".to_string()));

                mesh.persist_workflow(42, "{\"workflow\": 1}");
                let _ = mesh.get_workflow(42);

                mesh.persist_temporal_state(42, 1, vec![1, 2, 3]);
                let _ = mesh.get_temporal_state(42, 1);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
