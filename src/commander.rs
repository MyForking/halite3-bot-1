use ai_manager::AiManager;
use hlt::log::Log;
use hlt::position::Position;
use hlt::ShipId;
use pda::{StackOp, StateStack};
use ship_ai::{BuildDropoff, Collect, Deliver, ShipAiState};
use GameState;

pub struct Commander {
    states: StateStack<Box<dyn CommanderState>>,
}

impl Commander {
    pub fn new() -> Self {
        Commander {
            states: StateStack::new(Box::new(MidGame)),
        }
    }

    pub fn think(&mut self, aimgr: &AiManager, world: &mut GameState) {
        let op = self.current_state().step(aimgr, world);
        self.states.transition(op);
    }

    pub fn request_task(&mut self, id: ShipId, world: &GameState) -> Box<dyn ShipAiState> {
        self.current_state().request_task(id, world)
    }

    pub fn current_state(&mut self) -> &mut dyn CommanderState {
        self.states
            .top_mut()
            .expect("The Commander should never run out of tasks")
            .as_mut()
    }
}

pub trait CommanderState {
    fn step(&self, aimgr: &AiManager, world: &mut GameState) -> StackOp<Box<dyn CommanderState>>;
    fn request_task(&self, id: ShipId, world: &GameState) -> Box<dyn ShipAiState>;
}

struct MidGame;

impl CommanderState for MidGame {
    fn step(&self, aimgr: &AiManager, world: &mut GameState) -> StackOp<Box<dyn CommanderState>> {
        let mut want_dropoff = world.avg_return_length
            >= world.config.expansion.return_distance as f64;

        if want_dropoff {
            let have_builder = aimgr.ships.values()
                .any(|ship_ai| ship_ai.borrow().top_state().map(|s| s.is_builder()).unwrap_or(false));

            if !have_builder {
                let exploc = world
                    .halite_density
                    .iter()
                    .enumerate()
                    .flat_map(|(i, row)| row.iter().enumerate().map(move |(j, &x)| (i, j, x)))
                    .map(|(i, j, x)| {
                        (
                            Position {
                                x: j as i32,
                                y: i as i32,
                            },
                            x,
                        )
                    })
                    .filter(|&(p, _)| world.is_valid_expansion_location(p))
                    .map(|(p, x)| (p, x - world.distance_to_nearest_dropoff(p) as i32))
                    .max_by_key(|(_, x)| *x);

                if let Some((max_pos, max_density)) = exploc {
                    want_dropoff = max_density >= world.config.expansion.min_halite_density;

                    if want_dropoff {
                        if let Some(id) = world.find_nearest_ship(max_pos) {
                            Log::log(&format!("Commander: Instructing {:?} to build dropoff", id));
                            aimgr.ship(id).push_task(Box::new(BuildDropoff::new(max_pos)));
                            Log::log(&format!("{:?}", aimgr.ship(id)));
                        }
                    }
                } else {
                    want_dropoff = false;
                }
            }
        }

        let mut want_ship = {
            let bias = world.config.strategy.spawn_halite_floor;
            let halite_left: usize = world
                .game
                .map
                .iter()
                .map(|cell| cell.halite.max(bias) - bias)
                .sum();
            let n_ships = world.game.ships.len() + 1;

            (halite_left / n_ships > world.game.constants.ship_cost)
                && world.rounds_left()
                > world.game.map.width * world.config.strategy.spawn_min_rounds_left_factor
        };

        want_ship &= !want_dropoff
            || world.me().halite
            >= world.game.constants.dropoff_cost + world.game.constants.ship_cost;

        if want_ship && world.me().halite >= world.game.constants.ship_cost {
            let pos = world.me().shipyard.position;
            world.gns.notify_spawn(pos);
            world.total_spent += world.game.constants.ship_cost; // assuming the spawn is always successful (it should be...)
        }

        StackOp::None
    }

    fn request_task(&self, id: ShipId, world: &GameState) -> Box<dyn ShipAiState> {
        let cargo = world.get_ship(id).halite;
        if cargo < 500 {
            Box::new(Collect)
        } else {
            Box::new(Deliver::new())
        }
    }
}
