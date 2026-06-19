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

/// Interface for external data sources to feed into the decision engine.
/// Data is provided as a zero-copy rkyv archived byte slice or equivalent buffer.
pub trait ExternalDataSource {
    /// Fetch external data and return it as raw rkyv bytes
    fn fetch_context_bytes(&self, topic: &str) -> anyhow::Result<Vec<u8>>;
}

pub struct DecisionEngine;

impl DecisionEngine {
    pub fn evaluate<D: ExternalDataSource>(topic: &str, data_source: &D) -> anyhow::Result<Vec<u8>> {
        // 1. Fetch external context bytes via zero-copy interface
        let context_bytes = data_source.fetch_context_bytes(topic)?;
        
        // 2. Safely cast or describe the external context
        let context_str = format!("External Data ({} bytes received via rkyv buffer)", context_bytes.len());

        let prompt = format!(
            "Topic: {}\nContext: {}\n\nTask: Evaluate the risk. Reply with exactly one word: Low, Medium, High, or Critical.\nRisk:",
            topic, context_str
        );

        // 3. Dispatch to Parallel Engine using 0-token Logit Classification
        // Hardcoded token IDs for Gemma 2B: Low=5687, Medium=10555, High=3665, Critical=9906
        let candidates = [5687, 10555, 3665, 9906];
        let best_token = crate::router::get_fallback_engine()
            .classify_logits(&prompt, &candidates)?;

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
}

