pub mod assembly_funnel;
pub mod levy_search;

pub use assembly_funnel::{AssemblyFunnel, FunnelObserver};

/// A generic interface representing a scientific exploration or optimization objective.
/// This allows the engine to run Physics, Math, Cybersecurity, or Biology without being coupled
/// to the specific domain.
pub trait ScienceObjective<T: Clone> {
    /// Computes the fitness score of a candidate. Lower is better (e.g., Energy, or negative Yield).
    fn evaluate_fitness(&self, candidate: &T) -> f32;

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
    fn check_archival(&self, candidate: &T, fitness: f32) -> bool;
}
