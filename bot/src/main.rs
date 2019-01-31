extern crate botlib;

use botlib::hlt;
use botlib::hlt::game::Game;
use botlib::hlt::log::Log;
use botlib::newturn::NewTurn;
use botlib::AiManager;
use botlib::GameState;
use std::env;

fn main() {
    let mut cfg_file = "config.json".to_string();
    let mut runid = String::new();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_ref() {
            "-c" | "--config" => cfg_file = args.next().unwrap(),
            "-r" | "--runid" => runid = args.next().unwrap(),
            _ => panic!("Invalid argument: {}", arg),
        }
    }

    Log::log(&format!("using config file: {}", cfg_file));

    let mut input = hlt::input::Input::new();

    let game = Game::from_input(&mut input);

    let mut ai_mgr = AiManager::new();

    let mut gamestate = GameState::new(&cfg_file, game);

    loop {
        let delta = NewTurn::from_input(&mut input, gamestate.game.players.len(), gamestate.game.constants.max_halite);
        
        gamestate.update_frame(delta);

        ai_mgr.think(&mut gamestate);

        let commands = gamestate.finalize_frame(&runid);

        Game::end_turn(commands);
    }
}
