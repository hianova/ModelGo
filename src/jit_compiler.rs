use anyhow::{Result, Context};
use std::process::Command;
use std::fs;
use std::env;
use serde_json::Value;

/// Simulates the JIT compilation of a natural language prompt into an executable script.
/// Implements a Self-Healing ReAct loop using vec101.
pub struct JitCompiler;

impl JitCompiler {
    /// Takes a natural language prompt and optional dynamic parameters.
    /// Generates a bash script via vec101. If the script fails, fetches `--help` 
    /// and tries again (ReAct loop).
    pub fn compile_and_execute(prompt: &str, params: Option<Value>) -> Result<()> {
        println!("\n[JIT Compiler] Received Natural Language Task: \"{}\"", prompt);
        if let Some(p) = &params {
            println!("[JIT Compiler] Provided Dynamic Parameters via IPC: {:?}", p);
        }

        let mut current_prompt = format!(
            "Generate a pure bash script to accomplish this task:\nTask: {}\nParameters: {:?}\n\nOutput only the bash script inside a ```bash block.",
            prompt, params
        );

        let max_retries = 2;
        
        for attempt in 1..=max_retries {
            println!("\n[JIT Compiler] Attempt {}/{} - Transpiling to native script via vec101...", attempt, max_retries);
            
            // Invoke the real 1bitLLM!
            let script_content = match crate::router::get_fallback_engine().generate_script(&current_prompt) {
                Ok(script) => {
                    println!("[JIT Compiler] Received script from LLM.");
                    script
                }
                Err(e) => {
                    eprintln!("[JIT Compiler Error] Failed to generate script from vec101: {}", e);
                    continue; // Skip to next attempt or fail
                }
            };

            // Write the script to a temporary executable file
            let mut temp_dir = env::temp_dir();
            temp_dir.push(format!("modelgo_jit_script_{}.sh", std::process::id()));
            
            fs::write(&temp_dir, &script_content)
                .context("Failed to write JIT script")?;

            // Ensure the script is executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&temp_dir)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&temp_dir, perms)?;
            }

            println!("[JIT Compiler] Script compiled to {}. Executing...", temp_dir.display());

            let output = Command::new(&temp_dir).output().context("Failed to execute JIT script")?;

            if output.status.success() {
                println!("{}", String::from_utf8_lossy(&output.stdout));
                
                // Persistence: Store the successful script in MemoryMesh (cdDB)
                println!("[JIT Compiler] Script execution SUCCESS. Persisting workflow to MemoryMesh cdDB Tiered Storage...");
                let mesh = crate::memory_mesh::MemoryMesh::global();
                let workflow_id = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() % 10000) as u32;
                mesh.persist_workflow(workflow_id, &script_content);
                
                return Ok(());
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("[JIT Compiler Error] Execution failed: {}", stderr);
                
                // --- Self-Healing ReAct Loop ---
                // Extract the failing command
                let failing_cmd = if stderr.contains("command not found") {
                    let parts: Vec<&str> = stderr.split(':').collect();
                    if parts.len() >= 3 {
                        // "script.sh: line 4: df: command not found" -> parts[2] is " df"
                        parts[parts.len() - 2].trim().split(' ').next().unwrap_or("").to_string()
                    } else {
                        stderr.split(' ').next().unwrap_or("unknown").to_string()
                    }
                } else {
                    "unknown".to_string()
                };
                
                println!("[JIT Compiler ReAct] Attempting to learn usage of '{}' via --help...", failing_cmd);
                let help_output = Command::new("bash").arg("-c").arg(format!("{} --help", failing_cmd)).output();
                let help_text = match help_output {
                    Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
                    Err(_) => "No manual entry available.".to_string(),
                };
                
                current_prompt = format!(
                    "Your previous bash script failed with error:\n{}\n\nHere is the manual for '{}':\n{}\n\nPlease fix the bash script.",
                    stderr, failing_cmd, help_text
                );
            }
        }

        anyhow::bail!("JIT compilation and execution failed after retries.")
    }
}
