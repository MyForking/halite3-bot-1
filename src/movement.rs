use super::GameState;
use hlt::direction::Direction;
use hlt::log::Log;
use hlt::position::Position;
use hlt::ship::Ship;
use hlt::ShipId;
use rand::Rng;
use std::collections::{VecDeque, HashSet};

pub fn greedy(state: &mut GameState, ship_id: ShipId) -> Direction {
    const PREFER_MOVE_FACTOR: usize = 2;
    const SEEK_LIMIT: usize = 50;

    let (pos, cargo) = {
        let ship = state.get_ship(ship_id);
        (ship.position, ship.halite)
    };

    let movement_cost =
        state.game.map.at_position(&pos).halite / state.game.constants.move_cost_ratio;

    if cargo < movement_cost {
        return Direction::Still;
    }

    let syp = state.me().shipyard.position;

    let current_value =
        state.game.map.at_position(&pos).halite / state.game.constants.extract_ratio;

    let mut mov = Direction::get_all_cardinals()
        .into_iter()
        .map(|d| (d, pos.directional_offset(d)))
        .map(|(d, p)| {
            (
                state.game.map.at_position(&p).halite / state.game.constants.extract_ratio,
                d,
                p,
            )
        })
        .filter(|&(value, _, _)| value >= 10)
        .filter(|&(_, _, p)| p != syp)
        .filter(|&(value, _, _)| value > movement_cost + current_value * PREFER_MOVE_FACTOR)
        .filter(|(_, _, p)| state.navi.is_safe(p))
        .max_by_key(|&(value, _, _)| value)
        .map(|(_, d, p)| (d, p));

    // if there is nothing to gather, find new resource location
    if mov.is_none() && current_value < 10 {
        let mut queue = VecDeque::new();
        for d in Direction::get_all_cardinals() {
            let p = pos.directional_offset(d);
            queue.push_back((p, d));
        }
        let mut visited = HashSet::new();
        while let Some((mut p, d)) = queue.pop_front() {
            p = state.game.map.normalize(&p);
            if visited.contains(&p) { continue }
            visited.insert(p);
            //Log::log(&format!("greedy ship {:?} evaluating position {:?}.", ship_id, p));
            if p == syp { continue }
            if state.navi.is_unsafe(&p)  { continue }
            if state.game.map.at_position(&p).halite >= SEEK_LIMIT {
                mov = Some((d, pos.directional_offset(d)));
                Log::log(&format!("greedy ship {:?} found new target: {:?}.", ship_id, p));
                break
            }
            for dn in Direction::get_all_cardinals() {
                let pn = p.directional_offset(dn);
                queue.push_back((pn, d));
            }
        }
        if mov.is_none() {Log::log(&format!("greedy ship {:?} does not know where to go.", ship_id)); }
    }

    let (d, p) = mov.unwrap_or((Direction::Still, pos));

    state.navi.mark_unsafe(&p, ship_id);
    d
}

pub fn seek(state: &mut GameState, ship_id: ShipId) -> Direction {
    let target_pos = {
        let ship = state.get_ship(ship_id);

        let movement_cost =
            state.game.map.at_entity(ship).halite / state.game.constants.move_cost_ratio;

        if ship.halite < movement_cost {
            return Direction::Still;
        }

        let target = state
            .game
            .map
            .cells
            .iter()
            .flat_map(|sub| sub.iter())
            .max_by_key(|cell| cell.halite)
            .unwrap();

        let current_value =
            state.game.map.at_entity(ship).halite / state.game.constants.extract_ratio;

        if current_value * 4 >= target.halite * 3 {
            return Direction::Still;
        }

        target.position
    };

    let ship = state.get_ship(ship_id).clone();
    state.navi.naive_navigate(&ship, &target_pos)
}

pub fn return_naive(state: &mut GameState, ship_id: ShipId) -> Direction {
    let ship = state.get_ship(ship_id).clone();
    let dest = state.game.players[state.game.my_id.0].shipyard.position;
    state.navi.naive_navigate(&ship, &dest)
}

pub fn return_dijkstra(state: &mut GameState, ship_id: ShipId) -> Direction {
    const STEP_COST: i64 = 1; // fixed cost of one step - tweak to prefer shorter paths

    let pos = state.get_ship(ship_id).position;

    let dest = state.me().shipyard.position;

    let d = state.get_dijkstra_move(pos, dest);
    let p = pos.directional_offset(d);
    if !state.navi.is_safe(&p) {
        return Direction::Still;
    } else {
        state.navi.mark_unsafe(&p, ship_id);
        return d;
    }
}

pub fn kamikaze(state: &mut GameState, ship_id: ShipId) -> Direction {
    const STEP_COST: i64 = 1; // fixed cost of one step - tweak to prefer shorter paths

    let pos = state.get_ship(ship_id).position;

    let dest = state.me().shipyard.position;

    let d = state.get_dijkstra_move(pos, dest);
    let p = pos.directional_offset(d);

    if p == dest
        && state
            .game
            .ships
            .values()
            .filter(|ship| ship.owner != state.me().id)
            .any(|ship| ship.position == dest)
    {
        return d;
    }

    if !state.navi.is_safe(&p) {
        return Direction::Still;
    } else {
        state.navi.mark_unsafe(&p, ship_id);
        return d;
    }
}
