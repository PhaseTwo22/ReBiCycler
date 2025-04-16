use std::{collections::HashMap, fmt::Display};

use crate::{errors::MicroError, Tag};
use rust_sc2::prelude::*;
use std::fmt::Debug;

const MINERAL_MINE_DISTANCE: f32 = 1.0;
const GAS_MINE_DISTANCE: f32 = 2.5;
const RETURN_CARGO_DISTANCE: f32 = 2.9;

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
    const fn harvesters(&self) -> usize {
        match self.resource {
            MinerAsset::Gas => 3,
            MinerAsset::Minerals => 2,
            MinerAsset::Townhall => 0,
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
    AlreadyEmployed(u64),
}

impl MinerManager {
    pub fn assign_miner(&mut self, miner: u64) -> Result<(), MiningError> {
        self.employ_miner(miner)
    }

    pub fn employed_miners(&self) -> impl Iterator<Item = &u64> {
        self.miners.keys()
    }

    pub fn saturation(&self) -> (usize, usize, usize, usize) {
        let mut mineral_max = 0;
        let mut mineral_assigned = 0;
        let mut gas_max = 0;
        let mut gas_assigned = 0;
        for (asset, rs) in &self.assets {
            let count = self.resource_assignment_counts.get(asset).unwrap_or(&0);
            if rs.is_gas() {
                gas_max += rs.harvesters();
                gas_assigned += count;
            } else {
                mineral_max += rs.harvesters();
                mineral_assigned += count;
            }
        }

        (mineral_max, mineral_assigned, gas_max, gas_assigned)
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
        if self.miners.contains_key(&miner) {
            return Err(MiningError::AlreadyEmployed(miner));
        }

        let job = self.find_job().transpose();
        if let Some(maybe_error) = job {
            let new_job = maybe_error?;
            self.assign_job_to_miner(miner, new_job);
            Ok(())
        } else {
            Err(MiningError::NoResources)
        }
    }

    fn assign_job_to_miner(&mut self, miner: u64, assignment: MinerAssignment) {
        *self
            .resource_assignment_counts
            .entry(assignment.resource.tag)
            .or_insert(0) += 1;
        self.miners
            .insert(miner, (assignment, MinerMicroState::Idle));
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
            let harvesters = resource.harvesters();
            if count < harvesters {
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
    pub fn add_resource_site(&mut self, site: ResourceSite) {
        self.assets.insert(site.tag, site);
    }
    pub fn add_resource(&mut self, unit: &Unit) -> Result<(), MiningError> {
        if unit.is_mineral() || unit.is_geyser() {
            self.add_resource_site(ResourceSite::from_unit(unit));
            Ok(())
        } else {
            Err(MiningError::NotHarvestable(Tag::from_unit(unit)))
        }
    }

    pub fn add_townhall(&mut self, unit: &Unit) -> Result<(), MiningError> {
        if unit.is_townhall() {
            self.add_resource_site(ResourceSite::from_unit(unit));
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
                if assignment.resource.is_mineral() {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn init_miner() -> MinerManager {
        let mut mm = MinerManager::default();
        mm.add_resource_site(ResourceSite {
            location: Point2::new(0.0, 0.0),
            resource: MinerAsset::Townhall,
            tag: 999_999,
        });
        mm
    }

    #[test]
    fn add_worker_to_empty_manager() {
        let mut mm = init_miner();

        assert!(mm.employ_miner(1).is_err());
    }
    #[test]
    fn add_three_to_patch() {
        let mut mm = init_miner();
        mm.add_resource_site(ResourceSite {
            location: Point2::new(0.0, 0.0),
            resource: MinerAsset::Minerals,
            tag: 999,
        });
        assert!(mm.assign_miner(1).is_ok());
        assert!(mm.assign_miner(2).is_ok());
        assert!(mm.assign_miner(3).is_err());
    }

    #[test]
    fn add_to_patch_twice() {
        let mut mm = init_miner();
        mm.add_resource_site(ResourceSite {
            location: Point2::new(0.0, 0.0),
            resource: MinerAsset::Minerals,
            tag: 999,
        });
        assert!(mm.assign_miner(1).is_ok());
        assert!(mm.assign_miner(1).is_err());
    }

    #[test]
    fn add_four_to_gas() {
        let mut mm = init_miner();
        mm.add_resource_site(ResourceSite {
            location: Point2::new(0.0, 0.0),
            resource: MinerAsset::Gas,
            tag: 999,
        });
        assert!(mm.assign_miner(1).is_ok());
        assert!(mm.assign_miner(2).is_ok());
        assert!(mm.assign_miner(3).is_ok());

        assert_eq!(mm.saturation(), (0, 0, 3, 3));
        assert!(mm.assign_miner(4).is_err());
    }

    #[test]
    fn add_and_remove_resource() {
        let mut mm = init_miner();
        mm.add_resource_site(ResourceSite {
            location: Point2::new(0.0, 0.0),
            resource: MinerAsset::Minerals,
            tag: 999,
        });
        assert!(mm.assign_miner(1).is_ok());
        assert_eq!(mm.remove_resource(999, true).len(), 1);
        assert_eq!(mm.employed_miners().count(), 0);
        assert_eq!(mm.saturation(), (0, 0, 0, 0));
    }

    #[test]
    fn add_two_townhalls() {
        let mut mm = MinerManager::default();
        mm.add_resource_site(ResourceSite {
            location: Point2::new(0.0, 0.0),
            resource: MinerAsset::Townhall,
            tag: 999_999,
        });
        mm.add_resource_site(ResourceSite {
            location: Point2::new(0.0, 0.0),
            resource: MinerAsset::Townhall,
            tag: 999_998,
        });
        mm.add_resource_site(ResourceSite {
            location: Point2::new(0.0, 0.0),
            resource: MinerAsset::Minerals,
            tag: 999,
        });
        assert!(mm.assign_miner(1).is_ok());
        assert!(mm.assign_miner(2).is_ok());
        assert!(mm.assign_miner(3).is_err());

        assert_eq!(
            format!("Patches: {}:{} | Gasses: {}:{}", 1, 2, 0, 0),
            format!("{}", mm)
        );
    }

    #[test]
    fn add_remove_addback() {
        let mut mm = init_miner();
        mm.add_resource_site(ResourceSite {
            location: Point2::new(0.0, 0.0),
            resource: MinerAsset::Minerals,
            tag: 999,
        });

        for i in 0..2 {
            mm.add_resource_site(ResourceSite {
                location: Point2::new(0.0, 0.0),
                resource: MinerAsset::Gas,
                tag: 10_000 + i,
            });
        }
        assert!(mm.assign_miner(1).is_ok());
        for _ in 0..10 {
            assert!(mm.remove_miner(1));
            assert!(mm.assign_miner(1).is_ok());
        }
        assert_eq!(mm.saturation(), (2, 1, 6, 0));
        assert_eq!(mm.resource_assignment_counts.values().sum::<usize>(), 1);
    }

    #[test]
    fn simulate_game() {
        let mut mm = init_miner();
        for i in 0..8 {
            mm.add_resource_site(ResourceSite {
                location: Point2::new(0.0, 0.0),
                resource: MinerAsset::Minerals,
                tag: 1_000 + i,
            });
        }
        assert_eq!(mm.saturation(), (16, 0, 0, 0));

        for i in 0..12 {
            assert!(mm.assign_miner(i).is_ok());
        }
        assert_eq!(mm.saturation(), (16, 12, 0, 0));

        for i in 0..4 {
            assert!(mm.assign_miner(12 + i).is_ok());
        }
        assert_eq!(mm.saturation(), (16, 16, 0, 0));

        assert!(matches!(mm.assign_miner(16), Err(MiningError::NoResources)));

        assert!(mm.remove_miner(15));
        assert!(mm.assign_miner(15).is_ok());
        assert!(mm.assign_miner(16).is_err());

        assert_eq!(
            format!("Patches: {}:{} | Gasses: {}:{}", 8, 16, 0, 0),
            format!("{}", mm)
        );
        assert_eq!(mm.saturation(), (16, 16, 0, 0));

        for i in 0..2 {
            mm.add_resource_site(ResourceSite {
                location: Point2::new(0.0, 0.0),
                resource: MinerAsset::Gas,
                tag: 10_000 + i,
            });
        }
        assert_eq!(mm.saturation(), (16, 16, 6, 0));

        for i in 0..6 {
            assert!(mm.assign_miner(16 + i).is_ok());
        }
        assert_eq!(mm.saturation(), (16, 16, 6, 6));
        assert_eq!(
            format!("Patches: {}:{} | Gasses: {}:{}", 8, 16, 2, 6),
            format!("{}", mm)
        );

        assert!(mm.assign_miner(22).is_err());

        mm.add_resource_site(ResourceSite {
            location: Point2::new(10.0, 0.0),
            resource: MinerAsset::Townhall,
            tag: 999_999 + 1,
        });
        for i in 0..8 {
            mm.add_resource_site(ResourceSite {
                location: Point2::new(0.0, 0.0),
                resource: MinerAsset::Minerals,
                tag: 1_008 + i,
            });
        }

        for i in 0..16 {
            assert!(mm.assign_miner(22 + i).is_ok());
        }
        assert_eq!(mm.saturation(), (32, 32, 6, 6));
        assert_eq!(
            format!("Patches: {}:{} | Gasses: {}:{}", 16, 32, 2, 6),
            format!("{}", mm)
        );
    }
}
