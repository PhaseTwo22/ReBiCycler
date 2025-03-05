use crate::errors::{BuildError, UnitEmploymentError};
use crate::protoss_bot::ReBiCycler;
use crate::{closest_index, Tag};
use rust_sc2::bot::Expansion;
use rust_sc2::prelude::*;

pub struct BaseManager {
    pub nexus: Option<Tag>,
    pub name: String,
    pub location: Point2,
    workers: Vec<Tag>,
    minerals: Vec<Tag>,
    geysers: Vec<Tag>,
    assimilators: Vec<Tag>,
}
impl From<BaseManager> for Point2 {
    fn from(val: BaseManager) -> Self {
        val.location
    }
}

impl BaseManager {
    pub fn new(bot: &ReBiCycler, expansion: &Expansion, name: String) -> Self {
        let base_tag = Self::base_tag(expansion);
        Self {
            nexus: base_tag,
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
                .filter_map(|u| Some(Tag::from_unit(u?)))
                .collect(),
            assimilators: Vec::new(),
        }
    }

    pub fn base_tag(expansion: &Expansion) -> Option<Tag> {
        if expansion.alliance.is_mine() {
            Some(Tag {
                tag: expansion.base?,
                type_id: UnitTypeId::Nexus,
            })
        } else {
            None
        }
    }

    pub fn assign_unit(&mut self, unit: &Unit) -> Result<(), UnitEmploymentError> {
        let unit_tag = Tag::from_unit(unit);

        if unit.is_mineral() {
            self.minerals.push(unit_tag);
        } else if unit.is_geyser() {
            self.geysers.push(unit_tag);
        } else {
            match unit_tag.type_id {
                UnitTypeId::Nexus => self.nexus = Some(unit_tag),
                UnitTypeId::Probe => self.workers.push(unit_tag),
                UnitTypeId::Assimilator => self.assimilators.push(unit_tag),

                _ => {
                    return Err(UnitEmploymentError(
                        "Unable to employ unit_tag at BaseManager".to_string(),
                    ))
                }
            }
        }
        Ok(())
    }

    pub fn unassign_unit(&mut self, unit_tag: &Tag) -> Result<(), UnitEmploymentError> {
        match unit_tag.type_id {
            UnitTypeId::Nexus => self.nexus = None,
            UnitTypeId::Probe => self.workers.retain(|x| x != unit_tag),
            UnitTypeId::MineralField => self.minerals.retain(|x| x != unit_tag),
            UnitTypeId::MineralField750 => self.minerals.retain(|x| x != unit_tag),
            UnitTypeId::VespeneGeyser => self.geysers.retain(|x| x != unit_tag),
            UnitTypeId::Assimilator => self.assimilators.retain(|x| x != unit_tag),

            _ => {
                return Err(UnitEmploymentError(
                    "Unable to employ unit_tag at BaseManager".to_string(),
                ))
            }
        }
        Ok(())
    }

    pub fn add_building(&mut self, building: &Unit) {
        use UnitTypeId::{Assimilator, AssimilatorRich, Nexus};
        let tag = Tag::from_unit(building);
        match building.type_id() {
            Assimilator | AssimilatorRich => self.assimilators.push(tag),
            Nexus => self.nexus = Some(tag),
            _ => (),
        };
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
        let nearest_nexus = self
            .units
            .my
            .townhalls
            .iter()
            .closest(worker)
            .ok_or_else(|| UnitEmploymentError("No nexi exist!".to_string()))?;

        let nn_tag = Tag::from_unit(nearest_nexus);
        self.base_managers
            .iter_mut()
            .find(|bm| bm.nexus == Some(nn_tag.clone()))
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
    pub fn new_base_finished(&mut self, position: Point2) -> Result<(), BuildError> {
        let mut bm = BaseManager::new(
            self,
            self.expansions
                .iter()
                .find(|e| e.loc == position)
                .ok_or(BuildError::NoBuildingLocationHere(position))?,
            format!("Expansion {}", self.counter().count(UnitTypeId::Nexus)),
        );

        for resource in self.units.resources.iter().closer(10.0, position) {
            if bm.assign_unit(resource).is_err() {
                println!(
                    "Tried to assign unexpected unit to base: {:?}",
                    resource.type_id()
                );
            }
        }

        for building in self.units.my.structures.iter().closer(15.0, position) {
            bm.add_building(building);
        }

        self.base_managers.push(bm);
        Ok(())
    }
}
