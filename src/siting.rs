use rust_sc2::prelude::*;

struct PylonLocation {
    loc: Point2,
    is_alive: bool,
}

struct SitingManager {
    pylon_locations: Vec<PylonLocation>,
}

impl SitingManager {
    fn new() -> Self {
        SitingManager {
            pylon_locations: Vec::new(),
        }
    }

    fn find_powered_spot(&self) -> Point2 {
        Point2::new(0.0, 0.0)
    }
}
