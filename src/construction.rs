use rust_sc2::ids::UnitTypeId;

use crate::{protoss_bot::ReBiCycler, Tag};

impl ReBiCycler {
    pub fn complete_construction(&mut self, building_tag: u64) {
        let Some(building) = self.units.my.structures.get(building_tag) else {
            println!("ConstructionComplete but unit not found! {building_tag}");
            return;
        };
        println!(
            "Building Finished! {:?}, {building_tag}",
            building.type_id()
        );
        let building = building.clone();
        if let Err(e) = self.siting_director.finish_construction(&building) {
            println!("Error finishing building: {e:?}");
        };

        if building.type_id() == UnitTypeId::Nexus {
            if let Err(e) = self.new_base_finished(&building.clone()) {
                println!("BaseManager failed to initialize: {e:?}");
            }
        } else if building.type_id() == UnitTypeId::Pylon {
            self.update_building_power(UnitTypeId::Pylon, building.position(), true);
        } else if crate::is_assimilator(building.type_id()) {
            let bc = building.clone();
            if let Err(e) = self.mining_manager.add_resource(bc) {
                println!("I expected this to be harvestable: {e:?}");
            };
        }
    }

    pub fn start_construction(&mut self, building_tag: u64) {
        let Some(building) = self.units.my.structures.get(building_tag).cloned() else {
            println!("ConstructionStarted but building not found! {building_tag}");
            return;
        };
        println!("New Building! {:?}, {building_tag}", building.type_id());
        let tag = Tag::from_unit(&building);

        if (building.type_id() == UnitTypeId::Assimilator)
            | (building.type_id() == UnitTypeId::AssimilatorRich)
        {
            if let Err(problem) = self.siting_director.add_assimilator(&building) {
                println!("Nowhere could place the assimilator we just started. {problem:?}");
            }
        } else if let Err(e) = self
            .siting_director
            .construction_begin(tag, building.position())
        {
            println!("No slot for new building: {e:?}");
        }
    }
}
