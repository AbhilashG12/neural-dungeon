use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InferenceError {
    #[error("Model Timeout")]
    Timeout,
    #[error("Failed to parse structured output")]
    ParseError,
}

#[derive(Debug, Clone)]
pub struct InferenceMetrics {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub time_to_first_token_ms: u64,
    pub total_latency_ms: u64,
    pub memory_usage_mb: f64,
    pub active_quantization: String,
}

pub struct InferenceRequest {
    pub context: String,
    pub user_input: String,
    pub allowed_actions: Vec<String>,
}

pub struct InferenceResponse {
    pub dialogue: String,
    pub proposed_action: Option<String>,
    pub metrics: InferenceMetrics,
    pub raw_attention_weights: Option<Vec<f32>>,
}

#[async_trait]
pub trait NpcInference {
    async fn generate_response(
        &self,
        req: InferenceRequest,
    ) -> Result<InferenceResponse, InferenceError>;
}
