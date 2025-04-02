use crate::errors::{BuildError, UnitEmploymentError};
use crate::protoss_bot::ReBiCycler;
use crate::siting::{BuildingStatus, PylonPower};
use crate::{closest_index, Tag};
use rust_sc2::bot::Expansion;
use rust_sc2::prelude::*;

pub struct GasLocation {
    pub geyser_tag: u64,
    pub location: Point2,
    status: crate::siting::BuildingStatus,
}

impl GasLocation {
    fn from_unit(unit: &Unit) -> Self {
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
}

pub struct BaseManager {
    pub nexus: BuildingStatus,
    pub name: String,
    pub location: Point2,
    workers: Vec<Tag>,
    minerals: Vec<Tag>,
    geysers: Vec<GasLocation>,
}
impl From<BaseManager> for Point2 {
    fn from(val: BaseManager) -> Self {
        val.location
    }
}

impl BaseManager {
    ///Called when a new base finshes. We don't want to manage a base if we haven't expanded there.
    pub fn new(bot: &ReBiCycler, nexus: &Unit, expansion: &Expansion, name: String) -> Self {
        Self {
            nexus: BuildingStatus::Built(Tag::from_unit(nexus), PylonPower::Depowered),
            location: expansion.loc,
            name,
            workers: Vec::new(),
            minerals: expansion
                .minerals
                .iter()
                .map(|a| bot.units.resources.get(*a))
                .filter_map(|u| Some(Tag::from_unit(u?)))
                .collect(),
            geysers: expansion
                .geysers
                .iter()
                .map(|a| bot.units.resources.get(*a))
                .filter_map(|u| Some(GasLocation::from_unit(u?)))
                .collect(),
        }
    }

    /// Adds a unit to be monitored by this base manager.
    /// Note that resources are added at initialization, no resources should be added after that.
    pub fn assign_unit(&mut self, unit: &Unit) -> Result<(), UnitEmploymentError> {
        let unit_tag = Tag::from_unit(unit);

        match unit_tag.type_id {
            UnitTypeId::Probe => self.workers.push(unit_tag),

            _ => {
                return Err(UnitEmploymentError(
                    "Unable to employ unit_tag at BaseManager".to_string(),
                ))
            }
        }
        Ok(())
    }

    pub fn unassign_unit(&mut self, unit_tag: &Tag) -> Result<(), UnitEmploymentError> {
        match unit_tag.type_id {
            UnitTypeId::Nexus => {
                self.nexus = BuildingStatus::Free(
                    Some(UnitTypeId::Nexus),
                    crate::siting::PylonPower::Depowered,
                );
            }
            UnitTypeId::Probe => self.workers.retain(|x| x != unit_tag),
            UnitTypeId::MineralField => self.minerals.retain(|x| x != unit_tag),
            UnitTypeId::MineralField750 => self.minerals.retain(|x| x != unit_tag),
            UnitTypeId::Assimilator | UnitTypeId::AssimilatorRich => {
                self.lose_assimilator(*unit_tag)?;
            }

            _ => {
                return Err(UnitEmploymentError(
                    "Unable to employ unit_tag at BaseManager".to_string(),
                ))
            }
        }
        Ok(())
    }

    pub fn add_building(&mut self, building: &Unit) -> Result<(), BuildError> {
        use UnitTypeId::{Assimilator, AssimilatorRich, Nexus};
        let tag = Tag::from_unit(building);
        match building.type_id() {
            Assimilator | AssimilatorRich => self.add_assimilator(building),
            Nexus => {
                self.nexus = BuildingStatus::Built(tag, crate::siting::PylonPower::Depowered);
                Ok(())
            }
            _ => Err(BuildError::InvalidUnit(format!(
                "Not a unit that can be assigned to a BaseManager: {:?}",
                building.type_id()
            ))),
        }
    }

    fn add_assimilator(&mut self, building: &Unit) -> Result<(), BuildError> {
        let geyser = self
            .geysers
            .iter_mut()
            .find(|gl| gl.location == building.position())
            .ok_or_else(|| BuildError::NoBuildingLocationHere(building.position()))?;
        geyser.status = BuildingStatus::Built(Tag::from_unit(building), PylonPower::Depowered);
        Ok(())
    }

    fn lose_assimilator(&mut self, building: Tag) -> Result<(), UnitEmploymentError> {
        let geyser = self
            .geysers
            .iter_mut()
            .find(|gl| gl.is_here(&building))
            .ok_or_else(|| {
                UnitEmploymentError(format!(
                    "We didn't have a built geyser with this tag: {building:?}",
                ))
            })?;
        geyser.status = BuildingStatus::Free(Some(UnitTypeId::Assimilator), PylonPower::Depowered);
        Ok(())
    }

    pub fn get_free_geyser(&self) -> Option<&GasLocation> {
        self.geysers
            .iter()
            .find(|gl| gl.status.can_build(&UnitTypeId::Assimilator))
    }
}

impl ReBiCycler {
    /// Assigns a worker to the nearest base.
    ///
    /// # Errors
    /// `UnitEmploymentError` if no base managers exist, or we have no townhalls.
    pub fn reassign_worker_to_nearest_base(
        &mut self,
        worker: &Unit,
    ) -> Result<(), UnitEmploymentError> {
        self.base_managers
            .iter_mut()
            .find(|bm| bm.nexus.is_mine())
            .map_or_else(
                || Err(UnitEmploymentError("No base managers exist!".to_string())),
                |bm| bm.assign_unit(worker),
            )
    }
    /// Find the nearest `BaseManager` to a point, if we have any.
    pub fn get_closest_base_manager(&mut self, position: Point2) -> Option<&mut BaseManager> {
        if self.base_managers.is_empty() {
            return None;
        }
        let bm_points = self.base_managers.iter().map(|bm| bm.location);
        let nearest_bm = closest_index(position, bm_points);
        match nearest_bm {
            Some(index) => Some(&mut self.base_managers[index]),
            None => None,
        }
    }

    /// When a new base finishes, we want to make a new Base Manager for it.
    /// Add the resources and existing buildings, if any.
    /// # Errors
    /// `BuildError::NoBuildingLocationHere` if the base isn't on an expansion location
    pub fn new_base_finished(&mut self, nexus: &Unit) -> Result<(), BuildError> {
        let nexus_position = nexus.position();
        let mut bm = BaseManager::new(
            self,
            nexus,
            self.expansions
                .iter()
                .find(|e| e.loc == nexus_position)
                .ok_or(BuildError::NoBuildingLocationHere(nexus_position))?,
            format!("Expansion {}", self.counter().count(UnitTypeId::Nexus)),
        );
        for building in self.units.my.structures.iter().closer(15.0, nexus_position) {
            bm.add_building(building);
        }

        self.base_managers.push(bm);
        Ok(())
    }
    /// Finds a gas to take at the specified base and builds it
    /// # Errors
    /// `BuildError::NoPlacementLocations` when there's no geysers free at this base
    /// `BuildError::NoBuildingLocationHere` when this isn't an expansion location
    pub fn take_gas(&self, at_base: Point2) -> Result<(), BuildError> {
        let base = self
            .base_managers
            .iter()
            .find(|bm| bm.location == at_base)
            .ok_or(BuildError::NoBuildingLocationHere(at_base))?;
        let gas = base.get_free_geyser();
        if let Some(geyser) = gas {
            let builder = self
                .units
                .my
                .workers
                .closest(geyser.location)
                .ok_or(BuildError::NoTrainer)?;
            builder.build_gas(geyser.geyser_tag, false);
            builder.sleep(5);
            println!("Build command sent: Assimilator");
            Ok(())
        } else {
            Err(BuildError::NoPlacementLocations)
        }
    }
}
