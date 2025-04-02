use std::collections::HashMap;

use rust_sc2::prelude::*;

use crate::{errors::MicroError, Tag};

const MINERAL_MINE_DISTANCE: f32 = 1.0;
const GAS_MINE_DISTANCE: f32 = 2.5;
const RETURN_CARGO_DISTANCE: f32 = 3.0;

enum ResourceType {
    Gas,
    Minerals,
}

pub struct MinerManager {
    miners: HashMap<u64, (MinerAssignment, MinerMicroState)>,
    resource_assignment_counts: HashMap<u64, usize>,
    assets: Units,
    priority: ResourceType,
}

enum MinerAsset {
    Resource,
    Townhall,
}
struct MinerAssignment {
    resource: Unit,
    townhall: Unit,
}

impl Default for MinerManager {
    fn default() -> Self {
        Self {
            miners: HashMap::new(),
            assets: Units::new(),
            priority: ResourceType::Minerals,
            resource_assignment_counts: HashMap::new(),
        }
    }
}

enum MinerMicroState {
    Idle,
    Gather,
    GatherMove(Point2),
    ReturnCargo,
    ReturnMove(Point2),
}
pub enum MiningError {
    NotHarvestable(u64),
    NoTownhalls,
    NoResources,
}

impl MinerManager {
    pub fn assign_miner(&mut self, miner: Unit) -> Result<(), MiningError> {
        self.employ_miner(miner.tag())
    }
    fn register_unit(&mut self, unit: Unit, assignment: MinerAssignment) {
        self.miners
            .insert(unit.tag(), (assignment, MinerMicroState::Idle));
    }

    pub fn available_jobs(&self) -> usize {
        self.assets
            .iter()
            .map(|u| {
                if u.ideal_harvesters().is_some() {
                    if u.is_mineral() {
                        2
                    } else {
                        3
                    }
                } else {
                    0
                }
            })
            .sum()
    }

    fn remove_asset_assignments(&mut self, removed_asset: u64, asset_type: MinerAsset) -> Vec<u64> {
        self.resource_assignment_counts.remove(&removed_asset);
        self.assets.remove(removed_asset);

        let mut out = Vec::new();
        for (miner, (assignment, _state)) in &self.miners {
            if match asset_type {
                MinerAsset::Resource => &assignment.resource,
                MinerAsset::Townhall => &assignment.townhall,
            }
            .tag()
                == removed_asset
            {
                out.push(*miner);
            }
        }
        for miner in &out {
            self.miners.remove(miner);
        }

        out
    }

    fn remove_asset(&mut self, asset: u64, asset_type: MinerAsset) -> Vec<u64> {
        self.assets.remove(asset);

        self.remove_asset_assignments(asset, asset_type)
    }

    fn employ_miner(&mut self, miner: u64) -> Result<(), MiningError> {
        if !self.assets.iter().any(Unit::is_townhall) {
            // we have no townhalls
            return Err(MiningError::NoTownhalls);
        }

        let job = self.find_job().transpose();
        if let Some(maybe_error) = job {
            let new_job = maybe_error?;
            self.miners.insert(miner, (new_job, MinerMicroState::Idle));
            let count = self.resource_assignment_counts.get(&miner).unwrap_or(&0);
            self.resource_assignment_counts.insert(miner, count + 1);
            Ok(())
        } else {
            Err(MiningError::NoResources)
        }
    }

    fn find_job(&mut self) -> Result<Option<MinerAssignment>, MiningError> {
        let minerals = self.assets.iter().filter(|u| u.is_mineral());
        let gasses = self
            .assets
            .iter()
            .filter(|u| !u.is_mineral() && u.ideal_harvesters().is_some());
        let find_order: Vec<&Unit> = {
            match self.priority {
                ResourceType::Gas => gasses.chain(minerals).collect(),

                ResourceType::Minerals => minerals.chain(gasses).collect(),
            }
        };

        for resource in find_order {
            let count = self
                .resource_assignment_counts
                .get(&resource.tag())
                .unwrap_or(&0usize);
            let employment = self.job_at_resource(resource, *count);
            employment?;
        }
        Ok(None)
    }

    fn job_at_resource(
        &self,
        resource: &Unit,
        count: usize,
    ) -> Result<Option<MinerAssignment>, MiningError> {
        let job = {
            let harvesters = if resource.is_mineral() { 2 } else { 3 };
            if count <= harvesters as usize {
                let nearest_townhall = self
                    .assets
                    .iter()
                    .filter(|u| u.is_townhall())
                    .closest(resource.position())
                    .ok_or(MiningError::NoTownhalls)?;

                let assignment = MinerAssignment {
                    townhall: nearest_townhall.clone(),
                    resource: resource.clone(),
                };

                Some(assignment)
            } else {
                // resource fully allocated
                None
            }
        };
        Ok(job)
    }

    pub fn prioritize(&mut self, resource: ResourceType) {
        self.priority = resource;
    }

    pub fn add_resource(&mut self, unit: Unit) -> Result<(), MiningError> {
        if unit.ideal_harvesters().is_some() {
            self.assets.push(unit);
            Ok(())
        } else {
            Err(MiningError::NotHarvestable(unit.tag()))
        }
    }

    pub fn add_townhall(&mut self, unit: Unit) -> Result<(), MiningError> {
        if unit.is_townhall() {
            self.assets.push(unit);
            Ok(())
        } else {
            Err(MiningError::NotHarvestable(unit.tag()))
        }
    }

    pub fn remove_townhall(&mut self, unit: u64) -> Vec<u64> {
        self.remove_asset(unit, MinerAsset::Townhall)
    }

    fn update_miners<'a>(&'a mut self, my_units: &'a Units) -> Vec<(&'a Unit, MicroError)> {
        my_units
            .iter()
            .filter_map(|unit| {
                let tag = unit.tag();
                if let Some((assignment, state)) = self.miners.remove(&tag) {
                    let new_state = worker_update(unit, state, &assignment);
                    self.miners.insert(tag, (assignment, new_state));
                    None
                } else {
                    Some((unit, MicroError::UnitNotRegistered(Tag::from_unit(unit))))
                }
            })
            .collect()
    }

    fn micro_miners<'a>(&'a self, my_units: &'a Units) -> Vec<(&'a Unit, MicroError)> {
        my_units
            .iter()
            .map(|unit| {
                if let Some(state) = self.miners.get(&unit.tag()) {
                    // do the micro
                    Ok(())
                } else {
                    Err((unit, MicroError::UnitNotRegistered(Tag::from_unit(unit))))
                }
            })
            .filter_map(Result::err)
            .collect()
    }
}

fn worker_micro(unit: &Unit, state: MinerMicroState, assignment: &MinerAssignment) {
    match state {
        MinerMicroState::Gather => unit.gather(assignment.resource.tag(), false),
        MinerMicroState::GatherMove(point) => unit.move_to(Target::Pos(point), false),
        MinerMicroState::ReturnCargo => unit.return_resource(false),
        MinerMicroState::ReturnMove(point) => unit.move_to(Target::Pos(point), false),
        MinerMicroState::Idle => unit.stop(false),
    }
}

fn worker_update(
    unit: &Unit,
    state: MinerMicroState,
    assignment: &MinerAssignment,
) -> MinerMicroState {
    match (unit.is_carrying_resource(), state) {
        (true, MinerMicroState::ReturnMove(point)) => {
            if unit.position().distance(point) < 0.1 {
                MinerMicroState::ReturnCargo
            } else {
                MinerMicroState::ReturnMove(point)
            }
        }
        (true, MinerMicroState::ReturnCargo) => MinerMicroState::ReturnCargo,
        (false, MinerMicroState::ReturnCargo) => MinerMicroState::GatherMove(
            assignment
                .resource
                .position()
                .towards(unit.position(), MINERAL_MINE_DISTANCE),
        ),
        (false, MinerMicroState::GatherMove(point)) => {
            if unit.position().distance(point) < 0.1 {
                MinerMicroState::Gather
            } else {
                MinerMicroState::GatherMove(point)
            }
        }
        (false, MinerMicroState::Gather) => MinerMicroState::Gather,
        (true, MinerMicroState::Gather) => MinerMicroState::ReturnMove(
            assignment
                .townhall
                .position()
                .towards(unit.position(), RETURN_CARGO_DISTANCE),
        ),
        _ => MinerMicroState::ReturnCargo,
    }
}
