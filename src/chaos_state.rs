//! ChaosState Engine
//! A neuro-symbolic, no_alloc, pure static math engine for simulating state transitions.
//! Strictly utilizes `core` and `libm` to remain heap-independent.

use core::f32;

/// Lightweight, zero-allocation Xorshift32 PRNG.
pub struct RngState {
    pub seed: u32,
}

impl RngState {
    pub fn new(seed: u32) -> Self {
        Self {
            seed: if seed == 0 { 0x12345678 } else { seed },
        }
    }

    /// Generates a uniform f32 in range [0, 1)
    pub fn next_f32(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32) / (u32::MAX as f32)
    }

    /// Generates a Zipf-distributed random variable using inverse transform approximation.
    pub fn next_zipf(&mut self, tweak: &MicroTweak) -> f32 {
        let mut u = self.next_f32();
        if u == 0.0 {
            u = 0.000001; // Avoid division by zero
        }
        
        let s_minus_1 = tweak.s_exponent - 1.0;
        let s_minus_1 = if s_minus_1 < 0.01 { 0.01 } else { s_minus_1 };
        
        let p = -1.0 / s_minus_1;
        libm::powf(u, p)
    }
}

/// Tweakable parameters adjusting the micro-randomness of the system.
pub struct MicroTweak {
    pub s_exponent: f32,
    pub max_elements: u32,
}

/// Adaptive Interface for dynamical systems to auto-tune Chaos.
pub trait StagnationFeedback {
    /// Returns the current stagnation gradient (e.g. 0.0 means moving fast, 1.0 means completely stuck).
    fn current_gradient(&self) -> f32;
}

/// A node representing the system's "probability cloud" across N branches and D dimensions.
#[derive(Clone, Copy)]
pub struct ChaosState<const N: usize, const D: usize> {
    pub macro_weights: [f32; N],
    pub base_values: [f32; D],
}

impl<const N: usize, const D: usize> ChaosState<N, D> {
    pub fn new(initial_values: [f32; D]) -> Self {
        let mut weights = [0.0; N];
        if N > 0 { weights[0] = 1.0; } // Default to first branch taking full probability
        Self {
            macro_weights: weights,
            base_values: initial_values,
        }
    }

    /// Adapts the tweak parameter based on external stagnation feedback.
    /// If the system is highly stagnant (gradient approaches 1.0), it decreases `s_exponent` to induce chaos.
    pub fn adapt_tweak(&self, tweak: &mut MicroTweak, feedback: &impl StagnationFeedback) {
        let grad = feedback.current_gradient();
        // Lower s -> more chaos. Bound between 1.1 (Extreme chaos) and 3.0 (Gaussian-like).
        let target_s = 3.0 - (grad * 1.9);
        tweak.s_exponent = target_s.clamp(1.1, 3.0);
    }
}

/// Normalizes a float array in-place so that its elements sum to 1.0.
fn normalize_weights<const N: usize>(weights: &mut [f32; N]) {
    let sum: f32 = weights.iter().sum();
    if sum > 0.0 {
        for w in weights.iter_mut() {
            *w /= sum;
        }
    } else {
        let equiprob = 1.0 / (N as f32);
        for w in weights.iter_mut() {
            *w = equiprob;
        }
    }
}

/// Progresses the N-Dimensional ChaosState without a hook.
pub fn step_forward_nd<const N: usize, const D: usize>(
    current: &ChaosState<N, D>,
    tweak: &MicroTweak,
    rng: &mut RngState,
) -> ChaosState<N, D> {
    step_forward_nd_with_hook(current, tweak, rng, |_, _| {})
}

/// Progresses the N-Dimensional ChaosState with a Black Swan event callback.
pub fn step_forward_nd_with_hook<const N: usize, const D: usize, F>(
    current: &ChaosState<N, D>,
    tweak: &MicroTweak,
    rng: &mut RngState,
    mut on_black_swan: F,
) -> ChaosState<N, D> 
where 
    F: FnMut(usize, f32) // Passes dimension index and extreme value impact
{
    let mut next_weights = [0.0; N];
    let mut next_bases = current.base_values;
    
    for i in 0..N {
        let w = current.macro_weights[i];
        
        let r = rng.next_zipf(tweak);
        let direction = if rng.next_f32() > 0.5 { 1.0 } else { -1.0 };
        let impact = w * r * direction;
        
        // N-Dimensional Multi-variate Levy Flight Jump
        for dim in 0..D {
            // Apply a slight dimension-specific variance multiplier
            let dim_variance = (rng.next_f32() * 2.0) - 1.0; 
            let final_impact = impact * dim_variance;
            
            next_bases[dim] += final_impact;

            // Trigger Black Swan Event Hook if impact is extremely massive (e.g. > 10.0 or < -10.0)
            if libm::fabsf(final_impact) > 10.0 {
                on_black_swan(dim, final_impact);
            }
        }
        
        let volatility = libm::fabsf(impact);
        next_weights[i] = w + volatility * 0.01;
    }
    
    normalize_weights(&mut next_weights);

    ChaosState {
        macro_weights: next_weights,
        base_values: next_bases,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockFeedback(f32);
    impl StagnationFeedback for MockFeedback {
        fn current_gradient(&self) -> f32 { self.0 }
    }

    #[test]
    fn test_xorshift32_deterministic() {
        let mut rng1 = RngState::new(42);
        let mut rng2 = RngState::new(42);
        assert_eq!(rng1.next_f32(), rng2.next_f32());
    }

    #[test]
    fn test_normalize_weights() {
        let mut weights = [0.0, 10.0, 30.0, 60.0];
        normalize_weights(&mut weights);
        assert!((weights[0] - 0.0).abs() < f32::EPSILON);
        assert!((weights[1] - 0.1).abs() < f32::EPSILON);
        assert!((weights[2] - 0.3).abs() < f32::EPSILON);
        assert!((weights[3] - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn test_step_forward_nd_with_hooks() {
        let state = ChaosState::<5, 3>::new([100.0, 50.0, 0.0]); // 5 branches, 3 dimensions
        let tweak = MicroTweak { s_exponent: 1.1, max_elements: 1000 }; // Extreme chaos
        let mut rng = RngState::new(1337);
        
        let mut swan_caught = false;
        
        let next_state = step_forward_nd_with_hook(&state, &tweak, &mut rng, |dim, val| {
            // Because s=1.1, a black swan is highly likely
            assert!(val.abs() > 10.0);
            assert!(dim < 3);
            swan_caught = true;
        });
        
        // Assert we actually modified bases
        assert_ne!(next_state.base_values[0], 100.0);
        
        let sum: f32 = next_state.macro_weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }
    
    #[test]
    fn test_adaptive_tweak() {
        let state = ChaosState::<2, 1>::new([0.0]);
        let mut tweak = MicroTweak { s_exponent: 2.0, max_elements: 1000 };
        
        // High stagnation (1.0) should lower s to induce chaos
        state.adapt_tweak(&mut tweak, &MockFeedback(1.0));
        assert!((tweak.s_exponent - 1.1).abs() < f32::EPSILON);
        
        // Low stagnation (0.0) should raise s to converge
        state.adapt_tweak(&mut tweak, &MockFeedback(0.0));
        assert!((tweak.s_exponent - 3.0).abs() < f32::EPSILON);
    }
}
