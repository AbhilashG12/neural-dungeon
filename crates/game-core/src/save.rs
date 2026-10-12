use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};

#[derive(Serialize, Deserialize)]
pub struct GameSnapshot {
    pub version: String,
    pub seed: u64,
    pub state: crate::state::GameState,
}

pub struct SaveManager {
    snapshot_path: String,
    wal_path: String,
    wal_writer: BufWriter<File>,
}

impl SaveManager {
    pub fn init(dir: &str) -> anyhow::Result<Self> {
        let wal_path = format!("{}/actions.wal", dir);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)?;

        Ok(Self {
            snapshot_path: format!("{}/snapshot.json", dir),
            wal_path,
            wal_writer: BufWriter::new(file),
        })
    }

    pub fn record_action(&mut self, action: &crate::action::GameAction) -> anyhow::Result<()> {
        let serialized = serde_json::to_string(action)?;
        writeln!(self.wal_writer, "{}", serialized)?;
        self.wal_writer.flush()?;
        Ok(())
    }

    pub fn checkpoint(&mut self, state: &crate::state::GameState) -> anyhow::Result<()> {
        let snapshot = GameSnapshot {
            version: env!("CARGO_PKG_VERSION").to_string(),
            seed: state.seed,
            state: state.clone(),
        };
        let file = File::create(&self.snapshot_path)?;
        serde_json::to_writer_pretty(file, &snapshot)?;
        self.wal_writer = BufWriter::new(File::create(&self.wal_path)?);
        Ok(())
    }
}
