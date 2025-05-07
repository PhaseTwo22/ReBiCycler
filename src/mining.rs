use std::{collections::HashMap, fmt::Display};

use rust_sc2::{
    action::Target,
    ids::AbilityId,
    prelude::{Distance, Point2},
    unit::Unit,
    units::Units,
};

use crate::assignment_manager::{AssignmentManager, Assigns, Commands, Identity};

const MINERAL_MINE_DISTANCE: f32 = 1.0;
const GAS_MINE_DISTANCE: f32 = 2.5;
const RETURN_CARGO_DISTANCE: f32 = 2.9;

#[derive(Default)]
pub struct MinerController {
    pub mining_manager: AssignmentManager<Miner, ResourcePairing, u64, JobId>,
}

pub struct Miner {
    worker_tag: u64,
    state: MinerMicroState,
    holding_resource: bool,
    position: Point2,
}
impl Miner {
    fn update(unit: &Unit, last_state: MinerMicroState) -> Self {
        Self {
            worker_tag: unit.tag(),
            state: last_state,
            holding_resource: unit.is_carrying_resource(),
            position: unit.position(),
        }
    }

    pub fn new(unit: &Unit) -> Self {
        Self {
            worker_tag: unit.tag(),
            state: MinerMicroState::Idle,
            holding_resource: unit.is_carrying_resource(),
            position: unit.position(),
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ResourcePairing {
    pub resource: MinerAsset,
    pub townhall: Townhall,
    pub location: Point2,
    pub tag: u64,
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct JobId {
    resource_tag: u64,
    townhall_tag: u64,
}

#[derive(Clone)]
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
enum AssetType {
    Minerals,
    Gas,
}
impl ResourcePairing {
    fn is_mineral(&self) -> bool {
        self.resource.asset_type == AssetType::Minerals
    }
    fn is_gas(&self) -> bool {
        self.resource.asset_type == AssetType::Gas
    }

    fn new(resource: &Unit, nearest_townhall: &Unit) -> Self {
        Self {
            resource: MinerAsset {
                location: resource.position(),
                asset_type: if resource.is_mineral() {
                    AssetType::Minerals
                } else {
                    AssetType::Gas
                },
            },
            townhall: Townhall {
                tag: nearest_townhall.tag(),
                location: nearest_townhall.position(),
            },
            location: resource.position(),
            tag: resource.tag(),
        }
    }
}
impl Identity<JobId> for ResourcePairing {
    fn id(&self) -> JobId {
        JobId {
            resource_tag: self.tag,
            townhall_tag: self.townhall.tag,
        }
    }
}
impl Identity<u64> for Miner {
    fn id(&self) -> u64 {
        self.worker_tag
    }
}

impl MinerController {
    pub fn add_worker(&mut self, new_miner: Miner) -> Result<(), u8> {
        let new_job = self
            .mining_manager
            .count_assignments()
            .into_iter()
            .find_map(|(pairing, count)| {
                if (pairing.is_mineral() && count < 2) || (pairing.is_gas() && count < 3) {
                    Some(pairing)
                } else {
                    None
                }
            })
            .ok_or(2)?;

        let _ = self.mining_manager.assign(new_miner, &new_job.clone());
        Ok(())
    }

    pub fn remove_worker(&mut self, worker_tag: u64) -> bool {
        self.mining_manager.unassign(worker_tag).is_ok()
    }

    pub fn add_resource(&mut self, resource: &Unit, nearest_townhall: &Unit) {
        let less_close_mining: Vec<JobId> = self
            .mining_manager
            .iter_roles()
            .filter_map(|p| {
                if p.tag == resource.tag() {
                    Some(p.id())
                } else {
                    None
                }
            })
            .collect();

        for lc in less_close_mining {
            self.mining_manager.remove_role(lc);
        }

        let new_resource = ResourcePairing::new(resource, nearest_townhall);
        self.mining_manager.add_role(new_resource);
    }

    pub fn add_townhall(&mut self, townhall: &Unit, nearby_resources: &Units) {
        let new_roles: Vec<ResourcePairing> = nearby_resources
            .iter()
            .map(|resource| ResourcePairing::new(resource, townhall))
            .collect();
        for role in new_roles {
            self.mining_manager.add_role(role);
        }
    }

    pub fn remove_resource(&mut self, resource_tag: u64) -> Vec<u64> {
        let destroyed_roles: Vec<JobId> = self
            .mining_manager
            .iter_roles()
            .filter(|j| j.tag == resource_tag)
            .map(super::assignment_manager::Identity::id)
            .collect();
        let mut newly_unemployed = Vec::new();
        for role_id in destroyed_roles {
            if let Ok(unemployed) = self.mining_manager.remove_role(role_id) {
                newly_unemployed.extend(unemployed);
            }
        }
        newly_unemployed
    }

    pub fn remove_townhall(&mut self, townhall_tag: u64) -> Vec<u64> {
        let destroyed_roles: Vec<JobId> = self
            .mining_manager
            .get_role_ids()
            .filter(|j| j.townhall_tag == townhall_tag)
            .cloned()
            .collect();
        let mut newly_unemployed = Vec::new();
        for role_id in destroyed_roles {
            if let Ok(unemployed) = self.mining_manager.remove_role(role_id) {
                newly_unemployed.extend(unemployed);
            }
        }
        newly_unemployed
    }

    pub fn employed_miners(&self) -> impl Iterator<Item = u64> + use<'_> {
        self.mining_manager
            .iter_assignees()
            .map(super::assignment_manager::Identity::id)
    }

    pub fn saturation(&self) -> HashMap<&ResourcePairing, usize> {
        self.mining_manager.count_assignments()
    }
}

impl Display for MinerController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut total_gas_jobs = 0;
        let mut total_minerals_jobs = 0;
        let mut gas_buildings = 0;
        let mut mineral_fields = 0;
        for (pair, count) in self.saturation() {
            if pair.is_gas() {
                gas_buildings += 1;
                total_gas_jobs += count;
            } else {
                mineral_fields += 1;
                total_minerals_jobs += count;
            }
        }
        write!(
            f,
            "M:{total_minerals_jobs}:{mineral_fields} G:{total_gas_jobs}:{gas_buildings}"
        )
    }
}

type MiningCommand = (AbilityId, Target, bool);

impl Commands<MiningCommand, Miner, u64, Units> for MinerController {
    fn issue_commands(&self) -> Vec<(u64, MiningCommand)> {
        self.mining_manager
            .iter_assignments()
            .map(|(a, r)| (a.id(), worker_micro(a, r)))
            .collect()
    }
    fn get_peon_updates(&mut self, data: Units) -> Vec<Miner> {
        data.iter()
            .filter_map(|unit| {
                if let (Ok(last_observation), Ok(last_assignment)) = (
                    self.mining_manager.get_assignee(unit.tag()),
                    self.mining_manager.get_assignment(unit.tag()),
                ) {
                    let this_observation = Miner::update(unit, last_observation.state.clone());
                    let new_state = worker_update(&this_observation, last_assignment);
                    Some(Miner::update(unit, new_state))
                } else {
                    None
                }
            })
            .collect()
    }

    fn apply_peon_updates(&mut self, updates: Vec<Miner>) {
        for up in updates {
            let _ = self.mining_manager.update_assignee(up.id(), up);
        }
    }
}

const fn worker_micro(unit: &Miner, assignment: &ResourcePairing) -> MiningCommand {
    let (ability, target) = match unit.state {
        MinerMicroState::Gather => (AbilityId::Smart, Target::Tag(assignment.tag)),

        MinerMicroState::ReturnCargo => (AbilityId::HarvestReturn, Target::None),
        MinerMicroState::GatherMove(point) | MinerMicroState::ReturnMove(point) => {
            (AbilityId::Move, Target::Pos(point))
        }
        MinerMicroState::Idle => (AbilityId::Stop, Target::None),
    };
    (ability, target, false)
}

#[allow(clippy::match_same_arms)]
fn worker_update(unit: &Miner, assignment: &ResourcePairing) -> MinerMicroState {
    match (&unit.holding_resource, &unit.state) {
        (true, MinerMicroState::ReturnMove(point)) => {
            if unit.position.distance(point) < RETURN_CARGO_DISTANCE {
                MinerMicroState::ReturnCargo
            } else {
                MinerMicroState::ReturnMove(*point)
            }
        }
        (true, MinerMicroState::ReturnCargo) => MinerMicroState::ReturnCargo,
        (false, MinerMicroState::ReturnCargo) => {
            MinerMicroState::GatherMove(assignment.resource.location.towards(
                unit.position,
                if assignment.is_mineral() {
                    MINERAL_MINE_DISTANCE
                } else {
                    GAS_MINE_DISTANCE
                },
            ))
        }
        (false, MinerMicroState::GatherMove(point)) => {
            if unit.position.distance(point) < GAS_MINE_DISTANCE {
                MinerMicroState::Gather
            } else {
                MinerMicroState::GatherMove(*point)
            }
        }
        (false, MinerMicroState::Gather) => MinerMicroState::Gather,
        (true, MinerMicroState::Gather) => MinerMicroState::ReturnMove(
            assignment
                .townhall
                .location
                .towards(unit.position, RETURN_CARGO_DISTANCE),
        ),
        _ => MinerMicroState::ReturnCargo,
    }
}
