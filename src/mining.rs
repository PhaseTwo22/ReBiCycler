use rust_sc2::prelude::Point2;

use crate::assignment_manager::{AssignmentManager, Identity};

pub struct MinerManager {
    priority: MinerAsset,
    //assignment_manager: AssignmentManager<Miner, ResourceSite, u64>,
}

struct MinerAssignment {
    resource: ResourceSite,
    townhall: ResourceSite,
}

struct Miner {
    tag: u64,
    state: MinerMicroState,
}
impl Identity<u64> for Miner {
    fn id(&self) -> u64 {
        self.tag
    }
}

enum MinerMicroState {
    Idle,
    Gather,
    GatherMove(Point2),
    ReturnCargo,
    ReturnMove(Point2),
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum MinerAsset {
    Minerals,
    Gas,
    Townhall,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ResourceSite {
    pub resource: MinerAsset,
    pub location: Point2,
    pub tag: u64,
}
impl Identity<u64> for ResourceSite {
    fn id(&self) -> u64 {
        self.tag
    }
}
