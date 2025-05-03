use std::fmt::Display;

use rust_sc2::{ids::UnitTypeId, prelude::DistanceIterator, unit::Unit};

use crate::{
    errors::{AssignmentError, AssignmentIssue, BuildingTransitionError},
    protoss_bot::ReBiCycler,
    siting::ConstructionSite,
    Assigns, Tag,
};

pub struct ConstructionManager {
    active_projects: Vec<ConstructionProject>,
}

pub struct ConstructionProject {
    building: UnitTypeId,
    location: ConstructionSite,
    builder: Option<u64>,
    needs_detector: bool,
    detector: Option<u64>,
    needs_clearing: bool,
    clearing_crew: Vec<u64>,
}

impl ConstructionProject {
    pub fn new(building: UnitTypeId, location: ConstructionSite) -> Self {
        Self {
            building,
            location,
            builder: None,
            needs_detector: false,
            detector: None,
            needs_clearing: false,
            clearing_crew: Vec::new(),
        }
    }
}
impl Display for ConstructionProject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Construction[{:?} @ {}]", self.building, self.location)
    }
}

impl Assigns for ConstructionProject {
    fn assign(&mut self, unit: Tag) -> Result<(), crate::errors::AssignmentError> {
        let issue = match unit.unit_type {
            UnitTypeId::Probe => {
                if self.builder.is_none() {
                    self.builder = Some(unit.tag);
                    Ok(())
                } else {
                    Err(AssignmentIssue::DifferentUnitAssignedInRole)
                }
            }
            UnitTypeId::Observer => {
                if self.detector.is_none() {
                    self.detector = Some(unit.tag);
                    Ok(())
                } else {
                    Err(AssignmentIssue::DifferentUnitAssignedInRole)
                }
            }
            _ => {
                self.clearing_crew.push(unit.tag);
                Ok(())
            }
        };
        issue.map_err(|i| AssignmentError::new(unit, self.to_string(), i))
    }

    fn remove(&mut self, unit: Tag) -> Result<(), AssignmentError> {
        let issue = match unit.unit_type {
            UnitTypeId::Probe => {
                if let Some(builder) = self.builder {
                    if builder == unit.tag {
                        self.builder = None;
                        Ok(())
                    } else {
                        Err(AssignmentIssue::DifferentUnitAssignedInRole)
                    }
                } else {
                    Err(AssignmentIssue::UnitNotAssigned)
                }
            }
            UnitTypeId::Observer => {
                if let Some(detector) = self.detector {
                    if detector == unit.tag {
                        self.detector = None;
                        Ok(())
                    } else {
                        Err(AssignmentIssue::DifferentUnitAssignedInRole)
                    }
                } else {
                    Err(AssignmentIssue::UnitNotAssigned)
                }
            }

            _ => {
                if self.clearing_crew.contains(&unit.tag) {
                    self.clearing_crew.retain(|tag| *tag != unit.tag);
                    Ok(())
                } else {
                    Err(AssignmentIssue::UnitNotAssigned)
                }
            }
        };
        issue.map_err(|i| AssignmentError::new(unit, self.to_string(), i))
    }
}

impl ReBiCycler {
    /// tells a worker to build buildimg at location.
    /// marks the resources as spent and adds the construction to the queue
    fn queue_construction(
        &mut self,
        builder: Unit,
        building: UnitTypeId,
        location: ConstructionSite,
    ) -> Result<(), BuildingTransitionError> {
    }

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

    pub fn maintain_supply(&mut self) {
        if self.supply_cap == 200 {
            return;
        }
        let production_structures = self
            .units
            .my
            .structures
            .iter()
            .filter(|u| crate::is_protoss_production(&u.type_id()))
            .count();

        let producing_workers = self.counter().ordered().count(UnitTypeId::Probe) > 0;

        let wanted_free_supply = production_structures * 2 + if producing_workers { 2 } else { 0 };

        if self.supply_left >= wanted_free_supply as u32 {
            return;
        }

        let ordered_pylons = self.counter().ordered().count(UnitTypeId::Pylon);
        let almost_done_nexi = self
            .units
            .my
            .townhalls
            .iter()
            .filter(|u| !u.is_ready() && u.is_almost_ready())
            .count();

        let over_supply = self.supply_used > self.supply_cap;
        todo!()
    }
}
