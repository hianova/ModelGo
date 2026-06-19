use model_go::science::{AssemblyFunnel, FunnelObserver, MathObjective};
use model_go::science::assembly_funnel::FunnelConfig;
use std::time::Instant;
use std::io::Write;

pub struct MathObserver;

impl FunnelObserver for MathObserver {
    fn on_generation_complete(&mut self, generation: u64, global_iters: u64, best_fitness: f32, total_found: usize) {
        let iters_k = global_iters as f64 / 1_000.0;
        print!("\r\x1b[2K[Math-Space] Gen: {} | Iters: {:.1}k | Gen Best MSE: {:.6} | 💎 Laws Mined: {}", generation, iters_k, best_fitness, total_found);
        std::io::stdout().flush().unwrap();
    }

    fn on_archive_success(&mut self, _generation: u64, _global_iters: u64, _fitness: f32) {
        // The output is printed inside MathObjective::check_archival
    }
}

fn main() {
    println!("============================================================");
    println!("🧪 ISOMORPHIC PROJECTION: MATH-SPACE DISCOVERY");
    println!("============================================================");
    
    // Define a hidden target mathematical law: Y = X^2 + 5X
    // It's a simple quadratic formula. The system has to deduce the structure 
    // of this formula purely from chaotic permutations of RPN tokens.
    println!("Target Physics Law (Hidden): Y = X^2 + 5X");
    
    let mut dataset = Vec::new();
    for i in 1..=10 {
        let x = i as f32;
        let y = x * x + 5.0 * x;
        dataset.push((x, y));
    }
    
    let objective = MathObjective {
        dataset,
        start_time: Instant::now(),
    };
    
    let config = FunnelConfig {
        tier1_population: 50_000,       // Large initial sample for diversity
        tier2_retention_ratio: 0.1,     // Keep top 10%
        tier3_dfs_depth: 30,            // Depth of Lévy search mutations
        stagnation_patience: 10,        // Reset if we get stuck
        stagnation_delta: 0.5,
        rng_seed: 0x4D415448,           // 'MATH'
    };
    
    let mut funnel = AssemblyFunnel::new(config);
    let mut observer = MathObserver;
    
    println!("Igniting Chaos Engine... Searching for Universal Mathematical Law.");
    funnel.run_evolution_loop(&objective, &mut observer);
}
