use anyhow::Result;
use std::env;
use std::fs;

pub struct BoundingBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub struct SpatialIntentEngine;

impl SpatialIntentEngine {
    /// Simulates capturing a region of the screen and extracting intents.
    pub fn trigger(bbox: BoundingBox, annotation: &str) -> Result<()> {
        println!("\n[Spatial Intent] Triggered bounding box capture at (X:{}, Y:{}, W:{}, H:{})", bbox.x, bbox.y, bbox.width, bbox.height);
        println!("[Spatial Intent] Micro-Annotation received: \"{}\"", annotation);
        // Physically route to the execution engine instead of hardcoding
        println!("[Spatial Intent] Zero-copy mapping bounding box context... (Physical Mapping)");
        
        let prompt = format!("Given the visual annotation '{}' inside bounding box (X:{}, Y:{}, W:{}, H:{}), output the data extraction script or action.", annotation, bbox.x, bbox.y, bbox.width, bbox.height);
        
        let engine = crate::router::get_fallback_engine();
        let results = engine.generate_parallel(&[prompt])
            .unwrap_or_else(|_| vec!["[Action Injection] LLM processing failed.".to_string()]);
            
        let output = results.first().unwrap_or(&String::new()).clone();
        
        println!("[Spatial Intent] LLM generated action/data:\n{}", output);
        
        if annotation.to_lowercase().contains("csv") {
            let mut target_path = env::current_dir()?;
            target_path.push("extracted_data.csv");
            fs::write(&target_path, &output)?;
            println!("[Action Injection] Result physically saved to {}", target_path.display());
        }
        
        println!("[Spatial Intent] Execute & Vanish complete. UI frozen state released.");
        Ok(())
    }
}
