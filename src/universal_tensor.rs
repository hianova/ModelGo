//! Universal Tensor Abstraction
//! 
//! Provides a domain-agnostic representation of a mathematical state.
//! Can represent anything from 3D biological lattice coordinates to 
//! multi-dimensional logic gate routing weights.

use core::f32;

/// A generic multidimensional mathematical state.
/// D represents the dimensionality of the domain constraint.
#[derive(Debug, Clone, Copy)]
pub struct UniversalTensor<const D: usize> {
    pub values: [f32; D],
}

impl<const D: usize> UniversalTensor<D> {
    pub fn new(initial_values: [f32; D]) -> Self {
        Self {
            values: initial_values,
        }
    }

    /// Helper to measure the Euclidean distance to another tensor.
    /// Useful for domain plugins calculating spatial metrics.
    pub fn distance(&self, other: &Self) -> f32 {
        let mut sum_sq = 0.0;
        for i in 0..D {
            let diff = self.values[i] - other.values[i];
            sum_sq += diff * diff;
        }
        libm::sqrtf(sum_sq)
    }

    /// Access inner array
    pub fn as_slice(&self) -> &[f32; D] {
        &self.values
    }

    pub fn as_mut_slice(&mut self) -> &mut [f32; D] {
        &mut self.values
    }
}

impl<const D: usize> Default for UniversalTensor<D> {
    fn default() -> Self {
        Self { values: [0.0; D] }
    }
}
