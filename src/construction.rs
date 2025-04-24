use rust_sc2::{ids::UnitTypeId, prelude::DistanceIterator, unit::Unit};

use crate::{protoss_bot::ReBiCycler, Tag};

impl ReBiCycler {
    ///transitions a `BuildingLocation` that finished construction to the completed status
    /// also adds new nexuses and assimilators to the mining manager
    pub fn complete_construction(&mut self, building_tag: u64) {
        let Some(building) = self.units.my.structures.get(building_tag) else {
            println!("ConstructionComplete but unit not found! {building_tag}");
            return;
        };
        let building = building.clone();
        if let Err(e) = self.siting_director.finish_construction(&building) {
            self.log_error(format!("Error finishing building: {e:?}"));
        };

        if building.type_id() == UnitTypeId::Nexus {
            if let Err(e) = self.new_base_finished(&building.clone()) {
                self.log_error(format!("Can't add nexus to Mining Manager: {e:?}"));
            }
            let minerals: Vec<Unit> = self
                .units
                .mineral_fields
                .iter()
                .closer(10.0, building)
                .cloned()
                .collect();
            let mut issues = Vec::new();
            for mineral in minerals {
                if let Err(e) = self.mining_manager.add_resource(&mineral) {
                    issues.push(format!("Can't add mineral to Mining Manager: {e:?}"));
                }
            }
            for iss in issues {
                self.log_error(iss);
            }
        } else if building.type_id() == UnitTypeId::Pylon {
            self.update_building_power(UnitTypeId::Pylon, building.position(), true);
        } else if crate::is_assimilator(building.type_id()) {
            let bc = building.clone();
            if let Err(e) = self.mining_manager.add_resource(&bc) {
                println!("Can't mine this: {e:?}");
            };
        }
    }
    /// marks a building location as constructing
    /// and sends all idle workers back to mining
    pub fn start_construction(&mut self, building_tag: u64) {
        let Some(building) = self.units.my.structures.get(building_tag).cloned() else {
            println!("ConstructionStarted but building not found! {building_tag}");
            return;
        };
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

        let _: () = self
            .units
            .my
            .workers
            .idle()
            .iter()
            .map(|worker| {
                self.back_to_work(worker.tag());
                println!("BACK TO WORK!");
            })
            .collect();
    }
}
