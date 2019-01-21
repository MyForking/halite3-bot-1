use std::collections::HashMap;

use pathfinding::kuhn_munkres::{kuhn_munkres, Weights};

use hlt::command::Command;
use hlt::direction::Direction;
use hlt::log::Log;
use hlt::position::Position;
use hlt::ShipId;

enum Action {
    Move(ShipId, Direction),
    Spawn,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum Actor {
    Ship(ShipId),
    Shipyard,
}

pub struct NavigationSystem {
    map_width: usize,
    map_height: usize,
    positions: Vec<Position>,
    position_indices: HashMap<Position, usize>,
    ships: HashMap<Actor, [(usize, i64); 5]>,
    final_actions: Vec<Action>,
    force_actions: Vec<Action>,
}

impl NavigationSystem {
    pub fn new(map_width: usize, map_height: usize) -> Self {
        NavigationSystem {
            map_width,
            map_height,
            positions: Vec::new(),
            position_indices: HashMap::new(),
            ships: HashMap::new(),
            final_actions: Vec::new(),
            force_actions: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.positions.clear();
        self.position_indices.clear();
        self.ships.clear();
        self.final_actions.clear();
        self.force_actions.clear();
    }

    pub fn force_move(&mut self, id: ShipId, d: Direction) {
        self.force_actions.push(Action::Move(id, d))
    }

    pub fn notify_spawn(&mut self, pos: Position) {
        Log::log(&format!("planned spawn @ {:?}", pos));
        let p0 = self.pos(pos);
        self.ships.insert(
            Actor::Shipyard,
            [
                (p0, i32::min_value() as i64),
                // I hope this works... otherwise I need to find some dummy positions
                (p0, i32::max_value() as i64),
                (p0, i32::max_value() as i64),
                (p0, i32::max_value() as i64),
                (p0, i32::max_value() as i64),
            ],
        );
    }

    pub fn plan_move(
        &mut self,
        id: ShipId,
        pos: Position,
        stay_cost: i32,
        n_cost: i32,
        s_cost: i32,
        e_cost: i32,
        w_cost: i32,
    ) {
        let p0 = self.pos(pos);
        let pn = self.pos(pos.directional_offset(Direction::North));
        let ps = self.pos(pos.directional_offset(Direction::South));
        let pe = self.pos(pos.directional_offset(Direction::East));
        let pw = self.pos(pos.directional_offset(Direction::West));

        /*Log::log(&format!(
            "planned move {:?} @ {:?}: {:?}, {:?}, {:?}, {:?}, {:?}",
            id, p0, stay_cost, n_cost, s_cost, e_cost, w_cost
        ));*/
        //Log::log(&format!("{:?}", self.positions));

        self.ships.insert(
            Actor::Ship(id),
            [
                (p0, stay_cost as i64),
                (pn, n_cost as i64),
                (ps, s_cost as i64),
                (pe, e_cost as i64),
                (pw, w_cost as i64),
            ],
        );
    }

    fn pos(&mut self, p: Position) -> usize {
        let p = self.normalize(p);
        match self.position_indices.get(&p) {
            Some(&n) => n,
            None => {
                let n = self.positions.len();
                self.position_indices.insert(p, n);
                self.positions.push(p);
                n
            }
        }
    }

    pub fn normalize(&self, position: Position) -> Position {
        let width = self.map_width as i32;
        let height = self.map_height as i32;
        let x = ((position.x % width) + width) % width;
        let y = ((position.y % height) + height) % height;
        Position { x, y }
    }

    pub fn solve_moves(&mut self) {
        let mut rows = Vec::new();
        let mut actors = Vec::new();
        for (&actor, &row) in self.ships.iter() {
            rows.push(row);
            actors.push((actor, row));
        }

        let weights = WeightMatrix {
            n_columns: self.positions.len(),
            rows,
        };

        /*Log::log(&format!("{:?}", self.positions));

        for r in 0..weights.rows() {
            let row: Vec<_> = (0..weights.columns()).map(|c| weights.at(r, c)).collect();
            Log::log(&format!("{:?}", row));
        }*/

        let (_total_neg_cost, assignments) = kuhn_munkres(&weights);

        //Log::log(&format!("{:?}", assignments));

        //let mut final_positions = std::collections::HashSet::new();

        self.final_actions = actors
            .iter()
            .zip(&assignments)
            /*.inspect(|((actor, pc), i)| {
                Log::log(&format!(
                    "{:?} {:?} -> {:?}",
                    actor, pc, self.positions[**i]
                ));
            })*/
            .map(|((actor, pc), &i)| match actor {
                &Actor::Ship(id) => Action::Move(
                    id,
                    self.positions[i]
                        .relative_to(
                            self.positions[pc[0].0],
                            self.map_width as i32,
                            self.map_height as i32,
                        )
                        .unwrap(),
                ),
                Actor::Shipyard => Action::Spawn,
            })
            .collect();
    }

    pub fn execute(&self) -> impl Iterator<Item = Command> + '_ {
        self.final_actions
            .iter()
            .chain(&self.force_actions)
            .map(|action| match action {
                &Action::Move(id, d) => Command::move_ship(id, d),
                &Action::Spawn => Command::spawn_ship(),
            })
    }
}

struct WeightMatrix {
    n_columns: usize,
    rows: Vec<[(usize, i64); 5]>,
}

impl Weights<i64> for WeightMatrix {
    fn rows(&self) -> usize {
        self.rows.len()
    }

    fn columns(&self) -> usize {
        self.n_columns
    }

    fn at(&self, row: usize, col: usize) -> i64 {
        self.rows[row]
            .iter()
            .find(|&&(i, _)| i == col)
            .map(|&(_, c)| -c)
            .unwrap_or(i32::min_value() as i64)
    }

    fn neg(&self) -> Self {
        unimplemented!()
    }
}
