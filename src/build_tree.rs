use std::{
    collections::VecDeque,
    fmt::{format, Display},
};

use itertools::Itertools;

use crate::{
    build_orders::{BuildCondition, BuildOrderAction, ComponentState},
    protoss_bot::ReBiCycler,
};

/// an arena-based tree structure to store my build order(s).
#[derive(Default)]
pub struct BuildOrderTree {
    /// The arena for all the nodes to live in
    nodes: Vec<TreeNode>,
    roots: Vec<usize>,
}

/// An element of the build order tree.
pub struct TreeNode {
    pub parent: Option<usize>,
    children: Vec<usize>,
    index: usize,
    pub value: BuildComponent,
}

#[derive(Clone)]
pub struct BuildComponent {
    /// Conditions that signal the activation of this node
    start: Vec<ConditionGroup>,
    /// Conditions that end the activation of this node
    end: Vec<ConditionGroup>,
    /// A friendly name for the node
    name: String,
    /// A restrictive build component blocks all children components when it's not finished.
    /// So if it becomes un-finished, like it loses a structure, nothing below it can happen.
    restrictive: bool,
    /// The action for the bot to take.
    /// A restrictive component with None action could be like a checkpoint for other stuff.
    action: Option<BuildOrderAction>,
    /// A state to measure this thing's status
    state: ComponentState,
    /// Whether or not we want to display this node
    display: bool,
}
impl BuildComponent {
    pub fn new(
        name: &str,
        start: &[ConditionGroup],
        end: &[ConditionGroup],
        restrictive: bool,
        action: Option<BuildOrderAction>,
        display: bool,
    ) -> Self {
        Self {
            start: start.to_vec(),
            end: end.to_vec(),
            name: name.to_string(),
            restrictive,
            action,
            state: ComponentState::NotYetStarted,
            display,
        }
    }
    pub const fn action(&self) -> Option<BuildOrderAction> {
        self.action
    }
}

impl TreeNode {
    ///registers a new child of this node
    fn add_child(&mut self, index: usize) {
        self.children.push(index);
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
    pub const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            roots: Vec::new(),
        }
    }

    /// adds a node to the tree. if parent is none, adds the node as a root of a new tree
    pub fn add_node(
        &mut self,
        component: BuildComponent,
        parent: Option<usize>,
    ) -> Result<usize, TreeError> {
        let index = self.len();
        let new_node = TreeNode {
            parent,
            children: Vec::new(),
            index,
            value: component,
        };

        if let Some(parent_index) = parent {
            if parent_index >= index {
                return Err(TreeError::NodeNotInTree);
            }
            self.nodes[parent_index].add_child(index);
        } else {
            self.roots.push(index);
        }

        self.nodes.push(new_node);
        Ok(index)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get(&self, node: usize) -> Option<&TreeNode> {
        self.nodes.get(node)
    }

    fn get_mut(&mut self, node: usize) -> Option<&mut TreeNode> {
        self.nodes.get_mut(node)
    }
    /// returns a vec of indexes for the
    /// tree in breadth first order
    pub fn breadth_first(&self) -> Vec<usize> {
        let mut visits = Vec::new();
        let mut queue = VecDeque::new();

        queue.extend(self.roots.iter());

        while let Some(next) = queue.pop_front() {
            if let Some(node) = self.get(next) {
                visits.push(next);

                queue.extend(node.children.iter());
            }
        }

        visits
    }

    ///returns the depth off the given node.
    fn depth_of(&self, index: usize) -> Option<usize> {
        let node = self.get(index)?;
        node.parent.map_or(Some(0), |parent| {
            Some(self.depth_of(parent).unwrap_or(0) + 1)
        })
    }

    /// updates all descendants of node to restricted, recursively.
    fn restrict_descendants(&mut self, of_node: usize) {
        if let Some(node) = self.get_mut(of_node) {
            node.value.state = ComponentState::Restricted;

            for child in &node.children.clone() {
                self.restrict_descendants(*child);
            }
        }
    }
}

impl Display for BuildOrderTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut visits = Vec::new();
        let mut stack = Vec::new();
        stack.extend(self.roots.iter().map(|r| (r, true, "".to_string())));

        while let Some((next, was_last, prefix)) = stack.pop() {
            if let Some(node) = self.get(*next) {
                // we arrive at a node, we write it.
let this_pointer = if was_last {"└"} else {"├"};
                if node.value.display {
                    writeln!(f, "{}{}{}{}", prefix, this_pointer, node.value.name, node.value.state);
                }

let new_prefix = if was_last {
     format!("{prefix} ")
       } else {
     format!("{prefix}|")
                let child_count = node.children.len();
                for (i,child) in node.children.iter().enumerate() {

let is_last = i == child_count - 1;
stack.push((child, is_last, new_prefix));


 };

}

            }
        }
        write!(f, "")
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

    fn evaluate_build_component(&self, component: &BuildComponent) -> (bool, bool) {
        let start_status = component
            .start
            .iter()
            .all(|cg| self.evaluate_condition_group(cg));
        let end_status = component
            .end
            .iter()
            .all(|cg| self.evaluate_condition_group(cg));
        (start_status, end_status)
    }

    // fn update_build_order_progress(&mut self, bot: BuildOrderTree) {
    //     for root in bot.roots.clone().iter() {
    //         let active_node = *root;
    //         let pruned = false;

    //         self.evaluate_condition_group(condition_group)
    //     }
    // }

    fn update_component(&mut self, index: usize) -> Option<(&TreeNode, bool)> {
        let node = self.build_order.get(index)?;
        let (start, end) = self.evaluate_build_component(&node.value);
        let should_activate = start && !end;
        let should_restrict = !end && node.value.restrictive;

        if should_restrict {
            self.build_order.restrict_descendants(index);
        }

        let node = self.build_order.get_mut(index)?;
        if should_activate {
            node.value.state = ComponentState::Active;
        } else if end {
            node.value.state = ComponentState::Completed;
        }

        Some((node, should_restrict))
    }

    /// Uses a breadth-first walk of the build order tree to
    /// update the build's state.
    /// Returns a vec of active build components
    pub fn update_build(&mut self) -> Vec<BuildComponent> {
        let mut visits = Vec::new();
        let mut queue = VecDeque::new();
        queue.extend(self.build_order.roots.iter());

        while let Some(next) = queue.pop_front() {
            if let Some((node, pruning)) = self.update_component(next) {
                if node.value.state == ComponentState::Active {
                    visits.push(node.value.clone());
                }
                if pruning {
                    continue;
                }
                queue.extend(node.children.iter());
            } else {
                self.log_error(format!("failed to walk the build order at index {next}"));
            }
        }

        visits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_component() -> BuildComponent {
        BuildComponent::new(
            "ROOT",
            &[ConditionGroup::new(
                &[BuildCondition::Always],
                ConditionOperator::All,
            )],
            &[ConditionGroup::new(
                &[BuildCondition::Always],
                ConditionOperator::All,
            )],
            true,
            None,
            true,
        )
    }

    fn blank_component() -> BuildComponent {
        BuildComponent::new(
            "child",
            &[ConditionGroup::new(
                &[BuildCondition::Always],
                ConditionOperator::All,
            )],
            &[ConditionGroup::new(
                &[BuildCondition::Always],
                ConditionOperator::All,
            )],
            true,
            None,
            true,
        )
    }

    #[test]
    fn add_a_root() {
        let mut tree = BuildOrderTree::new();
        assert!(tree.add_node(root_component(), None).is_ok());
    }

    #[test]
    fn add_a_child() {
        let mut tree = BuildOrderTree::new();
        assert!(tree.add_node(root_component(), None).is_ok());
        assert!(tree.add_node(blank_component(), Some(0)).is_ok());

        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn breadth_first_order_ok() {
        let mut tree = BuildOrderTree::new();
        assert!(tree.add_node(root_component(), None).is_ok()); // 0
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 1
        assert!(tree.add_node(blank_component(), Some(1)).is_ok()); // 2
        assert!(tree.add_node(blank_component(), Some(2)).is_ok()); // 3
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 4
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 5

        assert_eq!(tree.breadth_first(), vec![0, 1, 4, 5, 2, 3]);
    }
    #[test]
    fn depth_first_order_ok() {
        let mut tree = BuildOrderTree::new();
        assert!(tree.add_node(root_component(), None).is_ok()); // 0
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 1
        assert!(tree.add_node(blank_component(), Some(1)).is_ok()); // 2
        assert!(tree.add_node(blank_component(), Some(2)).is_ok()); // 3
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 4
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 5
        assert!(tree.add_node(blank_component(), Some(5)).is_ok()); // 6

        assert_eq!(
            tree.depth_first(),
            vec![(0, 0), (5, 1), (6, 2), (4, 1), (1, 1), (2, 2), (3, 3)]
        );
    }

    #[test]
    fn display_good() {
        let mut tree = BuildOrderTree::new();
        assert!(tree.add_node(root_component(), None).is_ok()); // 0
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

        let mut tree_two = BuildOrderTree::new();
        assert!(tree_two.add_node(root_component(), None).is_ok()); // 0
        assert!(tree_two.add_node(blank_component(), Some(0)).is_ok()); // 1
        assert!(tree_two.add_node(blank_component(), None).is_ok()); // 2

        assert_eq!(
            tree_two.to_string(),
            "ROOT➖\n-child➖\nchild➖\n".to_string()
        );
    }
    #[test]
    fn update_state() {
        let mut tree = BuildOrderTree::new();
        assert!(tree.add_node(root_component(), None).is_ok()); // 0
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 1

        let one = tree.get_mut(1);
        assert!(one.is_some());
        let one = one.unwrap();
        one.value.state = ComponentState::Restricted;

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
