use ratatui::{backend::Backend, widgets::{Block, Borders, Paragraph}, Terminal};
use crossterm::event::{self, Event, KeyCode, KeyModifiers, KeyEventKind};
use std::time::Duration;
use game_core::state::GameState;
use game_core::action::{GameAction, process_action};

#[derive(PartialEq)]
pub enum UiMode {
    Playing,
    Paused,
    Inventory,
}

pub struct GameApp {
    pub state: GameState,
    pub ui_mode: UiMode,
    pub should_quit: bool,
    pub log_messages: Vec<String>,
}

impl GameApp{
    pub fn new() -> Self {
        Self {
            state: GameState::new_test_state(),
            ui_mode: UiMode::Playing,
            should_quit: false,
            log_messages: vec!["Welcome to Neural Dungeon.".to_string()],
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
                    KeyCode::Char('w') | KeyCode::Up => action = Some(GameAction::Move { dx: 0, dy: -1 }),
                    KeyCode::Char('s') | KeyCode::Down => action = Some(GameAction::Move { dx: 0, dy: 1 }),
                    KeyCode::Char('a') | KeyCode::Left => action = Some(GameAction::Move { dx: -1, dy: 0 }),
                    KeyCode::Char('d') | KeyCode::Right => action = Some(GameAction::Move { dx: 1, dy: 0 }),
                    KeyCode::Char('i') => self.ui_mode = UiMode::Inventory,
                    KeyCode::Char('>') | KeyCode::Char('.') => action = Some(GameAction::Descend),
                    KeyCode::Esc => self.ui_mode = UiMode::Paused,
                    _ => {}
                }

                if let Some(act) = action {
                    match process_action(&mut self.state, act) {
                        Ok(_) => {
                        }
                        Err(e) => {
                            self.log(e.to_string());
                        }
                    }
                }
            }
            UiMode::Paused => {
                match key.code {
                    KeyCode::Esc => self.ui_mode = UiMode::Playing,
                    KeyCode::Char('q') => self.should_quit = true,
                    _ => {}
                }
            }
            UiMode::Inventory => {
                if key.code == KeyCode::Esc || key.code == KeyCode::Char('i') {
                    self.ui_mode = UiMode::Playing;
                }
            }
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
