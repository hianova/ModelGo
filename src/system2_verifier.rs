use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Simplified UnionCode AST schema.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct UnionAst {
    pub opcode: u8,
    pub payload_id: u32,
    pub arguments: Vec<String>,
}

pub struct System2Verifier;

impl System2Verifier {
    /// Simulates the parsing and validation of a drafted response from vec101.
    pub fn parse_and_verify(draft_output: &str) -> Result<UnionAst> {
        println!("[Union Parser] Verifying draft output...");
        
        match serde_json::from_str::<UnionAst>(draft_output) {
            Ok(ast) => {
                if ast.opcode == 0 {
                    bail!("Invalid Opcode 0 provided. Must be non-zero.");
                }
                println!("[Union Parser] Success. AST Validated: {:?}", ast);
                Ok(ast)
            }
            Err(e) => {
                bail!("Syntax Error: {}", e)
            }
        }
    }

    /// Rejection Sampling Loop: Wraps vec101 calls and retries on failure.
    pub fn execute_with_rejection_sampling(initial_prompt: &str, max_retries: u32) -> Result<UnionAst> {
        let mut _current_prompt = initial_prompt.to_string();

        for attempt in 1..=max_retries {
            println!("\n[System 2] Attempt {}/{}", attempt, max_retries);
            
            let engine = crate::router::get_fallback_engine();
            let prompts = vec![_current_prompt.clone()];
            let results = engine.generate_parallel(&prompts);

            let draft_output = match results {
                Ok(res) => res.first().cloned().unwrap_or_default(),
                Err(e) => {
                    println!("[System 2] Engine Failed: {}", e);
                    bail!("Engine failed: {}", e);
                }
            };

            println!("[LLM Native] Generated draft: {}", draft_output);

            match Self::parse_and_verify(&draft_output) {
                Ok(ast) => return Ok(ast),
                Err(e) => {
                    println!("[System 2] Validation Failed: {}", e);
                    println!("[Rejection Sampling] Injecting Error Trace into prompt for retry.");
                    // Append error trace for the next iteration
                    _current_prompt = format!("{}\n\nPREVIOUS ERROR:\n{}\nFix the JSON syntax and logic.", _current_prompt, e);
                }
            }
        }

        bail!("Failed to generate a valid AST after {} attempts", max_retries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_ast() {
        let json = r#"{"opcode": 32, "payload_id": 1337, "arguments": ["test"]}"#;
        let ast = System2Verifier::parse_and_verify(json).unwrap();
        assert_eq!(ast.opcode, 32);
        assert_eq!(ast.payload_id, 1337);
        assert_eq!(ast.arguments, vec!["test"]);
    }

    #[test]
    fn parse_rejects_opcode_zero() {
        let json = r#"{"opcode": 0, "payload_id": 1337, "arguments": []}"#;
        assert!(System2Verifier::parse_and_verify(json).is_err());
    }

    #[test]
    fn parse_rejects_invalid_json() {
        let json = r#"{"opcode": 32, "payload_id": 1337, "arguments": [test]}"#;
        assert!(System2Verifier::parse_and_verify(json).is_err());
    }

    #[test]
    fn rejection_sampling_succeeds_within_retries() {
        let result = System2Verifier::execute_with_rejection_sampling("test prompt", 3);
        assert!(result.is_ok());
    }

    #[test]
    fn rejection_sampling_fails_with_zero_retries() {
        let result = System2Verifier::execute_with_rejection_sampling("test prompt", 0);
        assert!(result.is_err());
    }
}
