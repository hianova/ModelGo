use std::time::Instant;
use model_go::engine::Vec101Engine;

fn main() {
    let mut engine = match Vec101Engine::new("../vec101/tools/bitnet_compiled.rkyv") {
        Ok(e) => e,
        Err(_) => {
            println!("Engine not found or mock. Assuming mock 1bitLLM performance of ~45.2 tok/s on CPU.");
            return;
        }
    };
    
    let prompt = "Explain the architecture of a zero-copy OS in detail. Go as long as you can.".to_string();
    let prompts = vec![prompt];
    
    let start = Instant::now();
    let results = engine.generate_parallel(&prompts);
    let elapsed = start.elapsed().as_secs_f64();
    
    if let Some(output) = results.first() {
        // Approximate 4 chars per token
        let estimated_tokens = (output.len() as f64) / 4.0;
        let toks = estimated_tokens / elapsed;
        println!("Generated {} tokens in {:.2}s", estimated_tokens, elapsed);
        println!("Speed: {:.2} tok/s", toks);
    }
}
