use rkyv::{Archive, Deserialize, Serialize};
use serde::Deserialize as SerdeDeserialize;

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

/// Interface for external data sources to feed into the decision engine.
/// Data is provided as a zero-copy rkyv archived byte slice or equivalent buffer.
pub trait ExternalDataSource {
    /// Fetch external data and return it as raw rkyv bytes
    fn fetch_context_bytes(&self, topic: &str) -> anyhow::Result<Vec<u8>>;
}

pub struct DecisionEngine;

impl DecisionEngine {
    /// Evaluates a topic using the global LLM Fallback Engine and an external data source.
    /// Returns the result serialized as `rkyv` bytes.
    pub fn evaluate<D: ExternalDataSource>(topic: &str, data_source: &D) -> anyhow::Result<Vec<u8>> {
        // 1. Fetch external context bytes via zero-copy interface
        let context_bytes = data_source.fetch_context_bytes(topic)?;
        
        // 2. Safely cast or describe the external context
        // In a true zero-copy flow, we would route these bytes directly to the L1 engine's native buffer.
        let context_str = format!("External Data ({} bytes received via rkyv buffer)", context_bytes.len());

        let prompt = format!(
            "Topic: {}\nContext: {}\n\nTask: Provide a binary Pro/Con analysis and a Risk Rating (Low, Medium, High, Critical). Return ONLY valid JSON matching this schema: {{\"pros\": [\"...\"], \"cons\": [\"...\"], \"risk_rating\": \"...\"}}",
            topic, context_str
        );

        // 3. Dispatch to Parallel Engine
        let results = crate::router::get_fallback_engine()
            .generate_parallel(&[prompt])
            .map_err(|e| anyhow::anyhow!("Generation failed: {}", e))?;

        if let Some(json_str) = results.first() {
            #[derive(SerdeDeserialize)]
            struct LlmOutput {
                pros: Vec<String>,
                cons: Vec<String>,
                risk_rating: String,
            }

            // 4. Parse JSON back into structured payload
            let parsed: LlmOutput = serde_json::from_str(json_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse LLM output. Raw Output: {}\nError: {}", json_str, e))?;

            let risk_rating = match parsed.risk_rating.to_lowercase().as_str() {
                "low" => RiskRating::Low,
                "medium" => RiskRating::Medium,
                "high" => RiskRating::High,
                "critical" => RiskRating::Critical,
                _ => RiskRating::Medium, // Default
            };

            let decision = DecisionResult {
                pros: parsed.pros,
                cons: parsed.cons,
                risk_rating,
            };

            // 5. Serialize the final structure into rkyv bytes to return to caller
            let rkyv_bytes = rkyv::to_bytes::<_, 256>(&decision)
                .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))?;
                
            return Ok(rkyv_bytes.into_vec());
        }

        anyhow::bail!("Failed to generate decision")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockDataSource;
    impl ExternalDataSource for MockDataSource {
        fn fetch_context_bytes(&self, _topic: &str) -> anyhow::Result<Vec<u8>> {
            let data = String::from("Market is volatile but tech is solid.");
            let bytes = rkyv::to_bytes::<_, 256>(&data).unwrap();
            Ok(bytes.into_vec())
        }
    }

    #[test]
    fn test_decision_engine_serialization() {
        let decision = DecisionResult {
            pros: vec!["Fast".to_string()],
            cons: vec!["Expensive".to_string()],
            risk_rating: RiskRating::Medium,
        };

        // Serialize
        let bytes = rkyv::to_bytes::<_, 256>(&decision).unwrap();

        // Deserialize & Validate via bytecheck
        let archived = rkyv::check_archived_root::<DecisionResult>(&bytes).unwrap();
        
        let deserialized: DecisionResult = archived.deserialize(&mut rkyv::Infallible).unwrap();
        
        assert_eq!(deserialized.pros.len(), 1);
        assert_eq!(deserialized.pros[0].as_str(), "Fast");
        assert_eq!(deserialized.risk_rating, RiskRating::Medium);
    }
}
