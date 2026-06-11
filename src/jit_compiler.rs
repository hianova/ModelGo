use anyhow::{Result, Context};
use std::process::Command;
use std::fs;
use std::env;

/// Simulates the JIT compilation of a natural language prompt into an executable script.
pub struct JitCompiler;

impl JitCompiler {
    /// Takes a natural language prompt, sends it to vec101 (mocked here),
    /// generates a script, and executes it.
    pub fn compile_and_execute(prompt: &str) -> Result<()> {
        println!("\n[JIT Compiler] Received Natural Language Task: \"{}\"", prompt);
        println!("[JIT Compiler] Transpiling to native script via vec101...");

        // Mocking the vec101 transpilation phase:
        // In a real scenario, we would use LlmBackend to prompt the local LLM
        // to write a bash or python script for the workflow requested.
        
        let script_content = if prompt.contains("invoice") || prompt.contains("發票") {
            r#"#!/usr/bin/env bash
echo "[Executing Native Script] Mocking downloading invoices to ~/Documents/Invoices..."
mkdir -p ~/Documents/Invoices
echo "invoice_content_2026.pdf" > ~/Documents/Invoices/invoice_1.txt
echo "[Executing Native Script] Completed at $(date)"
"#
        } else {
            r#"#!/usr/bin/env bash
echo "[Executing Native Script] Running generic background automation task..."
echo "[Executing Native Script] Task finished successfully."
"#
        };

        // Write the script to a temporary executable file
        let mut temp_dir = env::temp_dir();
        temp_dir.push("modelgo_jit_script.sh");
        
        fs::write(&temp_dir, script_content)
            .context("Failed to write JIT script")?;

        // Ensure the script is executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_dir)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&temp_dir, perms)?;
        }

        println!("[JIT Compiler] Script compiled to {}. Executing at native speed...", temp_dir.display());

        let output = Command::new(&temp_dir)
            .output()
            .context("Failed to execute JIT script")?;

        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        } else {
            eprintln!("[JIT Compiler Error] {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }
}
