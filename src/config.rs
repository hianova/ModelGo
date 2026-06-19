#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Number of threads for the vec101 execution context (prevents M1 efficiency core straggler effects).
    pub vec101_num_threads: usize,
    
    // --- DualCacheFF / cdDB Hardcoded Variables --- //
    
    /// Tiered KV Cache RAM limit.
    pub kv_cache_memory_budget: usize,
    /// Tiered KV Cache GC eviction threshold.
    pub kv_cache_eviction_threshold: u32,
    
    /// O(1) Memory Mesh routing cache budget.
    pub mesh_memory_budget: usize,
    /// O(1) Memory Mesh GC eviction threshold.
    pub mesh_eviction_threshold: u32,
    
    /// Fallback Router memory budget.
    pub router_memory_budget: usize,
    /// Fallback Router GC eviction threshold.
    pub router_eviction_threshold: u32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            vec101_num_threads: 4, 
            kv_cache_memory_budget: 512 * 1024 * 1024, // 512MB RAM
            kv_cache_eviction_threshold: 80,
            mesh_memory_budget: 1000,
            mesh_eviction_threshold: 100,
            router_memory_budget: 1,
            router_eviction_threshold: 100,
        }
    }
}
