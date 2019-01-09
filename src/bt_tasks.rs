use behavior_tree::{continuous, interrupt, lambda, run_or_fail, select, sequence, BtNode, BtState};
use hlt::direction::Direction;
use hlt::ShipId;
use GameState;

fn stuck_move(id: ShipId, state: &mut GameState) -> bool {
    let pos = state.get_ship(id).position;
    let cargo = state.get_ship(id).halite;
    let cap = state.get_ship(id).capacity();

    let harvest = state.config.navigation.return_step_cost as i32
        - state.halite_gain(&pos).min(cap) as i32; // we may actually gain something from waiting...

    if state.movement_cost(&pos) > cargo {
        state.gns.plan_move(id, pos, harvest, i32::max_value(), i32::max_value(), i32::max_value(), i32::max_value());
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

        state.add_pheromone(pos, cargo as f64);

        turns_taken += 1;

        BtState::Running
    })
}

fn go_home(id: ShipId) -> Box<impl BtNode<GameState>> {
    lambda(move |state: &mut GameState| {
        let pos = state.get_ship(id).position;
        let cargo = state.get_ship(id).halite;

        if stuck_move(id, state) {return BtState::Running}

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

        if stuck_move(id, state) {return BtState::Running}

        let pos = state.get_ship(id).position;
        let cap = state.get_ship(id).capacity();
        let cargo = state.get_ship(id).halite;

        let p0 = state.get_pheromone(pos);
        let pn = state.get_pheromone(pos.directional_offset(Direction::North));
        let ps = state.get_pheromone(pos.directional_offset(Direction::South));
        let pe = state.get_pheromone(pos.directional_offset(Direction::East));
        let pw = state.get_pheromone(pos.directional_offset(Direction::West));

        let [r0, rn, rs, re, rw] = state.get_return_dir_costs(pos);
        let rn = (rn - r0) as f64;
        let rs = (rs - r0) as f64;
        let re = (re - r0) as f64;
        let rw = (rw - r0) as f64;

        let mc = state.movement_cost(&pos) as i32;

        // todo: factor in neighboring halite deposits
        //       return cost factor did not seem to have much effect

        let cn = mc + ((pn - p0) * state.config.ships.seek_pheromone_cost + rn * state.config.ships.seek_return_cost_factor) as i32;
        let cs = mc + ((ps - p0) * state.config.ships.seek_pheromone_cost + rn * state.config.ships.seek_return_cost_factor) as i32;
        let ce = mc + ((pe - p0) * state.config.ships.seek_pheromone_cost + rn * state.config.ships.seek_return_cost_factor) as i32;
        let cw = mc + ((pw - p0) * state.config.ships.seek_pheromone_cost + rn * state.config.ships.seek_return_cost_factor) as i32;
        let c0 = -(state.halite_gain(&pos).min(cap) as i32);

        state.gns.plan_move(id, pos, c0, cn, cs, ce, cw);

        BtState::Running
    })
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
    continuous(
    select(vec![
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

pub fn kamikaze(id: ShipId) -> Box<impl BtNode<GameState>> {
    go_home(id)
}
