use std::{
    collections::{HashSet, VecDeque},
    fmt::Display,
};

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
struct TreeNode {
    parent: Option<usize>,
    children: Vec<usize>,
    index: usize,
    value: BuildComponent,
}

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
    fn new(conditions: &[BuildCondition], operator: ConditionOperator) -> Self {
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
        let index = self.nodes.len();
        let new_node = TreeNode {
            parent,
            children: Vec::new(),
            index,
            value: component,
        };

        if let Some(parent_index) = parent {
            if parent_index >= self.nodes.len() {
                return Err(TreeError::NodeNotInTree);
            }
            self.nodes[parent_index].add_child(index);
        } else {
            self.roots.push(index);
        }

        self.nodes.push(new_node);
        Ok(index)
    }

    fn len(&self) -> usize {
        self.nodes.len()
    }

    fn get(&self, node: usize) -> Result<&TreeNode, TreeError> {
        self.nodes.get(node).ok_or(TreeError::NodeNotInTree)
    }

    fn get_mut(&mut self, node: usize) -> Result<&mut TreeNode, TreeError> {
        self.nodes.get_mut(node).ok_or(TreeError::NodeNotInTree)
    }
/// returns a vec out indexes forthy
///e tree in breadth first order
    pub fn breadth_first(&self) -> Vec<usize> {
        let mut visits = Vec::new();
        let mut queue = VecDeque::new();
        let mut unvisited: HashSet<usize> = (0..self.nodes.len()).collect();
        queue.extend(self.roots.iter());

        while let Some(next) = queue.pop_front() {
            let node = self.get(next);
            visits.push(next);
            unvisited.remove(&next);

            match node {
                Ok(treenode) => {
                    queue.extend(treenode.children.iter());
                }
                Err(TreeError::NodeNotInTree) => println!("wtf"),
                Err(TreeError::TreeNotEmpty) => println!("wtf"),
            }
        }

        visits
    }

///returns a vec of indexes of the tree in depth first order
    pub fn depth_first(&self) -> Vec<usize> {
        let mut visits = Vec::new();
        let mut queue = VecDeque::new();
        queue.extend(self.roots.iter());

        while let Some(next) = queue.pop_front() {
            let node = self.get(next);
            visits.push(next);

            match node {
                Ok(treenode) => {
                    let _: () = treenode
                        .children
                        .iter()
                        .map(|c| queue.push_front(*c))
                        .collect();
                }
                Err(TreeError::NodeNotInTree) => println!("wtf"),
                Err(TreeError::TreeNotEmpty) => println!("wtf"),
            }
        }

        visits
    }
///returns the depth off the given node. 
    fn depth_of(&self, node: usize) -> Result<usize, TreeError> {
        if let Some(parent) = self.get(node)?.parent {
            Ok(self.depth_of(parent)? + 1)
        } else {
            Ok(0)
        }
    }

/// updates all descendants of node to restricted, recursively. 
    fn restrict_descendants(&mut self, of_node: usize) -> Result<(), TreeError> {
        let node = self.get_mut(of_node)?;
        node.value.state = ComponentState::Restricted;

        for child in &node.children.clone() {
            self.restrict_descendants(*child)?;
        }
        Ok(())
    }
}

impl Display for BuildOrderTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        let mut queue = VecDeque::new();
        queue.extend(self.roots.iter());

        while let Some(next) = queue.pop_front() {
            let node = self.get(next);

            if let Ok(treenode) = node {
                let _: () = treenode
                    .children
                    .iter()
                    .map(|c| queue.push_front(*c))
                    .collect();
                let depth = self.depth_of(treenode.index).unwrap_or(0);
                out += &format!(
                    "{}{}{}\n",
                    "-".repeat(depth),
                    treenode.value.name,
                    treenode.value.state
                );
            }
        }
        write!(f, "{out}")
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
                iter.map(evaluator)
                    .map(|b| if b { 1 } else { 0 })
                    .sum::<usize>()
                    == n
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

    fn update_walk(&mut self, index: usize) -> Result<Vec<usize>, TreeError> {
        let restrict = self.update_component(index)?;

        let mut out = vec![index];
        if restrict {
            self.build_order_tree.restrict_descendants(index);
        } else {
            for child in &self.build_order_tree.get(index)?.children.clone() {
                let descendants = self.update_walk(*child)?;
                out.extend(descendants);
            }
        }
        Ok(out)
    }

    fn update_component(&mut self, index: usize) -> Result<bool, TreeError> {
        let node = self.build_order_tree.get(index)?;
        let (start, end) = self.evaluate_build_component(&node.value);
        let should_activate = start && !end;
        let should_restrict = !end && node.value.restrictive;

        if should_restrict {
            self.build_order_tree.restrict_descendants(index)?;
        }

        let node = self.build_order_tree.get_mut(index)?;
        if should_activate {
            node.value.state = ComponentState::Active;
        }

        Ok(should_restrict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(tree.add_first_node(BuildComponent::root()).is_ok());
    }

    #[test]
    fn add_a_child() {
        let mut tree = BuildOrderTree::new();
        assert!(tree.add_first_node(BuildComponent::root()).is_ok());
        assert!(tree.add_node(blank_component(), Some(0)).is_ok());

        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn breadth_first_order_ok() {
        let mut tree = BuildOrderTree::new();
        assert!(tree.add_first_node(BuildComponent::root()).is_ok()); // 0
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
        assert!(tree.add_first_node(BuildComponent::root()).is_ok()); // 0
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 1
        assert!(tree.add_node(blank_component(), Some(1)).is_ok()); // 2
        assert!(tree.add_node(blank_component(), Some(2)).is_ok()); // 3
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 4
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 5
        assert!(tree.add_node(blank_component(), Some(5)).is_ok()); // 6

        assert_eq!(tree.depth_first(), vec![0, 5, 6, 4, 1, 2, 3]);
    }

    #[test]
    fn display_good() {
        let mut tree = BuildOrderTree::new();
        assert!(tree.add_first_node(BuildComponent::root()).is_ok()); // 0
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
        assert!(tree_two.add_first_node(BuildComponent::root()).is_ok()); // 0
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
        assert!(tree.add_first_node(BuildComponent::root()).is_ok()); // 0
        assert!(tree.add_node(blank_component(), Some(0)).is_ok()); // 1

        let one = tree.get_mut(1);
        assert!(one.is_ok());
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

    #[test]
    fn evaluate_works() {
        let mut rebi = ReBiCycler::default();
    }
}
