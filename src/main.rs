use game_tui::app::run_app;
use std::fs::File;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_file = File::create("neural_dungeon_debug.log")?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(log_file)
        .with_ansi(false)
        .init();
    tracing::info!("Initializing Neural Dungeon Core Systems...");

    let mut terminal = game_tui::setup_terminal()?;

    let result = run_app(&mut terminal).await;

    game_tui::restore_terminal(&mut terminal)?;

    if let Err(e) = result {
        tracing::error!("Fatal Application error: {:?}", e);
        eprintln!("The game crashed. Check neural_dungeon_debug.log for details.");
    }
    Ok(())
}
