use std::collections::HashMap;
use std::io::Read;

use serde_derive::Deserialize;

use crate::Error;

/// Raw replay as stored in .hlt files
#[derive(Debug, Clone, Deserialize)]
pub struct Replay {
    #[serde(rename = "ENGINE_VERSION")]
    pub engine_version: String,

    #[serde(rename = "REPLAY_FILE_VERSION")]
    pub replay_file_version: i32,

    #[serde(rename = "GAME_CONSTANTS")]
    pub game_constants: GameConstants,

    pub full_frames: Vec<Frame>,
    pub game_statistics: GameStatistics,
    pub map_generator_seed: usize,
    pub number_of_players: usize,
    pub players: Vec<Player>,
    pub production_map: Map,
}

impl Replay {
    /// deserialize JSON from string
    pub fn from_str(data: &str) -> Result<Self, Error> {
        let replay = serde_json::from_str(&data)?;
        Ok(replay)
    }

    /// deserialize JSON from reader
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, Error> {
        let replay = serde_json::from_reader(reader)?;
        Ok(replay)
    }

    /// deserialize compressed JSON (such as found in `.hlt` replay files) from reader
    pub fn from_compressed<R: Read>(reader: R) -> Result<Self, Error> {
        let data = zstd::stream::decode_all(reader).map(String::from_utf8)??;
        Self::from_str(&data)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameConstants {
    #[serde(rename = "MAX_ENERGY")]
    pub max_halite: usize,

    #[serde(rename = "NEW_ENTITY_ENERGY_COST")]
    pub ship_cost: usize,

    #[serde(rename = "DROPOFF_COST")]
    pub dropoff_cost: usize,

    #[serde(rename = "MAX_TURNS")]
    pub max_turns: usize,

    #[serde(rename = "EXTRACT_RATIO")]
    pub extract_ratio: usize,

    #[serde(rename = "MOVE_COST_RATIO")]
    pub move_cost_ratio: usize,

    #[serde(rename = "INSPIRATION_ENABLED")]
    pub inspiration_enabled: bool,

    #[serde(rename = "INSPIRATION_RADIUS")]
    pub inspiration_radius: usize,

    #[serde(rename = "INSPIRATION_SHIP_COUNT")]
    pub inspiration_ship_count: usize,

    #[serde(rename = "INSPIRED_EXTRACT_RATIO")]
    pub inspired_extract_ratio: usize,

    #[serde(rename = "INSPIRED_BONUS_MULTIPLIER")]
    pub inspired_bonus_multiplier: f64,

    #[serde(rename = "INSPIRED_MOVE_COST_RATIO")]
    pub inspired_move_cost_ratio: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameStatistics {
    pub execution_time: usize,
    pub map_total_halite: usize,
    pub number_turns: usize,
    pub player_statistics: Vec<PlayerStatistics>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerStatistics {
    pub all_collisions: usize,
    pub average_entity_distance: usize,
    pub carried_at_end: usize,
    pub dropoff_collisions: usize,
    pub final_production: usize,
    pub halite_per_dropoff: Vec<(Position, usize)>,
    pub interaction_opportunities: usize,
    pub last_turn_alive: usize,
    pub last_turn_ship_spawn: usize,
    pub max_entity_distance: usize,
    pub mining_efficiency: f64,
    pub number_dropoffs: usize,
    pub player_id: PlayerId,
    pub random_id: usize,
    pub rank: usize,
    pub self_collisions: usize,
    pub ships_peak: usize,
    pub ships_spawned: usize,
    pub total_bonus: usize,
    pub total_dropped: usize,
    pub total_mined: usize,
    pub total_production: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Player {
    pub energy: usize,
    pub entities: Vec<ShipId>,
    pub factory_location: Position,
    pub name: String,
    pub player_id: PlayerId,

    #[serde(skip)]
    pub dropoffs: Vec<Position>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Map {
    pub grid: Vec<Vec<MapCell>>,
    pub width: usize,
    pub height: usize,
}

impl Map {
    pub fn set(&mut self, x: usize, y: usize, energy: usize) {
        self.grid[y][x].energy = energy;
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct MapCell {
    pub energy: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Frame {
    pub cells: Vec<FrameCell>,
    pub deposited: HashMap<PlayerId, usize>,
    pub energy: HashMap<PlayerId, usize>,
    pub entities: HashMap<PlayerId, HashMap<ShipId, Entity>>,
    pub events: Vec<Event>,
    pub moves: HashMap<PlayerId, Vec<Move>>,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct FrameCell {
    pub production: usize,
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct Entity {
    pub energy: usize,
    pub is_inspired: bool,
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    #[serde(rename = "spawn")]
    Spawn {
        energy: usize,
        id: ShipId,
        location: Position,
        owner_id: PlayerId,
    },

    #[serde(rename = "construct")]
    Construct {
        id: ShipId,
        location: Position,
        owner_id: PlayerId,
    },

    #[serde(rename = "shipwreck")]
    Shipwreck {
        location: Position,
        ships: Vec<ShipId>,
    },
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Move {
    #[serde(rename = "g")]
    Spawn,

    #[serde(rename = "c")]
    Construct { id: ShipId },

    #[serde(rename = "m")]
    Move { direction: Direction, id: ShipId },
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub enum Direction {
    #[serde(rename = "n")]
    North,

    #[serde(rename = "s")]
    South,

    #[serde(rename = "e")]
    East,

    #[serde(rename = "w")]
    West,

    #[serde(rename = "o")]
    Still,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct PlayerId(pub usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct ShipId(pub usize);
