//! AI System: Behavior Trees + Utility AI
//!
//! This module provides AI decision-making tools for NPCs through behavior trees
//! and utility-based scoring.
//!
//! # Behavior Trees
//!
//! Behavior trees provide hierarchical, reactive decision-making:
//!
//! ```rhai
//! let tree = bt_selector([
//!     bt_sequence([
//!         bt_condition("is_low_health"),
//!         bt_action("flee")
//!     ]),
//!     bt_sequence([
//!         bt_condition("has_target"),
//!         bt_action("attack")
//!     ]),
//!     bt_action("patrol")
//! ]);
//! ```
//!
//! # Utility AI
//!
//! Utility AI scores multiple options and picks the best:
//!
//! ```rhai
//! bt_utility([
//!     #{ action: "attack", considerations: ["target_health", "my_ammo"], weight: 1.0 },
//!     #{ action: "retreat", considerations: ["my_health_low", "outnumbered"], weight: 1.2 },
//! ])
//! ```
//!
//! # Integration
//!
//! The AI system integrates with behaviors via the `on_think` hook and
//! `behavior_tree` field in EntityBehavior.

mod behavior_tree;
mod utility;
mod bindings;
mod runner;

pub use behavior_tree::*;
pub use utility::*;
pub use bindings::register as register_ai_bindings;
pub use bindings::BtNodeWrapper;
pub use runner::{BehaviorTreeRunner, AiContext, BtResult};
