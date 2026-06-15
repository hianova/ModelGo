use cdDB::{CdDBDispatcher, WriteCommand, QueryNode, QueryResult, Attributes, UserWriter};
use dualcache_ff::{DualCacheFF, Config};
use std::sync::Arc;

/// A Tiered KV Cache OS leveraging DualCache-FF and cdDB.
pub struct TieredKVCache {
    pub cache: Arc<DualCacheFF<u64, Arc<Vec<f32>>>>,
    db: CdDBDispatcher<1024>,
    kv_writer: UserWriter,
    pub block_size: usize,
    pub hidden_dim: usize,
    session_id: u32,
}

impl TieredKVCache {
    pub fn new(session_id: u32, hidden_dim: usize, block_size: usize) -> Self {
        let config = Config::with_memory_budget(512 * 1024 * 1024, 80); // 512MB RAM Budget
        let cache = Arc::new(DualCacheFF::new(config));
        
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
        // warmup() executes Command::InsertT1 pinning it to T1 with Rank 255.
        self.cache.begin_cold_start_session().warmup(key, payload.clone());
        println!("[TieredKVCache] System Prompt Block {} permanently pinned to T1.", block_idx);
        
        // Optionally persist to cdDB
        self.persist_to_disk(block_idx, payload);
    }

    /// Phase 3 (Semantic GC): Insert normal conversation block.
    pub fn insert_block(&self, block_idx: usize, tensor_data: Vec<f32>) {
        let key = self.block_key(block_idx);
        let payload = Arc::new(tensor_data);
        self.cache.insert(key, payload.clone());
        self.persist_to_disk(block_idx, payload);
    }

    fn persist_to_disk(&self, block_idx: usize, payload: Arc<Vec<f32>>) {
        // Transmute Vec<f32> to Vec<u8> for storage
        let byte_len = payload.len() * 4;
        let bytes = unsafe { std::slice::from_raw_parts(payload.as_ptr() as *const u8, byte_len) };
        
        let mut attributes_blob = Attributes::new();
        attributes_blob.insert("tensor".to_string(), bytes.to_vec());

        self.kv_writer.send(WriteCommand::Insert {
            entity_id: self.block_key(block_idx) as usize,
            attributes: Attributes::new(),
            attributes_int: Attributes::new(),
            attributes_blob,
        }).unwrap();
    }

    /// Phase 11 (Intelligent Warmup): Fetch block. If missed, read from cdDB Disk.
    pub fn fetch_block(&self, block_idx: usize) -> Option<Arc<Vec<f32>>> {
        let key = self.block_key(block_idx);
        
        if let Some(block) = self.cache.get(&key) {
            return Some(block);
        }

        println!("[TieredKVCache] TLS Context Switch Detected. Page Fault on Block {}. Loading from cdDB...", block_idx);
        let route = self.db.get_route("kv_storage")?;
        
        let mut loaded = None;
        let nodes = [QueryNode::Get { entity_id: key as usize, attr: "tensor" }];
        route.execute_batch(&nodes, |res| {
            if let QueryResult::Blob(b) = res {
                // Transmute back to Vec<f32>
                let floats_len = b.len() / 4;
                let mut vec_f32 = Vec::with_capacity(floats_len);
                unsafe {
                    std::ptr::copy_nonoverlapping(b.as_ptr() as *const f32, vec_f32.as_mut_ptr(), floats_len);
                    vec_f32.set_len(floats_len);
                }
                loaded = Some(Arc::new(vec_f32));
            }
        });

        if let Some(payload) = loaded {
            // Batch-Aware Fast Pass -> Re-insert to cache
            self.cache.insert(key, payload.clone());
            return Some(payload);
        }

        None
    }

    /// Cache Invalidation: Purge blocks from both DualCacheFF and cdDB persistent storage.
    pub fn invalidate_blocks(&self, start_block: usize, end_block: usize) {
        println!("[TieredKVCache] Invalidating Cache Blocks {} to {} due to file modification...", start_block, end_block);
        for block_idx in start_block..=end_block {
            let key = self.block_key(block_idx);
            
            // 1. Invalidate L1/L2 in-memory Cache (DualCacheFF)
            self.cache.remove(&key);
            
            // 2. Invalidate L3 Disk Cache (cdDB)
            if self.kv_writer.send(WriteCommand::Delete {
                entity_id: key as usize,
            }).is_ok() {
                // Background worker will perform the deletion
            }
        }
    }
}
