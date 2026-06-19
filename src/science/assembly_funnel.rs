use crate::science::ScienceObjective;
use crate::chaos_state::{ChaosState, MicroTweak, RngState, step_forward_nd};
use std::time::Instant;

/// Observer trait to decouple the engine from specific UI/Telemetry implementations (e.g. motionview_core).
pub trait FunnelObserver {
    fn on_generation_complete(&mut self, generation: u64, global_iters: u64, best_fitness: f32, total_found: usize);
    fn on_archive_success(&mut self, generation: u64, global_iters: u64, fitness: f32);
}

/// Configuration for the 3-Tier Assembly Funnel
pub struct FunnelConfig {
    pub tier1_population: usize,
    pub tier2_retention_ratio: f32, // e.g. 0.2 for top 20%
    pub tier3_dfs_depth: usize,
    pub stagnation_patience: u64,
    pub stagnation_delta: f32,
    pub rng_seed: u32,
}

impl Default for FunnelConfig {
    fn default() -> Self {
        Self {
            tier1_population: 10_000,
            tier2_retention_ratio: 0.2,
            tier3_dfs_depth: 100,
            stagnation_patience: 10,
            stagnation_delta: 0.1,
            rng_seed: 0x2026,
        }
    }
}

pub struct AssemblyFunnel<T: Clone> {
    config: FunnelConfig,
    parent_population: Vec<T>,
}

impl<T: Clone> AssemblyFunnel<T> {
    pub fn new(config: FunnelConfig) -> Self {
        Self {
            config,
            parent_population: Vec::new(),
        }
    }

    /// Run the infinite evolution loop for the given objective.
    pub fn run_evolution_loop<O: ScienceObjective<T>, F: FunnelObserver>(&mut self, objective: &O, observer: &mut F) {
        let mut seed: usize = self.config.rng_seed as usize;
        let start_time = Instant::now();
        let mut global_iterations = 0u64;
        let mut total_found = 0;
        let mut generation = 0u64;

        let mut stagnation_counter = 0;
        let mut global_best_fitness = f32::MAX;

        let tweak = MicroTweak {
            s_exponent: 1.5,
            max_elements: 1000,
        };
        let mut rng_state = RngState::new(self.config.rng_seed);
        let mut chaos_state = ChaosState::<10, 1>::new([0.0]);

        loop {
            generation += 1;
            
            // ========================================================
            // TIER 1: Brute-Force Generation
            // ========================================================
            let mut tier1_population = Vec::with_capacity(self.config.tier1_population);
            
            for _ in 0..self.config.tier1_population {
                global_iterations += 1;
                
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let is_from_parent = !self.parent_population.is_empty() && (seed % 100 < 90);
                
                let parent = if is_from_parent {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let parent_idx = seed % self.parent_population.len();
                    Some(&self.parent_population[parent_idx])
                } else {
                    None
                };

                let candidate = objective.generate_seed(seed, parent);
                let fitness = objective.evaluate_fitness(&candidate);
                tier1_population.push((candidate, fitness));
            }

            // ========================================================
            // TIER 2: Selection Mechanism (Zipf Emergence)
            // ========================================================
            tier1_population.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let retention_count = (self.config.tier1_population as f32 * self.config.tier2_retention_ratio) as usize;
            let tier2_pool: Vec<_> = tier1_population.into_iter().take(retention_count).collect();
            let gen_best_fitness = tier2_pool[0].1;

            observer.on_generation_complete(generation, global_iterations, gen_best_fitness, total_found);

            // ========================================================
            // TIER 3: DFS Deep Dive (Assembly Theory)
            // ========================================================
            let mut next_parent_population = Vec::with_capacity(retention_count);
            let mut found_target_in_gen = false;

            for (mut candidate, mut current_fitness) in tier2_pool {
                for _ in 0..self.config.tier3_dfs_depth {
                    global_iterations += 1;
                    chaos_state = step_forward_nd(&chaos_state, &tweak, &mut rng_state);
                    let cv = chaos_state.base_values[0];
                    
                    // Lévy flight mutation scale mapping
                    let mutation_scale = (0.01 + (cv.abs() as f32 * 0.1)).min(2.0);

                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let child = objective.perturb(&candidate, mutation_scale, seed);
                    
                    if !objective.is_valid(&child) {
                        continue;
                    }

                    let child_fitness = objective.evaluate_fitness(&child);
                    
                    if child_fitness < current_fitness {
                        current_fitness = child_fitness;
                        candidate = child;
                    }
                    
                    if objective.check_archival(&candidate, current_fitness) {
                        total_found += 1;
                        observer.on_archive_success(generation, global_iterations, current_fitness);
                        found_target_in_gen = true;
                        break;
                    }
                }
                
                if found_target_in_gen {
                    break;
                }
                
                next_parent_population.push(candidate);
            }
            
            // ========================================================
            // Cross-Generational Loop Closure & Stagnation Control
            // ========================================================
            if found_target_in_gen {
                self.parent_population.clear();
                global_best_fitness = f32::MAX;
                stagnation_counter = 0;
                continue;
            }
            
            self.parent_population = next_parent_population;
            
            if gen_best_fitness < global_best_fitness - self.config.stagnation_delta {
                global_best_fitness = gen_best_fitness;
                stagnation_counter = 0;
            } else {
                stagnation_counter += 1;
            }
            
            if stagnation_counter >= self.config.stagnation_patience {
                // Stagnation detected. Triggering Big Bang Reset.
                self.parent_population.clear();
                global_best_fitness = f32::MAX;
                stagnation_counter = 0;
            }
        }
    }
}
