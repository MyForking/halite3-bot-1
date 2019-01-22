use hlt::game::Game;
use hlt::map_cell::Structure;
use hlt::position::Position;
//use hlt::ShipId;
//use std::collections::HashMap;
//use std::mem;

/*struct ShipInfo {
    pos: Position,
    last_pos: Position,
    cargo: usize,
}*/

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Threat {
    Clear,
    Reachable,
    Occupied,
}

impl Threat {
    fn set(&mut self, other: Self) {
        use self::Threat::*;
        match (*self, other) {
            (Clear, o) => *self = o,
            (Reachable, Occupied) => *self = Occupied,
            _ => {}
        }
    }
}

pub struct MovementPredictor {
    //ships: HashMap<ShipId, ShipInfo>,
    threat_level: Vec<Vec<Threat>>,
}

impl MovementPredictor {
    pub fn new(w: usize, h: usize) -> Self {
        MovementPredictor {
            //ships: HashMap::new(),
            threat_level: vec![vec![Threat::Clear; w]; h],
        }
    }

    pub fn update_frame(&mut self, game: &Game) {
        for row in &mut self.threat_level {
            for tl in row {
                *tl = Threat::Clear;
            }
        }

        //let prev_ships = mem::replace(&mut self.ships, HashMap::with_capacity(game.ships.len()));

        for (&_id, ship) in &game.ships {
            if ship.owner == game.my_id {
                continue;
            }
            let pos = ship.position;
            /*let cargo = ship.halite;
            let last_pos = prev_ships.get(&id).map(|si| si.pos).unwrap_or(pos);
            self.ships.insert(
                id,
                ShipInfo {
                    pos,
                    last_pos,
                    cargo,
                },
            );*/

            for p in pos.get_surrounding_cardinals().into_iter()
                .chain(std::iter::once(pos))
                .map(|p| self.normalize(p))
                .collect::<Vec<_>>()
                {
                match game.map.at_position(&pos).structure
                {
                    // simply ignore enemy ships at my own structures
                    Structure::Dropoff(did) if game.dropoffs[&did].owner == game.my_id => continue,
                    Structure::Shipyard(pid) if pid == game.my_id => continue,
                    _ => {},
                };

                if p == pos {
                    self.threat_level[p.y as usize][p.x as usize] = Threat::Occupied;
                } else {
                    self.threat_level[p.y as usize][p.x as usize].set(Threat::Reachable);
                }
            }
        }
    }

    pub fn is_occupied(&self, pos: Position) -> bool {
        let pos = self.normalize(pos);
        self.threat_level[pos.y as usize][pos.x as usize] == Threat::Occupied
    }

    pub fn is_reachable(&self, pos: Position) -> bool {
        let pos = self.normalize(pos);
        self.threat_level[pos.y as usize][pos.x as usize] != Threat::Clear
    }

    pub fn normalize(&self, position: Position) -> Position {
        let width = self.threat_level[0].len() as i32;
        let height = self.threat_level.len() as i32;
        let x = ((position.x % width) + width) % width;
        let y = ((position.y % height) + height) % height;
        Position { x, y }
    }
}
