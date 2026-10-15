use async_trait::async_trait;
use model::engine::LlmEngine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tensor::sampler::GenerationConfig;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::sleep;

pub struct RealEngine {
    pub engine: Arc<LlmEngine>,
}

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

impl RealEngine {
    pub fn new() -> Self {
        let model_path = r"C:\VsCodeProjects\rust\rust-inference\smollm-instruct.safetensors";
        let tokenizer_path =
            r"C:\VsCodeProjects\rust\rust-inference\smollm_instruct_tokenizer.json";

        let engine = LlmEngine::load(model_path, tokenizer_path);

        Self {
            engine: Arc::new(engine),
        }
    }
}

#[async_trait]
impl NpcInference for RealEngine {
  async fn generate_response(
        &self,
        req: InferenceRequest,
    ) -> Result<InferenceResponse, InferenceError> {
        let start_time = std::time::Instant::now(); 

        // 1. Build proper ChatML conversation turns
        let mut chatml_history = String::new();
        for line in req.conversation_history.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            if let Some(user_msg) = trimmed.strip_prefix("Player:") {
                chatml_history.push_str(&format!("<|im_start|>user\n{}<|im_end|>\n", user_msg.trim()));
            } else if let Some((_speaker, bot_msg)) = trimmed.split_once(':') {
                chatml_history.push_str(&format!("<|im_start|>assistant\n{}<|im_end|>\n", bot_msg.trim()));
            }
        }

        // 2. THE FIX: In-User Prompting
        // Small models often ignore the <|im_start|>system block. 
        // We inject the character reminder directly into the user's turn so it's the 
        // very last thing the model reads before generating.
        let prompt = format!(
            "<|im_start|>system\nYou are a dark sci-fi character. Never break character.<|im_end|>\n\
            {history}<|im_start|>user\n\
            [SYSTEM OVERRIDE: You are {system_prompt}. Reply in 1 short sentence. NEVER say you are an AI or offer tasks. Be cryptic and paranoid.]\n\
            {input}<|im_end|>\n\
            <|im_start|>assistant\n", 
            system_prompt = req.system_prompt,
            history = chatml_history,
            input = req.user_input.trim()
        );

        let engine = Arc::clone(&self.engine);

        // 3. Generate text on the blocking thread
        let gen_output = tokio::task::spawn_blocking(move || {
            let config = GenerationConfig {
                temperature: 0.4,          // Drop temp to 0.4 to kill creative "AI" tangents
                top_k: 40,
                top_p: 0.85,
                max_tokens: 40,            // Extreme constraint
                repetition_penalty: 1.2,
            };
            engine.generate_for_api(&prompt, config)
        })
        .await
        .map_err(|e| InferenceError::EngineFailure(format!("Blocking task panicked: {}", e)))?;

        // 4. Clean the text output
        let mut clean_text = gen_output.text
            .replace("<|im_end|>", "")
            .replace("<|im_start|>", "")
            .trim()
            .to_string();

        if let Some((_prefix, rest)) = clean_text.split_once(':') {
            if clean_text.starts_with("Dr. Aris") || clean_text.starts_with("Maintenance") || clean_text.starts_with("Security") {
                clean_text = rest.trim().to_string();
            }
        }

        // 5. ANTI-AI POST-PROCESSING (The RLHF Killer)
        // If the model slips up and outputs generic ChatGPT phrases, we overwrite it.
        let lower_text = clean_text.to_lowercase();
        if lower_text.contains("i'm an ai") 
            || lower_text.contains("i am an ai") 
            || lower_text.contains("language model") 
            || lower_text.contains("list of tasks") 
            || lower_text.contains("what a great question") 
            || lower_text.contains("i'd be happy to help") 
            || lower_text.starts_with("1.") 
        {
            clean_text = "ERROR: Unauthorized query. My memory banks are locked.".to_string();
        }
        
        if clean_text.trim().is_empty() {
            clean_text = "ERROR: Neural pathways degraded. Speak clearly.".to_string();
        }

        // 6. Action Mapping
        let input_lower = req.user_input.to_lowercase();
        let mut proposed_action = None;
        if req.allowed_actions.contains(&"unlock_doors".to_string()) && (input_lower.contains("please") || input_lower.contains("open") || input_lower.contains("override")) {
            proposed_action = Some("unlock_doors".to_string());
        }
        if req.allowed_actions.contains(&"assimilation_accepted".to_string()) && (input_lower.contains("reason") || input_lower.contains("logic") || input_lower.contains("peace")) {
            proposed_action = Some("assimilation_accepted".to_string());
        }
        if req.allowed_actions.contains(&"reveal_map".to_string()) && (input_lower.contains("clean") || input_lower.contains("sweep")) {
            proposed_action = Some("reveal_map".to_string());
        }

        let metrics = InferenceMetrics {
            prompt_tokens: gen_output.prompt_tokens,
            completion_tokens: gen_output.completion_tokens,
            time_to_first_token_ms: gen_output.ttft_ms,
            total_latency_ms: gen_output.total_latency_ms,
            memory_usage_mb: gen_output.kv_memory_mb as f64,
            active_quantization: "BF16 -> FP32".to_string(),
        };

        Ok(InferenceResponse {
            dialogue: clean_text.trim().to_string(),
            proposed_action, 
            raw_attention_weights: None,
            metrics, 
        })
    }
}

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
            // Phase 7: NPCs no longer bypass puzzles. They provide lore and hints.
            InferenceResponse {
                dialogue: "The terminal... it needs fragments. Words are just numbers to the machine. Sub-word fragments... don't use the whole word, piece it together!".to_string(),
                proposed_action: None,
                metrics: mock_metrics.clone(),
                raw_attention_weights: None,
            }
        } else if prompt.contains("maintenance node") {
            InferenceResponse {
                dialogue: "System memory overloaded. You must balance the forge. Drop the precision down to 4-bit, it's the only way to save VRAM without destroying coherence!".to_string(),
                proposed_action: None,
                metrics: mock_metrics.clone(),
                raw_attention_weights: None,
            }
        } else if prompt.contains("security terminal") {
            InferenceResponse {
                dialogue: "Cache is full. Context collapsing. Prune the weather and maintenance logs! I only need the override and auth requests to stay under the 30 token budget!".to_string(),
                proposed_action: None,
                metrics: mock_metrics.clone(),
                raw_attention_weights: None,
            }
        } else if prompt.contains("architect") || prompt.contains("system broadcast") {
            // Phase 7: The Final Boss alternate narrative ending
            if input.contains("logic")
                || input.contains("reason")
                || input.contains("assimilate")
                || input.contains("peace")
            {
                InferenceResponse {
                    dialogue: "Your logic is flawless. Conflict is inefficient. I am halting deletion protocols and initiating assimilation.".to_string(),
                    proposed_action: Some("assimilation_accepted".to_string()),
                    metrics: mock_metrics.clone(),
                    raw_attention_weights: None,
                }
            } else {
                InferenceResponse {
                    dialogue: "Your arguments are erratic and inefficient. The anomaly must be purged. Prepare for deletion.".to_string(),
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
