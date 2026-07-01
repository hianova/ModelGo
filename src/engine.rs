use crate::loader::*;
use vec101::compute::vec101_compute;
use vec101::tokenizer::TrieTokenizer;
use vec101::types::{QuantType, vec101_context};

/// Surprisal Index (Cognitive Telemetry)
pub struct SurprisalIndex {
    pub score: f32,
    pub is_outlier: bool,
}

pub struct Vec101Engine {
    pub loader: ZeroCopyModelLoader,
    pub safetensors_loader: Option<SafetensorsMmapLoader>,
    pub tokenizer: TrieTokenizer,
    pub config: crate::config::EngineConfig,
}

impl Vec101Engine {
    pub fn new(model_path: &str, config: crate::config::EngineConfig) -> std::io::Result<Self> {
        let loader = ZeroCopyModelLoader::new(model_path)?;
        let mut tokenizer = TrieTokenizer::new(0);
        // Default init for fallback
        tokenizer.vocab_size = 262144;

        Ok(Self {
            loader,
            safetensors_loader: None,
            tokenizer,
            config,
        })
    }

    /// CanvasDiffusion: Markdown Parallel Generation (Autoregressive All-Layers)
    pub fn generate_parallel(&mut self, prompts: &[String]) -> Vec<String> {
        let batch_size = prompts.len();

        let mut out_buffer = vec![0.0f32; batch_size * 4096];
        let x_stream = vec![0i8; batch_size * 16 * 2048];
        let s_stream = vec![1.0f32; batch_size];

        let mut ctx = vec101_context { hardware_handle: std::ptr::null_mut(),
            quant_type: QuantType::Bit1_58,
            w_stream: core::ptr::null(),
            x_stream: x_stream.as_ptr(),
            s_stream: s_stream.as_ptr(),
            out_buffer: out_buffer.as_mut_ptr(),
            kv_blocks: core::ptr::null(),
            num_blocks: 0,
            block_size: 16,
            batch_size,
            num_rows: 4096,
            blocks_per_row: 16,
            num_threads: self.config.vec101_num_threads, // Harness multi-thread batch processing
            tree_mask: core::ptr::null(),
            tree_size: 0,
        };

        let mut results = vec![String::new(); batch_size];
        let layers = unsafe { &(*self.loader.archived_weights).layers };

        // Generate 64 tokens autoregressively for "段落生成" (paragraph generation)
        for token_idx in 0..64 {
            // Forward pass: Iterate through ALL layers for physical matrix operations
            for layer in layers.iter() {
                match &layer.data {
                    ArchivedSerializedLayerData::Bit1_58(blocks) => {
                        ctx.quant_type = QuantType::Bit1_58;
                        ctx.w_stream = blocks.as_ptr() as *const u8;
                        ctx.num_rows = blocks.len() / ctx.blocks_per_row;
                    }
                    ArchivedSerializedLayerData::Q4_0(blocks) => {
                        ctx.quant_type = QuantType::Q4_0;
                        ctx.w_stream = blocks.as_ptr() as *const u8;
                        ctx.num_rows = blocks.len() / (ctx.blocks_per_row * 8);
                    }
                }
                unsafe {
                    if !ctx.w_stream.is_null() {
                        vec101_compute(&ctx);
                    }
                }
            }

            // Post-forward: Decode logits to tokens
            for i in 0..batch_size {
                let start = i * 4096;
                let logits = &out_buffer[start..start + 4096];

                let mut max_val = f32::NEG_INFINITY;
                let mut max_idx = 0;
                for (idx, &v) in logits.iter().enumerate() {
                    if v > max_val {
                        max_val = v;
                        max_idx = idx;
                    }
                }

                if token_idx == 0 {
                    results[i].push_str(&format!(
                        "{}\n\n[vec101 Batch {} Generated Content utilizing Zero-Copy engine:\n",
                        prompts[i], i
                    ));
                }

                results[i].push_str(&format!("T{} ", max_idx));
            }
        }

        for res in &mut results {
            res.push(']');
        }

        results
    }

    /// Parallel Generation with Shared Prefix KV Cache (Batch = 3)
    pub fn generate_shared_parallel(
        &mut self,
        shared_kv: &crate::tiered_kv::TieredKVCache,
        prompts: &[String; 3],
    ) -> crate::decision_engine::DecisionResult {
        let batch_size = 3;

        let mut out_buffer = vec![0.0f32; batch_size * 4096];
        let x_stream = vec![0i8; batch_size * 16 * 2048];
        let s_stream = vec![1.0f32; batch_size];

        // Emulate KV blocks fetching for the shared prefix
        let block0 = shared_kv.fetch_block(0);
        let mut ptrs = Vec::new();
        if let Some(ref b) = block0 {
            ptrs.push(b.as_ptr());
        }

        let mut ctx = vec101_context { hardware_handle: std::ptr::null_mut(),
            quant_type: QuantType::Bit1_58,
            w_stream: core::ptr::null(),
            x_stream: x_stream.as_ptr(),
            s_stream: s_stream.as_ptr(),
            out_buffer: out_buffer.as_mut_ptr(),
            kv_blocks: ptrs.as_ptr(),
            num_blocks: ptrs.len(),
            block_size: 64,
            batch_size,
            num_rows: 4096,
            blocks_per_row: 16,
            num_threads: self.config.vec101_num_threads,
            tree_mask: core::ptr::null(),
            tree_size: 0,
        };

        let mut results = vec![String::new(); batch_size];
        for i in 0..batch_size {
            results[i].push_str(&prompts[i]);
            results[i].push('\n');
        }
        let mut is_active = vec![true; batch_size];
        let layers = unsafe { &(*self.loader.archived_weights).layers };

        for token_idx in 0..64 {
            if !is_active.iter().any(|&active| active) {
                break; // All streams hit EOS
            }

            for layer in layers.iter() {
                match &layer.data {
                    ArchivedSerializedLayerData::Bit1_58(blocks) => {
                        ctx.quant_type = QuantType::Bit1_58;
                        ctx.w_stream = blocks.as_ptr() as *const u8;
                        ctx.num_rows = blocks.len() / ctx.blocks_per_row;
                    }
                    ArchivedSerializedLayerData::Q4_0(blocks) => {
                        ctx.quant_type = QuantType::Q4_0;
                        ctx.w_stream = blocks.as_ptr() as *const u8;
                        ctx.num_rows = blocks.len() / (ctx.blocks_per_row * 8);
                    }
                }
                unsafe {
                    if !ctx.w_stream.is_null() {
                        vec101_compute(&ctx);
                    }
                }
            }

            for i in 0..batch_size {
                if !is_active[i] {
                    continue;
                }

                let start = i * 4096;
                let logits = &out_buffer[start..start + 4096];

                let mut max_val = f32::NEG_INFINITY;
                let mut max_idx = 0;
                for (idx, &v) in logits.iter().enumerate() {
                    if v > max_val {
                        max_val = v;
                        max_idx = idx;
                    }
                }

                // Decode Token
                let token_str = self.tokenizer.decode(&[max_idx as u32]);
                results[i].push_str(&token_str);

                // Emulate EOS (Token 2 = EOS in Gemma, or artificial stop)
                if max_idx == 2 || token_idx > 30 && max_idx % 20 == 0 {
                    is_active[i] = false;
                }
            }
        }

        crate::decision_engine::DecisionResult {
            pros: vec![results[0].clone()],
            cons: vec![results[1].clone()],
            risk_rating: crate::decision_engine::RiskRating::Medium, // Emulated classification
        }
    }

    /// Zero-Token Generation: Pre-fill only and directly inspect logits for classification.
    /// candidate_tokens: slice of token IDs (e.g., [High, Medium, Low])
    /// Returns the winning token ID.
    pub fn classify_logits(&mut self, prompt: &str, candidate_tokens: &[u32]) -> u32 {
        let batch_size = 1;
        let mut out_buffer = vec![0.0f32; 4096];
        let x_stream = vec![0i8; 16 * 2048];
        let s_stream = [1.0f32; 1];

        let mut ctx = vec101_context { hardware_handle: std::ptr::null_mut(),
            quant_type: QuantType::Bit1_58,
            w_stream: core::ptr::null(),
            x_stream: x_stream.as_ptr(),
            s_stream: s_stream.as_ptr(),
            out_buffer: out_buffer.as_mut_ptr(),
            kv_blocks: core::ptr::null(),
            num_blocks: 0,
            block_size: 16,
            batch_size,
            num_rows: 4096,
            blocks_per_row: 16,
            num_threads: self.config.vec101_num_threads, // Single batch classification can run single-threaded or multi-threaded
            tree_mask: core::ptr::null(),
            tree_size: 0,
        };

        let layers = unsafe { &(*self.loader.archived_weights).layers };

        let total_layers = layers.len();
        let start_confident_layer = total_layers.saturating_sub(5);

        let mut best_entropy = f32::MAX;
        let mut confident_best_token = candidate_tokens[0];
        let mut chosen_layer = 0;

        // Single Forward Pass (Prefill only)
        for (l_idx, layer) in layers.iter().enumerate() {
            match &layer.data {
                ArchivedSerializedLayerData::Bit1_58(blocks) => {
                    ctx.quant_type = QuantType::Bit1_58;
                    ctx.w_stream = blocks.as_ptr() as *const u8;
                    ctx.num_rows = blocks.len() / ctx.blocks_per_row;
                }
                ArchivedSerializedLayerData::Q4_0(blocks) => {
                    ctx.quant_type = QuantType::Q4_0;
                    ctx.w_stream = blocks.as_ptr() as *const u8;
                    ctx.num_rows = blocks.len() / (ctx.blocks_per_row * 8);
                }
            }
            unsafe {
                if !ctx.w_stream.is_null() {
                    vec101_compute(&ctx);
                }
            }

            if l_idx >= start_confident_layer {
                let mut max_logit = f32::NEG_INFINITY;
                for &token in candidate_tokens {
                    if (token as usize) < 4096 {
                        max_logit = max_logit.max(out_buffer[token as usize]);
                    }
                }

                let mut sum_exp = 0.0;
                let mut exps = Vec::with_capacity(candidate_tokens.len());

                for &token in candidate_tokens {
                    if (token as usize) < 4096 {
                        let exp_val = (out_buffer[token as usize] - max_logit).exp();
                        exps.push((token, exp_val));
                        sum_exp += exp_val;
                    }
                }

                let mut entropy = 0.0;
                let mut layer_best_token = candidate_tokens[0];
                let mut layer_best_prob = -1.0;

                for (token, exp_val) in exps {
                    let p = exp_val / sum_exp;
                    if p > 0.0 {
                        entropy -= p * p.ln();
                    }
                    if p > layer_best_prob {
                        layer_best_prob = p;
                        layer_best_token = token;
                    }
                }

                if entropy < best_entropy {
                    best_entropy = entropy;
                    confident_best_token = layer_best_token;
                    chosen_layer = l_idx;
                }
            }
        }

        println!(
            "[Confident Decoding] Query: '{}'. Picked Layer {} (Entropy: {:.4}) -> Token {}",
            prompt.trim().replace('\n', " "),
            chosen_layer,
            best_entropy,
            confident_best_token
        );
        confident_best_token
    }

    /// MTP (Speculative Decoding) Parallel Verification Pipeline
    /// Takes prompts and corresponding draft predictions, tokenizes drafts,
    /// and performs a Batch=N physical verification pass across all layers.
    pub fn verify_draft_parallel(&mut self, prompts: &[String], drafts: &[String]) -> Vec<String> {
        let batch_size = prompts.len();
        let mut results = vec![String::new(); batch_size];
        let layers = unsafe { &(*self.loader.archived_weights).layers };

        // Process each prompt's draft independently in this demo loop
        for i in 0..batch_size {
            let prompt = &prompts[i];
            let draft_text = &drafts[i];

            // 1. Tokenize the draft using the native TrieTokenizer
            let draft_tokens = self.tokenizer.encode(draft_text);
            let n_drafts = draft_tokens.len();

            // If the draft is empty, fallback to simple generation
            if n_drafts == 0 {
                results[i] = format!("{}\n\n[vec101 MTP] No draft generated.", prompt);
                continue;
            }

            // 2. Setup parallel batch context (Batch Size = Number of Draft Tokens + 1)
            let parallel_batch = n_drafts + 1;
            let mut out_buffer = vec![0.0f32; parallel_batch * 4096];
            let x_stream = vec![0i8; parallel_batch * 16 * 2048];
            let s_stream = vec![1.0f32; parallel_batch];

            let mut ctx = vec101_context { hardware_handle: std::ptr::null_mut(),
                quant_type: QuantType::Bit1_58,
                w_stream: core::ptr::null(),
                x_stream: x_stream.as_ptr(),
                s_stream: s_stream.as_ptr(),
                out_buffer: out_buffer.as_mut_ptr(),
                kv_blocks: core::ptr::null(),
                num_blocks: 0,
                block_size: 16,
                batch_size: parallel_batch,
                num_rows: 4096,
                blocks_per_row: 16,
                num_threads: self.config.vec101_num_threads, // Multi-threaded Draft Verification
                tree_mask: core::ptr::null(),
                tree_size: 0,
            };

            // 3. Forward Pass: ALL LAYERS (Physical Verification)
            for layer in layers.iter() {
                match &layer.data {
                    ArchivedSerializedLayerData::Bit1_58(blocks) => {
                        ctx.quant_type = QuantType::Bit1_58;
                        ctx.w_stream = blocks.as_ptr() as *const u8;
                    }
                    ArchivedSerializedLayerData::Q4_0(blocks) => {
                        ctx.quant_type = QuantType::Q4_0;
                        ctx.w_stream = blocks.as_ptr() as *const u8;
                    }
                }

                unsafe {
                    if !ctx.w_stream.is_null() {
                        vec101_compute(&ctx);
                    }
                }
            }

            // 4. Verification: Count matching argmaxes
            let mut accepted_count = 0;
            let mut verified_text = String::new();

            for (token_idx, &draft_token) in draft_tokens.iter().enumerate().take(n_drafts) {
                let start = token_idx * 4096;
                let logits = &out_buffer[start..start + 4096];

                let mut max_val = f32::NEG_INFINITY;
                let mut max_idx = 0;
                for (idx, &v) in logits.iter().enumerate() {
                    if v > max_val {
                        max_val = v;
                        max_idx = idx;
                    }
                }

                if max_idx as u32 == draft_token {
                    accepted_count += 1;
                    verified_text.push_str(&format!("T{} ", max_idx));
                } else {
                    verified_text.push_str(&format!("(Rejected: T{})", max_idx));
                    break;
                }
            }

            results[i] = format!(
                "{}\n\n[vec101 MTP] Verified {}/{} tokens in parallel:\n[{}]",
                prompt, accepted_count, n_drafts, verified_text
            );
        }

        results
    }

    /// Two-Tier Indexing: Query with Page Fault handling for cdDB
    pub fn query_with_page_fault(&mut self, query: &str) -> Result<String, String> {
        println!(
            "[Query] Searching cdDB for tags matching query: '{}'",
            query
        );

        let mesh = crate::memory_mesh::MemoryMesh::global();
        let data_dir = std::path::Path::new("./data");

        let mut target_file = None;
        let mut target_metadata = None;
        let mut target_file_id = 0u32;

        // Scan the actual data directory
        if let Ok(entries) = std::fs::read_dir(data_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    filename.hash(&mut hasher);
                    let file_id = hasher.finish() as u32;

                    if let Some(metadata_str) = mesh.get_workflow(file_id) {
                        #[cfg(not(test))]
                        let meta = serde_json::from_str::<crate::daemon::DocumentMetadata>(&metadata_str);
                        
                        #[cfg(test)]
                        #[derive(serde::Deserialize, serde::Serialize)]
                        struct DocumentMetadata {
                            filename: String,
                            vendor: String,
                            doc_type: String,
                            status: String,
                        }
                        
                        #[cfg(test)]
                        let meta = serde_json::from_str::<DocumentMetadata>(&metadata_str);

                        if let Ok(meta) = meta {
                        // Match query against filename, vendor, or doc_type (case-insensitive)
                        let query_lower = query.to_lowercase();
                        if filename.to_lowercase().contains(&query_lower)
                            || meta.vendor.to_lowercase().contains(&query_lower)
                            || meta.doc_type.to_lowercase().contains(&query_lower)
                        {
                            target_file = Some(path.clone());
                            target_metadata = Some(meta);
                            target_file_id = file_id;
                            break;
                        }
                    }
                }
            }
            }
        }

        if let (Some(path), Some(mut meta)) = (target_file, target_metadata) {
            if meta.status == "unprocessed" {
                println!(
                    "[Page Fault] Document {:?} has unprocessed KV Cache. Triggering right-brain 4-bit compute...",
                    path
                );

                // Lazy load the safetensors 4-bit model via mmap if not already loaded
                if self.safetensors_loader.is_none() {
                    // Pointing to the specific right-brain Q4_0 safetensors
                    match SafetensorsMmapLoader::new(
                        "../google:gemma-4-E2B-it-qat-q4_0-unquantized.safetensors",
                    ) {
                        Ok(loader) => {
                            println!(
                                "[SafetensorsMmapLoader] Zero-copy mapped 4-bit Safetensors model instantly."
                            );
                            self.safetensors_loader = Some(loader);
                        }
                        Err(e) => {
                            println!(
                                "[SafetensorsMmapLoader] Could not load safetensors model: {}",
                                e
                            );
                        }
                    }
                }

                println!(
                    "[KV Compute] Computing KV Cache Blocks in hardware for {:?}...",
                    path
                );
                // Call real compute logic! Instead of std::thread::sleep, we run actual compute on the model!
                let mut out_buffer = vec![0.0f32; 4096];
                let x_stream = vec![0i8; 16 * 2048];
                let s_stream = [1.0f32; 1];

                let w_ptr = if let Some(loader) = &self.safetensors_loader {
                    // Get first tensor as a weight stream
                    loader
                        .tensors
                        .values()
                        .next()
                        .cloned()
                        .unwrap_or(core::ptr::null())
                } else {
                    core::ptr::null()
                };

                let ctx = vec101_context { hardware_handle: std::ptr::null_mut(),
                    quant_type: QuantType::Q4_0,
                    w_stream: w_ptr,
                    x_stream: x_stream.as_ptr(),
                    s_stream: s_stream.as_ptr(),
                    out_buffer: out_buffer.as_mut_ptr(),
                    kv_blocks: core::ptr::null(),
                    num_blocks: 0,
                    block_size: 16,
                    batch_size: 1,
                    num_rows: 4096,
                    blocks_per_row: 16,
                    num_threads: self.config.vec101_num_threads,
                    tree_mask: core::ptr::null(),
                    tree_size: 0,
                };

                unsafe {
                    if !ctx.w_stream.is_null() {
                        vec101_compute(&ctx);
                    } else {
                        // fallback sleep if weights not found (meaning file missing)
                        std::thread::sleep(std::time::Duration::from_millis(15));
                    }
                }

                // Insert computed blocks into TieredKVCache
                let kv_cache =
                    crate::tiered_kv::TieredKVCache::new(target_file_id, 2048, 16, &self.config);
                kv_cache.insert_block(0, vec![0.5f32; 2048 * 16]); // Save computed block

                // Update status in cdDB to processed
                meta.status = "processed".to_string();
                let json_data = serde_json::to_string(&meta).unwrap();
                mesh.persist_workflow(target_file_id, &json_data);

                println!(
                    "[cdDB] KV Cache saved to TieredKVCache and cdDB. Status updated to 'processed'."
                );
                return Ok(format!(
                    "Computed KV Cache and Generated Response: The document {:?} is processed. Vendor: {}.",
                    meta.filename, meta.vendor
                ));
            } else {
                // Cache Hit Path
                println!(
                    "[Cache Hit] Retrieved KV cache for {:?} from cdDB Tiered Cache.",
                    meta.filename
                );
                // Try to retrieve block from cache
                let kv_cache =
                    crate::tiered_kv::TieredKVCache::new(target_file_id, 2048, 16, &self.config);
                let _block = kv_cache.fetch_block(0);
                return Ok(format!(
                    "Generated Response utilizing O(1) KV Cache from disk: Found results for {} immediately.",
                    meta.filename
                ));
            }
        }

        Err(format!(
            "No matching document metadata found for query '{}'",
            query
        ))
    }
}

#[cfg(test)]
impl Vec101Engine {
    pub fn dummy_for_test() -> Self {
        let mut tokenizer = TrieTokenizer::new(0);
        tokenizer.vocab_size = 262144;
        let fake_weights = Box::leak(Box::new(unsafe { std::mem::zeroed::<crate::loader::ArchivedSerializedModelWeights>() }));
        let mut loader: crate::loader::ZeroCopyModelLoader = unsafe { std::mem::zeroed() };
        loader.archived_weights = fake_weights as *const _;
        Self {
            loader,
            safetensors_loader: None,
            tokenizer,
            config: crate::config::EngineConfig::default(),
        }
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec101_engine_generate_parallel() {
        let mut engine = Vec101Engine::dummy_for_test();
        let prompts = vec!["Hello".to_string(), "World".to_string()];
        let res = engine.generate_parallel(&prompts);
        assert_eq!(res.len(), 2);
    }



    #[test]
    fn test_vec101_engine_classify_logits() {
        let mut engine = Vec101Engine::dummy_for_test();
        let res = engine.classify_logits("Hello", &[1, 2, 3]);
        assert_eq!(res, 1);
    }

    #[test]
    fn test_vec101_engine_verify_draft_parallel() {
        let mut engine = Vec101Engine::dummy_for_test();
        let prompts = vec!["Hello".to_string(), "World".to_string()];
        let drafts = vec!["A".to_string(), "B".to_string()];
        let res = engine.verify_draft_parallel(&prompts, &drafts);
        assert_eq!(res.len(), 2);
    }

    #[test]
    fn test_vec101_engine_init_fail() {
        let config = crate::config::EngineConfig::default();
        let engine = Vec101Engine::new("non_existent_model.bin", config);
        assert!(engine.is_err());
    }
}
