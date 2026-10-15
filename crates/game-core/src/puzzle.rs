#[derive(Clone, Debug, PartialEq)]
pub enum PuzzleState {
    TokenVault {
        fragments: Vec<(&'static str, u32)>,
        selected: Vec<usize>,
        target: &'static str,
    },
    QuantizationForge {
        current_bits: u8,
    },
    ContextCollapse {
        logs: Vec<(&'static str, u32, bool)>, // (Text, Token Cost, Is Essential)
        selected: Vec<usize>,
        max_budget: u32,
    },
    InferenceReactor {
        stages: Vec<&'static str>,
        cursor: usize,
        selected: Option<usize>,
    },
}

impl PuzzleState {
    pub fn new(puzzle_type: &crate::world::PuzzleType) -> Self {
        match puzzle_type {
            crate::world::PuzzleType::TokenVault => PuzzleState::TokenVault {
                fragments: vec![
                    ("at", 204),
                    ("ten", 983),
                    ("tion", 401),
                    ("_", 90),
                    ("mech", 3302),
                    ("an", 112),
                    ("ism", 781),
                    ("attention", 8921),
                    ("mechan", 7712),
                    ("a", 43),
                ],
                selected: vec![],
                target: "attention_mechanism",
            },
            crate::world::PuzzleType::QuantizationForge => {
                PuzzleState::QuantizationForge { current_bits: 16 }
            }
            crate::world::PuzzleType::ContextCollapse => PuzzleState::ContextCollapse {
                logs: vec![
                    ("Sys_Log: Routine maintenance completed.", 15, false),
                    ("Cmd_Override: Alpha-7-Zulu.", 10, true), // Essential
                    ("Weather_Data: Raining in Sector 4.", 12, false),
                    ("Auth_Req: Security terminal requires override.", 18, true), // Essential
                ],
                selected: vec![],
                max_budget: 30,
            },
            crate::world::PuzzleType::InferenceReactor => PuzzleState::InferenceReactor {
                stages: vec!["Logits", "Attention", "Sample", "Tokenize", "Embed"], // Scrambled
                cursor: 0,
                selected: None,
            },
        }
    }

    pub fn get_advanced_explanation(&self) -> &'static str {
        match self {
            PuzzleState::TokenVault { .. } => {
                "ADVANCED: BPE (Byte-Pair Encoding) merges frequent character pairs. 'attention' might be ID [8921], but if misspelled, it fractures into 'at'[204], 'ten'[983]. This fracturing destroys semantic meaning in the embedding space."
            }
            PuzzleState::QuantizationForge { .. } => {
                "ADVANCED: FP16 uses 16 bits per weight. Q4_K_M groups weights into blocks (e.g., 32 weights) sharing a single FP16 scaling factor, rounding the rest to 4-bit integers. This allows a 7B model to run on 4GB VRAM instead of 14GB."
            }
            PuzzleState::ContextCollapse { .. } => {
                "ADVANCED: The KV Cache stores pre-computed Key and Value tensors for past tokens to avoid recomputing them. If the cache exceeds sequence length (e.g., 8192), the model must evict older tensors, causing 'Context Collapse' (forgetting)."
            }
            PuzzleState::InferenceReactor { .. } => {
                "ADVANCED: Input -> Tokenize (IDs) -> Embed (Dense Vectors) -> Attention (Contextual Mixing) -> Feed Forward (Non-linear projection) -> Logits (Raw scores) -> Softmax (Probabilities) -> Sample (Next ID)."
            }
        }
    }

    pub fn check_win_condition(&self) -> bool {
        match self {
            PuzzleState::TokenVault {
                fragments,
                selected,
                target,
            } => {
                let combined: String = selected.iter().map(|&i| fragments[i].0).collect();
                &combined == target && selected.len() > 2
            }
            PuzzleState::QuantizationForge { current_bits } => *current_bits == 4,
            PuzzleState::ContextCollapse {
                logs,
                selected,
                max_budget,
            } => {
                let cost: u32 = selected.iter().map(|&i| logs[i].1).sum();
                let has_all_essentials = logs
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| l.2)
                    .all(|(i, _)| selected.contains(&i));
                cost <= *max_budget && has_all_essentials
            }
            PuzzleState::InferenceReactor { stages, .. } => {
                stages == &["Tokenize", "Embed", "Attention", "Logits", "Sample"]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantization_win_state() {
        let mut puzzle = PuzzleState::new(&crate::world::PuzzleType::QuantizationForge);
        assert!(!puzzle.check_win_condition()); // Starts at 16-bit

        if let PuzzleState::QuantizationForge { current_bits } = &mut puzzle {
            *current_bits = 4;
        }
        assert!(puzzle.check_win_condition());
    }

    #[test]
    fn test_inference_reactor_alignment() {
        let mut puzzle = PuzzleState::new(&crate::world::PuzzleType::InferenceReactor);
        assert!(!puzzle.check_win_condition());

        if let PuzzleState::InferenceReactor { stages, .. } = &mut puzzle {
            *stages = vec!["Tokenize", "Embed", "Attention", "Logits", "Sample"];
        }
        assert!(puzzle.check_win_condition());
    }
}
