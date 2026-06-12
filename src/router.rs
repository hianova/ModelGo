use anyhow::Result;
use union_code::{CompressedIntent, UnionCode};
use dualcache_ff::static_cache::static_cache::StaticDualCache;
use dualcache_ff::config::Config;
use serde_json::Value;

/// The core routing trait for resolving intents.
pub trait IntentRouter {
    /// Routes the incoming byte stream to a deterministic CompressedIntent and dynamic parameters.
    /// Returns Err(0x06) if the intent is not recognized and should fall back.
    fn route(&self, input: &[u8]) -> Result<(CompressedIntent, Option<Value>), u8>;
}

/// The ultra-fast L0 engine using UnionCode.
pub struct UnionCodeEngine<'a> {
    uc: UnionCode<'a, StaticDualCache<u32, CompressedIntent, 64>>,
}

impl<'a> UnionCodeEngine<'a> {
    pub fn new() -> Self {
        // We use a small memory footprint static dual cache for embedded/high-perf.
        let config = Config::with_memory_budget(1, 100);
        let cache = StaticDualCache::<u32, CompressedIntent, 64>::new(config);
        
        Self {
            uc: UnionCode::new(cache),
        }
    }
}

impl<'a> IntentRouter for UnionCodeEngine<'a> {
    #[inline(always)]
    fn route(&self, input: &[u8]) -> Result<(CompressedIntent, Option<Value>), u8> {
        self.uc.decode(input).map(|i| (i, None))
    }
}

use std::process::{Command, Stdio};
use std::io::{Write, BufReader, BufRead};
use std::sync::Mutex;

/// The intelligent L1 fallback engine powered by vec101 1bitLLM python bridge
pub struct Vec101FallbackEngine {
    bridge: Mutex<Option<(std::process::ChildStdin, BufReader<std::process::ChildStdout>)>>,
}

impl Vec101FallbackEngine {
    pub fn new() -> Self {
        println!("[Vec101FallbackEngine] Spawning 1bitLLM inference bridge...");
        
        let mut child = Command::new("python3")
            .env("PYTHONWARNINGS", "ignore")
            .current_dir("../vec101/tools/bitnet_b1_58-large")
            .arg("inference_bridge.py")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Failed to start vec101 inference bridge");

        let stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");
        let reader = BufReader::new(stdout);

        Self {
            bridge: Mutex::new(Some((stdin, reader))),
        }
    }
}

impl IntentRouter for Vec101FallbackEngine {
    fn route(&self, input: &[u8]) -> Result<(CompressedIntent, Option<Value>), u8> {
        let input_str = String::from_utf8_lossy(input);
        println!("\n[L1 Fallback] UnionCode L0 Missed. Waking up vec101 1bitLLM to analyze: {}", input_str);

        let mut lock = self.bridge.lock().unwrap();
        if let Some((stdin, stdout)) = lock.as_mut() {
            
            // Generate mmap path for dynamic parameter passing
            let tmp_dir = std::env::temp_dir();
            let mmap_path = tmp_dir.join(format!("modelgo_mmap_{}.json", std::process::id()));
            let _ = std::fs::write(&mmap_path, b"");

            let req = serde_json::json!({
                "intent": input_str.to_string(),
                "mmap_path": mmap_path.to_str().unwrap()
            });
            let req_str = req.to_string() + "\n";
            
            if let Err(e) = stdin.write_all(req_str.as_bytes()) {
                eprintln!("Failed to write to bridge: {}", e);
                return Err(0x06);
            }
            if let Err(e) = stdin.flush() {
                eprintln!("Failed to flush bridge: {}", e);
                return Err(0x06);
            }

            let mut response = String::new();
            if let Err(e) = stdout.read_line(&mut response) {
                eprintln!("Failed to read from bridge: {}", e);
                return Err(0x06);
            }

            if response.trim().is_empty() {
                return Err(0x06);
            }

            if let Ok(mut val) = serde_json::from_str::<Value>(&response) {
                if val["status"] == "mmap_written" {
                    // Read parameters directly from the shared memory / mmap backed file
                    if let Ok(mmap_data) = std::fs::read(&mmap_path) {
                        let json_str = String::from_utf8_lossy(&mmap_data);
                        let json_str = json_str.trim_end_matches('\0');
                        if let Ok(mmap_val) = serde_json::from_str::<Value>(json_str) {
                            val = mmap_val;
                        }
                    }
                }

                let opcode = val["opcode"].as_u64().unwrap_or(0) as u8;
                let payload_id = val["payload_id"].as_u64().unwrap_or(0) as u16;
                let parameters = val.get("parameters").cloned();
                
                println!("[1bitLLM] Recognized: opcode=0x{:02X}, parameters={:?}", opcode, parameters);
                
                return Ok((CompressedIntent { opcode, payload_id }, parameters));
            }
        }

        Err(0x06)
    }
}

impl Vec101FallbackEngine {
    pub fn generate_script(&self, prompt_text: &str) -> Result<String, String> {
        let mut lock = self.bridge.lock().unwrap();
        if let Some((stdin, stdout)) = lock.as_mut() {
            let tmp_dir = std::env::temp_dir();
            let mmap_path = tmp_dir.join(format!("modelgo_mmap_script_{}.json", std::process::id()));
            let _ = std::fs::write(&mmap_path, b"");

            let req = serde_json::json!({
                "intent": prompt_text,
                "task_type": "generate_bash",
                "mmap_path": mmap_path.to_str().unwrap()
            });
            let req_str = req.to_string() + "\n";
            
            if let Err(e) = stdin.write_all(req_str.as_bytes()) {
                return Err(format!("Failed to write to bridge: {}", e));
            }
            if let Err(e) = stdin.flush() {
                return Err(format!("Failed to flush bridge: {}", e));
            }

            let mut response = String::new();
            if let Err(e) = stdout.read_line(&mut response) {
                return Err(format!("Failed to read from bridge: {}", e));
            }

            if response.trim().is_empty() {
                return Err("Empty response from bridge".to_string());
            }

            if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&response) {
                if val["status"] == "mmap_written" {
                    if let Ok(mmap_data) = std::fs::read(&mmap_path) {
                        let json_str = String::from_utf8_lossy(&mmap_data);
                        let json_str = json_str.trim_end_matches('\0');
                        if let Ok(mmap_val) = serde_json::from_str::<serde_json::Value>(json_str) {
                            val = mmap_val;
                        }
                    }
                }

                if let Some(script) = val.get("script").and_then(|s| s.as_str()) {
                    return Ok(script.to_string());
                } else if let Some(err) = val.get("error").and_then(|e| e.as_str()) {
                    return Err(err.to_string());
                }
            }
        }
        Err("Failed to communicate with bridge".to_string())
    }
}

use std::sync::OnceLock;

static FALLBACK_ENGINE: OnceLock<Vec101FallbackEngine> = OnceLock::new();

pub fn get_fallback_engine() -> &'static Vec101FallbackEngine {
    FALLBACK_ENGINE.get_or_init(|| Vec101FallbackEngine::new())
}

/// The Hybrid Router unifying L0 and L1
pub struct HybridRouter<'a> {
    fast_path: UnionCodeEngine<'a>,
}

impl<'a> HybridRouter<'a> {
    pub fn new() -> Self {
        Self {
            fast_path: UnionCodeEngine::new(),
        }
    }
}

impl<'a> IntentRouter for HybridRouter<'a> {
    #[inline(always)]
    fn route(&self, input: &[u8]) -> Result<(CompressedIntent, Option<Value>), u8> {
        match self.fast_path.route(input) {
            Ok(intent) => Ok(intent), // 28ns ~ 148ns execution
            Err(0x06) => get_fallback_engine().route(input), // Fallback to LLM execution
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_union_code_engine_route() {
        let engine = UnionCodeEngine::new();
        
        // Test basic valid route that should be handled by the fast path (FST / Cache)
        let result = engine.route("請幫我拿咖啡".as_bytes());
        assert_eq!(
            result,
            Ok((CompressedIntent { opcode: 0x20, payload_id: 0x0A42 }, None))
        );
        
        // Test fallback error for unmapped intents
        let miss_result = engine.route("一個完全沒有見過的未知指令".as_bytes());
        assert_eq!(miss_result, Err(0x06)); // 0x06 is NotFound
    }
}
