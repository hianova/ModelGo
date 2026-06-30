use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[archive(check_bytes)]
pub enum RiskRating {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct DecisionResult {
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub risk_rating: RiskRating,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct TheoryDistillationResult {
    pub decision: DecisionResult,
    pub extracted_rule: Option<String>,
}

/// Represents the delta/mutation of a topology or state jump.
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct TopologyDiff {
    pub old_state: Vec<f32>,
    pub new_state: Vec<f32>,
    pub score_jump: f32,
    pub dimension: usize,
}

/// Interface for external data sources to feed into the decision engine.
/// Data is provided as a zero-copy rkyv archived byte slice or equivalent buffer.
pub trait ExternalDataSource {
    /// Fetch external data and return it as raw rkyv bytes
    fn fetch_context_bytes(&self, topic: &str) -> anyhow::Result<Vec<u8>>;
}

pub struct DecisionEngine;

impl DecisionEngine {
    pub fn evaluate<D: ExternalDataSource>(
        topic: &str,
        data_source: &D,
    ) -> anyhow::Result<Vec<u8>> {
        // 1. Fetch external context bytes via zero-copy interface
        let context_bytes = data_source.fetch_context_bytes(topic)?;

        // 2. Safely cast or describe the external context
        let context_str = format!(
            "External Data ({} bytes received via rkyv buffer)",
            context_bytes.len()
        );

        let prompt = format!(
            "Topic: {}\nContext: {}\n\nTask: Evaluate the risk. Reply with exactly one word: Low, Medium, High, or Critical.\nRisk:",
            topic, context_str
        );

        // 3. Dispatch to Parallel Engine using 0-token Logit Classification
        // Hardcoded token IDs for Gemma 2B: Low=5687, Medium=10555, High=3665, Critical=9906
        let candidates = [5687, 10555, 3665, 9906];
        let best_token =
            crate::router::get_fallback_engine().classify_logits(&prompt, &candidates)?;

        let risk_rating = match best_token {
            5687 => RiskRating::Low,
            3665 => RiskRating::High,
            9906 => RiskRating::Critical,
            _ => RiskRating::Medium, // 10555
        };

        let decision = DecisionResult {
            pros: vec!["Fast Logit Classification Used".to_string()],
            cons: vec![],
            risk_rating,
        };

        // 5. Serialize the final structure into rkyv bytes to return to caller
        let rkyv_bytes = rkyv::to_bytes::<_, 256>(&decision)
            .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))?;

        Ok(rkyv_bytes.into_vec())
    }

    /// Evaluates a mutation (diff) from the ChaosState.
    /// Uses 0-token classification to determine if it's a High Reward (Low Risk) mutation.
    /// If so, it performs Theory Distillation using LLM text generation to extract a logical rule.
    pub fn evaluate_mutation(diff_bytes: &[u8]) -> anyhow::Result<TheoryDistillationResult> {
        // 1. Deserialize the TopologyDiff using zero-copy validation
        let archived = rkyv::check_archived_root::<TopologyDiff>(diff_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to validate archived TopologyDiff: {}", e))?;

        // We can deserialize it for easy manipulation (or access archived fields directly)
        let diff: TopologyDiff = archived.deserialize(&mut rkyv::Infallible).unwrap();

        // 2. Format a prompt for 0-token logit classification
        let prompt = format!(
            "State mutated on dimension {}. Score jumped by {:.4}. Is this a high risk or low risk (high reward) mutation? Reply with exactly one word: Low, Medium, High, or Critical.\nRisk:",
            diff.dimension, diff.score_jump
        );

        let candidates = [5687, 10555, 3665, 9906];
        let best_token = crate::router::get_fallback_engine()
            .classify_logits(&prompt, &candidates)
            .unwrap_or(10555);

        let risk_rating = match best_token {
            5687 => RiskRating::Low,
            3665 => RiskRating::High,
            9906 => RiskRating::Critical,
            _ => RiskRating::Medium,
        };

        let mut extracted_rule = None;
        let mut pros = vec![];

        // 3. Theory Distillation
        // If it's a Low Risk (High Reward) jump, we ask the LLM *why* it worked.
        if risk_rating == RiskRating::Low {
            println!(
                "[DecisionEngine] High Reward Mutation Detected! Starting Theory Distillation..."
            );

            let distillation_prompt = format!(
                "Analyze this topology mutation. Dimension {} shifted, causing a massive score jump of {:.4}. Explain why this is effective and formulate it as a rule. Rule format: [RULE: <macro_strategy>]",
                diff.dimension, diff.score_jump
            );

            // Allow a short generation to extract Pros/Cons and the rule
            if let Ok(response) =
                crate::router::get_fallback_engine().generate_script(&distillation_prompt)
            {
                println!("[DecisionEngine] LLM Output: {}", response);

                pros.push("Extracted causal rule from LLM".to_string());

                // Extremely simple Regex/parsing for the mock example
                if let Some(start) = response.find("[RULE:") {
                    let from_start = &response[start + 6..];
                    if let Some(end) = from_start.find("]") {
                        let rule = from_start[..end].trim().to_string();
                        extracted_rule = Some(rule);
                        println!(
                            "[DecisionEngine] Distilled Theory: {}",
                            extracted_rule.as_ref().unwrap()
                        );
                    }
                }

                // Fallback for demonstration if no strict format was output
                if extracted_rule.is_none() {
                    extracted_rule = Some(format!("DIM_{}_XNOR_CASCADE", diff.dimension));
                    println!(
                        "[DecisionEngine] Distilled Theory (Heuristic Fallback): {}",
                        extracted_rule.as_ref().unwrap()
                    );
                }
            } else {
                eprintln!("[DecisionEngine] Failed to generate distillation script.");
            }
        }

        Ok(TheoryDistillationResult {
            decision: DecisionResult {
                pros,
                cons: vec![],
                risk_rating,
            },
            extracted_rule,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockDataSource;
    impl ExternalDataSource for MockDataSource {
        fn fetch_context_bytes(&self, topic: &str) -> anyhow::Result<Vec<u8>> {
            Ok(topic.as_bytes().to_vec())
        }
    }

    #[test]
    fn test_decision_engine_fetch() {
        let src = MockDataSource;
        let bytes = src.fetch_context_bytes("test_topic").unwrap();
        assert_eq!(bytes, b"test_topic");
    }

    #[test]
    fn test_topology_diff_serialization() {
        let diff = TopologyDiff {
            old_state: vec![1.0],
            new_state: vec![2.0],
            score_jump: 1.0,
            dimension: 0,
        };
        let bytes = rkyv::to_bytes::<_, 256>(&diff).unwrap();
        let archived = rkyv::check_archived_root::<TopologyDiff>(&bytes).unwrap();
        let decoded: TopologyDiff = archived.deserialize(&mut rkyv::Infallible).unwrap();
        assert_eq!(decoded.score_jump, diff.score_jump);
    }
}
