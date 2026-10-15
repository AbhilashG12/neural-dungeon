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
                    if new_room.x2 - new_room.x1 > 6
                        && new_room.y2 - new_room.y1 > 6
                        && self.rng.gen_bool(0.3)
                    {
                        let (cx, cy) = new_room.center();
                        floor.set_tile(cx, cy, Tile::Wall);
                        floor.set_tile(cx + 1, cy, Tile::Wall);
                        floor.set_tile(cx, cy + 1, Tile::Wall);
                        floor.set_tile(cx + 1, cy + 1, Tile::Wall);
                    }
                    rooms.push(new_room);
                }
            }

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

            floor.set_tile(sx - 1, sy - 1, Tile::Wall);
            floor.set_tile(sx + 1, sy - 1, Tile::Wall);
            floor.set_tile(sx - 1, sy + 1, Tile::Wall);
            floor.set_tile(sx + 1, sy + 1, Tile::Wall);

            let terminal_pos = if rooms.len() > 3 {
                rooms[3].center()
            } else {
                player_start
            };

            // 3. Enemies, Hazards, and Educational Content
            if depth == 5 {
                // BOSS ARENA: Floor 5
                floor.enemies.push(Enemy {
                    id: 9999,
                    position: rooms[rooms.len() / 2].center(),
                    hp: 300,
                    max_hp: 300,
                    energy: 0,
                    speed: 25,
                    name: "The Nexus Mind (Architect)".to_string(),
                    state: EnemyState::Idle,
                    aggro_range: 15,
                });

                floor.npcs.push(game_core::world::Npc {
                    id: 4,
                    position: player_start,
                    name: "System Broadcast".to_string(),
                    personality: "Cold, absolute, calculating.".to_string(),
                    goal: "Eradicate the anomaly.".to_string(),
                    trust_level: 0,
                    memory: vec![
                        "System: You have reached the core. Your deletion is mandatory."
                            .to_string(),
                    ],
                    symbol: 'B',
                });
            } else {
                // STANDARD FLOORS: Enemies and Hazards
                for i in 1..rooms.len() - 1 {
                    if self.rng.gen_bool(0.6) {
                        let center = rooms[i].center();
                        let (name, hp, speed) = match self.rng.gen_range(0..3) {
                            0 => ("Security Drone", 20 + (depth * 5), 15 + depth),
                            1 => ("Swarm Bot", 10 + (depth * 2), 22 + depth),
                            _ => ("Glitch Anomaly", 40 + (depth * 8), 10 + depth),
                        };

                        floor.enemies.push(Enemy {
                            id: self.rng.r#gen(),
                            position: center,
                            hp,
                            max_hp: hp,
                            energy: 0,
                            speed,
                            name: name.to_string(),
                            state: EnemyState::Idle,
                            aggro_range: 6,
                        });
                    }

                    if self.rng.gen_bool(0.3) {
                        let (cx, cy) = rooms[i].center();
                        floor.set_tile(cx + 1, cy, Tile::Hazard { damage: 15 });
                    }

                    if self.rng.gen_bool(0.2) {
                        floor.items.push(game_core::world::Item {
                            id: self.rng.gen_range(1..999999),
                            name: "Health Patch".to_string(),
                            position: (rooms[i].x1 + 1, rooms[i].y1 + 1),
                            item_type: game_core::world::ItemType::Consumable { heal: 25 },
                        });
                    }
                }

                // THEMATIC PROGRESSION: One unique puzzle and NPC per floor
                let (puzzle_type, npc_name, npc_personality, npc_goal, npc_sym) = match depth {
                    1 => (
                        game_core::world::PuzzleType::TokenVault,
                        "Dr. Aris (Corrupted)",
                        "Paranoid, fragmented memory. Obsessed with language.",
                        "Teach the player that words are broken into tokens.",
                        'A',
                    ),
                    2 => (
                        game_core::world::PuzzleType::QuantizationForge,
                        "Maintenance Node 7",
                        "Obsessed with efficiency and memory constraints.",
                        "Warn the player that too much precision crashes the system.",
                        'M',
                    ),
                    3 => (
                        game_core::world::PuzzleType::ContextCollapse,
                        "Security Terminal",
                        "Strict, forgets things easily due to cache limits.",
                        "Explain that older logs must be deleted to fit context budgets.",
                        'S',
                    ),
                    _ => (
                        game_core::world::PuzzleType::InferenceReactor,
                        "The Architect",
                        "God complex. Speaks in pipeline sequences.",
                        "Demand the player align the forward pass pipeline.",
                        'N',
                    ),
                };

                // Spawn the Terminal
                if rooms.len() > 3 {
                    let center = rooms[3].center();
                    floor.set_tile(
                        center.0,
                        center.1,
                        Tile::Terminal {
                            puzzle: puzzle_type,
                            is_solved: false,
                        },
                    );
                }

                // Spawn the NPC
                floor.npcs.push(game_core::world::Npc {
                    id: depth as usize,
                    position: (player_start.0 + 1, player_start.1),
                    name: npc_name.to_string(),
                    personality: npc_personality.to_string(),
                    goal: npc_goal.to_string(),
                    trust_level: 0,
                    memory: vec![],
                    symbol: npc_sym,
                });
            }

            if self.validate_path(&floor, player_start, terminal_pos, false)
                && self.validate_path(&floor, terminal_pos, stairs_pos, true)
            {
                break;
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
