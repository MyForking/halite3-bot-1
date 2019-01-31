use std::collections::{BinaryHeap, HashMap};

use config;
use hlt::command::Command;
use hlt::direction::Direction;
use hlt::game::Game;
use hlt::log::Log;
use hlt::map_cell::Structure;
use hlt::navi::Navi;
use hlt::player::Player;
use hlt::position::Position;
use hlt::ship::Ship;
use hlt::ShipId;
use newturn::NewTurn;
use super::MovementPredictor;
use super::NavigationSystem;
use utils::NumericCast;

pub struct GameState {
    pub config: config::Config,

    pub game: Game,

    pub navi: Navi,
    pub mp: MovementPredictor,
    pub gns: NavigationSystem,
    pub command_queue: Vec<Command>,

    ship_map: Vec<Vec<Option<ShipId>>>,

    collect_statistic: Vec<f64>,
    last_halite: usize,
    pub total_spent: usize,

    pub pheromones: Vec<Vec<f64>>,
    pheromones_backbuffer: Vec<Vec<f64>>,
    pheromones_temporary_sources: Vec<(Position, f64)>,

    pub halite_density: Vec<Vec<i32>>,
    return_map_directions: Vec<Vec<Direction>>,
    return_cumultive_costs: Vec<Vec<i32>>,

    halite_percentiles: Vec<usize>,
    pub avg_return_length: f64,
}

impl GameState {
    pub fn new(cfg_file: &str, game: Game) -> Self {
        let state = GameState {
            config: config::Config::from_file(cfg_file),
            navi: Navi::new(game.map.width, game.map.height),
            mp: MovementPredictor::new(game.map.width, game.map.height),
            gns: NavigationSystem::new(game.map.width, game.map.height),
            command_queue: vec![],
            ship_map: vec![vec![None; game.map.width]; game.map.height],
            collect_statistic: Vec::with_capacity(game.constants.max_turns),
            last_halite: 5000,
            total_spent: 0,

            pheromones: vec![vec![0.0; game.map.width]; game.map.height],
            pheromones_backbuffer: vec![vec![0.0; game.map.width]; game.map.height],
            pheromones_temporary_sources: vec![],

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

    pub fn update_frame(&mut self, delta: NewTurn) {
        self.game.update_frame(delta);

        self.ship_map = vec![vec![None; self.game.map.width]; self.game.map.height];
        for (&id, pos) in self.game.ships.iter().map(|(id, s)| (id, s.position)) {
            self.ship_map[pos.y as usize][pos.x as usize] = Some(id);
        }

        self.navi.update_frame(&self.game);
        self.mp.update_frame(&self.game);
        self.gns.clear();

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

        /*if self.me().halite > self.last_halite {
            let diff =
                (self.me().halite - self.last_halite) as f64 / self.me().ship_ids.len() as f64;
            self.collect_statistic.push(diff);
        } else {
            self.collect_statistic.push(0.0);
        }*/

        let delta = (self.me().halite + self.total_spent - self.last_halite) as f64;
        let nship = self.me().ship_ids.len() as f64;
        self.collect_statistic.push(delta / nship);

        self.last_halite = self.me().halite + self.total_spent;

        self.command_queue.clear();
    }

    pub fn finalize_frame(&mut self, _runid: &str) -> &[Command]{
        //Log::log(&format!("issuing commands: {:?}", command_queue));

        /*if self.game.turn_number == self.game.constants.max_turns - 5 {
            Log::log("dumping neural net");
            self.collector_net.dump(&format!("netdump{}-{}.txt", runid, self.game.my_id.0));
        }*/

        if self.game.turn_number >= self.game.constants.max_turns - 1 {
            Log::log(&format!("collection rate: {:?}", self.collect_statistic));
        }

        &self.command_queue
    }

    pub fn notify_return(&mut self, turns_taken: usize) {
        const UPDATE_RATE: f64 = 0.9;
        self.avg_return_length =
            self.avg_return_length * UPDATE_RATE + turns_taken as f64 * (1.0 - UPDATE_RATE);
        Log::log(&format!(
            "Average return length: {}",
            self.avg_return_length
        ));
    }

    pub fn rounds_left(&self) -> usize {
        self.game.constants.max_turns - self.game.turn_number
    }

    pub fn me(&self) -> &Player {
        &self.game.players[self.game.my_id.0]
    }

    pub fn my_ships<'a>(&'a self) -> impl Iterator<Item = ShipId> + 'a {
        self.me().ship_ids.iter().cloned()
    }

    pub fn get_ship(&self, id: ShipId) -> &Ship {
        &self.game.ships[&id]
    }

    pub fn get_ship_mut(&mut self, id: ShipId) -> &mut Ship {
        self.game.ships.get_mut(&id).unwrap()
    }

    pub fn get_ship_at(&self, pos: Position) -> Option<&Ship> {
        self.game.ships.values().find(|ship| ship.position == pos)
    }

    pub fn find_nearest_opponent(&self, pos: Position, exclude_pos: bool) -> Option<ShipId> {
        let mut ships: Vec<_> = self
            .game
            .ships
            .values()
            .filter(|ship| ship.owner != self.game.my_id)
            .collect();
        ships.sort_unstable_by_key(|ship| self.game.map.calculate_distance(&pos, &ship.position));

        if exclude_pos {
            ships.iter().find(|ship| ship.position != pos)
        } else {
            ships.first()
        }
            .map(|ship| ship.id)
    }

    pub fn distance_to_nearest_dropoff(&self, id: ShipId) -> usize {
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

    pub fn ships_in_range<'a>(&'a self, pos: Position, r: usize) -> impl Iterator<Item = ShipId> + 'a {
        self.my_ships().filter(move |&id| {
            self.game
                .map
                .calculate_distance(&pos, &self.get_ship(id).position)
                <= r
        })
    }

    pub fn try_build_dropoff(&mut self, id: ShipId) -> bool {
        if self.me().halite < self.game.constants.dropoff_cost {
            return false;
        }

        let cmd = self.get_ship_mut(id).make_dropoff();
        self.command_queue.push(cmd);

        self.total_spent += self.game.constants.dropoff_cost; // assuming the spawn is always successful (it should be...)

        self.avg_return_length = 0.0;

        true
    }

    pub fn movement_cost(&self, pos: &Position) -> i32 {
        (self.game.map.at_position(&pos).halite / self.game.constants.move_cost_ratio).saturate()
    }

    pub fn halite_gain(&self, pos: &Position) -> usize {
        let inspired = self
            .game
            .ships
            .values()
            .filter(|ship| ship.owner != self.me().id)
            .filter(|ship| self.game.map.calculate_distance(pos, &ship.position) <= 4)
            .count()
            >= 2;

        // todo: round up?
        let gain = self.game.map.at_position(&pos).halite / self.game.constants.extract_ratio;

        if inspired {
            gain * 3
        } else {
            gain
        }
    }

    pub fn get_return_dir_costs(&self, pos: Position) -> [i32; 5] {
        let p0 = self.game.map.normalize(&pos);
        let pn = self
            .game
            .map
            .normalize(&pos.directional_offset(Direction::North));
        let ps = self
            .game
            .map
            .normalize(&pos.directional_offset(Direction::South));
        let pe = self
            .game
            .map
            .normalize(&pos.directional_offset(Direction::East));
        let pw = self
            .game
            .map
            .normalize(&pos.directional_offset(Direction::West));
        [
            self.return_cumultive_costs[p0.y as usize][p0.x as usize],
            self.return_cumultive_costs[pn.y as usize][pn.x as usize],
            self.return_cumultive_costs[ps.y as usize][ps.x as usize],
            self.return_cumultive_costs[pe.y as usize][pe.x as usize],
            self.return_cumultive_costs[pw.y as usize][pw.x as usize],
        ]
    }

    pub fn get_return_distance(&self, mut pos: Position) -> usize {
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

    pub fn get_dijkstra_move(&mut self, pos: Position, dest: Position) -> [i32; 5] {
        let mut costs = [i32::max_value(); 5];

        let moves: Vec<_> = Direction::get_all_options()
            .into_iter()
            .map(|d| self.game.map.normalize(&pos.directional_offset(d)))
            .collect();
        let mut visited = [false; 5];

        let mut queue = BinaryHeap::new();
        queue.push(DijkstraMinNode::new(0, self.game.map.normalize(&dest)));

        let mut cumulative_costs = HashMap::new();

        while let Some(node) = queue.pop() {
            let p = self.game.map.normalize(&node.data);

            match cumulative_costs.get(&p) {
                Some(&c) if node.cost >= c => continue,
                _ => {}
            }

            for ((q, v), c) in moves.iter().zip(&mut visited).zip(&mut costs) {
                if p == *q {
                    *c = node.cost;
                    *v = true;
                }
            }

            if visited.iter().all(|&v| v) {
                break;
            }

            cumulative_costs.insert(p, node.cost);

            for d in Direction::get_all_cardinals() {
                let q = p.directional_offset(d);
                if let Structure::Shipyard(pid) = self.game.map.at_position(&q).structure {
                    // don't trigger opponent's anti griefing mechanic
                    if pid != self.game.my_id {
                        continue;
                    }
                }
                let c = node.cost + self.movement_cost(&q) as i32 + 1;
                queue.push(DijkstraMinNode::new(c, q));
            }
        }

        return costs;
    }

    fn compute_return_map(&mut self) {
        for cc in self
            .return_cumultive_costs
            .iter_mut()
            .flat_map(|row| row.iter_mut())
            {
                *cc = i32::max_value();
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
                if let Structure::Shipyard(pid) = self.game.map.at_position(&p).structure {
                    // don't trigger opponent's anti griefing mechanic
                    if pid != self.game.my_id {
                        continue;
                    }
                }
                let c = node
                    .cost
                    .saturating_add(self.movement_cost(&p))
                    .saturating_add(self.config.navigation.return_step_cost);
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

    fn update_pheromones(&mut self) {
        let w = self.game.map.width;
        let h = self.game.map.height;

        for _ in 0..self.config.pheromones.n_steps {
            for i in 0..h {
                for j in 0..w {
                    let phi0 = self.pheromones[i][j];
                    let mut dphi = (self.pheromones[(i - 1) % h][j]
                        + self.pheromones[(i + 1) % h][j]
                        + self.pheromones[i][(j - 1) % w]
                        + self.pheromones[i][(j + 1) % w]
                        - phi0 * 4.0)
                        * self.config.pheromones.diffusion_coefficient;

                    dphi -= phi0 * self.config.pheromones.decay_rate;

                    dphi += (self.game.map.cells[i][j].halite as f64 - phi0).max(0.0);

                    self.pheromones_backbuffer[i][j] =
                        phi0 + dphi * self.config.pheromones.time_step;
                }
            }

            for ship in self.game.ships.values() {
                let (p, cargo, cap) =
                    { (ship.position, ship.halite as f64, ship.capacity() as f64) };

                let phi0 = self.pheromones[p.y as usize][p.x as usize];

                let dphi = if ship.owner == self.game.my_id {
                    (phi0 - cap).min(0.0) * self.config.pheromones.ship_absorbtion
                } else {
                    (cargo - phi0).max(0.0) * 0.1
                };

                self.pheromones_backbuffer[p.y as usize][p.x as usize] +=
                    dphi * self.config.pheromones.time_step;
            }

            for (p, dphi) in &self.pheromones_temporary_sources {
                self.pheromones_backbuffer[p.y as usize][p.x as usize] +=
                    dphi * self.config.pheromones.time_step;
            }

            self.pheromones_temporary_sources.clear();

            std::mem::swap(&mut self.pheromones, &mut self.pheromones_backbuffer);
        }
    }

    pub fn add_pheromone(&mut self, pos: Position, rate: f64) {
        let pos = self.game.map.normalize(&pos);
        self.pheromones_temporary_sources.push((pos, rate));
    }

    pub fn get_pheromone(&self, pos: Position) -> f64 {
        let pos = self.game.map.normalize(&pos);
        let (i, j) = (pos.y as usize, pos.x as usize);
        self.pheromones[i][j]
    }
}

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
