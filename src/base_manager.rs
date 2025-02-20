use rust_sc2::prelude::*;
use crate::{Tag, UnitEmploymentError};

pub struct BaseManager{
    pub nexus: Option<Tag>,
    workers: Vec<Tag>,
    minerals: Vec<Tag>,
    geysers: Vec<Tag>,
    assimilators: Vec<Tag>,
}

impl BaseManager {
    pub fn new(nexus: Tag) -> Self {
        BaseManager {
            nexus: Some(nexus),
            workers: Vec::new(),
            minerals: Vec::new(),
            geysers: Vec::new(),
            assimilators: Vec::new(),
        }
    }

    pub fn nexus(&self) -> &Option<Tag> {
        &self.nexus
    }

    pub fn workers(&self) -> &Vec<Tag> {
        &self.workers
    }

    pub fn minerals(&self) -> &Vec<Tag> {
        &self.minerals
    }

    pub fn geysers(&self) -> &Vec<Tag> {
        &self.geysers
    }

    pub fn assimilators(&self) -> &Vec<Tag> {
        &self.assimilators
    }

    pub fn assign_unit(&mut self, unit_tag: Tag) -> Result<(), UnitEmploymentError> {
        println!("Assigning new unit_tag to base manager: {:?}", unit_tag);
        match unit_tag.type_id {
            UnitTypeId::Nexus => self.nexus = Some(unit_tag),
            UnitTypeId::Probe => self.workers.push(unit_tag),
            UnitTypeId::MineralField => self.minerals.push(unit_tag),
            UnitTypeId::MineralField750 => self.minerals.push(unit_tag),
            UnitTypeId::VespeneGeyser => self.geysers.push(unit_tag),
            UnitTypeId::Assimilator => self.assimilators.push(unit_tag),

            _ => {
                return Err(UnitEmploymentError(
                    "Unable to employ unit_tag at BaseManager".to_string(),
                ))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    fn init_base_manager() -> BaseManager {
        let nexus = Tag{tag: 1, type_id: UnitTypeId::Nexus};
        let mut bm = BaseManager::new(nexus);

        bm.assign_unit(Tag{tag: 2, type_id: UnitTypeId::Probe}).unwrap();
        bm.assign_unit(Tag{tag: 3, type_id: UnitTypeId::MineralField750}).unwrap();
        bm.assign_unit(Tag{tag: 4, type_id: UnitTypeId::VespeneGeyser}).unwrap();
        bm.assign_unit(Tag{tag: 5, type_id: UnitTypeId::Assimilator}).unwrap();

        bm
    }
    #[test]
    fn base_manager_assigns_units_properly() {
        let mut bm = BaseManager::new(Tag{tag: 1, type_id: UnitTypeId::Nexus});
        bm.assign_unit(Tag{tag: 2, type_id: UnitTypeId::Probe}).unwrap();
        bm.assign_unit(Tag{tag: 3, type_id: UnitTypeId::MineralField750}).unwrap();
        bm.assign_unit(Tag{tag: 4, type_id: UnitTypeId::VespeneGeyser}).unwrap();
        bm.assign_unit(Tag{tag: 5, type_id: UnitTypeId::Assimilator}).unwrap();

        assert_eq!(bm.nexus(), &Some(Tag{tag: 1, type_id: UnitTypeId::Nexus}));
        assert_eq!(bm.workers(), &vec![Tag{tag: 2, type_id: UnitTypeId::Probe}]);
        assert_eq!(bm.minerals(), &vec![Tag{tag: 3, type_id: UnitTypeId::MineralField750}]);
        assert_eq!(bm.geysers(), &vec![Tag{tag: 4, type_id: UnitTypeId::VespeneGeyser}]);
        assert_eq!(bm.assimilators(), &vec![Tag{tag: 5, type_id: UnitTypeId::Assimilator}]);
    }

    #[test]
    fn base_manager_surrenders_units_properly() {
        let mut bm = init_base_manager();

        bm.unassign_unit(Tag{tag: 1, type_id: UnitTypeId::Nexus}).unwrap();
        bm.unassign_unit(Tag{tag: 2, type_id: UnitTypeId::Probe}).unwrap();
        bm.unassign_unit(Tag{tag: 3, type_id: UnitTypeId::MineralField750})
            .unwrap();
        bm.unassign_unit(Tag{tag: 4, type_id: UnitTypeId::VespeneGeyser}).unwrap();
        bm.unassign_unit(Tag{tag: 5, type_id: UnitTypeId::Assimilator}).unwrap();

        assert_eq!(bm.nexus(), &None);
        assert!(bm.workers().is_empty());
        assert!(bm.minerals().is_empty());
        assert!(bm.geysers().is_empty());
        assert!(bm.assimilators().is_empty());
    }
}
