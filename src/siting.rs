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
    pub fn matches(&self, type_id: &UnitTypeId) -> bool {
        *self == Self::Free || *self == Self::Intended(*type_id)
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

    pub fn intersects_other(&self, other: &Self) -> bool {
        other
            .get_four_corners()
            .iter()
            .any(|p| self.inside_corners(*p))
    }
    fn inside_corners(&self, point: Point2) -> bool {
        let (top_right, bottom_left) = self.get_two_corners();
        let inside_x = bottom_left.x < point.x && point.x < top_right.x;
        let inside_y = bottom_left.y < point.y && point.y < top_right.y;
        inside_x && inside_y
    }

    fn get_two_corners(&self) -> (Point2, Point2) {
        let my_radius = self.size.radius();
        (
            self.location + (Point2::new(1.0, 1.0) * my_radius),
            self.location + (Point2::new(-1.0, -1.0) * my_radius),
        )
    }

    fn get_four_corners(&self) -> [Point2; 4] {
        let my_radius = self.size.radius();
        [
            self.location + (Point2::new(1.0, 1.0) * my_radius),
            self.location + (Point2::new(-1.0, -1.0) * my_radius),
            self.location + (Point2::new(-1.0, 1.0) * my_radius),
            self.location + (Point2::new(1.0, -1.0) * my_radius),
        ]
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

    const fn default_checker(&self) -> UnitTypeId {
        match self {
            Self::Tumor => UnitTypeId::CreepTumor,
            Self::Small => UnitTypeId::SupplyDepot,
            Self::Standard => UnitTypeId::Barracks,
            Self::Townhall => UnitTypeId::Nexus,
        }
    }
}

#[derive(Default)]
pub struct SitingDirector {
    building_locations: Vec<BuildingLocation>,
}
impl Debug for SitingDirector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let small_sites = self
            .building_locations
            .iter()
            .filter(|bl| bl.size == SlotSize::Small && bl.status != BuildingStatus::Blocked)
            .count();
        let standard_sites = self
            .building_locations
            .iter()
            .filter(|bl| bl.size == SlotSize::Standard && bl.status != BuildingStatus::Blocked)
            .count();
        let large_sites = self
            .building_locations
            .iter()
            .filter(|bl| bl.size == SlotSize::Townhall && bl.status != BuildingStatus::Blocked)
            .count();
        write!(
            f,
            "Siting Director: S:{small_sites:?} M:{standard_sites:?} L:{large_sites:?}"
        )
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

    pub fn get_available_building_sites<'a>(
        &'a self,
        size: &'a SlotSize,
        type_id: &'a UnitTypeId,
    ) -> impl 'a + Iterator<Item = &'a BuildingLocation> {
        self.building_locations.iter().filter(|bl| {
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
                let fits_intention = bl.status.matches(&type_id);
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

    fn flip_x(x: (f32, f32)) -> (f32, f32) {
        (-x.0, x.1)
    }

    fn flip_y(x: (f32, f32)) -> (f32, f32) {
        (x.0, -x.1)
    }

    fn mirror_to_four_quadrants(offsets: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
        let top_two = offsets
            .iter()
            .map(|p| Self::flip_x(*p))
            .chain(offsets.clone());
        let all_four = top_two.clone().map(Self::flip_y).chain(top_two);
        all_four.collect()
    }

    pub fn modified_artosis_pattern(center_point: Point2) -> Vec<BuildingLocation> {
        let pylon_radius = SlotSize::Small.radius();
        let pylon_width = pylon_radius * 2.0;
        let standard_radius = SlotSize::Standard.radius();
        let standard_width = standard_radius * 2.0;

        let top_pylon_point = Point2::new(0.0, pylon_radius);

        let first_quadrant_building_offsets = [
            (pylon_radius + standard_radius, -pylon_radius),
            (
                pylon_radius + standard_radius + standard_radius,
                -pylon_radius,
            ),
            (standard_radius, pylon_radius + standard_radius),
            (
                standard_radius + standard_radius,
                pylon_radius + standard_radius,
            ),
        ]
        .into_iter()
        .map(|(x, y)| (x + top_pylon_point.x, y + top_pylon_point.y));

        let all_building_points =
            Self::mirror_to_four_quadrants(first_quadrant_building_offsets.collect())
                .into_iter()
                .map(|(x, y)| center_point.offset(x, y));

        let first_quadrant_pylon_offset = [(2.0 * standard_width + pylon_radius, pylon_width)];
        let all_pylon_points = Self::mirror_to_four_quadrants(first_quadrant_pylon_offset.to_vec())
            .into_iter()
            .map(|(x, y)| center_point.offset(x, y));

        let mut pylons: Vec<BuildingLocation> = all_pylon_points
            .map(|p| BuildingLocation::new(p, SlotSize::Small, Some(UnitTypeId::Pylon)))
            .collect();
        let mut buildings: Vec<BuildingLocation> = all_building_points
            .map(|p| BuildingLocation::new(p, SlotSize::Standard, None))
            .collect();

        buildings.append(&mut pylons);
        buildings
    }

    pub fn add_pylon_site(&mut self, location: Point2) {
        let pl = BuildingLocation::new(location, SlotSize::Small, Some(UnitTypeId::Pylon));
        self.building_locations
            .append(&mut Self::modified_artosis_pattern(location));
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
            .get_available_building_sites(&size, &structure_type)
            .find(|bl| self.validate_build_location(bl, structure_type));

        if let Some(position) = position {
            let builder = self
                .units
                .my
                .workers
                .closest(position.location)
                .ok_or(BuildError::NoTrainer)?;
            builder.build(structure_type, position.location, false);
            builder.sleep(5);
            println!("Build command sent: {structure_type:?}");
            Ok(())
        } else {
            Err(BuildError::NoPlacementLocations)
        }
    }
    pub fn validate_building_locations(&mut self) {
        let blockers: Vec<bool> = self
            .siting_director
            .building_locations
            .iter()
            .map(|bl| {
                self.validate_build_location(
                    bl,
                    if let BuildingStatus::Intended(type_id) = bl.status {
                        type_id
                    } else {
                        bl.size.default_checker()
                    },
                )
            })
            .collect();

        let _: () = self
            .siting_director
            .building_locations
            .iter_mut()
            .zip(blockers)
            .map(|(bl, cp)| {
                if !cp {
                    bl.mark_blocked();
                }
            })
            .collect();
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
    fn intersect_ok() {
        let origin = Point2::new(0.0, 0.0);
        let two_over = Point2::new(2.0, 0.0);
        let bl1 = BuildingLocation::new(origin, SlotSize::Small, None);
        let bl2 = BuildingLocation::new(two_over, SlotSize::Small, None);

        assert!(!bl1.intersects_other(&bl2));
    }

    #[test]
    fn artosis_pattern_places_well() {
        let origin = Point2::new(0.0, 0.0);

        let buildings = SitingDirector::modified_artosis_pattern(origin);
        for (i, b) in buildings.iter().enumerate() {
            for (oi, ob) in buildings.iter().enumerate() {
                assert!(
                    !b.intersects_other(ob),
                    "intersecting: [{i}] {b}, [{oi}] {ob}"
                );
            }
        }
    }
}
