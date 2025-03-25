use std::vec;

use rust_sc2::prelude::{AbilityId, UnitTypeId, UpgradeId};

use crate::build_order_manager::BuildOrder;

#[derive(Debug, Clone)]
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
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildOrderAction {
    Train(UnitTypeId, AbilityId),
    Construct(UnitTypeId),
    Chrono(AbilityId),
    Research(UpgradeId, UnitTypeId, AbilityId),
    Expand,
}
#[derive(Clone)]
pub struct BuildOrderComponent {
    pub name: String,
    pub start_conditions: Vec<BuildCondition>,
    pub end_conditions: Vec<BuildCondition>,
    pub action: BuildOrderAction,
}
impl BuildOrderComponent {
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
                });
            } else {
                out.push(Self {
                    action: *action,
                    name: name.clone(),
                    end_conditions: last_end_conditions.to_vec(),
                    start_conditions: start_conditions.to_vec(),
                });
            }
        }
        out
    }
}

#[allow(clippy::too_many_lines, clippy::enum_glob_use)]
pub fn four_base_charge() -> BuildOrder {
    use BuildCondition::*;
    use BuildOrderAction::*;
    use BuildOrderComponent as Component;
    let mut parts = BuildOrderComponent::consecutive(
        &[
            (
                "Build a nexus if we don't have any".to_string(),
                &[
                    DontHaveAnyStarted(UnitTypeId::Nexus),
                    DontHaveAnyDone(UnitTypeId::Nexus),
                ],
                Expand,
            ),
            (
                "Probe up to 14".to_string(),
                &[
                    LessThanCount(UnitTypeId::Probe, 14),
                    AtLeastCount(UnitTypeId::Nexus, 1),
                ],
                Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe),
            ),
            (
                "Build first pylon".to_string(),
                &[
                    AtLeastCount(UnitTypeId::Probe, 14),
                    DontHaveAnyStarted(UnitTypeId::Pylon),
                ],
                Construct(UnitTypeId::Pylon),
            ),
        ],
        &[AtLeastCount(UnitTypeId::Pylon, 1)],
    );

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
        },
        Component {
            name: "Take 4 gasses".to_string(),
            start_conditions: vec![
                AtLeastCount(UnitTypeId::Pylon, 1),
                AtLeastCount(UnitTypeId::Nexus, 1),
                AtLeastCount(UnitTypeId::Probe, 16),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::Assimilator, 4)],
            action: Construct(UnitTypeId::Assimilator),
        },
        Component {
            name: "Probe to 48".to_string(),
            start_conditions: vec![AtLeastCount(UnitTypeId::Pylon, 1)],
            end_conditions: vec![AtLeastCount(UnitTypeId::Probe, 48)],
            action: Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe),
        },
        Component {
            name: "Expand to 4 bases".to_string(),
            start_conditions: vec![
                AtLeastCount(UnitTypeId::Pylon, 1),
                AtLeastCount(UnitTypeId::Nexus, 1),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::Nexus, 4)],
            action: Expand,
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
        },
        Component {
            name: "Build cyber core".to_string(),
            start_conditions: vec![
                StructureComplete(UnitTypeId::Gateway),
                AtLeastCount(UnitTypeId::Nexus, 1),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::CyberneticsCore, 1)],
            action: Construct(UnitTypeId::CyberneticsCore),
        },
        Component {
            name: "Reseach warpgate".to_string(),
            start_conditions: vec![StructureComplete(UnitTypeId::CyberneticsCore)],
            end_conditions: vec![TechComplete(UpgradeId::WarpGateResearch)],
            action: Research(
                UpgradeId::WarpGateResearch,
                UnitTypeId::CyberneticsCore,
                AbilityId::ResearchWarpGate,
            ),
        },
        Component {
            name: "Build twilight council".to_string(),
            start_conditions: vec![
                StructureComplete(UnitTypeId::CyberneticsCore),
                AtLeastCount(UnitTypeId::Nexus, 1),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::TwilightCouncil, 1)],
            action: Construct(UnitTypeId::TwilightCouncil),
        },
        Component {
            name: "Reseach charge".to_string(),
            start_conditions: vec![
                StructureComplete(UnitTypeId::TwilightCouncil),
                AtLeastCount(UnitTypeId::Nexus, 1),
            ],
            end_conditions: vec![TechComplete(UpgradeId::Charge)],
            action: Research(
                UpgradeId::Charge,
                UnitTypeId::TwilightCouncil,
                AbilityId::ResearchCharge,
            ),
        },
        Component {
            name: "Chrono charge".to_string(),
            start_conditions: vec![StructureComplete(UnitTypeId::TwilightCouncil)],
            end_conditions: vec![TechComplete(UpgradeId::Charge)],
            action: Chrono(AbilityId::ResearchCharge),
        },
    ];

    let mut add_more_gateways = vec![
        Component {
            name: "Up to 6 gates".to_string(),
            start_conditions: vec![AtLeastCount(UnitTypeId::Nexus, 2)],
            end_conditions: vec![AtLeastCount(UnitTypeId::Gateway, 6)],
            action: Construct(UnitTypeId::Gateway),
        },
        Component {
            name: "Up to 12 gates".to_string(),
            start_conditions: vec![
                AtLeastCount(UnitTypeId::Nexus, 3),
                AtLeastCount(UnitTypeId::TwilightCouncil, 1),
            ],
            end_conditions: vec![AtLeastCount(UnitTypeId::Gateway, 6)],
            action: Construct(UnitTypeId::Gateway),
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
                LessThanCount(UnitTypeId::Zealot, 3),
            ],
            action: Train(UnitTypeId::Zealot, AbilityId::GatewayTrainZealot),
        },
        Component {
            name: "Make zealots indefinitely".to_string(),
            start_conditions: vec![StructureComplete(UnitTypeId::WarpGate)],
            end_conditions: vec![],
            action: Train(UnitTypeId::Zealot, AbilityId::GatewayTrainZealot),
        },
    ];
    parts.append(&mut expand_and_probe);
    parts.append(&mut get_charge);
    parts.append(&mut add_more_gateways);
    parts.append(&mut train_zealots);
    BuildOrder(parts)
}
