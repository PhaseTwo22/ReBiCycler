use core::fmt;
use std::ops::Deref;

use rust_sc2::prelude::*;

use crate::{errors::BuildError, protoss_bot::ReBiCycler};

const CHRONOBOOST_COST: u32 = 50;
/// This module serves to manage our build orders.
/// We want to use kiss principle here, but still have a flexible system.
///
/// A build order is made of components, each with a condition and an action.
/// It also contains policies, which are a set of actions that are executed until a condition is met.
/// Policies can be active and inactive.

#[derive(Default)]
pub struct BuildOrderManager {
    pub policies: Vec<Policy>,
    pub build_order: Vec<BuildOrderComponent>,
}

impl BuildOrderManager {
    pub fn new() -> Self {
        use BuildCondition::*;
        use BuildOrderAction::*;
        use UnitTypeId::*;
        Self {
            policies: vec![
                Policy::new(
                    Train(Probe, AbilityId::NexusTrainProbe),
                    vec![LessThanCount(Probe, 14), LessThanCount(Pylon, 1)],
                ),
                Policy::new(
                    Train(Probe, AbilityId::NexusTrainProbe),
                    vec![
                        StructureComplete(UnitTypeId::Pylon),
                        LessThanCount(Probe, 48),
                    ],
                ),
                Policy::new(
                    Construct(Pylon),
                    vec![AtLeastCount(Probe, 13), SupplyLeftBelow(4)],
                ),
                Policy::new(
                    Construct(Gateway),
                    vec![StructureComplete(Pylon), LessThanCount(Gateway, 4)],
                ),
                Policy::new(
                    Chrono(AbilityId::NexusTrainProbe),
                    vec![SupplyBetween(16, 48)],
                ),
            ],
            build_order: Vec::new(),
        }
    }

    pub fn get_next_component(&self) -> Option<BuildOrderComponent> {
        Some(self.build_order.first()?.clone())
    }

    pub fn mark_component_done(&mut self) {
        self.build_order.remove(0);
    }

    pub fn add_component(&mut self, component: BuildOrderComponent) {
        self.build_order.push(component);
    }
}

#[derive(Debug, Clone)]
pub enum BuildCondition {
    SupplyAtLeast(u32),
    SupplyBetween(u32, u32),
    SupplyLeftBelow(u32),
    TechComplete(UpgradeId),
    StructureComplete(UnitTypeId),
    LessThanCount(UnitTypeId, usize),
    AtLeastCount(UnitTypeId, usize),
}
#[derive(Debug, Clone)]
pub enum BuildOrderAction {
    Train(UnitTypeId, AbilityId),
    Construct(UnitTypeId),
    Chrono(AbilityId),
    Research(UpgradeId, UnitTypeId, AbilityId),
}
#[derive(Clone)]
pub struct BuildOrderComponent {
    pub conditions: Vec<BuildCondition>,
    pub action: BuildOrderAction,
}

#[derive(Clone)]
pub struct Policy {
    pub action: BuildOrderAction,
    pub active: bool,
    pub conditions: Vec<BuildCondition>,
}
impl fmt::Display for Policy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, {:?}, {:?}",
            self.active, self.action, self.conditions
        )
    }
}
impl Policy {
    pub fn new(action: BuildOrderAction, conditions: Vec<BuildCondition>) -> Self {
        Policy {
            action,
            conditions,
            active: true,
        }
    }
}

impl ReBiCycler {
    pub fn step_build(&mut self) {
        self.progress_build();
        self.check_policies();
    }

    fn progress_build(&mut self) -> Option<()> {
        let next_task = self.bom.get_next_component()?.clone();
        if self.evaluate_conditions(&next_task.conditions)
            && self.can_do_build_action(&next_task.action)
        {
            self.attempt_build_action(&next_task.action)
        }
        None
    }

    fn can_do_build_action(&self, action: &BuildOrderAction) -> bool {
        match action {
            BuildOrderAction::Chrono(_) => self
                .units
                .my
                .townhalls
                .iter()
                .any(|n| n.energy().unwrap_or(0) >= CHRONOBOOST_COST),
            BuildOrderAction::Construct(building) => {
                let afford = self.can_afford(*building, true);
                let has_worker = !self.units.my.workers.is_empty();
                afford && has_worker
            }
            BuildOrderAction::Research(upgrade, reseacher, _) => {
                self.units
                    .my
                    .structures
                    .of_type(*reseacher)
                    .idle()
                    .is_empty()
                    && self.can_afford_upgrade(*upgrade)
            }
            BuildOrderAction::Train(_, ability) => {
                let has_trainer = self
                    .units
                    .my
                    .structures
                    .iter()
                    .any(|s| s.has_ability(*ability));
                has_trainer
            }
        }
    }

    fn check_policies(&mut self) {
        let doable_policies: Vec<Policy> = self
            .bom
            .policies
            .iter()
            .filter(|policy| {
                policy.active
                    && self.evaluate_conditions(&policy.conditions)
                    && self.can_do_build_action(&policy.action)
            })
            .cloned()
            .collect();

        let _: () = doable_policies
            .iter()
            .map(|policy| self.attempt_build_action(&policy.action))
            .collect();
    }

    fn evaluate_conditions(&self, conditions: &Vec<BuildCondition>) -> bool {
        conditions.iter().all(|condition| match condition {
            BuildCondition::SupplyAtLeast(supply) => self.supply_used >= *supply,
            BuildCondition::SupplyBetween(low, high) => {
                self.supply_used >= *low && self.supply_used < *high
            }
            BuildCondition::LessThanCount(unit_type, desired_count) => {
                let unit_count = self.counter().count(*unit_type);
                unit_count < *desired_count
            }
            BuildCondition::SupplyLeftBelow(remaining_supply) => {
                self.supply_left < *remaining_supply
            }
            BuildCondition::StructureComplete(structure_type) => self
                .units
                .my
                .structures
                .iter()
                .ready()
                .any(|u| u.type_id() == *structure_type),
            BuildCondition::TechComplete(upgrade) => self.upgrade_progress(*upgrade) > 0.95,
            BuildCondition::AtLeastCount(unit_type, desired_count) => {
                self.counter().count(*unit_type) >= *desired_count
            }
        })
    }

    fn attempt_build_action(&mut self, action: &BuildOrderAction) {
        let result = match action {
            BuildOrderAction::Construct(unit_type) => self.build(unit_type),
            BuildOrderAction::Train(unit_type, _) => self.train(*unit_type),
            BuildOrderAction::Chrono(ability) => self.chrono_boost(*ability),
            BuildOrderAction::Research(upgrade, researcher, ability) => {
                self.research(*researcher, *upgrade, *ability)
            }
        };

        result.inspect_err(|err| match err {
            BuildError::CantPlace(location, _type_id) => {
                self.siting_director.mark_position_blocked(location)
            }
            _ => println!("Build order blocked: {:?} > {:?}", action, err),
        });
    }

    fn train(&self, unit_type: UnitTypeId) -> Result<(), BuildError> {
        self.units
            .my
            .townhalls
            .first()
            .ok_or(BuildError::NoTrainer)?
            .train(unit_type, false);
        Ok(())
    }

    fn chrono_boost(&self, ability: AbilityId) -> Result<(), BuildError> {
        let nexus = self
            .units
            .my
            .townhalls
            .iter()
            .find(|unit| unit.energy().unwrap_or(0) >= CHRONOBOOST_COST)
            .ok_or(BuildError::CantAfford)?;
        let target = self
            .units
            .my
            .structures
            .iter()
            .find(|s| s.is_using(ability) && !s.has_buff(BuffId::ChronoBoostEnergyCost))
            .ok_or(BuildError::NoTrainer)?;

        nexus.command(
            AbilityId::EffectChronoBoost,
            Target::Tag(target.tag()),
            false,
        );
        Ok(())
    }

    pub fn research(
        &self,
        researcher: UnitTypeId,
        upgrade: UpgradeId,
        ability: AbilityId,
    ) -> Result<(), BuildError> {
        if self.has_upgrade(upgrade) && self.is_ordered_upgrade(upgrade) {
            Err(BuildError::AlreadyResearching)
        } else {
            let researcher = self
                .units
                .my
                .all
                .iter()
                .of_type(researcher)
                .ready()
                .idle()
                .next()
                .ok_or(BuildError::NoTrainer)?;

            if self.can_afford_upgrade(upgrade) {
                researcher.use_ability(ability, true);
                Ok(())
            } else {
                Err(BuildError::CantAfford)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    const fn test_build_order_manager() {}
}
