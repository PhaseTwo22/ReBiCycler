use rust_sc2::{action::Target, ids::AbilityId, prelude::Point2, unit::Unit};

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

    pub fn assign_to_army(&mut self, unit: UnitState) -> Result<(), ArmyIssue> {
        self.army_manager.assign_unit(unit)
    }

    pub fn new_mission(&mut self, mission: MissionType, rally_point: Point2) -> usize {
        self.army_manager.add_mission(mission, rally_point)
    }
}

type Command = (u64, AbilityId, Target, bool);

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
            (true, true) | (false, false) => true,
            (false, true) | (true, false) => false,
        }
    }
}

#[derive(Debug)]
pub struct ArmyIssue;
pub struct UnitState {
    is_detector: bool,
    tag: u64,
}
impl UnitState {
    fn from_unit(_: &Unit) -> Self {
        Self {
            is_detector: false,
            tag: 10,
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
impl MissionStatus {
    fn begin(self) -> Self {
        match self {
            Self::PendingForces => Self::InProgress,
            otherwise => otherwise,
        }
    }

    fn finish(self) -> Self {
        match self {
            Self::InProgress => Self::Complete,
            otherwise => otherwise,
        }
    }
}
#[derive(Default)]
pub struct ArmyController;
impl ArmyController {
    fn update_army_states(&self) {
        todo!()
    }
    fn update_unit_state(&mut self, _: UnitState) {
        todo!()
    }
    fn command_all_units(&self) -> Vec<Result<Command, ArmyIssue>> {
        todo!()
    }
    fn assign_unit(&self, _: UnitState) -> Result<(), ArmyIssue> {
        todo!()
    }
    fn add_mission(&mut self, _: MissionType, _: Point2) -> usize {
        todo!()
    }
}
