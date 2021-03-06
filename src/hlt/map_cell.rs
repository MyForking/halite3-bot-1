use hlt::position::Position;
use hlt::DropoffId;
use hlt::PlayerId;

#[derive(Serialize)]
pub struct MapCell {
    #[serde(skip)]
    pub position: Position,
    pub halite: usize,
    pub structure: Structure,
}

#[derive(Eq, PartialEq, Serialize)]
pub enum Structure {
    None,
    Dropoff(DropoffId),
    Shipyard(PlayerId),
}

impl Structure {
    pub fn is_some(&self) -> bool {
        match *self {
            Structure::None => false,
            _ => true,
        }
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }
}
