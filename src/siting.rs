use std::{
    cmp::Ordering,
    fmt::{self, Debug, Display},
};

use crate::{
    errors::{BuildError, InvalidUnitError},
    protoss_bot::ReBiCycler,
    Tag,
};
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
#[derive(PartialEq, Debug, Clone)]
enum BuildingStatus {
    Blocked,
    Intended(UnitTypeId),
    Built(Tag),
    Free,
}
impl BuildingStatus {
    pub fn matches(&self, type_id: &UnitTypeId) -> bool {
        *self == BuildingStatus::Free || *self == BuildingStatus::Intended(*type_id)
    }
}

const PYLON_DISTANCE_FROM_NEXUS: f32 = 9.0;
#[derive(PartialEq, Clone)]
pub struct BuildingLocation {
    pub location: Point2,
    status: BuildingStatus,
    size: SlotSize,
}
impl Display for BuildingLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {:?}, {:?}", self.location, self.status, self.size)
    }
}
impl BuildingLocation {
    pub fn new(location: Point2, size: SlotSize) -> BuildingLocation {
        BuildingLocation {
            location,
            status: BuildingStatus::Free,
            size,
        }
    }
    pub fn destroy(&mut self) -> Result<(), BuildError> {
        match &self.status {
            BuildingStatus::Built(tag) => {
                self.status = BuildingStatus::Intended(tag.type_id);
                Ok(())
            }
            _ => Err(BuildError::InvalidUnit(
                "Trying to destroy in a available building slot!".to_string(),
            )),
        }
    }
    pub fn build(&mut self, building: Tag) {
        self.status = BuildingStatus::Built(building);
    }
    pub fn mark_blocked(&mut self) {
        self.status = BuildingStatus::Blocked;
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum SlotSize {
    Tumor,
    Small,
    Standard,
    Townhall,
}

impl SlotSize {
    pub fn from(structure_type: &UnitTypeId) -> Result<SlotSize, BuildError> {
        match structure_type {
            UnitTypeId::Nexus => Ok(SlotSize::Townhall),
            UnitTypeId::Assimilator
            | UnitTypeId::Gateway
            | UnitTypeId::WarpGate
            | UnitTypeId::Forge
            | UnitTypeId::FleetBeacon
            | UnitTypeId::TwilightCouncil
            | UnitTypeId::Stargate
            | UnitTypeId::TemplarArchive
            | UnitTypeId::RoboticsBay
            | UnitTypeId::RoboticsFacility
            | UnitTypeId::CyberneticsCore => Ok(SlotSize::Standard),
            UnitTypeId::PhotonCannon
            | UnitTypeId::DarkShrine
            | UnitTypeId::ShieldBattery
            | UnitTypeId::Pylon => Ok(SlotSize::Small),
            _ => Err(BuildError::InvalidUnit(format!(
                "This is not a protoss structure: {structure_type:?}"
            ))),
        }
    }
}

#[derive(Default)]
pub struct SitingDirector {
    building_locations: Vec<BuildingLocation>,
}
impl Debug for SitingDirector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sites: Vec<String> = self
            .building_locations
            .iter()
            .map(|v| v.to_string())
            .collect();
        write!(f, "{:?}", sites.join("\n"))
    }
}

impl SitingDirector {
    pub fn new() -> Self {
        Self {
            building_locations: Vec::new(),
        }
    }

    pub fn initialize_global_placement(&mut self, expansions: Vec<Expansion>, map_center: Point2) {
        expansions
            .iter()
            .map(|e| self.build_expansion_template(e.loc, e.center, map_center));
    }

    pub fn construction_begin(&mut self, tag: Tag, location: Point2) -> Result<(), BuildError> {
        let has_spot = self
            .building_locations
            .iter_mut()
            .find(|bl| bl.location == location);
        if let Some(spot) = has_spot {
            spot.build(tag);
            Ok(())
        } else {
            Err(BuildError::CantPlace(location, tag.type_id))
        }
    }

    pub fn mark_position_blocked(&mut self, location: &Point2) {
        self.building_locations
            .iter_mut()
            .find(|bl| bl.location == *location)
            .map(|bl| bl.mark_blocked());
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

    pub fn get_available_building_site(
        &self,
        size: &SlotSize,
        type_id: &UnitTypeId,
    ) -> Option<&BuildingLocation> {
        self.building_locations.iter().find(|bl| {
            let fits_intention = (bl.status.matches(type_id)) | (bl.status == BuildingStatus::Free);
            let fits_size = bl.size == *size;
            fits_size && fits_intention
        })
    }

    pub fn get_available_building_site_prioritized<F>(
        &self,
        size: &SlotSize,
        type_id: &UnitTypeId,
        priority_closure: F,
    ) -> Option<&BuildingLocation>
    where
        F: Fn(&&BuildingLocation, &&BuildingLocation) -> Ordering,
    {
        self.building_locations
            .iter()
            .filter(|bl| {
                let fits_intention = bl.status.matches(type_id);
                let fits_size = bl.size == *size;
                fits_size && fits_intention
            })
            .min_by(priority_closure)
    }

    pub fn generic_build_location_pattern(&mut self, pylon: &BuildingLocation) {
        let _: () = [
            pylon.location.offset(2.0, 0.0),
            pylon.location.offset(2.0, 2.0),
            pylon.location.offset(2.0, 2.0),
        ]
        .iter()
        .map(|p| self.add_building_location(*p, SlotSize::Standard, None))
        .collect();
    }

    pub fn add_pylon_site(&mut self, location: Point2) {
        let pl = BuildingLocation {
            location,
            size: SlotSize::Small,
            status: BuildingStatus::Intended(UnitTypeId::Pylon),
        };
        self.generic_build_location_pattern(&pl);
        self.building_locations.push(pl);
    }

    fn add_building_location(
        &mut self,
        location: Point2,
        size: SlotSize,
        intent: Option<UnitTypeId>,
    ) {
        self.building_locations.push(BuildingLocation {
            location,
            status: if let Some(type_id) = intent {
                BuildingStatus::Intended(type_id)
            } else {
                BuildingStatus::Free
            },
            size,
        });
    }

    fn check_valid_sites(
        &self,
        bot: &Bot,
        size: SlotSize,
        building: &UnitTypeId,
        construct_ability: &AbilityId,
    ) -> Option<&BuildingLocation> {
        let spots: Vec<(AbilityId, Point2, Option<_>)> = self
            .building_locations
            .iter()
            .filter(|bl| bl.size == size && bl.status.matches(building))
            .map(|bl| (*construct_ability, bl.location, None))
            .collect();

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
    ) -> Result<(), InvalidUnitError> {
        match building.type_id {
            UnitTypeId::Nexus
            | UnitTypeId::Pylon
            | UnitTypeId::Assimilator
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
                self.fill_in_building_location(building, location);
                Ok(())
            }
            _ => Err(InvalidUnitError("Not a Protoss building!".to_string())),
        }
    }

    fn find_building_location(&self, location: Point2) -> Option<&BuildingLocation> {
        self.building_locations
            .iter()
            .find(|bl| bl.location == location)
    }

    fn fill_in_building_location(
        &mut self,
        building: Tag,
        location: Point2,
    ) -> Result<(), BuildError> {
        let filled_slot = self
            .building_locations
            .iter_mut()
            .find(|bl| bl.location == location);
        if let Some(bl) = filled_slot {
            bl.build(building);
            Ok(())
        } else {
            Err(BuildError::NoPlacementLocations)
        }
    }

    pub fn find_and_destroy_building(&mut self, building: Tag) -> Result<(), BuildError> {
        if let Some(found_building) = self
            .building_locations
            .iter_mut()
            .find(|l| l.status == BuildingStatus::Built(building.clone()))
        {
            found_building.destroy()
        } else {
            Err(BuildError::InvalidUnit(format!(
                "couldn't find building to destroy: {building:?}"
            )))
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

    pub fn build(&self, structure_type: &UnitTypeId) -> Result<(), BuildError> {
        let size = SlotSize::from(structure_type)?;
        //self.game_data.units[structure_type]

        let position = self
            .siting_director
            .get_available_building_site(&size, structure_type);

        if let Some(position) = position {
            if !self.validate_build_location(position, structure_type) {
                Err(BuildError::CantPlace(position.location, *structure_type))
            } else {
                let builder = self.units.my.workers.closest(position.location).unwrap();
                builder.build(*structure_type, position.location, false);
                builder.sleep(5);
                Ok(())
            }
        } else {
            Err(BuildError::NoPlacementLocations)
        }
    }

    fn validate_build_location(
        &self,
        build_location: &BuildingLocation,
        structure_type: &UnitTypeId,
    ) -> bool {
        self.can_place(*structure_type, build_location.location)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_template_places_12_buildings() {
        let origin = Point2::new(0.0, 0.0);
        let mut sd = SitingDirector::new();

        sd.build_expansion_template(origin, Point2::new(-5.0, -5.0), Point2::new(10.0, 10.0));
        assert_eq!(sd.building_locations.len(), 12)
    }
}
