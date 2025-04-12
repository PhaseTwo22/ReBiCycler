use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{self, Debug, Display},
    iter::once,
};

use crate::{
    errors::{BuildError, BuildingTransitionError, UnitEmploymentError},
    protoss_bot::ReBiCycler,
    Tag, PRISM_POWER_RADIUS, PYLON_POWER_RADIUS,
};
use rust_sc2::{action::ActionResult, bot::Expansion, prelude::*};

const ACCEPTABLE_GAS_DISTANCE: f32 = 12.0;

#[allow(dead_code)]
const EXPANSION_NAMES: [&str; 48] = [
    "Α", "Β", "Γ", "Δ", "Ε", "Ζ", "Η", "Θ", "Ι", "Κ", "Λ", "Μ", "Ν", "Ξ", "Ο", "Π", "Ρ", "Σ", "Τ",
    "Υ", "Φ", "Χ", "Ψ", "Ω", "Α\'", "Β\'", "Γ\'", "Δ\'", "Ε\'", "Ζ\'", "Η\'", "Θ\'", "Ι\'", "Κ\'",
    "Λ\'", "Μ\'", "Ν\'", "Ξ\'", "Ο\'", "Π\'", "Ρ\'", "Σ\'", "Τ\'", "Υ\'", "Φ\'", "Χ\'", "Ψ\'",
    "Ω\'",
];
#[derive(PartialEq, Debug, Clone, Eq)]
pub enum BuildingStatus {
    Blocked(Option<UnitTypeId>, PylonPower),
    Free(Option<UnitTypeId>, PylonPower),

    Built(Tag, PylonPower),
    Constructing(Tag, PylonPower),
}
impl BuildingStatus {
    pub fn can_build(&self, type_id: UnitTypeId) -> bool {
        match self {
            Self::Free(None, PylonPower::Powered)
            | Self::Free(
                Some(
                    UnitTypeId::Pylon
                    | UnitTypeId::Nexus
                    | UnitTypeId::Assimilator
                    | UnitTypeId::AssimilatorRich,
                ),
                _,
            ) => true,
            Self::Free(Some(intent), PylonPower::Powered) => *intent == type_id,
            _ => false,
        }
    }

    pub const fn is_mine(&self) -> bool {
        matches!(self, Self::Built(_, _) | Self::Constructing(_, _))
    }

    pub const fn depower(self) -> Result<Self, BuildingTransitionError> {
        use PylonPower as P;
        match self {
            Self::Blocked(whatever, P::Powered) => Ok(Self::Blocked(whatever, P::Depowered)),
            Self::Free(whatever, P::Powered) => Ok(Self::Free(whatever, P::Depowered)),
            Self::Built(whatever, P::Powered) => Ok(Self::Built(whatever, P::Depowered)),
            Self::Constructing(whatever, P::Powered) => {
                Ok(Self::Constructing(whatever, P::Depowered))
            }
            _ => Err(BuildingTransitionError::InvalidTransition {
                from: self,
                change: BuildingTransition::DePower,
            }),
        }
    }

    pub const fn repower(self) -> Self {
        use PylonPower as P;
        match self {
            Self::Blocked(whatever, P::Depowered) => Self::Blocked(whatever, P::Powered),
            Self::Free(whatever, P::Depowered) => Self::Free(whatever, P::Powered),
            Self::Built(whatever, P::Depowered) => Self::Built(whatever, P::Powered),
            Self::Constructing(whatever, P::Depowered) => Self::Constructing(whatever, P::Powered),
            other => other, // powered buildings can be powered multiple times
        }
    }
}

pub struct GasLocation {
    pub geyser_tag: u64,
    pub location: Point2,
    pub status: BuildingStatus,
}

impl GasLocation {
    pub fn from_unit(unit: &Unit) -> Self {
        Self {
            geyser_tag: unit.tag(),
            location: unit.position(),
            status: BuildingStatus::Free(
                Some(UnitTypeId::Assimilator),
                crate::siting::PylonPower::Depowered,
            ),
        }
    }

    pub fn is_here(&self, building: &Tag) -> bool {
        use BuildingStatus as S;
        match self.status {
            S::Built(tag, _) | S::Constructing(tag, _) => *building == tag,
            _ => false,
        }
    }

    pub fn transition(
        &mut self,
        transition: BuildingTransition,
    ) -> Result<(), BuildingTransitionError> {
        use BuildingStatus as S;
        use BuildingTransition as T;
        let new_status = match (transition, self.status.clone()) {
            (T::DePower, state) => state.depower(),
            (T::RePower, state) => Ok(state.repower()),
            (T::Construct(tag), S::Free(_, power)) => {
                if self.status.can_build(tag.unit_type) {
                    Ok(S::Constructing(tag, power))
                } else {
                    Err(BuildingTransitionError::InvalidTransition {
                        from: self.status.clone(),
                        change: transition,
                    })
                }
            }
            (T::Obstruct, S::Free(intent, power) | S::Blocked(intent, power)) => {
                Ok(S::Blocked(intent, power))
            }

            (T::Finish, S::Constructing(tag, power)) => Ok(S::Built(tag, power)),

            (T::Destroy, S::Built(tag, power) | S::Constructing(tag, power)) => {
                Ok(S::Free(Some(tag.unit_type), power))
            }
            (T::UnObstruct, S::Blocked(intent, power) | S::Free(intent, power)) => {
                Ok(S::Free(intent, power))
            }

            (T::UnObstruct, _)
            | (T::Finish | T::Destroy, S::Free(_, _))
            | (T::Obstruct | T::Finish | T::Construct(_), S::Built(_, _))
            | (T::Obstruct | T::Construct(_), S::Constructing(_, _))
            | (T::Finish | T::Destroy | T::Construct(_), S::Blocked(_, _)) => {
                Err(BuildingTransitionError::InvalidTransition {
                    from: self.status.clone(),
                    change: transition,
                })
            }
        }?;
        self.status = new_status;
        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum PylonPower {
    Powered,
    Depowered,
}

#[derive(Debug, Clone, Copy)]
pub enum BuildingTransition {
    Destroy,
    Obstruct,
    UnObstruct,
    DePower,
    RePower,
    Construct(Tag),
    Finish,
}

const PYLON_DISTANCE_FROM_NEXUS: f32 = 9.0;
#[derive(PartialEq, Clone, Eq)]
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
    pub const fn new(location: Point2, size: SlotSize, intention: Option<UnitTypeId>) -> Self {
        Self {
            location,
            status: BuildingStatus::Free(intention, PylonPower::Depowered),
            size,
        }
    }
    pub const fn pylon(location: Point2) -> Self {
        Self::new(location, SlotSize::Small, Some(UnitTypeId::Pylon))
    }

    pub const fn standard(location: Point2) -> Self {
        Self::new(location, SlotSize::Standard, None)
    }

    pub const fn is_free(&self) -> bool {
        matches!(self.status, BuildingStatus::Free(_, _))
    }

    pub const fn is_mine(&self) -> bool {
        matches!(
            self.status,
            BuildingStatus::Constructing(_, _) | BuildingStatus::Built(_, _)
        )
    }

    pub fn is_here(&self, building: &Tag) -> bool {
        use BuildingStatus as S;
        match self.status {
            S::Built(tag, _) | S::Constructing(tag, _) => *building == tag,
            _ => false,
        }
    }

    pub fn transition(
        &mut self,
        transition: BuildingTransition,
    ) -> Result<(), BuildingTransitionError> {
        use BuildingStatus as S;
        use BuildingTransition as T;
        let new_status = match (transition, self.status.clone()) {
            (T::DePower, state) => state.depower(),
            (T::RePower, state) => Ok(state.repower()),
            (T::Construct(tag), S::Free(_, power)) => {
                if self.status.can_build(tag.unit_type) {
                    Ok(S::Constructing(tag, power))
                } else {
                    Err(BuildingTransitionError::InvalidTransition {
                        from: self.status.clone(),
                        change: transition,
                    })
                }
            }
            (T::Obstruct, S::Free(intent, power) | S::Blocked(intent, power)) => {
                Ok(S::Blocked(intent, power))
            }

            (T::Finish, S::Constructing(tag, power)) => Ok(S::Built(tag, power)),

            (T::Destroy, S::Built(tag, power) | S::Constructing(tag, power)) => {
                Ok(S::Free(Some(tag.unit_type), power))
            }
            (T::UnObstruct, S::Blocked(intent, power) | S::Free(intent, power)) => {
                Ok(S::Free(intent, power))
            }

            (T::UnObstruct, _)
            | (T::Finish | T::Destroy, S::Free(_, _))
            | (T::Obstruct | T::Finish | T::Construct(_), S::Built(_, _))
            | (T::Obstruct | T::Construct(_), S::Constructing(_, _))
            | (T::Finish | T::Destroy | T::Construct(_), S::Blocked(_, _)) => {
                Err(BuildingTransitionError::InvalidTransition {
                    from: self.status.clone(),
                    change: transition,
                })
            }
        }?;
        self.status = new_status;
        Ok(())
    }

    pub fn intersects_other(&self, other: &Self) -> bool {
        other
            .get_four_corners()
            .iter()
            .chain(once(&other.location))
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

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
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

    const fn radius(self) -> f32 {
        match self {
            Self::Tumor => 0.5,
            Self::Small => 1.0,
            Self::Standard => 1.5,
            Self::Townhall => 2.5,
        }
    }

    const fn width(self) -> f32 {
        self.radius() * 2.0
    }

    const fn default_checker(self) -> UnitTypeId {
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
    building_locations: HashMap<Point2, BuildingLocation>,
    gas_locations: HashMap<u64, GasLocation>,
}
impl Debug for SitingDirector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let small_sites = self
            .building_locations
            .values()
            .filter(|bl| bl.size == SlotSize::Small && bl.is_free())
            .count();
        let standard_sites = self
            .building_locations
            .values()
            .filter(|bl| bl.size == SlotSize::Standard && bl.is_free())
            .count();
        let large_sites = self
            .building_locations
            .values()
            .filter(|bl| bl.size == SlotSize::Townhall && bl.is_free())
            .count();
        write!(
            f,
            "Siting Director: S:{small_sites:?} M:{standard_sites:?} L:{large_sites:?}"
        )
    }
}

impl SitingDirector {
    pub fn initialize_global_placement(
        &mut self,
        expansions: &[Expansion],
        geysers: Units,
        map_center: Point2,
    ) {
        let _: () = expansions
            .iter()
            .map(|e| self.build_expansion_template(e.loc, e.center, map_center))
            .collect();

        let _: () = geysers
            .into_iter()
            .map(|u| {
                self.gas_locations
                    .insert(u.tag(), GasLocation::from_unit(&u));
            })
            .collect();
    }

    pub fn add_assimilator(&mut self, building: &Unit) -> Result<(), BuildError> {
        let geyser = self
            .gas_locations
            .values_mut()
            .find(|gl| gl.location == building.position())
            .ok_or_else(|| BuildError::NoBuildingLocationHere(building.position()))?;
        geyser.status = BuildingStatus::Built(Tag::from_unit(building), PylonPower::Depowered);
        Ok(())
    }

    pub fn lose_assimilator(&mut self, building: Tag) -> Result<(), UnitEmploymentError> {
        let geyser = self
            .gas_locations
            .values_mut()
            .find(|gl| gl.is_here(&building))
            .ok_or_else(|| {
                UnitEmploymentError(format!(
                    "We didn't have a built geyser with this tag: {building:?}",
                ))
            })?;
        geyser.status = BuildingStatus::Free(Some(UnitTypeId::Assimilator), PylonPower::Depowered);
        Ok(())
    }

    pub fn get_free_geyser(&self, near: Point2, distance: f32) -> Option<&GasLocation> {
        self.gas_locations.values().find(|gl| {
            gl.location.distance(near) <= distance && gl.status.can_build(UnitTypeId::Assimilator)
        })
    }

    pub fn construction_begin(&mut self, tag: Tag, location: Point2) -> Result<(), BuildError> {
        if crate::is_protoss_building(tag.unit_type) && !crate::is_assimilator(tag.unit_type) {
            Ok(())
        } else {
            Err(BuildError::InvalidUnit(format!(
                "{:?} at {:?}",
                tag.unit_type, location
            )))
        }?;

        self.building_locations.get_mut(&location).map_or(
            Err(BuildError::NoBuildingLocationHere(location)),
            |spot| {
                spot.transition(BuildingTransition::Construct(tag))
                    .map_err(|_| BuildError::NoBuildingLocationHere(location))
            },
        )
    }

    pub fn finish_construction(&mut self, structure: &Unit) -> Result<(), BuildError> {
        if structure.is_geyser() {
            self.gas_locations
                .get_mut(&structure.tag())
                .ok_or(BuildError::NoBuildingLocationForFinishedBuilding)?
                .transition(BuildingTransition::Finish)
                .map_err(BuildError::CantTransitionBuildingLocation)
        } else {
            self.building_locations
                .get_mut(&structure.position())
                .ok_or(BuildError::NoBuildingLocationForFinishedBuilding)?
                .transition(BuildingTransition::Finish)
                .map_err(BuildError::CantTransitionBuildingLocation)
        }
    }

    pub fn mark_position_blocked(&mut self, location: Point2) -> Result<(), BuildError> {
        self.building_locations
            .get_mut(&location)
            .ok_or(BuildError::NoBuildingLocationHere(location))?
            .transition(BuildingTransition::Obstruct)
            .map_err(|_| BuildError::NoBuildingLocationHere(location))
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
            .map(|p| self.add_pylon_site(p.round()))
            .collect();

        self.building_locations.insert(
            base_location,
            BuildingLocation {
                location: base_location,
                status: BuildingStatus::Free(Some(UnitTypeId::Nexus), PylonPower::Depowered),
                size: SlotSize::Townhall,
            },
        );
    }

    pub fn get_available_building_sites<'a>(
        &'a self,
        size: &'a SlotSize,
        type_id: &'a UnitTypeId,
    ) -> impl 'a + Iterator<Item = &'a BuildingLocation> {
        self.building_locations.values().filter(|bl| {
            let fits_intention = bl.status.can_build(*type_id);
            let fits_size = bl.size == *size;
            fits_size && fits_intention
        })
    }

    pub fn get_available_building_site_prioritized<F>(
        &self,
        size: SlotSize,
        type_id: UnitTypeId,
        priority_closure: F,
    ) -> Option<&BuildingLocation>
    where
        F: Fn(&&BuildingLocation, &&BuildingLocation) -> Ordering,
    {
        self.building_locations
            .values()
            .filter(|bl| {
                let fits_intention = bl.status.can_build(type_id);
                let fits_size = bl.size == size;
                fits_size && fits_intention
            })
            .min_by(priority_closure)
    }

    fn rotate_to_four_quadrants(offsets: &[Point2]) -> Vec<Point2> {
        let rotato = |point: &Point2| {
            vec![
                *point,
                point.rotate90(true),
                point.rotate90(true).rotate90(true),
                point.rotate90(false),
            ]
        };

        offsets.iter().flat_map(rotato).collect()
    }

    pub fn pylon_flower(center_point: Point2) -> Vec<BuildingLocation> {
        let pylon_radius = SlotSize::Small.radius();
        let pylon_width = SlotSize::Small.width();
        let standard_radius = SlotSize::Standard.radius();

        let to_the_right = vec![Point2::new(
            pylon_radius + standard_radius,
            standard_radius - pylon_width,
        )];

        Self::rotate_to_four_quadrants(&to_the_right)
            .iter()
            .map(|point| {
                BuildingLocation::new(*point, SlotSize::Standard, Some(UnitTypeId::Gateway))
            })
            .chain(vec![BuildingLocation::pylon(center_point)])
            .collect()
    }

    pub fn pylon_blossom(center_point: Point2) -> Vec<BuildingLocation> {
        let pylon_radius = SlotSize::Small.radius();
        let pylon_width = SlotSize::Small.width();
        let standard_radius = SlotSize::Standard.radius();
        let standard_width = SlotSize::Standard.width();

        let right_and_up = vec![
            Point2::new(
                pylon_radius + standard_radius,
                standard_radius - pylon_width,
            ),
            Point2::new(
                pylon_radius + standard_radius,
                standard_radius - pylon_width + standard_width,
            ),
        ];

        Self::rotate_to_four_quadrants(&right_and_up)
            .iter()
            .map(|point| BuildingLocation::standard(center_point + *point))
            .chain(vec![BuildingLocation::pylon(center_point)])
            .collect()
    }

    pub fn add_pylon_site(&mut self, location: Point2) {
        for bl in Self::pylon_blossom(location) {
            self.building_locations.insert(bl.location, bl);
        }
    }

    pub fn find_and_destroy_building(&mut self, building: &Tag) -> Result<(), BuildError> {
        self.building_locations
            .values_mut()
            .find(|l| l.is_here(building))
            .ok_or_else(|| {
                BuildError::InvalidUnit(format!("couldn't find building to destroy: {building:?}"))
            })?
            .transition(BuildingTransition::Destroy)
            .map_err(|_| {
                BuildError::InvalidUnit(format!(
                    "Target Building {building:?} is here but can't be destroyed?!"
                ))
            })
    }
}

impl ReBiCycler {
    pub fn is_location_powered(&self, point: Point2) -> bool {
        self.state
            .observation
            .raw
            .psionic_matrix
            .iter()
            .any(|m| point.is_closer(m.radius, m.pos))
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
            .find(|bl| self.location_is_buildable(bl, structure_type));

        if let Some(position) = position {
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
            Err(BuildError::NoPlacementLocations)
        }
    }
    /// Tells a base with a free geyser to build an assimilator.
    /// # Errors
    /// `BuildError::NoPlacementLocations` when no geysers are free at any base.
    pub fn build_gas(&self) -> Result<(), BuildError> {
        for nexus in &self.units.my.townhalls {
            if self.take_gas(nexus.position()).is_ok() {
                return Ok(());
            }
        }
        Err(BuildError::NoPlacementLocations)
    }

    pub fn update_building_power(
        &mut self,
        unit_change: UnitTypeId,
        power_point: Point2,
        turned_on: bool,
    ) {
        let change_radius = match unit_change {
            UnitTypeId::Pylon => PYLON_POWER_RADIUS,
            UnitTypeId::WarpPrismPhasing => PRISM_POWER_RADIUS,
            UnitTypeId::WarpPrism => PRISM_POWER_RADIUS,
            _ => 0.0,
        };
        let change_type = if turned_on {
            BuildingTransition::RePower
        } else {
            BuildingTransition::DePower
        };

        let errors: Vec<BuildingTransitionError> = self
            .siting_director
            .building_locations
            .values_mut()
            .filter_map(|bl| {
                if bl.location.distance(power_point) <= change_radius {
                    bl.transition(change_type).err()
                } else {
                    None
                }
            })
            .collect();

        for error in errors {
            println!("{error:?}");
        }
    }

    pub fn update_building_obstructions(&mut self) {
        let blocked: Vec<bool> = self
            .siting_director
            .building_locations
            .values()
            .filter(|bl| !bl.is_mine())
            .map(|bl| {
                self.location_is_buildable(
                    bl,
                    if let BuildingStatus::Free(Some(type_id), _) = bl.status {
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
            .values_mut()
            .zip(blocked)
            .map(|(bl, can_place)| {
                if can_place {
                    if let Err(e) = bl.transition(BuildingTransition::UnObstruct) {
                        println!("{e:?}");
                    };
                } else if let Err(e) = bl.transition(BuildingTransition::Obstruct) {
                    println!("{e:?}");
                }
            })
            .collect();

        println!("Building locations updated: {:?}", self.siting_director);
    }

    fn location_is_buildable(
        &self,
        build_location: &BuildingLocation,
        structure_type: UnitTypeId,
    ) -> bool {
        let result = self
            .query_placement(
                vec![(
                    self.game_data.units[&structure_type].ability.unwrap(),
                    build_location.location,
                    None,
                )],
                false,
            )
            .unwrap()[0];
        if result == ActionResult::Success {
            true
        } else if result == ActionResult::CantBuildLocationInvalid {
            false
        } else {
            println!(
                "Location {:?} is blocked: {:?}",
                build_location.location, result
            );
            false
        }
    }

    /// Assigns a worker to the nearest base.
    ///
    /// # Errors
    /// `UnitEmploymentError` if no base managers exist, or we have no townhalls.
    pub fn back_to_work(&mut self, worker: &Unit) -> Result<(), UnitEmploymentError> {
        if let Err(e) = self.mining_manager.assign_miner(worker) {
            println!("Can't employ worker: {e:?}");
        }
        Ok(())
    }

    /// When a new base finishes, we want to make a new Base Manager for it.
    /// Add the resources and existing buildings, if any.
    /// # Errors
    /// `BuildError::NoBuildingLocationHere` if the base isn't on an expansion location
    pub fn new_base_finished(&mut self, nexus: &Unit) -> Result<(), BuildError> {
        self.mining_manager.add_townhall(nexus.clone());
        Ok(())
    }
    /// Finds a gas to take at the specified base and builds it
    /// # Errors
    /// `BuildError::NoPlacementLocations` when there's no geysers free at this base
    /// `BuildError::NoBuildingLocationHere` when this isn't an expansion location
    pub fn take_gas(&self, near: Point2) -> Result<(), BuildError> {
        let gas = self
            .siting_director
            .get_free_geyser(near, ACCEPTABLE_GAS_DISTANCE);
        if let Some(geyser) = gas {
            let builder = self
                .units
                .my
                .workers
                .closest(geyser.location)
                .ok_or(BuildError::NoTrainer)?;
            builder.build_gas(geyser.geyser_tag, false);
            builder.sleep(5);
            Ok(())
        } else {
            Err(BuildError::NoPlacementLocations)
        }
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

        let bl3 = BuildingLocation::new(origin, SlotSize::Standard, None);
        let bl4 = BuildingLocation::new(origin, SlotSize::Standard, None);

        assert!(bl3.intersects_other(&bl4));
    }

    #[test]
    fn pylon_flower_makes_five() {
        assert_eq!(
            SitingDirector::pylon_flower(Point2 { x: 0.0, y: 0.0 }).len(),
            5
        );
    }

    fn buildings_intersect(buildings: &[BuildingLocation]) -> bool {
        for a in buildings {
            for b in buildings {
                if a == b {
                    continue;
                }
                if a.intersects_other(b) {
                    return true;
                }
            }
        }
        false
    }

    #[test]
    fn pylon_flower_doesnt_self_intersect() {
        let origin = Point2::new(0.0, 0.0);
        let buildings = SitingDirector::pylon_flower(origin);

        assert!(!buildings_intersect(&buildings));
    }

    #[test]
    fn pylon_blossom_doesnt_self_intersect() {
        let origin = Point2::new(0.0, 0.0);
        let buildings = SitingDirector::pylon_blossom(origin);

        assert!(!buildings_intersect(&buildings));
    }
}
