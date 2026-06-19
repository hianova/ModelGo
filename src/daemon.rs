use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::thread;
use std::fs;
use anyhow::Result;
use notify::{Watcher, RecursiveMode, EventKind, event::ModifyKind};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state: i32, event_type: u32) -> f64;
}

#[cfg(target_os = "macos")]
#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {
    fn objc_getClass(name: *const u8) -> *mut std::ffi::c_void;
    fn sel_registerName(name: *const u8) -> *mut std::ffi::c_void;
    fn objc_msgSend(obj: *mut std::ffi::c_void, sel: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

#[cfg(target_os = "macos")]
fn get_macos_thermal_state() -> i64 {
    unsafe {
        let class_ptr = objc_getClass(b"NSProcessInfo\0".as_ptr());
        if class_ptr.is_null() {
            return 0;
        }
        let sel_info = sel_registerName(b"processInfo\0".as_ptr());
        let info_ptr = objc_msgSend(class_ptr, sel_info);
        if info_ptr.is_null() {
            return 0;
        }
        let sel_state = sel_registerName(b"thermalState\0".as_ptr());
        objc_msgSend(info_ptr, sel_state) as i64
    }
}

pub static IS_PROCESSING_REQUEST: AtomicBool = AtomicBool::new(false);

pub struct HeuristicsScheduler {
    pub simulated_temp: f64,
}

impl HeuristicsScheduler {
    pub fn new() -> Self {
        Self {
            simulated_temp: 45.0,
        }
    }

    /// Update simulated temp based on workload
    pub fn update_temp(&mut self, active_workload: bool) {
        if active_workload {
            self.simulated_temp += 5.0; // Heat up
        } else {
            self.simulated_temp -= 1.0; // Cool down
        }
        self.simulated_temp = self.simulated_temp.clamp(35.0, 90.0);
    }

    pub fn is_safe_to_run(&mut self) -> bool {
        // Rule A: Check if processing user request
        if IS_PROCESSING_REQUEST.load(Ordering::SeqCst) {
            println!("[Daemon Sleep] ModelGo is processing a user request. Pausing background tasks...");
            return false;
        }

        // Rule A: Check keyboard/mouse idle time
        #[allow(unused_assignments)]
        let mut idle_seconds = 300.0;
        #[cfg(target_os = "macos")]
        {
            unsafe {
                idle_seconds = CGEventSourceSecondsSinceLastEventType(0, !0);
            }
        }
        
        if idle_seconds < 300.0 {
            println!("[Daemon Sleep] User is active (idle for {:.1}s < 300s). Pausing...", idle_seconds);
            return false;
        }

        // Rule B: CPU temperature/thermal level
        #[cfg(target_os = "macos")]
        {
            let state = get_macos_thermal_state();
            if state >= 2 { // 2 = Serious, 3 = Critical
                println!("[Daemon Sleep] CPU thermal level is serious or critical ({}). Pausing...", state);
                return false;
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            if self.simulated_temp >= 60.0 {
                println!("[Daemon Sleep] CPU Temp too high ({:.1}°C). Cooling down...", self.simulated_temp);
                return false;
            }
        }

        true
    }
}

pub struct Daemon;

#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentMetadata {
    pub filename: String,
    pub doc_type: String,
    pub vendor: String,
    pub date: String,
    pub status: String,
}

impl Daemon {
    pub fn run() -> Result<()> {
        println!("[Daemon] Starting background Heuristics Scheduler and Tagging Service...");
        
        // Ensure data directory exists
        let data_dir = PathBuf::from("./data");
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
        }

        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(&data_dir, RecursiveMode::NonRecursive)?;

        let mut scheduler = HeuristicsScheduler::new();

        loop {
            // Process any pending filesystem events
            while let Ok(event) = rx.try_recv() {
                if let Ok(ev) = event {
                    for path in ev.paths {
                        match ev.kind {
                            EventKind::Create(_) => {
                                println!("[Daemon] Detected new file: {:?}", path);
                                Self::process_new_file(&path);
                            },
                            EventKind::Modify(ModifyKind::Data(_)) => {
                                println!("[Daemon] Detected file modification: {:?}", path);
                                Self::invalidate_cache(&path);
                            },
                            _ => {}
                        }
                    }
                }
            }

            // Check if it's safe to do some background processing (mock loop)
            if scheduler.is_safe_to_run() {
                // In a real system, we would pop from a queue of unprocessed files here
                scheduler.update_temp(false); // idle cool down
            } else {
                scheduler.update_temp(false);
            }

            thread::sleep(Duration::from_secs(5));
        }
    }

    fn extract_clean_text(path: &Path) -> Result<String> {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        match ext.to_lowercase().as_str() {
            "pdf" => {
                let text = pdf_extract::extract_text(path).unwrap_or_else(|_| String::new());
                Ok(text)
            },
            "xlsx" => {
                use calamine::{Reader, open_workbook_auto, Data};
                let mut excel = open_workbook_auto(path).map_err(|e| anyhow::anyhow!("Excel error: {}", e))?;
                let mut text = String::new();
                if let Some(Ok(r)) = excel.worksheet_range_at(0) {
                    for row in r.rows() {
                        for cell in row.iter() {
                            if let Data::String(s) = cell {
                                text.push_str(s);
                                text.push(' ');
                            }
                        }
                    }
                }
                Ok(text)
            },
            _ => {
                // Default to raw string read for txt or others
                Ok(fs::read_to_string(path).unwrap_or_else(|_| String::new()))
            }
        }
    }

    fn process_new_file(path: &Path) {
        println!("[Daemon] Text Extraction Barrier active for {:?}", path);
        let text = match Self::extract_clean_text(path) {
            Ok(t) => t,
            Err(e) => {
                println!("[Daemon Error] Failed to extract text: {}", e);
                return;
            }
        };

        if text.is_empty() {
            println!("[Daemon Warning] Extracted text is empty. Skipping.");
            return;
        }

        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let filename_lower = filename.to_lowercase();
        let text_lower = text.to_lowercase();
        
        // Dynamic heuristic metadata extraction
        let doc_type = if filename_lower.contains("contract") || text_lower.contains("contract") || text_lower.contains("agreement") {
            "contract".to_string()
        } else if filename_lower.contains("invoice") || text_lower.contains("invoice") {
            "invoice".to_string()
        } else {
            "report".to_string()
        };

        let vendor = if filename_lower.contains("supplier_a") || text_lower.contains("supplier a") {
            "Supplier A".to_string()
        } else if filename_lower.contains("supplier_b") || text_lower.contains("supplier b") {
            "Supplier B".to_string()
        } else if text_lower.contains("google") {
            "Google".to_string()
        } else if text_lower.contains("microsoft") {
            "Microsoft".to_string()
        } else {
            "Unknown Vendor".to_string()
        };

        let date = {
            let mut found_year = "2026".to_string();
            let chars: Vec<char> = filename.chars().chain(text.chars().take(1000)).collect();
            if chars.len() >= 4 {
                for i in 0..chars.len() - 3 {
                    if chars[i].is_ascii_digit() && chars[i+1].is_ascii_digit() && chars[i+2].is_ascii_digit() && chars[i+3].is_ascii_digit() {
                        let year: String = chars[i..i+4].iter().collect();
                        if let Ok(y) = year.parse::<u32>() {
                            if (2000..=2099).contains(&y) {
                                found_year = year;
                                break;
                            }
                        }
                    }
                }
            }
            found_year
        };

        let metadata = DocumentMetadata {
            filename: filename.clone(),
            doc_type,
            vendor,
            date,
            status: "unprocessed".to_string(),
        };

        let _preview: String = text.chars().take(500).collect();
        println!("[Daemon] Left Brain (1.58-bit) analyzing 500-char preview...");
        
        // Execute 0-Token generation logit classification to eliminate decoding overhead
        let engine = crate::router::get_fallback_engine();
        // Assume candidate tokens [1, 2, 3] map to Document Categories (e.g., Contract, Invoice, Report)
        let _ = engine.classify_logits(&format!("Classify Document Type: {}", _preview), &[1, 2, 3]);

        // Write to cdDB
        let mesh = crate::memory_mesh::MemoryMesh::global();
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        metadata.filename.hash(&mut hasher);
        let file_id = hasher.finish() as u32;

        let json_data = serde_json::to_string(&metadata).unwrap();
        mesh.persist_workflow(file_id, &json_data);
        
        println!("[Daemon] Successfully stored metadata tag in cdDB: {}", json_data);
    }

    fn invalidate_cache(path: &Path) {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let mut hasher = DefaultHasher::new();
        filename.hash(&mut hasher);
        let session_id = hasher.finish() as u32;

        println!("[Daemon] Cache Invalidation Triggered! Purging KV Blocks for {:?}", filename);
        let kv_cache = crate::tiered_kv::TieredKVCache::new(session_id, 2048, 16, &crate::config::EngineConfig::default());
        kv_cache.invalidate_blocks(0, 100);

        // Extract metadata dynamically from the modified file
        let text = Self::extract_clean_text(path).unwrap_or_default();
        let filename_lower = filename.to_lowercase();
        let text_lower = text.to_lowercase();
        
        let doc_type = if filename_lower.contains("contract") || text_lower.contains("contract") || text_lower.contains("agreement") {
            "contract".to_string()
        } else if filename_lower.contains("invoice") || text_lower.contains("invoice") {
            "invoice".to_string()
        } else {
            "report".to_string()
        };

        let vendor = if filename_lower.contains("supplier_a") || text_lower.contains("supplier a") {
            "Supplier A".to_string()
        } else if filename_lower.contains("supplier_b") || text_lower.contains("supplier b") {
            "Supplier B".to_string()
        } else if text_lower.contains("google") {
            "Google".to_string()
        } else {
            "Unknown Vendor".to_string()
        };

        let date = {
            let mut found_year = "2026".to_string();
            let chars: Vec<char> = filename.chars().chain(text.chars().take(1000)).collect();
            if chars.len() >= 4 {
                for i in 0..chars.len() - 3 {
                    if chars[i].is_ascii_digit() && chars[i+1].is_ascii_digit() && chars[i+2].is_ascii_digit() && chars[i+3].is_ascii_digit() {
                        let year: String = chars[i..i+4].iter().collect();
                        if let Ok(y) = year.parse::<u32>() {
                            if (2000..=2099).contains(&y) {
                                found_year = year;
                                break;
                            }
                        }
                    }
                }
            }
            found_year
        };

        // Downgrade status to unprocessed
        let metadata = DocumentMetadata {
            filename: filename.clone(),
            doc_type,
            vendor,
            date,
            status: "unprocessed".to_string(),
        };

        let mesh = crate::memory_mesh::MemoryMesh::global();
        let json_data = serde_json::to_string(&metadata).unwrap();
        mesh.persist_workflow(session_id, &json_data);
        println!("[Daemon] Metadata downgraded to 'unprocessed' for {:?}", filename);
    }
}
