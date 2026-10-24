use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use dungeon_gen::Generator;
use game_core::action::GameAction;
use game_core::puzzle::PuzzleState;
use game_core::state::GameState;
use game_core::world::PuzzleType;
use inference_adapter::{InferenceRequest, NpcInference};
use std::sync::Arc;
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
    Victory,
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
    TokenStream {
        npc_id: usize,
        token: String,
    },
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

        let (floor, player_start) = generator.generate_floor(56, 32, state.floor_number);
        state.current_floor = floor;
        state.player.position = player_start;

        // Initialize First Optional Quest
        state.player.active_quests.clear();
        state.player.active_quests.push(game_core::state::Quest {
            id: format!("quest_{}", state.floor_number),
            title: "Locate the Hidden Weapon Cache".to_string(),
            completed: false,
            target_count: 3,
            current_count: 0,
            reward_xp: 100,
        });

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
                AsyncEvent::TokenStream { npc_id, token } => {
                    // Ignore special control tokens in the live stream
                    let clean_tok = token.replace("<|im_end|>", "").replace("<|im_start|>", "");
                    if clean_tok.is_empty() {
                        continue;
                    }

                    if let Some(npc) = self
                        .state
                        .current_floor
                        .npcs
                        .iter_mut()
                        .find(|n| n.id == npc_id)
                    {
                        let prefix = format!("{}:", npc.name);
                        let placeholder = format!("{}: [Decrypting signal...]", npc.name);

                        if let Some(last_memory) = npc.memory.last_mut() {
                            if *last_memory == placeholder {
                                // First streamed token replaces the "Decrypting..." indicator
                                *last_memory = format!("{}: {}", npc.name, clean_tok.trim_start());
                            } else if last_memory.starts_with(&prefix) {
                                last_memory.push_str(&clean_tok);
                            } else {
                                npc.memory
                                    .push(format!("{}: {}", npc.name, clean_tok.trim_start()));
                            }
                        } else {
                            npc.memory
                                .push(format!("{}: {}", npc.name, clean_tok.trim_start()));
                        }
                    }
                }
                AsyncEvent::InferenceComplete { npc_id, response } => {
                    // Replace the streamed line with the final sanitized response
                    if let Some(npc) = self
                        .state
                        .current_floor
                        .npcs
                        .iter_mut()
                        .find(|n| n.id == npc_id)
                    {
                        let prefix = format!("{}:", npc.name);
                        if let Some(last_memory) = npc.memory.last_mut() {
                            if last_memory.starts_with(&prefix) {
                                *last_memory = format!("{}: {}", npc.name, response.dialogue);
                            } else {
                                npc.memory
                                    .push(format!("{}: {}", npc.name, response.dialogue));
                            }
                        } else {
                            npc.memory
                                .push(format!("{}: {}", npc.name, response.dialogue));
                        }
                    }

                   if let Some(act) = &response.proposed_action {
                        if act == "unlock_doors" || response.dialogue.contains("[unlock_doors]") {
                            for tile in self.state.current_floor.tiles.iter_mut() {
                                if let game_core::world::Tile::Door { is_open, .. } = tile {
                                    *is_open = true;
                                }
                            }
                            self.log("AI SYSTEM: Override accepted. Doors unlocked.".to_string());
                        } else if act == "reveal_map" || response.dialogue.contains("[reveal_map]") {
                            for explored in self.state.current_floor.explored.iter_mut() {
                                *explored = true;
                            }
                            self.log("AI SYSTEM: Sector map data downloaded.".to_string());
                        } else if act == "assimilation_accepted" || response.dialogue.contains("[assimilation_accepted]") {
                            self.state
                                .current_floor
                                .enemies
                                .retain(|e| e.name != "The Nexus Mind (Architect)");
                            self.log("NARRATIVE VICTORY: The Nexus Mind has accepted your logic and assimilated.".to_string());
                            
                            // Trigger the victory screen instantly!
                            self.ui_mode = UiMode::Victory;
                        }
                    } else if response.dialogue.contains("[assimilation_accepted]") {
                        // Hard fallback if proposed_action was None but the tag is in the text
                        self.state.current_floor.enemies.retain(|e| e.name != "The Nexus Mind (Architect)");
                        self.log("NARRATIVE VICTORY: The Nexus Mind has assimilated.".to_string());
                        self.ui_mode = UiMode::Victory;
                    }

                    self.log(format!(
                        "TTFT: {}ms | Total: {}ms | In: {} | Out: {} tok",
                        response.metrics.time_to_first_token_ms,
                        response.metrics.total_latency_ms,
                        response.metrics.prompt_tokens,
                        response.metrics.completion_tokens
                    ));

                    if let UiMode::Dialogue { is_waiting, .. } = &mut self.ui_mode {
                        *is_waiting = false;
                    }
                }
                AsyncEvent::InferenceError { npc_id, error } => {
                    self.log(format!("INFERENCE ERROR: {}", error));

                    if let Some(npc) = self
                        .state
                        .current_floor
                        .npcs
                        .iter_mut()
                        .find(|n| n.id == npc_id)
                    {
                        let prefix = format!("{}:", npc.name);
                        let err_line = format!(
                            "{}: My neural link is failing. I cannot process that right now.",
                            npc.name
                        );
                        if let Some(last_memory) = npc.memory.last_mut() {
                            if last_memory.starts_with(&prefix) {
                                *last_memory = err_line;
                            } else {
                                npc.memory.push(err_line);
                            }
                        } else {
                            npc.memory.push(err_line);
                        }
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
                        self.ui_mode = UiMode::HelpMenu;
                    }
                    KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => {
                        action = Some(GameAction::Move { dx: 0, dy: -1 });
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => {
                        action = Some(GameAction::Move { dx: 0, dy: 1 });
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => {
                        action = Some(GameAction::Move { dx: -1, dy: 0 });
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => {
                        action = Some(GameAction::Move { dx: 1, dy: 0 });
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        // DEV WARP: Instantly teleport to Floor 5 with boss-ready stats
                        self.state.floor_number = 5;
                        self.state.player.level = 10;
                        self.state.player.max_hp = 300;
                        self.state.player.hp = 300;
                        self.state.player.weapon = Some(game_core::state::Weapon {
                            name: "Dev Admin Blade".to_string(),
                            bonus_damage: 50,
                        });
                        
                        let mut generator = dungeon_gen::Generator::new(self.state.seed + 5);
                        let (floor, player_start) =
                            generator.generate_floor(56, 32, self.state.floor_number);
                        self.state.current_floor = floor;
                        self.state.player.position = player_start;
                        
                        self.log("DEV MODE: Warped to Floor 5. Stats maxed.".to_string());
                    }

                    KeyCode::Char('x') | KeyCode::Char('X') => {
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
                                attack_type: game_core::action::AttackType::Strike,
                            });
                        } else {
                            self.log("No adjacent drone in range for Strike.".to_string());
                        }
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
                                    let puzzle_seed = self
                                        .state
                                        .seed
                                        .wrapping_add(self.state.floor_number as u64 * 1000)
                                        .wrapping_add((tx as u64) * 31 + (ty as u64));
                                    self.ui_mode = UiMode::Puzzle {
                                        puzzle: puzzle.clone(),
                                        state: game_core::puzzle::PuzzleState::new(
                                            &puzzle,
                                            puzzle_seed,
                                        ),
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
                        action = Some(GameAction::UseItem { index: 0 });
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
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
                            if self.state.floor_number >= 5 {
                                // Cap the game at Floor 5 and trigger the ending!
                                self.ui_mode = UiMode::Victory;
                                self.log("ESCAPE VELOCITY ACHIEVED. Sector 7 conquered.".to_string());
                            } else {
                                self.state.floor_number += 1;
                                let mut generator = dungeon_gen::Generator::new(
                                    self.state.seed + self.state.floor_number as u64,
                                );
                                let (floor, player_start) =
                                    generator.generate_floor(56, 32, self.state.floor_number);
                                self.state.current_floor = floor;
                                self.state.player.position = player_start;
                                self.log(format!("Descended to Sector {}.", self.state.floor_number));

                                // Generate new Optional Quest for the next floor
                                self.state.player.active_quests.clear();
                                self.state.player.active_quests.push(game_core::state::Quest {
                                    id: format!("quest_{}", self.state.floor_number),
                                    title: if self.state.floor_number % 2 == 0 {
                                        "Purge 3 Security Drones".to_string()
                                    } else {
                                        "Locate the Hidden Weapon Cache".to_string()
                                    },
                                    completed: false,
                                    target_count: 3,
                                    current_count: 0,
                                    reward_xp: 100,
                                });
                            }
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
                KeyCode::Char('q') | KeyCode::Char('Q') => self.should_quit = true,
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    if let Err(e) = self.state.save_to_file("neural_save.json") {
                        self.log(format!("SAVE FAILED: {}", e));
                    } else {
                        self.log("SYSTEM SAVED to neural_save.json".to_string());
                        self.ui_mode = UiMode::Playing;
                    }
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    match game_core::state::GameState::load_from_file("neural_save.json") {
                        Ok(loaded_state) => {
                            self.state = loaded_state;
                            self.log("SYSTEM RESTORED from neural_save.json".to_string());
                            self.ui_mode = UiMode::Playing;
                        }
                        Err(e) => {
                            self.log(format!("LOAD FAILED: {}", e));
                        }
                    }
                }
                _ => {}
            },
            UiMode::Help | UiMode::HelpMenu => {
                if matches!(
                    key.code,
                    KeyCode::Esc
                        | KeyCode::Enter
                        | KeyCode::Char('?')
                        | KeyCode::Char('h')
                        | KeyCode::Char('H')
                ) {
                    self.ui_mode = UiMode::Playing;
                }
            }
            UiMode::Inventory => match key.code {
                KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.ui_mode = UiMode::Playing;
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    // Loot Crafting System
                    let patches: Vec<usize> = self.state.player.inventory.iter()
                        .enumerate()
                        .filter(|(_, item)| item.name == "Health Patch")
                        .map(|(i, _)| i)
                        .collect();

                    if patches.len() >= 2 {
                        self.state.player.inventory.remove(patches[1]); // Remove higher index first
                        self.state.player.inventory.remove(patches[0]);
                        self.state.player.inventory.push(game_core::world::Item {
                            id: self.state.turn_count as usize,
                            name: "Overclock Stim (+50 HP)".to_string(),
                            position: (0, 0),
                            // FIXED: Changed heal_amount to heal
                            item_type: game_core::world::ItemType::Consumable { heal: 50 },
                        });
                        self.log("CRAFTED: 2x Health Patch -> 1x Overclock Stim.".to_string());
                    } else {
                        self.log("CRAFTING FAILED: Need 2x Health Patches.".to_string());
                    }
                }
                _ => {}
            },
            UiMode::PuzzleSolved { .. } => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ')
                ) {
                    self.ui_mode = UiMode::Playing;
                }
            }
            UiMode::Puzzle {
                puzzle,
                state,
                coords,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        self.ui_mode = UiMode::Playing;
                    }
                    KeyCode::Backspace | KeyCode::Char('c') | KeyCode::Char('C') => {
                        if let game_core::puzzle::PuzzleState::TokenVault {
                            selected,
                            feedback,
                            ..
                        } = state
                        {
                            selected.clear();
                            *feedback = Some("Buffer cleared.".to_string());
                        }
                        self.ui_mode = current_mode;
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let idx = c.to_digit(10).unwrap() as usize;
                        match state {
                            game_core::puzzle::PuzzleState::TokenVault {
                                fragments,
                                selected,
                                feedback,
                                ..
                            } => {
                                if idx < fragments.len() {
                                    if let Some(pos) = selected.iter().position(|&x| x == idx) {
                                        selected.remove(pos);
                                    } else {
                                        selected.push(idx);
                                    }
                                    *feedback = None;
                                }
                            }
                            game_core::puzzle::PuzzleState::ContextCollapse {
                                logs,
                                selected,
                                feedback,
                                ..
                            } => {
                                if idx < logs.len() {
                                    if let Some(pos) = selected.iter().position(|&x| x == idx) {
                                        selected.remove(pos);
                                    } else {
                                        selected.push(idx);
                                    }
                                    *feedback = None;
                                }
                            }
                            _ => {}
                        }
                        self.ui_mode = current_mode;
                    }
                    KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                        match state {
                            game_core::puzzle::PuzzleState::QuantizationForge {
                                current_bits,
                                feedback,
                                ..
                            } => {
                                *current_bits = match *current_bits {
                                    2 => 4,
                                    4 => 8,
                                    8 => 16,
                                    _ => 32,
                                };
                                *feedback = None;
                            }
                            game_core::puzzle::PuzzleState::InferenceReactor {
                                cursor, feedback, ..
                            } => {
                                if *cursor > 0 {
                                    *cursor -= 1;
                                }
                                *feedback = None;
                            }
                            _ => {}
                        }
                        self.ui_mode = current_mode;
                    }
                    KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                        match state {
                            game_core::puzzle::PuzzleState::QuantizationForge {
                                current_bits,
                                feedback,
                                ..
                            } => {
                                *current_bits = match *current_bits {
                                    32 => 16,
                                    16 => 8,
                                    8 => 4,
                                    _ => 2,
                                };
                                *feedback = None;
                            }
                            game_core::puzzle::PuzzleState::InferenceReactor {
                                stages,
                                cursor,
                                feedback,
                                ..
                            } => {
                                if *cursor + 1 < stages.len() {
                                    *cursor += 1;
                                }
                                *feedback = None;
                            }
                            _ => {}
                        }
                        self.ui_mode = current_mode;
                    }
                    KeyCode::Char(' ') => {
                        if let game_core::puzzle::PuzzleState::InferenceReactor {
                            stages,
                            cursor,
                            selected,
                            feedback,
                            ..
                        } = state
                        {
                            if let Some(sel_idx) = *selected {
                                stages.swap(sel_idx, *cursor);
                                *selected = None;
                            } else {
                                *selected = Some(*cursor);
                            }
                            *feedback = None;
                        }
                        self.ui_mode = current_mode;
                    }
                    KeyCode::Enter => {
                        if state.evaluate_submission() {
                            let explanation = state.get_advanced_explanation();
                            let target_coords = *coords;
                            let solved_puzzle = puzzle.clone();

                            if let Some(game_core::world::Tile::Terminal { is_solved, .. }) = self
                                .state
                                .current_floor
                                .get_tile_mut(target_coords.0, target_coords.1)
                            {
                                *is_solved = true;
                            }

                            // Dispense key to inventory
                            self.state.player.inventory.push(game_core::world::Item {
                                id: self.state.turn_count as usize,
                                name: "Sector Key".to_string(),
                                position: (0, 0),
                                item_type: game_core::world::ItemType::Key {
                                    door_id: "exit".to_string(),
                                },
                            });

                            // AI Companion Buffs / Puzzle Rewards
                            let reward_msg = match solved_puzzle {
                                game_core::world::PuzzleType::TokenVault => {
                                    self.state.player.max_hp += 20;
                                    self.state.player.hp = self.state.player.max_hp;
                                    "Dr. Aris optimizes your link. Max HP +20! Sector Key dispensed."
                                }
                                game_core::world::PuzzleType::QuantizationForge => {
                                    self.state.player.xp += 150;
                                    "Maintenance Node 7 overclocks your servos. +150 XP! Sector Key dispensed."
                                }
                                game_core::world::PuzzleType::ContextCollapse => {
                                    self.state.player.energy = 150;
                                    "Security Terminal clears cache. Energy Overcharged! Sector Key dispensed."
                                }
                                game_core::world::PuzzleType::InferenceReactor => {
                                    self.state.player.xp += 75;
                                    "The Architect aligns your pipeline. Combat efficiency maxed. Sector Key dispensed."
                                }
                            };
                            self.log(reward_msg.to_string());

                            // Transition directly into the educational explanation screen!
                            self.ui_mode = UiMode::PuzzleSolved { explanation };
                        } else {
                            self.log("Terminal compilation failed. Check diagnostic.".to_string());
                            // Persist the updated `state.feedback` diagnostic on the puzzle popup
                            self.ui_mode = current_mode;
                        }
                    }
                    _ => {}
                }
            }
            UiMode::GameOver | UiMode::Victory => match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => self.should_quit = true,
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.state = game_core::state::GameState::new_test_state();
                    let mut generator = dungeon_gen::Generator::new(self.state.seed);
                    let (floor, player_start) =
                        generator.generate_floor(56, 32, self.state.floor_number);
                    self.state.current_floor = floor;
                    self.state.player.position = player_start;
                    self.state.player.active_quests.clear();
                    self.state.player.active_quests.push(game_core::state::Quest {
                        id: format!("quest_{}", self.state.floor_number),
                        title: "Locate the Hidden Weapon Cache".to_string(),
                        completed: false,
                        target_count: 3,
                        current_count: 0,
                        reward_xp: 100,
                    });

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
                    if key.code == KeyCode::Esc {
                        self.ui_mode = UiMode::Playing;
                    }
                    return;
                }

                match key.code {
                    KeyCode::Esc => self.ui_mode = UiMode::Playing,
                    KeyCode::Backspace => {
                        input_buffer.pop();
                        self.ui_mode = current_mode;
                    }
                    KeyCode::Char(c) => {
                        input_buffer.push(c);
                        self.ui_mode = current_mode;
                    }
                    KeyCode::Enter => {
                        if input_buffer.trim().is_empty() {
                            return;
                        }

                        let msg = input_buffer.clone();
                        input_buffer.clear();
                        *is_waiting = true;
                        let id = *npc_id;

                        // Save updated input_buffer and is_waiting state back to self.ui_mode!
                        self.ui_mode = current_mode;

                        // Extract live puzzle context from the current floor's terminal so the NPC can teach the player
                        let mut live_puzzle_context = String::new();
                        for y in 0..self.state.current_floor.height as i32 {
                            for x in 0..self.state.current_floor.width as i32 {
                                if let Some(game_core::world::Tile::Terminal {
                                    puzzle,
                                    is_solved: false,
                                }) = self.state.current_floor.get_tile(x, y)
                                {
                                    let p_seed = self
                                        .state
                                        .seed
                                        .wrapping_add(self.state.floor_number as u64 * 1000)
                                        .wrapping_add((x as u64) * 31 + (y as u64));
                                    let temp_state =
                                        game_core::puzzle::PuzzleState::new(&puzzle, p_seed);
                                    live_puzzle_context = temp_state.get_npc_teaching_context();
                                    break;
                                }
                            }
                        }

                        let npc_idx = self
                            .state
                            .current_floor
                            .npcs
                            .iter()
                            .position(|n| n.id == id)
                            .unwrap();
                        let npc = &mut self.state.current_floor.npcs[npc_idx];

                        let prior_history = npc.memory.join("\n");

                        npc.memory.push(format!("Player: {}", msg));
                        npc.memory
                            .push(format!("{}: [Decrypting signal...]", npc.name));

                        if npc.memory.len() > 12 {
                            npc.memory.drain(0..2);
                            npc.memory
                                .insert(0, "[System: Earlier memory archived]".to_string());
                        }

                        let req = InferenceRequest {
                            system_prompt: format!(
                                "You are {}. Personality: {}. Goal: {} CURRENT FLOOR TERMINAL DATA: {}\nIf the player logically convinces you to cooperate, open doors, or assimilate, you MUST include one of these exact strings in your response to trigger the game engine: [unlock_doors], [reveal_map], or [assimilation_accepted].",
                                npc.name, npc.personality, npc.goal, live_puzzle_context
                            ),
                            conversation_history: prior_history,
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

                        let (token_tx, mut token_rx) = mpsc::unbounded_channel::<String>();
                        let tx_tokens = tx.clone();
                        tokio::spawn(async move {
                            while let Some(token) = token_rx.recv().await {
                                let _ =
                                    tx_tokens.send(AsyncEvent::TokenStream { npc_id: id, token });
                            }
                        });

                        tokio::spawn(async move {
                            let result = tokio::time::timeout(
                                std::time::Duration::from_secs(90),
                                engine.generate_streaming(req, token_tx),
                            )
                            .await;

                            match result {
                                Ok(Ok(res)) => {
                                    let _ = tx.send(AsyncEvent::InferenceComplete {
                                        npc_id: id,
                                        response: res,
                                    });
                                }
                                Ok(Err(e)) => {
                                    let _ = tx.send(AsyncEvent::InferenceError {
                                        npc_id: id,
                                        error: e.to_string(),
                                    });
                                }
                                Err(_) => {
                                    let _ = tx.send(AsyncEvent::InferenceError {
                                        npc_id: id,
                                        error: "Inference timed out".to_string(),
                                    });
                                }
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
