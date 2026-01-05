//! Behavior Tree Node Types
//!
//! Defines the core behavior tree structure used for NPC decision making.

use std::fmt;
use serde::{Deserialize, Serialize};

/// Status returned by behavior tree node execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BtStatus {
    /// Node completed successfully
    Success,
    /// Node failed
    Failure,
    /// Node is still running (will continue next tick)
    Running,
}

impl fmt::Display for BtStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "Success"),
            Self::Failure => write!(f, "Failure"),
            Self::Running => write!(f, "Running"),
        }
    }
}

/// A node in a behavior tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BtNode {
    /// Execute children in order until one fails.
    /// Returns Success if all children succeed.
    /// Returns Failure on first child failure.
    Sequence(Vec<BtNode>),

    /// Execute children in order until one succeeds.
    /// Returns Failure if all children fail.
    /// Returns Success on first child success.
    Selector(Vec<BtNode>),

    /// Execute an action by calling a script function.
    /// The function name is stored and called with the AI context.
    Action(String),

    /// Check a condition by calling a script function.
    /// The function should return true/false.
    Condition(String),

    /// Utility AI decision point.
    /// Scores multiple options and executes the best one.
    Utility(Vec<UtilityOption>),

    /// Decorator that modifies child behavior.
    Decorator {
        kind: DecoratorKind,
        child: Box<BtNode>,
    },

    /// Execute all children regardless of their status.
    /// Useful for cleanup or parallel-like behavior.
    ForceSequence(Vec<BtNode>),

    /// Wait for a number of ticks before succeeding.
    Wait(u32),

    /// Always succeed.
    Succeed,

    /// Always fail.
    Fail,

    /// Subtree reference by name (for modularity).
    Subtree(String),
}

impl BtNode {
    /// Create a sequence node.
    pub fn sequence(children: Vec<BtNode>) -> Self {
        Self::Sequence(children)
    }

    /// Create a selector node.
    pub fn selector(children: Vec<BtNode>) -> Self {
        Self::Selector(children)
    }

    /// Create an action node.
    pub fn action(name: impl Into<String>) -> Self {
        Self::Action(name.into())
    }

    /// Create a condition node.
    pub fn condition(name: impl Into<String>) -> Self {
        Self::Condition(name.into())
    }

    /// Create a utility node.
    pub fn utility(options: Vec<UtilityOption>) -> Self {
        Self::Utility(options)
    }

    /// Create a decorator node.
    pub fn decorator(kind: DecoratorKind, child: BtNode) -> Self {
        Self::Decorator {
            kind,
            child: Box::new(child),
        }
    }

    /// Wrap this node with a decorator.
    pub fn with_decorator(self, kind: DecoratorKind) -> Self {
        Self::decorator(kind, self)
    }

    /// Create a repeat decorator.
    pub fn repeat(count: Option<u32>, child: BtNode) -> Self {
        Self::decorator(DecoratorKind::Repeat(count), child)
    }

    /// Create an until-fail decorator.
    pub fn until_fail(child: BtNode) -> Self {
        Self::decorator(DecoratorKind::UntilFail, child)
    }

    /// Create an invert decorator.
    pub fn invert(child: BtNode) -> Self {
        Self::decorator(DecoratorKind::Invert, child)
    }

    /// Create a cooldown decorator.
    pub fn cooldown(seconds: f64, child: BtNode) -> Self {
        Self::decorator(DecoratorKind::Cooldown(seconds), child)
    }

    /// Create a wait node.
    pub fn wait(ticks: u32) -> Self {
        Self::Wait(ticks)
    }

    /// Get a debug description of this node type.
    pub fn node_type(&self) -> &'static str {
        match self {
            Self::Sequence(_) => "Sequence",
            Self::Selector(_) => "Selector",
            Self::Action(_) => "Action",
            Self::Condition(_) => "Condition",
            Self::Utility(_) => "Utility",
            Self::Decorator { .. } => "Decorator",
            Self::ForceSequence(_) => "ForceSequence",
            Self::Wait(_) => "Wait",
            Self::Succeed => "Succeed",
            Self::Fail => "Fail",
            Self::Subtree(_) => "Subtree",
        }
    }
}

/// Decorator types that modify child node behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecoratorKind {
    /// Repeat the child N times (None = infinite).
    Repeat(Option<u32>),

    /// Repeat the child until it fails.
    UntilFail,

    /// Invert the child's result (Success <-> Failure).
    Invert,

    /// Only run the child if cooldown has elapsed.
    /// The f64 is seconds since last execution.
    Cooldown(f64),

    /// Run the child only once, then always return the same result.
    RunOnce,

    /// Force success regardless of child result.
    AlwaysSucceed,

    /// Force failure regardless of child result.
    AlwaysFail,

    /// Timeout - fail if child runs longer than N seconds.
    Timeout(f64),
}

/// An option for utility AI scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilityOption {
    /// Name of the action function to call if this option is chosen.
    pub action: String,

    /// List of consideration function names.
    /// Each function should return a score from 0.0 to 1.0.
    pub considerations: Vec<String>,

    /// Weight multiplier for this option's final score.
    pub weight: f64,
}

impl UtilityOption {
    /// Create a new utility option.
    pub fn new(
        action: impl Into<String>,
        considerations: Vec<String>,
        weight: f64,
    ) -> Self {
        Self {
            action: action.into(),
            considerations,
            weight,
        }
    }
}

/// Runtime state for a behavior tree node.
/// Used to track running states across ticks.
#[derive(Debug, Clone, Default)]
pub struct BtNodeState {
    /// Index of currently running child (for Sequence/Selector).
    pub child_index: usize,

    /// Number of repetitions completed (for Repeat decorator).
    pub repeat_count: u32,

    /// Last execution time (for Cooldown decorator).
    pub last_execution: Option<f64>,

    /// Ticks waited so far (for Wait node).
    pub ticks_waited: u32,

    /// Stored result (for RunOnce decorator).
    pub stored_result: Option<BtStatus>,
}

impl BtNodeState {
    pub fn reset(&mut self) {
        self.child_index = 0;
        self.repeat_count = 0;
        self.ticks_waited = 0;
        // Note: last_execution and stored_result are intentionally not reset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_construction() {
        let tree = BtNode::selector(vec![
            BtNode::sequence(vec![
                BtNode::condition("is_low_health"),
                BtNode::action("flee"),
            ]),
            BtNode::action("attack"),
        ]);

        assert_eq!(tree.node_type(), "Selector");
        if let BtNode::Selector(children) = tree {
            assert_eq!(children.len(), 2);
        }
    }

    #[test]
    fn test_decorator_construction() {
        let node = BtNode::action("attack")
            .with_decorator(DecoratorKind::Cooldown(5.0));

        assert_eq!(node.node_type(), "Decorator");
        if let BtNode::Decorator { kind, .. } = node {
            assert!(matches!(kind, DecoratorKind::Cooldown(_)));
        }
    }

    #[test]
    fn test_utility_option() {
        let option = UtilityOption::new(
            "attack",
            vec!["target_health".into(), "my_ammo".into()],
            1.5,
        );

        assert_eq!(option.action, "attack");
        assert_eq!(option.considerations.len(), 2);
        assert_eq!(option.weight, 1.5);
    }
}
