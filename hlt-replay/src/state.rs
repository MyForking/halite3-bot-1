use std::collections::HashMap;

use crate::replay::{Frame, Map, Player, PlayerId, Position, Replay, ShipId};

/// Convert a [`Replay`](../replay/struct.Replay.html) into sequence of [`GameState`](struct.GameState.html)s.
pub fn unpack_replay(replay: &Replay) -> Vec<GameState> {
    let mut states = Vec::with_capacity(replay.game_statistics.number_turns);
    let mut state = GameState::from_replay(replay);

    states.push(state.clone());
    for _ in 0..replay.game_statistics.number_turns {
        state.advance();
        states.push(state.clone());
    }

    states
}

/// State of the game after a turn
#[derive(Debug, Clone)]
pub struct GameState<'a> {
    pub replay: &'a Replay,

    pub map: Map,
    pub players: Vec<Player>,
    pub ships: HashMap<ShipId, Ship>,

    pub turn_nr: usize,
}

impl<'a> GameState<'a> {
    pub fn from_replay(replay: &'a Replay) -> Self {
        GameState {
            //game_statistics: replay.game_statistics.clone(),
            map: replay.production_map.clone(),
            players: replay.players.clone(),
            ships: HashMap::new(),
            turn_nr: 0,
            replay,
        }
    }

    pub fn frame(&self) -> &Frame {
        assert!(self.turn_nr <= self.replay.game_statistics.number_turns);
        &self.replay.full_frames[self.turn_nr]
    }

    pub fn advance(&mut self) {
        assert!(self.turn_nr < self.replay.game_statistics.number_turns);

        let old_frame = &self.replay.full_frames[self.turn_nr];
        self.turn_nr += 1;
        let new_frame = &self.replay.full_frames[self.turn_nr];

        for cell in &old_frame.cells {
            self.map.set(cell.x, cell.y, cell.production);
        }

        for (pid, &energy) in &old_frame.energy {
            self.players[pid.0].energy = energy;
            self.players[pid.0].entities.clear();
        }

        for ship in new_frame.entities.iter().flat_map(|(&pid, entities)| {
            entities.iter().map(move |(&sid, ent)| Ship {
                owner: pid,
                id: sid,
                position: Position { x: ent.x, y: ent.y },
                halite: ent.energy,
                is_inspired: ent.is_inspired,
            })
        }) {
            self.players[ship.owner.0].entities.push(ship.id);
            self.ships.insert(ship.id, ship);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ship {
    pub owner: PlayerId,
    pub id: ShipId,
    pub position: Position,
    pub halite: usize,
    pub is_inspired: bool,
}
