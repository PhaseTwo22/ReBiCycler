use std::{
    cmp::Ordering,
    fmt::{self, Debug, Display},
};

use crate::{errors::BuildError, protoss_bot::ReBiCycler, Tag};
use rust_sc2::{bot::Expansion, prelude::*};

const PYLON_POWER_DISTANCE: f32 = 6.5;
#[allow(dead_code)]
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
    pub fn matches(&self, type_id: UnitTypeId) -> bool {
        *self == Self::Free || *self == Self::Intended(type_id)
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
    pub fn new(location: Point2, size: SlotSize, intention: Option<UnitTypeId>) -> Self {
        Self {
            location,
            status: intention.map_or_else(|| BuildingStatus::Free, BuildingStatus::Intended),
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

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum SlotSize {
    Tumor,
    Small,
    Standard,
    Townhall,
}

impl SlotSize {
    pub fn from(structure_type: UnitTypeId) -> Result<Self, BuildError> {
        match structure_type {
            UnitTypeId::Nexus => Ok(Self::Townhall),
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
            | UnitTypeId::CyberneticsCore => Ok(Self::Standard),
            UnitTypeId::PhotonCannon
            | UnitTypeId::DarkShrine
            | UnitTypeId::ShieldBattery
            | UnitTypeId::Pylon => Ok(Self::Small),
            _ => Err(BuildError::InvalidUnit(format!(
                "This is not a protoss structure: {structure_type:?}"
            ))),
        }
    }

    const fn radius(&self) -> f32 {
        match self {
            Self::Tumor => 0.5,
            Self::Small => 1.0,
            Self::Standard => 1.5,
            Self::Townhall => 2.5,
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
            .map(BuildingLocation::to_string)
            .collect();
        write!(f, "{:?}", sites.join("\n"))
    }
}

impl SitingDirector {
    pub fn initialize_global_placement(&mut self, expansions: &[Expansion], map_center: Point2) {
        let _: () = expansions
            .iter()
            .map(|e| self.build_expansion_template(e.loc, e.center, map_center))
            .collect();
    }

    pub fn construction_begin(&mut self, tag: Tag, location: Point2) -> Result<(), BuildError> {
        match tag.type_id {
            UnitTypeId::Nexus
            | UnitTypeId::Pylon
            | UnitTypeId::Assimilator
            | UnitTypeId::AssimilatorRich
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
            | UnitTypeId::ShieldBattery => Ok(()),
            _ => Err(BuildError::InvalidUnit(format!(
                "{:?} at {:?}",
                tag.type_id, location
            ))),
        }?;
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

    pub fn mark_position_blocked(&mut self, location: Point2) -> Result<(), BuildError> {
        self.building_locations
            .iter_mut()
            .find(|bl| bl.location == location)
            .ok_or(BuildError::NoBuildingLocationHere(location))?
            .mark_blocked();
        Ok(())
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

        let places = pylon_spots.map_or([Some(pylon_behind_location), None, None], |spots| {
            [Some(pylon_behind_location), Some(spots[0]), Some(spots[1])]
        });

        let _: () = places
            .iter()
            .flatten()
            .map(|p| self.add_pylon_site(*p))
            .collect();
    }

    pub fn get_available_building_site(
        &self,
        size: &SlotSize,
        type_id: UnitTypeId,
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
        type_id: UnitTypeId,
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

    fn generic_build_location_pattern(pylon_point: Point2) -> Vec<BuildingLocation> {
        [
            pylon_point.offset(2.0, 0.0),
            pylon_point.offset(2.0, 2.0),
            pylon_point.offset(2.0, 2.0),
        ]
        .iter()
        .map(|p| BuildingLocation::new(*p, SlotSize::Standard, None))
        .collect()
    }

    pub fn modified_artosis_pattern(top_pylon_point: Point2) -> Vec<BuildingLocation> {
        let pylon_radius = SlotSize::Small.radius();
        let standard_radius = SlotSize::Standard.radius();
        let standard_width = standard_radius * 2.0;

        let top_half_offsets = [
            (-pylon_radius - standard_radius, -pylon_radius), // left adjacent to pylon
            (
                // left of the one above
                -pylon_radius - standard_radius - standard_width,
                -pylon_radius,
            ),
            (
                // up left from pylon
                -standard_radius,
                pylon_radius + standard_radius,
            ),
            (
                // left of above
                -standard_radius - standard_width,
                pylon_radius + standard_radius,
            ),
            (pylon_radius + standard_radius, -pylon_radius), // right adjacent to pylon
            (
                // right of the one above
                pylon_radius + standard_radius + standard_width,
                -pylon_radius,
            ),
            (
                // up right from pylon
                standard_radius,
                pylon_radius + standard_radius,
            ),
            (
                // right of above
                standard_radius + standard_width,
                pylon_radius + standard_radius,
            ),
        ]
        .into_iter()
        .map(|(x, y)| Point2::new(x, y));
        let bottom_half_offsets = top_half_offsets
            .clone()
            .map(|p| p.rotate(180.0).offset(0.0, pylon_radius * 4.0));

        let bottom_pylon_point = top_pylon_point.offset(0.0, -pylon_radius * 4.0);
        let pylons: Vec<BuildingLocation> = [
            top_pylon_point,
            top_pylon_point.offset(-(2.0 * standard_width), standard_width), // top far left
            top_pylon_point.offset(2.0 * standard_width, standard_width),    // top far right
            bottom_pylon_point,
            bottom_pylon_point.offset(-(2.0 * standard_width), -standard_width), // top far left
            bottom_pylon_point.offset(2.0 * standard_width, -standard_width),    // top far right
        ]
        .into_iter()
        .map(|p| BuildingLocation::new(p, SlotSize::Small, Some(UnitTypeId::Pylon)))
        .collect();

        let full: Vec<BuildingLocation> = top_half_offsets
            .chain(bottom_half_offsets)
            .map(|point| BuildingLocation::new(point, SlotSize::Standard, None))
            .chain(pylons)
            .collect();

        full
    }

    pub fn add_pylon_site(&mut self, location: Point2) {
        let pl = BuildingLocation::new(location, SlotSize::Small, Some(UnitTypeId::Pylon));
        self.building_locations
            .append(&mut Self::generic_build_location_pattern(location));
        self.building_locations.push(pl);
    }

    pub fn find_and_destroy_building(&mut self, building: &Tag) -> Result<(), BuildError> {
        self.building_locations
            .iter_mut()
            .find(|l| l.status == BuildingStatus::Built(building.clone()))
            .ok_or_else(|| {
                BuildError::InvalidUnit(format!("couldn't find building to destroy: {building:?}"))
            })?
            .destroy()
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

    /// Finds a site for the building, validates the position, and commands a worker to go build it.
    /// # Errors
    /// - `BuildError::NoPlacementLocation`
    /// - `BuildError::NoTrainer` if we have no workers
    /// - `BuildError::CantPlace` if we can't place at the found location.
    pub fn build(&self, structure_type: UnitTypeId) -> Result<(), BuildError> {
        let size = SlotSize::from(structure_type)?;
        //self.game_data.units[structure_type]

        let position = self
            .siting_director
            .get_available_building_site(&size, structure_type)
            .ok_or(BuildError::NoPlacementLocations)?;

        if self.validate_build_location(position, structure_type) {
            let builder = self
                .units
                .my
                .workers
                .closest(position.location)
                .ok_or(BuildError::NoTrainer)?;
            builder.build(structure_type, position.location, false);
            builder.sleep(5);
            Ok(())
        } else {
            Err(BuildError::CantPlace(position.location, structure_type))
        }
    }

    fn validate_build_location(
        &self,
        build_location: &BuildingLocation,
        structure_type: UnitTypeId,
    ) -> bool {
        self.can_place(structure_type, build_location.location)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_template_places_12_buildings() {
        let origin = Point2::new(0.0, 0.0);
        let mut sd = SitingDirector::default();

        sd.build_expansion_template(origin, Point2::new(-5.0, -5.0), Point2::new(10.0, 10.0));
        assert_eq!(sd.building_locations.len(), 12);
    }
}
