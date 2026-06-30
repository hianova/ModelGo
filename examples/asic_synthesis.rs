use cdDB::DualCacheFF;
use model_go::science::assembly_funnel::FunnelConfig;
use model_go::science::{AsicCircuit, AsicObjective, AssemblyFunnel, FunnelObserver};
use std::sync::Arc;

pub struct AsicObserver {
    pub best_seen: f32,
}

impl FunnelObserver for AsicObserver {
    fn on_generation_complete(
        &mut self,
        _generation: u64,
        _global_iters: u64,
        best_fitness: (u32, u32),
        _total_found: usize,
    ) {
        // use best_fitness.0 as primary metric for display
        let best_f_f32 = best_fitness.0 as f32;
        if best_f_f32 < self.best_seen {
            self.best_seen = best_f_f32;
            println!(
                "[Optimization] Discovered Better Topology -> Score (Incorrect, Gates): {:?}",
                best_fitness
            );
        }
    }

    fn on_archive_success(&mut self, _generation: u64, _global_iters: u64, _fitness: (u32, u32)) {
        // Output is handled inside AsicObjective::check_archival
    }
}

fn main() {
    println!("============================================================");
    println!("🔬 ASIC LOGIC SYNTHESIS: 2-BIT MULTIPLIER");
    println!("============================================================");

    // Define 2-bit Multiplier Truth Table
    // Inputs: A1, A0, B1, B0 (4 bits)
    // Outputs: P3, P2, P1, P0 (4 bits) where P = A * B
    let mut truth_table = Vec::with_capacity(16);

    for a in 0..4 {
        for b in 0..4 {
            let p = a * b; // 0 to 9, needs 4 bits

            let a1 = (a >> 1) & 1 == 1;
            let a0 = a & 1 == 1;
            let b1 = (b >> 1) & 1 == 1;
            let b0 = b & 1 == 1;

            let p3 = (p >> 3) & 1 == 1;
            let p2 = (p >> 2) & 1 == 1;
            let p1 = (p >> 1) & 1 == 1;
            let p0 = p & 1 == 1;

            truth_table.push((vec![a1, a0, b1, b0], vec![p3, p2, p1, p0]));
        }
    }
    let cached_motifs = Arc::new(DualCacheFF::<
        u64,
        Arc<(usize, AsicCircuit)>,
        cdDB::dualcache_ff::core::config::DefaultExponentialPolicy,
        1024,
        2048,
        4096,
        7168,
        16,
        1024,
        64,
    >::new());

    let objective = AsicObjective::new(
        4, // num_inputs
        4, // num_outputs
        truth_table,
        12, // max_gates
        cached_motifs,
    );

    let config = FunnelConfig {
        tier1_population: 100_000,  // Large sample for diversity
        tier2_retention_ratio: 0.1, // Keep top 10%
        tier3_dfs_depth: 50,        // Deep Lévy mutations
        stagnation_patience: 15,    // Patience before chaos reset
        stagnation_delta: 0.5,
        rng_seed: 0x41534943, // 'ASIC'
    };

    let mut funnel = AssemblyFunnel::new(config);
    let mut observer = AsicObserver {
        best_seen: f32::MAX,
    };

    println!("Igniting Chaos Engine... Searching for minimal gate 2-bit Multiplier DAG.");
    funnel.run_evolution_loop(&objective, &mut observer);
}
