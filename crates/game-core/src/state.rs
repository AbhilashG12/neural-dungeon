use crate::world::{DungeonFloor, Enemy, EnemyState, Item, Tile};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GameState {
    pub turn_count: u64,
    pub seed: u64,
    pub floor_number: u32,
    pub player: Player,
    pub current_floor: DungeonFloor,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub completed: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub position: (i32, i32),
    pub hp: u32,
    pub max_hp: u32,
    pub energy: u32,
    pub speed: u32,
    pub inventory: Vec<Item>,
    pub active_quests: Vec<Quest>,
    pub evading: bool,
    pub max_inventory: usize,
}

impl GameState {
    pub fn new_test_state() -> Self {
        let width = 40;
        let height = 20;
        let mut floor = DungeonFloor::new(width, height);

        for y in 1..height - 1 {
            for x in 1..width - 1 {
                floor.set_tile(x as i32, y as i32, Tile::Floor)
            }
        }

        for y in 5..15 {
            if y == 10 {
                floor.set_tile(
                    20,
                    y as i32,
                    Tile::Door {
                        is_open: false,
                        locked: false,
                    },
                );
            } else {
                floor.set_tile(20, y as i32, Tile::Wall);
            }
        }
        floor.set_tile(35, 10, Tile::StairsDown);
        floor.enemies.push(Enemy {
            id: 1,
            position: (25, 10),
            hp: 30,
            max_hp: 30,
            energy: 0,
            speed: 15,
            name: "Security Drone".to_string(),
            state: EnemyState::Idle,
            aggro_range: 6,
        });
        Self {
            turn_count: 0,
            seed: 420193,
            floor_number: 1,
            player: Player {
                position: (5, 5),
                hp: 100,
                max_hp: 100,
                energy: 100,
                speed: 20,
                inventory: vec![],
                active_quests: vec![Quest {
                    id: "main_01".to_string(),
                    title: "Explore and locate Sector Key".to_string(),
                    completed: false,
                }],
                evading: false,
                max_inventory: 12,
            },
            current_floor: DungeonFloor::new(40, 20),
        }
    }
}
