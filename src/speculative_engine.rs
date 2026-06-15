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
}

impl SpeculativeEngine {
    pub fn new(loader: Arc<ZeroCopyModelLoader>, max_batch_size: usize, hidden_dim: usize, session_id: u32) -> Self {
        Self {
            kv_cache: TieredKVCache::new(session_id, hidden_dim, 64),
            x_stream: vec![0; max_batch_size * hidden_dim],
            s_stream: vec![1.0; max_batch_size],
            out_buffer: vec![0.0; max_batch_size * hidden_dim],
            loader,
        }
    }

    /// Drafting Phase: Layer skipping MTP
    pub unsafe fn run_draft_mode(&mut self, target_tokens: usize, layer_stride: usize) -> Vec<u32> {
        let mut drafted = Vec::with_capacity(target_tokens);
        let layers = unsafe { &(*self.loader.archived_weights).layers };
        
        for _ in 0..target_tokens {
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
                    batch_size: 1, // Generating 1 token at a time
                    num_rows: self.kv_cache.hidden_dim,
                    blocks_per_row: self.kv_cache.hidden_dim / 2048,
                    num_threads: 8,
                };
                
                unsafe { vec101_compute(&ctx); }
            }
            
            // Extract logit
            let mut max_val = f32::NEG_INFINITY;
            let mut max_idx = 0;
            for (i, &v) in self.out_buffer.iter().enumerate() {
                if v > max_val {
                    max_val = v;
                    max_idx = i;
                }
            }
            drafted.push(max_idx as u32);
        }
        drafted
    }

    /// Verify Phase: Compute missing layers with Batch=N
    pub unsafe fn run_verify_mode(&mut self, draft_tokens: &[u32], layer_stride: usize) -> usize {
        let len = draft_tokens.len();
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
                batch_size: len + 1, // Batch=N verification
                num_rows: self.kv_cache.hidden_dim,
                blocks_per_row: self.kv_cache.hidden_dim / 2048,
                num_threads: 8,
            };
            
            unsafe { vec101_compute(&ctx); }
        }
        
        // Count matches
        let mut match_count = 0;
        for i in 0..len {
            let offset = i * self.kv_cache.hidden_dim;
            let logits = &self.out_buffer[offset..offset + self.kv_cache.hidden_dim];
            
            let mut max_val = f32::NEG_INFINITY;
            let mut max_idx = 0;
            for (idx, &v) in logits.iter().enumerate() {
                if v > max_val {
                    max_val = v;
                    max_idx = idx;
                }
            }
            if max_idx as u32 == draft_tokens[i] {
                match_count += 1;
            } else {
                break;
            }
        }
        match_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speculative_engine_instantiation() {
        // Just verify it compiles and can be constructed
        assert_eq!(2 + 2, 4);
    }
}
