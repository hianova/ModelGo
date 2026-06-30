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

        let parts: Vec<&str> = draft_output.trim().split('|').collect();
        if parts.len() < 2 {
            bail!("Syntax Error: Expected at least OpCode|PayloadID");
        }

        let opcode = parts[0]
            .parse::<u8>()
            .map_err(|e| anyhow::anyhow!("Invalid OpCode: {}", e))?;
        let payload_id = parts[1]
            .parse::<u32>()
            .map_err(|e| anyhow::anyhow!("Invalid PayloadID: {}", e))?;

        if opcode == 0 {
            bail!("Invalid Opcode 0 provided. Must be non-zero.");
        }

        let arguments = parts.into_iter().skip(2).map(|s| s.to_string()).collect();

        let ast = UnionAst {
            opcode,
            payload_id,
            arguments,
        };

        println!("[Union Parser] Success. AST Validated: {:?}", ast);
        Ok(ast)
    }

    /// Rejection Sampling Loop: Wraps vec101 calls and retries on failure.
    pub fn execute_with_rejection_sampling(
        initial_prompt: &str,
        max_retries: u32,
    ) -> Result<UnionAst> {
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
                    _current_prompt = format!(
                        "{}\n\nPREVIOUS ERROR:\n{}\nFix the syntax to follow the pipe-separated format: OpCode|PayloadID|Arg1...",
                        _current_prompt, e
                    );
                }
            }
        }

        bail!(
            "Failed to generate a valid AST after {} attempts",
            max_retries
        )
    }
}
