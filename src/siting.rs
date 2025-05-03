use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{self, Debug, Display},
};

use crate::{
    errors::{BuildError, BuildingTransitionError, UnitEmploymentError},
    micro::MiningError,
    protoss_bot::ReBiCycler,
    Tag, PRISM_POWER_RADIUS, PYLON_POWER_RADIUS,
};
use image::Rgba;
use itertools::{iproduct, Either};
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
        let needs_power = crate::structure_needs_power(&type_id);

        match self {
            Self::Free(intent, power) => {
                let power_ok = !needs_power || *power == PylonPower::Powered;
                let intent_ok = intent.map_or(true, |i| i == type_id);
                power_ok && intent_ok
            }

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

pub struct ConstructionSite {
    pub status: BuildingStatus,
    pub location: LocationType,
}
impl Display for ConstructionSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status_part = match self.status {
            BuildingStatus::Free(_, _) => "Free",
            BuildingStatus::Built(tag, _) | BuildingStatus::Constructing(tag, _) => {
                &format!("{:?}", tag.unit_type)
            }
            BuildingStatus::Blocked(_, _) => "Blocked",
        };
        let location_part = match self.location {
            LocationType::AtPoint(p, s) => format!("{}({:.1},{:.1})", s, p.x, p.y),
            LocationType::OnGeyser(_g, p) => format!("OnGeyser({},{})", p.x, p.y),
        };
        write!(f, "{status_part}{location_part}")
    }
}

impl ConstructionSite {
    pub const fn new(intention: Option<UnitTypeId>, location: LocationType) -> Self {
        let status = BuildingStatus::Free(intention, PylonPower::Depowered);

        Self { status, location }
    }

    pub const fn pylon(location: Point2) -> Self {
        Self::new(
            Some(UnitTypeId::Pylon),
            LocationType::AtPoint(location, SlotSize::Small),
        )
    }

    pub const fn nexus(location: Point2) -> Self {
        Self::new(
            Some(UnitTypeId::Nexus),
            LocationType::AtPoint(location, SlotSize::Townhall),
        )
    }

    pub const fn standard(location: Point2) -> Self {
        Self::new(None, LocationType::AtPoint(location, SlotSize::Standard))
    }

    pub fn new_gas(geyser: &Unit) -> Self {
        Self::new(
            Some(UnitTypeId::Assimilator),
            LocationType::from_geyser(geyser),
        )
    }

    pub const fn is_free(&self) -> bool {
        matches!(self.status, BuildingStatus::Free(_, _))
    }

    pub const fn is_gas(&self) -> bool {
        matches!(self.location, LocationType::OnGeyser(_, _))
    }

    pub const fn size(&self) -> SlotSize {
        match self.location {
            LocationType::AtPoint(_p, s) => s,
            LocationType::OnGeyser(_g, _p) => SlotSize::Standard,
        }
    }

    pub const fn location(&self) -> Point2 {
        match self.location {
            LocationType::AtPoint(p, _s) => p,
            LocationType::OnGeyser(_g, p) => p,
        }
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

    pub fn could_build(&self) -> bool {
        match self.status {
            BuildingStatus::Free(intent, _) => self
                .status
                .can_build(intent.unwrap_or_else(|| self.size().default_checker())),
            _ => false,
        }
    }

    pub const fn color(&self, a: u8) -> Rgba<u8> {
        Rgba(match self.status {
            BuildingStatus::Free(_, PylonPower::Powered) => [0, 128, 0, a], //"green",
            BuildingStatus::Free(_, PylonPower::Depowered) => [255, 255, 0, a], //"yellow",
            BuildingStatus::Built(_, PylonPower::Depowered) => [0, 0, 205, a], //"dark blue",
            BuildingStatus::Built(_, PylonPower::Powered) => [0, 0, 255, a], //"blue",
            BuildingStatus::Constructing(_, PylonPower::Depowered) => [135, 206, 250, a], //"light blue",
            BuildingStatus::Constructing(_, PylonPower::Powered) => [70, 130, 180, a],    //"blue",
            BuildingStatus::Blocked(_, PylonPower::Depowered) => [124, 72, 72, a], //"dark red",
            BuildingStatus::Blocked(_, PylonPower::Powered) => [139, 0, 0, a],     //"red",
        })
    }

    /// Gets a `UnitTypeId` to use to evaluate whether or not we can place something here
    /// If we already have something here, return None
    /// Otherwise, return something we could use to actually check it.
    pub fn placement_checker(&self) -> Option<UnitTypeId> {
        // when is it good to check?
        // when I don't have anything here.
        // when it's maybe blocked.
        match self.status {
            BuildingStatus::Blocked(intent, _) | BuildingStatus::Free(intent, _) => {
                intent.or_else(|| Some(self.size().default_checker()))
            }
            _ => None,
        }
    }

    pub const fn needs_power(&self) -> bool {
        let unit_type = match self.status {
            BuildingStatus::Blocked(Some(intent), _) | BuildingStatus::Free(Some(intent), _) => {
                intent
            }
            BuildingStatus::Constructing(tag, _) | BuildingStatus::Built(tag, _) => tag.unit_type,
            _ => return true, //no intent, probably needs power.
        };
        crate::structure_needs_power(&unit_type)
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
            (T::Morph(new_type), S::Built(tag, power)) => Ok(S::Built(
                Tag {
                    tag: tag.tag,
                    unit_type: new_type,
                },
                power,
            )),

            (T::UnObstruct | T::Morph(_), _)
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

pub enum LocationType {
    OnGeyser(u64, Point2),
    AtPoint(Point2, SlotSize),
}

impl LocationType {
    pub fn from_geyser(geyser: &Unit) -> Self {
        Self::OnGeyser(geyser.tag(), geyser.position())
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
    Morph(UnitTypeId),
}

const PYLON_DISTANCE_FROM_NEXUS: f32 = 9.0;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum SlotSize {
    Tumor,
    Small,
    Standard,
    Townhall,
}
impl Display for SlotSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Tumor => "1x1",
                Self::Small => "2x2",
                Self::Standard => "3x3",
                Self::Townhall => "5x5",
            }
        )
    }
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
            Self::Small => UnitTypeId::Pylon,
            Self::Standard => UnitTypeId::Gateway,
            Self::Townhall => UnitTypeId::Nexus,
        }
    }
    pub fn contained_points(self, center: Point2) -> impl Iterator<Item = (u32, u32)> {
        let bottom_left = center.offset(-self.radius(), -self.radius());

        let range = match self {
            Self::Tumor => todo!(),
            Self::Small => 0..2,
            Self::Standard => 0..3,
            Self::Townhall => 0..5,
        };
        let offsets = iproduct!(range.clone(), range);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        offsets.map(move |(x, y)| (bottom_left.x as u32 + x, bottom_left.y as u32 + y))
    }
}

#[derive(Default)]
pub struct SitingDirector {
    sites: HashMap<Point2, ConstructionSite>,
}
impl Debug for SitingDirector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut smalls = 0;
        let mut standards = 0;
        let mut larges = 0;
        let mut gasses = 0;

        for site in self.sites.values() {
            if site.is_gas() {
                gasses += 1;
            } else {
                match site.size() {
                    SlotSize::Small => {
                        smalls += 1;
                    }
                    SlotSize::Standard => {
                        standards += 1;
                    }
                    SlotSize::Townhall => {
                        larges += 1;
                    }
                    SlotSize::Tumor => (),
                }
            }
        }
        write!(
            f,
            "Sites: S:{smalls:?} M:{standards:?} L:{larges:?} G:{gasses:?}",
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
        let structures: Vec<(Point2, ConstructionSite)> = expansions
            .iter()
            .flat_map(|e| self.expansion_template(e.loc, e.center, map_center))
            .map(|cs| (cs.location(), cs))
            .collect();

        let gasses: Vec<(Point2, ConstructionSite)> = geysers
            .iter()
            .map(ConstructionSite::new_gas)
            .map(|cs| (cs.location(), cs))
            .collect();

        self.sites.extend(structures);
        self.sites.extend(gasses);
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, Point2, ConstructionSite> {
        self.sites.iter()
    }

    pub fn add_initial_nexus(&mut self, nexus: &Unit) -> Result<(), BuildError> {
        let home_loc = self
            .sites
            .get_mut(&nexus.position())
            .ok_or_else(|| BuildError::CantPlace(nexus.position(), nexus.type_id()))?;
        home_loc.status = BuildingStatus::Built(Tag::from_unit(nexus), PylonPower::Depowered);
        Ok(())
    }

    pub fn add_assimilator(&mut self, building: &Unit) -> Result<(), BuildError> {
        let geyser = self
            .sites
            .get_mut(&building.position())
            .ok_or_else(|| BuildError::NoConstructionSiteHere(building.position()))?;
        geyser.status = BuildingStatus::Built(Tag::from_unit(building), PylonPower::Depowered);
        Ok(())
    }

    pub fn lose_assimilator(&mut self, building: Tag) -> Result<(), UnitEmploymentError> {
        let geyser = self
            .sites
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

    pub fn get_free_geyser(&self, near: Point2, distance: f32) -> Option<&ConstructionSite> {
        self.sites.values().find(|gl| {
            gl.location().distance(near) <= distance && gl.status.can_build(UnitTypeId::Assimilator)
        })
    }

    pub fn construction_begin(&mut self, tag: Tag, location: Point2) -> Result<(), BuildError> {
        if crate::is_protoss_building(&tag.unit_type) && !crate::is_assimilator(tag.unit_type) {
            Ok(())
        } else {
            Err(BuildError::InvalidUnit(format!(
                "{:?} at {:?}",
                tag.unit_type, location
            )))
        }?;

        self.sites.get_mut(&location).map_or(
            Err(BuildError::NoConstructionSiteHere(location)),
            |spot| {
                spot.transition(BuildingTransition::Construct(tag))
                    .map_err(|_| BuildError::NoConstructionSiteHere(location))
            },
        )
    }

    pub fn finish_construction(&mut self, structure: &Unit) -> Result<(), BuildError> {
        if structure.is_geyser() {
            self.sites
                .get_mut(&structure.position())
                .ok_or_else(|| {
                    BuildError::NoConstructionSiteForFinishedBuilding(structure.type_id())
                })?
                .transition(BuildingTransition::Finish)
                .map_err(BuildError::CantTransitionBuildingLocation)
        } else {
            self.sites
                .get_mut(&structure.position())
                .ok_or_else(|| {
                    BuildError::NoConstructionSiteForFinishedBuilding(structure.type_id())
                })?
                .transition(BuildingTransition::Finish)
                .map_err(BuildError::CantTransitionBuildingLocation)
        }
    }

    pub fn mark_position_blocked(
        &mut self,
        location: Point2,
        make_obstructed: BuildingTransition,
    ) -> Result<(), Either<BuildError, BuildingTransitionError>> {
        self.sites
            .get_mut(&location)
            .ok_or(Either::Left(BuildError::NoConstructionSiteHere(location)))?
            .transition(make_obstructed)
            .map_err(|_| Either::Left(BuildError::NoConstructionSiteHere(location)))
    }

    fn expansion_template(
        &self,
        base_location: Point2,
        mineral_center: Point2,

        map_center: Point2,
    ) -> Vec<ConstructionSite> {
        let mut new_sites = Vec::new();

        let distance_to_minerals = 6.0;
        let pylon_behind_location =
            { base_location.towards(mineral_center, distance_to_minerals + 3.0) };

        let direction_to_map_center = base_location.towards(map_center, 8.0);

        let pylon_spots =
            base_location.circle_intersection(direction_to_map_center, PYLON_DISTANCE_FROM_NEXUS);

        let places = pylon_spots.map_or([Some(pylon_behind_location), None, None], |spots| {
            [Some(pylon_behind_location), Some(spots[0]), Some(spots[1])]
        });

        let pylon_patterns: Vec<ConstructionSite> = places
            .iter()
            .flatten()
            .flat_map(|p| self.pylon_pattern(p.round()))
            .collect();

        new_sites.extend(pylon_patterns);
        new_sites.push(ConstructionSite::nexus(base_location));
        new_sites
    }

    pub fn get_available_building_sites<'a>(
        &'a self,
        size: &'a SlotSize,
        type_id: &'a UnitTypeId,
    ) -> impl 'a + Iterator<Item = &'a ConstructionSite> {
        self.sites.values().filter(|bl| {
            let fits_status = bl.status.can_build(*type_id);
            let fits_size = bl.size() == *size;
            fits_size && fits_status
        })
    }

    pub fn get_available_building_site_prioritized<F>(
        &self,
        size: SlotSize,
        type_id: UnitTypeId,
        priority_closure: F,
    ) -> Option<&ConstructionSite>
    where
        F: Fn(&&ConstructionSite, &&ConstructionSite) -> Ordering,
    {
        self.sites
            .values()
            .filter(|bl| {
                let fits_intention = bl.status.can_build(type_id);
                let fits_size = bl.size() == size;
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

    fn mirror_to_four_quadrants(offsets: &[Point2]) -> Vec<Point2> {
        let flippify = |point: &Point2| {
            let (x, y) = point.as_tuple();
            vec![(x, y), (-x, y), (-x, -y), (x, -y)]
        };
        offsets
            .iter()
            .flat_map(flippify)
            .map(|(x, y)| Point2::new(x, y))
            .collect()
    }

    pub fn pylon_flower(center_point: Point2) -> Vec<ConstructionSite> {
        let pylon_radius = SlotSize::Small.radius();
        let pylon_width = SlotSize::Small.width();
        let standard_radius = SlotSize::Standard.radius();

        let to_the_right = vec![Point2::new(
            pylon_radius + standard_radius,
            standard_radius - pylon_width,
        )];

        Self::rotate_to_four_quadrants(&to_the_right)
            .iter()
            .map(|point| ConstructionSite::standard(*point))
            .chain(vec![ConstructionSite::pylon(center_point)])
            .collect()
    }

    pub fn pylon_interceptor(center_point: Point2) -> Vec<ConstructionSite> {
        let pylon_radius = SlotSize::Small.radius();

        let standard_radius = SlotSize::Standard.radius();
        let standard_width = SlotSize::Standard.width();

        let l_shape_to_the_right = vec![
            Point2::new(pylon_radius + standard_radius, standard_radius),
            Point2::new(
                pylon_radius + standard_radius + standard_width,
                standard_radius,
            ),
            Point2::new(
                pylon_radius + standard_radius,
                standard_radius + standard_width,
            ),
        ];

        Self::mirror_to_four_quadrants(&l_shape_to_the_right)
            .iter()
            .map(|point| ConstructionSite::standard(center_point + *point))
            .chain(vec![ConstructionSite::pylon(center_point)])
            .collect()
    }

    pub fn pylon_pattern(&self, location: Point2) -> Vec<ConstructionSite> {
        Self::pylon_interceptor(location)
    }

    pub fn find_and_destroy_building(&mut self, building: &Tag) -> Result<(), BuildError> {
        self.sites
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
    pub fn check_morph_gateways(&mut self, warpgates: Units) -> Vec<BuildingTransitionError> {
        let gateways = self.sites.values_mut().filter_map(|bl| match bl.status {
            BuildingStatus::Built(
                Tag {
                    tag,
                    unit_type: UnitTypeId::Gateway,
                },
                _,
            ) => Some((bl, tag)),
            _ => None,
        });
        let changed: Vec<BuildingTransitionError> = gateways
            .filter(|(_, gw)| warpgates.contains_tag(*gw))
            .map(|(bl, _)| bl.transition(BuildingTransition::Morph(UnitTypeId::WarpGate)))
            .filter_map(std::result::Result::err)
            .collect();

        changed
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
    pub fn build(&mut self, structure_type: UnitTypeId) -> Result<(), BuildError> {
        let size = SlotSize::from(structure_type)?;
        //self.game_data.units[structure_type]

        let position = self
            .siting_director
            .get_available_building_site_prioritized(size, structure_type, |a, b| {
                a.location()
                    .distance(self.start_location)
                    .total_cmp(&b.location().distance(self.start_location))
            })
            .ok_or(BuildError::NoPlacementLocations)?;

        let builder = self
            .units
            .my
            .workers
            .closest(position.location())
            .ok_or(BuildError::NoTrainer)?
            .clone();

        let builder_tag = builder.tag();
        let builder_conscripted = self.mining_manager.remove_miner(builder_tag);

        builder.build(structure_type, position.location(), false);
        if builder_conscripted {
            self.log_error(format!(
                "took builder {builder_tag} from mining to build {structure_type:?}"
            ));
        }
        Ok(())
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
            UnitTypeId::WarpPrismPhasing | UnitTypeId::WarpPrism => PRISM_POWER_RADIUS,
            _ => 0.0,
        };
        let change_type = if turned_on {
            BuildingTransition::RePower
        } else {
            BuildingTransition::DePower
        };

        let errors: Vec<BuildingTransitionError> = self
            .siting_director
            .sites
            .values_mut()
            .filter_map(|bl| {
                if bl.location().distance(power_point) <= change_radius {
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

    pub fn update_building_obstructions(
        &mut self,
    ) -> Vec<Either<BuildError, BuildingTransitionError>> {
        // a site is worth checking for this update if it's blocked or its free, constructing and built locations shouldn't be checked
        let worth_checking = self.siting_director.sites.iter().filter_map(|(p, bl)| {
            bl.placement_checker()
                .map(|checker| (p.to_owned(), checker))
        });

        // then we check if those locations are actually obstructed
        #[allow(clippy::needless_collect)]
        let changes: Vec<(Point2, BuildingTransition)> = worth_checking
            .into_iter()
            .map(|(point, checker)| (point, self.location_is_obstructed(point, checker)))
            .collect();

        // then we update their status based on our findings
        changes
            .into_iter()
            .filter_map(|(point, make_blocked)| {
                self.siting_director
                    .mark_position_blocked(point, make_blocked)
                    .err()
            })
            .collect()
    }

    fn location_is_obstructed(
        &self,
        point: Point2,
        structure_type: UnitTypeId,
    ) -> BuildingTransition {
        // ask the game if we can place this building here
        let result = self
            .query_placement(
                vec![(
                    self.game_data.units[&structure_type].ability.unwrap(),
                    point,
                    None,
                )],
                false,
            )
            .unwrap()[0];

        // Success: great.
        // If the error we get is no power, i think that means it's ok.
        if result == ActionResult::Success {
            BuildingTransition::UnObstruct
        }
        // this seems to be the catchall result
        else if result == ActionResult::CantBuildLocationInvalid
            || result == ActionResult::CantBuildTooFarFromBuildPowerSource
        {
            BuildingTransition::Obstruct
        } else {
            // this is a new result that I haven't seen before
            println!("Location {point:?} for {structure_type:?} is blocked: {result:?}");
            BuildingTransition::Obstruct
        }
    }

    /// Assigns a worker an available resource.
    pub fn back_to_work(&mut self, worker: u64) {
        if let Err(e) = self.mining_manager.assign_miner(worker) {
            self.log_error(format!(
                "Can't employ worker: {:?}| {:?}",
                e,
                self.mining_manager.saturation()
            ));
        }
    }

    /// When a new base finishes, we want to make a new Base Manager for it.
    /// Add the resources and existing buildings, if any.
    /// # Errors
    /// `BuildError::NoBuildingLocationHere` if the base isn't on an expansion location
    pub fn new_base_finished(&mut self, nexus: &Unit) -> Result<(), BuildError> {
        self.mining_manager.add_townhall(nexus).map_err(|e| {
            if let MiningError::NotTownhall(tag) = e {
                BuildError::InvalidUnit(format!("{tag:?} is not a townhall"))
            } else {
                BuildError::InvalidUnit(format!("new error from finising a base: {e:?}"))
            }
        })
    }
    /// Finds a gas to take at the specified base and builds it
    /// # Errors
    /// `BuildError::NoPlacementLocations` when there's no geysers free at this base
    /// `BuildError::NoBuildingLocationHere` when this isn't an expansion location
    pub fn take_gas(&self, near: Point2) -> Result<(), BuildError> {
        let gas = self
            .siting_director
            .get_free_geyser(near, ACCEPTABLE_GAS_DISTANCE)
            .ok_or(BuildError::NoPlacementLocations)?;

        let builder = self
            .units
            .my
            .workers
            .closest(gas.location())
            .ok_or(BuildError::NoTrainer)?;

        if let LocationType::OnGeyser(geyser, _point) = gas.location {
            builder.build_gas(geyser, false);
            builder.sleep(5);
            Ok(())
        } else {
            Err(BuildError::CantPlace(
                gas.location(),
                UnitTypeId::Assimilator,
            ))
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const ORIGIN: Point2 = Point2 { x: 0.0, y: 0.0 };

    #[test]
    fn pylon_flower_makes_five() {
        assert_eq!(
            SitingDirector::pylon_flower(Point2 { x: 0.0, y: 0.0 }).len(),
            5
        );
    }

    #[test]
    fn pylon_transitions_ok() {
        let mut pylon = ConstructionSite::pylon(Point2::new(0.0, 0.0));

        assert!(!pylon.needs_power());
        assert!(pylon.is_free());
        assert!(pylon.status.can_build(UnitTypeId::Pylon));

        let _ = pylon.transition(BuildingTransition::Construct(Tag {
            tag: 1,
            unit_type: UnitTypeId::Pylon,
        }));

        assert!(pylon.is_mine());

        assert!(pylon.transition(BuildingTransition::Finish).is_ok());

        assert!(pylon.transition(BuildingTransition::Destroy).is_ok());

        assert!(pylon.is_free());
    }

    #[test]
    fn cant_build_unpowered_gateway() {
        let mut gate_location = ConstructionSite::standard(ORIGIN);

        assert!(gate_location
            .transition(BuildingTransition::Construct(Tag {
                tag: 2,
                unit_type: UnitTypeId::Gateway,
            }))
            .is_err());

        assert!(gate_location
            .transition(BuildingTransition::RePower)
            .is_ok());

        assert!(gate_location.could_build());
    }

    #[test]
    fn point_containers_ok() {
        let pylon_location = ConstructionSite::pylon(ORIGIN);
        let points: Vec<(u32, u32)> = pylon_location.size().contained_points(ORIGIN).collect();
        assert_eq!(points, vec![(0, 0), (0, 1), (1, 0), (1, 1)]);

        let gate_location = ConstructionSite::standard(Point2::new(1.5, 1.5));
        let gate_points: Vec<(u32, u32)> = gate_location
            .size()
            .contained_points(gate_location.location())
            .collect();
        assert_eq!(
            gate_points,
            vec![
                (0, 0),
                (0, 1),
                (0, 2),
                (1, 0),
                (1, 1),
                (1, 2),
                (2, 0),
                (2, 1),
                (2, 2)
            ]
        );
    }

    #[test]
    fn pylon_interceptor_points_ok() {
        let pattern = SitingDirector::pylon_interceptor(Point2::new(20.0, 20.0));
        let mut touch_counts: HashMap<(u32, u32), usize> = HashMap::new();

        for bl in pattern {
            let contained_points = bl.size().contained_points(bl.location());
            for (x, y) in contained_points {
                touch_counts
                    .entry((x, y))
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
            }
        }
        touch_counts.retain(|_, v| v > &mut 1);
        assert!(touch_counts.is_empty(), "{touch_counts:?}");
    }
}
