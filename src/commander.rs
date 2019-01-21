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
            states: StateStack::new(Box::new(DefaultState)),
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
            .top()
            .expect("The Commander should never run out of tasks")
            .as_mut()
    }
}

pub trait CommanderState {
    fn step(&self, aimgr: &AiManager, world: &mut GameState) -> StackOp<Box<dyn CommanderState>>;
    fn request_task(&self, id: ShipId, world: &GameState) -> Box<dyn ShipAiState>;
}

struct DefaultState;

impl CommanderState for DefaultState {
    fn step(&self, aimgr: &AiManager, world: &mut GameState) -> StackOp<Box<dyn CommanderState>> {
        let (max_pos, max_density) = world
            .halite_density
            .iter()
            .enumerate()
            .flat_map(|(i, row)| row.iter().enumerate().map(move |(j, &x)| (i, j, x)))
            .max_by_key(|(_, _, x)| *x)
            .map(|(i, j, x)| {
                (
                    Position {
                        x: j as i32,
                        y: i as i32,
                    },
                    x,
                )
            })
            .unwrap();

        let want_dropoff = world.avg_return_length
            >= world.config.expansion.expansion_distance as f64
            && max_density >= world.config.expansion.min_halite_density;

        if want_dropoff {
            // create a massive pheromone spike at a good dropoff location
            //state.add_pheromone(max_pos, 100000.0);
            world.add_pheromone(max_pos, 100000.0);
        }

        if want_dropoff && world.me().halite >= world.game.constants.dropoff_cost {
            let id = aimgr
                .ships
                .keys()
                .filter(|&&id| {
                    world
                        .game
                        .map
                        .at_entity(world.get_ship(id))
                        .structure
                        .is_none()
                })
                .filter(|&&id| {
                    world.distance_to_nearest_dropoff(id)
                        >= world.config.expansion.expansion_distance
                })
                .filter(|&&id| {
                    world
                        .ships_in_range(
                            world.get_ship(id).position,
                            world.config.expansion.ship_radius,
                        )
                        .count()
                        >= world.config.expansion.n_ships
                })
                .map(|&id| {
                    let p = world.get_ship(id).position;
                    (
                        id,
                        world.halite_density[p.y as usize][p.x as usize],
                        world.pheromones[p.y as usize][p.x as usize],
                    )
                })
                .filter(|&(_, density, _)| density >= world.config.expansion.min_halite_density)
                .max_by_key(|&(_, _, phi)| phi as i64)
                .map(|(id, _, _)| id);

            if let Some(id) = id {
                Log::log(&format!("Commander: Instructing {:?} to build dropoff", id));
                aimgr.ship(id).push_task(Box::new(BuildDropoff));
                Log::log(&format!("{:?}", aimgr.ship(id)));
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
