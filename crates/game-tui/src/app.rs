use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use dungeon_gen::Generator;
use game_core::action::{GameAction, process_action};
use game_core::state::GameState;
use ratatui::{
    Terminal,
    backend::Backend,
    widgets::{Block, Borders, Paragraph},
};
use std::time::Duration;

#[derive(PartialEq)]
pub enum UiMode {
    Playing,
    Paused,
    Inventory,
    GameOver,
}

pub struct GameApp {
    pub state: GameState,
    pub ui_mode: UiMode,
    pub should_quit: bool,
    pub log_messages: Vec<String>,
}

impl GameApp {
    pub fn new() -> Self {
        let mut state = GameState::new_test_state();
        game_core::ai::process_engine_tick(&mut state);
        let mut generator = Generator::new(state.seed);

        let (floor, player_start) = generator.generate_floor(70, 30, state.floor_number);
        state.current_floor = floor;
        state.player.position = player_start;
        game_core::ai::update_fov(&mut state);
        Self {
            state,
            ui_mode: UiMode::Playing,
            should_quit: false,
            log_messages: vec![
                "Welcome to Neural Dungeon.".to_string(),
                format!("Run ID: {:X} | V1", 420193),
            ],
        }
    }

    pub fn log(&mut self, msg: String) {
        self.log_messages.push(msg);
        if self.log_messages.len() > 5 {
            self.log_messages.remove(0);
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

        match self.ui_mode {
            UiMode::Playing => {
                let mut action = None;

                match key.code {
                   KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => action = Some(GameAction::Move { dx: 0, dy: -1 }),
                    KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => action = Some(GameAction::Move { dx: 0, dy: 1 }),
                    KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => action = Some(GameAction::Move { dx: -1, dy: 0 }),
                    KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => action = Some(GameAction::Move { dx: 1, dy: 0 }),
                    
                    KeyCode::Char('e') | KeyCode::Char('E') => action = Some(GameAction::Evade),
                    KeyCode::Char(' ') | KeyCode::Char('.') => action = Some(GameAction::Wait),
                    
                    KeyCode::Char('f') | KeyCode::Char('F') => action = Some(GameAction::Interact), 
                    KeyCode::Char('g') | KeyCode::Char('G') => action = Some(GameAction::GrabItem),
                    KeyCode::Char('u') | KeyCode::Char('U') => action = Some(GameAction::UseItem { index: 0 }),
                    KeyCode::Char('i') | KeyCode::Char('I') => self.ui_mode = UiMode::Inventory,
                    KeyCode::Esc => self.ui_mode = UiMode::Paused,
                    KeyCode::Char('>') => {
                        let (px, py) = self.state.player.position;
                        if let Some(game_core::world::Tile::StairsDown) = self.state.current_floor.get_tile(px, py) {
                            self.state.floor_number += 1;
                            let mut generator = dungeon_gen::Generator::new(self.state.seed + self.state.floor_number as u64);
                            let (floor, player_start) = generator.generate_floor(40, 20, self.state.floor_number);
                            self.state.current_floor = floor;
                            self.state.player.position = player_start;
                            self.log(format!("Descended to Sector {}.", self.state.floor_number));
                        } else {
                            self.log("There are no stairs here.".to_string());
                        }
                    }                    KeyCode::Esc => self.ui_mode = UiMode::Paused,
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
            UiMode::Inventory => {
                if key.code == KeyCode::Esc || key.code == KeyCode::Char('i') {
                    self.ui_mode = UiMode::Playing;
                }
            }
            UiMode::GameOver => match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('r') => {
                    self.state = GameState::new_test_state();
                    self.ui_mode = UiMode::Playing;
                    self.log_messages.clear();
                    self.log("System rebooted. New clone deployed.".to_string());
                }
                _ => {}
            },
        }
    }
}

pub async fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> anyhow::Result<()> {
    let mut app = GameApp::new();

    loop {
        terminal.draw(|f| crate::rendering::render(f, &app))?;
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
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
