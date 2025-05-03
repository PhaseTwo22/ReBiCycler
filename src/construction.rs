use std::{collections::HashMap, fmt::Display};

use rust_sc2::{
    action::Target,
    ids::UnitTypeId,
    prelude::{DistanceIterator, Point2},
    unit::Unit,
};

use crate::{
    army::MissionType,
    errors::{AssignmentError, AssignmentIssue, BuildError},
    protoss_bot::ReBiCycler,
    siting::{ConstructionSite, LocationType},
    Assigns, Tag,
};

const PROJECT_MAX_LIFETIME: u32 = 23 * 120;
const CONSTRUCTION_RALLY_DISTANCE: f32 = 7.0;

#[derive(Default)]
pub struct ConstructionManager {
    pub active_projects: HashMap<Point2, ConstructionProject>,
}

impl ConstructionManager {
    fn new_project(&mut self, building: UnitTypeId, site: ConstructionSite, current_step: u32) {
        let loc = site.location();
        let project = ConstructionProject::new(building, site, current_step);

        self.active_projects.insert(loc, project);
    }

    pub fn remove_project(&mut self, site: ConstructionSite) {
        self.active_projects.remove(&site.location());
    }

    fn add_babysitters(
        &mut self,
        location: Point2,
        babysitter_mission: usize,
    ) -> Result<(), AssignmentIssue> {
        self.active_projects
            .get_mut(&location)
            .ok_or(AssignmentIssue::InvalidProject)?
            .clearing_crew = Some(babysitter_mission);
        Ok(())
    }

    fn add_detector_mission(
        &mut self,
        location: Point2,
        detector_mission: usize,
    ) -> Result<(), AssignmentIssue> {
        self.active_projects
            .get_mut(&location)
            .ok_or(AssignmentIssue::InvalidProject)?
            .detector = Some(detector_mission);
        Ok(())
    }

    fn get(&self, location: Point2) -> Option<&ConstructionProject> {
        self.active_projects.get(&location)
    }

    fn log_construction_queued(
        &mut self,
        location: Point2,
        queued_state: bool,
    ) -> Result<(), AssignmentIssue> {
        self.active_projects
            .get_mut(&location)
            .ok_or(AssignmentIssue::InvalidProject)?
            .builder_ordered = queued_state;
        Ok(())
    }

    fn add_builder(&mut self, location: Point2, builder: u64) -> Result<(), AssignmentIssue> {
        self.active_projects
            .get_mut(&location)
            .ok_or(AssignmentIssue::InvalidProject)?
            .builder = Some(builder);
        Ok(())
    }
}

pub struct ConstructionProject {
    building: UnitTypeId,
    site: ConstructionSite,
    builder: Option<u64>,
    builder_ordered: bool,
    needs_detector: bool,
    detector: Option<usize>,
    needs_clearing: bool,
    clearing_crew: Option<usize>,
    created_step: u32,
}

impl ConstructionProject {
    pub const fn new(building: UnitTypeId, site: ConstructionSite, current_step: u32) -> Self {
        Self {
            building,
            site,
            builder: None,
            builder_ordered: false,
            needs_detector: false,
            detector: None,
            needs_clearing: false,
            clearing_crew: None,
            created_step: current_step,
        }
    }
}
impl Display for ConstructionProject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Construction[{:?} @ {}]", self.building, self.site)
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
            _ => Err(AssignmentIssue::InvalidUnit),
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
            _ => Err(AssignmentIssue::InvalidUnit),
        };
        issue.map_err(|i| AssignmentError::new(unit, self.to_string(), i))
    }
}

impl ReBiCycler {
    /// Creates a construction project for the construction manager to deal with.
    fn queue_construction(&mut self, building: UnitTypeId, site: ConstructionSite) {
        self.construction_manager
            .new_project(building, site, self.game_step());
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

    pub fn maintain_supply(&mut self) -> Result<(), BuildError> {
        if self.supply_cap == 200 {
            return Ok(());
        }
        let production_structures = self
            .units
            .my
            .structures
            .iter()
            .filter(|u| crate::is_protoss_production(&u.type_id()))
            .count();

        let over_supply = self.supply_used.saturating_sub(self.supply_cap);

        let producing_workers = self.counter().ordered().count(UnitTypeId::Probe) > 0;

        let wanted_free_supply = production_structures * 2 + if producing_workers { 2 } else { 0 };

        if self.supply_left >= wanted_free_supply as u32 {
            return Ok(());
        }

        let ordered_pylons = self.counter().ordered().count(UnitTypeId::Pylon);
        let almost_done_nexi = self
            .units
            .my
            .townhalls
            .iter()
            .filter(|u| !u.is_ready() && u.is_almost_ready())
            .count();

        let pending_new_supply = 8 * ordered_pylons + 15 * almost_done_nexi;

        if pending_new_supply >= (wanted_free_supply + over_supply as usize) {
            return Ok(());
        }

        self.build(UnitTypeId::Pylon)
    }

    pub fn process_construction_projects(&mut self) {
        let needs = self.check_construction_projects();

        for (location, need) in needs.iter() {
            let problem = match need {
                ProjectNeeds::Cancelling | ProjectNeeds::Nothing => Ok(()),
                ProjectNeeds::Cleaners => {
                    let mission_id = self.new_mission(
                        MissionType::BabysitConstruction(*location),
                        location.towards(self.game_info.map_center, CONSTRUCTION_RALLY_DISTANCE),
                    );
                    self.construction_manager
                        .add_babysitters(*location, mission_id)
                }
                ProjectNeeds::Detector => {
                    let mission_id =
                        self.new_mission(MissionType::DetectArea(*location), *location);
                    self.construction_manager
                        .add_detector_mission(*location, mission_id)
                }
                ProjectNeeds::BuilderRallied => {
                    let find_builder = self.rally_builder(*location);
                    match find_builder {
                        Ok(builder) => self.construction_manager.add_builder(*location, builder),
                        Err(issue) => Err(issue),
                    }
                }
                ProjectNeeds::BuilderOrdered(builder) => {
                    let tried_command = self.command_builder(*location, *builder);
                    match tried_command {
                        Ok(queued_up) => self
                            .construction_manager
                            .log_construction_queued(*location, queued_up),
                        Err(issue) => Err(issue),
                    }
                }
            };
            problem.map_err(|issue| {
                self.log_error(format!(
                    "Can't address {:?} at construction at {:?}: {:?}",
                    need, location, issue
                ))
            });
        }
    }

    pub fn check_construction_projects(&self) -> Vec<(Point2, ProjectNeeds)> {
        self.construction_manager
            .active_projects
            .iter()
            .map(|(location, project)| (*location, self.evaluate_construction_project(project)))
            .collect()
    }

    fn evaluate_construction_project(&self, project: &ConstructionProject) -> ProjectNeeds {
        if project.clearing_crew.is_none() && self.knowledge.expansions_need_clearing {
            return ProjectNeeds::Cleaners;
        }
        if project.detector.is_none() && self.knowledge.expansions_need_detectors {
            return ProjectNeeds::Detector;
        }
        if let Some(builder) = project.builder {
            if !project.builder_ordered {
                return ProjectNeeds::BuilderOrdered(builder);
            }
        } else {
            return ProjectNeeds::BuilderRallied;
        }

        if project.created_step + PROJECT_MAX_LIFETIME < self.game_step() {
            return ProjectNeeds::Cancelling;
        }

        return ProjectNeeds::Nothing;
    }

    fn rally_builder(&self, location: Point2) -> Result<u64, AssignmentIssue> {
        let distance_from_project =
            |unit: &&Unit| crate::distance_squared(&unit.position(), &location);

        let closest_miner = self
            .mining_manager
            .employed_miners()
            .flat_map(|tag| self.units.my.workers.get(*tag))
            .min_by(|worker_a, worker_b| {
                distance_from_project(worker_a).total_cmp(&distance_from_project(worker_b))
            });

        let builder = closest_miner.ok_or(AssignmentIssue::NoUnits)?;

        builder.move_to(Target::Pos(location), false);
        Ok(builder.tag())
    }

    fn command_builder(&self, location: Point2, builder: u64) -> Result<bool, AssignmentIssue> {
        let project = self
            .construction_manager
            .get(location)
            .ok_or(AssignmentIssue::InvalidProject)?;

        let builder = self
            .units
            .my
            .workers
            .get(builder)
            .ok_or(AssignmentIssue::InvalidUnit)?;

        if self.can_afford(project.building, false) {
            match project.site.location {
                LocationType::AtPoint(point, _size) => {
                    builder.build(project.building, point, false)
                }
                LocationType::OnGeyser(geyser, _point) => builder.build_gas(geyser, false),
            };
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[derive(Debug)]
pub enum ProjectNeeds {
    BuilderRallied,
    BuilderOrdered(u64),
    Detector,
    Cleaners,
    Nothing,
    Cancelling,
}
