pub const TURN_ENERGY_THRESHOLD: u32 = 100;

pub struct TimeSystem{
    pub current_tick: u64,
}

impl TimeSystem {
    
    pub fn advance_until_player(&mut self, state: &mut GameState) -> Vec<GameEvent> {
        let mut events = Vec::new();

        loop{
            if state.player.energy >= TURN_ENERGY_THRESHOLD {
                break;
            }

            state.player.energy += state.player.speed;
            for enemy in &mut state.count_floor.enemies{
                enemy.energy += enemy.speed;

                if enemy.energy >= TURN_ENERGY_THRESHOLD{
                    let action = enemy.decide_action(state);
                    events.push(apply_enemy_action(state, enemy.id, action));
                    enemy.energy -= TURN_ENERGY_THRESHOLD;
                }
            }
            self.current_tick += 1;
        }
        events
    }

}
