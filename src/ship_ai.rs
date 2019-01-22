use ai_manager::AiManager;
use hlt::direction::Direction;
use hlt::log::Log;
use hlt::map_cell::Structure;
use hlt::ShipId;
use pda::{StackOp, StateStack};
use GameState;

#[derive(Debug)]
pub struct ShipAi {
    id: ShipId,
    states: StateStack<Box<dyn ShipAiState>>,
}

impl ShipAi {
    pub fn new(id: ShipId) -> Self {
        ShipAi {
            id,
            states: StateStack::default(),
        }
    }

    pub fn think(&mut self, aimgr: &AiManager, world: &mut GameState) {
        loop {
            if self.states.is_empty() {
                let op = StackOp::Push(aimgr.commander().request_task(self.id, world));
                self.states.transition(op);
            }

            let op = self.states.top().unwrap().step(self.id, world);
            if let StackOp::None = op {
                break
            } else {
                self.states.transition(op);
            }
        }
    }

    pub fn push_task(&mut self, task: Box<dyn ShipAiState>) {
        self.states.push(task);
    }
}

pub trait ShipAiState: std::fmt::Debug {
    fn step(&mut self, id: ShipId, world: &mut GameState) -> StackOp<Box<dyn ShipAiState>>;
}

#[derive(Debug)]
pub struct Collect;

impl ShipAiState for Collect {
    fn step(&mut self, id: ShipId, world: &mut GameState) -> StackOp<Box<dyn ShipAiState>> {
        if world.get_ship(id).is_full() {
            return StackOp::Done
        }

        if stuck_move(id, world) {
            return StackOp::None
        }

        let pos = world.get_ship(id).position;

        let dist = world.get_return_distance(world.get_ship(id).position);
        if world.rounds_left()
            <= dist
            + (world.me().ship_ids.len() * world.config.navigation.go_home_safety_factor)
            / (1 * (1 + world.me().dropoff_ids.len()))
        {
            return StackOp::Override(Box::new(GoHome))
        }

        let cargo = world.get_ship(id).halite as i32;

        let mc = world.movement_cost(&pos);

        let current_halite = world.halite_gain(&pos) * world.game.constants.extract_ratio; // factor inspiration into current_halite
        let phi0 = world.get_pheromone(pos);

        let mut weights: Vec<_> = if cargo < mc {
            vec![9999999.0, 0.0, 0.0, 0.0, 0.0]
        } else {
            Direction::get_all_options()
                .into_iter()
                .map(|d| pos.directional_offset(d))
                .map(|p| world.get_pheromone(p))
                .collect()
        };

        if current_halite < 1
            && weights[0] < 1.0
            && weights[1] < 1.0
            && weights[2] < 1.0
            && weights[3] < 1.0
        {
            weights[4] = -9999999.0; // no loitering on empty cells
            let [c0, cn, cs, ce, cw] = world.get_return_dir_costs(pos);
            weights[0] += 0.1 * (cw - c0) as f64;
            weights[1] += 0.1 * (ce - c0) as f64;
            weights[2] += 0.1 * (cn - c0) as f64;
            weights[3] += 0.1 * (cs - c0) as f64;
        } else if world.game.map.at_position(&pos).structure != Structure::None {
            weights[4] = -9999999.0; // no loitering at the shipyard
        } else if current_halite > world.config.ships.greedy_harvest_limit && phi0 < 1000.0 {
            weights[4] = 1000.0 + current_halite as f64;
        } else if current_halite as f64 > phi0 {
            weights[4] = current_halite as f64;
        }

        let ok_n = !world
            .mp
            .is_occupied(pos.directional_offset(Direction::North));
        let ok_s = !world
            .mp
            .is_occupied(pos.directional_offset(Direction::South));
        let ok_e = !world
            .mp
            .is_occupied(pos.directional_offset(Direction::East));
        let ok_w = !world
            .mp
            .is_occupied(pos.directional_offset(Direction::West));

        let mut cn = if ok_n {
            -(weights[2] * 100.0) as i32
        } else {
            i32::max_value()
        };
        let mut cs = if ok_s {
            -(weights[3] * 100.0) as i32
        } else {
            i32::max_value()
        };
        let mut ce = if ok_e {
            -(weights[1] * 100.0) as i32
        } else {
            i32::max_value()
        };
        let mut cw = if ok_w {
            -(weights[0] * 100.0) as i32
        } else {
            i32::max_value()
        };
        let c0 = -(weights[4] * 100.0) as i32;

        let mut prey = vec![];
        for p in Direction::get_all_cardinals().into_iter().map(|d| pos.directional_offset(d)) {
            prey.push(None);
            if let Some(ship) = world.get_ship_at(p) {
                if ship.owner == world.game.my_id {
                    continue
                }

                let other_cargo = ship.halite as i32;
                if other_cargo <= cargo {
                    continue
                }

                let r = world
                    .find_nearest_oponent(p, true)
                    .map(|id| world.get_ship(id).position)
                    .map(|sp| world.game.map.calculate_distance(&p, &sp))
                    .unwrap_or(10);

                Log::log(&format!("potential prey at {:?} with nearest opponent {} steps away...", p, r));

                let free_cargo = world.my_ships()
                    .map(|id| world.get_ship(id))
                    .filter(|ship| ship.position != pos)
                    .filter(|ship| world.game.map.calculate_distance(&p, &ship.position) < r)
                    .inspect(|ship| Log::log(&format!("   ... and friendly ship at {:?}", ship.position)))
                    .map(|ship| ship.capacity() as i32)
                    .sum::<i32>();

                if free_cargo > cargo {
                    let aggressiveness = if world.game.players.len() == 2 {
                        1000
                    } else {
                        10
                    };
                    *prey.last_mut().unwrap() = Some(aggressiveness * (other_cargo - cargo) as i32);
                }
            }
        }

        if let Some(gain) = prey[0] {
            cw = -gain;
        }

        if let Some(gain) = prey[1] {
            ce = -gain;
        }

        if let Some(gain) = prey[2] {
            cn = -gain;
        }

        if let Some(gain) = prey[3] {
            cs = -gain;
        }

        if prey.into_iter().any(|pr| pr.is_some()) {
            // attract nearby ships a bit more
            world.add_pheromone(pos, 1000.0);
        }

        world.gns.plan_move(id, pos, c0, cn, cs, ce, cw);

        StackOp::None
    }
}

#[derive(Debug)]
pub struct Deliver {
    turns_taken: usize,
}

impl Deliver {
    pub fn new() -> Self {
        Deliver { turns_taken: 0 }
    }
}

impl ShipAiState for Deliver {
    fn step(&mut self, id: ShipId, world: &mut GameState) -> StackOp<Box<dyn ShipAiState>> {
        if world.get_ship(id).halite <= 0 {
            world.notify_return(self.turns_taken);
            return StackOp::Done;
        }

        let pos = world.get_ship(id).position;
        let cap = world.get_ship(id).capacity();
        let cargo = world.get_ship(id).halite;

        let harvest = world.config.navigation.return_step_cost
            - world.halite_gain(&pos).min(cap) as i32; // we may actually gain something from waiting...

        if !stuck_move(id, world) {
            let [c0, cn, cs, ce, cw] = world.get_return_dir_costs(pos);

            let ok_n = !world
                .mp
                .is_reachable(pos.directional_offset(Direction::North));
            let ok_s = !world
                .mp
                .is_reachable(pos.directional_offset(Direction::South));
            let ok_e = !world
                .mp
                .is_reachable(pos.directional_offset(Direction::East));
            let ok_w = !world
                .mp
                .is_reachable(pos.directional_offset(Direction::West));

            let cn = if ok_n { cn - c0 } else { i32::max_value() };
            let cs = if ok_s { cs - c0 } else { i32::max_value() };
            let ce = if ok_e { ce - c0 } else { i32::max_value() };
            let cw = if ok_w { cw - c0 } else { i32::max_value() };
            world.gns.plan_move(id, pos, harvest, cn, cs, ce, cw);
        }

        let ev = world.config.pheromones.ship_evaporation;
        world.add_pheromone(pos, cargo as f64 * ev);

        self.turns_taken += 1;

        StackOp::None
    }
}

#[derive(Debug)]
pub struct GoHome;

impl ShipAiState for GoHome {
    fn step(&mut self, id: ShipId, world: &mut GameState) -> StackOp<Box<dyn ShipAiState>> {
        let pos = world.get_ship(id).position;

        if stuck_move(id, world) {
            return StackOp::None
        }

        for d in Direction::get_all_cardinals() {
            match world
                .game
                .map
                .at_position(&pos.directional_offset(d))
                .structure
            {
                Structure::Dropoff(did) if world.game.dropoffs[&did].owner == world.game.my_id => {
                    world.gns.force_move(id, d);
                    return StackOp::None
                }
                Structure::Shipyard(pid) if pid == world.game.my_id => {
                    world.gns.force_move(id, d);
                    return StackOp::None
                }
                _ => {}
            }
        }

        let [c0, cn, cs, ce, cw] = world.get_return_dir_costs(pos);

        let ok_n = !world
            .mp
            .is_occupied(pos.directional_offset(Direction::North));
        let ok_s = !world
            .mp
            .is_occupied(pos.directional_offset(Direction::South));
        let ok_e = !world
            .mp
            .is_occupied(pos.directional_offset(Direction::East));
        let ok_w = !world
            .mp
            .is_occupied(pos.directional_offset(Direction::West));

        let cn = if ok_n { cn - c0 } else { i32::max_value() };
        let cs = if ok_s { cs - c0 } else { i32::max_value() };
        let ce = if ok_e { ce - c0 } else { i32::max_value() };
        let cw = if ok_w { cw - c0 } else { i32::max_value() };
        let c0 = world.config.navigation.return_step_cost;
        world.gns.plan_move(id, pos, c0, cn, cs, ce, cw);

        StackOp::None
    }
}

#[derive(Debug)]
pub struct BuildDropoff;

impl ShipAiState for BuildDropoff {
    fn step(&mut self, id: ShipId, world: &mut GameState) -> StackOp<Box<dyn ShipAiState>> {
        // todo: move a few steps up the density gradient
        if world.try_build_dropoff(id) {
            Log::log(&format!("{:?} building dropoff", id));
            StackOp::None
        } else {
            // if it fails the commander will eventually tell us to try again. Otherwise, continue with previous task
            Log::log(&format!("{:?} failed to build dropoff", id));
            StackOp::Done
        }
    }
}

fn stuck_move(id: ShipId, state: &mut GameState) -> bool {
    let pos = state.get_ship(id).position;
    let cargo = state.get_ship(id).halite as i32;
    let cap = state.get_ship(id).capacity();

    let harvest =
        state.config.navigation.return_step_cost - state.halite_gain(&pos).min(cap) as i32; // we may actually gain something from waiting...

    if state.movement_cost(&pos) > cargo {
        state.gns.plan_move(
            id,
            pos,
            harvest,
            i32::max_value(),
            i32::max_value(),
            i32::max_value(),
            i32::max_value(),
        );
        true
    } else {
        false
    }
}
