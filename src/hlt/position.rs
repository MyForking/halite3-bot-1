use hlt::direction::Direction;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn directional_offset(&self, d: Direction) -> Position {
        let (dx, dy) = match d {
            Direction::North => (0, -1),
            Direction::South => (0, 1),
            Direction::East => (1, 0),
            Direction::West => (-1, 0),
            Direction::Still => (0, 0),
        };

        Position {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    pub fn get_surrounding_cardinals(&self) -> Vec<Position> {
        vec![
            self.directional_offset(Direction::North),
            self.directional_offset(Direction::South),
            self.directional_offset(Direction::East),
            self.directional_offset(Direction::West),
        ]
    }

    pub fn get_all_neighbors(&self) -> Vec<Position> {
        vec![
            self.directional_offset(Direction::West),
            self.directional_offset(Direction::East),
            self.directional_offset(Direction::North),
            self.directional_offset(Direction::South),
            *self
        ]
    }

    pub fn relative_to(&self, other: Position, width: i32, height: i32) -> Option<Direction> {
        match (self.x - other.x, self.y - other.y) {
            (0, -1) => Some(Direction::North),
            (0, 1) => Some(Direction::South),
            (-1, 0) => Some(Direction::West),
            (1, 0) => Some(Direction::East),
            (0, 0) => Some(Direction::Still),
            (0, dy) if dy == height - 1 => Some(Direction::North),
            (0, dy) if dy == 1 - height => Some(Direction::South),
            (dx, 0) if dx == width - 1 => Some(Direction::West),
            (dx, 0) if dx == 1 - width => Some(Direction::East),
            _ => panic!("relative_to({:?}, {:?})", self, other),
        }
    }
}
