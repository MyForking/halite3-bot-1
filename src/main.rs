#[macro_use]
extern crate lazy_static;
extern crate rand;

use behavior_tree::BtNode;
use bt_tasks::{build_dropoff, collector, kamikaze};
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
//use std::env;
//use std::time::SystemTime;
//use std::time::UNIX_EPOCH;

mod behavior_tree;
mod bt_tasks;
mod hlt;

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

pub struct GameState {
    game: Game,
    navi: Navi,
    command_queue: Vec<Command>,
    collect_statistic: Vec<f64>,
    last_halite: usize,

    halite_density: Vec<Vec<i32>>,

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

            halite_density: vec![vec![0; game.map.width]; game.map.height],

            halite_percentiles: [0; 101],

            game,
        };

        Game::ready("MyRustBot");

        state
    }

    fn update_frame(&mut self) {
        self.game.update_frame();
        self.navi.update_frame(&self.game);

        self.compute_halite_density();

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

    fn distance_to_nearest_dropoff(&self, id: ShipId) -> usize {
        let pos = self.get_ship(id).position;
        let mut dist = self.game.map.calculate_distance(&self.me().shipyard.position, &pos);

        self.me().dropoff_ids.iter()
            .map(|did| self.game.dropoffs[did].position)
            .map(|p| self.game.map.calculate_distance(&p, &pos))
            .fold(dist, |dist, d| dist.min(d))
    }

    fn try_build_dropoff(&mut self, id: ShipId) -> bool {
        if self.me().halite < self.game.constants.dropoff_cost {
            return false
        }

        let cmd = self.get_ship(id).make_dropoff();
        self.command_queue.push(cmd);

        true
    }

    fn can_move(&self, id: ShipId) -> bool {
        let ship = self.get_ship(id);
        ship.halite
            >= self.game.map.at_position(&ship.position).halite
                / self.game.constants.move_cost_ratio
    }

    fn move_ship(&mut self, id: ShipId, mut d: Direction) {
        if self.can_move(id) {
            let p0 = self.get_ship(id).position;
            let p1 = p0.directional_offset(d);
            self.navi.mark_safe(&p0);
            self.navi.mark_unsafe(&p1, id);
        } else {
            d = Direction::Still;
        }
        let cmd = self.get_ship(id).move_ship(d);
        self.command_queue.push(cmd);
    }

    fn try_move_ship(&mut self, id: ShipId, d: Direction) -> bool {
        if !self.can_move(id) {
            return false;
        }
        let p0 = self.get_ship(id).position;
        let p1 = p0.directional_offset(d);
        if self.navi.is_safe(&p1) {
            self.navi.mark_safe(&p0);
            self.navi.mark_unsafe(&p1, id);
            let cmd = self.get_ship(id).move_ship(d);
            self.command_queue.push(cmd);
            true
        } else {
            false
        }
    }

    fn move_ship_or_wait(&mut self, id: ShipId, d: Direction) {
        if !self.try_move_ship(id, d) {
            let cmd = Command::move_ship(id, Direction::Still);
            self.command_queue.push(cmd);
        }
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
            if visited.contains(&p) {
                continue;
            }
            visited.insert(p);
            if p == self.me().shipyard.position {
                continue;
            }
            if self.navi.is_unsafe(&p) {
                continue;
            }
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

    fn compute_halite_density(&mut self) {
        let r = 5_i32;
        let n = 2 * r * (r + 1) + 1; // number of pixels within manhatten distance of r

        for (i, row) in self.halite_density.iter_mut().enumerate() {
            for (j, d) in row.iter_mut().enumerate() {
                *d = 0;
                for a in -r..=r {
                    for b in -r..=r {
                        if a.abs() + b.abs() > r {
                            continue;
                        }
                        *d += self
                            .game
                            .map
                            .at_position(&Position {
                                x: j as i32 - b,
                                y: i as i32 - a,
                            })
                            .halite as i32;
                    }
                }
                *d /= n
            }
        }
    }
}

struct Commander {
    new_ships: HashSet<ShipId>,
    lost_ships: HashSet<ShipId>,
    ships: HashSet<ShipId>,
    ship_ais: HashMap<ShipId, Box<dyn BtNode<GameState>>>,

    builder: Option<ShipId>,
    kamikaze: Option<ShipId>,
}

impl Commander {
    fn new() -> Self {
        Commander {
            new_ships: HashSet::new(),
            lost_ships: HashSet::new(),
            ships: HashSet::new(),
            ship_ais: HashMap::new(),
            builder: None,
            kamikaze: None,
        }
    }

    fn sync(&mut self, state: &GameState) {
        let state_ships: HashSet<_> = state.my_ships().collect();

        self.new_ships.extend(&state_ships - &self.ships);
        self.lost_ships.extend(&self.ships - &state_ships);
        self.ships = &self.ships & &state_ships;
    }

    fn process_frame(&mut self, state: &mut GameState) {
        for id in self.lost_ships.drain() {
            self.ship_ais.remove(&id);
            if self.kamikaze == Some(id) {
                self.kamikaze = None
            }
            if self.builder == Some(id) {
                self.builder = None
            }
        }

        for id in self.new_ships.drain() {
            self.ships.insert(id);
            self.ship_ais.insert(id, collector(id));
        }

        let syp = state.me().shipyard.position;

        if let Some(id) = self.kamikaze {
            if state.get_ship(id).position == syp {
                *self.ship_ais.get_mut(&id).unwrap() = collector(id);
                self.kamikaze = None;
            }
        }

        let want_dropoff = state.game.turn_number > 100 && state.me().dropoff_ids.is_empty();

        if want_dropoff && state.me().halite >= state.game.constants.dropoff_cost {
            const EXPANSION_DISTANCE: usize = 10;
            const MIN_EXPANSION_DENSITY: i32 = 100;

            let id = self.ships.iter()
                .filter(|&&id| state.distance_to_nearest_dropoff(id) >= EXPANSION_DISTANCE)
                .map(|&id| {
                    let p = state.get_ship(id).position;
                    (id, state.halite_density[p.y as usize][p.x as usize])
                })
                .filter(|&(_, density)| density >= MIN_EXPANSION_DENSITY)
                .max_by_key(|&(_, density)| density)
                .map(|(id, _)| id);

            self.builder = id;

            if let Some(id) = id {
                *self.ship_ais.get_mut(&id).unwrap() = build_dropoff(id);
            }
        }

        for (&id, ai) in &mut self.ship_ais {
            if state.get_ship(id).position == syp {
                Log::log(&format!("force moving ship {:?} from spawn", id));
                for d in Direction::get_all_cardinals() {
                    if state.try_move_ship(id, d) {
                        Log::log(&format!("        to {:?}", d));
                        break;
                    }
                }
            } else {
                ai.tick(state);
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
                //.filter(|(_, ai)| ai.is_returning_collector())
                .map(|(&id, _)| (id, state.get_ship(id).position))
                .map(|(id, pos)| (id, (pos.x - t.x).abs() + (pos.y - t.y).abs()))
                .min_by_key(|&(_, dist)| dist)
            {
                self.kamikaze = Some(id);
                *self.ship_ais.get_mut(&id).unwrap() = kamikaze(id);
            }
        }

        let mut want_ship = if state.game.turn_number > 100 {
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

        want_ship &= !want_dropoff || state.me().halite >= state.game.constants.dropoff_cost + state.game.constants.ship_cost;

        /*match (want_dropoff, self.builder) {
            (Some(target_pos), None) => {
                let id = self.ships.iter()
                    .map(|&id| (state.game.map.calculate_distance(&target_pos, &state.get_ship(id).position), id))
                    .min_by_key(|(d, _)| *d)
                    .map(|(_, id)| id);
                self.builder = id;
                if let Some(id) = id {
                    *self.ship_ais.get_mut(&id).unwrap() = builder(id, target_pos);
                }
            }
            _ => {}
        }*/

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
    //let args: Vec<String> = env::args().collect();
    /*let rng_seed: u64 = if args.len() > 1 {
        args[1].parse().unwrap()
    } else {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    };*/
    /*let seed_bytes: Vec<u8> = (0..16).map(|x| ((rng_seed >> (x % 8)) & 0xFF) as u8).collect();
    let mut rng: XorShiftRng = SeedableRng::from_seed([
        seed_bytes[0], seed_bytes[1], seed_bytes[2], seed_bytes[3],
        seed_bytes[4], seed_bytes[5], seed_bytes[6], seed_bytes[7],
        seed_bytes[8], seed_bytes[9], seed_bytes[10], seed_bytes[11],
        seed_bytes[12], seed_bytes[13], seed_bytes[14], seed_bytes[15]
    ]);*/

    /*let net = if args.len() > 1 {
        Log::log(&format!("loading network from file {}", args[1]));
        movement::CollectorNeuralNet::from_file(&args[1])
    } else {
        Log::log("using default network");
        movement::CollectorNeuralNet::new()
    };*/

    let mut commander = Commander::new();
    let mut game = GameState::new();

    loop {
        game.update_frame();

        commander.sync(&game);

        commander.process_frame(&mut game);

        game.finalize_frame();
    }
}
