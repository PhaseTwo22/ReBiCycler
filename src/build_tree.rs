use crate::build_orders::{BuildCondition, BuildOrderAction, ComponentState};

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

    restrictive: bool,
    /// The action for the bot to take
    action: BuildOrderAction,
    /// A state to measure this thing's status
    state: ComponentState,
    /// Whether or not we want to display this node
    display: bool,
}

impl TreeNode {
    fn log_child(&mut self, index: usize) {
        self.children.push(index);
    }
}
/// Groups conditions using the logical operator
struct ConditionGroup {
    conditions: Vec<BuildCondition>,
    operator: ConditionOperator,
}

/// Operator for logically combining `BuildConditions`
enum ConditionOperator {
    And,
    Or,
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
        self.nodes[parent].log_child(index);
        Ok(index)
    }
}

enum TreeError {
    TreeNotEmpty,
    ParentNotInTree,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_a_root() {
        let mut tree = BuildOrderTree::new();
        tree.add_first_node(BuildComponent {
            start: (),
            end: (),
            name: (),
            restrictive: (),
            action: (),
            state: (),
            display: (),
        })
    }
}
