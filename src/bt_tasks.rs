use behavior_tree::{interrupt, lambda, run_or_fail, select, sequence, BtNode, BtState};
use hlt::direction::Direction;
use hlt::log::Log;
use hlt::ShipId;
use GameState;
use std::f64;

fn deliver(id: ShipId) -> Box<impl BtNode<GameState>> {
    let mut turns_taken = 0;
    lambda(move |state: &mut GameState| {
        if state.get_ship(id).halite <= 0 {
            state.notify_return(turns_taken);
            return BtState::Success;
        }

        let pos = state.get_ship(id).position;
        //let dest = state.me().shipyard.position;
        //let path = state.get_dijkstra_path(pos, dest);
        //let d = path.first().cloned().unwrap_or(Direction::Still);
        let d = state.get_return_dir(pos);
        if !state.try_move_ship(id, d) {
            let d = state.get_return_dir_alternative(pos);
            state.move_ship_or_wait(id, d);
        }

        /*let cargo = state.get_ship(id).halite;
        state.add_pheromone(pos, cargo as f64);*/

        turns_taken += 1;

        BtState::Running
    })
}

fn go_home(id: ShipId) -> Box<impl BtNode<GameState>> {
    lambda(move |state: &mut GameState| {
        let pos = state.get_ship(id).position;
        //let dest = state.me().shipyard.position;
        //let path = state.get_dijkstra_path(pos, dest);
        //let d = path.first().cloned().unwrap_or(Direction::Still);
        let d = state.get_return_dir(pos);
        let p = pos.directional_offset(d);

        if state.game.map.at_position(&p).structure.is_some() {
            state.move_ship(id, d);
        } else {
            if !state.try_move_ship(id, d) {
                let d = state.get_return_dir_alternative(pos);
                state.move_ship_or_wait(id, d);
            }
        }

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

fn find_res(id: ShipId) -> Box<impl BtNode<GameState>> {
    lambda(move |state: &mut GameState| {
        let pos = state.get_ship(id).position;
        let current_halite = state.game.map.at_position(&pos).halite;

        if current_halite >= state.config.ships.greedy_seek_limit {
            return BtState::Success;
        }

        let d = Direction::get_all_options()
            .into_iter()
            .map(|d| (d, state.game.map.normalize(&pos.directional_offset(d))))
            .filter(|(_, p)| state.navi.is_safe(p) || *p == pos)
            .max_by_key(|(_, p)| (state.get_pheromone(*p) * 1000.0) as i32)
            .map(|(d, _)| d)
            .unwrap_or(Direction::Still);

        state.move_ship(id, d);

        BtState::Running

        /*match state.get_nearest_halite_move(pos, SEEK_LIMIT) {
            Some(d) => {
                state.move_ship(id, d);
                BtState::Running
            }
            None => BtState::Failure,
        }*/
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
            Some(d) => {
                state.move_ship(id, d);
                BtState::Running
            }
            None => BtState::Failure,
        }
    })
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

fn greedy(id: ShipId) -> Box<impl BtNode<GameState>> {
    lambda(move |state: &mut GameState| {
        if state.get_ship(id).is_full() {
            return BtState::Success;
        }

        let (pos, cargo) = {
            let ship = state.get_ship(id);
            (ship.position, ship.halite)
        };

        let movement_cost =
            state.game.map.at_position(&pos).halite / state.game.constants.move_cost_ratio;

        if cargo < movement_cost {
            state.move_ship(id, Direction::Still);
            return BtState::Running;
        }

        let syp = state.me().shipyard.position;

        let current_halite = state.game.map.at_position(&pos).halite;
        let current_value = current_halite / state.game.constants.extract_ratio;
        let phi0 = state.get_pheromone(pos);

        Log::log(&format!("{:?}", id));
        Log::log(&format!("    @ {:?}: {} halite; {} pheromone", pos, current_halite, phi0));

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

        let mut weights: Vec<_> = if cargo < movement_cost {
                vec![f64::INFINITY, 0.0, 0.0, 0.0, 0.0]
            } else {
                Direction::get_all_options().into_iter()
                    .map(|d| pos.directional_offset(d))
                    .map(|p| if state.navi.is_unsafe(&p) || p == syp { 0.0 } else { state.get_pheromone(p) })
                    .collect()
            };

        if current_halite > state.config.ships.greedy_harvest_limit {
            weights[4] = 1000.0 + current_halite as f64;
        } else if current_halite as f64 > phi0 {
            weights[4] = current_halite as f64;
        }

        Log::log(&format!("    {:?}", weights));

        let dir = weights.iter().enumerate()
            .max_by(|&(_, a), &(_, b)|
                if a <= b {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                })
            .map(|(i, _)| match i {
                0 => Direction::West,
                1 => Direction::East,
                2 => Direction::North,
                3 => Direction::South,
                4 => Direction::Still,
                _ => unreachable!(),
            }).unwrap();

        state.move_ship(id, dir);

        BtState::Running

        /*if current_halite >= state.config.ships.greedy_harvest_limit {
            state.move_ship(id, Direction::Still);
            return BtState::Running;
        }

        let mov = Direction::get_all_cardinals()
            .into_iter()
            .map(|d| (d, pos.directional_offset(d)))
            .map(|(d, p)| {
                (
                    state.game.map.at_position(&p).halite / state.game.constants.extract_ratio,
                    d,
                    p,
                )
            })
            .filter(|&(_, _, p)| p != syp)
            .filter(|&(value, _, _)| {
                value > movement_cost + current_value * state.config.ships.greedy_prefer_stay_factor
            })
            .filter(|(_, _, p)| state.navi.is_safe(p))
            .map(|(value, d, p)| {
                (
                    value
                        + (state.get_pheromone(p) * state.config.ships.greedy_pheromone_weight)
                            as usize,
                    d,
                    p,
                )
            })
            .max_by_key(|&(value, _, _)| value)
            .map(|(_, d, _)| d);

        let d = match mov {
            None if current_halite < state.config.ships.greedy_seek_limit => {
                return BtState::Failure
            }
            None => Direction::Still,
            Some(d) => d,
        };

        state.move_ship(id, d);

        BtState::Running
        */
    })
}

fn desperate(id: ShipId) -> Box<impl BtNode<GameState>> {
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
}

pub fn build_dropoff(id: ShipId) -> Box<impl BtNode<GameState>> {
    sequence(vec![
        // todo: move a few steps up the density gradient
        run_or_fail(move |state: &mut GameState| state.try_build_dropoff(id)),
    ])
}

pub fn collector(id: ShipId) -> Box<impl BtNode<GameState>> {
    select(vec![
        interrupt(
            select(vec![
                sequence(vec![greedy(id), deliver(id)]),
                find_res(id),
                sequence(vec![desperate(id), deliver(id)]),
                find_desperate(id),
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
    ])
}

pub fn kamikaze(id: ShipId) -> Box<impl BtNode<GameState>> {
    go_home(id)
}
