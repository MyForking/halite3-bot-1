use behavior_tree::{
    continuous, interrupt, lambda, run_or_fail, select, sequence, BtNode, BtState,
};
use hlt::direction::Direction;
use hlt::log::Log;
use hlt::map_cell::Structure;
use hlt::position::Position;
use hlt::ShipId;
use std::cell::{Cell, RefCell};
use std::f64;
use std::rc::Rc;
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

            let ok_n = !state
                .mp
                .is_occupied(pos.directional_offset(Direction::North));
            let ok_s = !state
                .mp
                .is_occupied(pos.directional_offset(Direction::South));
            let ok_e = !state
                .mp
                .is_occupied(pos.directional_offset(Direction::East));
            let ok_w = !state
                .mp
                .is_occupied(pos.directional_offset(Direction::West));

            let cn = if ok_n { cn - c0 } else { i32::max_value() };
            let cs = if ok_s { cs - c0 } else { i32::max_value() };
            let ce = if ok_e { ce - c0 } else { i32::max_value() };
            let cw = if ok_w { cw - c0 } else { i32::max_value() };
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
            match state
                .game
                .map
                .at_position(&pos.directional_offset(d))
                .structure
            {
                Structure::Dropoff(did) if state.game.dropoffs[&did].owner == state.game.my_id => {
                    state.gns.force_move(id, d);
                    return BtState::Running;
                }
                Structure::Shipyard(pid) if pid == state.game.my_id => {
                    state.gns.force_move(id, d);
                    return BtState::Running;
                }
                _ => {}
            }
        }

        let [c0, cn, cs, ce, cw] = state.get_return_dir_costs(pos);

        let ok_n = !state
            .mp
            .is_occupied(pos.directional_offset(Direction::North));
        let ok_s = !state
            .mp
            .is_occupied(pos.directional_offset(Direction::South));
        let ok_e = !state
            .mp
            .is_occupied(pos.directional_offset(Direction::East));
        let ok_w = !state
            .mp
            .is_occupied(pos.directional_offset(Direction::West));

        let cn = if ok_n { cn - c0 } else { i32::max_value() };
        let cs = if ok_s { cs - c0 } else { i32::max_value() };
        let ce = if ok_e { ce - c0 } else { i32::max_value() };
        let cw = if ok_w { cw - c0 } else { i32::max_value() };
        let c0 = state.config.navigation.return_step_cost as i32;
        state.gns.plan_move(id, pos, c0, cn, cs, ce, cw);

        BtState::Running
    })
}

fn go_to(id: ShipId, dest: Position) -> Box<impl BtNode<GameState>> {
    lambda(move |state: &mut GameState| {
        let pos = state.get_ship(id).position;

        if pos == dest {
            return BtState::Success;
        }

        if stuck_move(id, state) {
            return BtState::Running;
        }

        let costs = state.get_dijkstra_move(pos, dest);
        state
            .gns
            .plan_move(id, pos, costs[4], costs[2], costs[3], costs[1], costs[0]);

        BtState::Running
    })
}

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

        /*Log::log(&format!("{:?}", id));
        Log::log(&format!(
            "    @ {:?}: {} halite; {} pheromone",
            pos, current_halite, phi0
        ));*/

        let mut weights: Vec<_> = if cargo < mc {
            vec![9999999.0, 0.0, 0.0, 0.0, 0.0]
        } else {
            Direction::get_all_options()
                .into_iter()
                .map(|d| pos.directional_offset(d))
                .map(|p| state.get_pheromone(p))
                .collect()
        };

        if current_halite < 1
            && weights[0] < 1.0
            && weights[1] < 1.0
            && weights[2] < 1.0
            && weights[3] < 1.0
        {
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

        //Log::log(&format!("    {:?}", weights));

        let ok_n = !state
            .mp
            .is_occupied(pos.directional_offset(Direction::North));
        let ok_s = !state
            .mp
            .is_occupied(pos.directional_offset(Direction::South));
        let ok_e = !state
            .mp
            .is_occupied(pos.directional_offset(Direction::East));
        let ok_w = !state
            .mp
            .is_occupied(pos.directional_offset(Direction::West));

        let cn = if ok_n {
            -(weights[2] * 100.0) as i32
        } else {
            i32::max_value()
        };
        let cs = if ok_s {
            -(weights[3] * 100.0) as i32
        } else {
            i32::max_value()
        };
        let ce = if ok_e {
            -(weights[1] * 100.0) as i32
        } else {
            i32::max_value()
        };
        let cw = if ok_w {
            -(weights[0] * 100.0) as i32
        } else {
            i32::max_value()
        };
        let c0 = -(weights[4] * 100.0) as i32;

        state.gns.plan_move(id, pos, c0, cn, cs, ce, cw);

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
            sequence(vec![
                select(vec![/*battle_camper(id), */ greedy(id)]),
                deliver(id),
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

pub fn battle_camper(id: ShipId) -> Box<impl BtNode<GameState>> {
    let mut target = Rc::new(Cell::new((
        Position { x: 4, y: 4 },
        Position { x: 5, y: 5 },
    )));

    sequence(vec![
        // 1. find target
        {
            let target = target.clone();
            lambda(move |state: &mut GameState| {
                if state.rounds_left() > 200 {
                    return BtState::Failure;
                }

                match state.camps.assign_ship(id) {
                    Some(p) => {
                        target.replace(p);
                        BtState::Success
                    }
                    None => BtState::Failure,
                }
            })
        },
        // 2. go to target
        {
            let target = target.clone();
            lambda(move |state: &mut GameState| {
                let dest = target.get().1;
                let pos = state.get_ship(id).position;

                if pos == dest {
                    return BtState::Success;
                }

                if stuck_move(id, state) {
                    return BtState::Running;
                }

                let costs = state.get_dijkstra_move(pos, dest);
                state
                    .gns
                    .plan_move(id, pos, costs[4], costs[2], costs[3], costs[1], costs[0]);

                BtState::Running
            })
        },
        // 3. wait...
        {
            let target = target.clone();
            lambda(move |state: &mut GameState| {
                const MIN_CARGO: usize = 600;

                let pos = state.get_ship(id).position;

                let center = target.get().0;

                let p1 = state.game.map.normalize(&Position {
                    x: center.x - 1,
                    y: center.y - 1,
                });
                let p2 = state.game.map.normalize(&Position {
                    x: center.x + 1,
                    y: center.y + 1,
                });

                if state.ship_map[p1.y as usize][p1.x as usize].is_none()
                    || state.ship_map[p2.y as usize][p2.x as usize].is_none()
                {
                    return BtState::Running;
                }

                let p_n = state.game.map.normalize(&Position {
                    x: center.x,
                    y: center.y - 2,
                });
                let p_s = state.game.map.normalize(&Position {
                    x: center.x,
                    y: center.y + 2,
                });
                let p_e = state.game.map.normalize(&Position {
                    x: center.x + 2,
                    y: center.y,
                });
                let p_w = state.game.map.normalize(&Position {
                    x: center.x - 2,
                    y: center.y,
                });

                let cargo_n = state.ship_map[p_n.y as usize][p_n.x as usize]
                    .map(|id| state.game.ships[&id].halite)
                    .unwrap_or(0);
                let cargo_s = state.ship_map[p_s.y as usize][p_s.x as usize]
                    .map(|id| state.game.ships[&id].halite)
                    .unwrap_or(0);
                let cargo_e = state.ship_map[p_e.y as usize][p_e.x as usize]
                    .map(|id| state.game.ships[&id].halite)
                    .unwrap_or(0);
                let cargo_w = state.ship_map[p_w.y as usize][p_w.x as usize]
                    .map(|id| state.game.ships[&id].halite)
                    .unwrap_or(0);

                match (pos.x - center.x, pos.y - center.y) {
                    (-1, -1) if cargo_n >= MIN_CARGO => state.gns.plan_move(
                        id,
                        pos,
                        i32::max_value(),
                        i32::max_value(),
                        i32::max_value(),
                        0,
                        i32::max_value(),
                    ),
                    (1, 1) if cargo_n >= MIN_CARGO => {
                        state.gns.plan_move(
                            id,
                            pos,
                            i32::max_value(),
                            0,
                            i32::max_value(),
                            i32::max_value(),
                            i32::max_value(),
                        );
                        return BtState::Success;
                    }

                    (-1, -1) if cargo_w >= MIN_CARGO => state.gns.plan_move(
                        id,
                        pos,
                        i32::max_value(),
                        i32::max_value(),
                        0,
                        i32::max_value(),
                        i32::max_value(),
                    ),
                    (1, 1) if cargo_w >= MIN_CARGO => {
                        state.gns.plan_move(
                            id,
                            pos,
                            i32::max_value(),
                            i32::max_value(),
                            i32::max_value(),
                            i32::max_value(),
                            0,
                        );
                        return BtState::Success;
                    }

                    (1, 1) if cargo_s >= MIN_CARGO => state.gns.plan_move(
                        id,
                        pos,
                        i32::max_value(),
                        i32::max_value(),
                        i32::max_value(),
                        i32::max_value(),
                        0,
                    ),
                    (-1, -1) if cargo_s >= MIN_CARGO => {
                        state.gns.plan_move(
                            id,
                            pos,
                            i32::max_value(),
                            i32::max_value(),
                            0,
                            i32::max_value(),
                            i32::max_value(),
                        );
                        return BtState::Success;
                    }

                    (1, 1) if cargo_e >= MIN_CARGO => state.gns.plan_move(
                        id,
                        pos,
                        i32::max_value(),
                        0,
                        i32::max_value(),
                        i32::max_value(),
                        i32::max_value(),
                    ),
                    (-1, -1) if cargo_e >= MIN_CARGO => {
                        state.gns.plan_move(
                            id,
                            pos,
                            i32::max_value(),
                            i32::max_value(),
                            i32::max_value(),
                            0,
                            i32::max_value(),
                        );
                        return BtState::Success;
                    }

                    _ => state.gns.plan_move(
                        id,
                        pos,
                        0,
                        i32::max_value(),
                        i32::max_value(),
                        i32::max_value(),
                        i32::max_value(),
                    ),
                }

                BtState::Running
            })
        },
        // 4. collect...
        {
            let target = target.clone();
            lambda(move |state: &mut GameState| BtState::Failure)
        },
    ])
}
