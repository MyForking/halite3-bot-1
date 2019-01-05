use hlt::command::Command;
use hlt::direction::Direction;
use hlt::entity::Entity;
use hlt::input::Input;
use hlt::position::Position;
use hlt::PlayerId;
use hlt::ShipId;

#[derive(Clone)]
pub struct Ship {
    pub owner: PlayerId,
    pub id: ShipId,
    pub position: Position,
    pub halite: usize,
    max_halite: usize,
    pub command: Option<Command>,
}

impl Ship {
    pub fn is_full(&self) -> bool {
        self.halite >= self.max_halite
    }
    pub fn capacity(&self) -> usize {
        self.max_halite - self.halite
    }

    pub fn make_dropoff(&mut self) {
        self.command = Some(Command::transform_ship_into_dropoff_site(self.id));
    }

    pub fn move_ship(&mut self, direction: Direction) {
        self.command = Some(Command::move_ship(self.id, direction));
    }

    pub fn stay_still(&mut self) {
        self.command = Some(Command::move_ship(self.id, Direction::Still));
    }

    pub fn is_moving(&mut self) -> bool {
        self.command
            .as_ref()
            .map(|cmd| cmd.0.starts_with('m'))
            .unwrap_or(false)
    }

    pub fn generate(input: &mut Input, player_id: PlayerId, max_halite: usize) -> Ship {
        input.read_and_parse_line();
        let id = ShipId(input.next_usize());
        let x = input.next_i32();
        let y = input.next_i32();
        let halite = input.next_usize();

        Ship {
            owner: player_id,
            id,
            position: Position { x, y },
            halite,
            max_halite,
            command: None,
        }
    }
}

impl Entity for Ship {
    fn owner(&self) -> PlayerId {
        self.owner
    }

    fn position(&self) -> Position {
        self.position
    }
}
