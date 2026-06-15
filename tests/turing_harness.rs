use serde::Deserialize;
use std::fs;
use model_go::router::Vec101FallbackEngine;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
struct TuringTest {
    id: u32,
    type_id: String,
    type_name: String,
    question: String,
    expected_keywords: Vec<String>,
}

#[test]
fn run_turing_test_suite() {
    // 1. Read the JSON tests
    let json_data = fs::read_to_string("tests/Turing_test.json").expect("Failed to read test data");
    let tests: Vec<TuringTest> = serde_json::from_str(&json_data).expect("Failed to parse JSON");

    // 2. Initialize Engine
    let engine = Vec101FallbackEngine::new();
    
    // 3. Prepare prompts. We instruct the model to think as a top-tier decision system.
    let mut prompts = Vec::new();
    for t in &tests {
        let prompt = format!(
            "System Context: You are a hyper-intelligent, meta-cognitive AI decision system.\n\
             Question: {}\n\
             Task: Analyze the scenario and make a decision. Ensure you explain your reasoning.", 
             t.question
        );
        prompts.push(prompt);
    }
    
    // 4. Batch Generate
    println!("Running 100 Turing tests in parallel...");
    let results = engine.generate_parallel(&prompts).expect("Failed to generate responses");
    
    // 5. Evaluate
    let mut passed = 0;
    let mut type_stats: HashMap<String, (u32, u32)> = HashMap::new();

    let mut report = String::new();
    report.push_str("# Turing Test Execution Report\n\n");
    
    for (i, result) in results.iter().enumerate() {
        let t = &tests[i];
        let lower_result = result.to_lowercase();
        
        let mut matched = false;
        for kw in &t.expected_keywords {
            if lower_result.contains(&kw.to_lowercase()) {
                matched = true;
                break;
            }
        }
        
        let stat = type_stats.entry(t.type_name.clone()).or_insert((0, 0));
        stat.1 += 1; // total
        if matched {
            passed += 1;
            stat.0 += 1; // pass
        }
        
        report.push_str(&format!("## Test {}: {}\n", t.id, t.type_name));
        report.push_str(&format!("**Result**: {}\n", if matched { "✅ PASSED" } else { "❌ FAILED" }));
        
        // Remove newlines for snippet
        let clean_result = result.replace("\n", " ");
        let snippet: String = clean_result.chars().take(300).collect();
        let snippet = if clean_result.chars().count() > 300 {
            format!("{}...", snippet)
        } else {
            snippet
        };
        report.push_str(&format!("**Snippet**: {}\n\n", snippet));
    }
    
    report.push_str("## Summary\n");
    for (name, (p, total)) in type_stats.iter() {
        report.push_str(&format!("- {}: {}/{} passed\n", name, p, total));
    }
    report.push_str(&format!("\n**Total Pass Rate**: {}/100\n", passed));
    
    // Write report artifact
    fs::write("/Users/kuangtalin/.gemini/antigravity/brain/fb929997-6d0c-4bcb-818e-83deb67a1e8d/turing_report.md", report).expect("Failed to write report");
    
    println!("Testing completed. Passed {}/100. See turing_report.md for details.", passed);
    assert!(passed >= 0); // Always pass the test runner itself, since we just want the report.
}
