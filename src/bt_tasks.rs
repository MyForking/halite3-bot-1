use behavior_tree::{lambda, run_or_fail, select, sequence, BtNode, BtState};
use hlt::direction::Direction;
use hlt::ShipId;
use GameState;

type ShipEnv = (ShipId, GameState);

fn try_move_ship(d: Direction) -> Box<impl BtNode<ShipEnv>> {
    run_or_fail(move |(id, state): &mut ShipEnv| {
        let p = state.get_ship(*id).position;
        state.try_move_ship(*id, d)
    })
}

fn deliver() -> Box<impl BtNode<ShipEnv>> {
    lambda(|(id, state): &mut ShipEnv| {
        if state.get_ship(*id).halite <= 0 {
            return BtState::Success;
        }

        let pos = state.get_ship(*id).position;
        let dest = state.me().shipyard.position;
        let path = state.get_dijkstra_path(pos, dest);
        let d = path.first().cloned().unwrap_or(Direction::Still);
        state.move_ship_or_wait(*id, d);

        BtState::Running
    })
}

fn find_res() -> Box<impl BtNode<ShipEnv>> {
    lambda(|(id, state): &mut ShipEnv| {
        const SEEK_LIMIT: usize = 50;

        let pos = state.get_ship(*id).position;
        let current_halite = state.game.map.at_position(&pos).halite;

        if current_halite >= SEEK_LIMIT {
            return BtState::Success;
        }

        match state.get_nearest_halite_move(pos, SEEK_LIMIT) {
            Some(d) => {
                state.move_ship(*id, d);
                BtState::Running
            }
            None => BtState::Failure,
        }
    })
}

fn greedy() -> Box<impl BtNode<ShipEnv>> {
    lambda(|(id, state): &mut ShipEnv| {
        const PREFER_STAY_FACTOR: usize = 2;
        const HARVEST_LIMIT: usize = 10;
        const SEEK_LIMIT: usize = 50;

        if state.get_ship(*id).is_full() {
            return BtState::Success;
        }

        let (pos, cargo) = {
            let ship = state.get_ship(*id);
            (ship.position, ship.halite)
        };

        let movement_cost =
            state.game.map.at_position(&pos).halite / state.game.constants.move_cost_ratio;

        if cargo < movement_cost {
            state.move_ship(*id, Direction::Still);
            return BtState::Running;
        }

        let syp = state.me().shipyard.position;

        let current_halite = state.game.map.at_position(&pos).halite;
        let current_value = current_halite / state.game.constants.extract_ratio;

        let mut mov = Direction::get_all_cardinals()
            .into_iter()
            .map(|d| (d, pos.directional_offset(d)))
            .map(|(d, p)| {
                (
                    state.game.map.at_position(&p).halite,
                    state.game.map.at_position(&p).halite / state.game.constants.extract_ratio,
                    d,
                    p,
                )
            })
            .filter(|&(halite, _, _, _)| halite >= HARVEST_LIMIT)
            .filter(|&(_, _, _, p)| p != syp)
            .filter(|&(_, value, _, _)| value > movement_cost + current_value * PREFER_STAY_FACTOR)
            .filter(|(_, _, _, p)| state.navi.is_safe(p))
            .max_by_key(|&(_, value, _, _)| value)
            .map(|(_, _, d, p)| (d, p));

        if mov.is_none() && current_halite < SEEK_LIMIT {
            return BtState::Failure;
        }

        let (d, _) = mov.unwrap_or((Direction::Still, pos));

        state.move_ship(*id, d);

        BtState::Running
    })
}

fn collector() -> Box<impl BtNode<ShipEnv>> {
    select(vec![sequence(vec![greedy(), deliver()]), find_res()])
}
