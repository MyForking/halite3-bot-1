use serde_json;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize)]
pub struct Config {
    pub expansion: Expansion,
    pub navigation: Navigation,
    pub pheromones: Pheromones,
    pub statistics: Statistics,
    pub ships: Ships,
}

#[derive(Deserialize)]
pub struct Expansion {
    pub expansion_distance: usize,
    pub min_halite_density: i32,
    pub ship_radius: usize,
    pub n_ships: usize,
}

#[derive(Deserialize)]
pub struct Navigation {
    pub return_step_cost: usize,
    pub go_home_safety_factor: usize,
}

#[derive(Deserialize)]
pub struct Pheromones {
    pub evaporation_rate: f64,
    pub diffusion_rate: f64,
}

#[derive(Deserialize)]
pub struct Ships {
    pub greedy_prefer_stay_factor: usize,
    pub greedy_harvest_limit: usize,
    pub greedy_seek_limit: usize,
    pub greedy_pheromone_weight: f64,

    pub seek_pheromone_cost: f64,
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
