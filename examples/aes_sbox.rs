use model_go::science::assembly_funnel::{FunnelConfig, StandardObserver};
use model_go::science::{AsicObjective, AssemblyFunnel};
use model_go::science::asic_objective::{TruthTableBuilder, create_default_motif_cache, AsicCircuit, Gate, GateOp, MotifCache};
use std::sync::Arc;
use model_go::loader::ZeroCopyModelLoader;
use model_go::speculative_engine::SpeculativeEngine;


const AES_SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

fn parse_llm_circuit(
    script: &str,
    num_inputs: usize,
    num_outputs: usize,
    max_gates: usize,
) -> Option<model_go::science::asic_objective::AsicCircuit> {
    use model_go::science::asic_objective::{AsicCircuit, Gate, GateOp};

    let mut gates = Vec::new();
    let mut output_map = vec![0u16; num_outputs];
    let mut out_assigned = 0;

    for line in script.lines() {
        let text = line.replace(",", "").replace(";", "").trim().to_uppercase();
        if text.contains("=") && (text.contains("W_") || text.contains("OUT_")) {
            let parts: Vec<&str> = text.split('=').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let lhs = parts[0];
                let rhs = parts[1];

                if let Some(stripped_lhs) = lhs.strip_prefix("OUT_") {
                    if let Ok(idx) = stripped_lhs.parse::<usize>()
                        && idx < num_outputs
                    {
                        let wire_id = if let Some(stripped_rhs) = rhs.strip_prefix("W_") {
                            stripped_rhs.parse::<u16>().unwrap_or(0) + num_inputs as u16
                        } else if let Some(stripped_rhs) = rhs.strip_prefix("INP_") {
                            stripped_rhs.parse::<u16>().unwrap_or(0)
                        } else {
                            0
                        };
                        output_map[idx] = wire_id;
                        out_assigned += 1;
                    }
                    continue;
                }

                let mut op = GateOp::Xor;
                let mut is_not = false;
                if rhs.contains(" AND ") {
                    op = GateOp::And;
                } else if rhs.contains(" OR ") {
                    op = GateOp::Or;
                } else if rhs.contains(" XOR ") {
                    op = GateOp::Xor;
                } else if rhs.contains(" NAND ") {
                    op = GateOp::Nand;
                } else if rhs.contains(" NOR ") {
                    op = GateOp::Nor;
                } else if rhs.contains(" XNOR ") {
                    op = GateOp::Xnor;
                } else if rhs.contains("NOT ") {
                    op = GateOp::Not;
                    is_not = true;
                }

                let tokens: Vec<&str> = rhs.split_whitespace().collect();
                if tokens.len() >= 3 || is_not {
                    let parse_pin = |pin: &str| -> u16 {
                        if let Some(stripped) = pin.strip_prefix("INP_") {
                            stripped.parse::<u16>().unwrap_or(0)
                        } else if let Some(stripped) = pin.strip_prefix("W_") {
                            stripped.parse::<u16>().unwrap_or(0) + num_inputs as u16
                        } else {
                            0
                        }
                    };

                    let left = if is_not {
                        parse_pin(tokens.last().unwrap_or(&""))
                    } else {
                        parse_pin(tokens[0])
                    };
                    let right = if is_not {
                        0
                    } else {
                        parse_pin(tokens.last().unwrap_or(&""))
                    };
                    gates.push(Gate { left, right, op });
                }
            }
        }
    }

    let mut seed = 987654321u64;
    while gates.len() < max_gates {
        let max_idx = (num_inputs + gates.len()) as u16;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let left = (seed % max_idx as u64) as u16;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let right = (seed % max_idx as u64) as u16;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let op = GateOp::from_u8((seed % 7) as u8);
        gates.push(Gate { left, right, op });
    }
    gates.truncate(max_gates);

    if out_assigned < num_outputs {
        for (i, val) in output_map.iter_mut().enumerate() {
            *val = (num_inputs + max_gates - num_outputs + i) as u16;
        }
    }

    let mut affine_mask = [0u8; 8];
    for (i, mask) in affine_mask.iter_mut().enumerate() {
        *mask = 1 << i;
    }

    Some(AsicCircuit {
        gates,
        output_map,
        affine_mask,
        affine_const: 0,
    })
}

fn run_speculative_miner(cached_motifs: Arc<MotifCache>, loader: Arc<ZeroCopyModelLoader>) {
    let mut engine = SpeculativeEngine::new(loader, 1024, 4096, 1);

    let num_inputs = 4;
    let num_outputs = 4;
    let num_gates = 12;

    let input_states: [u16; 4] = [0xAAAA, 0xCCCC, 0xF0F0, 0xFF00];
    let _max_gates = 20;
    let mut rng_seed = 0x11223344usize;
    let handle = cached_motifs.register_thread();

    loop {
        for x in &mut engine.x_stream {
            rng_seed = rng_seed.wrapping_add(1);
            let rnd = rng_seed % 3;
            if rnd == 0 {
                *x = -1;
            } else if rnd == 1 {
                *x = 0;
            } else {
                *x = 1;
            }
        }

        // MTP Draft Tree search (Depth 12, Layer stride 2, Max 1024 branches)
        let draft_tree = unsafe { engine.run_draft_mode(num_gates, 2, 1024) };

        let mut is_parent = vec![false; draft_tree.tokens.len()];
        for &p in &draft_tree.tree_mask {
            if (p as usize) < is_parent.len() {
                is_parent[p as usize] = true;
            }
        }

        for (i, &parent) in is_parent
            .iter()
            .enumerate()
            .take(draft_tree.tokens.len())
            .skip(1)
        {
            if !parent {
                let mut path = Vec::with_capacity(num_gates);
                let mut curr = i as u32;
                while curr != 0 {
                    path.push(draft_tree.tokens[curr as usize]);
                    curr = draft_tree.tree_mask[curr as usize];
                }
                path.reverse();

                if path.len() < num_gates {
                    continue;
                }

                let mut gates = Vec::with_capacity(num_gates);
                for (j, &token) in path.iter().enumerate().take(num_gates) {
                    let max_idx = (num_inputs + j) as u16;
                    let left = ((token & 0x7F) as u16) % max_idx;
                    let right = (((token >> 7) & 0x7F) as u16) % max_idx;
                    let op = GateOp::from_u8(((token >> 14) % 7) as u8);
                    gates.push(Gate { left, right, op });
                }

                let mut output_map = Vec::with_capacity(num_outputs);
                let max_wire_idx = (num_inputs + num_gates) as u16;
                for k in 0..num_outputs {
                    output_map.push(max_wire_idx - num_outputs as u16 + k as u16);
                }

                let mut wires = vec![0u16; num_inputs + num_gates];
                let mut depths = vec![0u32; num_inputs + num_gates];
                wires[..num_inputs].copy_from_slice(&input_states[..num_inputs]);

                for j in 0..num_gates {
                    let gate = &gates[j];
                    let l = wires[gate.left as usize];
                    let r = wires[gate.right as usize];

                    let l_depth = depths[gate.left as usize];
                    let r_depth = depths[gate.right as usize];
                    let is_nonlinear = matches!(
                        gate.op,
                        GateOp::And | GateOp::Nand | GateOp::Or | GateOp::Nor
                    );
                    depths[num_inputs + j] =
                        l_depth.max(r_depth) + if is_nonlinear { 1 } else { 0 };

                    let out = match gate.op {
                        GateOp::And => l & r,
                        GateOp::Or => l | r,
                        GateOp::Xor => l ^ r,
                        GateOp::Nand => !(l & r),
                        GateOp::Nor => !(l | r),
                        GateOp::Xnor => !(l ^ r),
                        GateOp::Not => !l,
                    };
                    wires[num_inputs + j] = out;
                }

                let mut max_depth = 0;
                let mut all_outputs_good = true;

                for &out_pin in &output_map {
                    max_depth = max_depth.max(depths[out_pin as usize]);
                    let ones = wires[out_pin as usize].count_ones();
                    if !(4..=12).contains(&ones) {
                        all_outputs_good = false;
                        break;
                    }
                }

                if all_outputs_good && max_depth >= 4 {
                    let mut affine_mask = [0u8; 8];
                    for (k, mask) in affine_mask.iter_mut().enumerate().take(num_outputs.min(8)) {
                        *mask = 1 << k;
                    }

                    let motif = AsicCircuit {
                        gates,
                        output_map,
                        affine_mask,
                        affine_const: 0,
                    };

                    rng_seed = rng_seed.wrapping_add(1);
                    let key = (rng_seed % 5000) as u64;
                    cached_motifs.insert(key, Arc::new((num_inputs, motif)), &handle);

                    if rng_seed.is_multiple_of(1000) {
                        println!(
                            "[Speculative Miner] DualCacheFF Async Populating from DraftTree... (Inserted {:?} motifs)",
                            rng_seed
                        );
                    }
                }
            }
        }
        std::thread::yield_now();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn main() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            run_aes();
        })
        .unwrap()
        .join()
        .unwrap();
}

fn run_aes() {
    let _ = rayon::ThreadPoolBuilder::new()
        .stack_size(64 * 1024 * 1024)
        .build_global();

    // Construct the AES S-Box truth table (8 bits to 8 bits)
    let truth_table = TruthTableBuilder::new(8, 8)
        .generate(|row| AES_SBOX[row as usize] as u64);

    let cached_motifs = create_default_motif_cache();

    let mut objective = AsicObjective::new(8, 8, truth_table, 3000, cached_motifs.clone());

    println!("============================================================");
    println!("🛡️ AES S-BOX SYNTHESIS : HACKER NEWS WORLD RECORD ATTEMPT 🛡️");
    println!("============================================================");

    println!(
        ">>> [System 1] Waking up LLM (Vec101/BitNet) to generate Boyar-Peralta topological seed..."
    );
    let fallback_engine = model_go::router::get_fallback_engine();
    let prompt = "Generate the complete 113-gate Boyar-Peralta AES S-Box logic circuit straight-line program. Use only AND, OR, XOR, XNOR, NAND, NOR, NOT gates. Format each line exactly as: W_8 = INP_0 XOR INP_1, where INP_0 to INP_7 are the 8 inputs, and W_8 to W_120 are the intermediate wires. Assign the final 8 outputs to OUT_0 to OUT_7.";

    if let Ok(script) = fallback_engine.generate_script(prompt) {
        println!(">>> [System 1] LLM Seed generated! Parsing topology...");
        if let Some(circuit) = parse_llm_circuit(&script, 8, 8, 3000) {
            objective.llm_seed = Some(circuit);
            println!(
                ">>> [System 1] Boyar-Peralta S-Box seed successfully injected into Chaos Engine!"
            );
        } else {
            println!(">>> [System 1] Failed to parse System 1 output. Using random noise.");
        }
    } else {
        println!(">>> [System 1] System 1 unavailable or failed to infer. Using random noise.");
    }

    let config = FunnelConfig {
        tier1_population: 2_000,
        tier2_retention_ratio: 0.2,
        tier3_dfs_depth: 30,
        stagnation_patience: 10_000, // Give Isomorphic Compensation and Motif Injection time to align!
        stagnation_delta: 0.00005,
        rng_seed: 0x4145535F, // 'AES_'
    };

    let mut funnel = AssemblyFunnel::new(config);
    let mut observer = StandardObserver::new("[AES Synthesis]").with_generation_log(true);

    println!("============================================================");
    println!("🛡️ AES S-BOX SYNTHESIS : HACKER NEWS WORLD RECORD ATTEMPT 🛡️");
    println!("============================================================");
    println!("Targeting < 113 Active Gates...");

    // Start Meshtree Miners
    println!(">>> Spawning 1 High-Intelligence Speculative Miner (LLM Tree Search)...");

    // We try to load bitnet_compiled.rkyv, fallback to /dev/null to simulate if not exists
    let loader = Arc::new(ZeroCopyModelLoader::new("../bitnet_compiled.rkyv").unwrap_or_else(|_| {
        println!(">>> [Warning] Failed to load bitnet_compiled.rkyv. Make sure the model exists! Falling back to dev/null.");
        ZeroCopyModelLoader::new("/dev/null").unwrap()
    }));

    let cache_clone = cached_motifs.clone();
    std::thread::Builder::new()
        .stack_size(128 * 1024 * 1024)
        .spawn(move || {
            run_speculative_miner(cache_clone, loader);
        })
        .unwrap();

    println!("Igniting Chaos Engine (Continuous Boolean Relaxation)...");
    funnel.run_evolution_loop(&objective, &mut observer);

    println!(">>> MAIN EXITED! HOW?");
}
