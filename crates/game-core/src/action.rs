use crate::state::GameState;
use crate::world::{ItemType, Tile};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttackType {
    Strike,
    Overload,
}

#[derive(Debug, Error)]
pub enum GameError {
    #[error("Movement blocked by a wall.")]
    BlockedByWall,
    #[error("Movement blocked by a locked door.")]
    DoorLocked,
    #[error("Movement blocked by an entity.")]
    BlockedByEntity,
    #[error("Target not found.")]
    InvalidTarget,
    #[error("Not enough energy.")]
    InsufficientEnergy,
    #[error("Inventory is full.")]
    InventoryFull,
    #[error("Nothing to pick up here.")]
    NoItemHere,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameAction {
    Move {
        dx: i32,
        dy: i32,
    },
    Attack {
        target_id: usize,
        attack_type: AttackType,
    },
    Interact,
    Converse {
        npc_id: usize,
        input: String,
    },
    UseItem {
        index: usize,
    },
    Evade,
    GrabItem,
    Descend,
    Wait,
}

pub const ACTION_COST: u32 = 100;

pub fn process_action(state: &mut GameState, action: GameAction) -> Result<Vec<String>, GameError> {
    let mut logs = Vec::new();

    state.player.evading = false;
    match action {
        GameAction::Move { dx, dy } => {
            let (px, py) = state.player.position;
            let (new_x, new_y) = (px + dx, py + dy);

            if let Some(enemy) = state.current_floor.get_enemy_at(new_x, new_y) {
                let target_id = enemy.id;
                return process_action(
                    state,
                    GameAction::Attack {
                        target_id,
                        attack_type: AttackType::Strike,
                    },
                );
            }

            let tile = state
                .current_floor
                .get_tile(new_x, new_y)
                .unwrap_or(Tile::Wall);
            if matches!(tile, Tile::Wall) {
                return Err(GameError::BlockedByWall);
            }
            if let Tile::Door { is_open: false, .. } = tile {
                return Err(GameError::DoorLocked);
            }

            state.player.position = (new_x, new_y);
            if let Tile::Hazard { damage } = tile {
                state.player.hp = state.player.hp.saturating_sub(damage as u32);
                state.player.hit_timer = 2; // Hit flash for hazard
                logs.push(format!(
                    "You stepped on a hazard and took {} damage!",
                    damage
                ));
            }
            state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
        }
        GameAction::Attack {
            target_id,
            attack_type,
        } => {
            let base_dmg = if let Some(w) = &state.player.weapon {
                10 + w.bonus_damage
            } else {
                10
            };
            let total_dmg = base_dmg + (state.player.level as i32 * 2);

            let (damage, cost) = match attack_type {
                AttackType::Strike => (total_dmg, ACTION_COST),
                AttackType::Overload => (total_dmg * 2, ACTION_COST + 40),
            };

            if state.player.energy < cost {
                return Err(GameError::InsufficientEnergy);
            }

            let enemy_idx = state
                .current_floor
                .enemies
                .iter()
                .position(|e| e.id == target_id)
                .ok_or(GameError::InvalidTarget)?;

            // 1. Apply damage in a restricted scope to free the borrow immediately
            let mut is_dead = false;
            {
                let enemy = &mut state.current_floor.enemies[enemy_idx];
                enemy.hp = std::cmp::max(0, enemy.hp.saturating_sub(damage as u32));
                enemy.hit_timer = 2; // Trigger visual hit flash
                logs.push(format!("You hit {} for {} damage.", enemy.name, damage));
                if enemy.hp == 0 {
                    is_dead = true;
                }
            }

            // 2. If dead, remove the enemy entirely. We now OWN `dead_enemy`!
            if is_dead {
                let dead_enemy = state.current_floor.enemies.remove(enemy_idx);

                if dead_enemy.id == 9999 {
                    logs.push("========================================".to_string());
                    logs.push("VICTORY! The Nexus Mind is shattered.".to_string());
                    logs.push("Sector 7 is secure. The simulation ends.".to_string());
                    logs.push("========================================".to_string());
                    
                    // Since we aren't borrowing an enemy anymore, manipulating doors works perfectly
                    for y in 0..state.current_floor.height as i32 {
                        for x in 0..state.current_floor.width as i32 {
                            if let Some(Tile::Door { .. }) = state.current_floor.get_tile(x, y) {
                                state.current_floor.set_tile(
                                    x, y, 
                                    Tile::Door { is_open: true, locked: false }
                                );
                            }
                        }
                    }
                } else {
                    let xp_gained = 50 + (state.floor_number * 10);
                    state.player.xp += xp_gained;
                    logs.push(format!(
                        "{} destroyed! Gained {} XP.",
                        dead_enemy.name, xp_gained
                    ));

                    // Check active side-quests for drone kills
                    for quest in &mut state.player.active_quests {
                        if !quest.completed && quest.id.starts_with("quest_") {
                            quest.current_count += 1;
                            if quest.current_count >= quest.target_count {
                                quest.completed = true;
                                state.player.xp += quest.reward_xp;
                                logs.push(format!(
                                    "QUEST COMPLETE: {}! (+{} XP)",
                                    quest.title, quest.reward_xp
                                ));
                            }
                        }
                    }

                    if state.player.xp >= state.player.level * 150 {
                        state.player.xp -= state.player.level * 150;
                        state.player.level += 1;
                        state.player.max_hp += 20;
                        state.player.hp = state.player.max_hp;
                        logs.push(format!(
                            "LEVEL UP! You are now Level {}.",
                            state.player.level
                        ));
                    }
                }

                // Drop loot at the exact position the enemy died
                let item_drop = if state.player.level > 2 && state.turn_count % 3 == 0 {
                    crate::world::Item {
                        id: state.turn_count as usize,
                        name: "Thermal Katana".to_string(),
                        position: dead_enemy.position,
                        item_type: crate::world::ItemType::Weapon { damage: 15 },
                    }
                } else {
                    crate::world::Item {
                        id: state.turn_count as usize,
                        name: "Health Patch".to_string(),
                        position: dead_enemy.position,
                        item_type: crate::world::ItemType::Consumable { heal: 50 },
                    }
                };

                state.current_floor.items.push(item_drop);
            }
            state.player.energy = state.player.energy.saturating_sub(cost);
        }
        GameAction::Wait => {
            state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
            logs.push("You wait.".to_string());
        }

        GameAction::Interact => {
            let (px, py) = state.player.position;
            let mut interacted = false;

            let directions = [(0, 1), (0, -1), (1, 0), (-1, 0)];
            for (dx, dy) in directions.iter() {
                if let Some(Tile::Door { is_open, locked }) =
                    state.current_floor.get_tile(px + dx, py + dy)
                {
                    if locked {
                        if let Some(key_idx) = state.player.inventory.iter().position(|item| {
                            matches!(item.item_type, crate::world::ItemType::Key { .. })
                        }) {
                            state.player.inventory.remove(key_idx);
                            state.current_floor.set_tile(
                                px + dx,
                                py + dy,
                                Tile::Door {
                                    is_open: true,
                                    locked: false,
                                },
                            );
                            logs.push(
                                "You unlocked the door! The Sector Key was consumed.".to_string(),
                            );
                        } else {
                            logs.push("The door is locked. You need the Sector Key.".to_string());
                        }
                    } else {
                        state.current_floor.set_tile(
                            px + dx,
                            py + dy,
                            Tile::Door {
                                is_open: !is_open,
                                locked: false,
                            },
                        );
                        logs.push(
                            if is_open {
                                "You closed the door."
                            } else {
                                "You opened the door."
                            }
                            .to_string(),
                        );
                    }
                    interacted = true;
                    break;
                }
            }

            if !interacted {
                logs.push("Nothing to interact with here.".to_string());
            }
            state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
        }
        GameAction::Converse { npc_id, input: _ } => {
            state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
            logs.push(format!("You tried to speak to NPC {}.", npc_id));
        }

        GameAction::Descend => {
            let (px, py) = state.player.position;
            if let Some(Tile::StairsDown) = state.current_floor.get_tile(px, py) {
                state.current_floor = crate::state::GameState::new_test_state().current_floor;
                state.player.position = (5, 5);
                state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
                logs.push("You descended deeper into the facility.".to_string());
            } else {
                return Err(GameError::InvalidTarget);
            }
        }

        GameAction::Evade => {
            state.player.evading = true;
            state.player.energy = state.player.energy.saturating_sub(ACTION_COST / 2);

            let (px, py) = state.player.position;
            let base_dmg = if let Some(w) = &state.player.weapon { 10 + w.bonus_damage } else { 10 };
            let counter_dmg = (base_dmg + (state.player.level as i32 * 2)) / 2; // 50% riposte damage

            let mut riposte_logs = Vec::new();
            let mut killed_indices = Vec::new();

            for (idx, enemy) in state.current_floor.enemies.iter_mut().enumerate() {
                let dist = (enemy.position.0 - px).abs() + (enemy.position.1 - py).abs();
                if dist == 1 && matches!(enemy.state, crate::world::EnemyState::Telegraphing { .. }) {
                    enemy.hp = enemy.hp.saturating_sub(counter_dmg as u32);
                    enemy.hit_timer = 2; // Trigger hit animation
                    riposte_logs.push(format!(
                        "EVADE & RIPOSTE! You dodged {} and countered for {} dmg! (HP: {})",
                        enemy.name, counter_dmg, enemy.hp
                    ));
                    if enemy.hp == 0 {
                        killed_indices.push(idx);
                    }
                }
            }

            if riposte_logs.is_empty() {
                logs.push("You brace for impact and enter an evasive stance.".to_string());
            } else {
                logs.extend(riposte_logs);
            }

            // Process Riposte Kills & Quest completion
            // Process Riposte Kills & Quest completion
            for &idx in killed_indices.iter().rev() {
                let dead_enemy = state.current_floor.enemies.remove(idx);
                
                if dead_enemy.id == 9999 {
                    logs.push("========================================".to_string());
                    logs.push("VICTORY! The Nexus Mind was shattered by a counter-strike!".to_string());
                    logs.push("Sector 7 is secure. Facility doors unlocked.".to_string());
                    logs.push("========================================".to_string());
                    
                    for y in 0..state.current_floor.height as i32 {
                        for x in 0..state.current_floor.width as i32 {
                            if let Some(Tile::Door { .. }) = state.current_floor.get_tile(x, y) {
                                state.current_floor.set_tile(
                                    x, y, 
                                    Tile::Door { is_open: true, locked: false }
                                );
                            }
                        }
                    }
                } else {
                    let xp_gained = 50 + (state.floor_number * 10);
                    state.player.xp += xp_gained;
                    logs.push(format!("{} destroyed by counter-strike! +{} XP.", dead_enemy.name, xp_gained));

                    for quest in &mut state.player.active_quests {
                        if !quest.completed && quest.id.starts_with("quest_") {
                            quest.current_count += 1;
                            if quest.current_count >= quest.target_count {
                                quest.completed = true;
                                state.player.xp += quest.reward_xp;
                                logs.push(format!(
                                    "QUEST COMPLETE: {}! (+{} XP)",
                                    quest.title, quest.reward_xp
                                ));
                            }
                        }
                    }
                }
            }
        }
        GameAction::GrabItem => {
            let pos = state.player.position;
            if let Some(idx) = state
                .current_floor
                .items
                .iter()
                .position(|i| i.position == pos)
            {
                if state.player.inventory.len() >= state.player.max_inventory {
                    return Err(GameError::InventoryFull);
                }
                let item = state.current_floor.items.remove(idx);
                logs.push(format!("You picked up {}.", item.name));
                if matches!(item.item_type, ItemType::Key { .. }) {
                    if let Some(q) = state
                        .player
                        .active_quests
                        .iter_mut()
                        .find(|q| q.id == "main_01")
                    {
                        q.title = "Unlock the exit and descend".to_string();
                        q.completed = true;
                    }
                }
                state.player.inventory.push(item);
                state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
            } else {
                return Err(GameError::NoItemHere);
            }
        }
        GameAction::UseItem { index } => {
            if index >= state.player.inventory.len() {
                return Err(GameError::InvalidTarget);
            }

            let item = state.player.inventory.remove(index);

            match item.item_type {
                crate::world::ItemType::Consumable { heal } => {
                    state.player.hp = std::cmp::min(state.player.max_hp, state.player.hp + heal);
                    logs.push(format!("You used {} and recovered {} HP.", item.name, heal));
                }
                crate::world::ItemType::Weapon { damage } => {
                    if let Some(old_weapon) = state.player.weapon.take() {
                        state.player.inventory.push(crate::world::Item {
                            id: 999,
                            name: old_weapon.name,
                            position: (0, 0),
                            item_type: crate::world::ItemType::Weapon {
                                damage: old_weapon.bonus_damage,
                            },
                        });
                    }
                    state.player.weapon = Some(crate::state::Weapon {
                        name: item.name.clone(),
                        bonus_damage: damage,
                    });
                    logs.push(format!("Equipped {}. Damage increased.", item.name));
                }
                _ => {
                    state.player.inventory.push(item);
                    return Err(GameError::InvalidTarget);
                }
            }
            state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
        }
    }
    Ok(logs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{GameState, Player};
    use crate::world::{DungeonFloor, Tile};

    fn setup_test_state() -> GameState {
        let mut floor = DungeonFloor::new(5, 5);
        floor.set_tile(1, 1, Tile::Floor);
        floor.set_tile(2, 1, Tile::Wall);
        floor.set_tile(1, 2, Tile::Floor);

        GameState {
            turn_count: 0,
            seed: 42,
            floor_number: 1,
            player: Player {
                position: (1, 1),
                hp: 100,
                max_hp: 100,
                energy: 100,
                speed: 20,
                inventory: vec![],
                active_quests: vec![],
                evading: false,
                hit_timer: 0, // Updated for test compliance
                max_inventory: 12,
                weapon: None,
            },
            current_floor: floor,
        }
    }

    #[test]
    fn test_valid_movement_consumes_energy() {
        let mut state = setup_test_state();
        let action = GameAction::Move { dx: 0, dy: 1 };

        let result = process_action(&mut state, action);

        assert!(result.is_ok());
        assert_eq!(state.player.position, (1, 2));
        assert_eq!(state.player.energy, 0);
    }

    #[test]
    fn test_invalid_movement_blocked_by_wall() {
        let mut state = setup_test_state();
        let action = GameAction::Move { dx: 1, dy: 0 };

        let result = process_action(&mut state, action);

        assert!(matches!(result, Err(GameError::BlockedByWall)));
        assert_eq!(state.player.position, (1, 1));
        assert_eq!(state.player.energy, 100);
    }
}
