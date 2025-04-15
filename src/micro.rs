use std::{collections::HashMap, fmt::Display};

use crate::{errors::MicroError, Tag};
use rust_sc2::prelude::*;
use std::fmt::Debug;

const MINERAL_MINE_DISTANCE: f32 = 1.0;
const GAS_MINE_DISTANCE: f32 = 2.5;
const RETURN_CARGO_DISTANCE: f32 = 3.0;

#[derive(Debug, Clone)]
pub struct ResourceSite {
    pub resource: MinerAsset,
    pub location: Point2,
    pub tag: u64,
}
impl ResourceSite {
    fn is_mineral(&self) -> bool {
        self.resource == MinerAsset::Minerals
    }
    fn is_gas(&self) -> bool {
        self.resource == MinerAsset::Gas
    }
    fn is_townhall(&self) -> bool {
        self.resource == MinerAsset::Townhall
    }
    fn from_unit(unit: &Unit) -> Self {
        Self {
            resource: if unit.is_mineral() {
                MinerAsset::Minerals
            } else if unit.is_townhall() {
                MinerAsset::Townhall
            } else {
                MinerAsset::Gas
            },
            location: unit.position(),
            tag: unit.tag(),
        }
    }
    const fn harvesters(&self) -> u32 {
        match self.resource {
            MinerAsset::Gas => 3,
            MinerAsset::Minerals => 2,
            MinerAsset::Townhall => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MinerAsset {
    Minerals,
    Gas,
    Townhall,
}

struct MinerAssignment {
    resource: ResourceSite,
    townhall: ResourceSite,
}

impl Debug for MinerAssignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MiningAssignment {:?} at base near {:?}",
            self.resource.resource, self.townhall.location
        )
    }
}

pub struct MinerManager {
    miners: HashMap<u64, (MinerAssignment, MinerMicroState)>,
    resource_assignment_counts: HashMap<u64, usize>,
    assets: HashMap<u64, ResourceSite>,
    priority: MinerAsset,
}

impl Default for MinerManager {
    fn default() -> Self {
        Self {
            miners: HashMap::new(),
            assets: HashMap::new(),
            priority: MinerAsset::Minerals,
            resource_assignment_counts: HashMap::new(),
        }
    }
}

impl Debug for MinerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.resource_assignment_counts.values())
    }
}
impl Display for MinerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let minerals = self.assets.values().filter(|u| u.is_mineral()).count();
        let gasses = self.assets.values().filter(|u| u.is_gas()).count();
        let mineral_assignments = self
            .miners
            .values()
            .filter(|(assignment, _)| assignment.resource.is_mineral())
            .count();
        let gas_assignments = self
            .miners
            .values()
            .filter(|(assignment, _)| assignment.resource.is_gas())
            .count();
        write!(
            f,
            "Patches: {minerals}:{mineral_assignments} | Gasses: {gasses}:{gas_assignments}"
        )
    }
}
enum MinerMicroState {
    Idle,
    Gather,
    GatherMove(Point2),
    ReturnCargo,
    ReturnMove(Point2),
}
#[derive(Debug)]
pub enum MiningError {
    NotHarvestable(Tag),
    NotTownhall(Tag),
    NoTownhalls,
    NoResources,
}

impl MinerManager {
    pub fn micro(&mut self, units: &Units) {
        let errors: Vec<MicroError> = self.update_miners(units);
        for e in errors {
            if let Err(second_error) = match e {
                MicroError::UnitNotRegistered(tag) => self.employ_miner(tag.tag),
            } {
                println!("Unable to resolve error within MinerManager: {second_error:?}");
            };
        }
        self.micro_miners(units);
    }
    pub fn assign_miner(&mut self, miner: u64) -> Result<(), MiningError> {
        self.employ_miner(miner)
    }

    pub fn employed_miners(&self) -> impl Iterator<Item = &u64> {
        self.miners.keys()
    }

    pub fn available_jobs(&self) -> usize {
        self.assets
            .values()
            .map(|site| match site.resource {
                MinerAsset::Gas => 3,
                MinerAsset::Minerals => 2,
                MinerAsset::Townhall => 0,
            })
            .sum()
    }

    pub fn remove_miner(&mut self, miner: u64) -> bool {
        if let Some(old_job) = self.miners.remove(&miner) {
            self.resource_assignment_counts
                .entry(old_job.0.resource.tag)
                .and_modify(|count| {
                    *count = count.saturating_sub(1);
                });
            true
        } else {
            false
        }
    }

    fn remove_asset_assignments(&mut self, removed_asset: u64, asset_type: MinerAsset) -> Vec<u64> {
        self.resource_assignment_counts.remove(&removed_asset);
        self.assets.remove(&removed_asset);

        let mut out = Vec::new();
        for (miner, (assignment, _state)) in &self.miners {
            if match asset_type {
                MinerAsset::Gas | MinerAsset::Minerals => assignment.resource.tag,
                MinerAsset::Townhall => assignment.townhall.tag,
            } == removed_asset
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
        self.assets.remove(&asset);

        self.remove_asset_assignments(asset, asset_type)
    }

    fn employ_miner(&mut self, miner: u64) -> Result<(), MiningError> {
        if !self.assets.values().any(ResourceSite::is_townhall) {
            // we have no townhalls
            return Err(MiningError::NoTownhalls);
        }

        let job = self.find_job().transpose();
        if let Some(maybe_error) = job {
            let new_job = maybe_error?;
            *self
                .resource_assignment_counts
                .entry(new_job.resource.tag)
                .or_insert(0) += 1;

            self.miners.insert(miner, (new_job, MinerMicroState::Idle));
            Ok(())
        } else {
            Err(MiningError::NoResources)
        }
    }

    fn find_job(&self) -> Result<Option<MinerAssignment>, MiningError> {
        let minerals = self.assets.values().filter(|u| u.is_mineral());
        let gasses = self.assets.values().filter(|u| u.is_gas());
        let find_order: Vec<&ResourceSite> = {
            match self.priority {
                MinerAsset::Gas => gasses.chain(minerals).collect(),
                MinerAsset::Minerals | MinerAsset::Townhall => minerals.chain(gasses).collect(),
            }
        };

        for resource in find_order {
            let count = self
                .resource_assignment_counts
                .get(&resource.tag)
                .unwrap_or(&0usize);
            let employment = self.job_at_resource(resource, *count);
            if let Some(job) = employment? {
                return Ok(Some(job));
            }
        }
        Ok(None)
    }

    fn job_at_resource(
        &self,
        resource: &ResourceSite,
        count: usize,
    ) -> Result<Option<MinerAssignment>, MiningError> {
        let job = {
            let harvesters: u32 = resource.harvesters();
            if count < harvesters as usize {
                let nearest_townhall = self
                    .assets
                    .values()
                    .filter(|u| u.is_townhall())
                    .min_by(|a, b| crate::closeratest(resource.location, a.location, b.location))
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

    pub fn prioritize(&mut self, resource: MinerAsset) {
        if resource == MinerAsset::Townhall {
            self.prioritize(MinerAsset::Minerals);
        } else {
            self.priority = resource;
        }
    }

    pub fn add_resource(&mut self, unit: &Unit) -> Result<(), MiningError> {
        if unit.is_mineral() || unit.is_geyser() {
            self.assets
                .insert(unit.tag(), ResourceSite::from_unit(unit));
            Ok(())
        } else {
            Err(MiningError::NotHarvestable(Tag::from_unit(unit)))
        }
    }

    pub fn add_townhall(&mut self, unit: &Unit) -> Result<(), MiningError> {
        if unit.is_townhall() {
            self.assets
                .insert(unit.tag(), ResourceSite::from_unit(unit));
            Ok(())
        } else {
            Err(MiningError::NotTownhall(Tag::from_unit(unit)))
        }
    }

    pub fn remove_townhall(&mut self, unit: u64) -> Vec<u64> {
        self.remove_asset(unit, MinerAsset::Townhall)
    }

    pub fn remove_resource(&mut self, unit: u64, is_minerals: bool) -> Vec<u64> {
        self.remove_asset(
            unit,
            if is_minerals {
                MinerAsset::Minerals
            } else {
                MinerAsset::Gas
            },
        )
    }

    fn update_miners<'a>(&'a mut self, my_units: &'a Units) -> Vec<MicroError> {
        my_units
            .iter()
            .filter_map(|unit| {
                let tag = unit.tag();
                if let Some((assignment, state)) = self.miners.remove(&tag) {
                    let new_state = worker_update(unit, state, &assignment);
                    self.miners.insert(tag, (assignment, new_state));
                    None
                } else {
                    Some(MicroError::UnitNotRegistered(Tag::from_unit(unit)))
                }
            })
            .collect()
    }

    fn micro_miners<'a>(&'a self, my_units: &'a Units) -> Vec<MicroError> {
        my_units
            .iter()
            .map(|unit| {
                if let Some((assignment, state)) = self.miners.get(&unit.tag()) {
                    worker_micro(unit, state, assignment);
                    Ok(())
                } else {
                    Err(MicroError::UnitNotRegistered(Tag::from_unit(unit)))
                }
            })
            .filter_map(Result::err)
            .collect()
    }
}

fn worker_micro(unit: &Unit, state: &MinerMicroState, assignment: &MinerAssignment) {
    match state {
        MinerMicroState::Gather => unit.gather(assignment.resource.tag, false),

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
        (false, MinerMicroState::ReturnCargo) => {
            MinerMicroState::GatherMove(assignment.resource.location.towards(
                unit.position(),
                if assignment.resource.is_mineral() {
                    MINERAL_MINE_DISTANCE
                } else {
                    GAS_MINE_DISTANCE
                },
            ))
        }
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
                .location
                .towards(unit.position(), RETURN_CARGO_DISTANCE),
        ),
        _ => MinerMicroState::ReturnCargo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_miner() -> MinerManager {
        MinerManager::default()
    }

    #[test]
    fn add_worker_to_empty_manager() {
        let mut mm = init_miner();

        assert!(mm.employ_miner(1).is_err());
    }
}
