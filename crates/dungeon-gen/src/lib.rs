use game_core::world::{DungeonFloor, Enemy, EnemyState, Item, ItemType, Tile};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;

struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}
impl Rect {
    fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        }
    }
    fn center(&self) -> (i32, i32) {
        ((self.x1 + self.x2) / 2, (self.y1 + self.y2) / 2)
    }
    fn intersects(&self, other: &Rect) -> bool {
        self.x1 <= other.x2 && self.x2 >= other.x1 && self.y1 <= other.y2 && self.y2 >= other.y1
    }
}

pub struct Generator {
    rng: StdRng,
}

impl Generator {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn generate_floor(
        &mut self,
        width: usize,
        height: usize,
        depth: u32,
    ) -> (DungeonFloor, (i32, i32)) {
        let mut floor;
        let mut player_start;

        loop {
            floor = DungeonFloor::new(width, height);
            let mut rooms: Vec<Rect> = Vec::new();

            // 1. Place Rooms
            for _ in 0..15 {
                let w = self.rng.gen_range(5..10);
                let h = self.rng.gen_range(5..10);
                let x = self.rng.gen_range(1..(width as i32 - w - 1));
                let y = self.rng.gen_range(1..(height as i32 - h - 1));
                let new_room = Rect::new(x, y, w, h);

                let mut failed = false;
                for other in &rooms {
                    if new_room.intersects(other) {
                        failed = true;
                        break;
                    }
                }
                if !failed {
                    for ry in new_room.y1..new_room.y2 {
                        for rx in new_room.x1..new_room.x2 {
                            floor.set_tile(rx, ry, Tile::Floor);
                        }
                    }
                    rooms.push(new_room);
                }
            }

            // 2. Connect Corridors
            for i in 1..rooms.len() {
                let (px, py) = rooms[i - 1].center();
                let (cx, cy) = rooms[i].center();

                if self.rng.gen_bool(0.5) {
                    self.apply_horizontal_tunnel(&mut floor, px, cx, py);
                    self.apply_vertical_tunnel(&mut floor, py, cy, cx);
                } else {
                    self.apply_vertical_tunnel(&mut floor, py, cy, px);
                    self.apply_horizontal_tunnel(&mut floor, px, cx, cy);
                }
            }

            player_start = rooms[0].center();
            let stairs_pos = rooms.last().unwrap().center();
            floor.set_tile(stairs_pos.0, stairs_pos.1, Tile::StairsDown);

            // Spawn 3 distinct NPCs for Phase 5 compliance
            if rooms.len() > 3 {
                floor.npcs.push(game_core::world::Npc {
                    id: 1,
                    name: "Dr. Aris (Corrupted)".to_string(),
                    symbol: 'A',
                    position: (player_start.0 + 1, player_start.1),
                    personality: "Paranoid, highly technical, fragmented memory.".to_string(),
                    goal: "Protect the sector keys from unauthorized drones.".to_string(),
                    trust_level: 20,
                    memory: vec![],
                });

                let pos2 = rooms[1].center();
                floor.npcs.push(game_core::world::Npc {
                    id: 2,
                    name: "Maintenance Node 7".to_string(),
                    symbol: 'M',
                    position: pos2,
                    personality: "Depressed, bureaucratic, obsessed with cleaning protocols."
                        .to_string(),
                    goal: "Ensure the floors are devoid of organic material.".to_string(),
                    trust_level: 50,
                    memory: vec![],
                });

                let pos3 = rooms[2].center();
                floor.npcs.push(game_core::world::Npc {
                    id: 3,
                    name: "Security Terminal".to_string(),
                    symbol: 'T',
                    position: pos3,
                    personality: "Aggressive, suspicious, strictly adheres to protocol."
                        .to_string(),
                    goal: "Prevent access to the stairs.".to_string(),
                    trust_level: 0,
                    memory: vec![],
                });
            }

            // 3. Enemies and Loot (Difficulty scaling based on depth)
            for i in 1..rooms.len() - 1 {
                if self.rng.gen_bool(0.6) {
                    let center = rooms[i].center();
                    floor.enemies.push(Enemy {
                        id: self.rng.gen_range(1..999999),
                        position: center,
                        hp: 20 + (depth * 5),
                        max_hp: 20 + (depth * 5),
                        energy: 0,
                        speed: 15 + depth,
                        name: if self.rng.gen_bool(0.3) {
                            "Swarm Bot".to_string()
                        } else {
                            "Security Drone".to_string()
                        },
                        state: EnemyState::Idle,
                        aggro_range: 6,
                    });
                }
                if self.rng.gen_bool(0.2) {
                    floor.items.push(Item {
                        id: self.rng.gen_range(1..999999),
                        name: "Health Patch".to_string(),
                        position: (rooms[i].x1 + 1, rooms[i].y1 + 1),
                        item_type: ItemType::Consumable { heal: 25 },
                    });
                }
            }

            // Place the Sector Key in a middle room
            let key_room_idx = rooms.len() / 2;
            let key_pos = rooms[key_room_idx].center();
            floor.items.push(Item {
                id: 999,
                name: "Sector Key".to_string(),
                position: key_pos,
                item_type: ItemType::Key {
                    door_id: "exit".to_string(),
                },
            });

            // Place a locked door right in front of the stairs
            let (sx, sy) = stairs_pos;
            floor.set_tile(
                sx - 1,
                sy,
                Tile::Door {
                    is_open: false,
                    locked: true,
                },
            );
            floor.set_tile(
                sx + 1,
                sy,
                Tile::Door {
                    is_open: false,
                    locked: true,
                },
            );
            floor.set_tile(
                sx,
                sy - 1,
                Tile::Door {
                    is_open: false,
                    locked: true,
                },
            );
            floor.set_tile(
                sx,
                sy + 1,
                Tile::Door {
                    is_open: false,
                    locked: true,
                },
            );

            // Surround stairs with walls on corners to prevent diagonal bypass
            floor.set_tile(sx - 1, sy - 1, Tile::Wall);
            floor.set_tile(sx + 1, sy - 1, Tile::Wall);
            floor.set_tile(sx - 1, sy + 1, Tile::Wall);
            floor.set_tile(sx + 1, sy + 1, Tile::Wall);

            // 4. Validation (Check if Key is reachable, AND Stairs are reachable from Key)
            if self.validate_path(&floor, player_start, key_pos, false)
                && self.validate_path(&floor, key_pos, stairs_pos, true)
            {
                break; // Dungeon is valid, exit generation loop
            }
        }

        (floor, player_start)
    }

    fn apply_horizontal_tunnel(&self, floor: &mut DungeonFloor, x1: i32, x2: i32, y: i32) {
        for x in std::cmp::min(x1, x2)..=std::cmp::max(x1, x2) {
            floor.set_tile(x, y, Tile::Floor);
        }
    }

    fn apply_vertical_tunnel(&self, floor: &mut DungeonFloor, y1: i32, y2: i32, x: i32) {
        for y in std::cmp::min(y1, y2)..=std::cmp::max(y1, y2) {
            floor.set_tile(x, y, Tile::Floor);
        }
    }

    fn validate_path(
        &self,
        floor: &DungeonFloor,
        start: (i32, i32),
        end: (i32, i32),
        ignore_locked_doors: bool,
    ) -> bool {
        let mut queue = VecDeque::new();
        let mut visited = std::collections::HashSet::new();
        queue.push_back(start);
        visited.insert(start);

        while let Some(curr) = queue.pop_front() {
            if curr == end {
                return true;
            }
            for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
                let next = (curr.0 + dx, curr.1 + dy);
                if !visited.contains(&next) {
                    let tile = floor.get_tile(next.0, next.1).unwrap_or(Tile::Wall);
                    let passable = match tile {
                        Tile::Wall => false,
                        Tile::Door { locked: true, .. } => ignore_locked_doors,
                        _ => true,
                    };

                    if passable {
                        visited.insert(next);
                        queue.push_back(next);
                    }
                }
            }
        }
        false
    }
}
