pub mod replay;
pub mod state;

pub use replay::Replay;
pub use state::{unpack_replay, GameState};

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    Utf8Error(std::string::FromUtf8Error),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::JsonError(e)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Error::Utf8Error(e)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn load() {
        use super::replay::Replay;
        use super::state::unpack_replay;
        use std::fs::File;
        use std::io::BufReader;

        let f = File::open("../replays/replay-20190122-230512+0100-1548194710-32-32.hlt").unwrap();
        let f = BufReader::new(f);

        let replay = Replay::from_compressed(f).unwrap();

        for (i, frame) in replay.full_frames.iter().enumerate() {
            println!("Frame {}:", i);
            println!("    events: {:?}", frame.events);
            println!("    energy: {:?}", frame.energy);
            //println!("    entity: {:?}", frame.entities);
            println!("    cells: {:?}", frame.cells);
            //println!("    moves: {:?}", frame.moves);
            println!();
        }

        println!("{:?}", replay.players);

        let game = unpack_replay(&replay);

        println!("{:?}", game[0].ships);
        println!("{:?}", game[1].ships);
        println!("{:?}", game[2].ships);

        println!("{:?}", game[337].map.grid[10][10]);
        println!("{:?}", game[339].map.grid[10][10]);
        println!("{:?}", game.last().unwrap().map.grid[10][10]);

        assert_eq!(
            replay.game_statistics.number_turns + 1,
            replay.full_frames.len()
        );
    }
}
