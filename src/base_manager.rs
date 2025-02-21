use crate::errors::{InvalidUnitError, UnitEmploymentError};
use crate::siting::SitingManager;
use crate::Tag;
use rust_sc2::prelude::*;

pub struct BaseManager {
    pub nexus: Option<Tag>,
    pub location: Point2,
    workers: Vec<Tag>,
    minerals: Vec<Tag>,
    geysers: Vec<Tag>,
    assimilators: Vec<Tag>,
    pub siting_manager: SitingManager,
}
impl From<BaseManager> for Point2 {
    fn from(val: BaseManager) -> Self {
        val.location
    }
}

impl BaseManager {
    pub fn new(nexus: Option<Tag>, name: String, location: Point2) -> Self {
        Self {
            nexus: nexus.clone(),
            location,
            workers: Vec::new(),
            minerals: Vec::new(),
            geysers: Vec::new(),
            assimilators: Vec::new(),
            siting_manager: SitingManager::new(nexus, name, location),
        }
    }

    pub const fn nexus(&self) -> &Option<Tag> {
        &self.nexus
    }

    pub const fn workers(&self) -> &Vec<Tag> {
        &self.workers
    }

    pub const fn minerals(&self) -> &Vec<Tag> {
        &self.minerals
    }

    pub const fn geysers(&self) -> &Vec<Tag> {
        &self.geysers
    }

    pub const fn assimilators(&self) -> &Vec<Tag> {
        &self.assimilators
    }

    pub fn assign_unit(&mut self, unit: &Unit) -> Result<(), UnitEmploymentError> {
        let unit_tag = Tag::from_unit(unit);
        println!("Assigning new unit_tag to base manager: {unit_tag:?}");

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

    pub fn unassign_unit(&mut self, unit_tag: Tag) -> Result<(), UnitEmploymentError> {
        match unit_tag.type_id {
            UnitTypeId::Nexus => self.nexus = None,
            UnitTypeId::Probe => self.workers.retain(|x| *x != unit_tag),
            UnitTypeId::MineralField => self.minerals.retain(|x| *x != unit_tag),
            UnitTypeId::MineralField750 => self.minerals.retain(|x| *x != unit_tag),
            UnitTypeId::VespeneGeyser => self.geysers.retain(|x| *x != unit_tag),
            UnitTypeId::Assimilator => self.assimilators.retain(|x| *x != unit_tag),

            _ => {
                return Err(UnitEmploymentError(
                    "Unable to employ unit_tag at BaseManager".to_string(),
                ))
            }
        }
        Ok(())
    }

    pub fn add_building(&mut self, building: &Unit) -> Result<(), InvalidUnitError> {
        if let Some(size) = building.building_size() {
            self.siting_manager
                .add_building(Tag::from_unit(building), building.position(), size)
        } else {
            Err(InvalidUnitError(
                "All Protoss buildings have Some(building_size())!".to_string(),
            ))
        }
    }

    pub fn destroy_building_by_tag(&mut self, building: Tag) -> bool {
        self.siting_manager.destroy_building_by_tag(building)
    }
}
