use ratatui::{backend::Backend, widgets::{Block, Borders, Paragraph}, Terminal};
use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

pub async fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| {
            let size = f.size();
            let block = Paragraph::new("Neural Dungeon Core Initialized.\nPress 'q' to quit. Phase 2 will render the dungeon here.")
                .block(Block::default().title(" Neural Dungeon ").borders(Borders::ALL));
            f.render_widget(block, size);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    return Ok(());
                }
            }
        }
    }
}
