use rustfft::{FftPlanner, num_complex::Complex};

/// Metrics returned by the FFT Chaos Analysis.
#[derive(Debug, Clone)]
pub struct ChaosMetrics {
    /// The Shannon entropy of the normalized power spectrum.
    /// Higher values indicate more "white noise" (random/chaotic).
    /// Lower values indicate structured cyclical patterns (predictable).
    pub spectral_entropy: f64,
    /// The index of the frequency bin with the highest power (excluding DC component).
    pub dominant_frequency_index: usize,
    /// The proportion of total power held by the dominant frequency.
    pub dominant_power_ratio: f64,
}

/// Analyzer for running FFT on time-series data to extract chaos/entropy metrics.
pub struct FftChaosAnalyzer {
    planner: FftPlanner<f64>,
}

impl FftChaosAnalyzer {
    pub fn new() -> Self {
        Self {
            planner: FftPlanner::new(),
        }
    }

    /// Analyzes a time series of float values (e.g. stock prices).
    /// Returns the ChaosMetrics for the sequence.
    pub fn analyze_time_series(&mut self, data: &[f64]) -> Option<ChaosMetrics> {
        let n = data.len();
        if n < 2 {
            return None; // Not enough data for FFT
        }

        // Convert input to Complex
        let mut buffer: Vec<Complex<f64>> = data
            .iter()
            .map(|&val| Complex { re: val, im: 0.0 })
            .collect();

        // Perform FFT
        let fft = self.planner.plan_fft_forward(n);
        fft.process(&mut buffer);

        // Compute power spectrum (magnitude squared)
        // We only care about the positive frequencies (up to Nyquist limit n/2)
        let half_n = n / 2;
        
        let mut power_spectrum = Vec::with_capacity(half_n);
        let mut total_power = 0.0;

        // Skip DC component (index 0) because we care about variations, not the mean value
        for i in 1..half_n {
            let magnitude_sq = buffer[i].norm_sqr();
            power_spectrum.push(magnitude_sq);
            total_power += magnitude_sq;
        }

        if total_power == 0.0 || power_spectrum.is_empty() {
            return Some(ChaosMetrics {
                spectral_entropy: 0.0,
                dominant_frequency_index: 0,
                dominant_power_ratio: 0.0,
            });
        }

        // Normalize power spectrum and compute Shannon Entropy
        let mut spectral_entropy = 0.0;
        let mut max_power = -1.0;
        let mut dominant_idx = 0;

        for (i, &power) in power_spectrum.iter().enumerate() {
            if power > max_power {
                max_power = power;
                dominant_idx = i + 1; // +1 because we skipped DC
            }
            
            let p_i = power / total_power;
            if p_i > 0.0 {
                spectral_entropy -= p_i * p_i.ln();
            }
        }

        // Normalize entropy to [0, 1] range based on max possible entropy ln(N)
        let max_entropy = (power_spectrum.len() as f64).ln();
        let normalized_entropy = if max_entropy > 0.0 {
            spectral_entropy / max_entropy
        } else {
            0.0
        };

        Some(ChaosMetrics {
            spectral_entropy: normalized_entropy,
            dominant_frequency_index: dominant_idx,
            dominant_power_ratio: max_power / total_power,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
