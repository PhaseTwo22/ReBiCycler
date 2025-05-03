use std::collections::HashMap;

use rust_sc2::{ids::UnitTypeId, prelude::Point2, score::Vital, units::Units};

use crate::protoss_bot::ReBiCycler;

impl ReBiCycler {
    pub fn plan_army(&mut self, army: Units) {}

    pub fn command_army(&self) {}

    fn command_unit(&self, state: UnitState) {}

    pub fn reassign_unit(&self, unit: UnitState) {}

    pub fn new_mission(&mut self, mission: MissionType) -> usize {
        self.army_manager.add_mission(mission)
    }
}
#[derive(Default)]
pub struct ArmyController {
    active_missions: Vec<Mission>,
    assignments: HashMap<u64, MissionAssignment>,
}

impl ArmyController {
    fn add_mission(&mut self, mission_type: MissionType) -> usize {
        let new_index = self.active_missions.len();
        self.active_missions
            .push(Mission::new(new_index, mission_type));
        new_index
    }
}

struct MissionAssignment {
    unit: UnitState,
    assignment: Mission,
}
struct Mission {
    id: usize,
    mission_type: MissionType,
    status: MissionStatus,
}

impl Mission {
    fn new(id: usize, mission_type: MissionType) -> Self {
        Self {
            id,
            mission_type,
            status: MissionStatus::PendingForces,
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

enum Tactic {
    AttackMove(Point2),
    StutterMove(Point2),
    DirectMove(Point2),
}

struct UnitState {
    tag: u64,
    type_id: UnitTypeId,
    vitals: Vital,
    weapon_cooldown: Option<(f32, f32)>,
    energy: Option<f32>,
}
