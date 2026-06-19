use model_go::chaos_state::{ChaosState, MicroTweak, RngState, step_forward_nd};

fn main() {
    println!("=== ChaosState Macro Discovery Experiment ===");
    println!("Comparing Linear Statistical Counting vs Zipf Stochastic Learning\n");

    let num_simulations = 10000;
    
    // Tweak parameter identical to self_evolving_loop
    let tweak = MicroTweak {
        s_exponent: 1.5,
        max_elements: 1000,
    };

    let mut linear_attempts_total = 0;
    let mut chaos_attempts_total = 0;

    let mut min_chaos = u32::MAX;
    let mut max_chaos = 0;
    
    let mut chaos_distribution = [0; 20]; // Count how many times it took N attempts

    // Use a baseline seed
    let mut rng = RngState::new(0x2026);

    for _ in 0..num_simulations {
        // 1. Linear System (Always exactly 3)
        let linear_attempts = 3;
        linear_attempts_total += linear_attempts;

        // 2. Chaos System
        let mut state = ChaosState::<10, 1>::new([0.0]);
        let mut chaos_attempts = 0;
        
        loop {
            chaos_attempts += 1;
            state = step_forward_nd(&state, &tweak, &mut rng);
            
            // The exact threshold used in self_evolving_loop
            if state.base_values[0].abs() > 2.0 {
                break;
            }
            
            // Failsafe to avoid infinite loops in extreme edge cases of the random walk
            if chaos_attempts > 100 {
                break;
            }
        }
        
        chaos_attempts_total += chaos_attempts;
        
        if chaos_attempts < min_chaos { min_chaos = chaos_attempts; }
        if chaos_attempts > max_chaos { max_chaos = chaos_attempts; }
        
        let bucket = std::cmp::min(chaos_attempts as usize, 19);
        chaos_distribution[bucket] += 1;
    }

    println!("Simulation Runs: {}", num_simulations);
    println!("--------------------------------------------------");
    println!("[Linear Statistical Counter]");
    println!("Average Attempts to Learn: {:.2}", (linear_attempts_total as f64) / (num_simulations as f64));
    println!("Absolute Determinism. Always takes exactly 3 attempts.\n");

    println!("[ChaosState Zipf Learning]");
    println!("Average Attempts to Learn: {:.2}", (chaos_attempts_total as f64) / (num_simulations as f64));
    println!("Min Attempts (Instant Discovery): {}", min_chaos);
    println!("Max Attempts (Slow Convergence):  {}", max_chaos);
    println!("Distribution of Attempts Needed:");
    for i in 1..20 {
        if chaos_distribution[i] > 0 {
            let percentage = (chaos_distribution[i] as f64 / num_simulations as f64) * 100.0;
            // Draw a basic ascii bar
            let bars = "*".repeat((percentage / 2.0) as usize);
            println!("{:2} attempts: {:>5.1}% | {}", i, percentage, bars);
        }
    }
    
    println!("\nConclusion:");
    println!("While a linear counter enforces a rigid 'wait for 3 times' rule, ChaosState");
    println!("allows the system to 'intuitively' grasp macro logic instantly (1 attempt)");
    println!("when extreme Zipf outliers occur, bridging deterministic code with human-like 'Aha!' moments.");
}
