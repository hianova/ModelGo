use model_go::chaos_state::*;

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
    
    let mut _swan_caught = false;
    
    let next_state = step_forward_nd_with_hook(&state, &tweak, &mut rng, |dim, val| {
        // Because s=1.1, a black swan is highly likely
        assert!(val.abs() > 10.0);
        assert!(dim < 3);
        _swan_caught = true;
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

#[test]
fn test_serialization() {
    let state = ChaosState::<2, 3>::new([1.5, -2.5, 3.123]);
    let bytes = state.to_bytes();
    assert_eq!(bytes.len(), 5 * 4); // 2 weights + 3 bases

    let recovered = ChaosState::<2, 3>::from_bytes(&bytes).unwrap();
    assert_eq!(state.macro_weights, recovered.macro_weights);
    assert_eq!(state.base_values, recovered.base_values);
}
