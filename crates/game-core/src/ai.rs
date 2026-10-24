use crate::action::ACTION_COST;
use crate::state::GameState;
use crate::world::{EnemyState, Tile};

pub fn update_fov(state: &mut GameState) {
    let (px, py) = state.player.position;
    let radius = 6;
    for y in (py - radius)..=(py + radius) {
        for x in (px - radius)..=(px + radius) {
            let dist_sq = (px - x) * (px - x) + (py - y) * (py - y);
            if dist_sq <= radius * radius {
                if x >= 0
                    && y >= 0
                    && x < state.current_floor.width as i32
                    && y < state.current_floor.height as i32
                {
                    let idx = state.current_floor.idx(x, y);
                    state.current_floor.explored[idx] = true;
                }
            }
        }
    }
}

pub fn process_engine_tick(state: &mut GameState) -> Vec<String> {
    // 1. Process Visual Polish Timers (Hit Flashes)
    if state.player.hit_timer > 0 {
        state.player.hit_timer -= 1;
    }
    for enemy in &mut state.current_floor.enemies {
        if enemy.hit_timer > 0 {
            enemy.hit_timer -= 1;
        }
    }

    let mut logs = Vec::new();
    
    // 2. Update Fog of War
    update_fov(state);

    // 3. Process Energy Tick Loop
    while state.player.energy < ACTION_COST {
        state.player.energy += state.player.speed;

        for i in 0..state.current_floor.enemies.len() {
            state.current_floor.enemies[i].energy += state.current_floor.enemies[i].speed;

            if state.current_floor.enemies[i].energy >= ACTION_COST {
                let log = execute_enemy_fsm(state, i);
                if let Some(msg) = log {
                    logs.push(msg);
                }
                state.current_floor.enemies[i].energy -= ACTION_COST;
            }
        }
    }

    if state.player.energy > ACTION_COST {
        state.player.energy = std::cmp::min(state.player.energy + 5, 100);
    }

    logs
}

fn execute_enemy_fsm(state: &mut GameState, enemy_idx: usize) -> Option<String> {
    let px = state.player.position.0;
    let py = state.player.position.1;
    let ex = state.current_floor.enemies[enemy_idx].position.0;
    let ey = state.current_floor.enemies[enemy_idx].position.1;

    let dist = ((px - ex).abs() + (py - ey).abs()) as i32;
    let aggro = state.current_floor.enemies[enemy_idx].aggro_range;
    let name = state.current_floor.enemies[enemy_idx].name.clone();

    let current_state = state.current_floor.enemies[enemy_idx].state.clone();

    match current_state {
        EnemyState::Idle => {
            if dist <= aggro {
                state.current_floor.enemies[enemy_idx].state = EnemyState::Chasing;
                return Some(format!("{} spots you!", name));
            }
        }
        EnemyState::Chasing => {
            if dist == 1 {
                state.current_floor.enemies[enemy_idx].state = EnemyState::Telegraphing {
                    target_pos: (px, py),
                    damage: 20,
                };
                return Some(format!("{} is preparing to strike!", name));
            } else if dist <= aggro * 2 {
                // BFS Pathfinding
                let mut queue = std::collections::VecDeque::new();
                let mut came_from = std::collections::HashMap::new();
                let start = (ex, ey);
                queue.push_back(start);

                while let Some(current) = queue.pop_front() {
                    if current == (px, py) {
                        break;
                    }

                    for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
                        let next = (current.0 + dx, current.1 + dy);
                        if !came_from.contains_key(&next)
                            && state
                                .current_floor
                                .get_tile(next.0, next.1)
                                .unwrap_or(Tile::Wall)
                                != Tile::Wall
                        {
                            came_from.insert(next, current);
                            queue.push_back(next);
                        }
                    }
                }

                let mut current = (px, py);
                let mut next_step = start;
                while let Some(&prev) = came_from.get(&current) {
                    if prev == start {
                        next_step = current;
                        break;
                    }
                    current = prev;
                }

                if state
                    .current_floor
                    .get_enemy_at(next_step.0, next_step.1)
                    .is_none()
                    && (next_step.0, next_step.1) != (px, py)
                {
                    state.current_floor.enemies[enemy_idx].position = next_step;
                }
            } else {
                state.current_floor.enemies[enemy_idx].state = EnemyState::Idle;
            }
        }
        EnemyState::Telegraphing { target_pos, damage } => {
            state.current_floor.enemies[enemy_idx].state = EnemyState::Chasing;
            if state.player.position == target_pos {
                if state.player.evading {
                    return Some(format!("{} attacks, but you evaded gracefully!", name));
                } else {
                    state.player.hp = state.player.hp.saturating_sub(damage);
                    state.player.hit_timer = 2; // TRIGGER VISUAL HIT FLASH ON PLAYER
                    return Some(format!("{} strikes you for {} damage!", name, damage));
                }
            } else {
                return Some(format!("{} swings at empty air!", name));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Player;
    use crate::world::{DungeonFloor, Enemy, Tile};

    #[test]
    fn test_telegraph_and_evade() {
        let mut floor = DungeonFloor::new(5, 5);
        for x in 0..5 {
            for y in 0..5 {
                floor.set_tile(x, y, Tile::Floor);
            }
        }

        let enemy = Enemy {
            id: 1,
            position: (2, 1),
            hp: 10,
            max_hp: 10,
            energy: 0,
            speed: 100,
            name: "Bot".to_string(),
            state: EnemyState::Telegraphing {
                target_pos: (1, 1),
                damage: 50,
            },
            aggro_range: 5,
            hit_timer: 0, // Added to fix test compile
        };
        floor.enemies.push(enemy);

        let mut state = GameState {
            turn_count: 0,
            seed: 0,
            floor_number: 1,
            player: Player {
                position: (1, 1),
                hp: 100,
                max_hp: 100,
                energy: 0,
                speed: 100,
                inventory: vec![],
                active_quests: vec![],
                evading: true,
                max_inventory: 10,
                hit_timer: 0, // Added to fix test compile
                level: 1,
                xp: 0,
                weapon: None,
            },
            current_floor: floor,
        };

        // Engine tick forces the enemy to execute its telegraphed attack
        let logs = process_engine_tick(&mut state);

        // Assert player dodged it because evading was true
        assert!(logs.iter().any(|msg| msg.contains("evaded gracefully")));
        assert_eq!(state.player.hp, 100); // Took no damage!
    }
}
