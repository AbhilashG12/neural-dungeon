use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Floor,
    Wall,
    Door { is_open: bool, locked: bool },
    StairsDown,
    Terminal { puzzle: PuzzleType, is_solved: bool },
    Hazard { damage: i32 },
}

#[derive(Serialize, Deserialize, Copy, Clone, Eq, Debug, PartialEq)]
pub enum PuzzleType {
    TokenVault,
    QuantizationForge,
    ContextCollapse,
    InferenceReactor,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Npc {
    pub id: usize,
    pub name: String,
    pub symbol: char,
    pub position: (i32, i32),
    pub personality: String,
    pub goal: String,
    pub trust_level: i32,
    pub memory: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum EnemyState {
    Idle,
    Chasing,
    Telegraphing { target_pos: (i32, i32), damage: u32 },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Enemy {
    pub id: usize,
    pub position: (i32, i32),
    pub hp: u32,
    pub max_hp: u32,
    pub energy: u32,
    pub speed: u32,
    pub name: String,
    pub state: EnemyState,
    pub aggro_range: i32,
    pub hit_timer: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ItemType {
    Consumable { heal: u32 },
    Key { door_id: String },
    Weapon { damage: i32 },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Item {
    pub id: usize,
    pub name: String,
    pub position: (i32, i32),
    pub item_type: ItemType,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DungeonFloor {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Tile>,
    pub explored: Vec<bool>,
    pub enemies: Vec<Enemy>,
    pub items: Vec<Item>,
    pub npcs: Vec<Npc>,
}

impl DungeonFloor {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            tiles: vec![Tile::Wall; width * height],
            explored: vec![false; width * height],
            enemies: Vec::new(),
            items: Vec::new(),
            npcs: Vec::new(),
        }
    }
    pub fn get_tile_mut(&mut self, x: i32, y: i32) -> Option<&mut Tile> {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            Some(&mut self.tiles[(y * self.width as i32 + x) as usize])
        } else {
            None
        }
    }
    pub fn get_npc_at(&self, x: i32, y: i32) -> Option<&Npc> {
        self.npcs.iter().find(|n| n.position == (x, y))
    }

    pub fn get_tile(&self, x: i32, y: i32) -> Option<Tile> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some(self.tiles[self.idx(x, y)])
    }

    pub fn set_tile(&mut self, x: i32, y: i32, tile: Tile) {
        if x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32 {
            let idx = self.idx(x, y);
            self.tiles[idx] = tile;
        }
    }
    pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width as i32 + x) as usize
    }
    pub fn is_explored(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }
        self.explored[self.idx(x, y)]
    }

    pub fn get_enemy_at(&self, x: i32, y: i32) -> Option<&Enemy> {
        self.enemies.iter().find(|e| e.position == (x, y))
    }
}
