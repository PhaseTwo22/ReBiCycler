use std::fmt;

use crate::{errors::InvalidUnitError, Tag};
use rust_sc2::{geometry::Size, prelude::*};

const PYLON_POWER_DISTANCE: f32 = 7.0;
struct PylonLocation {
    location: Point2,
    built: Option<Tag>,
}
impl PylonLocation {
    fn powers_point(&self, point: Point2) -> bool {
        if self.built.is_some() {
            self.location.is_closer(PYLON_POWER_DISTANCE, point)
        } else {
            false
        }
    }

    pub fn destroy(&mut self) -> bool {
        self.built = None;
        true
    }
}

struct BuildingLocation {
    location: Point2,
    built: Option<Tag>,
    is_powered: bool,
    size: Size,
}
impl BuildingLocation {
    pub fn destroy(&mut self) -> bool {
        self.built = None;
        true
    }
}

pub struct SitingManager {
    nexus: Option<Tag>,
    name: String,
    location: Point2,
    pylon_locations: Vec<PylonLocation>,
    building_locations: Vec<BuildingLocation>,
}
impl fmt::Display for SitingManager {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(
            f,
            "Name: {}, ({},{}), P:{} B:{}",
            self.name,
            self.location.x,
            self.location.y,
            self.pylon_locations.len(),
            self.building_locations.len()
        )
    }
}

impl SitingManager {
    pub const fn new(nexus: Option<Tag>, name: String, location: Point2) -> Self {
        Self {
            nexus,
            location,
            name,
            pylon_locations: Vec::new(),
            building_locations: Vec::new(),
        }
    }

    fn find_powered_spot(&self) -> Point2 {
        Point2::new(0.0, 0.0)
    }

    pub fn add_building(
        &mut self,
        building: Tag,
        location: Point2,
        size: Size,
    ) -> Result<(), InvalidUnitError> {
        let is_powered = self
            .pylon_locations
            .iter()
            .any(|pylon| pylon.powers_point(location));
        match building.type_id {
            UnitTypeId::Nexus => {
                self.nexus = Some(building);
                Ok(())
            }
            UnitTypeId::Pylon => {
                self.pylon_locations.push(PylonLocation {
                    location,
                    built: Some(building),
                });
                Ok(())
            }
            UnitTypeId::Assimilator => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            UnitTypeId::Gateway => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            UnitTypeId::Forge => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            UnitTypeId::FleetBeacon => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            UnitTypeId::TwilightCouncil => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            UnitTypeId::PhotonCannon => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            UnitTypeId::Stargate => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            UnitTypeId::TemplarArchive => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            UnitTypeId::DarkShrine => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            UnitTypeId::RoboticsBay => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            UnitTypeId::RoboticsFacility => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            UnitTypeId::CyberneticsCore => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            UnitTypeId::ShieldBattery => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    is_powered,
                    size,
                });
                Ok(())
            }
            _ => Err(InvalidUnitError("Not a Protoss building!".to_string())),
        }
    }

    pub fn destroy_building_by_tag(&mut self, building: Tag) -> bool {
        match building.type_id {
            UnitTypeId::Pylon => self.find_and_destroy_pylon(building),
            UnitTypeId::Nexus => {
                if self.nexus == Some(building) {
                    self.nexus = None;
                    true
                } else {
                    false
                }
            }
            _ => self.find_and_destroy_building(building),
        }
    }

    fn find_and_destroy_pylon(&mut self, pylon: Tag) -> bool {
        self.pylon_locations
            .iter_mut().find(|l| l.built == Some(pylon.clone()))
            .is_some_and(PylonLocation::destroy)
    }

    fn find_and_destroy_building(&mut self, pylon: Tag) -> bool {
        self.building_locations
            .iter_mut().find(|l| l.built == Some(pylon.clone()))
            .is_some_and(BuildingLocation::destroy)
    }

    fn has_no_structures(&self) -> bool {
        self.pylon_locations.is_empty() && self.nexus.is_none()
    }
}

impl Default for SitingManager {
    fn default() -> Self {
        Self {
            nexus: None,
            location: Point2::new(0.0, 0.0),
            name: "Unnamed".to_string(),
            pylon_locations: Vec::new(),
            building_locations: Vec::new(),
        }
    }
}
