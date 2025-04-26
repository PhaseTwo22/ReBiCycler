use crate::{
    build_orders::BuildOrderAction,
    build_tree::{BuildComponent, BuildOrderTree, ConditionGroup},
};

pub const fn two_base_charge() -> BuildOrderTree {
    BuildOrderTree::new()
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
}
