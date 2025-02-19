use rust_sc2::prelude::*;
use std::fmt::Debug;

#[derive(Debug, PartialEq)]
pub struct Tag(u64, UnitTypeId);

#[bot]
#[derive(Default)]
pub struct ReBiCycler {}
impl Player for ReBiCycler {
    fn get_player_settings(&self) -> PlayerSettings {
        PlayerSettings::new(Race::Protoss).raw_crop_to_playable_area(true)
    }
    fn on_start(&mut self) -> SC2Result<()> {

        
        let main_nexus = self.units.my.townhalls.first().unwrap();
        let main_tag = Tag(self.units.my.townhalls.first().unwrap().tag(), UnitTypeId::Nexus);
        let mut bm = BaseManager::new(main_tag);
        for worker in &self.units.my.workers {
            bm.assign_unit(Tag(worker.tag(), worker.type_id())).unwrap();
            worker.attack(Target::Tag(bm.nexus().as_ref().unwrap().0), false);
        }
        println!("Game start!");
        println!("Main Nexus has {:?} workers assigned.", bm.workers().len());
        Ok(())
    }
    
    fn on_step(&mut self, frame_no: usize) -> SC2Result<()> {
        self.step_build();
        //self.micro();
        //println!("Step step step {}", frame_no);
        Ok(())
    }
}


impl ReBiCycler {
    pub fn new() -> Self {
        Self {
            /* initializing fields */
            ..Default::default()
        }
    }


    fn step_build(&mut self) {
        let idle_nexi = self.units.my.townhalls.idle();
        for nexus in idle_nexi{
            nexus.train(UnitTypeId::Probe, false);
        }
    }


}


pub struct UnitEmploymentError(String);
impl Debug for UnitEmploymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error in employment: {}", self.0)
    }
}


pub struct BaseManager {
    nexus: Option<Tag>,
    workers: Vec<Tag>,
    minerals: Vec<Tag>,
    geysers: Vec<Tag>,
    assimilators: Vec<Tag>
    
}

impl BaseManager {
    pub fn new(nexus: Tag) -> Self{

        BaseManager{
            nexus: Some(nexus),
            workers: Vec::new(),
            minerals: Vec::new(),
            geysers: Vec::new(),
            assimilators: Vec::new()
        }
    }

    pub fn nexus(&self) -> &Option<Tag>{
        &self.nexus
    }

    pub fn workers(&self) -> &Vec<Tag>{
        &self.workers
    }

    pub fn minerals(&self) -> &Vec<Tag>{
        &self.minerals

    }

    pub fn geysers(&self) -> &Vec<Tag> {
        &self.geysers
    }

    pub fn assimilators(&self) -> &Vec<Tag>{
        &self.assimilators
    }


    pub fn assign_unit(&mut self, unit: Tag) -> Result<(), UnitEmploymentError>{
        match unit {
            Tag(_, UnitTypeId::Nexus) => self.nexus = Some(unit),
            Tag(_, UnitTypeId::Probe) => self.workers.push(unit),
            Tag(_, UnitTypeId::MineralField) => self.minerals.push(unit),
            Tag(_, UnitTypeId::MineralField750) => self.minerals.push(unit),
            Tag(_, UnitTypeId::VespeneGeyser) => self.geysers.push(unit),
            Tag(_, UnitTypeId::Assimilator) => self.assimilators.push(unit),

            _ => return Err(UnitEmploymentError("Unable to employ unit at BaseManager".to_string()))
        }
        Ok(())
    }

    pub fn unassign_unit(&mut self, unit: Tag) -> Result<(), UnitEmploymentError>{
        match unit {
            Tag(_, UnitTypeId::Nexus) => self.nexus = None,
            Tag(_, UnitTypeId::Probe) => self.workers.retain(|x| *x != unit),
            Tag(_, UnitTypeId::MineralField) => self.minerals.retain(|x| *x != unit),
            Tag(_, UnitTypeId::MineralField750) => self.minerals.retain(|x| *x != unit),
            Tag(_, UnitTypeId::VespeneGeyser) => self.geysers.retain(|x| *x != unit),
            Tag(_, UnitTypeId::Assimilator) => self.assimilators.retain(|x| *x != unit),

            _ => return Err(UnitEmploymentError("Unable to employ unit at BaseManager".to_string()))
        }
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    fn init_base_manager()-> BaseManager{
        let nexus = Tag(1, UnitTypeId::Nexus);
        let mut bm = BaseManager::new(nexus);

        bm.assign_unit(Tag(2, UnitTypeId::Probe)).unwrap();
        bm.assign_unit(Tag(3, UnitTypeId::MineralField750)).unwrap();
        bm.assign_unit(Tag(4, UnitTypeId::VespeneGeyser)).unwrap();
        bm.assign_unit(Tag(5, UnitTypeId::Assimilator)).unwrap();

        bm

    }
    #[test]
    fn base_manager_assigns_units_properly(){
        let mut bm = BaseManager::new(Tag(1,UnitTypeId::Nexus));
        bm.assign_unit(Tag(2, UnitTypeId::Probe)).unwrap();
        bm.assign_unit(Tag(3, UnitTypeId::MineralField750)).unwrap();
        bm.assign_unit(Tag(4, UnitTypeId::VespeneGeyser)).unwrap();
        bm.assign_unit(Tag(5, UnitTypeId::Assimilator)).unwrap();

        assert_eq!(bm.nexus(), &Some(Tag(1,UnitTypeId::Nexus)));
        assert_eq!(bm.workers(), &vec![Tag(2, UnitTypeId::Probe)]);
        assert_eq!(bm.minerals(), &vec![Tag(3, UnitTypeId::MineralField750)]);
        assert_eq!(bm.geysers(), &vec![Tag(4, UnitTypeId::VespeneGeyser)]);
        assert_eq!(bm.assimilators(), &vec![Tag(5, UnitTypeId::Assimilator)]);
    }

    #[test]
    fn base_manager_surrenders_units_properly() {

        let mut bm = init_base_manager();

        bm.unassign_unit(Tag(1, UnitTypeId::Nexus)).unwrap();
        bm.unassign_unit(Tag(2, UnitTypeId::Probe)).unwrap();
        bm.unassign_unit(Tag(3, UnitTypeId::MineralField750)).unwrap();
        bm.unassign_unit(Tag(4, UnitTypeId::VespeneGeyser)).unwrap();
        bm.unassign_unit(Tag(5, UnitTypeId::Assimilator)).unwrap();

        assert_eq!(bm.nexus(), &None);
        assert!(bm.workers().is_empty());
        assert!(bm.minerals().is_empty());
        assert!(bm.geysers().is_empty());
        assert!(bm.assimilators().is_empty());

    }

}