use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use dungeon_gen::Generator;
use game_core::action::GameAction;
use game_core::puzzle::PuzzleState;
use game_core::state::GameState;
use game_core::world::{PuzzleType, Tile};
use inference_adapter::{InferenceRequest, MockEngine, NpcInference};
use ratatui::{Terminal, backend::Backend};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(PartialEq)]
pub enum Theme {
    Cyberpunk,
    Monochrome,
}

#[derive(PartialEq, Clone)]
pub enum UiMode {
    Playing,
    Paused,
    Inventory,
    GameOver,
    Help,
    Dialogue {
        npc_id: usize,
        input_buffer: String,
        is_waiting: bool,
    },
    Puzzle {
        puzzle: PuzzleType,
        state: PuzzleState,
        coords: (i32, i32),
    },
    PuzzleSolved {
        explanation: String,
    },
    HelpMenu,
}

pub struct GameApp {
    pub state: GameState,
    pub ui_mode: UiMode,
    pub should_quit: bool,
    pub log_messages: Vec<String>,
    pub async_tx: mpsc::UnboundedSender<AsyncEvent>,
    pub async_rx: mpsc::UnboundedReceiver<AsyncEvent>,
    pub inference_engine: Arc<dyn NpcInference>,
    pub theme: Theme,
}

pub enum AsyncEvent {
    InferenceComplete {
        npc_id: usize,
        response: inference_adapter::InferenceResponse,
    },
    InferenceError {
        npc_id: usize,
        error: String,
    },
}

impl GameApp {
    pub fn new() -> Self {
        let mut state = GameState::new_test_state();
        game_core::ai::process_engine_tick(&mut state);
        let mut generator = Generator::new(state.seed);

        let (floor, player_start) = generator.generate_floor(70, 30, state.floor_number);
        state.current_floor = floor;
        state.player.position = player_start;

        // Initialize the Tokio multi-producer, single-consumer channel for async inference
        let (tx, rx) = mpsc::unbounded_channel();
        let real_engine = inference_adapter::RealEngine::new();

        Self {
            state,
            ui_mode: UiMode::Playing,
            should_quit: false,
            log_messages: vec![
                "Welcome to Neural Dungeon.".to_string(),
                format!("Run ID: {:X} | V1", 420193),
            ],
            async_tx: tx,
            async_rx: rx,
            inference_engine: Arc::new(real_engine),
            theme: Theme::Cyberpunk,
            //inference_engine: Arc::new(real_engine),
        }
    }

    pub fn log(&mut self, msg: String) {
        self.log_messages.push(msg);
        if self.log_messages.len() > 5 {
            self.log_messages.remove(0);
        }
    }

    /// Processes messages sent back from the background LLM thread
    pub fn process_async_events(&mut self) {
        while let Ok(event) = self.async_rx.try_recv() {
            match event {
                // CORRECT PATTERN: We only extract npc_id and the unified response object
                AsyncEvent::InferenceComplete { npc_id, response } => {
                    if let Some(npc) = self
                        .state
                        .current_floor
                        .npcs
                        .iter_mut()
                        .find(|n| n.id == npc_id)
                    {
                        npc.memory
                            .push(format!("{}: {}", npc.name, response.dialogue));
                    }

                    if let Some(act) = &response.proposed_action {
                        if act == "unlock_doors" {
                            for tile in self.state.current_floor.tiles.iter_mut() {
                                if let game_core::world::Tile::Door { locked, .. } = tile {
                                    *locked = false;
                                }
                            }
                            self.log("AI SYSTEM: Override accepted. Doors unlocked.".to_string());
                        } else if act == "reveal_map" {
                            for explored in self.state.current_floor.explored.iter_mut() {
                                *explored = true;
                            }
                            self.log("AI SYSTEM: Sector map data downloaded.".to_string());
                        } else if act == "assimilation_accepted" {
                            self.state
                                .current_floor
                                .enemies
                                .retain(|e| e.name != "The Nexus Mind (Architect)");
                            self.log("NARRATIVE VICTORY: The Nexus Mind has accepted your logic and assimilated. Sector 7 is secure.".to_string());
                        }
                    }

                    self.log(format!(
                        "Inference: {}ms | In: {} tok | Out: {} tok",
                        response.metrics.total_latency_ms,
                        response.metrics.prompt_tokens,
                        response.metrics.completion_tokens
                    ));

                    if let UiMode::Dialogue { is_waiting, .. } = &mut self.ui_mode {
                        *is_waiting = false;
                    }
                }
                // CORRECT PATTERN: Extracting fields from the struct variant, not a tuple variant
                AsyncEvent::InferenceError { npc_id, error } => {
                    self.log(format!("INFERENCE ERROR: {}", error));

                    if let Some(npc) = self
                        .state
                        .current_floor
                        .npcs
                        .iter_mut()
                        .find(|n| n.id == npc_id)
                    {
                        npc.memory.push(format!(
                            "{}: My neural link is failing. I cannot process that right now.",
                            npc.name
                        ));
                    }

                    if let UiMode::Dialogue { is_waiting, .. } = &mut self.ui_mode {
                        *is_waiting = false;
                    }
                }
            }
        }
    }

    pub fn handle_input(&mut self, key: crossterm::event::KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        // We clone the mode to avoid borrow checker conflicts when mutating state
        let mut current_mode = self.ui_mode.clone();

        match &mut current_mode {
            UiMode::Playing => {
                let mut action = None;

                match key.code {
                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        self.theme = if self.theme == Theme::Cyberpunk {
                            Theme::Monochrome
                        } else {
                            Theme::Cyberpunk
                        };
                    }
                    KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('H') => {
                        self.ui_mode = UiMode::HelpMenu
                    }

                    KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => {
                        action = Some(GameAction::Move { dx: 0, dy: -1 })
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => {
                        action = Some(GameAction::Move { dx: 0, dy: 1 })
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => {
                        action = Some(GameAction::Move { dx: -1, dy: 0 })
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => {
                        action = Some(GameAction::Move { dx: 1, dy: 0 })
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') => action = Some(GameAction::Evade),
                    KeyCode::Char(' ') | KeyCode::Char('.') => action = Some(GameAction::Wait),

                    KeyCode::Char('f') | KeyCode::Char('F') => {
                        let (px, py) = self.state.player.position;
                        let dirs = [(0, 1), (0, -1), (1, 0), (-1, 0)];
                        let mut interacted = false;

                        for (dx, dy) in dirs {
                            let tx = px + dx;
                            let ty = py + dy;
                            if let Some(game_core::world::Tile::Terminal { puzzle, is_solved }) =
                                self.state.current_floor.get_tile(tx, ty)
                            {
                                if !is_solved {
                                    self.ui_mode = UiMode::Puzzle {
                                        puzzle: puzzle.clone(),
                                        state: game_core::puzzle::PuzzleState::new(&puzzle),
                                        coords: (tx, ty),
                                    };
                                } else {
                                    self.log("Terminal already bypassed.".to_string());
                                }
                                interacted = true;
                                break;
                            }
                            if let Some(npc) = self.state.current_floor.get_npc_at(tx, ty) {
                                self.ui_mode = UiMode::Dialogue {
                                    npc_id: npc.id,
                                    input_buffer: String::new(),
                                    is_waiting: false,
                                };
                                interacted = true;
                                break;
                            }
                        }
                        if !interacted {
                            action = Some(game_core::action::GameAction::Interact);
                        }
                    }
                    KeyCode::Char('g') | KeyCode::Char('G') => action = Some(GameAction::GrabItem),
                    KeyCode::Char('u') | KeyCode::Char('U') => {
                        action = Some(GameAction::UseItem { index: 0 })
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        // Find adjacent enemy to target with ability
                        let (px, py) = self.state.player.position;
                        let mut target_id = None;
                        for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
                            if let Some(enemy) =
                                self.state.current_floor.get_enemy_at(px + dx, py + dy)
                            {
                                target_id = Some(enemy.id);
                                break;
                            }
                        }
                        if let Some(id) = target_id {
                            action = Some(GameAction::Attack {
                                target_id: id,
                                attack_type: game_core::action::AttackType::Overload,
                            });
                        } else {
                            self.log("No adjacent target for Overload Strike.".to_string());
                        }
                    }

                    KeyCode::Char('i') | KeyCode::Char('I') => self.ui_mode = UiMode::Inventory,
                    KeyCode::Esc => self.ui_mode = UiMode::Paused,
                    KeyCode::Char('>') => {
                        let (px, py) = self.state.player.position;
                        if let Some(game_core::world::Tile::StairsDown) =
                            self.state.current_floor.get_tile(px, py)
                        {
                            self.state.floor_number += 1;
                            let mut generator = dungeon_gen::Generator::new(
                                self.state.seed + self.state.floor_number as u64,
                            );
                            let (floor, player_start) =
                                generator.generate_floor(70, 30, self.state.floor_number);
                            self.state.current_floor = floor;
                            self.state.player.position = player_start;
                            self.log(format!("Descended to Sector {}.", self.state.floor_number));
                        } else {
                            self.log("There are no stairs here.".to_string());
                        }
                    }
                    _ => {}
                }

                if let Some(act) = action {
                    match game_core::action::process_action(&mut self.state, act) {
                        Ok(action_logs) => {
                            for log in action_logs {
                                self.log(log);
                            }
                            let ai_logs = game_core::ai::process_engine_tick(&mut self.state);
                            for log in ai_logs {
                                self.log(log);
                            }
                            if self.state.player.hp == 0 {
                                self.ui_mode = UiMode::GameOver;
                                self.log("CRITICAL FAILURE: Vital signs lost.".to_string());
                            }
                        }
                        Err(e) => {
                            self.log(e.to_string());
                        }
                    }
                }
            }
            UiMode::Paused => match key.code {
                KeyCode::Esc => self.ui_mode = UiMode::Playing,
                KeyCode::Char('q') => self.should_quit = true,
                _ => {}
            },
            UiMode::Help => {
                if let KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('H') =
                    key.code
                {
                    self.ui_mode = UiMode::Playing;
                }
            }
            UiMode::Inventory => {
                if key.code == KeyCode::Esc
                    || key.code == KeyCode::Char('i')
                    || key.code == KeyCode::Char('I')
                {
                    self.ui_mode = UiMode::Playing;
                }
            }
            UiMode::HelpMenu => {
                if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
                    self.ui_mode = UiMode::Playing;
                }
            }
            UiMode::PuzzleSolved { .. } => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ')) {
                    self.ui_mode = UiMode::Playing;
                } else {
                    self.ui_mode = current_mode;
                }
            }
            UiMode::Puzzle {
                puzzle,
                state,
                coords,
            } => {
                let current_coords = *coords;
                let puzzle_type = puzzle.clone();
                let explanation = state.get_advanced_explanation().to_string();
                let mut solved = false;
                let mut abort = false;

                match key.code {
                    KeyCode::Esc => abort = true,
                    KeyCode::Enter => {
                        if state.check_win_condition() {
                            // 1. Mark the terminal as solved on the map
                            if let Some(game_core::world::Tile::Terminal { is_solved, .. }) =
                                self.state.current_floor.get_tile_mut(coords.0, coords.1)
                            {
                                *is_solved = true;
                            }

                            // 2. Dispense the mandatory progression item
                            self.state.player.inventory.push(game_core::world::Item {
                                id: self.state.turn_count as usize,
                                name: "Sector Key".to_string(),
                                position: (0, 0),
                                item_type: game_core::world::ItemType::Key {
                                    door_id: "exit".to_string(),
                                },
                            });

                            // 3. Display the educational lesson based on the puzzle type
                            let reward_msg = match puzzle {
                                game_core::world::PuzzleType::TokenVault => {
                                    "Token sequence accepted. Sector Key dispensed. (Lesson: LLMs process sub-word fragments)."
                                }
                                game_core::world::PuzzleType::QuantizationForge => {
                                    "Precision balanced. Sector Key dispensed. (Lesson: Q4 reduces VRAM with minimal loss)."
                                }
                                game_core::world::PuzzleType::ContextCollapse => {
                                    "Cache pruned. Sector Key dispensed. (Lesson: Finite KV caches require context budgeting)."
                                }
                                game_core::world::PuzzleType::InferenceReactor => {
                                    "Pipeline aligned. Sector Key dispensed. (Lesson: Embed -> Attention -> Logits -> Sample)."
                                }
                            };
                            self.log(reward_msg.to_string());

                            // 4. Close the puzzle overlay
                            self.ui_mode = UiMode::Playing;
                        } else {
                            self.log("ACCESS DENIED: Neural configuration invalid.".to_string());
                        }
                    }

                    KeyCode::Char(c) => match state {
                        game_core::puzzle::PuzzleState::TokenVault {
                            selected,
                            fragments,
                            ..
                        } => {
                            if let Some(digit) = c.to_digit(10) {
                                let idx = digit as usize;
                                if idx < fragments.len() {
                                    if let Some(pos) = selected.iter().position(|&x| x == idx) {
                                        selected.remove(pos);
                                    } else {
                                        selected.push(idx);
                                    }
                                }
                            }
                        }
                        game_core::puzzle::PuzzleState::ContextCollapse {
                            selected, logs, ..
                        } => {
                            if let Some(digit) = c.to_digit(10) {
                                let idx = digit as usize;
                                if idx < logs.len() {
                                    if let Some(pos) = selected.iter().position(|&x| x == idx) {
                                        selected.remove(pos);
                                    } else {
                                        selected.push(idx);
                                    }
                                }
                            }
                        }
                        game_core::puzzle::PuzzleState::InferenceReactor {
                            cursor,
                            stages,
                            selected,
                        } => {
                            if (c == 'w' || c == 'W') && *cursor > 0 {
                                *cursor -= 1;
                            }
                            if (c == 's' || c == 'S') && *cursor < stages.len() - 1 {
                                *cursor += 1;
                            }
                            if c == ' ' {
                                if let Some(sel_idx) = selected {
                                    stages.swap(*sel_idx, *cursor);
                                    *selected = None;
                                } else {
                                    *selected = Some(*cursor);
                                }
                            }
                        }
                        _ => {}
                    },
                    KeyCode::Up | KeyCode::Down => match state {
                        game_core::puzzle::PuzzleState::QuantizationForge { current_bits } => {
                            if key.code == KeyCode::Up && *current_bits < 32 {
                                *current_bits *= 2;
                            }
                            if key.code == KeyCode::Down && *current_bits > 1 {
                                *current_bits /= 2;
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }

                if abort {
                    self.ui_mode = UiMode::Playing;
                } else if solved {
                    if let Some(game_core::world::Tile::Terminal { is_solved, .. }) = self
                        .state
                        .current_floor
                        .get_tile_mut(current_coords.0, current_coords.1)
                    {
                        *is_solved = true;
                    }

                    let reward_msg = match puzzle_type {
                        game_core::world::PuzzleType::TokenVault => {
                            "REWARD: Plasma Battery. Terminal hacked."
                        }
                        game_core::world::PuzzleType::QuantizationForge => {
                            "REWARD: Sector Map. Terminal hacked."
                        }
                        game_core::world::PuzzleType::ContextCollapse => {
                            "REWARD: Max HP +20. Terminal hacked."
                        }
                        game_core::world::PuzzleType::InferenceReactor => {
                            "REWARD: Energy +50. Terminal hacked."
                        }
                    };
                    self.log(reward_msg.to_string());

                    self.ui_mode = UiMode::PuzzleSolved { explanation };
                } else {
                    self.ui_mode = current_mode;
                }
            }
            UiMode::GameOver => match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('r') => {
                    self.state = game_core::state::GameState::new_test_state();
                    let mut generator = dungeon_gen::Generator::new(self.state.seed);
                    let (floor, player_start) =
                        generator.generate_floor(40, 20, self.state.floor_number);
                    self.state.current_floor = floor;
                    self.state.player.position = player_start;
                    self.ui_mode = UiMode::Playing;
                    self.log_messages.clear();
                    self.log("SYSTEM REBOOT: New clone initialized.".to_string());
                }
                _ => {}
            },
            UiMode::Dialogue {
                npc_id,
                input_buffer,
                is_waiting,
            } => {
                if *is_waiting {
                    return;
                }

                match key.code {
                    KeyCode::Esc => self.ui_mode = UiMode::Playing,
                    KeyCode::Backspace => {
                        input_buffer.pop();
                        self.ui_mode = current_mode; // Save buffer back to state
                    }
                    KeyCode::Char(c) => {
                        input_buffer.push(c);
                        self.ui_mode = current_mode; // Save buffer back to state
                    }
                    KeyCode::Enter => {
                        if input_buffer.trim().is_empty() {
                            return;
                        }

                        let msg = input_buffer.clone();
                        input_buffer.clear();
                        *is_waiting = true;

                        let npc_idx = self
                            .state
                            .current_floor
                            .npcs
                            .iter()
                            .position(|n| n.id == *npc_id)
                            .unwrap();
                        let npc = &mut self.state.current_floor.npcs[npc_idx];

                        npc.memory.push(format!("Player: {}", msg));

                        if npc.memory.len() > 10 {
                            npc.memory.drain(0..2);
                            npc.memory
                                .insert(0, "[System: Earlier memory archived]".to_string());
                        }

                        let req = InferenceRequest {
                            system_prompt: format!(
                                "You are {}. Personality: {}. Goal: {}",
                                npc.name, npc.personality, npc.goal
                            ),
                            conversation_history: npc.memory.join("\n"),
                            user_input: msg,
                            allowed_actions: vec![
                                "unlock_doors".to_string(),
                                "reveal_map".to_string(),
                                "assimilation_accepted".to_string(),
                            ],
                            max_tokens: 150,
                        };

                        let tx = self.async_tx.clone();
                        let engine = self.inference_engine.clone();
                        let id = *npc_id;

                        tokio::spawn(async move {
                            let result = tokio::time::timeout(
                                std::time::Duration::from_secs(60),
                                engine.generate_response(req),
                            )
                            .await;

                            match result {
                                Ok(Ok(res)) => {
                                    tx.send(AsyncEvent::InferenceComplete {
                                        npc_id: id,
                                        response: res,
                                    })
                                    .unwrap();
                                }
                                Ok(Err(e)) => tx
                                    .send(AsyncEvent::InferenceError {
                                        npc_id: id,
                                        error: e.to_string(),
                                    })
                                    .unwrap(),
                                Err(_) => tx
                                    .send(AsyncEvent::InferenceError {
                                        npc_id: id,
                                        error: "Inference timed out".to_string(),
                                    })
                                    .unwrap(),
                            }
                        });
                    }
                    _ => {}
                }
            }
        }
    }
}

pub async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
) -> anyhow::Result<()> {
    let mut app = GameApp::new();
    terminal.clear()?;
    terminal.draw(|f| crate::rendering::render(f, &app))?;

    loop {
        app.process_async_events();
        terminal.draw(|f| crate::rendering::render(f, &app))?;

        if crossterm::event::poll(std::time::Duration::from_millis(16))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                app.handle_input(key);
            }
        }
        if app.should_quit {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};

    #[test]
    fn test_ui_mode_transitions() {
        let mut app = GameApp::new();
        assert!(app.ui_mode == UiMode::Playing);

        app.handle_input(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(app.ui_mode == UiMode::Paused);

        let pos_before = app.state.player.position;
        app.handle_input(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::empty()));
        assert_eq!(app.state.player.position, pos_before);

        app.handle_input(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(app.ui_mode == UiMode::Playing);
    }
}
