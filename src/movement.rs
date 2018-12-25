use hlt::direction::Direction;
use hlt::position::Position;
use hlt::ship::Ship;
use hlt::ShipId;
use rand::Rng;
use super::GameState;

pub fn greedy(state: &mut GameState, ship_id: ShipId) -> Direction {
    const PREFER_MOVE_FACTOR: usize = 2;

    let (pos, cargo) = {
        let ship = state.get_ship(ship_id);
        (ship.position, ship.halite)
    };

    let movement_cost = state.game.map.at_position(&pos).halite / state.game.constants.move_cost_ratio;

    if cargo < movement_cost {
        return Direction::Still;
    }

    let current_value = state.game.map.at_position(&pos).halite / state.game.constants.extract_ratio;

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
        .filter(|&(value, _, _)| {
            value > movement_cost + current_value * PREFER_MOVE_FACTOR
        })
        .filter(|(_, _, p)| state.navi.is_safe(p))
        .max_by_key(|&(value, _, _)| value);

    // hope this prevents cycling between two empty tiles
    if mov.is_none() && current_value == 0 {
        let all: Vec<_> = Direction::get_all_cardinals()
            .into_iter()
            .filter(|&d| state.navi.is_safe(&pos.directional_offset(d)))
            .collect();
        if let Some(&d) = rand::thread_rng().choose(&all) {
            let p = pos.directional_offset(d);
            state.navi.mark_unsafe(&p, ship_id);
            return d;
        }
    }

    let (d, p) = mov
        .map(|(_, d, p)| (d, p))
        .unwrap_or((Direction::Still, pos));

    state.navi.mark_unsafe(&p, ship_id);
    d
}

pub fn seek(state: &mut GameState, ship_id: ShipId) -> Direction {
    let target_pos = {
        let ship = state.get_ship(ship_id);

        let movement_cost = state.game.map.at_entity(ship).halite / state.game.constants.move_cost_ratio;

        if ship.halite < movement_cost {
            return Direction::Still;
        }

        let target = state.game
            .map
            .cells
            .iter()
            .flat_map(|sub| sub.iter())
            .max_by_key(|cell| cell.halite)
            .unwrap();

        let current_value = state.game.map.at_entity(ship).halite / state.game.constants.extract_ratio;

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

    if p == dest && state.game
        .ships
        .values()
        .filter(|ship| ship.owner != state.me().id)
        .any(|ship| ship.position == dest) {
        return d
    }

    if !state.navi.is_safe(&p)  {
        return Direction::Still;
    } else {
        state.navi.mark_unsafe(&p, ship_id);
        return d;
    }
}