use std::fmt::Display;

use crate::{
    build_orders::{BuildCondition, BuildOrderAction, ComponentState},
    protoss_bot::ReBiCycler,
};

use ego_tree::Tree;

pub struct BuildOrderTree {
    pub(crate) tree: Tree<BuildComponent>,
}

impl Default for BuildOrderTree {
    fn default() -> Self {
        let tree = Tree::new(BuildComponent {
            complete_when: ConditionGroup::new(&[], ConditionOperator::All),
            name: "ROOT".to_string(),
            action: None,
            state: ComponentState::NotYetStarted,
            display: false,
        });
        Self { tree }
    }
}
/// An element of the build order tree.

#[derive(Clone)]
pub struct BuildComponent {
    /// The action for the bot to take.
    /// BuildComponents with None actions are checkpoints that can
    /// unleash reactions, i think
    action: Option<BuildOrderAction>,
    /// Conditions that end the activation of this node
    complete_when: ConditionGroup,
    /// A friendly name for the node
    name: String,
    /// Whether or not we want to display this node
    display: bool,
    /// A state to measure this thing's status
    state: ComponentState,
}
impl BuildComponent {
    pub fn new(
        name: &str,
        end: ConditionGroup,
        action: Option<BuildOrderAction>,
        display: bool,
    ) -> Self {
        Self {
            complete_when: end,
            name: name.to_string(),
            action,
            state: ComponentState::NotYetStarted,
            display,
        }
    }
    pub const fn action(&self) -> Option<BuildOrderAction> {
        self.action
    }
}

/// Groups conditions using the logical operator
#[derive(Clone)]
pub struct ConditionGroup {
    pub conditions: Vec<BuildCondition>,
    pub operator: ConditionOperator,
}
impl ConditionGroup {
    pub fn new(conditions: &[BuildCondition], operator: ConditionOperator) -> Self {
        Self {
            conditions: conditions.to_vec(),
            operator,
        }
    }
}

/// Operator for logically combining `BuildConditions`
#[derive(Clone)]
pub enum ConditionOperator {
    All,
    NotAll,
    Any,
    NoneOf,
    ExactlyNOf(usize),
}

impl BuildOrderTree {
    /// adds a node to the tree. if parent is none, adds the node as a root of a new tree
    pub fn add_node(
        &mut self,
        component: BuildComponent,
        parent: Option<usize>,
    ) -> Result<usize, TreeError> {
        todo!()
    }

    pub fn get(&self, node: usize) -> Option<&BuildComponent> {
        todo!()
    }

    /// returns a vec of indexes for the
    /// tree in breadth first order
    pub fn breadth_first(&self) -> Vec<usize> {
        todo!()
    }

    /// updates all descendants of node to restricted, recursively.
    fn restrict_descendants(&mut self, of_node: usize) {
        todo!()
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut BuildComponent> {
        todo!()
    }
}

impl Display for BuildOrderTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
#[derive(Debug)]
pub enum TreeError {
    TreeNotEmpty,
    NodeNotInTree,
}

impl ReBiCycler {
    fn evaluate_condition_group(&self, condition_group: &ConditionGroup) -> bool {
        let mut iter = condition_group.conditions.iter();
        let evaluator = |c| self.evaluate_condition(c);
        match condition_group.operator {
            ConditionOperator::All => iter.all(evaluator),
            ConditionOperator::NotAll => !iter.all(evaluator),
            ConditionOperator::Any => iter.any(evaluator),
            ConditionOperator::NoneOf => iter.all(|c| !evaluator(c)),
            ConditionOperator::ExactlyNOf(n) => {
                iter.map(evaluator).map(usize::from).sum::<usize>() == n
            }
        }
    }

    fn evaluate_build_component(&self, component: &BuildComponent) -> bool {
        self.evaluate_condition_group(&component.complete_when)
    }

    fn update_component(&mut self, index: usize) -> Option<&BuildComponent> {
        let component = self.build_order.get(index)?;
        let end = self.evaluate_build_component(&component);
        let should_activate = !end;

        let node = self.build_order.get_mut(index)?;
        if should_activate {
            node.state = ComponentState::Active;
        } else if end {
            node.state = ComponentState::Completed;
        }

        Some(node)
    }

    /// Uses a breadth-first walk of the build order tree to
    /// update the build's state.
    /// Returns a vec of active build components
    pub fn update_build(&mut self) -> Vec<BuildComponent> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank_component() -> BuildComponent {
        BuildComponent::new(
            "child",
            ConditionGroup::new(&[], ConditionOperator::All),
            None,
            true,
        )
    }

    // #[test]
    // fn add_a_child() {
    //     let mut tree = BuildOrderTree::default();
    //     assert!(tree.add_node(blank_component(), Some(0)).is_ok());

    //     assert_eq!(tree.len(), 2);
    // }

    #[test]
    fn breadth_first_order_ok() {
        let mut tree = BuildOrderTree::default();

        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 1
        assert!(tree.add_node(blank_component(), Some(1)).is_ok()); // 2
        assert!(tree.add_node(blank_component(), Some(2)).is_ok()); // 3
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 4
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 5

        assert_eq!(tree.breadth_first(), vec![0, 1, 4, 5, 2, 3]);
    }

    #[test]
    fn display_good() {
        let mut tree = BuildOrderTree::default();

        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 1
        assert!(tree.add_node(blank_component(), Some(1)).is_ok()); // 2
        assert!(tree.add_node(blank_component(), Some(2)).is_ok()); // 3
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 4
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 5
        assert!(tree.add_node(blank_component(), Some(5)).is_ok()); // 6

        assert_eq!(
            tree.to_string(),
            "ROOT➖\n-child➖\n--child➖\n-child➖\n-child➖\n--child➖\n---child➖\n".to_string()
        );

        let mut tree_two = BuildOrderTree::default();

        assert!(tree_two.add_node(blank_component(), Some(0)).is_ok()); // 1
        assert!(tree_two.add_node(blank_component(), None).is_ok()); // 2

        assert_eq!(
            tree_two.to_string(),
            "ROOT➖\n-child➖\nchild➖\n".to_string()
        );
    }
    #[test]
    fn update_state() {
        let mut tree = BuildOrderTree::default();
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 1

        let one = tree.get_mut(1);
        assert!(one.is_some());
        let one = one.unwrap();
        one.state = ComponentState::Restricted;

        assert_eq!(tree.to_string(), "ROOT➖\n-child❌\n".to_string());
    }

    #[test]
    fn check_logic() {
        let rebi = ReBiCycler::default();

        let one_off = &[
            BuildCondition::Never,
            BuildCondition::Always,
            BuildCondition::Always,
        ];
        let all_good = &[
            BuildCondition::Always,
            BuildCondition::Always,
            BuildCondition::Always,
        ];

        assert!(
            rebi.evaluate_condition_group(&ConditionGroup::new(all_good, ConditionOperator::All))
        );
        assert!(
            !rebi.evaluate_condition_group(&ConditionGroup::new(one_off, ConditionOperator::All))
        );

        assert!(
            rebi.evaluate_condition_group(&ConditionGroup::new(all_good, ConditionOperator::Any))
        );
        assert!(
            rebi.evaluate_condition_group(&ConditionGroup::new(one_off, ConditionOperator::Any))
        );

        assert!(!rebi
            .evaluate_condition_group(&ConditionGroup::new(all_good, ConditionOperator::NoneOf)));
        assert!(!rebi
            .evaluate_condition_group(&ConditionGroup::new(one_off, ConditionOperator::NoneOf)));

        assert!(!rebi.evaluate_condition_group(&ConditionGroup::new(
            all_good,
            ConditionOperator::ExactlyNOf(2)
        )));
        assert!(rebi.evaluate_condition_group(&ConditionGroup::new(
            one_off,
            ConditionOperator::ExactlyNOf(2)
        )));
    }
}
