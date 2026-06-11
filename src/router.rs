use anyhow::Result;
use union_code::{CompressedIntent, UnionCode};
use dualcache_ff::static_cache::static_cache::StaticDualCache;
use dualcache_ff::config::Config;

/// The core routing trait for resolving intents.
pub trait IntentRouter {
    /// Routes the incoming byte stream to a deterministic CompressedIntent.
    /// Returns Err(0x06) if the intent is not recognized and should fall back.
    fn route(&self, input: &[u8]) -> Result<CompressedIntent, u8>;
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
    fn route(&self, input: &[u8]) -> Result<CompressedIntent, u8> {
        self.uc.decode(input)
    }
}

/// The intelligent L1 fallback engine powered by vec101 (or local LLM via UnionTranslator API)
pub struct Vec101FallbackEngine;

impl Vec101FallbackEngine {
    pub fn new() -> Self {
        Self
    }
}

impl IntentRouter for Vec101FallbackEngine {
    fn route(&self, input: &[u8]) -> Result<CompressedIntent, u8> {
        // In a full implementation, this would:
        // 1. Send the input byte string to the local vec101 instance or UnionTranslator endpoint.
        // 2. Transpile/Process using a 1B-3B model.
        // 3. Output the exact opcode and payload_id.
        // For the sake of this mock/demo, we simulate a latency and return a default.
        println!("\n[L1 Fallback] UnionCode L0 Missed. Waking up vec101 to analyze: {}", String::from_utf8_lossy(input));
        
        // Let's pretend it figured out it's a general task
        // OpCode 0x20 = Task, Payload 0x1337 = Custom
        Ok(CompressedIntent {
            opcode: 0x20,
            payload_id: 0x1337,
        })
    }
}

/// The Hybrid Router unifying L0 and L1
pub struct HybridRouter<'a> {
    fast_path: UnionCodeEngine<'a>,
    slow_path: Vec101FallbackEngine,
}

impl<'a> HybridRouter<'a> {
    pub fn new() -> Self {
        Self {
            fast_path: UnionCodeEngine::new(),
            slow_path: Vec101FallbackEngine::new(),
        }
    }
}

impl<'a> IntentRouter for HybridRouter<'a> {
    #[inline(always)]
    fn route(&self, input: &[u8]) -> Result<CompressedIntent, u8> {
        match self.fast_path.route(input) {
            Ok(intent) => Ok(intent), // 28ns ~ 148ns execution
            Err(0x06) => self.slow_path.route(input), // Fallback to 235 tok/s LLM execution
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_engine() {
        let engine = Vec101FallbackEngine::new();
        let intent = engine.route(b"unknown command").unwrap();
        // The mock fallback engine should return OpCode 0x20 and Payload 0x1337
        assert_eq!(intent.opcode, 0x20);
        assert_eq!(intent.payload_id, 0x1337);
    }

    #[test]
    fn test_hybrid_router_fallback() {
        let router = HybridRouter::new();
        // Since the static cache in UnionCode is empty by default in tests,
        // it should yield an Err(0x06) which triggers the fallback engine.
        let intent = router.route(b"unknown intent").unwrap();
        assert_eq!(intent.opcode, 0x20);
        assert_eq!(intent.payload_id, 0x1337);
    }
}
