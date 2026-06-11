use std::collections::HashMap;
use std::sync::Mutex;
use crate::system2_verifier::UnionAst;

/// Simulates the Self-Evolving logic: discovering O(1) macros from O(N) paths.
pub struct SelfEvolvingLoop {
    /// Tracks occurrences of specific AST execution patterns.
    /// Key: A serialized representation of the AST path. Value: Frequency count.
    path_frequencies: Mutex<HashMap<String, u32>>,
}

impl SelfEvolvingLoop {
    pub fn new() -> Self {
        Self {
            path_frequencies: Mutex::new(HashMap::new()),
        }
    }

    /// Intercepts a successful System 2 verification and records its path.
    pub fn intercept_success(&self, ast: &UnionAst) {
        // Serialize the AST to a mock string path representation
        let path_key = format!("Op:{}_Payload:{}_Args:{}", ast.opcode, ast.payload_id, ast.arguments.join(","));
        
        let mut freqs = self.path_frequencies.lock().unwrap();
        let count = freqs.entry(path_key.clone()).or_insert(0);
        *count += 1;

        println!("[Self-Evolving Loop] Intercepted successful workflow execution.");

        // If this specific workflow path is executed enough times, wrap it in a macro.
        if *count == 3 {
            println!("\n[Macro Discovery] !!! HIGH FREQUENCY PATH DETECTED !!!");
            println!("[Macro Discovery] The path [{}] has been executed 3 times.", path_key);
            println!("[Macro Discovery] Auto-generating an O(1) UnionCode Macro to replace this O(N) evaluation sequence.");
            println!("[Macro Discovery] Macro injected into DualCacheFF.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_discovery_threshold() {
        let evolver = SelfEvolvingLoop::new();
        let ast = UnionAst {
            opcode: 32,
            payload_id: 1337,
            arguments: vec!["arg1".to_string()],
        };

        // First attempt
        evolver.intercept_success(&ast);
        let freqs = evolver.path_frequencies.lock().unwrap();
        assert_eq!(freqs.values().next(), Some(&1));
        drop(freqs);

        // Second attempt
        evolver.intercept_success(&ast);
        let freqs2 = evolver.path_frequencies.lock().unwrap();
        assert_eq!(freqs2.values().next(), Some(&2));
        drop(freqs2);

        // Third attempt triggers Macro Discovery
        evolver.intercept_success(&ast);
        let freqs3 = evolver.path_frequencies.lock().unwrap();
        assert_eq!(freqs3.values().next(), Some(&3));
    }
}
