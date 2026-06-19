use model_go::{UniversalTensor, ObjectiveFunction, UniversalOptimizationEngine};

/// A Mock Biotech Plugin testing the mathematical abstraction.
/// Simulates calculating the "Free Energy" of a 3D Protein Lattice structure.
struct BiotechFreeEnergyPlugin {
    pub target_structure: [f32; 3], // The optimal 3D crystal coordinate to fold into
}

impl ObjectiveFunction<3> for BiotechFreeEnergyPlugin {
    fn evaluate(&self, tensor: &UniversalTensor<3>) -> f32 {
        let optimal_tensor = UniversalTensor::new(self.target_structure);
        let spatial_distance = tensor.distance(&optimal_tensor);
        
        // Return Free Energy Gradient (closer to 0.0 is more stable)
        spatial_distance
    }
}

#[test]
fn test_universal_optimization_math_core() {
    let mock_plugin = BiotechFreeEnergyPlugin {
        target_structure: [100.0, 50.0, -25.0], // The mathematically stable state
    };
    
    // We start completely randomly (Unfolded state)
    let initial_tensor = UniversalTensor::new([0.0, 0.0, 0.0]);
    let mut solver = UniversalOptimizationEngine::<1, 3>::new(initial_tensor, 42);
    
    let mut lowest_energy = f32::MAX;
    let mut best_tensor = initial_tensor.clone();

    // The core ModelGo Chaos engine will autonomously "bump" its way down the energy gradient
    for _ in 0..1000 {
        let next_tensor = solver.step_evolution(&mock_plugin);
        let energy = mock_plugin.evaluate(&next_tensor);
        
        if energy < lowest_energy {
            lowest_energy = energy;
            best_tensor = next_tensor.clone();
        }
    }
    
    // After 1000 chaotic evolutionary iterations, the math engine should have moved 
    // the protein coordinates significantly closer to the target optimal state without 
    // knowing ANY physics or biology logic internally!
    println!("Final Stable Energy: {}", lowest_energy);
    println!("Discovered Coordinates: {:?}", best_tensor.values);
    
    // The starting distance was sqrt(100^2 + 50^2 + (-25)^2) = ~114.56
    assert!(lowest_energy < 114.56, "The universal optimization engine failed to descend the energy gradient.");
}
