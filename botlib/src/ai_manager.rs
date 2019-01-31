use commander::Commander;
use hlt::log::Log;
use hlt::ShipId;
use ship_ai::ShipAi;
use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use GameState;

pub struct AiManager {
    commander: RefCell<Commander>,
    pub ships: HashMap<ShipId, RefCell<ShipAi>>,

    new_ships: HashSet<ShipId>,
    lost_ships: HashSet<ShipId>,
    prev_ships: HashSet<ShipId>,
}

impl AiManager {
    pub fn new() -> Self {
        AiManager {
            commander: RefCell::new(Commander::new()),
            ships: HashMap::new(),
            new_ships: HashSet::new(),
            lost_ships: HashSet::new(),
            prev_ships: HashSet::new(),
        }
    }

    pub fn think(&mut self, world: &mut GameState) {
        let state_ships: HashSet<_> = world.my_ships().collect();
        self.new_ships.extend(&state_ships - &self.prev_ships);
        self.lost_ships.extend(&self.prev_ships - &state_ships);
        self.prev_ships = &self.prev_ships & &state_ships;

        for id in self.lost_ships.drain() {
            self.ships.remove(&id);
        }

        for id in self.new_ships.drain() {
            self.prev_ships.insert(id);
            self.ships.insert(id, RefCell::new(ShipAi::new(id)));
        }

        Log::log(&format!("commanding {} ships", self.ships.len()));

        self.commander.borrow_mut().think(self, world);

        for ship in self.ships.values() {
            ship.borrow_mut().think(self, world)
        }

        world.gns.solve_moves();
        world.command_queue.extend(world.gns.execute());
    }

    pub fn commander(&self) -> RefMut<Commander> {
        self.commander.borrow_mut()
    }

    pub fn ship(&self, id: ShipId) -> RefMut<ShipAi> {
        self.ships[&id].borrow_mut()
    }
}
