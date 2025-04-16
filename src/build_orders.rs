use std::{fmt::Display, vec};

use rust_sc2::prelude::{AbilityId, UnitTypeId, UpgradeId};

use crate::build_order_manager::BuildOrder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildCondition {
    SupplyAtLeast(u32),
    SupplyBetween(u32, u32),
    SupplyLeftBelow(u32),
    TechComplete(UpgradeId),
    StructureComplete(UnitTypeId),
    LessThanCount(UnitTypeId, usize),
    AtLeastCount(UnitTypeId, usize),
    DontHaveAnyDone(UnitTypeId),
    DontHaveAnyStarted(UnitTypeId),
    Never,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildOrderAction {
    Train(UnitTypeId, AbilityId),
    Construct(UnitTypeId),
    Chrono(AbilityId),
    ChronoWhatever(UnitTypeId),
    Research(UpgradeId, AbilityId, UnitTypeId),
    Expand,
    Surrender,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentState {
    NotYetStarted,
    Active,
    Completed,
}

impl Display for ComponentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            Self::NotYetStarted => "➖",
            Self::Active => "⏳",
            Self::Completed => "✅",
        }
        .to_string();
        write!(f, "{out}")
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BuildOrderComponent {
    pub name: String,
    pub start_conditions: Vec<BuildCondition>,
    pub end_conditions: Vec<BuildCondition>,
    pub action: BuildOrderAction,
    pub state: ComponentState,
}

impl Display for BuildOrderComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.state)
    }
}
impl BuildOrderComponent {
    pub fn new(
        name: &str,
        start_conditions: &[BuildCondition],
        end_conditions: &[BuildCondition],
        action: BuildOrderAction,
    ) -> Self {
        Self {
            name: name.to_string(),
            start_conditions: start_conditions.into(),
            end_conditions: end_conditions.into(),
            action,
            state: ComponentState::NotYetStarted,
        }
    }
    pub fn consecutive(
        chain: &[(String, &[BuildCondition], BuildOrderAction)],
        last_end_conditions: &[BuildCondition],
    ) -> Vec<Self> {
        let mut numbered_and_peeks = chain.iter().peekable();

        let mut out = Vec::new();
        while let Some(consecutive_part) = numbered_and_peeks.next() {
            let (name, start_conditions, action) = consecutive_part;
            if let Some((_, next_start, _)) = numbered_and_peeks.peek() {
                let end_conditions = next_start;

                out.push(Self {
                    action: *action,
                    name: name.clone(),
                    end_conditions: end_conditions.to_vec(),
                    start_conditions: start_conditions.to_vec(),
                    state: ComponentState::NotYetStarted,
                });
            } else {
                out.push(Self {
                    action: *action,
                    name: name.clone(),
                    end_conditions: last_end_conditions.to_vec(),
                    start_conditions: start_conditions.to_vec(),
                    state: ComponentState::NotYetStarted,
                });
            }
        }
        out
    }

    pub fn complete(&mut self) {
        self.state = ComponentState::Completed;
    }

    pub fn activate(&mut self) {
        self.state = ComponentState::Active;
    }
}

#[allow(clippy::too_many_lines, clippy::enum_glob_use)]
pub fn four_base_charge() -> BuildOrder {
    use BuildCondition::*;
    use BuildOrderAction::*;
    use BuildOrderComponent as Component;

    let mut surrender = vec![Component::new(
        "surrender",
        &[SupplyBetween(0, 1), LessThanCount(UnitTypeId::Nexus, 1)],
        &[Never],
        Surrender,
    )];
    let mut rebuild_from_nothing = vec![
        Component::new(
            "Build a nexus if we don't have any",
            &[
                DontHaveAnyStarted(UnitTypeId::Nexus),
                DontHaveAnyDone(UnitTypeId::Nexus),
            ],
            &[AtLeastCount(UnitTypeId::Nexus, 1)],
            Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe),
        ),
        Component::new(
            "Probe up to 10",
            &[LessThanCount(UnitTypeId::Probe, 10)],
            &[AtLeastCount(UnitTypeId::Probe, 10)],
            Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe),
        ),
    ];

    let mut opener = vec![
        BuildOrderComponent::new(
            "Probe up to 14",
            &[
                LessThanCount(UnitTypeId::Probe, 14),
                DontHaveAnyStarted(UnitTypeId::Probe),
            ],
            &[
                AtLeastCount(UnitTypeId::Probe, 14),
                AtLeastCount(UnitTypeId::Nexus, 1),
            ],
            Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe),
        ),
        BuildOrderComponent::new(
            "Build first pylon",
            &[
                AtLeastCount(UnitTypeId::Probe, 13),
                DontHaveAnyStarted(UnitTypeId::Pylon),
            ],
            &[AtLeastCount(UnitTypeId::Pylon, 1)],
            Construct(UnitTypeId::Pylon),
        ),
    ];

    let mut maintain_supply = vec![
        Component::new(
            "Maintain supply 1",
            &[SupplyLeftBelow(4), DontHaveAnyStarted(UnitTypeId::Pylon)],
            &[AtLeastCount(UnitTypeId::Gateway, 3)],
            Construct(UnitTypeId::Pylon),
        ),
        Component::new(
            "Maintain supply 2",
            &[SupplyLeftBelow(8), DontHaveAnyStarted(UnitTypeId::Pylon)],
            &[Never],
            Construct(UnitTypeId::Pylon),
        ),
    ];

    let mut expand_and_probe = vec![
        Component {
            name: "Chrono probes".to_string(),
            start_conditions: vec![
                AtLeastCount(UnitTypeId::Pylon, 1),
                AtLeastCount(UnitTypeId::Nexus, 1),
                LessThanCount(UnitTypeId::Probe, 40),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::Probe, 40)],
            action: Chrono(AbilityId::NexusTrainProbe),
            state: ComponentState::NotYetStarted,
        },
        Component {
            name: "Take 2 gasses".to_string(),
            start_conditions: vec![
                AtLeastCount(UnitTypeId::Pylon, 1),
                AtLeastCount(UnitTypeId::Nexus, 1),
                AtLeastCount(UnitTypeId::Probe, 16),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::Assimilator, 2)],
            action: Construct(UnitTypeId::Assimilator),
            state: ComponentState::NotYetStarted,
        },
        Component {
            name: "Probe to 40".to_string(),
            start_conditions: vec![AtLeastCount(UnitTypeId::Pylon, 1)],
            end_conditions: vec![AtLeastCount(UnitTypeId::Probe, 40)],
            action: Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe),
            state: ComponentState::NotYetStarted,
        },
        Component {
            name: "Expand to 2 bases".to_string(),
            start_conditions: vec![
                AtLeastCount(UnitTypeId::Pylon, 1),
                AtLeastCount(UnitTypeId::Nexus, 1),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::Nexus, 2)],
            action: Expand,
            state: ComponentState::NotYetStarted,
        },
        Component {
            name: "Up to 4 gasses".to_string(),
            start_conditions: vec![
                AtLeastCount(UnitTypeId::Pylon, 2),
                AtLeastCount(UnitTypeId::Nexus, 2),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::Assimilator, 4)],
            action: Construct(UnitTypeId::Assimilator),
            state: ComponentState::NotYetStarted,
        },
    ];

    let mut get_charge = vec![
        Component {
            name: "Build first gateway".to_string(),
            start_conditions: vec![
                StructureComplete(UnitTypeId::Pylon),
                AtLeastCount(UnitTypeId::Nexus, 1),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::Gateway, 1)],
            action: Construct(UnitTypeId::Gateway),
            state: ComponentState::NotYetStarted,
        },
        Component {
            name: "Build cyber core".to_string(),
            start_conditions: vec![
                StructureComplete(UnitTypeId::Gateway),
                AtLeastCount(UnitTypeId::Nexus, 1),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::CyberneticsCore, 1)],
            action: Construct(UnitTypeId::CyberneticsCore),
            state: ComponentState::NotYetStarted,
        },
        Component {
            name: "Reseach warpgate".to_string(),
            start_conditions: vec![StructureComplete(UnitTypeId::CyberneticsCore)],
            end_conditions: vec![TechComplete(UpgradeId::WarpGateResearch)],
            action: Research(
                UpgradeId::WarpGateResearch,
                AbilityId::ResearchWarpGate,
                UnitTypeId::CyberneticsCore,
            ),
            state: ComponentState::NotYetStarted,
        },
        Component {
            name: "Build twilight council".to_string(),
            start_conditions: vec![
                StructureComplete(UnitTypeId::CyberneticsCore),
                AtLeastCount(UnitTypeId::Nexus, 2),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::TwilightCouncil, 1)],
            action: Construct(UnitTypeId::TwilightCouncil),
            state: ComponentState::NotYetStarted,
        },
        Component {
            name: "Reseach charge".to_string(),
            start_conditions: vec![
                StructureComplete(UnitTypeId::TwilightCouncil),
                AtLeastCount(UnitTypeId::Nexus, 2),
            ],
            end_conditions: vec![TechComplete(UpgradeId::Charge)],
            action: Research(
                UpgradeId::Charge,
                AbilityId::ResearchCharge,
                UnitTypeId::TwilightCouncil,
            ),
            state: ComponentState::NotYetStarted,
        },
        Component {
            name: "Chrono charge".to_string(),
            start_conditions: vec![StructureComplete(UnitTypeId::TwilightCouncil)],
            end_conditions: vec![TechComplete(UpgradeId::Charge)],
            action: Chrono(AbilityId::ResearchCharge),
            state: ComponentState::NotYetStarted,
        },
    ];

    let mut add_more_gateways = vec![
        Component {
            name: "Up to 6 gates".to_string(),
            start_conditions: vec![AtLeastCount(UnitTypeId::Nexus, 2)],
            end_conditions: vec![AtLeastCount(UnitTypeId::Gateway, 6)],
            action: Construct(UnitTypeId::Gateway),
            state: ComponentState::NotYetStarted,
        },
        Component {
            name: "Up to 12 gates".to_string(),
            start_conditions: vec![
                AtLeastCount(UnitTypeId::Nexus, 3),
                AtLeastCount(UnitTypeId::TwilightCouncil, 1),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::Gateway, 6)],
            action: Construct(UnitTypeId::Gateway),
            state: ComponentState::NotYetStarted,
        },
    ];

    let mut train_zealots = vec![
        Component {
            name: "Train first 2 zealots".to_string(),
            start_conditions: vec![
                StructureComplete(UnitTypeId::Gateway),
                AtLeastCount(UnitTypeId::CyberneticsCore, 1),
            ],
            end_conditions: vec![
                TechComplete(UpgradeId::WarpGateResearch),
                AtLeastCount(UnitTypeId::Zealot, 2),
            ],
            action: Train(UnitTypeId::Zealot, AbilityId::GatewayTrainZealot),
            state: ComponentState::NotYetStarted,
        },
        Component {
            name: "Make zealots indefinitely".to_string(),
            start_conditions: vec![
                StructureComplete(UnitTypeId::WarpGate),
                AtLeastCount(UnitTypeId::Nexus, 2),
            ],
            end_conditions: vec![Never],
            action: Train(UnitTypeId::Zealot, AbilityId::WarpGateTrainZealot),
            state: ComponentState::NotYetStarted,
        },
    ];

    let mut get_upgrades = vec![
        Component::new(
            "Build Forges",
            &[AtLeastCount(UnitTypeId::TwilightCouncil, 1)],
            &[AtLeastCount(UnitTypeId::Forge, 2)],
            Construct(UnitTypeId::Forge),
        ),
        Component::new(
            "Chrono Forges",
            &[TechComplete(UpgradeId::Charge)],
            &[Never],
            ChronoWhatever(UnitTypeId::Forge),
        ),
        Component::new(
            "Armor 1",
            &[StructureComplete(UnitTypeId::Forge)],
            &[TechComplete(UpgradeId::ProtossGroundArmorsLevel1)],
            Research(
                UpgradeId::ProtossGroundArmorsLevel1,
                AbilityId::ForgeResearchProtossGroundArmorLevel1,
                UnitTypeId::Forge,
            ),
        ),
        Component::new(
            "Weapons 1",
            &[StructureComplete(UnitTypeId::Forge)],
            &[TechComplete(UpgradeId::ProtossGroundWeaponsLevel1)],
            Research(
                UpgradeId::ProtossGroundWeaponsLevel1,
                AbilityId::ForgeResearchProtossGroundWeaponsLevel1,
                UnitTypeId::Forge,
            ),
        ),
        Component::new(
            "Armor 2",
            &[TechComplete(UpgradeId::ProtossGroundArmorsLevel1)],
            &[TechComplete(UpgradeId::ProtossGroundArmorsLevel2)],
            Research(
                UpgradeId::ProtossGroundArmorsLevel2,
                AbilityId::ForgeResearchProtossGroundArmorLevel2,
                UnitTypeId::Forge,
            ),
        ),
        Component::new(
            "Weapons 2",
            &[TechComplete(UpgradeId::ProtossGroundWeaponsLevel1)],
            &[TechComplete(UpgradeId::ProtossGroundWeaponsLevel2)],
            Research(
                UpgradeId::ProtossGroundWeaponsLevel2,
                AbilityId::ForgeResearchProtossGroundWeaponsLevel2,
                UnitTypeId::Forge,
            ),
        ),
        Component::new(
            "Armor 3",
            &[TechComplete(UpgradeId::ProtossGroundArmorsLevel2)],
            &[TechComplete(UpgradeId::ProtossGroundArmorsLevel3)],
            Research(
                UpgradeId::ProtossGroundArmorsLevel3,
                AbilityId::ForgeResearchProtossGroundArmorLevel3,
                UnitTypeId::Forge,
            ),
        ),
        Component::new(
            "Weapons 3",
            &[TechComplete(UpgradeId::ProtossGroundWeaponsLevel2)],
            &[TechComplete(UpgradeId::ProtossGroundWeaponsLevel3)],
            Research(
                UpgradeId::ProtossGroundWeaponsLevel3,
                AbilityId::ForgeResearchProtossGroundWeaponsLevel3,
                UnitTypeId::Forge,
            ),
        ),
    ];

    let mut parts = Vec::new();
    parts.append(&mut surrender);
    parts.append(&mut rebuild_from_nothing);
    parts.append(&mut opener);
    parts.append(&mut maintain_supply);
    parts.append(&mut expand_and_probe);
    parts.append(&mut get_charge);
    parts.append(&mut get_upgrades);
    parts.append(&mut add_more_gateways);
    parts.append(&mut train_zealots);
    BuildOrder(parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consecutive_works_ok() {
        use BuildCondition::*;
        use BuildOrderAction::*;
        let parts = BuildOrderComponent::consecutive(
            &[
                (
                    "1".to_string(),
                    &[BuildCondition::AtLeastCount(UnitTypeId::Probe, 1)],
                    Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe),
                ),
                (
                    "2".to_string(),
                    &[BuildCondition::AtLeastCount(UnitTypeId::Probe, 2)],
                    Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe),
                ),
            ],
            &[AtLeastCount(UnitTypeId::Probe, 3)],
        );

        let actual = vec![
            BuildOrderComponent {
                name: "1".to_string(),
                start_conditions: vec![BuildCondition::AtLeastCount(UnitTypeId::Probe, 1)],
                end_conditions: vec![BuildCondition::AtLeastCount(UnitTypeId::Probe, 2)],
                action: BuildOrderAction::Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe),
                state: ComponentState::NotYetStarted,
            },
            BuildOrderComponent {
                name: "2".to_string(),
                start_conditions: vec![BuildCondition::AtLeastCount(UnitTypeId::Probe, 2)],
                end_conditions: vec![BuildCondition::AtLeastCount(UnitTypeId::Probe, 3)],
                action: BuildOrderAction::Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe),
                state: ComponentState::NotYetStarted,
            },
        ];

        assert_eq!(parts, actual);
    }
}
