#[macro_use]
extern crate lazy_static;
extern crate rand;

use hlt::command::Command;
use hlt::direction::Direction;
use hlt::game::Game;
use hlt::log::Log;
use hlt::navi::Navi;
use hlt::ship::Ship;
use rand::Rng;
//use rand::SeedableRng;
//use rand::XorShiftRng;
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::env;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

mod hlt;

/*struct Path {
    start: Position,
    steps: Vec<Direction>,
}

impl Path {
    fn rect(size: usize, start: Position, first_step: Direction) -> Self {
        let mut steps = vec![];

        steps.extend(iter::repeat(first_step).take(size));

        let step = first_step.turn_right();
        steps.extend(iter::repeat(step).take(size));

        let step = step.turn_right();
        steps.extend(iter::repeat(step).take(size));

        let step = step.turn_right();
        steps.extend(iter::repeat(step).take(size));

        Path {
            start, steps
        }
    }
/*
    fn evaluate(&self, mut capacity: usize, game: &Game) {
        let map = game.map.clone();
        let mut pos = self.start;
        for &d in &self.steps {
            if d == Direction::Still {
                let delta = map.at_position(&pos).halite / game.constants.extract_ratio;
                map.at_position_mut(&pos) -= delta;
                capacity = (capacity + delta).min(game.constants.max_halite);
            } else {
                pos = pos.directional_offset(d);
            }
        }
    }*/
}*/

struct ShipGreedy;

impl ShipGreedy {
    const PREFER_MOVE_FACTOR: usize = 2;

    fn get_move(game: &Game, navi: &mut Navi, ship: &Ship) -> Direction {
        let movement_cost = game.map.at_entity(ship).halite / game.constants.move_cost_ratio;

        if ship.halite < movement_cost {
            return Direction::Still;
        }

        let current_value = game.map.at_entity(ship).halite / game.constants.extract_ratio;

        let mov = Direction::get_all_cardinals()
            .into_iter()
            .map(|d| (d, ship.position.directional_offset(d)))
            .map(|(d, p)| {
                (
                    game.map.at_position(&p).halite / game.constants.extract_ratio,
                    d,
                    p,
                )
            })
            .filter(|&(value, _, _)| {
                value > movement_cost + current_value * ShipGreedy::PREFER_MOVE_FACTOR
            })
            .filter(|(_, _, p)| navi.is_safe(p))
            .max_by_key(|&(value, _, _)| value);

        // hope this prevents cycling between two empty tiles
        if mov.is_none() && current_value == 0 {
            let all: Vec<_> = Direction::get_all_cardinals().into_iter().filter(|&d| navi.is_safe(&ship.position.directional_offset(d))).collect();
            if let Some(&d) = rand::thread_rng().choose(&all) {
                let p = ship.position.directional_offset(d);
                navi.mark_unsafe(&p, ship.id);
                return d;
            }
        }

        let (d, p) = mov
            .map(|(_, d, p)| (d, p))
            .unwrap_or((Direction::Still, ship.position));

        navi.mark_unsafe(&p, ship.id);
        d
    }
}

struct ShipSeeker;

impl ShipSeeker {
    fn get_move(game: &Game, navi: &mut Navi, ship: &Ship) -> Direction {
        let movement_cost = game.map.at_entity(ship).halite / game.constants.move_cost_ratio;

        if ship.halite < movement_cost {
            return Direction::Still;
        }

        let target = game
            .map
            .cells
            .iter()
            .flat_map(|sub| sub.iter())
            .max_by_key(|cell| cell.halite)
            .unwrap();

        let current_value = game.map.at_entity(ship).halite / game.constants.extract_ratio;

        if current_value * 4 >= target.halite * 3 {
            return Direction::Still;
        }

        navi.naive_navigate(ship, &target.position)
    }
}

struct ShipReturnNaive;

impl ShipReturnNaive {
    fn get_move(game: &Game, navi: &mut Navi, ship: &Ship) -> Direction {
        let dest = game.players[game.my_id.0].shipyard.position;
        navi.naive_navigate(ship, &dest)
    }
}

struct ShipReturnDijkstra;

impl ShipReturnDijkstra {
    fn get_move(game: &Game, navi: &mut Navi, ship: &Ship) -> Direction {
        const STEP_COST: i64 = 1;  // fixed cost of one step - tweak to prefer shorter paths

        let dest = game.players[game.my_id.0].shipyard.position;

        let mut visited = HashSet::new();

        let mut queue = BinaryHeap::new();
        queue.push(DijkstraNode::new(0, (ship.position, vec![])));

        let maxlen = ((ship.position.x - dest.x).abs() + (ship.position.y - dest.y).abs()).max(5) * 2; // todo: tweak me

        while let Some(node) = queue.pop() {
            let (pos, path) = node.data;

            if path.len() > maxlen as usize {continue}

            if pos == dest {
                let d = path[0];
                let p = ship.position.directional_offset(d);
                if !navi.is_safe(&p) {
                    return Direction::Still
                } else {
                    navi.mark_unsafe(&p, ship.id);
                    return d
                }
            }

            if visited.contains(&pos){ continue }
            visited.insert(pos);

            let movement_cost = game.map.at_position(&pos).halite / game.constants.move_cost_ratio;

            for d in Direction::get_all_cardinals() {
                let p = pos.directional_offset(d);
                if !navi.is_safe(&p) && p != dest {continue}
                if p.x == dest.x + 1 && p.y == dest.y {continue}  // keep one path open
                let mut newpath = path.clone();
                newpath.push(d);
                queue.push(DijkstraNode::new(node.cost as i64 - movement_cost as i64 - STEP_COST, (p, newpath)));
            }
        }
        Direction::Still
    }
}

#[derive(Eq, PartialEq)]
struct DijkstraNode<C: Ord, T: Eq> {
    cost: C,
    data: T,
}

impl<C: Ord, T: Eq> std::cmp::PartialOrd for DijkstraNode<C, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.cost.partial_cmp(&other.cost)
    }
}

impl<C: Ord, T: Eq> std::cmp::Ord for DijkstraNode<C, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cost.cmp(&other.cost)
    }
}

impl<C: Ord, T: Eq> DijkstraNode<C, T> {
    fn new(cost: C, data: T) -> Self {
        DijkstraNode {
            cost, data
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ShipAI {
    Collect,
    Seek,
    Return,
}

impl ShipAI {
    fn get_move(&self, game: &Game, navi: &mut Navi, ship: &Ship) -> Direction {
        match self {
            ShipAI::Collect => ShipGreedy::get_move(game, navi, ship),
            ShipAI::Seek => ShipSeeker::get_move(game, navi, ship),
            ShipAI::Return => ShipReturnDijkstra::get_move(game, navi, ship),
        }
    }

    fn consider_state(&mut self, game: &Game, ship: &Ship) {
        let first_ship = game
            .players
            .iter()
            .find(|p| p.id == game.my_id)
            .unwrap()
            .ship_ids[0];
        match self {
            ShipAI::Collect | ShipAI::Seek if ship.is_full() => *self = ShipAI::Return,
            ShipAI::Return if ship.halite == 0 && ship.id == first_ship => *self = ShipAI::Seek,
            ShipAI::Return if ship.halite == 0 => *self = ShipAI::Collect,
            _ => {}
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let rng_seed: u64 = if args.len() > 1 {
        args[1].parse().unwrap()
    } else {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    };
    /*let seed_bytes: Vec<u8> = (0..16).map(|x| ((rng_seed >> (x % 8)) & 0xFF) as u8).collect();
    let mut rng: XorShiftRng = SeedableRng::from_seed([
        seed_bytes[0], seed_bytes[1], seed_bytes[2], seed_bytes[3],
        seed_bytes[4], seed_bytes[5], seed_bytes[6], seed_bytes[7],
        seed_bytes[8], seed_bytes[9], seed_bytes[10], seed_bytes[11],
        seed_bytes[12], seed_bytes[13], seed_bytes[14], seed_bytes[15]
    ]);*/

    let mut game = Game::new();
    let mut navi = Navi::new(game.map.width, game.map.height);
    let mut ai = HashMap::new();

    let mut collection = Vec::with_capacity(game.constants.max_turns);

    // At this point "game" variable is populated with initial map data.
    // This is a good place to do computationally expensive start-up pre-processing.
    // As soon as you call "ready" function below, the 2 second per turn timer will start.
    Game::ready("MyRustBot");

    Log::log(&format!(
        "Successfully created bot! My Player ID is {}. Bot rng seed is {}.",
        game.my_id.0, rng_seed
    ));

    Log::log("Constants:");
    Log::log(&format!("ship_cost: {}", game.constants.ship_cost));
    Log::log(&format!("dropoff_cost: {}", game.constants.dropoff_cost));
    Log::log(&format!("max_halite: {}", game.constants.max_halite));
    Log::log(&format!("max_turns: {}", game.constants.max_turns));
    Log::log(&format!("extract_ratio: {}", game.constants.extract_ratio));
    Log::log(&format!(
        "move_cost_ratio: {}",
        game.constants.move_cost_ratio
    ));
    Log::log(&format!(
        "inspiration_enabled: {}",
        game.constants.inspiration_enabled
    ));
    Log::log(&format!(
        "inspiration_radius: {}",
        game.constants.inspiration_radius
    ));
    Log::log(&format!(
        "inspiration_ship_count: {}",
        game.constants.inspiration_ship_count
    ));
    Log::log(&format!(
        "inspired_extract_ratio: {}",
        game.constants.inspired_extract_ratio
    ));
    Log::log(&format!(
        "inspired_bonus_multiplier: {}",
        game.constants.inspired_bonus_multiplier
    ));
    Log::log(&format!(
        "inspired_move_cost_ratio: {}",
        game.constants.inspired_move_cost_ratio
    ));

    let mut last_halite = 5000;

    loop {
        game.update_frame();
        navi.update_frame(&game);

        let me = &game.players[game.my_id.0];
        //let map = &game.map;

        let mut command_queue: Vec<Command> = Vec::new();

        ai.retain(|ship_id, _| me.ship_ids.contains(ship_id));

        Log::log(&format!("# Collect: {}", ai.values().filter(|&&ship_ai| ship_ai == ShipAI::Collect).count()));
        Log::log(&format!("# Seek: {}", ai.values().filter(|&&ship_ai| ship_ai == ShipAI::Seek).count()));
        Log::log(&format!("# Return: {}", ai.values().filter(|&&ship_ai| ship_ai == ShipAI::Return).count()));

        for ship_id in &me.ship_ids {
            let ship_ai = ai.entry(*ship_id).or_insert(ShipAI::Collect);

            let ship = &game.ships[ship_id];

            ship_ai.consider_state(&game, ship);

            Log::log(&format!("ship {:?} AI state: {:?}", ship_id, ship_ai));

            command_queue.push(ship.move_ship(ship_ai.get_move(&game, &mut navi, ship)));
            //Log::log(".");
        }

        if me.halite > last_halite {
            collection.push((me.halite - last_halite) as f64 / me.ship_ids.len() as f64);
        } else {
            collection.push(0.0);
        }

        let want_ship = if game.turn_number > 100 {
            // average halite collected per ship in the last 100 turns
            let avg_collected = collection[game.turn_number - 100..].iter().sum::<f64>() / 100.0;

            let rounds_to_go = game.constants.max_turns - game.turn_number;

            let predicted_profit = avg_collected * rounds_to_go as f64;

            predicted_profit as usize > game.constants.ship_cost * 2 // safety factor...
        } else {
            true
        };

        let enemy_blocks = game
            .ships
            .values()
            .filter(|ship| ship.owner != me.id)
            .any(|ship| ship.position == me.shipyard.position);

        if enemy_blocks && me.halite >= game.constants.ship_cost
            || (want_ship && navi.is_safe(&me.shipyard.position))
                && me.halite >= game.constants.ship_cost * 2
        {
            command_queue.push(me.shipyard.spawn());
        }

        last_halite = me.halite;

        //Log::log(&format!("issuing commands: {:?}", command_queue));

        Game::end_turn(&command_queue);

        if game.turn_number == game.constants.max_turns {
            Log::log(&format!("collection rate: {:?}", collection));
        }
    }
}
