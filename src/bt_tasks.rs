use behavior_tree::{
    continuous, interrupt, lambda, run_or_fail, select, sequence, BtNode, BtState,
};
use hlt::direction::Direction;
use hlt::log::Log;
use hlt::map_cell::Structure;
use hlt::ShipId;
use std::f64;
use GameState;

fn stuck_move(id: ShipId, state: &mut GameState) -> bool {
    let pos = state.get_ship(id).position;
    let cargo = state.get_ship(id).halite;
    let cap = state.get_ship(id).capacity();

    let harvest =
        state.config.navigation.return_step_cost as i32 - state.halite_gain(&pos).min(cap) as i32; // we may actually gain something from waiting...

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

fn deliver(id: ShipId) -> Box<impl BtNode<GameState>> {
    let mut turns_taken = 0;
    lambda(move |state: &mut GameState| {
        if state.get_ship(id).halite <= 0 {
            state.notify_return(turns_taken);
            return BtState::Success;
        }

        let pos = state.get_ship(id).position;
        let cap = state.get_ship(id).capacity();
        let cargo = state.get_ship(id).halite;

        let harvest = state.config.navigation.return_step_cost as i32
            - state.halite_gain(&pos).min(cap) as i32; // we may actually gain something from waiting...

        if !stuck_move(id, state) {
            let [c0, cn, cs, ce, cw] = state.get_return_dir_costs(pos);

            let cn = cn - c0;
            let cs = cs - c0;
            let ce = ce - c0;
            let cw = cw - c0;
            state.gns.plan_move(id, pos, harvest, cn, cs, ce, cw);
        }

        let ev = state.config.pheromones.ship_evaporation;
        state.add_pheromone(pos, cargo as f64 * ev);

        turns_taken += 1;

        BtState::Running
    })
}

fn go_home(id: ShipId) -> Box<impl BtNode<GameState>> {
    lambda(move |state: &mut GameState| {
        let pos = state.get_ship(id).position;
        let cargo = state.get_ship(id).halite;

        if stuck_move(id, state) {
            return BtState::Running;
        }

        for d in Direction::get_all_cardinals() {
            if state
                .game
                .map
                .at_position(&pos.directional_offset(d))
                .structure
                .is_some()
            {
                // todo: only my own structures?
                state.gns.force_move(id, d);
                return BtState::Running;
            }
        }

        let [c0, cn, cs, ce, cw] = state.get_return_dir_costs(pos);

        let cn = cn - c0;
        let cs = cs - c0;
        let ce = ce - c0;
        let cw = cw - c0;
        let c0 = state.config.navigation.return_step_cost as i32;
        state.gns.plan_move(id, pos, c0, cn, cs, ce, cw);

        BtState::Running
    })
}

/*fn go_to(id: ShipId, dest: Position) -> Box<impl BtNode<GameState>> {
    lambda(move |state: &mut GameState| {
        if state.get_ship(id).position == dest {
            return BtState::Success;
        }

        let pos = state.get_ship(id).position;
        let path = state.get_dijkstra_path(pos, dest);
        let d = path.first().cloned().unwrap_or(Direction::Still);
        state.move_ship_or_wait(id, d);

        BtState::Running
    })
}*/

/*fn find_res(id: ShipId) -> Box<impl BtNode<GameState>> {
    lambda(move |state: &mut GameState| {
        let pos = state.get_ship(id).position;
        let cap = state.get_ship(id).capacity();
        let current_halite = state.game.map.at_position(&pos).halite;
        let cargo = state.get_ship(id).halite;

        if current_halite >= state.config.ships.greedy_seek_limit {
            return BtState::Success;
        }

        let harvest = state.config.navigation.return_step_cost as i32
            - state.halite_gain(&pos).min(cap) as i32; // we may actually gain something from waiting...

        if stuck_move(id, state) {return BtState::Running}

        let mc = state.movement_cost(&pos) as i32;

        let cap = state.get_ship(id).capacity();
        let cargo = state.get_ship(id).halite;

        let p0 = state.get_pheromone(pos);
        let pn = state.get_pheromone(pos.directional_offset(Direction::North));
        let ps = state.get_pheromone(pos.directional_offset(Direction::South));
        let pe = state.get_pheromone(pos.directional_offset(Direction::East));
        let pw = state.get_pheromone(pos.directional_offset(Direction::West));

        let cn = mc + ((pn - p0) * state.config.ships.seek_pheromone_cost) as i32;
        let cs = mc + ((ps - p0) * state.config.ships.seek_pheromone_cost) as i32;
        let ce = mc + ((pe - p0) * state.config.ships.seek_pheromone_cost) as i32;
        let cw = mc + ((pw - p0) * state.config.ships.seek_pheromone_cost) as i32;
        let c0 = -(state.halite_gain(&pos).min(cap) as i32); // we may actually gain something from waiting...
        state.gns.plan_move(id, pos, c0, cn, cs, ce, cw);

        BtState::Running
    })
}

fn find_desperate(id: ShipId) -> Box<impl BtNode<GameState>> {
    lambda(move |state: &mut GameState| {
        let pos = state.get_ship(id).position;
        let current_halite = state.game.map.at_position(&pos).halite;

        if current_halite > 0 {
            return BtState::Success;
        }

        match state.get_nearest_halite_move(pos, 1) {
            Some(Direction::North) => state.gns.plan_move(id, pos, 2, 1, 3, 3, 3),
            Some(Direction::South) => state.gns.plan_move(id, pos, 2, 3, 1, 3, 3),
            Some(Direction::East) => state.gns.plan_move(id, pos, 2, 3, 3, 1, 3),
            Some(Direction::West) => state.gns.plan_move(id, pos, 2, 3, 3, 3, 1),
            Some(Direction::Still) => state.gns.plan_move(id, pos, 0, 2, 2, 2, 2),
            None => return BtState::Failure,
        }

        BtState::Running
    })
}*/

fn greedy(id: ShipId) -> Box<impl BtNode<GameState>> {
    lambda(move |state: &mut GameState| {
        if state.get_ship(id).is_full() {
            return BtState::Success;
        }

        if stuck_move(id, state) {
            return BtState::Running;
        }

        let pos = state.get_ship(id).position;
        let cap = state.get_ship(id).capacity() as f64;
        let cargo = state.get_ship(id).halite;

        let syp = state.me().shipyard.position;

        let mc = state.movement_cost(&pos);

        let current_halite = state.halite_gain(&pos) * state.game.constants.extract_ratio; // factor inspiration into current_halite
        let phi0 = state.get_pheromone(pos);

        Log::log(&format!("{:?}", id));
        Log::log(&format!(
            "    @ {:?}: {} halite; {} pheromone",
            pos, current_halite, phi0
        ));

        /*let (d, h) = Direction::get_all_cardinals().into_iter()
            .map(|d| (d, pos.directional_offset(d)))
            .map(|(d, p)| {
                let target_halite = state.game.map.at_position(&p).halite;
                let phi = state.get_pheromone(p);
                let bias = 50.min(state.halite_percentiles[75]);
                let x = if p == syp || state.navi.is_unsafe(&p) {
                    -f64::INFINITY
                } else {
                    0.1 * (- 1.0 * current_halite as f64 + 0.15 * target_halite as f64 + phi * 0.01)
                };
                Log::log(&format!("    - {:?}: {} ... {} halite; {} pheromone", p, x, target_halite, phi));
                (d, x)
            })
            .map(|(d, x)| (d, sigmoid(x)))
            .inspect(|x| Log::log(&format!("{:?}", x)))
            .max_by(|&(_, activation1), &(_, activation2)|
            if activation1 < activation2 {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            })
            .unwrap();

        let h0 = 1.0 - h;

        if h0 >= h {
            state.move_ship(id, Direction::Still);
            return BtState::Running;
        } else {
            state.move_ship(id, d);
            return BtState::Running;
        }*/

        let mut weights: Vec<_> = if cargo < mc {
            vec![9999999.0, 0.0, 0.0, 0.0, 0.0]
        } else {
            Direction::get_all_options()
                .into_iter()
                .map(|d| pos.directional_offset(d))
                .map(|p| state.get_pheromone(p))
                .collect()
        };

        if current_halite < 1 && weights[0] < 1.0 && weights[1] < 1.0 && weights[2] < 1.0 && weights[3] < 1.0 {
            weights[4] = -9999999.0; // no loitering on empty cells
            let [c0, cn, cs, ce, cw] = state.get_return_dir_costs(pos);
            weights[0] += 0.1 * (cw - c0) as f64;
            weights[1] += 0.1 * (ce - c0) as f64;
            weights[2] += 0.1 * (cn - c0) as f64;
            weights[3] += 0.1 * (cs - c0) as f64;
        } else if state.game.map.at_position(&pos).structure != Structure::None {
            weights[4] = -9999999.0; // no loitering at the shipyard
        } else if current_halite > state.config.ships.greedy_harvest_limit && phi0 < 1000.0 {
            weights[4] = 1000.0 + current_halite as f64;
        } else if current_halite as f64 > phi0 {
            weights[4] = current_halite as f64;
        }

        Log::log(&format!("    {:?}", weights));

        state.gns.plan_move(
            id,
            pos,
            -(weights[4] * 100.0) as i32,
            -(weights[2] * 100.0) as i32,
            -(weights[3] * 100.0) as i32,
            -(weights[1] * 100.0) as i32,
            -(weights[0] * 100.0) as i32,
        );

        BtState::Running
    })
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/*fn desperate(id: ShipId) -> Box<impl BtNode<GameState>> {
    lambda(move |state: &mut GameState| {
        if state.get_ship(id).is_full() {
            return BtState::Success;
        }

        let pos = state.get_ship(id).position;

        let movement_cost =
            state.game.map.at_position(&pos).halite / state.game.constants.move_cost_ratio;

        if movement_cost > 0 {
            state.move_ship(id, Direction::Still);
            return BtState::Running;
        }

        let syp = state.me().shipyard.position;

        let mov = Direction::get_all_options()
            .into_iter()
            .map(|d| (d, pos.directional_offset(d)))
            .map(|(d, p)| (state.game.map.at_position(&p).halite, d, p))
            .filter(|&(halite, _, _)| halite > 0)
            .filter(|&(_, _, p)| p != syp)
            .filter(|(_, _, p)| state.navi.is_safe(p))
            .max_by_key(|&(halite, _, _)| halite)
            .map(|(_, d, _)| d);

        match mov {
            None => BtState::Failure,
            Some(d) => {
                state.move_ship(id, d);
                BtState::Running
            }
        }
    })
}*/

pub fn build_dropoff(id: ShipId) -> Box<impl BtNode<GameState>> {
    sequence(vec![
        // todo: move a few steps up the density gradient
        run_or_fail(move |state: &mut GameState| state.try_build_dropoff(id)),
    ])
}

pub fn collector(id: ShipId) -> Box<impl BtNode<GameState>> {
    continuous(select(vec![
        interrupt(
            select(vec![
                sequence(vec![greedy(id), deliver(id)]),
                /*find_res(id),
                sequence(vec![desperate(id), deliver(id)]),
                find_desperate(id),*/
            ]),
            move |env| {
                let dist = env.get_return_distance(env.get_ship(id).position);
                env.rounds_left()
                    <= dist
                        + (env.me().ship_ids.len() * env.config.navigation.go_home_safety_factor)
                            / (1 + env.me().dropoff_ids.len())
            },
        ),
        go_home(id),
    ]))
}
