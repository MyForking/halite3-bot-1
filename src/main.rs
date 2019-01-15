#[macro_use]
extern crate lazy_static;
//extern crate rand;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

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
use std::env;
use std::io::prelude::*;
//use std::time::SystemTime;
//use std::time::UNIX_EPOCH;

mod behavior_tree;
mod bt_tasks;
mod config;
mod hlt;

/*#[derive(Debug, Eq, PartialEq)]
struct DijkstraMaxNode<C: Ord, T: Eq> {
    cost: C,
    data: T,
}

impl<C: Ord, T: Eq> std::cmp::PartialOrd for DijkstraMaxNode<C, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.cost.partial_cmp(&other.cost)
    }
}

impl<C: Ord, T: Eq> std::cmp::Ord for DijkstraMaxNode<C, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cost.cmp(&other.cost)
    }
}

impl<C: Ord, T: Eq> DijkstraMaxNode<C, T> {
    fn new(cost: C, data: T) -> Self {
        DijkstraMaxNode { cost, data }
    }
}*/

#[derive(Debug, Eq, PartialEq)]
struct DijkstraMinNode<C: Ord, T: Eq> {
    cost: C,
    data: T,
}

impl<C: Ord, T: Eq> std::cmp::PartialOrd for DijkstraMinNode<C, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.cost.partial_cmp(&self.cost)
    }
}

impl<C: Ord, T: Eq> std::cmp::Ord for DijkstraMinNode<C, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost.cmp(&self.cost)
    }
}

impl<C: Ord, T: Eq> DijkstraMinNode<C, T> {
    fn new(cost: C, data: T) -> Self {
        DijkstraMinNode { cost, data }
    }
}

#[derive(Serialize)]
pub struct GameState {
    #[serde(skip)]
    config: config::Config,

    game: Game,

    #[serde(skip)]
    navi: Navi,

    #[serde(skip)]
    command_queue: Vec<Command>,

    collect_statistic: Vec<f64>,
    last_halite: usize,

    pheromones: Vec<Vec<f64>>,
    pheromones_backbuffer: Vec<Vec<f64>>,

    halite_density: Vec<Vec<i32>>,
    return_map_directions: Vec<Vec<Direction>>,
    return_cumultive_costs: Vec<Vec<usize>>,

    halite_percentiles: Vec<usize>,
    avg_return_length: f64,
}

impl GameState {
    fn new(cfg_file: &str) -> Self {
        let game = Game::new();
        let state = GameState {
            config: config::Config::from_file(cfg_file),
            navi: Navi::new(game.map.width, game.map.height),
            command_queue: vec![],
            collect_statistic: Vec::with_capacity(game.constants.max_turns),
            last_halite: 5000,

            pheromones: vec![vec![0.0; game.map.width]; game.map.height],
            pheromones_backbuffer: vec![vec![0.0; game.map.width]; game.map.height],

            halite_density: vec![vec![0; game.map.width]; game.map.height],
            return_map_directions: vec![vec![Direction::Still; game.map.width]; game.map.height],
            return_cumultive_costs: vec![vec![0; game.map.width]; game.map.height],

            halite_percentiles: vec![0; 101],
            avg_return_length: 0.0,

            game,
        };

        Game::ready("MyRustBot");

        state
    }

    fn update_frame(&mut self) {
        self.game.update_frame();
        self.navi.update_frame(&self.game);

        self.compute_halite_density();
        self.compute_return_map();

        self.update_pheromones();

        let mut map_halite: Vec<_> = self.game.map.iter().map(|cell| cell.halite).collect();
        map_halite.sort_unstable();
        let n = map_halite.len() - 1;
        for i in 0..=100 {
            self.halite_percentiles[i] = map_halite[(n * i) / 100];
        }
        Log::log(&format!("Halite quartiles: {:?}", self.halite_percentiles));

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

    fn finalize_frame(&mut self, _runid: &str, dumpfile: Option<&str>) {
        //Log::log(&format!("issuing commands: {:?}", command_queue));

        /*if self.game.turn_number == self.game.constants.max_turns - 5 {
            Log::log("dumping neural net");
            self.collector_net.dump(&format!("netdump{}-{}.txt", runid, self.game.my_id.0));
        }*/

        if let Some(file) = dumpfile {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(file)
                .unwrap();
            file.write_all(serde_json::to_string_pretty(self).unwrap().as_bytes())
                .unwrap();
            file.write_all(b"\n===\n").unwrap();
        }

        Game::end_turn(&self.command_queue);

        if self.game.turn_number == self.game.constants.max_turns {
            Log::log(&format!("collection rate: {:?}", self.collect_statistic));
        }
    }

    fn notify_return(&mut self, turns_taken: usize) {
        const UPDATE_RATE: f64 = 0.9;
        self.avg_return_length =
            self.avg_return_length * UPDATE_RATE + turns_taken as f64 * (1.0 - UPDATE_RATE);
        Log::log(&format!(
            "Average return length: {}",
            self.avg_return_length
        ));
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

    fn get_ship_mut(&mut self, id: ShipId) -> &mut Ship {
        self.game.ships.get_mut(&id).unwrap()
    }

    fn distance_to_nearest_dropoff(&self, id: ShipId) -> usize {
        let pos = self.get_ship(id).position;
        let dist = self
            .game
            .map
            .calculate_distance(&self.me().shipyard.position, &pos);

        self.me()
            .dropoff_ids
            .iter()
            .map(|did| self.game.dropoffs[did].position)
            .map(|p| self.game.map.calculate_distance(&p, &pos))
            .fold(dist, |dist, d| dist.min(d))
    }

    fn ships_in_range<'a>(&'a self, pos: Position, r: usize) -> impl Iterator<Item = ShipId> + 'a {
        self.my_ships().filter(move |&id| {
            self.game
                .map
                .calculate_distance(&pos, &self.get_ship(id).position)
                <= r
        })
    }

    fn try_build_dropoff(&mut self, id: ShipId) -> bool {
        if self.me().halite < self.game.constants.dropoff_cost {
            return false;
        }

        self.get_ship_mut(id).make_dropoff();

        self.avg_return_length = 0.0;

        true
    }

    fn movement_cost(&self, pos: &Position) -> usize {
        self.game.map.at_position(&pos).halite / self.game.constants.move_cost_ratio
    }

    fn can_move(&self, id: ShipId) -> bool {
        let ship = self.get_ship(id);
        ship.halite >= self.movement_cost(&ship.position)
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
        self.get_ship_mut(id).move_ship(d);
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
            self.get_ship_mut(id).move_ship(d);
            true
        } else {
            false
        }
    }

    fn move_ship_or_wait(&mut self, id: ShipId, d: Direction) {
        if !self.try_move_ship(id, d) {
            //self.get_ship_mut(id).move_ship(Direction::Still);
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

    /*fn get_dijkstra_path(&self, start: Position, dest: Position) -> Vec<Direction> {
        const STEP_COST: i64 = 1; // fixed cost of one step - tweak to prefer shorter paths

        let mut visited = HashSet::new();

        let mut queue = BinaryHeap::new();
        queue.push(DijkstraMaxNode::new(0, (start, vec![])));

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
                queue.push(DijkstraMaxNode::new(
                    node.cost as i64 - movement_cost as i64 - STEP_COST,
                    (p, newpath),
                ));
            }
        }
        vec![]
    }*/

    fn get_return_dir(&self, pos: Position) -> Direction {
        self.return_map_directions[pos.y as usize][pos.x as usize]
    }

    fn get_return_dir_alternative(&self, pos: Position) -> Direction {
        let original = self.get_return_dir(pos);
        Direction::get_all_cardinals()
            .iter()
            .filter(|&&d| d != original)
            .map(|&d| (d, self.game.map.normalize(&pos.directional_offset(d))))
            .min_by_key(|(_, p)| self.return_cumultive_costs[p.y as usize][p.x as usize])
            .unwrap()
            .0
    }

    fn get_return_distance(&self, mut pos: Position) -> usize {
        let mut dist = 0;
        loop {
            pos = self.game.map.normalize(&pos);
            match self.return_map_directions[pos.y as usize][pos.x as usize] {
                Direction::Still => return dist,
                d => pos = pos.directional_offset(d),
            }
            dist += 1;
        }
    }

    fn compute_return_map(&mut self) {
        for cc in self
            .return_cumultive_costs
            .iter_mut()
            .flat_map(|row| row.iter_mut())
        {
            *cc = usize::max_value();
        }

        let mut queue = BinaryHeap::new();
        queue.push(DijkstraMinNode::new(
            0,
            (self.me().shipyard.position, Direction::Still),
        ));
        for id in &self.me().dropoff_ids {
            queue.push(DijkstraMinNode::new(
                0,
                (self.game.dropoffs[id].position, Direction::Still),
            ));
        }

        while let Some(node) = queue.pop() {
            let (mut pos, dir) = node.data;
            pos = self.game.map.normalize(&pos);
            let (i, j) = (pos.y as usize, pos.x as usize);

            if node.cost >= self.return_cumultive_costs[i][j] {
                continue;
            }

            self.return_cumultive_costs[i][j] = node.cost;
            self.return_map_directions[i][j] = dir;

            for d in Direction::get_all_cardinals() {
                // make sure we leave an exit open
                if dir == Direction::Still && d == Direction::East {
                    continue;
                }
                let p = pos.directional_offset(d.invert_direction());
                let c =
                    node.cost + self.movement_cost(&p) + self.config.navigation.return_step_cost;
                queue.push(DijkstraMinNode::new(c, (p, d)));
            }
        }
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

    fn push(&mut self, pos: Position) {
        let id = if let Some(id) = self
            .my_ships()
            .find(|&id| self.get_ship(id).position == pos)
        {
            id
        } else {
            return;
        };

        Log::log(&format!("pushing {:?}", pos));

        if Direction::get_all_cardinals()
            .into_iter()
            .any(|d| self.try_move_ship(id, d))
        {
            return;
        }

        self.push(pos.directional_offset(Direction::West));
        self.try_move_ship(id, Direction::West);
    }

    fn update_pheromones(&mut self) {
        let w = self.game.map.width;
        let h = self.game.map.height;

        let ids: Vec<_> = self.my_ships().collect();

        for _ in 0..self.config.pheromones.n_steps {
            for i in 0..h {
                for j in 0..w {
                    let phi0 = self.pheromones[i][j];
                    let mut dphi = (self.pheromones[(i - 1) % h][j] + self.pheromones[(i + 1) % h][j] + self.pheromones[i][(j - 1) % w] + self.pheromones[i][(j + 1) % w] - phi0 * 4.0) * self.config.pheromones.diffusion_coefficient;

                    dphi -= phi0 * self.config.pheromones.decay_rate;

                    dphi += (self.game.map.cells[i][j].halite as f64 - phi0).max(0.0);

                    self.pheromones_backbuffer[i][j] = phi0 + dphi * self.config.pheromones.time_step;
                }
            }

            for id in &ids {
                let (p, cap) = {
                    let ship = self.get_ship(*id);
                    (ship.position, ship.capacity() as f64)
                };
                let phi0 = self.pheromones[p.y as usize][p.x as usize];

                let dphi = (phi0 - cap).min(0.0) * self.config.pheromones.ship_absorbtion;

                self.pheromones_backbuffer[p.y as usize][p.x as usize] += dphi * self.config.pheromones.time_step;
            }

            std::mem::swap(&mut self.pheromones, &mut self.pheromones_backbuffer);
        }
    }

    /*fn add_pheromone(&mut self, pos: Position, amount: f64) {
        let pos = self.game.map.normalize(&pos);
        let (i, j) = (pos.y as usize, pos.x as usize);
        self.pheromones[i][j] += amount;
    }*/

    fn get_pheromone(&self, pos: Position) -> f64 {
        let pos = self.game.map.normalize(&pos);
        let (i, j) = (pos.y as usize, pos.x as usize);
        self.pheromones[i][j]
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

        Log::log(&format!("commanding {} ships", self.ships.len()));

        let syp = state.me().shipyard.position;

        if let Some(id) = self.kamikaze {
            if state.get_ship(id).position == syp {
                *self.ship_ais.get_mut(&id).unwrap() = collector(id);
                self.kamikaze = None;
            }
        }

        let want_dropoff =
            state.avg_return_length >= state.config.expansion.expansion_distance as f64;

        if want_dropoff && state.me().halite >= state.game.constants.dropoff_cost {
            let id = self
                .ships
                .iter()
                .filter(|&&id| {
                    state.distance_to_nearest_dropoff(id)
                        >= state.config.expansion.expansion_distance
                })
                .filter(|&&id| {
                    state
                        .ships_in_range(
                            state.get_ship(id).position,
                            state.config.expansion.ship_radius,
                        )
                        .count()
                        >= state.config.expansion.n_ships
                })
                .map(|&id| {
                    let p = state.get_ship(id).position;
                    (id, state.halite_density[p.y as usize][p.x as usize])
                })
                .filter(|&(_, density)| density >= state.config.expansion.min_halite_density)
                .max_by_key(|&(_, density)| density)
                .map(|(id, _)| id);

            self.builder = id;

            if let Some(id) = id {
                *self.ship_ais.get_mut(&id).unwrap() = build_dropoff(id);
            }
        }

        state.push(syp);

        for id in state.me().dropoff_ids.clone() {
            let pos = state.game.dropoffs[&id].position;
            state.push(pos);
        }

        for (&id, ai) in &mut self.ship_ais {
            if state.get_ship(id).command.is_none() {
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
            let avg_collected = state.collect_statistic
                [state.game.turn_number - state.config.statistics.halite_collection_window..]
                .iter()
                .sum::<f64>()
                / state.config.statistics.halite_collection_window as f64;

            let predicted_profit = avg_collected * state.rounds_left() as f64;

            predicted_profit as usize > state.game.constants.ship_cost * 2 // safety factor...
        } else {
            true
        };

        want_ship &= !want_dropoff
            || state.me().halite
                >= state.game.constants.dropoff_cost + state.game.constants.ship_cost;

        if enemy_blocks && state.me().halite >= state.game.constants.ship_cost
            || (want_ship && state.navi.is_safe(&state.me().shipyard.position))
                && state.me().halite >= state.game.constants.ship_cost
        {
            let cmd = state.me().shipyard.spawn();
            state.command_queue.push(cmd);
        }

        let cmds = state
            .my_ships()
            .filter_map(|id| state.get_ship(id).command.clone())
            .collect::<Vec<_>>();
        state.command_queue.extend(cmds);
    }
}

fn main() {
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

    let mut cfg_file = "config.json".to_string();
    let mut dump_file = None;
    let mut runid = String::new();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_ref() {
            "-c" | "--config" => cfg_file = args.next().unwrap(),
            "-d" | "--dump" => dump_file = args.next(),
            "-r" | "--runid" => runid = args.next().unwrap(),
            _ => panic!("Invalid argument: {}", arg),
        }
    }

    Log::log(&format!("using config file: {}", cfg_file));

    let mut commander = Commander::new();
    let mut game = GameState::new(&cfg_file);

    loop {
        game.update_frame();

        commander.sync(&game);

        commander.process_frame(&mut game);

        game.finalize_frame(&runid, dump_file.as_ref().map(String::as_ref));
    }
}
