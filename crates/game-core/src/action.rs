use thiserror::Error;
use crate::state::GameState;
use crate::world::Tile;

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
    #[error("Not enough energy to perform this action.")]
    InsufficientEnergy,
}

pub enum ActionResult {
    Moved,
    Attacked { damage: u32, target_id: usize, target_destroyed: bool },
    Interacted { description: String },
    Waited,
}

pub enum GameAction {
    Move {dx: i32, dy: i32},
    Attack {target_id: usize, attack_type: AttackType},
    Interact {target_id: usize},
    Converse {npc_id: usize, input: String},
    Wait,
}

const ACTION_COST: u32 = 100;

pub fn process_action(state: &mut GameState, action: GameAction) -> Result<ActionResult, GameError> {
    match action {
        GameAction::Move { dx, dy } => {
            let new_x = state.player.position.0 + dx;
            let new_y = state.player.position.1 + dy;

            let tile = state.current_floor.get_tile(new_x, new_y).unwrap_or(Tile::Wall);
            match tile {
                Tile::Wall => return Err(GameError::BlockedByWall),
                Tile::Door { is_open, locked } => {
                    if !is_open {
                        if locked { return Err(GameError::DoorLocked); }
                        return Err(GameError::BlockedByWall);
                    }
                }
                Tile::Floor => {}
            }

            if state.current_floor.get_enemy_at(new_x, new_y).is_some() {
                return Err(GameError::BlockedByEntity);
            }

            state.player.position = (new_x, new_y);
            
            state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
            
            Ok(ActionResult::Moved)
        }

        GameAction::Attack { target_id, attack_type } => {
            let (damage, cost) = match attack_type {
                AttackType::Strike => (10, ACTION_COST),
                AttackType::Overload => (25, ACTION_COST + 40),
            };

            if state.player.energy < cost {
                return Err(GameError::InsufficientEnergy);
            }

            let enemy_idx = state.current_floor.enemies.iter().position(|e| e.id == target_id)
                .ok_or(GameError::InvalidTarget)?;

            let enemy = &mut state.current_floor.enemies[enemy_idx];
            enemy.hp = enemy.hp.saturating_sub(damage);
            let destroyed = enemy.hp == 0;

            if destroyed {
                state.current_floor.enemies.remove(enemy_idx);
            }

            state.player.energy = state.player.energy.saturating_sub(cost);

            Ok(ActionResult::Attacked { damage, target_id, target_destroyed: destroyed })
        }

        GameAction::Wait => {
            state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
            Ok(ActionResult::Waited)
        }

        GameAction::Interact => {
            state.player.energy = state.player.energy.saturating_sub(ACTION_COST);
            Ok(ActionResult::Interacted { description: "You looked around.".to_string() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{GameState, Player};
    use crate::world::{DungeonFloor, Tile};

    fn setup_test_state() -> GameState {
        let mut floor = DungeonFloor::new(5, 5);
        // Create a 5x5 room, player at (1,1), wall at (2,1), floor at (1,2)
        floor.set_tile(1, 1, Tile::Floor);
        floor.set_tile(2, 1, Tile::Wall);
        floor.set_tile(1, 2, Tile::Floor);

        GameState {
            turn_count: 0,
            player: Player {
                position: (1, 1),
                hp: 100,
                max_hp: 100,
                energy: 100,
                speed: 20,
                inventory: vec![],
            },
            current_floor: floor,
        }
    }

    #[test]
    fn test_valid_movement_consumes_energy() {
        let mut state = setup_test_state();
        let action = GameAction::Move { dx: 0, dy: 1 }; // Move down to (1,2)
        
        let result = process_action(&mut state, action);
        
        assert!(result.is_ok());
        assert_eq!(state.player.position, (1, 2));
        assert_eq!(state.player.energy, 0); // 100 energy consumed
    }

    #[test]
    fn test_invalid_movement_blocked_by_wall() {
        let mut state = setup_test_state();
        let action = GameAction::Move { dx: 1, dy: 0 }; // Move right to (2,1)
        
        let result = process_action(&mut state, action);
        
        assert!(matches!(result, Err(GameError::BlockedByWall)));
        assert_eq!(state.player.position, (1, 1)); // Position unchanged
        assert_eq!(state.player.energy, 100); // Energy NOT consumed
    }
}
