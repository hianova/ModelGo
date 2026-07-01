use model_go::science::assembly_funnel::{FunnelConfig, StandardObserver};
use model_go::science::{AssemblyFunnel, MathObjective};
use std::time::Instant;

fn main() {
    println!("============================================================");
    println!("🧪 ISOMORPHIC PROJECTION: MATH-SPACE DISCOVERY");
    println!("============================================================");

    // Define a hidden target mathematical law: Y = X^2 + 5X
    // It's a simple quadratic formula. The system has to deduce the structure
    // of this formula purely from chaotic permutations of RPN tokens.
    println!("Target Physics Law (Hidden): Y = X^2 + 5X");

    let mut dataset = Vec::new();
    // 最佳化邊界：涵蓋負數、零、正數與較大的範圍，確保能適應各種邊界情況
    let boundaries = [
        -100.0, -50.0, -10.0, -5.0, -2.0, -1.0, 0.0, 1.0, 2.0, 5.0, 10.0, 50.0, 100.0
    ];
    for &x in &boundaries {
        let y = x * x + 5.0 * x;
        dataset.push((x, y));
    }

    let objective = MathObjective {
        dataset,
        start_time: Instant::now(),
    };

    let config = FunnelConfig {
        tier1_population: 50_000,   // Large initial sample for diversity
        tier2_retention_ratio: 0.1, // Keep top 10%
        tier3_dfs_depth: 30,        // Depth of Lévy search mutations
        stagnation_patience: 10,    // Reset if we get stuck
        stagnation_delta: 0.5,
        rng_seed: 0x4D415448, // 'MATH'
    };

    let mut funnel = AssemblyFunnel::new(config);
    let mut observer = StandardObserver::new("[Math-Space]").with_generation_log(true);

    println!("Igniting Chaos Engine... Searching for Universal Mathematical Law.");
    funnel.run_evolution_loop(&objective, &mut observer);
}
