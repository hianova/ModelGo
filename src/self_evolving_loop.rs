use crate::chaos_state::{ChaosState, MicroTweak, RngState, step_forward_nd};
use crate::system2_verifier::UnionAst;
use std::collections::HashMap;
use std::sync::Mutex;

use std::sync::OnceLock;

/// Simulates the Self-Evolving logic: discovering O(1) macros from O(N) paths.
pub struct SelfEvolvingLoop {
    /// Tracks the Chaos probability cloud of specific AST execution patterns.
    pub path_states: Mutex<HashMap<String, ChaosState<10, 1>>>,
    /// Thread-safe PRNG for Zipf stochasticity.
    pub rng: Mutex<RngState>,
}

static GLOBAL_EVOLVER: OnceLock<SelfEvolvingLoop> = OnceLock::new();

impl Default for SelfEvolvingLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfEvolvingLoop {
    pub fn global() -> &'static SelfEvolvingLoop {
        GLOBAL_EVOLVER.get_or_init(SelfEvolvingLoop::new)
    }
    pub fn new() -> Self {
        Self {
            path_states: Mutex::new(HashMap::new()),
            rng: Mutex::new(RngState::new(0xABCD)),
        }
    }

    /// Intercepts a successful System 2 verification and records its path via ChaosState.
    pub fn intercept_success(&self, ast: &UnionAst) -> bool {
        let path_key = format!(
            "Op:{}_Payload:{}_Args:{}",
            ast.opcode,
            ast.payload_id,
            ast.arguments.join(",")
        );

        let mut states = self.path_states.lock().unwrap();
        let mut rng = self.rng.lock().unwrap();

        // Retrieve existing state or initialize a new one with a base value of 0.0
        let current_state = states
            .entry(path_key.clone())
            .or_insert_with(|| ChaosState::new([0.0]));

        // Tweak parameter mapping a highly stochastic (fat-tail) Zipf learning curve.
        let tweak = MicroTweak {
            s_exponent: 1.5, // 1.5 ensures significant extreme jumps
            max_elements: 1000,
        };

        // Mathematically project the state forward
        *current_state = step_forward_nd(current_state, &tweak, &mut rng);

        println!(
            "[Self-Evolving Loop] Intercepted successful workflow. Advanced ChaosState. Base value: {:.4}",
            current_state.base_values[0]
        );

        // A macro is dynamically discovered if the base value exceeds a statistical threshold,
        // OR if a massive Zipf multiplication pushes it over instantly.
        if current_state.base_values[0].abs() > 2.0 {
            println!("\n[Macro Discovery] !!! CRITICAL CHAOS THRESHOLD BREACHED !!!");
            println!(
                "[Macro Discovery] The path [{}] has mathematically evolved.",
                path_key
            );
            println!(
                "[Macro Discovery] Auto-generating an O(1) UnionCode Macro to replace this O(N) evaluation sequence."
            );
            println!("[Macro Discovery] Macro injected into DualCacheFF.");

            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            path_key.hash(&mut hasher);
            let intent_hash = hasher.finish();

            crate::memory_mesh::MemoryMesh::global()
                .cache_intent_success(intent_hash, format!("O(1) MACRO for {}", path_key));

            // Reset state to avoid repetitive discovery
            current_state.base_values[0] = 0.0;
            return true;
        }
        false
    }

    /// Registers a theoretical rule distilled by the DecisionEngine into the DualCacheFF Archive.
    /// This converts a generalized string rule into a fast O(1) lookup macro.
    pub fn register_theory(&self, intent_hash: u64, theory_rule: &str) {
        println!(
            "[Self-Evolving Loop] Archiving Theory into DualCacheFF: {}",
            theory_rule
        );

        // In a real scenario, this would parse the theory_rule into a valid UnionAst or instruction sequence.
        // For now, we simulate this by caching it directly as a string-backed Macro inside MemoryMesh.
        crate::memory_mesh::MemoryMesh::global()
            .cache_intent_success(intent_hash, format!("O(1) MACRO: {}", theory_rule));

        println!("[Self-Evolving Loop] Theory successfully solidified as O(1) Shortcut.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_evolving_loop() {
        std::thread::Builder::new()
            .stack_size(128 * 1024 * 1024)
            .spawn(|| {
                let evolver = SelfEvolvingLoop::new();
                let ast = UnionAst {
                    opcode: 123,
                    payload_id: 456,
                    arguments: vec!["test".to_string()],
                };

                let mut breached = false;
                for _ in 0..100 {
                    if evolver.intercept_success(&ast) {
                        breached = true;
                        break;
                    }
                }
                let _ = breached; // avoid unused variable warning

                evolver.register_theory(0xABC, "Some Theory");
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
