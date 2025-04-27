use rust_sc2::ids::{AbilityId, UnitTypeId, UpgradeId};

use crate::{
    build_orders::{BuildCondition, BuildOrderAction},
    build_tree::{BuildComponent, BuildOrderTree, ConditionGroup, ConditionOperator},
};

/// nexus first, get warpgate, then tech to twilight, research charge, then 8 gates
pub fn nexus_first_two_base_charge() -> BuildOrderTree {
    nexus_first()
        .root("graft", &[], &[], false, None, true)
        .subtree(straight_to_twilight())
        .tree
}

fn straight_to_twilight() -> TreePointer {
    use BuildCondition as C;
    use BuildOrderAction as A;
    use ConditionOperator as Op;

    TreePointer::new().root("gas 1&2",
&[ConditionGroup::new([C::AtLeastCount(UnitTypeId::Gateway, 2)], Op::All)],
&[ConditionGroup::new([C::AtLeastCount(UnitTypeId::Assimilator,2)], Op::All)],
true,
Some(Construct(UnitTypeId::Assimilator)),
true)
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
        )
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
fn nexus_first() -> TreePointer {
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
        )
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
        )
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
        ).leaf(
"probe to 38",
&[ConditionGroup::new(&[C::AtLeastCount(UnitTypeId::Gateway, 1)])],
&[ConditionGroup::new(&[C::TotalAndOrdered(UnitTypeId::Probe, 38)])],
false,
Some(A::Train(UnitTypeId::Probe)),
true,
)
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
    ) -> Self {
        let _ = self
            .tree
            .add_node(
                BuildComponent::new(name, start, end, restrictive, action, display),
                None,
            )
            .unwrap();
        Self {
            index: self.tree.len() - 1,
            tree: self.tree,
        }
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
    ) -> Self {
        let _ = self
            .tree
            .add_node(
                BuildComponent::new(name, start, end, restrictive, action, display),
                Some(self.index),
            )
            .unwrap();
        self
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
    ) -> Self {
        self.tree
            .add_node(
                BuildComponent::new(name, start, end, restrictive, action, display),
                Some(self.index),
            )
            .unwrap();
        Self {
            index: self.tree.len() - 1,
            tree: self.tree,
        }
    }

    /// Combines the two trees, with all roots becoming children of the current node
    fn subtree(mut self, other: Self) -> Self {
        for other_index in other.tree.breadth_first() {
            if let Some(node) = other.tree.get(other_index) {
                if self
                    .tree
                    .add_node(
                        node.value.clone(),
                        node.parent
                            .map_or(Some(self.index), |p| Some(p + self.index + 1)),
                    )
                    .is_err()
                {
                    panic!("Unable to fuse trees at index {other_index}")
                }
            }
        }
        self
    }
/// Add a leaf to the graph to build an Assimilator. will build assimilators until `number` is reached. 
    fn gas_leaf(self, number: usize) -> Self {
        use BuildCondition as C;
        use BuildOrderAction as A;
        use ConditionOperator as Op;

        self.leaf(
            &format!("gas #{number}"),
            &[],
            &[ConditionGroup::new(
                &[
                    C::AtLeastCount(UnitTypeId::Assimilator, number),
                    C::AtLeastCount(UnitTypeId::AssimilatorRich, number),
                ],
                Op::Any,
            )],
            false,
            Some(A::Construct(UnitTypeId::Assimilator)),
            true,
        )
    }
/// Adds a root node that does nothing, with the given name, if any. 
    fn empty_root(self, name:Option<&str>) -> Self {
        self.root(name.unwrap_or("empty_root"), &[], &[], false, None, name.is_some())
}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_root() {
        let pointer = TreePointer::new();
        let tree = pointer.root("first", &[], &[], false, None, true).tree;
        assert_eq!(tree.to_string(), "first➖\n");
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn two_roots() {
        let pointer = TreePointer::new();

        let tree = pointer
            .root("first", &[], &[], false, None, true)
            .root("second", &[], &[], false, None, true)
            .tree;

        assert_eq!(tree.to_string(), "first➖\nsecond➖\n");
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn one_child() {
        let pointer = TreePointer::new();
        let tree = pointer
            .root("first", &[], &[], false, None, true)
            .child("child", &[], &[], false, None, true)
            .tree;
        assert_eq!(tree.to_string(), "first➖\n-child➖\n");
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn one_leaf_one_child_one_leaf() {
        let pointer = TreePointer::new();
        let tree = pointer
            .root("first", &[], &[], false, None, true)
            .leaf("leaf1", &[], &[], false, None, true)
            .child("child", &[], &[], false, None, true)
            .leaf("leaf2", &[], &[], false, None, true)
            .tree;
        assert_eq!(tree.to_string(), "first➖\n-child➖\n--leaf2➖\n-leaf1➖\n");
        assert_eq!(tree.len(), 4);
    }

    #[test]
    fn subtree_ok() {
        let mut pointer = TreePointer::new()
            .root("first", &[], &[], false, None, true)
            .child("child", &[], &[], false, None, true);

        let subtree = TreePointer::new()
            .root("subroot", &[], &[], false, None, true)
            .leaf("subleaf", &[], &[], false, None, true)
            .leaf("subleaf2", &[], &[], false, None, true);

        assert_eq!(
            subtree.tree.to_string(),
            "subroot➖\n-subleaf2➖\n-subleaf➖\n"
        );

        pointer = pointer.subtree(subtree);

        assert_eq!(
            pointer.tree.to_string(),
            "first➖\n-child➖\n--subroot➖\n---subleaf2➖\n---subleaf➖\n"
        );
        assert_eq!(pointer.tree.len(), 5);

        let sub_sub_tree = TreePointer::new()
            .root("subsubroot", &[], &[], false, None, true)
            .leaf("subsubleaf", &[], &[], false, None, true)
            .leaf("subsubleaf2", &[], &[], false, None, true);

        pointer = pointer
            .child("graft", &[], &[], false, None, true)
            .subtree(sub_sub_tree);

        assert_eq!(
            pointer.tree.to_string(),
            "first➖\n-child➖\n--graft➖\n---subsubroot➖\n----subsubleaf2➖\n----subsubleaf➖\n--subroot➖\n---subleaf2➖\n---subleaf➖\n"
        );
    }
}
