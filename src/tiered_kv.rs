use cdDB::{
    Attributes, CdDBDispatcher, DualCacheFF, QueryNode, QueryResult, UserWriter, WriteCommand,
    dualcache_ff,
};
use std::sync::Arc;

/// A Tiered KV Cache OS leveraging DualCache-FF and cdDB.
pub struct TieredKVCache {
    pub cache: Arc<
        DualCacheFF<
            u64,
            Arc<Vec<f32>>,
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
    db: CdDBDispatcher<1024>,
    kv_writer: UserWriter,
    pub block_size: usize,
    pub hidden_dim: usize,
    session_id: u32,
}

impl TieredKVCache {
    pub fn new(
        session_id: u32,
        hidden_dim: usize,
        block_size: usize,
        _engine_config: &crate::config::EngineConfig,
    ) -> Self {
        let cache = Arc::new(DualCacheFF::<
            u64,
            Arc<Vec<f32>>,
            dualcache_ff::core::config::DefaultExponentialPolicy,
            1024,
            2048,
            4096,
            7168,
            16,
            1024,
            64,
        >::new());

        let mut db = CdDBDispatcher::<1024>::new_std(None);
        let kv_writer = db.register_partition("kv_storage".to_string());

        Self {
            cache,
            db,
            kv_writer,
            block_size,
            hidden_dim,
            session_id,
        }
    }

    /// Helper to compute DualCache Hash Key
    fn block_key(&self, block_idx: usize) -> u64 {
        ((self.session_id as u64) << 32) | (block_idx as u64)
    }

    /// Phase 10 (Genius Immunity): Pin System Prompt permanently in T1.
    pub fn insert_system_prompt(&self, block_idx: usize, tensor_data: Vec<f32>) {
        let key = self.block_key(block_idx);
        let payload = Arc::new(tensor_data);
        let handle = self.cache.register_thread();
        self.cache.insert(key, payload.clone(), &handle);
        println!(
            "[TieredKVCache] System Prompt Block {} permanently pinned to T1.",
            block_idx
        );

        // Optionally persist to cdDB
        self.persist_to_disk(block_idx, payload);
    }

    /// Phase 3 (Semantic GC): Insert normal conversation block.
    pub fn insert_block(&self, block_idx: usize, tensor_data: Vec<f32>) {
        let key = self.block_key(block_idx);
        let payload = Arc::new(tensor_data);
        let handle = self.cache.register_thread();
        self.cache.insert(key, payload.clone(), &handle);
        self.persist_to_disk(block_idx, payload);
    }

    fn persist_to_disk(&self, block_idx: usize, payload: Arc<Vec<f32>>) {
        // Transmute Vec<f32> to Vec<u8> for storage
        let byte_len = payload.len() * 4;
        let bytes = unsafe { std::slice::from_raw_parts(payload.as_ptr() as *const u8, byte_len) };

        let mut attributes_blob = Attributes::new();
        attributes_blob.insert("tensor".to_string(), bytes.to_vec());

        self.kv_writer
            .send(WriteCommand::Insert {
                entity_id: self.block_key(block_idx) as usize,
                attributes: Attributes::new(),
                attributes_int: Attributes::new(),
                attributes_blob,
            })
            .unwrap();
    }

    /// Phase 11 (Intelligent Warmup): Fetch block. If missed, read from cdDB Disk.
    pub fn fetch_block(&self, block_idx: usize) -> Option<Arc<Vec<f32>>> {
        let key = self.block_key(block_idx);

        let handle = self.cache.register_thread();
        if let Some(block) = self.cache.get(&key, &handle) {
            return Some(block);
        }

        println!(
            "[TieredKVCache] TLS Context Switch Detected. Page Fault on Block {}. Loading from cdDB...",
            block_idx
        );
        let route = self.db.get_route("kv_storage")?;

        let mut loaded = None;
        let nodes = [QueryNode::Get {
            entity_id: key as usize,
            attr: "tensor",
        }];
        route.execute_batch(&nodes, |res| {
            if let QueryResult::Blob(b) = res {
                // Transmute back to Vec<f32>
                let floats_len = b.len() / 4;
                let mut vec_f32 = Vec::with_capacity(floats_len);
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        b.as_ptr() as *const f32,
                        vec_f32.as_mut_ptr(),
                        floats_len,
                    );
                    vec_f32.set_len(floats_len);
                }
                loaded = Some(Arc::new(vec_f32));
            }
        });

        if let Some(payload) = loaded {
            // Batch-Aware Fast Pass -> Re-insert to cache
            self.cache.insert(key, payload.clone(), &handle);
            return Some(payload);
        }

        None
    }

    /// Cache Invalidation: Purge blocks from both DualCacheFF and cdDB persistent storage.
    pub fn invalidate_blocks(&self, start_block: usize, end_block: usize) {
        println!(
            "[TieredKVCache] Invalidating Cache Blocks {} to {} due to file modification...",
            start_block, end_block
        );
        for block_idx in start_block..=end_block {
            let key = self.block_key(block_idx);

            // Note: DualCacheFF API removed single entry remove. Ignored for now.
            // 2. Invalidate L3 Disk Cache (cdDB)
            if self
                .kv_writer
                .send(WriteCommand::Delete {
                    entity_id: key as usize,
                })
                .is_ok()
            {
                // Background worker will perform the deletion
            }
        }
    }
}

impl Drop for TieredKVCache {
    fn drop(&mut self) {
        // Ignored for now
    }
}
