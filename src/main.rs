use clap::{Parser, Subcommand};
use model_go::{
    HybridRouter, IntentRouter, JitCompiler, SpatialIntentEngine, BoundingBox, 
    ZeroCopyMmapReader, System2Verifier, OsDispatch, SelfEvolvingLoop,
    BenchmarkSuite, FftChaosAnalyzer
};

#[derive(Parser)]
#[command(name = "ModelGo")]
#[command(about = "The Ultimate OS-Level Neural Interface integrating UnionCode and vec101", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Zero-copy read a file into memory
    Mmap {
        #[arg(short, long)]
        file: String,
    },
    /// Route an intent using the Hybrid Router (UnionCode + vec101)
    Route {
        #[arg(short, long)]
        text: String,
    },
    /// Compile a Natural Language prompt to native script and execute it
    Jit {
        #[arg(short, long)]
        prompt: String,
    },
    /// Emulate Spatial Intent extraction
    Spatial {
        #[arg(long)]
        x: u32,
        #[arg(long)]
        y: u32,
        #[arg(long)]
        w: u32,
        #[arg(long)]
        h: u32,
        #[arg(short, long)]
        annotation: String,
    },
    /// Verify the System 2 Rejection Sampling parser loop
    Verify,
    /// Trigger the Markdown MapReduce parallel generator
    Mapreduce {
        #[arg(short, long)]
        topic: String,
    },
    /// Trigger the Macro Discovery self-evolving loop
    Macro,
    /// Run the brutal micro-latency physical benchmarks
    Benchmark,
    /// Run FFT chaos analysis on stock market data
    FftChaos {
        #[arg(short, long)]
        file: Option<String>,
    },
    /// Run sliding window FFT backtest natively and output mark array
    FftBacktest {
        #[arg(short, long)]
        file: String,
    },
    /// Start the background daemon for Two-Tier Indexing
    Daemon,
    /// Query the knowledge base, triggering Page Faults and KV Cache generation
    Query {
        #[arg(short, long)]
        text: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Mmap { file } => {
            println!("Memory mapping file: {}", file);
            let reader = ZeroCopyMmapReader::new(file)?;
            let bytes = reader.as_bytes();
            println!("Successfully mapped {} bytes using Zero-Copy IPC.", bytes.len());
        }
        Commands::Route { text } => {
            println!("Routing Intent for text: \"{}\"", text);
            let router = HybridRouter::new();
            
            match router.route(text.as_bytes()) {
                Ok((intent, parameters)) => {
                    println!("[Router Success] Resolved to OpCode: 0x{:02X}, PayloadID: 0x{:04X}, Params: {:?}", intent.opcode, intent.payload_id, parameters);
                }
                Err(e) => {
                    eprintln!("[Router Error] Failed to route intent. Error code: 0x{:02X}", e);
                }
            }
        }
        Commands::Jit { prompt } => {
            JitCompiler::compile_and_execute(prompt, None)?;
        }
        Commands::Spatial { x, y, w, h, annotation } => {
            let bbox = BoundingBox { x: *x, y: *y, width: *w, height: *h };
            SpatialIntentEngine::trigger(bbox, annotation)?;
        }
        Commands::Verify => {
            println!("Starting System 2 Verification Loop...");
            let initial_prompt = "Generate a JSON response for transferring $10 to Alice.";
            match System2Verifier::execute_with_rejection_sampling(initial_prompt, 3) {
                Ok(ast) => println!("\n[Final Output] Successfully acquired valid AST: {:?}", ast),
                Err(e) => eprintln!("\n[Final Output] All retries failed. System panic: {}", e),
            }
        }
        Commands::Mapreduce { topic } => {
            OsDispatch::markdown_map_reduce_generation(topic)?;
        }
        Commands::Macro => {
            println!("Starting Self-Evolving Macro Discovery...");
            let evolver = SelfEvolvingLoop::new();
            
            // Mock a specific workflow being successfully executed 3 times
            let dummy_ast = model_go::UnionAst {
                opcode: 32,
                payload_id: 99,
                arguments: vec!["param1".to_string(), "param2".to_string()],
            };

            for i in 1..=3 {
                println!("\n--- Iteration {} ---", i);
                evolver.intercept_success(&dummy_ast);
            }
        }
        Commands::Benchmark => {
            BenchmarkSuite::run_all()?;
        }
        Commands::FftChaos { file } => {
            println!("Starting FFT Chaos Analysis...");
            let mut analyzer = FftChaosAnalyzer::new();
            
            let data = if let Some(path) = file {
                println!("Reading data from {}", path);
                let content = std::fs::read_to_string(path)?;
                if path.ends_with(".csv") {
                    let mut values = Vec::new();
                    // Assuming format: Date;Open;High;Low;Close;Volume
                    // or standard comma separated if semicolon not found
                    for line in content.lines().skip(1) {
                        let delimiter = if line.contains(';') { ';' } else { ',' };
                        let parts: Vec<&str> = line.split(delimiter).collect();
                        // Try to find Close column, assuming it's around index 4
                        if parts.len() >= 5 {
                            if let Ok(val) = parts[4].parse::<f64>() {
                                values.push(val);
                            }
                        }
                    }
                    values
                } else {
                    let values: Vec<f64> = serde_json::from_str(&content)?;
                    values
                }
            } else {
                println!("No file provided. Generating mock stock market data (Random Walk + Sine + White Noise)...");
                let mut mock_data = Vec::with_capacity(512);
                let mut current_price = 100.0;
                let mut seed = 1337;
                for i in 0..512 {
                    // pseudo random noise
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    let noise = (seed as f64) / (u32::MAX as f64) * 2.0 - 1.0;
                    
                    // drift + cyclic + noise
                    current_price += 0.05 + 0.5 * ((i as f64) / 10.0).sin() + noise;
                    mock_data.push(current_price);
                }
                mock_data
            };
            
            println!("Analyzing {} data points...", data.len());
            if let Some(metrics) = analyzer.analyze_time_series(&data) {
                println!("\n--- FFT Chaos Analysis Results ---");
                println!("Spectral Entropy: {:.4} (0.0 = Predictable Cyclic, 1.0 = Chaotic White Noise)", metrics.spectral_entropy);
                println!("Dominant Frequency Index: {}", metrics.dominant_frequency_index);
                println!("Dominant Power Ratio: {:.4}", metrics.dominant_power_ratio);
                
                if metrics.spectral_entropy > 0.7 {
                    println!("Conclusion: Highly Chaotic (Market is noisy and unpredictable)");
                } else if metrics.spectral_entropy > 0.4 {
                    println!("Conclusion: Moderately Chaotic (Mixed trends and noise)");
                } else {
                    println!("Conclusion: Structured (Market shows clear cyclical patterns)");
                }
            } else {
                eprintln!("Failed to analyze data (not enough data points).");
            }
        }
        Commands::FftBacktest { file } => {
            let content = std::fs::read_to_string(&file)?;
            // Attempt to parse as JSON array of f64/strings
            let parsed: Vec<serde_json::Value> = serde_json::from_str(&content)?;
            let mut prices: Vec<f64> = Vec::new();
            for v in parsed {
                if let Some(n) = v.as_f64() {
                    prices.push(n);
                } else if let Some(s) = v.as_str() {
                    prices.push(s.parse().unwrap_or(0.0));
                }
            }

            let mut analyzer = FftChaosAnalyzer::new();
            let window_size = 256;
            let mut holding = false;
            let mut buy_idx = 0;
            let mut mark: Vec<usize> = Vec::new();

            if prices.len() >= window_size {
                for i in window_size..prices.len() {
                    let window = &prices[i - window_size..i];
                    if let Some(metrics) = analyzer.analyze_time_series(window) {
                        let ma: f64 = window.iter().sum::<f64>() / window_size as f64;
                        let current_price = prices[i];
                        
                        if !holding {
                            // Inverted Entry: Market turns chaotic or Downtrend
                            if metrics.spectral_entropy > 0.45 || current_price < ma {
                                holding = true;
                                buy_idx = i;
                            }
                        } else {
                            // Inverted Exit: Highly structured market & Uptrend
                            if metrics.spectral_entropy < 0.35 && current_price > ma {
                                holding = false;
                                mark.push(buy_idx);
                                mark.push(i);
                            }
                        }
                    }
                }
                
                // Close any open positions at the end
                if holding {
                    mark.push(buy_idx);
                    mark.push(prices.len() - 1);
                }
            }
            // Just print the JSON array of marks to stdout (stockgo JS script will capture it)
            println!("{}", serde_json::to_string(&mark)?);
        }
        Commands::Daemon => {
            model_go::daemon::Daemon::run()?;
        }
        Commands::Query { text } => {
            // Assume 1.58-bit model or any model placeholder path
            let mut engine = model_go::Vec101Engine::new("placeholder_path")?;
            match engine.query_with_page_fault(text) {
                Ok(response) => println!("[Query Result]\n{}", response),
                Err(e) => eprintln!("[Query Error] {}", e),
            }
        }
    }
    
    Ok(())
}
