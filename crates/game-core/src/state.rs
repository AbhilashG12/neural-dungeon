use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GameState {
    pub turn_count: u64,
    pub player: Player,
    pub current_floor: DungeonFloor,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Player{
    pub position: (i32,i32),
    pub hp: u32,
    pub max_hp: u32,
    pub energy: u32,
    pub inventory: Vec<Item>,
}
