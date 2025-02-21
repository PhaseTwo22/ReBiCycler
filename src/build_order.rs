use core::fmt;

use rust_sc2::prelude::*;

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
                    prereq: BuildCondition::Supply(14),
                    action: BuildOrderAction::Construct(UnitTypeId::Pylon),
                },
                BuildOrderComponent {
                    prereq: BuildCondition::Structure(UnitTypeId::Pylon),
                    action: BuildOrderAction::Construct(UnitTypeId::Gateway),
                },
                BuildOrderComponent {
                    prereq: BuildCondition::Tech(UpgradeId::ProtossShieldsLevel2),
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
                    condition: BuildCondition::Supply(22),
                },
                Policy {
                    action: BuildOrderAction::Train(
                        UnitTypeId::Zealot,
                        AbilityId::GatewayTrainZealot,
                    ),
                    active: true,
                    condition: BuildCondition::Supply(22),
                },
                Policy {
                    action: BuildOrderAction::Chrono(AbilityId::GatewayTrainZealot),
                    active: true,
                    condition: BuildCondition::Count(UnitTypeId::Zealot, 20),
                },
                Policy {
                    action: BuildOrderAction::Construct(UnitTypeId::Pylon),
                    active: true,
                    condition: BuildCondition::SupplyLeft(4),
                },
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
    Supply(u32),
    SupplyLeft(u32),
    Tech(UpgradeId),
    Structure(UnitTypeId),
    Count(UnitTypeId, usize),
}
#[derive(Debug)]
pub enum BuildOrderAction {
    Train(UnitTypeId, AbilityId),
    Construct(UnitTypeId),
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_build_order_manager() {}
}
