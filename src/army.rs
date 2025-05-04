use std::collections::HashMap;

use rust_sc2::{
    action::Target,
    ids::{AbilityId, UnitTypeId},
    prelude::Point2,
    unit::Unit,
};

use crate::protoss_bot::ReBiCycler;

impl ReBiCycler {
    pub fn update_army_states(&mut self) {
        let new_states: Vec<UnitState> = self
            .units
            .my
            .units
            .iter()
            .filter(|u| !u.is_worker())
            .map(UnitState::from_unit)
            .collect();

        for us in new_states {
            self.army_manager.update_unit_state(us);
        }
    }
    pub fn command_army(&mut self) {
        let commands: Vec<Result<Command, ArmyIssue>> = self.army_manager.command_all_units();
        for command in commands {
            match command {
                Ok((tag, ability, target, queue)) => {
                    if let Some(u) = self.units.my.units.get(tag) {
                        u.command(ability, target, queue);
                    }
                }
                Err(issue) => {
                    self.log_error(format!("Army issue:{issue:?}"));
                }
            }
        }
    }

    pub fn assign_to_army(&mut self, unit: UnitState) {
        self.army_manager.assign_unit(unit);
    }

    pub fn new_mission(&mut self, mission: MissionType, rally_point: Point2) -> usize {
        self.army_manager.add_mission(mission, rally_point)
    }
}
#[derive(Default)]
pub struct ArmyController {
    active_missions: Vec<Mission>,
    assignments: HashMap<u64, MissionAssignment>,
}

impl ArmyController {
    fn add_mission(&mut self, mission_type: MissionType, rally_point: Point2) -> usize {
        let new_index = self.active_missions.len();
        self.active_missions
            .push(Mission::new(new_index, mission_type, rally_point));
        new_index
    }

    fn command_all_units(&self) -> Vec<Result<Command, ArmyIssue>> {
        self.assignments
            .values()
            .map(|assignment| self.command_one_unit(assignment))
            .collect()
    }
    fn command_one_unit(&self, assignment: &MissionAssignment) -> Result<Command, ArmyIssue> {
        let mission = self
            .active_missions
            .get(assignment.mission)
            .ok_or(ArmyIssue::InvalidMission)?;
        Ok(mission.command(&assignment.unit))
    }

    fn assign_unit(&mut self, unit: UnitState) -> Result<(), ArmyIssue> {
        let mission = self
            .active_missions
            .iter()
            .find(|mission| mission.needs(&unit))
            .ok_or(ArmyIssue::NoAssignmentsForThisUnit)?;

        self.assignments.insert(
            unit.tag,
            MissionAssignment {
                unit,
                mission: mission.id,
            },
        );
        Ok(())
    }

    fn update_unit_state(&mut self, unit: UnitState) {
        if let Some(old_state) = self.assignments.get_mut(&unit.tag) {
            old_state.unit = unit;
        }
    }
}

type Command = (u64, AbilityId, Target, bool);

#[derive(Debug, Clone, Copy)]
enum ArmyIssue {
    NoAssignmentsForThisUnit,
    InvalidMission,
}

struct MissionAssignment {
    unit: UnitState,
    mission: usize,
}
struct Mission {
    id: usize,
    mission_type: MissionType,
    status: MissionStatus,
    rally_point: Point2,
}

impl Mission {
    const fn new(id: usize, mission_type: MissionType, rally: Point2) -> Self {
        Self {
            id,
            mission_type,
            status: MissionStatus::PendingForces,
            rally_point: rally,
        }
    }

    const fn command(&self, unit: &UnitState) -> Command {
        match self.mission_type {
            MissionType::BabysitConstruction(point) => (
                unit.tag,
                AbilityId::AttackAttackTowards,
                Target::Pos(point),
                false,
            ),
            MissionType::AttackEnemy(point) => {
                if matches!(self.status, MissionStatus::InProgress) {
                    (
                        unit.tag,
                        AbilityId::AttackAttackTowards,
                        Target::Pos(point),
                        false,
                    )
                } else {
                    (
                        unit.tag,
                        AbilityId::AttackAttackTowards,
                        Target::Pos(self.rally_point),
                        false,
                    )
                }
            }
            MissionType::DetectArea(point) => (
                unit.tag,
                AbilityId::AttackAttackTowards,
                Target::Pos(point),
                false,
            ),
        }
    }

    const fn needs(&self, unit: &UnitState) -> bool {
        let needs_detector = matches!(self.mission_type, MissionType::DetectArea(_));

        match (needs_detector, unit.is_detector) {
            (true, true) => true,
            (false, false) => true,
            (false, true) => false,
            (true, false) => false,
        }
    }
}

pub enum MissionType {
    BabysitConstruction(Point2),
    DetectArea(Point2),
    AttackEnemy(Point2),
}

pub enum MissionStatus {
    PendingForces,
    InProgress,
    Complete,
}

pub struct UnitState {
    tag: u64,
    type_id: UnitTypeId,

    is_detector: bool,
}
impl UnitState {
    pub fn from_unit(unit: &Unit) -> Self {
        Self {
            tag: unit.tag(),
            type_id: unit.type_id(),

            is_detector: unit.is_detector(),
        }
    }
}
