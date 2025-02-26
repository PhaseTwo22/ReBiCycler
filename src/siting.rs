use std::{
    collections::HashMap,
    fmt::{self, Debug},
};

use crate::{errors::InvalidUnitError, protoss_bot::ReBiCycler, Tag};
use rust_sc2::{
    action::ActionResult,
    bot::{Bot, Expansion},
    prelude::*,
};

const PYLON_POWER_DISTANCE: f32 = 6.5;
const EXPANSION_NAMES: [&str; 48] = [
    "Α", "Β", "Γ", "Δ", "Ε", "Ζ", "Η", "Θ", "Ι", "Κ", "Λ", "Μ", "Ν", "Ξ", "Ο", "Π", "Ρ", "Σ", "Τ",
    "Υ", "Φ", "Χ", "Ψ", "Ω", "Α\'", "Β\'", "Γ\'", "Δ\'", "Ε\'", "Ζ\'", "Η\'", "Θ\'", "Ι\'", "Κ\'",
    "Λ\'", "Μ\'", "Ν\'", "Ξ\'", "Ο\'", "Π\'", "Ρ\'", "Σ\'", "Τ\'", "Υ\'", "Φ\'", "Χ\'", "Ψ\'",
    "Ω\'",
];

const PYLON_DISTANCE_FROM_NEXUS: f32 = 9.0;
pub struct PylonLocation {
    location: Point2,
    built: Option<Tag>,
}
impl PylonLocation {
    pub fn destroy(&mut self) -> bool {
        self.built = None;
        true
    }

    pub fn build(&mut self, building: Tag) {
        self.built = Some(building);
    }

    pub fn points_in_range(&self) -> Vec<Point2> {
        let mut out: Vec<Point2> = Vec::new();
        for x in -7i8..7 {
            for y in -7i8..7 {
                if (x, y) == (0, 0) {
                    continue;
                }
                let offset = Point2::new(f32::from(x), f32::from(y));
                let center = self.location + offset;
                out.push(center);
            }
        }
        out
    }
}

pub struct BuildingLocation {
    pub location: Point2,
    built: Option<Tag>,
    radius: f32,
}
impl BuildingLocation {
    pub fn destroy(&mut self) -> bool {
        self.built = None;
        true
    }
    pub fn build(&mut self, building: Tag) {
        self.built = Some(building);
    }
}
#[derive(Default)]
pub struct SitingDirector {
    expansion_sites: HashMap<Point2, SitingManager>,
}
impl Debug for SitingDirector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sites: Vec<String> = self
            .expansion_sites
            .values()
            .map(|v| v.to_string())
            .collect();
        write!(f, "{:?}", sites.join("\n"))
    }
}

impl SitingDirector {
    pub fn new() -> Self {
        Self {
            expansion_sites: HashMap::new(),
        }
    }

    pub fn initialize_global_placement(&mut self, expansions: Vec<Expansion>, map_center: Point2) {
        self.expansion_sites = expansions
            .iter()
            .enumerate()
            .map(|(i, e)| Self::generate_building_sites(i, e, map_center))
            .collect();
    }

    fn generate_building_sites(
        index: usize,
        expansion: &Expansion,
        map_center: Point2,
    ) -> (Point2, SitingManager) {
        let base_tag = crate::base_manager::BaseManager::base_tag(expansion);
        let expansion_point = expansion.loc;
        let mut site_manager =
            SitingManager::new(base_tag, EXPANSION_NAMES[index].to_string(), expansion.loc);

        site_manager.build_expansion_template(expansion.loc, expansion.center, map_center);

        (expansion_point, site_manager)
    }

    /// Finds an available building site from our Siting Directors, nearest to the main.
    pub fn get_building_site_choices(
        &self,
        bot: &Bot,
        footprint_radius: &f32,
        structure_id: &UnitTypeId,
        ability_id: &AbilityId,
        main_location: Point2,
    ) -> impl Iterator<Item = &BuildingLocation> {
        let siting_managers_by_distance =
            self.expansion_sites.keys().sort_by_distance(main_location);

        siting_managers_by_distance
            .map(|sm_point| self.expansion_sites.get(sm_point))
            .filter_map(|sm| sm?.check_valid_sites(bot, footprint_radius, structure_id, ability_id))
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

    fn build_expansion_template(
        &mut self,
        base_location: Point2,
        mineral_center: Point2,
        map_center: Point2,
    ) {
        let distance_to_minerals = 6.0;
        let pylon_behind_location =
            { base_location.towards(mineral_center, distance_to_minerals + 3.0) };

        let direction_to_map_center = base_location.towards(map_center, 8.0);

        let pylon_spots =
            base_location.circle_intersection(direction_to_map_center, PYLON_DISTANCE_FROM_NEXUS);

        let places = if let Some(spots) = pylon_spots {
            [Some(pylon_behind_location), Some(spots[0]), Some(spots[1])]
        } else {
            [Some(pylon_behind_location), None, None]
        };

        places
            .iter()
            .flatten()
            .map(|p| self.add_pylon_site(*p))
            .collect()
    }

    pub fn get_pylon_site(&self) -> Option<&PylonLocation> {
        self.pylon_locations.first()
    }

    pub fn get_random_powered_points(&self) -> Vec<Point2> {
        self.pylon_locations.first().unwrap().points_in_range()
    }

    pub fn generic_build_location_pattern(&mut self, pylon: &PylonLocation) {
        let _: () = [
            pylon.location.offset(2.0, 0.0),
            pylon.location.offset(2.0, 2.0),
            pylon.location.offset(2.0, 2.0),
        ]
        .iter()
        .map(|p| self.add_building_location(*p, 3.0))
        .collect();
    }

    pub fn add_pylon_site(&mut self, location: Point2) {
        let pl = PylonLocation {
            location: location.round(),
            built: None,
        };
        self.generic_build_location_pattern(&pl);
        self.pylon_locations.push(pl);
    }

    fn add_building_location(&mut self, location: Point2, radius: f32) {
        self.building_locations.push(BuildingLocation {
            location,
            built: None,
            radius,
        });
    }

    fn check_valid_sites(
        &self,
        bot: &Bot,
        radius: &f32,
        building: &UnitTypeId,
        construct_ability: &AbilityId,
    ) -> Option<&BuildingLocation> {
        let spots: Vec<(AbilityId, Point2, Option<_>)> = if *building == UnitTypeId::Pylon {
            self.pylon_locations
                .iter()
                .filter(|pl| pl.built.is_none())
                .map(|bl| (*construct_ability, bl.location, None))
                .collect()
        } else {
            self.building_locations
                .iter()
                .filter(|bl| bl.radius == *radius && bl.built.is_none())
                .map(|bl| (*construct_ability, bl.location, None))
                .collect()
        };

        bot.query_placement(spots, false).map_or(None, |options| {
            options.iter().enumerate().find_map(|(i, ar)| {
                if *ar == ActionResult::Success {
                    self.building_locations.get(i)
                } else {
                    None
                }
            })
        })
    }

    pub fn add_building(
        &mut self,
        building: Tag,
        location: Point2,
        radius: f32,
    ) -> Result<(), InvalidUnitError> {
        match building.type_id {
            UnitTypeId::Nexus => {
                self.nexus = Some(building);
                Ok(())
            }
            UnitTypeId::Pylon => {
                self.add_pylon_site(location);
                Ok(())
            }
            UnitTypeId::Assimilator
            | UnitTypeId::Gateway
            | UnitTypeId::WarpGate
            | UnitTypeId::Forge
            | UnitTypeId::FleetBeacon
            | UnitTypeId::TwilightCouncil
            | UnitTypeId::PhotonCannon
            | UnitTypeId::Stargate
            | UnitTypeId::TemplarArchive
            | UnitTypeId::DarkShrine
            | UnitTypeId::RoboticsBay
            | UnitTypeId::RoboticsFacility
            | UnitTypeId::CyberneticsCore
            | UnitTypeId::ShieldBattery => {
                self.building_locations.push(BuildingLocation {
                    location,
                    built: Some(building),
                    radius,
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
            .iter_mut()
            .find(|l| l.built == Some(pylon.clone()))
            .is_some_and(PylonLocation::destroy)
    }

    fn find_and_destroy_building(&mut self, pylon: Tag) -> bool {
        self.building_locations
            .iter_mut()
            .find(|l| l.built == Some(pylon.clone()))
            .is_some_and(BuildingLocation::destroy)
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

impl ReBiCycler {
    pub fn is_location_powered(&self, point: Point2) -> bool {
        self.units
            .my
            .structures
            .iter()
            .of_type(UnitTypeId::Pylon)
            .ready()
            .closer(PYLON_POWER_DISTANCE, point)
            .next()
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_template_places_3_pylons() {
        let origin = Point2::new(0.0, 0.0);
        let mut sm = SitingManager::new(None, "test".to_string(), origin);

        sm.build_expansion_template(origin, Point2::new(-5.0, -5.0), Point2::new(10.0, 10.0));
        assert_eq!(sm.pylon_locations.len(), 3)
    }
}
