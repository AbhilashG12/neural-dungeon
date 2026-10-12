use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Floor,
    Wall,
    Door { is_open: bool, locked: bool },
    StairsDown,
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
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Item {
    pub id: usize,
    pub name: String,
    pub position: (i32, i32),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DungeonFloor {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Tile>, 
    pub enemies: Vec<Enemy>,
    pub items: Vec<Item>,
}

impl DungeonFloor {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            tiles: vec![Tile::Wall; width * height],
            enemies: Vec::new(),
            items: Vec::new(),
        }
    }

    pub fn get_tile(&self, x: i32, y: i32) -> Option<Tile> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some(self.tiles[(y * self.width as i32 + x) as usize])
    }

    pub fn set_tile(&mut self, x: i32, y: i32, tile: Tile) {
        if x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32 {
            self.tiles[(y * self.width as i32 + x) as usize] = tile;
        }
    }

    pub fn get_enemy_at(&self, x: i32, y: i32) -> Option<&Enemy> {
        self.enemies.iter().find(|e| e.position == (x, y))
    }
}
