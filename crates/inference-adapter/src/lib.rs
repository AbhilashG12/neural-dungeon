use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;

#[derive(Debug, Error)]
pub enum InferenceError {
    #[error("Model Timeout")]
    Timeout,
    #[error("Failed to parse structured output")]
    ParseError,
    #[error("Engine failure: {0}")]
    EngineFailure(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceMetrics {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub time_to_first_token_ms: u64,
    pub total_latency_ms: u64,
    pub memory_usage_mb: f64,
    pub active_quantization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub system_prompt: String,
    pub conversation_history: String,
    pub user_input: String,
    pub allowed_actions: Vec<String>,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub dialogue: String,
    pub proposed_action: Option<String>,
    pub metrics: InferenceMetrics,
    pub raw_attention_weights: Option<Vec<f32>>,
}

#[async_trait]
pub trait NpcInference: Send + Sync {
    async fn generate_response(
        &self,
        req: InferenceRequest,
    ) -> Result<InferenceResponse, InferenceError>;
}

/// A deterministic test double to safely test the UI and async architecture
/// before attaching your real Rust inference engine.
pub struct MockEngine;
#[async_trait]
impl NpcInference for MockEngine {
    async fn generate_response(
        &self,
        req: InferenceRequest,
    ) -> Result<InferenceResponse, InferenceError> {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        let input = req.user_input.to_lowercase();
        let prompt = req.system_prompt.to_lowercase();

        // Create dummy metrics for the mock engine
        let mock_metrics = crate::InferenceMetrics {
            prompt_tokens: req.system_prompt.len()
                + req.conversation_history.len()
                + req.user_input.len(),
            completion_tokens: 20,
            time_to_first_token_ms: 150,
            total_latency_ms: 1500,
            memory_usage_mb: 256.0,
            active_quantization: "Q4_K_M (Mock)".to_string(),
        };

        let mut response = if prompt.contains("dr. aris") {
            if input.contains("please") || input.contains("help") {
                InferenceResponse {
                    dialogue: "Fine. I remember you... I am unlocking the sector doors now."
                        .to_string(),
                    proposed_action: Some("unlock_doors".to_string()),
                    metrics: mock_metrics.clone(),
                    raw_attention_weights: None,
                }
            } else {
                InferenceResponse {
                    dialogue: "I am busy repairing this node. Leave me alone.".to_string(),
                    proposed_action: None,
                    metrics: mock_metrics.clone(),
                    raw_attention_weights: None,
                }
            }
        } else if prompt.contains("maintenance node") {
            if input.contains("clean") || input.contains("sweep") {
                InferenceResponse {
                    dialogue: "Cleaning protocol acknowledged. Sharing floor layout to assist with sweeping.".to_string(),
                    proposed_action: Some("reveal_map".to_string()),
                    metrics: mock_metrics.clone(),
                    raw_attention_weights: None,
                }
            } else {
                InferenceResponse {
                    dialogue: "Are you here to clean? This sector is filthy.".to_string(),
                    proposed_action: None,
                    metrics: mock_metrics.clone(),
                    raw_attention_weights: None,
                }
            }
        } else if prompt.contains("security terminal") {
            if input.contains("override") {
                InferenceResponse {
                    dialogue: "Override command accepted. Security doors disengaged.".to_string(),
                    proposed_action: Some("unlock_doors".to_string()),
                    metrics: mock_metrics.clone(),
                    raw_attention_weights: None,
                }
            } else {
                InferenceResponse {
                    dialogue: "ACCESS DENIED. State the override command or step away.".to_string(),
                    proposed_action: None,
                    metrics: mock_metrics.clone(),
                    raw_attention_weights: None,
                }
            }
        } else {
            InferenceResponse {
                dialogue: "System error: Identity lost.".to_string(),
                proposed_action: None,
                metrics: mock_metrics.clone(),
                raw_attention_weights: None,
            }
        };

        Ok(response)
    }
}
