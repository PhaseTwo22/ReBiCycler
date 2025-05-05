use std::collections::HashMap;

use rust_sc2::{
    action::Target,
    ids::AbilityId,
    prelude::{Distance, Point2},
    unit::Unit,
    units::Units,
};

use crate::assignment_manager::{AssignmentManager, Assigns, CommandError, Commands, Identity};

const MINERAL_MINE_DISTANCE: f32 = 1.0;
const GAS_MINE_DISTANCE: f32 = 2.5;
const RETURN_CARGO_DISTANCE: f32 = 2.9;

pub struct MinerManager {
    priority: MinerAsset,
    assignment_manager: AssignmentManager<Miner, ResourcePairing, u64, JobId>,
}

struct Miner {
    worker_tag: u64,
    state: MinerMicroState,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ResourcePairing {
    pub resource: MinerAsset,
    pub townhall: Townhall,
    pub location: Point2,
    pub tag: u64,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct JobId {
    resource_loc: Point2,
    townhall_loc: Point2,
}

enum MinerMicroState {
    Idle,
    Gather,
    GatherMove(Point2),
    ReturnCargo,
    ReturnMove(Point2),
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Townhall {
    tag: u64,
    location: Point2,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct MinerAsset {
    location: Point2,
    asset_type: AssetType,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum AssetType {
    Minerals,
    Gas,
}
impl ResourcePairing {
    fn is_mineral(&self) -> bool {
        self.resource.asset_type == AssetType::Minerals
    }
}
impl Identity<JobId> for ResourcePairing {
    fn id(&self) -> JobId {
        JobId {
            resource_loc: self.resource.location,
            townhall_loc: self.townhall.location,
        }
    }
}
impl Identity<u64> for Miner {
    fn id(&self) -> u64 {
        self.worker_tag
    }
}

impl MinerManager {}

type MiningCommand = (AbilityId, Target, bool);

impl Commands<MiningCommand, Miner, u64, Units> for MinerManager {
    fn issue_commands(&self) -> Vec<(Miner, MiningCommand)> {}
    fn update_peon_states(&mut self, data: Units) -> Result<(), CommandError> {}
}

fn worker_micro(unit: &Unit, state: &MinerMicroState, assignment: ResourcePairing) {
    match state {
        MinerMicroState::Gather => unit.gather(assignment.tag, false),

        MinerMicroState::ReturnCargo => unit.return_resource(false),
        MinerMicroState::GatherMove(point) | MinerMicroState::ReturnMove(point) => {
            unit.move_to(Target::Pos(*point), false);
        }
        MinerMicroState::Idle => unit.stop(false),
    }
}

#[allow(clippy::match_same_arms)]
fn worker_update(
    unit: &Unit,
    state: MinerMicroState,
    assignment: ResourcePairing,
) -> MinerMicroState {
    match (unit.is_carrying_resource(), state) {
        (true, MinerMicroState::ReturnMove(point)) => {
            if unit.position().distance(point) < RETURN_CARGO_DISTANCE {
                MinerMicroState::ReturnCargo
            } else {
                MinerMicroState::ReturnMove(point)
            }
        }
        (true, MinerMicroState::ReturnCargo) => MinerMicroState::ReturnCargo,
        (false, MinerMicroState::ReturnCargo) => {
            MinerMicroState::GatherMove(assignment.resource.location.towards(
                unit.position(),
                if assignment.is_mineral() {
                    MINERAL_MINE_DISTANCE
                } else {
                    GAS_MINE_DISTANCE
                },
            ))
        }
        (false, MinerMicroState::GatherMove(point)) => {
            if unit.position().distance(point) < GAS_MINE_DISTANCE {
                MinerMicroState::Gather
            } else {
                MinerMicroState::GatherMove(point)
            }
        }
        (false, MinerMicroState::Gather) => MinerMicroState::Gather,
        (true, MinerMicroState::Gather) => MinerMicroState::ReturnMove(
            assignment
                .townhall
                .location
                .towards(unit.position(), RETURN_CARGO_DISTANCE),
        ),
        _ => MinerMicroState::ReturnCargo,
    }
}
