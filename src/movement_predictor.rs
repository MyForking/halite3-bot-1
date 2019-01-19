use std::collections::HashMap;
use std::mem;
use hlt::direction::Direction;
use hlt::game::Game;
use hlt::ShipId;
use hlt::map_cell::Structure;
use hlt::position::Position;

struct ShipInfo {
    pos: Position,
    last_pos: Position,
    cargo: usize,
}

pub struct MovementPredictor {
    ships: HashMap<ShipId, ShipInfo>,
    probs: Vec<Vec<f64>>,
}

impl MovementPredictor {
    pub fn new(w: usize, h: usize) -> Self {
        MovementPredictor {
            ships: HashMap::new(),
            probs: vec![vec![0.0; w]; h],
        }
    }

    pub fn update_frame(&mut self, game: &Game) {
        for row in &mut self.probs {
            for p in row {
                *p = 0.0;
            }
        }

        let prev_ships = mem::replace(&mut self.ships, HashMap::with_capacity(game.ships.len()));

        for (&id, ship) in &game.ships {
            if ship.owner == game.my_id { continue }
            let pos = ship.position;
            let cargo = ship.halite;
            let last_pos = prev_ships.get(&id).map(|si| si.pos).unwrap_or(pos);
            self.ships.insert(id, ShipInfo {pos, last_pos, cargo});

            self.probs[pos.y as usize][pos.x as usize] = match game.map.at_position(&pos).structure {
                // simply ignore enemy ships at my own structures
                Structure::Dropoff(did) if game.dropoffs[&did].owner == game.my_id => 0.0,
                Structure::Shipyard(pid) if pid == game.my_id => 0.0,
                _ => 1.0,
            };
        }
    }

    pub fn is_occupied(&self, pos: Position) -> bool{
        let pos = self.normalize(pos);
        return self.probs[pos.y as usize][pos.x as usize] > 0.99;
    }

    pub fn normalize(&self, position: Position) -> Position {
        let width = self.probs[0].len() as i32;
        let height = self.probs.len() as i32;
        let x = ((position.x % width) + width) % width;
        let y = ((position.y % height) + height) % height;
        Position { x, y }
    }
}
