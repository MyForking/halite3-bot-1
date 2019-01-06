#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub enum Direction {
    North,
    East,
    South,
    West,
    Still,
}

impl Direction {
    pub fn invert_direction(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
            Direction::Still => Direction::Still,
        }
    }

    pub fn turn_right(&self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
            Direction::Still => Direction::Still,
        }
    }

    pub fn get_all_cardinals() -> Vec<Direction> {
        vec![
            Direction::West,
            Direction::East,
            Direction::North,
            Direction::South,
        ]
    }

    pub fn get_all_options() -> Vec<Direction> {
        vec![
            Direction::West,
            Direction::East,
            Direction::North,
            Direction::South,
            Direction::Still,
        ]
    }

    pub fn get_char_encoding(&self) -> char {
        match self {
            Direction::North => 'n',
            Direction::East => 'e',
            Direction::South => 's',
            Direction::West => 'w',
            Direction::Still => 'o',
        }
    }
}
