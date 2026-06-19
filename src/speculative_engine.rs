use vec101::{vec101_compute, vec101_context, types::QuantType};
use crate::loader::{ZeroCopyModelLoader, ArchivedSerializedLayerData};
use std::sync::Arc;

use crate::tiered_kv::TieredKVCache;

pub struct SpeculativeEngine {
    pub kv_cache: TieredKVCache,
    pub x_stream: Vec<i8>,
    pub s_stream: Vec<f32>,
    pub out_buffer: Vec<f32>,
    pub loader: Arc<ZeroCopyModelLoader>,
    pub mailbox: Arc<vec101::types::LockFreeMailbox>,
}

pub struct DraftTree {
    pub tokens: Vec<u32>,
    pub tree_mask: Vec<u32>, // parent_idx
}

impl SpeculativeEngine {
    pub fn new(loader: Arc<ZeroCopyModelLoader>, max_batch_size: usize, hidden_dim: usize, session_id: u32) -> Self {
        Self {
            kv_cache: TieredKVCache::new(session_id, hidden_dim, 64, &crate::config::EngineConfig::default()),
            x_stream: vec![0; max_batch_size * hidden_dim],
            s_stream: vec![1.0; max_batch_size],
            out_buffer: vec![0.0; max_batch_size * hidden_dim],
            loader,
            mailbox: Arc::new(vec101::types::LockFreeMailbox::new()),
        }
    }

    /// Drafting Phase: Layer skipping MTP with Tree Search
    pub unsafe fn run_draft_mode(&mut self, target_depth: usize, layer_stride: usize, max_nodes: usize) -> DraftTree {
        let mut tree = DraftTree {
            tokens: Vec::with_capacity(max_nodes),
            tree_mask: Vec::with_capacity(max_nodes),
        };
        let layers = unsafe { &(*self.loader.archived_weights).layers };
        
        let top_k = 2; // Beam width expansion factor
        let mut current_frontier = vec![0u32]; // Start with the prompt tip node index 0
        let mut node_count = 1; // 0 is root (implicit)
        tree.tokens.push(0); // Root token (dummy or prompt tip)
        tree.tree_mask.push(0); // Root parent is itself
        
        for _depth in 0..target_depth {
            if node_count >= max_nodes || current_frontier.is_empty() {
                break;
            }
            
            let batch_size = current_frontier.len();
            
            for (idx, layer) in layers.iter().enumerate() {
                if idx % layer_stride != 0 {
                    continue; // Skip layer
                }
                
                let (quant_type, w_stream) = match &layer.data {
                    ArchivedSerializedLayerData::Bit1_58(blocks) => (QuantType::Bit1_58, blocks.as_ptr() as *const u8),
                    ArchivedSerializedLayerData::Q4_0(blocks) => (QuantType::Q4_0, blocks.as_ptr() as *const u8),
                };

                let ctx = vec101_context {
                    quant_type,
                    w_stream,
                    x_stream: self.x_stream.as_ptr(),
                    s_stream: self.s_stream.as_ptr(),
                    out_buffer: self.out_buffer.as_mut_ptr(),
                    kv_blocks: core::ptr::null(), // Draft has no past kv
                    num_blocks: 0,
                    block_size: 64,
                    batch_size,
                    num_rows: self.kv_cache.hidden_dim,
                    blocks_per_row: self.kv_cache.hidden_dim / 2048,
                    num_threads: 8,
                    tree_mask: current_frontier.as_ptr(),
                    tree_size: current_frontier.len(),
                };
                
                unsafe { vec101_compute(&ctx); }
            }
            
            let mut next_frontier = Vec::new();
            for b in 0..batch_size {
                let parent_node_idx = current_frontier[b];
                let logits = &self.out_buffer[b * self.kv_cache.hidden_dim .. (b + 1) * self.kv_cache.hidden_dim];
                
                // Emulate Top-K token extraction
                let mut candidates: Vec<(u32, f32)> = logits.iter().enumerate().map(|(i, &v)| (i as u32, v)).collect();
                // Simple partial sort would be better, but sort_unstable_by is sufficient for emulation
                candidates.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                
                for k in 0..top_k {
                    if node_count >= max_nodes { break; }
                    let token_id = candidates[k].0;
                    tree.tokens.push(token_id);
                    tree.tree_mask.push(parent_node_idx);
                    next_frontier.push(node_count as u32);
                    let _ = self.mailbox.try_push(token_id);
                    node_count += 1;
                }
            }
            current_frontier = next_frontier;
        }
        tree
    }

    /// Verify Phase: Compute missing layers with Batch=TreeSize
    pub unsafe fn run_verify_mode(&mut self, draft_tree: &DraftTree, layer_stride: usize) -> Vec<u32> {
        let len = draft_tree.tokens.len();
        let layers = unsafe { &(*self.loader.archived_weights).layers };
        
        for (idx, layer) in layers.iter().enumerate() {
            // Only process the layers we skipped during draft
            if idx % layer_stride == 0 {
                continue; 
            }
            
            let (quant_type, w_stream) = match &layer.data {
                ArchivedSerializedLayerData::Bit1_58(blocks) => (QuantType::Bit1_58, blocks.as_ptr() as *const u8),
                ArchivedSerializedLayerData::Q4_0(blocks) => (QuantType::Q4_0, blocks.as_ptr() as *const u8),
            };

            // In a real scenario, we fetch the blocks needed for the attention
            let block0 = self.kv_cache.fetch_block(0);
            let mut ptrs = Vec::new();
            if let Some(ref b) = block0 {
                ptrs.push(b.as_ptr());
            }

            let ctx = vec101_context {
                quant_type,
                w_stream,
                x_stream: self.x_stream.as_ptr(),
                s_stream: self.s_stream.as_ptr(),
                out_buffer: self.out_buffer.as_mut_ptr(),
                kv_blocks: ptrs.as_ptr(),
                num_blocks: ptrs.len(),
                block_size: 64,
                batch_size: len, // Batch = Tree Size
                num_rows: self.kv_cache.hidden_dim,
                blocks_per_row: self.kv_cache.hidden_dim / 2048,
                num_threads: 8,
                tree_mask: draft_tree.tree_mask.as_ptr(),
                tree_size: len,
            };
            
            unsafe { vec101_compute(&ctx); }
        }
        
        // Extract verified logits for the entire tree batch
        let mut verified_logits = Vec::with_capacity(len);
        for i in 0..len {
            let offset = i * self.kv_cache.hidden_dim;
            let logits = &self.out_buffer[offset..offset + self.kv_cache.hidden_dim];
            
            let mut max_val = f32::NEG_INFINITY;
            let mut max_idx = 0;
            for (idx, &v) in logits.iter().enumerate() {
                if v > max_val {
                    max_val = v;
                    max_idx = idx as u32;
                }
            }
            verified_logits.push(max_idx);
        }

        // Graph traversal to find longest valid path starting from root
        let mut max_depth = 1;
        let mut best_leaf = 0;
        let mut node_depth = vec![0; len];
        let mut is_valid = vec![false; len];
        
        is_valid[0] = true; // Root is always valid initially
        node_depth[0] = 1;

        for i in 1..len {
            let p = draft_tree.tree_mask[i] as usize;
            if is_valid[p] && draft_tree.tokens[i] == verified_logits[p] {
                is_valid[i] = true;
                node_depth[i] = node_depth[p] + 1;
                if node_depth[i] > max_depth {
                    max_depth = node_depth[i];
                    best_leaf = i;
                }
            }
        }

        // Backtrack to extract the accepted tokens sequence
        let mut accepted_tokens = Vec::new();
        let mut curr = best_leaf;
        while curr != 0 {
            accepted_tokens.push(draft_tree.tokens[curr]);
            curr = draft_tree.tree_mask[curr] as usize;
        }
        accepted_tokens.reverse();
        
        // Push the final verified token from the leaf node
        accepted_tokens.push(verified_logits[best_leaf]);
        
        accepted_tokens
    }
}
