use core::fmt;

use rust_sc2::prelude::*;

use crate::protoss_bot::ReBiCycler;

const CHRONOBOOST_COST: u32 = 50;
/// This module serves to manage our build orders.
/// We want to use kiss principle here, but still have a flexible system.
///
/// A build order is made of components, each with a condition and an action.
/// It also contains policies, which are a set of actions that are executed until a condition is met.
/// Policies can be active and inactive.

#[derive(Default)]
pub struct BuildOrderManager {
    pub build_order: Vec<BuildOrderComponent>,
    pub policies: Vec<Policy>,
}

impl BuildOrderManager {
    pub fn new() -> Self {
        Self {
            build_order: vec![
                BuildOrderComponent {
                    prereq: BuildCondition::SupplyAtLeast(14),
                    action: BuildOrderAction::Construct(
                        UnitTypeId::Pylon,
                        AbilityId::ProtossBuildPylon,
                    ),
                },
                BuildOrderComponent {
                    prereq: BuildCondition::StructureComplete(UnitTypeId::Pylon),
                    action: BuildOrderAction::Construct(
                        UnitTypeId::Gateway,
                        AbilityId::ProtossBuildGateway,
                    ),
                },
                BuildOrderComponent {
                    prereq: BuildCondition::TechComplete(UpgradeId::ProtossShieldsLevel2),
                    action: BuildOrderAction::Research(
                        UpgradeId::ProtossShieldsLevel3,
                        UnitTypeId::Forge,
                        AbilityId::ForgeResearchProtossShieldsLevel3,
                    ),
                },
            ],
            policies: vec![
                Policy {
                    action: BuildOrderAction::Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe),
                    active: true,
                    condition: BuildCondition::LessThanCount(UnitTypeId::Probe, 22),
                },
                // Policy {
                //     action: BuildOrderAction::Train(
                //         UnitTypeId::Zealot,
                //         AbilityId::GatewayTrainZealot,
                //     ),
                //     active: true,
                //     condition: BuildCondition::SupplyAtLeast(22),
                // },
                // Policy {
                //     action: BuildOrderAction::Chrono(AbilityId::GatewayTrainZealot),
                //     active: true,
                //     condition: BuildCondition::LessThanCount(UnitTypeId::Zealot, 20),
                // },
                // Policy {
                //     action: BuildOrderAction::Construct(UnitTypeId::Pylon),
                //     active: true,
                //     condition: BuildCondition::SupplyLeft(4),
                // },
            ],
        }
    }

    pub fn get_next_component(&self) -> Option<&BuildOrderComponent> {
        self.build_order.first()
    }

    pub fn mark_component_done(&mut self) {
        self.build_order.remove(0);
    }

    pub fn add_component(&mut self, component: BuildOrderComponent) {
        self.build_order.push(component);
    }
}

#[derive(Debug)]
pub enum BuildCondition {
    SupplyAtLeast(u32),
    SupplyBetween(u32, u32),
    SupplyLeft(u32),
    TechComplete(UpgradeId),
    StructureComplete(UnitTypeId),
    LessThanCount(UnitTypeId, usize),
}
#[derive(Debug)]
pub enum BuildOrderAction {
    Train(UnitTypeId, AbilityId),
    Construct(UnitTypeId, AbilityId),
    Chrono(AbilityId),
    Research(UpgradeId, UnitTypeId, AbilityId),
}

pub struct BuildOrderComponent {
    pub prereq: BuildCondition,
    pub action: BuildOrderAction,
}

pub struct Policy {
    pub action: BuildOrderAction,
    pub active: bool,
    pub condition: BuildCondition,
}
impl fmt::Display for Policy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, {:?}, {:?}",
            self.active, self.action, self.condition
        )
    }
}

impl ReBiCycler {
    pub fn step_build(&mut self) {
        self.progress_build();
        self.check_policies();
    }

    fn progress_build(&mut self) {
        if let Some(next_task) = self.bom.get_next_component() {
            if self.can_do_build_action(&next_task.action)
                && self.evaluate_condition(&next_task.prereq)
            {
                self.attempt_build_action(&next_task.action);
                println!("we started a build action!");
                //self.bom.mark_component_done();
            }
        }
    }

    fn can_do_build_action(&self, action: &BuildOrderAction) -> bool {
        match action {
            BuildOrderAction::Chrono(_) => self
                .units
                .my
                .townhalls
                .iter()
                .any(|n| n.energy().unwrap_or(0) >= CHRONOBOOST_COST),
            BuildOrderAction::Construct(building, ability) => {
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

    fn check_policies(&self) {
        let mut attempted_policies = 0;
        for policy in &self.bom.policies {
            if !policy.active {
                continue;
            }
            if !self.evaluate_condition(&policy.condition) {
                continue;
            }
            if !self.can_do_build_action(&policy.action) {
                continue;
            }
            self.attempt_build_action(&policy.action);
            //println!("Attempted Policy Action! {policy}");
            attempted_policies += 1;
        }

        if attempted_policies == 0 {
            //println!("No policies attempted");
        };
    }

    fn evaluate_condition(&self, condition: &BuildCondition) -> bool {
        match condition {
            BuildCondition::SupplyAtLeast(supply) => self.supply_used >= *supply,
            BuildCondition::SupplyBetween(low, high) => {
                self.supply_used >= *low && self.supply_used < *high
            }
            BuildCondition::LessThanCount(unit_type, desired_count) => {
                let unit_count = self.counter().count(*unit_type);
                unit_count < *desired_count
            }
            BuildCondition::SupplyLeft(remaining_supply) => self.supply_left < *remaining_supply,
            BuildCondition::StructureComplete(structure_type) => self
                .units
                .my
                .structures
                .iter()
                .any(|u| u.type_id() == *structure_type),
            BuildCondition::TechComplete(upgrade) => self.upgrade_progress(*upgrade) > 0.95,
        }
    }

    fn attempt_build_action(&self, action: &BuildOrderAction) {
        match action {
            BuildOrderAction::Construct(unit_type, ability) => {
                self.build(unit_type, ability);
            }
            BuildOrderAction::Train(unit_type, _) => self.train(*unit_type),
            BuildOrderAction::Chrono(ability) => self.chrono_boost(*ability),
            BuildOrderAction::Research(upgrade, researcher, ability) => {
                self.research(*researcher, *upgrade, *ability);
            }
        }
    }

    fn build(&self, structure_type: &UnitTypeId, build_ability: &AbilityId) {
        let construct_ability = self.game_data.units[structure_type].ability.unwrap();
        let footprint = self.game_data.abilities[&construct_ability]
            .footprint_radius
            .unwrap();
        //self.game_data.units[structure_type]
        let mut position = self.siting_director.get_building_site_choices(
            self,
            &footprint,
            structure_type,
            build_ability,
            self.start_location,
        );
        if let Some(position) = position.next() {
            let builder = self.units.my.workers.closest(position.location).unwrap();
            builder.build(*structure_type, position.location, false);
            builder.sleep(5);
        } else {
            println!("Unable to find build location for {:?}", structure_type)
        }
    }

    fn train(&self, unit_type: UnitTypeId) {
        let trainer = self.units.my.townhalls.first().unwrap();
        trainer.train(unit_type, false);
    }

    fn chrono_boost(&self, ability: AbilityId) {
        let mut energetic_nexi = self
            .units
            .my
            .townhalls
            .iter()
            .filter(|unit| unit.energy().unwrap_or(0) >= CHRONOBOOST_COST);
        let target = self
            .units
            .my
            .structures
            .iter()
            .find(|s| s.is_using(ability));
        if let (Some(nexus), Some(target)) = (energetic_nexi.next(), target) {
            nexus.command(
                AbilityId::EffectChronoBoost,
                Target::Tag(target.tag()),
                false,
            );
        }
    }

    pub fn research(&self, researcher: UnitTypeId, upgrade: UpgradeId, ability: AbilityId) {
        let researchers = self.units.my.all.of_type(researcher).ready().idle();
        if !researchers.is_empty()
            && !self.has_upgrade(upgrade)
            && !self.is_ordered_upgrade(upgrade)
        {
            if let Some(candidate) = researchers.first() {
                if self.can_afford_upgrade(upgrade) {
                    candidate.use_ability(ability, true);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    const fn test_build_order_manager() {}
}
