//! C-FFI Bridge for ChaosState Engine
//! Allows foreign languages (C, C++, Python) to invoke ModelGo's no_alloc stochastic engine.

use crate::chaos_state::{ChaosState, MicroTweak, RngState, step_forward_nd};

/// C-FFI wrapper to perform a 1-dimensional Chaos step.
/// 
/// # Safety
/// This function relies on raw pointers. The caller must ensure `macro_weights` points to
/// a valid array of f32 with exactly `branches` elements, and `base_value` points to a single f32.
/// `seed` must point to a valid u32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn modelgo_chaos_step_1d(
    macro_weights: *mut f32,
    branches: usize,
    base_value: *mut f32,
    tweak_s: f32,
    seed: *mut u32,
) {
    if macro_weights.is_null() || base_value.is_null() || seed.is_null() {
        return; // Prevent null pointer dereference
    }

    // Since we cannot initialize const generics at runtime via a simple FFI, 
    // we bridge the standard N=10 dimension.
    if branches == 10 {
        unsafe {
            let mut state = ChaosState::<10, 1>::new([*base_value]);
            
            // Copy in the weights
            for i in 0..10 {
                state.macro_weights[i] = *macro_weights.add(i);
            }

            let tweak = MicroTweak {
                s_exponent: tweak_s,
                max_elements: 1000,
            };

            let mut rng = RngState::new(*seed);

            // Execute Levy Flight
            let next_state = step_forward_nd(&state, &tweak, &mut rng);

            // Write back results
            for i in 0..10 {
                *macro_weights.add(i) = next_state.macro_weights[i];
            }
            *base_value = next_state.base_values[0];
            *seed = rng.seed;
        }
    }
}
