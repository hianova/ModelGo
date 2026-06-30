use crate::chaos_state::{ChaosState, MicroTweak, RngState, StagnationFeedback, step_forward_nd};
use crate::science::ScienceObjective;
use std::time::Instant;

/// Observer trait to decouple the engine from specific UI/Telemetry implementations (e.g. motionview_core).
pub trait FunnelObserver: Send + Sync {
    fn on_generation_complete(
        &mut self,
        generation: u64,
        global_iters: u64,
        best_fitness: (u32, u32),
        total_found: usize,
    );
    fn on_archive_success(&mut self, generation: u64, global_iters: u64, fitness: (u32, u32));
}

struct FunnelStagnation {
    counter: u64,
    patience: u64,
}

impl StagnationFeedback for FunnelStagnation {
    fn current_gradient(&self) -> f32 {
        if self.patience == 0 {
            return 1.0;
        }
        (self.counter as f32 / self.patience as f32).min(1.0)
    }
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

pub struct ParetoArchive<T: Clone + Send + Sync> {
    pub elites: Vec<(u32, u32, T)>, // (Incorrect Bits, Active Gates, Circuit)
}

impl<T: Clone + Send + Sync> Default for ParetoArchive<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Send + Sync> ParetoArchive<T> {
    pub fn new() -> Self {
        Self { elites: Vec::new() }
    }

    pub fn try_add(&mut self, incorrect: u32, gates: u32, candidate: &T) -> bool {
        let mut is_dominated = false;
        let mut to_remove = Vec::new();

        for (i, &(arch_inc, arch_gates, _)) in self.elites.iter().enumerate() {
            if arch_inc <= incorrect && arch_gates <= gates {
                // We are dominated by an existing elite
                is_dominated = true;
                break;
            } else if incorrect <= arch_inc && gates <= arch_gates {
                // We dominate this existing elite
                to_remove.push(i);
            }
        }

        if !is_dominated {
            for &i in to_remove.iter().rev() {
                self.elites.remove(i);
            }
            self.elites.push((incorrect, gates, candidate.clone()));
            true
        } else {
            false
        }
    }

    pub fn get_random_seed(&self, seed: &mut u64) -> Option<T> {
        if self.elites.is_empty() {
            return None;
        }
        *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let idx = *seed as usize % self.elites.len();
        Some(self.elites[idx].2.clone())
    }
}

pub struct AssemblyFunnel<T: Clone + Send + Sync> {
    config: FunnelConfig,
    parent_population: Vec<T>,
    pub archive: ParetoArchive<T>,
}

impl<T: Clone + Send + Sync> AssemblyFunnel<T> {
    pub fn new(config: FunnelConfig) -> Self {
        Self {
            config,
            parent_population: Vec::new(),
            archive: ParetoArchive::new(),
        }
    }

    /// Run the infinite evolution loop for the given objective.
    pub fn run_evolution_loop<O: ScienceObjective<T>, F: FunnelObserver>(
        &mut self,
        objective: &O,
        observer: &mut F,
    ) {
        let mut seed: u64 = self.config.rng_seed as u64;
        let _start_time = Instant::now();
        let mut global_iterations = 0u64;
        let mut total_found = 0;
        let mut generation = 0u64;
        let mut stagnation_counter = 0;
        let mut global_best_fitness: (u32, u32) = (u32::MAX, u32::MAX);
        let mut global_best_candidate: Option<T> = None;

        let mut tweak = MicroTweak {
            s_exponent: 2.0, // Start with mild Zipf
            max_elements: 1000,
        };
        let mut rng_state = RngState::new(self.config.rng_seed);
        let mut chaos_state = ChaosState::<10, 1>::new([0.0]);

        loop {
            print!(
                "\r\x1b[K[AssemblyFunnel] Starting generation {}...",
                generation
            );
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            generation += 1;

            // ========================================================
            // TIER 1: Brute-Force Generation
            // ========================================================
            let mut tier1_candidates = Vec::with_capacity(self.config.tier1_population);

            while tier1_candidates.len() < self.config.tier1_population {
                global_iterations += 1;

                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let roll = seed % 100;

                if !self.parent_population.is_empty()
                    && self.parent_population.len() >= 2
                    && roll < 40
                {
                    // Crossover 2 parents from the pool
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let p1_idx = (seed % self.parent_population.len() as u64) as usize;
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let p2_idx = (seed % self.parent_population.len() as u64) as usize;

                    let children = objective.crossover(
                        &self.parent_population[p1_idx],
                        &self.parent_population[p2_idx],
                        seed as usize,
                    );
                    for mut child in children {
                        // Apply a tiny mutation to prevent exact clones from flooding
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        let scale = ((seed % 100) as f32 / 10000.0) + 0.0001;
                        child = objective.perturb(&child, scale, seed as usize);
                        if tier1_candidates.len() < self.config.tier1_population {
                            tier1_candidates.push(child);
                        }
                    }
                } else if !self.archive.elites.is_empty()
                    && self.archive.elites.len() >= 2
                    && roll < 50
                {
                    // Crossover 2 elites from the archive
                    if let Some(elite1) = self.archive.get_random_seed(&mut seed)
                        && let Some(elite2) = self.archive.get_random_seed(&mut seed)
                    {
                        let children = objective.crossover(&elite1, &elite2, seed as usize);
                        for mut child in children {
                            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                            let scale = ((seed % 100) as f32 / 10000.0) + 0.0001;
                            child = objective.perturb(&child, scale, seed as usize);
                            if tier1_candidates.len() < self.config.tier1_population {
                                tier1_candidates.push(child);
                            }
                        }
                    }
                } else if !self.parent_population.is_empty() && roll < 80 {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let parent_idx = (seed % self.parent_population.len() as u64) as usize;
                    tier1_candidates.push(
                        objective.generate_seed(
                            seed as usize,
                            Some(&self.parent_population[parent_idx]),
                        ),
                    );
                } else if !self.archive.elites.is_empty() && roll < 95 {
                    if let Some(elite) = self.archive.get_random_seed(&mut seed) {
                        tier1_candidates.push(objective.generate_seed(seed as usize, Some(&elite)));
                    } else {
                        tier1_candidates.push(objective.generate_seed(seed as usize, None));
                    }
                } else {
                    tier1_candidates.push(objective.generate_seed(seed as usize, None));
                }
            }

            let mut tier1_fitness = vec![(0u32, 0u32); self.config.tier1_population];
            objective.evaluate_fitness_batch(&tier1_candidates, &mut tier1_fitness);

            let mut tier1_population: Vec<_> =
                tier1_candidates.into_iter().zip(tier1_fitness).collect();

            // ========================================================
            // TIER 2: Selection Mechanism (Zipf Emergence)
            // ========================================================
            tier1_population.sort_by(|a, b| {
                if a.1.0 != b.1.0 {
                    a.1.0.cmp(&b.1.0)
                } else {
                    a.1.1.cmp(&b.1.1)
                }
            });
            let retention_count =
                (self.config.tier1_population as f32 * self.config.tier2_retention_ratio) as usize;
            let _island_a_limit = retention_count / 10;
            let island_b_limit = retention_count * 7 / 10;

            let mut tier2_pool = Vec::with_capacity(retention_count);
            // Island A & B (Top 70%): Keep the best of the newly generated children
            for item in tier1_population.iter().take(island_b_limit) {
                tier2_pool.push(item.clone());
            }

            // Island C (Bottom 30%): Preserve the deeply explored mutants from the previous generation!
            // This prevents the global sorting from mercilessly killing high-error topological building blocks.
            for (i, item) in tier1_population
                .iter()
                .enumerate()
                .take(retention_count)
                .skip(island_b_limit)
            {
                if self.parent_population.len() == retention_count {
                    // Carry over the heavily annealed mutant from the previous Tier 3 DFS
                    let parent = &self.parent_population[i];
                    let fitness = objective.evaluate_fitness(parent);
                    tier2_pool.push((parent.clone(), fitness));
                } else {
                    // Gen 0 fallback: just take the children
                    tier2_pool.push(item.clone());
                }
            }

            let gen_best_fitness = tier2_pool[0].1;

            observer.on_generation_complete(
                generation,
                global_iterations,
                gen_best_fitness,
                total_found,
            );

            // ========================================================
            // TIER 3: Wave-front Deep Dive (Vectorized Assembly Theory)
            // ========================================================
            let mut found_target_in_gen = false;

            for _depth in 0..self.config.tier3_dfs_depth {
                let mut children = Vec::with_capacity(retention_count);
                let island_a_limit = retention_count / 10;
                let island_b_limit = retention_count * 7 / 10;

                for (i, (candidate, _)) in tier2_pool.iter().enumerate() {
                    global_iterations += 1;
                    chaos_state = step_forward_nd(&chaos_state, &tweak, &mut rng_state);

                    let mutation_scale = if i < island_a_limit {
                        0.0001 // Island A (Top 10%): Strict 1-gate micro-mutation
                    } else if i < island_b_limit {
                        0.001 // Island B (Next 60%): Explorer ~3 gates
                    } else {
                        0.05 // Island C (Bottom 30%): Nuclear ~75 gates
                    };

                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    children.push(objective.perturb(candidate, mutation_scale, seed as usize));
                }

                let mut children_fitness = vec![(0u32, 0u32); retention_count];
                objective.evaluate_fitness_batch(&children, &mut children_fitness);

                for i in 0..retention_count {
                    if !objective.is_valid(&children[i]) {
                        continue;
                    }

                    let child_fitness = children_fitness[i];

                    let mut accept = false;

                    if child_fitness.0 <= tier2_pool[i].1.0 && child_fitness.1 <= tier2_pool[i].1.1
                    {
                        // Pareto Dominates or Equal (All Islands)
                        accept = true;
                    } else if i >= island_a_limit {
                        // Simulated Annealing (Island B and C only)
                        // Cryptographic Avalanche Effect means 1 gate change can flip 50-100 bits!
                        // We MUST tolerate huge temporary error spikes to cross the non-linear valleys.
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        let diff = child_fitness.0 as f32 - tier2_pool[i].1.0 as f32;

                        let prob = if i < island_b_limit {
                            // Island B: Medium SA (Accept up to +30 error bits due to avalanche)
                            if diff > 0.0 && diff <= 30.0 {
                                (-diff / 10.0).exp()
                            } else {
                                0.0
                            }
                        } else {
                            // Island C: Nuclear SA (Accept up to +400 error bits to cross the 1024-error random noise valley)
                            if diff > 0.0 && diff <= 400.0 {
                                (-diff / 100.0).exp()
                            } else {
                                0.0
                            }
                        };

                        if (seed % 1000) as f32 / 1000.0 < prob {
                            accept = true;
                        }
                    }

                    if accept {
                        tier2_pool[i].1 = child_fitness;
                        tier2_pool[i].0 = children[i].clone();

                        // Try to add to global Pareto Archive
                        if self
                            .archive
                            .try_add(child_fitness.0, child_fitness.1, &tier2_pool[i].0)
                        {
                            stagnation_counter = 0; // We found a new elite!
                        }
                    }

                    if objective.check_archival(&tier2_pool[i].0, tier2_pool[i].1) {
                        total_found += 1;
                        observer.on_archive_success(generation, global_iterations, tier2_pool[i].1);
                        found_target_in_gen = true;
                        break;
                    }
                }

                if found_target_in_gen {
                    break;
                }
            }

            let next_parent_population: Vec<_> =
                tier2_pool.iter().map(|(c, _)| c.clone()).collect();

            // ========================================================
            // Cross-Generational Loop Closure & Stagnation Control
            // ========================================================
            if found_target_in_gen {
                self.parent_population.clear();
                global_best_fitness = (u32::MAX, u32::MAX);
                global_best_candidate = None;
                stagnation_counter = 0;
                continue;
            }

            self.parent_population = next_parent_population;

            if gen_best_fitness.0 < global_best_fitness.0
                || (gen_best_fitness.0 == global_best_fitness.0
                    && gen_best_fitness.1 < global_best_fitness.1)
            {
                if let Some(old_best) = &global_best_candidate
                    && global_best_fitness.0 != u32::MAX
                {
                    let diff_score = global_best_fitness.0.saturating_sub(gen_best_fitness.0);
                    if diff_score >= 20 {
                        objective.distill_theory(old_best, &tier2_pool[0].0, diff_score);
                    }
                }
                global_best_fitness = gen_best_fitness;
                global_best_candidate = Some(tier2_pool[0].0.clone());
                stagnation_counter = 0;
            } else {
                stagnation_counter += 1;
            }

            // Levy Flight Adaptive Injection
            let feedback = FunnelStagnation {
                counter: stagnation_counter,
                patience: self.config.stagnation_patience,
            };
            chaos_state.adapt_tweak(&mut tweak, &feedback);

            if stagnation_counter >= self.config.stagnation_patience {
                let retention_count = (self.config.tier1_population as f32
                    * self.config.tier2_retention_ratio)
                    as usize;
                let island_b_limit = retention_count * 7 / 10;

                println!(
                    "[System 1] Partial Reset! Genetic pool trapped. Preserving Island C (Deep Explorers) and clearing Island A & B."
                );
                if self.parent_population.len() >= retention_count {
                    let island_c = self.parent_population.split_off(island_b_limit);
                    self.parent_population = island_c;
                } else {
                    self.parent_population.clear();
                }

                // Do NOT reset global_best_fitness, otherwise we lose track of our best.
                // But we DO reset the stagnation_counter to give Island C time to evolve.
                stagnation_counter = 0;
            }
        }
    }
}
