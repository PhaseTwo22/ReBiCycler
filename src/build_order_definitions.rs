use rust_sc2::ids::{AbilityId, UnitTypeId, UpgradeId};

use crate::{
    build_orders::{BuildCondition, BuildOrderAction},
    build_tree::{BuildComponent, BuildOrderTree, ConditionGroup, ConditionOperator, TreeError},
};

/// nexus first, get warpgate, then tech to twilight, research charge, then 8 gates
pub fn nexus_first_two_base_charge() -> Result<BuildOrderTree, TreeError> {
    let tree = nexus_first()?
        .empty_root(Some("graft"))?
        .subtree(make_units()?)?
        .subtree(straight_to_twilight()?)?
        .subtree(get_charge_and_plus_one()?)?
        .tree;
    Ok(tree)
}

fn straight_to_twilight() -> Result<TreePointer, TreeError> {
    use BuildCondition as C;
    use BuildOrderAction as A;
    use ConditionOperator as Op;

    TreePointer::new()
        .root(
            "gas 1&2",
            &[ConditionGroup::new(
                &[C::AtLeastCount(UnitTypeId::Gateway, 2)],
                Op::All,
            )],
            &[ConditionGroup::new(
                &[C::AtLeastCount(UnitTypeId::Assimilator, 2)],
                Op::All,
            )],
            true,
            Some(A::Construct(UnitTypeId::Assimilator)),
            true,
        )?
        .child(
            "cybercore",
            &[ConditionGroup::new(
                &[
                    C::StructureComplete(UnitTypeId::Gateway),
                    C::StructureComplete(UnitTypeId::WarpGate),
                ],
                Op::Any,
            )],
            &[ConditionGroup::new(
                &[C::AtLeastCount(UnitTypeId::CyberneticsCore, 1)],
                Op::All,
            )],
            true,
            Some(A::Construct(UnitTypeId::CyberneticsCore)),
            true,
        )?
        .leaf(
            "warpgate",
            &[ConditionGroup::new(
                &[C::StructureComplete(UnitTypeId::CyberneticsCore)],
                Op::All,
            )],
            &[ConditionGroup::new(
                &[C::TechComplete(UpgradeId::WarpGateResearch)],
                Op::All,
            )],
            false,
            Some(A::Research(
                UpgradeId::WarpGateResearch,
                AbilityId::ResearchWarpGate,
                UnitTypeId::CyberneticsCore,
            )),
            true,
        )?
        .child(
            "twilight",
            &[ConditionGroup::new(
                &[C::StructureComplete(UnitTypeId::CyberneticsCore)],
                Op::Any,
            )],
            &[ConditionGroup::new(
                &[C::AtLeastCount(UnitTypeId::TwilightCouncil, 1)],
                Op::All,
            )],
            true,
            Some(A::Construct(UnitTypeId::TwilightCouncil)),
            true,
        )
}
/// an opener: probes to 14, pylon, resume probes, nexus, then two gateways.
fn nexus_first() -> Result<TreePointer, TreeError> {
    use BuildCondition as C;
    use BuildOrderAction as A;
    use ConditionOperator as Op;
    TreePointer::new()
        .root(
            "probe to 14",
            &[ConditionGroup::new(&[C::Always], Op::All)],
            &[ConditionGroup::new(&[C::SupplyBetween(0, 15)], Op::NoneOf)],
            true,
            Some(A::Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe)),
            true,
        )?
        .child(
            "first pylon",
            &[ConditionGroup::new(&[C::Always], Op::All)],
            &[ConditionGroup::new(
                &[C::AtLeastCount(UnitTypeId::Pylon, 1)],
                Op::All,
            )],
            true,
            Some(A::Construct(UnitTypeId::Pylon)),
            true,
        )?
        .child(
            "nexus first",
            &[ConditionGroup::new(&[C::Always], Op::All)],
            &[ConditionGroup::new(
                &[C::AtLeastCount(UnitTypeId::Nexus, 2)],
                Op::All,
            )],
            true,
            Some(A::Construct(UnitTypeId::Nexus)),
            true,
        )?
        .leaf(
            "probe to 38",
            &[ConditionGroup::new(
                &[C::AtLeastCount(UnitTypeId::Gateway, 1)],
                Op::All,
            )],
            &[ConditionGroup::new(
                &[C::TotalAndOrderedAtLeast(UnitTypeId::Probe, 38)],
                Op::All,
            )],
            false,
            Some(A::Train(UnitTypeId::Probe, AbilityId::NexusTrainProbe)),
            true,
        )?
        .child(
            "two gateways",
            &[ConditionGroup::new(&[C::Always], Op::All)],
            &[ConditionGroup::new(
                &[
                    C::AtLeastCount(UnitTypeId::Gateway, 2),
                    C::TechComplete(UpgradeId::WarpGateResearch),
                ],
                Op::Any,
            )],
            false,
            Some(A::Construct(UnitTypeId::Gateway)),
            true,
        )
}

fn make_units() -> Result<TreePointer, TreeError> {
    use BuildCondition as C;
    use BuildOrderAction as A;
    use ConditionOperator as Op;
    TreePointer::new()
        .empty_root(Some("units"))?
        .child(
            "two zealots",
            &[ConditionGroup::new(&[C::Always], Op::All)],
            &[ConditionGroup::new(
                &[
                    C::TotalAndOrderedAtLeast(UnitTypeId::Zealot, 2),
                    C::TechComplete(UpgradeId::WarpGateResearch),
                ],
                Op::Any,
            )],
            false,
            Some(A::Train(
                UnitTypeId::Stalker,
                AbilityId::GatewayTrainStalker,
            )),
            true,
        )?
        .child(
            "safety stalkers",
            &[ConditionGroup::new(&[C::Always], Op::All)],
            &[ConditionGroup::new(
                &[
                    C::TotalAndOrderedAtLeast(UnitTypeId::Stalker, 6),
                    C::TechComplete(UpgradeId::WarpGateResearch),
                ],
                Op::Any,
            )],
            false,
            Some(A::Train(
                UnitTypeId::Stalker,
                AbilityId::GatewayTrainStalker,
            )),
            true,
        )?
        .child(
            "safety stalkers WG",
            &[ConditionGroup::new(&[C::Always], Op::All)],
            &[ConditionGroup::new(
                &[C::TotalAndOrderedAtLeast(UnitTypeId::Stalker, 6)],
                Op::Any,
            )],
            true,
            Some(A::Train(
                UnitTypeId::Stalker,
                AbilityId::WarpGateTrainStalker,
            )),
            true,
        )?
        .child(
            "zealots forever",
            &[ConditionGroup::new(&[C::Always], Op::All)],
            &[ConditionGroup::new(&[C::Never], Op::All)],
            false,
            Some(A::Train(UnitTypeId::Zealot, AbilityId::WarpGateTrainZealot)),
            true,
        )
}

fn get_charge_and_plus_one() -> Result<TreePointer, TreeError> {
    use BuildCondition as C;
    use BuildOrderAction as A;
    use ConditionOperator as Op;
    TreePointer::new()
        .empty_root(None)?
        .leaf(
            "charge",
            &[ConditionGroup::new(
                &[C::StructureComplete(UnitTypeId::TwilightCouncil)],
                Op::All,
            )],
            &[ConditionGroup::new(
                &[C::TechComplete(UpgradeId::Charge)],
                Op::All,
            )],
            false,
            Some(A::Research(
                UpgradeId::Charge,
                AbilityId::ResearchCharge,
                UnitTypeId::TwilightCouncil,
            )),
            true,
        )?
        .child(
            "forge",
            &[ConditionGroup::new(
                &[C::AtLeastCount(UnitTypeId::TwilightCouncil, 1)],
                Op::All,
            )],
            &[ConditionGroup::new(
                &[C::AtLeastCount(UnitTypeId::Forge, 1)],
                Op::All,
            )],
            false,
            Some(A::Construct(UnitTypeId::Forge)),
            true,
        )?
        .child(
            "plus 1",
            &[ConditionGroup::new(
                &[C::StructureComplete(UnitTypeId::TwilightCouncil)],
                Op::All,
            )],
            &[ConditionGroup::new(
                &[C::TechComplete(UpgradeId::ProtossGroundWeaponsLevel1)],
                Op::All,
            )],
            false,
            Some(A::Research(
                UpgradeId::ProtossGroundWeaponsLevel1,
                AbilityId::ForgeResearchProtossGroundWeaponsLevel1,
                UnitTypeId::Forge,
            )),
            true,
        )?
        .child(
            "plus 2",
            &[ConditionGroup::new(
                &[C::TechComplete(UpgradeId::ProtossGroundWeaponsLevel1)],
                Op::All,
            )],
            &[ConditionGroup::new(
                &[C::TechComplete(UpgradeId::ProtossGroundWeaponsLevel2)],
                Op::All,
            )],
            false,
            Some(A::Research(
                UpgradeId::ProtossGroundWeaponsLevel2,
                AbilityId::ForgeResearchProtossGroundWeaponsLevel2,
                UnitTypeId::Forge,
            )),
            true,
        )?
        .child(
            "plus 3",
            &[ConditionGroup::new(
                &[C::TechComplete(UpgradeId::ProtossGroundWeaponsLevel2)],
                Op::All,
            )],
            &[ConditionGroup::new(
                &[C::TechComplete(UpgradeId::ProtossGroundWeaponsLevel3)],
                Op::All,
            )],
            false,
            Some(A::Research(
                UpgradeId::ProtossGroundWeaponsLevel3,
                AbilityId::ForgeResearchProtossGroundWeaponsLevel3,
                UnitTypeId::Forge,
            )),
            true,
        )
}

struct TreePointer {
    tree: BuildOrderTree,
    index: usize,
}

impl TreePointer {
    const fn new() -> Self {
        Self {
            tree: BuildOrderTree::new(),
            index: 0,
        }
    }
    /// Adds a node to the tree with no parent
    fn root(
        mut self,
        name: &str,
        start: &[ConditionGroup],
        end: &[ConditionGroup],
        restrictive: bool,
        action: Option<BuildOrderAction>,
        display: bool,
    ) -> Result<TreePointer, TreeError> {
        let _ = self.tree.add_node(
            BuildComponent::new(name, start, end, restrictive, action, display),
            None,
        )?;

        Ok(Self {
            index: self.tree.len() - 1,
            tree: self.tree,
        })
    }
    /// Adds a new node to the tree as a child of the current node but leaves pointer at current node
    fn leaf(
        mut self,
        name: &str,
        start: &[ConditionGroup],
        end: &[ConditionGroup],
        restrictive: bool,
        action: Option<BuildOrderAction>,
        display: bool,
    ) -> Result<Self, TreeError> {
        let _ = self.tree.add_node(
            BuildComponent::new(name, start, end, restrictive, action, display),
            Some(self.index),
        )?;
        Ok(self)
    }
    /// Adds a node to the tree as a child of the current node and points to the new node
    fn child(
        mut self,
        name: &str,
        start: &[ConditionGroup],
        end: &[ConditionGroup],
        restrictive: bool,
        action: Option<BuildOrderAction>,
        display: bool,
    ) -> Result<Self, TreeError> {
        self.tree.add_node(
            BuildComponent::new(name, start, end, restrictive, action, display),
            Some(self.index),
        )?;
        Ok(Self {
            index: self.tree.len() - 1,
            tree: self.tree,
        })
    }

    /// Combines the two trees, with all roots becoming children of the current node
    fn subtree(mut self, other: Self) -> Result<Self, TreeError> {
        for other_index in other.tree.breadth_first() {
            if let Some(node) = other.tree.get(other_index) {
                self.tree.add_node(
                    node.value.clone(),
                    node.parent
                        .map_or(Some(self.index), |p| Some(p + self.index + 1)),
                )?;
            }
        }
        Ok(self)
    }

    /// Adds a root node that does nothing, with the given name, if any.
    fn empty_root(self, name: Option<&str>) -> Result<Self, TreeError> {
        self.root(
            name.unwrap_or("empty_root"),
            &[],
            &[],
            false,
            None,
            name.is_some(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_root() {
        let pointer = TreePointer::new();
        let tree = pointer
            .root("first", &[], &[], false, None, true)
            .expect("Tree construction failed!");
    }

    #[test]
    fn two_roots() {
        let pointer = TreePointer::new();

        let tree = pointer
            .root("first", &[], &[], false, None, true)
            .and_then(|t| t.root("second", &[], &[], false, None, true))
            .expect("Tree construction failed!")
            .tree;

        assert_eq!(tree.to_string(), "first➖\nsecond➖\n");
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn one_child() {
        let pointer = TreePointer::new();
        let tree = pointer
            .root("first", &[], &[], false, None, true)
            .and_then(|t| t.child("child", &[], &[], false, None, true))
            .expect("Tree construction failed!")
            .tree;
        assert_eq!(tree.to_string(), "first➖\n-child➖\n");
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn one_leaf_one_child_one_leaf() {
        let pointer = TreePointer::new();
        let tree = pointer
            .root("first", &[], &[], false, None, true)
            .and_then(|t| t.leaf("leaf1", &[], &[], false, None, true))
            .and_then(|t| t.child("child", &[], &[], false, None, true))
            .and_then(|t| t.leaf("leaf2", &[], &[], false, None, true))
            .expect("Tree construction failed!")
            .tree;
        assert_eq!(tree.to_string(), "first➖\n-child➖\n--leaf2➖\n-leaf1➖\n");
        assert_eq!(tree.len(), 4);
    }

    #[test]
    fn subtree_ok() {
        let mut pointer = TreePointer::new()
            .root("first", &[], &[], false, None, true)
            .and_then(|t| t.child("child", &[], &[], false, None, true))
            .expect("Tree construction failed!");

        let subtree = TreePointer::new()
            .root("subroot", &[], &[], false, None, true)
            .and_then(|t| t.leaf("subleaf", &[], &[], false, None, true))
            .and_then(|t| t.leaf("subleaf2", &[], &[], false, None, true))
            .expect("Tree construction failed!");

        assert_eq!(
            subtree.tree.to_string(),
            "subroot➖\n-subleaf2➖\n-subleaf➖\n"
        );

        pointer = pointer.subtree(subtree).expect("Tree construction failed!");

        assert_eq!(
            pointer.tree.to_string(),
            "first➖\n-child➖\n--subroot➖\n---subleaf2➖\n---subleaf➖\n"
        );
        assert_eq!(pointer.tree.len(), 5);

        let sub_sub_tree = TreePointer::new()
            .root("subsubroot", &[], &[], false, None, true)
            .and_then(|t| t.leaf("subsubleaf", &[], &[], false, None, true))
            .and_then(|t| t.leaf("subsubleaf2", &[], &[], false, None, true))
            .expect("Tree construction failed!");

        pointer = pointer
            .child("graft", &[], &[], false, None, true)
            .and_then(|t| t.subtree(sub_sub_tree))
            .expect("Tree construction failed!");

        assert_eq!(
            pointer.tree.to_string(),
            "first➖\n-child➖\n--graft➖\n---subsubroot➖\n----subsubleaf2➖\n----subsubleaf➖\n--subroot➖\n---subleaf2➖\n---subleaf➖\n"
        );
    }

    #[test]
    fn double_subtree() {
        let mut pointer = TreePointer::new()
            .root("first", &[], &[], false, None, true)
            .and_then(|t| t.child("child", &[], &[], false, None, true))
            .expect("Tree construction failed!");

        let subtree = TreePointer::new()
            .root("1subroot", &[], &[], false, None, true)
            .and_then(|t| t.leaf("1subleaf", &[], &[], false, None, true))
            .and_then(|t| t.leaf("1subleaf2", &[], &[], false, None, true))
            .expect("Tree construction failed!");

        let subtree2 = TreePointer::new()
            .root("2subroot", &[], &[], false, None, true)
            .and_then(|t| t.leaf("2subleaf", &[], &[], false, None, true))
            .and_then(|t| t.leaf("2subleaf2", &[], &[], false, None, true))
            .expect("Tree construction failed!");

        pointer = pointer
            .subtree(subtree)
            .expect("Tree construction failed!")
            .subtree(subtree2)
            .expect("Tree construction failed!");

        assert_eq!(
            pointer.tree.to_string(),
            "first➖\n-child➖\n--graft➖\n---subsubroot➖\n----subsubleaf2➖\n----subsubleaf➖\n--subroot➖\n---subleaf2➖\n---subleaf➖\n"
        );
    }
}
