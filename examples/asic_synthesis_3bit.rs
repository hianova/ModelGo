use model_go::science::assembly_funnel::{FunnelConfig, StandardObserver};
use model_go::science::{AsicObjective, AssemblyFunnel};
use model_go::science::asic_objective::{TruthTableBuilder, create_default_motif_cache};

fn main() {
    println!("============================================================");
    println!("🔬 ASIC LOGIC SYNTHESIS: 3-BIT MULTIPLIER");
    println!("============================================================");

    // 3-bit Multiplier Truth Table
    // Inputs: A2, A1, A0, B2, B1, B0 (6 bits)
    // Outputs: P5, P4, P3, P2, P1, P0 (6 bits)
    let truth_table = TruthTableBuilder::new(6, 6)
        .generate(|row| {
            let a = row >> 3;
            let b = row & 0b111;
            a * b
        });

    let cached_motifs = create_default_motif_cache();

    let objective = AsicObjective::new(
        6, // num_inputs
        6, // num_outputs
        truth_table,
        40, // max_gates (higher initial capacity for 3-bit multiplier)
        cached_motifs,
    );

    let config = FunnelConfig {
        tier1_population: 200_000,  // Increased for much larger search space
        tier2_retention_ratio: 0.1, // Keep top 10%
        tier3_dfs_depth: 50,
        stagnation_patience: 20,    // More patience
        stagnation_delta: 0.5,
        rng_seed: 0x4D554C33, // 'MUL3'
    };

    let mut funnel = AssemblyFunnel::new(config);
    let mut observer = StandardObserver::new("[Optimization]").with_generation_log(true);

    println!("Igniting Chaos Engine... Searching for minimal gate 3-bit Multiplier DAG.");
    funnel.run_evolution_loop(&objective, &mut observer);
}
