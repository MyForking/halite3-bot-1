use serde_json;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize)]
pub struct Config {
    pub strategy: Strategy,
    pub expansion: Expansion,
    pub navigation: Navigation,
    pub pheromones: Pheromones,
    pub statistics: Statistics,
    pub ships: Ships,
}

#[derive(Deserialize)]
pub struct Strategy {
    pub spawn_halite_floor: usize,
    pub spawn_min_rounds_left_factor: usize,
}

#[derive(Deserialize)]
pub struct Expansion {
    pub expansion_distance: usize,
    pub return_distance: usize,
    pub min_halite_density: i32,
    pub ship_radius: usize,
    pub n_ships: usize,
}

#[derive(Deserialize)]
pub struct Navigation {
    pub return_step_cost: i32,
    pub go_home_safety_factor: usize,
}

#[derive(Deserialize)]
pub struct Pheromones {
    pub evaporation_rate: f64,
    pub diffusion_coefficient: f64,
    pub decay_rate: f64,
    pub ship_absorbtion: f64,
    pub ship_evaporation: f64,
    pub time_step: f64,
    pub n_steps: usize,
}

#[derive(Deserialize)]
pub struct Ships {
    pub greedy_prefer_stay_factor: usize,
    pub greedy_harvest_limit: usize,
    pub greedy_seek_limit: usize,
    pub greedy_pheromone_weight: f64,

    pub greedy_move_cost_factor: f64,
    pub seek_greed_factor: f64,
    pub seek_return_cost_factor: f64,
    pub seek_pheromone_factor: f64,

    pub carefulness_limit: i32,
}

#[derive(Deserialize)]
pub struct Statistics {
    pub halite_collection_window: usize,
}

impl Config {
    pub fn from_file(file: &str) -> Self {
        let f = File::open(file).expect(&format!("Error loading file {:?}", file));
        serde_json::from_reader(BufReader::new(f)).expect("Deserialization Error")
    }
}
