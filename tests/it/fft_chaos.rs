use model_go::fft_chaos::*;
use std::f64::consts::PI;

#[test]
fn test_sine_wave_low_entropy() {
    let mut analyzer = FftChaosAnalyzer::new();
    let n = 256;
    let mut data = Vec::with_capacity(n);
    
    // Pure sine wave
    for i in 0..n {
        data.push((2.0 * PI * 10.0 * (i as f64) / (n as f64)).sin());
    }

    let metrics = analyzer.analyze_time_series(&data).unwrap();
    // A pure sine wave should have near zero spectral entropy (only one frequency bin has power)
    assert!(metrics.spectral_entropy < 0.1, "Entropy too high for sine wave: {}", metrics.spectral_entropy);
    assert!(metrics.dominant_power_ratio > 0.9, "Power ratio too low for sine wave: {}", metrics.dominant_power_ratio);
    assert_eq!(metrics.dominant_frequency_index, 10);
}

#[test]
fn test_random_noise_high_entropy() {
    let mut analyzer = FftChaosAnalyzer::new();
    let n = 256;
    let mut data = Vec::with_capacity(n);
    
    // Pseudo-random white noise
    let mut seed = 12345;
    for _ in 0..n {
        seed = seed ^ (seed << 13);
        seed = seed ^ (seed >> 17);
        seed = seed ^ (seed << 5);
        let val = (seed as f64) / (u32::MAX as f64) * 2.0 - 1.0;
        data.push(val);
    }

    let metrics = analyzer.analyze_time_series(&data).unwrap();
    // White noise should have high spectral entropy
    assert!(metrics.spectral_entropy > 0.8, "Entropy too low for white noise: {}", metrics.spectral_entropy);
    assert!(metrics.dominant_power_ratio < 0.2, "Power ratio too high for white noise: {}", metrics.dominant_power_ratio);
}
