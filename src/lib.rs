use rust_sc2::prelude::*;

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
        for worker in &self.units.my.workers {
            worker.attack(Target::Pos(self.enemy_start), false);
        }
        

        let main_tag = Tag(self.units.my.townhalls.first().unwrap().tag(), UnitTypeId::Nexus);
        //BaseManager::new(nexus: main_tag)
        println!("Game start!");
        Ok(())
    }
    
    fn on_step(&mut self, frame_no: usize) -> SC2Result<()> {
        self.step_build();
        //self.micro();
        println!("Step step step {}", frame_no);
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

    fn step_build(&mut self) -> () {
        let idle_nexi = self.units.my.townhalls.idle();
        for nexus in idle_nexi{
            nexus.train(UnitTypeId::Probe, false);
        }
    }
}

pub struct UnitEmploymentError(String);

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
            _ => Ok(())
        }
    }
}
#[cfg(tests)]
mod tests {
    
    fn init_base_manager()-> BaseManager{
        let nexus = Tag(1, UnitTypeId::Nexus);
        let mut bm = BaseManager::new(nexus);

        bm.assign_unit(Tag(2, UnitTypeId::Probe));
        bm.assign_unit(Tag(3, UnitTypeId::MineralPatch750));
        bm.assign_unit(Tag(4, UnitTypeId::GasGeyser));
        bm.assign_unit(Tag(5, UnitTypeId::Assimilator));

        bm

    }
    #[test]
    fn base_manager_assigns_units_properly(){
        let bm = BaseManager::new(Tag(1,UnitTypeId::Nexus));
        bm.assign_unit(Tag(2, UnitTypeId::Probe));
        bm.assign_unit(Tag(3, UnitTypeId::MineralField750));
        bm.assign_unit(Tag(4, UnitTypeId::GasGeyser));
        bm.assign_unit(Tag(5, UnitTypeId::Assimilator));
    }

    #[test]
    fn base_manager_surrenders_units_properly() {

        let mut bm = init_base_manager();

        bm.unassign_unit(Tag(1, UnitTypeI::Nexus)).unwrap();
        bm.unassign_unit(Tag(2, UnitTypeId::Probe)).unwrap();
        bm.unassign_unit(Tag(3, UnitTypeId::MineralField750)).unwrap();
        bm.unassign_unit(Tag(4, UnitTypeId::GasGeyser)).unwrap();
        bm.unassign_unit(Tag(5, UnitTypeId::Assimilator)).unwrap();

        assert_eq!(bm.nexus(), None);
        assert!(bm.workers().len() == 0);
        assert!(bm.minerals().len() == 0);
        assert!(bm.geysers().len() == 0);
        assert!(bm.assimilators().len() == 0);

    }

}