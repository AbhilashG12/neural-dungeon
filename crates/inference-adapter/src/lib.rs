use async_trait::async_trait;
use model::engine::LlmEngine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tensor::sampler::GenerationConfig;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

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

    async fn generate_streaming(
        &self,
        req: InferenceRequest,
        token_tx: UnboundedSender<String>,
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
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        self.generate_streaming(req, tx).await
    }

    async fn generate_streaming(
        &self,
        req: InferenceRequest,
        token_tx: UnboundedSender<String>,
    ) -> Result<InferenceResponse, InferenceError> {
        // 1. Cleanly parse conversation history into ChatML turns (capped at last 4 valid lines)
        let mut chatml_history = String::new();
        let valid_lines: Vec<&str> = req
            .conversation_history
            .lines()
            .map(|l| l.trim())
            .filter(|l| {
                !l.is_empty()
                    && !l.contains("neural link is failing")
                    && !l.contains("ERROR:")
                    && !l.contains("[Decrypting signal...]")
                    && !l.contains("[System:")
            })
            .collect();

        let start_idx = valid_lines.len().saturating_sub(4);
        for line in &valid_lines[start_idx..] {
            if let Some(player_msg) = line.strip_prefix("Player:") {
                chatml_history.push_str(&format!(
                    "<|im_start|>user\n{}<|im_end|>\n",
                    player_msg.trim()
                ));
            } else if let Some((_speaker, npc_msg)) = line.split_once(':') {
                chatml_history.push_str(&format!(
                    "<|im_start|>assistant\n{}<|im_end|>\n",
                    npc_msg.trim()
                ));
            }
        }

        // 2. Dynamic Architectural Guide Prompt
        let prompt = format!(
            "<|im_start|>system\n\
            {system_prompt}\n\
            Directive: You are an entity inside a dark sci-fi facility. Stay strictly in character and guide the player using your technical knowledge so they can solve this sector's terminal puzzle and obtain the Sector Key. Reply concisely in 1 or 2 complete sentences.<|im_end|>\n\
            {history}\
            <|im_start|>user\n{input}<|im_end|>\n\
            <|im_start|>assistant\n",
            system_prompt = req.system_prompt,
            history = chatml_history,
            input = req.user_input.trim()
        );

        let engine = Arc::clone(&self.engine);

        // 3. Run generation on blocking thread with token streaming callback
        let gen_output = tokio::task::spawn_blocking(move || {
            let config = GenerationConfig {
                temperature: 0.4,
                top_k: 30,
                top_p: 0.85,
                max_tokens: 65,
                repetition_penalty: 1.30,
            };

            engine.generate_streaming(&prompt, config, move |token| {
                let _ = token_tx.send(token.to_string());
            })
        })
        .await
        .map_err(|e| InferenceError::EngineFailure(format!("Blocking task panicked: {}", e)))?;

        // 4. Sanitize final completed dialogue
        let mut clean_text = gen_output
            .text
            .replace("<|im_end|>", "")
            .replace("<|im_start|>", "")
            .lines()
            .next()
            .unwrap_or("...signal lost...")
            .trim()
            .to_string();

        // Strip self-prefixed speaker labels if the model outputs "Dr. Aris: ..."
        if let Some((prefix, rest)) = clean_text.split_once(':') {
            if prefix.contains("Dr. Aris")
                || prefix.contains("Maintenance")
                || prefix.contains("Security")
                || prefix.contains("Architect")
            {
                clean_text = rest.trim().to_string();
            }
        }

        let lower = clean_text.to_lowercase();
        if lower.contains("human being")
            || lower.contains("great question")
            || lower.contains("user of the game")
        {
            clean_text =
                "The neural signal flickers and distorts... focus on the terminal parameters."
                    .to_string();
        }

        // 5. Phase 7 Narrative Action Trigger:
        // Floor 1-3 NPCs provide hints to solve puzzles rather than bypassing doors.
        // Only The Architect / System Broadcast (Floor 5) triggers peaceful assimilation.
        let sys_lower = req.system_prompt.to_lowercase();
        let input_lower = req.user_input.to_lowercase();
        let mut proposed_action = None;

        if (sys_lower.contains("architect") || sys_lower.contains("system broadcast"))
            && req.allowed_actions.contains(&"assimilation_accepted".to_string())
            && (input_lower.contains("reason")
                || input_lower.contains("logic")
                || input_lower.contains("peace")
                || input_lower.contains("assimilate"))
        {
            proposed_action = Some("assimilation_accepted".to_string());
        }

        let metrics = InferenceMetrics {
            prompt_tokens: gen_output.prompt_tokens,
            completion_tokens: gen_output.completion_tokens,
            time_to_first_token_ms: gen_output.ttft_ms,
            total_latency_ms: gen_output.total_latency_ms,
            memory_usage_mb: gen_output.kv_memory_mb as f64,
            active_quantization: "BF16 -> FP32 (AVX2)".to_string(),
        };

        Ok(InferenceResponse {
            dialogue: if clean_text.is_empty() {
                "...radio silence...".to_string()
            } else {
                clean_text
            },
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
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        self.generate_streaming(req, tx).await
    }

    async fn generate_streaming(
        &self,
        req: InferenceRequest,
        token_tx: UnboundedSender<String>,
    ) -> Result<InferenceResponse, InferenceError> {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        let input = req.user_input.to_lowercase();
        let prompt = req.system_prompt.to_lowercase();

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

        let response = if prompt.contains("dr. aris") {
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

        // Simulate token streaming for mock engine
        for word in response.dialogue.split_inclusive(' ') {
            let _ = token_tx.send(word.to_string());
            tokio::time::sleep(Duration::from_millis(30)).await;
        }

        Ok(response)
    }
}
