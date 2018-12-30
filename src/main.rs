#[macro_use]
extern crate lazy_static;
extern crate rand;

use hlt::command::Command;
use hlt::direction::Direction;
use hlt::game::Game;
use hlt::log::Log;
use hlt::navi::Navi;
use hlt::player::Player;
use hlt::position::Position;
use hlt::ship::Ship;
use hlt::ShipId;
//use rand::SeedableRng;
//use rand::XorShiftRng;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::env;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

mod hlt;
mod movement;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ShipTask {
    Greedy,
    Cleaner,
    ReturnNaive,
    ReturnDijkstra,
    Kamikaze,
    GoHome,
}

impl ShipTask {
    fn get_move(&self, state: &mut GameState, ship_id: ShipId) -> Direction {
        match self {
            ShipTask::Greedy => movement::greedy(state, ship_id),
            ShipTask::Cleaner => movement::cleaner(state, ship_id),
            ShipTask::ReturnNaive => movement::return_naive(state, ship_id),
            ShipTask::ReturnDijkstra => movement::return_dijkstra(state, ship_id),
            ShipTask::Kamikaze => movement::kamikaze(state, ship_id),
            ShipTask::GoHome => movement::go_home(state, ship_id),
        }
    }

    fn is_returning(&self) -> bool {
        match self {
            ShipTask::ReturnNaive | ShipTask::ReturnDijkstra => true,
            _ => false,
        }
    }
}

#[derive(Eq, PartialEq)]
struct DijkstraNode<C: Ord, T: Eq> {
    cost: C,
    data: T,
}

impl<C: Ord, T: Eq> std::cmp::PartialOrd for DijkstraNode<C, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.cost.partial_cmp(&other.cost)
    }
}

impl<C: Ord, T: Eq> std::cmp::Ord for DijkstraNode<C, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cost.cmp(&other.cost)
    }
}

impl<C: Ord, T: Eq> DijkstraNode<C, T> {
    fn new(cost: C, data: T) -> Self {
        DijkstraNode { cost, data }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ShipAI {
    Collector(ShipTask),
    Cleaner(ShipTask),
    Kamikaze,
    GoHome,
}

impl ShipAI {
    fn new_collector() -> Self {
        ShipAI::Collector(ShipTask::Greedy)
    }

    fn new_cleaner() -> Self {
        ShipAI::Cleaner(ShipTask::Cleaner)
    }

    fn think(&mut self, ship_id: ShipId, state: &mut GameState) {
        match self {
            ShipAI::Collector(ref mut task) => {
                {
                    let ship = state.get_ship(ship_id);
                    match task {
                        ShipTask::Greedy if ship.is_full() => *task = ShipTask::ReturnDijkstra,
                        ShipTask::ReturnDijkstra if ship.halite == 0 => *task = ShipTask::Greedy,
                        _ => {}
                    }
                }
                let d = task.get_move(state, ship_id);
                state.move_ship(ship_id, d);
            }

            ShipAI::Cleaner(ref mut task) => {
                {
                    let ship = state.get_ship(ship_id);
                    match task {
                        ShipTask::Cleaner if ship.is_full() => *task = ShipTask::ReturnDijkstra,
                        ShipTask::ReturnDijkstra if ship.halite == 0 => *task = ShipTask::Cleaner,
                        _ => {}
                    }
                }
                let d = task.get_move(state, ship_id);
                state.move_ship(ship_id, d);
            }

            ShipAI::Kamikaze => {
                let d = ShipTask::Kamikaze.get_move(state, ship_id);
                state.move_ship(ship_id, d);
            }

            ShipAI::GoHome => {
                let d = ShipTask::GoHome.get_move(state, ship_id);
                state.move_ship(ship_id, d);
            }
        }
    }

    fn is_returning_collector(&self) -> bool {
        if let ShipAI::Collector(task) = self {
            task.is_returning()
        } else {
            false
        }
    }
}

pub struct GameState {
    game: Game,
    navi: Navi,
    command_queue: Vec<Command>,
    collect_statistic: Vec<f64>,
    last_halite: usize,

    halite_percentiles: [usize; 101],
}

impl GameState {
    fn new() -> Self {
        let game = Game::new();
        let state = GameState {
            navi: Navi::new(game.map.width, game.map.height),
            command_queue: vec![],
            collect_statistic: Vec::with_capacity(game.constants.max_turns),
            last_halite: 5000,
            game,

            halite_percentiles: [0; 101],
        };

        Game::ready("MyRustBot");

        state
    }

    fn update_frame(&mut self) {
        self.game.update_frame();
        self.navi.update_frame(&self.game);

        let mut map_halite: Vec<_> = self.game.map.iter().map(|cell| cell.halite).collect();
        map_halite.sort_unstable();
        let n = map_halite.len() - 1;
        for i in 0..=100 {
            self.halite_percentiles[i] = map_halite[(n * i) / 100];
        }
        //Log::log(&format!("Halite quartiles: {:?}", self.halite_percentiles));

        if self.me().halite > self.last_halite {
            let diff =
                (self.me().halite - self.last_halite) as f64 / self.me().ship_ids.len() as f64;
            self.collect_statistic.push(diff);
        } else {
            self.collect_statistic.push(0.0);
        }

        self.last_halite = self.me().halite;

        self.command_queue.clear();

        let ids = self.me().ship_ids.clone();
        //self.ship_ais.retain(|ship_id, _| ids.contains(ship_id));
    }

    fn finalize_frame(&mut self) {
        //Log::log(&format!("issuing commands: {:?}", command_queue));

        Game::end_turn(&self.command_queue);

        if self.game.turn_number == self.game.constants.max_turns {
            Log::log(&format!("collection rate: {:?}", self.collect_statistic));
        }
    }

    fn rounds_left(&self) -> usize {
        self.game.constants.max_turns - self.game.turn_number
    }

    fn me(&self) -> &Player {
        &self.game.players[self.game.my_id.0]
    }

    fn my_ships<'a>(&'a self) -> impl Iterator<Item = ShipId> + 'a {
        self.me().ship_ids.iter().cloned()
    }

    fn get_ship(&self, id: ShipId) -> &Ship {
        &self.game.ships[&id]
    }

    fn move_ship(&mut self, id: ShipId, d: Direction) {
        let cmd = self.get_ship(id).move_ship(d);
        self.command_queue.push(cmd);
    }

    fn get_nearest_halite_move(&self, start: Position, min_halite: usize) -> Option<Direction> {
        let mut queue = VecDeque::new();
        for d in Direction::get_all_cardinals() {
            let p = start.directional_offset(d);
            queue.push_back((p, d));
        }
        let mut visited = HashSet::new();
        while let Some((mut p, d)) = queue.pop_front() {
            p = self.game.map.normalize(&p);
            if visited.contains(&p) { continue }
            visited.insert(p);
            if p == self.me().shipyard.position { continue }
            if self.navi.is_unsafe(&p)  { continue }
            if self.game.map.at_position(&p).halite >= min_halite {
                return Some(d);
            }
            for dn in Direction::get_all_cardinals() {
                let pn = p.directional_offset(dn);
                queue.push_back((pn, d));
            }
        }
        None
    }

    fn get_dijkstra_path(&self, start: Position, dest: Position) -> Vec<Direction> {
        const STEP_COST: i64 = 1; // fixed cost of one step - tweak to prefer shorter paths

        let mut visited = HashSet::new();

        let mut queue = BinaryHeap::new();
        queue.push(DijkstraNode::new(0, (start, vec![])));

        let maxlen = ((start.x - dest.x).abs() + (start.y - dest.y).abs()).max(5) * 2; // todo: tweak me

        while let Some(node) = queue.pop() {
            let (mut pos, path) = node.data;
            pos = self.game.map.normalize(&pos);

            if path.len() > maxlen as usize {
                continue;
            }

            if pos == dest {
                return path;
            }

            if visited.contains(&pos) {
                continue;
            }
            visited.insert(pos);

            let movement_cost =
                self.game.map.at_position(&pos).halite / self.game.constants.move_cost_ratio;

            for d in Direction::get_all_cardinals() {
                let p = pos.directional_offset(d);
                if !self.navi.is_safe(&p) && p != dest {
                    continue;
                }
                // keep one path open
                if p.x == dest.x + 1 && p.y == dest.y {
                    continue;
                }
                let mut newpath = path.clone();
                newpath.push(d);
                queue.push(DijkstraNode::new(
                    node.cost as i64 - movement_cost as i64 - STEP_COST,
                    (p, newpath),
                ));
            }
        }
        vec![]
    }
}

#[derive(Default)]
struct Commander {
    new_ships: HashSet<ShipId>,
    lost_ships: HashSet<ShipId>,
    ships: HashSet<ShipId>,
    ship_ais: HashMap<ShipId, ShipAI>,

    kamikaze: Option<ShipId>,
}

impl Commander {
    fn new() -> Self {
        Self::default()
    }

    fn sync(&mut self, state: &GameState) {
        let state_ships: HashSet<_> = state.my_ships().collect();

        self.new_ships.extend(&state_ships - &self.ships);
        self.lost_ships.extend(&self.ships - &state_ships);
        self.ships = &self.ships & &state_ships;
    }

    fn process_frame(&mut self, state: &mut GameState) {
        let shipyard_pos = state.me().shipyard.position;

        for id in self.lost_ships.drain() {
            self.ship_ais.remove(&id);
            if self.kamikaze == Some(id) {
                self.kamikaze = None
            }
        }

        for id in self.new_ships.drain() {
            self.ships.insert(id);
            self.ship_ais.insert(id, ShipAI::new_collector());
        }

        let syp = state.me().shipyard.position;

        if let Some(id) = self.kamikaze {
            if state.get_ship(id).position == syp {
                *self.ship_ais.get_mut(&id).unwrap() = ShipAI::new_collector();
                self.kamikaze = None;
            }
        }

        for (&id, ai) in &mut self.ship_ais {
            if state.rounds_left() < 150 && ai != &ShipAI::GoHome {
                const GO_HOME_SAFETY_FACTOR: usize = 1;

                let path = state.get_dijkstra_path(state.get_ship(id).position, shipyard_pos);

                if path.len() >= state.rounds_left() - self.ships.len() * GO_HOME_SAFETY_FACTOR {
                    *ai = ShipAI::GoHome;
                }
            }

            if state.halite_percentiles[99] < 100 {
                if let ShipAI::Collector(_) = ai {
                    *ai = ShipAI::new_cleaner();
                }
            }

            if state.get_ship(id).position == syp && ai != &ShipAI::GoHome {
                Log::log(&format!("force moving ship {:?} from spawn", id));
                for d in Direction::get_all_cardinals() {
                    let p = syp.directional_offset(d);
                    if state.navi.is_safe(&p) {
                        state.navi.mark_unsafe(&p, id);
                        state.move_ship(id, d);
                        Log::log(&format!("        to {:?}", d));
                        break
                    }
                }
            } else {
                ai.think(id, state);
            }

        }

        let enemy_blocks = state
            .game
            .ships
            .values()
            .filter(|ship| ship.owner != state.me().id)
            .any(|ship| ship.position == state.me().shipyard.position);

        if enemy_blocks && self.kamikaze.is_none() {
            let t = state.me().shipyard.position;
            if let Some((id, _)) = self
                .ship_ais
                .iter()
                .filter(|(_, ai)| ai.is_returning_collector())
                .map(|(&id, _)| (id, state.get_ship(id).position))
                .map(|(id, pos)| (id, (pos.x - t.x).abs() + (pos.y - t.y).abs()))
                .min_by_key(|&(_, dist)| dist)
            {
                self.kamikaze = Some(id);
                *self.ship_ais.get_mut(&id).unwrap() = ShipAI::Kamikaze;
            }
        }

        let want_ship = if state.game.turn_number > 100 {
            // average halite collected per ship in the last n turns
            let avg_collected = state.collect_statistic[state.game.turn_number - 100..]
                .iter()
                .sum::<f64>()
                / 100.0;

            let predicted_profit = avg_collected * state.rounds_left() as f64;

            predicted_profit as usize > state.game.constants.ship_cost * 2 // safety factor...
        } else {
            true
        };

        if enemy_blocks && state.me().halite >= state.game.constants.ship_cost
            || (want_ship && state.navi.is_safe(&state.me().shipyard.position))
                && state.me().halite >= state.game.constants.ship_cost
        {
            let cmd = state.me().shipyard.spawn();
            state.command_queue.push(cmd);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let rng_seed: u64 = if args.len() > 1 {
        args[1].parse().unwrap()
    } else {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    };
    /*let seed_bytes: Vec<u8> = (0..16).map(|x| ((rng_seed >> (x % 8)) & 0xFF) as u8).collect();
    let mut rng: XorShiftRng = SeedableRng::from_seed([
        seed_bytes[0], seed_bytes[1], seed_bytes[2], seed_bytes[3],
        seed_bytes[4], seed_bytes[5], seed_bytes[6], seed_bytes[7],
        seed_bytes[8], seed_bytes[9], seed_bytes[10], seed_bytes[11],
        seed_bytes[12], seed_bytes[13], seed_bytes[14], seed_bytes[15]
    ]);*/

    let mut commander = Commander::new();
    let mut game = GameState::new();

    loop {
        game.update_frame();

        commander.sync(&game);

        commander.process_frame(&mut game);

        game.finalize_frame();
    }
}
