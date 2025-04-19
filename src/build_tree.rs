use crate::{
    build_orders::{BuildCondition, BuildOrderAction, ComponentState},
    protoss_bot::ReBiCycler,
};

/// an arena-based tree structure to store my build order(s).
struct BuildOrderTree {
    /// The arena for all the nodes to live in
    nodes: Vec<TreeNode>,
}

/// An element of the build order tree.
struct TreeNode {
    parent: Option<usize>,
    children: Vec<usize>,
    index: usize,
    value: BuildComponent,
}

struct BuildComponent {
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
    fn root() -> Self {
        Self {
            start: vec![ConditionGroup::new(
                &[BuildCondition::Always],
                ConditionOperator::All,
            )],
            end: vec![ConditionGroup::new(
                &[BuildCondition::Always],
                ConditionOperator::All,
            )],
            name: "ROOT".to_string(),
            restrictive: false,
            action: None,
            state: ComponentState::NotYetStarted,
            display: false,
        }
    }
    fn new(
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
}

impl TreeNode {
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
    fn new(conditions: &[BuildCondition], operator: ConditionOperator) -> Self {
        Self {
            conditions: conditions.to_vec(),
            operator,
        }
    }
}

/// Operator for logically combining `BuildConditions`
#[derive(Clone)]
enum ConditionOperator {
    All,
    NotAll,
    Any,
    NoneOf,
    ExactlyOneOf,
}

impl BuildOrderTree {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_first_node(&mut self, root: BuildComponent) -> Result<usize, TreeError> {
        if !self.nodes.is_empty() {
            return Err(TreeError::TreeNotEmpty);
        }
        let root_component = TreeNode {
            parent: None,
            children: Vec::new(),
            value: root,
            index: 0,
        };
        self.nodes.push(root_component);
        Ok(0)
    }

    pub fn add_node(
        &mut self,
        component: BuildComponent,
        parent: usize,
    ) -> Result<usize, TreeError> {
        if parent >= self.nodes.len() {
            return Err(TreeError::ParentNotInTree);
        }
        let index = self.nodes.len();
        let new_node = TreeNode {
            parent: Some(parent),
            children: Vec::new(),
            index,
            value: component,
        };
        self.nodes.push(new_node);
        self.nodes[parent].add_child(index);
        Ok(index)
    }

    fn len(&self) -> usize {
        self.nodes.len()
    }
}

enum TreeError {
    TreeNotEmpty,
    ParentNotInTree,
}

impl ReBiCycler {
    fn evaluate_condition_group(&self, condition_group: ConditionGroup) -> bool {
        let mut iter = condition_group.conditions.iter();
        let evaluator = |c| self.evaluate_condition(c);
        match condition_group.operator {
            ConditionOperator::All => iter.all(evaluator),
            ConditionOperator::NotAll => !iter.all(evaluator),
            ConditionOperator::Any => iter.any(evaluator),
            ConditionOperator::NoneOf => iter.all(|c| !evaluator(c)),
            ConditionOperator::ExactlyOneOf => {
                iter.map(evaluator)
                    .map(|b| if b { 1 } else { 0 })
                    .sum::<usize>()
                    == 1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_a_root() {
        let mut tree = BuildOrderTree::new();
        assert!(tree.add_first_node(BuildComponent::root()).is_ok());
    }

    #[test]
    fn add_a_child() {
        let mut tree = BuildOrderTree::new();
        assert!(tree.add_first_node(BuildComponent::root()).is_ok());
        assert!(tree
            .add_node(
                BuildComponent::new(
                    "child",
                    &[ConditionGroup::new(
                        &[BuildCondition::Always],
                        ConditionOperator::All
                    )],
                    &[ConditionGroup::new(
                        &[BuildCondition::Always],
                        ConditionOperator::All
                    )],
                    true,
                    None,
                    true
                ),
                0
            )
            .is_ok());

        assert_eq!(tree.len(), 2)
    }
}
