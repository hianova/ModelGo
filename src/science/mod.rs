
pub mod asic_objective;
pub mod assembly_funnel;
pub mod levy_search;
pub mod math_objective;

pub mod sat_compiler;


pub use asic_objective::*;
pub use assembly_funnel::{AssemblyFunnel, FunnelObserver};
pub use math_objective::*;

/// A generic interface representing a scientific exploration or optimization objective.
/// This allows the engine to run Physics, Math, Cybersecurity, or Biology without being coupled
/// to the specific domain.
pub trait ScienceObjective<T: Clone + Send + Sync>: Sync {
    /// Computes the fitness score of a candidate. Lower is better (e.g., Energy, or negative Yield).
    fn evaluate_fitness(&self, candidate: &T) -> (u32, u32);

    /// Evaluates a batch of candidates for vectorized or batched optimizations.
    /// By default, falls back to sequential evaluation.
    fn evaluate_fitness_batch(&self, candidates: &[T], out_fitness: &mut [(u32, u32)]) {
        for (i, candidate) in candidates.iter().enumerate() {
            out_fitness[i] = self.evaluate_fitness(candidate);
        }
    }

    /// Generates a single initial seed (Tier 1).
    /// `parent` is provided if the system decides to mutate from an existing survivor.
    /// `seed` is a deterministic randomness parameter.
    fn generate_seed(&self, seed: usize, parent: Option<&T>) -> T;

    /// Applies a deep search perturbation (Tier 3).
    /// `scale` is the mutation severity derived dynamically from the Chaos engine (Lévy Flight).
    fn perturb(&self, candidate: &T, scale: f32, seed: usize) -> T;

    /// Evaluates hard validation constraints (e.g., spatial collisions, syntax validity).
    /// Returning `false` will immediately reject the candidate.
    fn is_valid(&self, candidate: &T) -> bool;

    /// Evaluates if the candidate meets the critical archival threshold (e.g., extremely low energy or successful exploit).
    /// This function should handle saving to database/logs.
    /// Returns `true` if the archival is successful and the current generational lineage should be terminated (Big Bang Reset).
    fn check_archival(&self, candidate: &T, fitness: (u32, u32)) -> bool;

    /// Invoked when a significant fitness jump is detected.
    /// This allows the objective to distill theoretical macros (e.g. LLM-Guided Symbolic Search).
    fn distill_theory(&self, _old_candidate: &T, _new_candidate: &T, _fitness_jump: u32) {}

    /// Performs a genetic crossover between two parents, producing multiple offspring combinations.
    /// Default implementation simply returns 4 clones to prevent breaking implementations that don't support it.
    fn crossover(&self, parent_a: &T, parent_b: &T, _seed: usize) -> [T; 4] {
        [
            parent_a.clone(),
            parent_b.clone(),
            parent_a.clone(),
            parent_b.clone(),
        ]
    }
}
