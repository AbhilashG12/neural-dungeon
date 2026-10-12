use crate::state::GameState;

pub const TURN_ENERGY_THRESHOLD: u32 = 100;

pub struct TimeSystem {
    pub current_tick: u64,
}

impl TimeSystem {
    pub fn advance_until_player(&mut self, state: &mut GameState) {
        loop {
            if state.player.energy >= TURN_ENERGY_THRESHOLD {
                break;
            }

            state.player.energy += state.player.speed;
            for enemy in &mut state.current_floor.enemies {
                enemy.energy += enemy.speed;

                if enemy.energy >= TURN_ENERGY_THRESHOLD {
                    enemy.energy -= TURN_ENERGY_THRESHOLD;
                }
            }
            self.current_tick += 1;
        }
    }
}
