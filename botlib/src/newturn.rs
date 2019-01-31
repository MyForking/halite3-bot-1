use hlt::input::Input;
use hlt::ship::Ship;
use hlt::dropoff::Dropoff;
use hlt::PlayerId;

/// Abstraction over turn updates
pub struct NewTurn {
    pub turn_number: usize,
    pub players: Vec<Player>,
    pub map_updates: Vec<MapUpdate>,
}

impl NewTurn {
    pub fn from_input(input: &mut Input, n_players: usize, max_halite: usize) -> Self {
        input.read_and_parse_line();

        let turn_number = input.next_usize();

        let mut players = vec![];

        for _ in 0..n_players {
            input.read_and_parse_line();
            let player_id = PlayerId(input.next_usize());
            let num_ships = input.next_usize();
            let num_dropoffs = input.next_usize();
            let halite = input.next_usize();

            let ships: Vec<_> = (0..num_ships).map(|_| Ship::generate(input, player_id, max_halite)).collect();
            let dropoffs: Vec<_> = (0..num_dropoffs).map(|_| Dropoff::generate(input, player_id)).collect();

            players.push(Player {
                player_id,
                halite,
                ships,
                dropoffs
            })
        }

        input.read_and_parse_line();
        let update_count = input.next_usize();

        let map_updates: Vec<_> = (0..update_count).map(|_| {
            input.read_and_parse_line();
            MapUpdate {
                x: input.next_usize(),
                y: input.next_usize(),
                halite: input.next_usize(),
            }
        }).collect();

        NewTurn {
            turn_number,
            players,
            map_updates,
        }
    }
}

pub struct Player {
    pub player_id: PlayerId,
    pub halite: usize,
    pub ships: Vec<Ship>,
    pub dropoffs: Vec<Dropoff>,
}

pub struct MapUpdate {
    pub x: usize,
    pub y: usize,
    pub halite: usize,
}
