use crate::state::GameState;
use crate::world::Tile;
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
                return process_action(state, GameAction::Attack { 
                    target_id, 
                    attack_type: AttackType::Strike 
                });
            }

            let tile = state.current_floor.get_tile(new_x, new_y).unwrap_or(Tile::Wall);
            if matches!(tile, Tile::Wall) { return Err(GameError::BlockedByWall); }
            if let Tile::Door { is_open: false, .. } = tile { return Err(GameError::DoorLocked); }

            state.player.position = (new_x, new_y);
            state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
        }
        GameAction::Attack { target_id, attack_type } => {
            let (damage, cost) = match attack_type {
                AttackType::Strike => (10, ACTION_COST),
                AttackType::Overload => (25, ACTION_COST + 40),
            };

            if state.player.energy < cost { return Err(GameError::InsufficientEnergy); }
            
            let enemy_idx = state.current_floor.enemies.iter().position(|e| e.id == target_id).ok_or(GameError::InvalidTarget)?;
            let enemy = &mut state.current_floor.enemies[enemy_idx];
            
            enemy.hp = enemy.hp.saturating_sub(damage);
            logs.push(format!("You hit {} for {} damage.", enemy.name, damage));

            if enemy.hp == 0 {
                logs.push(format!("{} was destroyed! It dropped loot.", enemy.name));
                state.current_floor.items.push(crate::world::Item {
                    id: state.turn_count as usize,
                    name: "Health Patch".to_string(),
                    position: enemy.position,
                    item_type: crate::world::ItemType::Consumable { heal: 50 },
                });
                state.current_floor.enemies.remove(enemy_idx);
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
                        logs.push("The door is locked.".to_string());
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
            if state.player.energy < ACTION_COST {
                return Err(GameError::InsufficientEnergy);
            }
            state.player.evading = true;
            state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
            logs.push("You prepare to evade the next attack.".to_string());
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
            if let crate::world::ItemType::Consumable { heal } =
                state.player.inventory[index].item_type
            {
                state.player.hp = std::cmp::min(state.player.max_hp, state.player.hp + heal);
                let item = state.player.inventory.remove(index);
                logs.push(format!("You used {} and recovered {} HP.", item.name, heal));
                state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
            }
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
            player: Player {
                position: (1, 1),
                hp: 100,
                max_hp: 100,
                energy: 100,
                speed: 20,
                inventory: vec![],
                active_quests: vec![],
                evading: false,
                max_inventory: 12,
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
