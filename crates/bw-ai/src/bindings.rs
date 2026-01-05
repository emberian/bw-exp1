//! Rhai Bindings for AI System
//!
//! Provides script functions for constructing and using behavior trees.
//!
//! # Example
//!
//! ```rhai
//! // Build a behavior tree
//! let tree = bt_selector([
//!     bt_sequence([
//!         bt_condition("is_low_health"),
//!         bt_action("flee")
//!     ]),
//!     bt_sequence([
//!         bt_condition("has_target"),
//!         bt_utility([
//!             #{ action: "attack", considerations: ["target_health", "my_ammo"], weight: 1.0 },
//!             #{ action: "flank", considerations: ["cover_nearby", "target_distracted"], weight: 0.8 },
//!         ])
//!     ]),
//!     bt_action("patrol")
//! ]);
//! ```

use rhai::{Engine, Array, Map, CustomType, TypeBuilder};
use super::{BtNode, BtStatus, DecoratorKind, UtilityOption};

/// Register all AI bindings with the Rhai engine.
pub fn register(engine: &mut Engine) {
    // Register BtNode as a custom type
    engine.build_type::<BtNodeWrapper>();

    // Register BtStatus as a custom type
    engine.build_type::<BtStatusWrapper>();

    // === Node Construction Functions ===

    // bt_sequence([child1, child2, ...])
    engine.register_fn("bt_sequence", |children: Array| -> BtNodeWrapper {
        let nodes: Vec<BtNode> = children
            .into_iter()
            .filter_map(|d| d.try_cast::<BtNodeWrapper>())
            .map(|w| w.0)
            .collect();
        BtNodeWrapper(BtNode::Sequence(nodes))
    });

    // bt_selector([child1, child2, ...])
    engine.register_fn("bt_selector", |children: Array| -> BtNodeWrapper {
        let nodes: Vec<BtNode> = children
            .into_iter()
            .filter_map(|d| d.try_cast::<BtNodeWrapper>())
            .map(|w| w.0)
            .collect();
        BtNodeWrapper(BtNode::Selector(nodes))
    });

    // bt_action("action_name")
    engine.register_fn("bt_action", |name: &str| -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Action(name.to_string()))
    });

    // bt_condition("condition_name")
    engine.register_fn("bt_condition", |name: &str| -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Condition(name.to_string()))
    });

    // bt_succeed()
    engine.register_fn("bt_succeed", || -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Succeed)
    });

    // bt_fail()
    engine.register_fn("bt_fail", || -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Fail)
    });

    // bt_wait(ticks)
    engine.register_fn("bt_wait", |ticks: i64| -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Wait(ticks.max(0) as u32))
    });

    // bt_subtree("subtree_name")
    engine.register_fn("bt_subtree", |name: &str| -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Subtree(name.to_string()))
    });

    // bt_force_sequence([child1, child2, ...])
    engine.register_fn("bt_force_sequence", |children: Array| -> BtNodeWrapper {
        let nodes: Vec<BtNode> = children
            .into_iter()
            .filter_map(|d| d.try_cast::<BtNodeWrapper>())
            .map(|w| w.0)
            .collect();
        BtNodeWrapper(BtNode::ForceSequence(nodes))
    });

    // === Utility AI ===

    // bt_utility([
    //   #{ action: "attack", considerations: ["target_health"], weight: 1.0 },
    //   #{ action: "flee", considerations: ["my_health_low"], weight: 1.2 },
    // ])
    engine.register_fn("bt_utility", |options: Array| -> BtNodeWrapper {
        let utility_options: Vec<UtilityOption> = options
            .into_iter()
            .filter_map(|d| {
                let map = d.try_cast::<Map>()?;

                let action = map.get("action")?.clone().into_string().ok()?;

                let considerations: Vec<String> = map.get("considerations")?
                    .clone()
                    .try_cast::<Array>()?
                    .into_iter()
                    .filter_map(|c| c.into_string().ok())
                    .collect();

                let weight = map.get("weight")
                    .and_then(|w| w.clone().try_cast::<f64>())
                    .unwrap_or(1.0);

                Some(UtilityOption::new(action, considerations, weight))
            })
            .collect();

        BtNodeWrapper(BtNode::Utility(utility_options))
    });

    // === Decorators ===

    // bt_invert(child)
    engine.register_fn("bt_invert", |child: BtNodeWrapper| -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Decorator {
            kind: DecoratorKind::Invert,
            child: Box::new(child.0),
        })
    });

    // bt_repeat(count, child) - count of 0 means infinite
    engine.register_fn("bt_repeat", |count: i64, child: BtNodeWrapper| -> BtNodeWrapper {
        let repeat_count = if count <= 0 { None } else { Some(count as u32) };
        BtNodeWrapper(BtNode::Decorator {
            kind: DecoratorKind::Repeat(repeat_count),
            child: Box::new(child.0),
        })
    });

    // bt_until_fail(child)
    engine.register_fn("bt_until_fail", |child: BtNodeWrapper| -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Decorator {
            kind: DecoratorKind::UntilFail,
            child: Box::new(child.0),
        })
    });

    // bt_cooldown(seconds, child)
    engine.register_fn("bt_cooldown", |seconds: f64, child: BtNodeWrapper| -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Decorator {
            kind: DecoratorKind::Cooldown(seconds),
            child: Box::new(child.0),
        })
    });

    // bt_run_once(child)
    engine.register_fn("bt_run_once", |child: BtNodeWrapper| -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Decorator {
            kind: DecoratorKind::RunOnce,
            child: Box::new(child.0),
        })
    });

    // bt_always_succeed(child)
    engine.register_fn("bt_always_succeed", |child: BtNodeWrapper| -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Decorator {
            kind: DecoratorKind::AlwaysSucceed,
            child: Box::new(child.0),
        })
    });

    // bt_always_fail(child)
    engine.register_fn("bt_always_fail", |child: BtNodeWrapper| -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Decorator {
            kind: DecoratorKind::AlwaysFail,
            child: Box::new(child.0),
        })
    });

    // bt_timeout(seconds, child)
    engine.register_fn("bt_timeout", |seconds: f64, child: BtNodeWrapper| -> BtNodeWrapper {
        BtNodeWrapper(BtNode::Decorator {
            kind: DecoratorKind::Timeout(seconds),
            child: Box::new(child.0),
        })
    });

    // === Response Curves ===

    engine.register_fn("curve_linear", |x: f64| -> f64 {
        super::utility::curves::linear(x)
    });

    engine.register_fn("curve_quadratic", |x: f64| -> f64 {
        super::utility::curves::quadratic(x)
    });

    engine.register_fn("curve_inverse_quadratic", |x: f64| -> f64 {
        super::utility::curves::inverse_quadratic(x)
    });

    engine.register_fn("curve_logistic", |x: f64| -> f64 {
        super::utility::curves::logistic(x)
    });

    engine.register_fn("curve_exponential_decay", |x: f64| -> f64 {
        super::utility::curves::exponential_decay(x)
    });

    engine.register_fn("curve_step", |x: f64| -> f64 {
        super::utility::curves::step(x)
    });

    engine.register_fn("curve_threshold", |x: f64, cutoff: f64| -> f64 {
        super::utility::curves::threshold(x, cutoff)
    });

    engine.register_fn("curve_inverse", |x: f64| -> f64 {
        super::utility::curves::inverse(x)
    });

    engine.register_fn("curve_polynomial", |x: f64, exponent: f64| -> f64 {
        super::utility::curves::polynomial(x, exponent)
    });

    // === Consideration Helpers ===

    engine.register_fn("consider_health", |current: f64, max: f64| -> f64 {
        super::utility::considerations::health_percent(current, max)
    });

    engine.register_fn("consider_damage", |current: f64, max: f64| -> f64 {
        super::utility::considerations::damage_percent(current, max)
    });

    engine.register_fn("consider_proximity", |distance: f64, max_range: f64| -> f64 {
        super::utility::considerations::proximity(distance, max_range)
    });

    engine.register_fn("consider_resource", |current: f64, max: f64| -> f64 {
        super::utility::considerations::resource_percent(current, max)
    });

    engine.register_fn("consider_resource_urgency", |current: f64, max: f64, threshold: f64| -> f64 {
        super::utility::considerations::resource_urgency(current, max, threshold)
    });

    engine.register_fn("consider_numeric_advantage", |allies: i64, enemies: i64| -> f64 {
        super::utility::considerations::numeric_advantage(allies as usize, enemies as usize)
    });

    engine.register_fn("consider_outnumbered", |allies: i64, enemies: i64| -> f64 {
        super::utility::considerations::outnumbered(allies as usize, enemies as usize)
    });

    // === Status Constants ===

    engine.register_fn("BT_SUCCESS", || -> BtStatusWrapper {
        BtStatusWrapper(BtStatus::Success)
    });

    engine.register_fn("BT_FAILURE", || -> BtStatusWrapper {
        BtStatusWrapper(BtStatus::Failure)
    });

    engine.register_fn("BT_RUNNING", || -> BtStatusWrapper {
        BtStatusWrapper(BtStatus::Running)
    });
}

/// Wrapper for BtNode to expose to Rhai.
#[derive(Debug, Clone)]
pub struct BtNodeWrapper(pub BtNode);

impl CustomType for BtNodeWrapper {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("BehaviorTree")
            .with_fn("to_string", |wrapper: &mut BtNodeWrapper| -> String {
                format!("BehaviorTree({})", wrapper.0.node_type())
            });
    }
}

/// Wrapper for BtStatus to expose to Rhai.
#[derive(Debug, Clone)]
pub struct BtStatusWrapper(pub BtStatus);

impl CustomType for BtStatusWrapper {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("BtStatus")
            .with_fn("to_string", |wrapper: &mut BtStatusWrapper| -> String {
                format!("{}", wrapper.0)
            })
            .with_fn("is_success", |wrapper: &mut BtStatusWrapper| -> bool {
                wrapper.0 == BtStatus::Success
            })
            .with_fn("is_failure", |wrapper: &mut BtStatusWrapper| -> bool {
                wrapper.0 == BtStatus::Failure
            })
            .with_fn("is_running", |wrapper: &mut BtStatusWrapper| -> bool {
                wrapper.0 == BtStatus::Running
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bindings_register() {
        let mut engine = Engine::new();
        register(&mut engine);

        // Test that functions are registered
        let result = engine.eval::<BtNodeWrapper>("bt_action(\"test\")");
        assert!(result.is_ok());

        let result = engine.eval::<BtNodeWrapper>("bt_succeed()");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sequence_construction() {
        let mut engine = Engine::new();
        register(&mut engine);

        let result = engine.eval::<BtNodeWrapper>(r#"
            bt_sequence([
                bt_action("first"),
                bt_action("second")
            ])
        "#);

        assert!(result.is_ok());
        let wrapper = result.unwrap();
        assert!(matches!(wrapper.0, BtNode::Sequence(_)));
    }

    #[test]
    fn test_utility_construction() {
        let mut engine = Engine::new();
        register(&mut engine);

        let result = engine.eval::<BtNodeWrapper>(r#"
            bt_utility([
                #{ action: "attack", considerations: ["target_health", "my_ammo"], weight: 1.0 },
                #{ action: "flee", considerations: ["low_health"], weight: 1.2 }
            ])
        "#);

        assert!(result.is_ok());
        let wrapper = result.unwrap();
        if let BtNode::Utility(options) = wrapper.0 {
            assert_eq!(options.len(), 2);
            assert_eq!(options[0].action, "attack");
            assert_eq!(options[1].weight, 1.2);
        } else {
            panic!("Expected Utility node");
        }
    }

    #[test]
    fn test_decorator_construction() {
        let mut engine = Engine::new();
        register(&mut engine);

        let result = engine.eval::<BtNodeWrapper>(r#"
            bt_cooldown(5.0, bt_action("heal"))
        "#);

        assert!(result.is_ok());
        let wrapper = result.unwrap();
        assert!(matches!(wrapper.0, BtNode::Decorator { .. }));
    }

    #[test]
    fn test_curves() {
        let mut engine = Engine::new();
        register(&mut engine);

        let result = engine.eval::<f64>("curve_linear(0.5)").unwrap();
        assert!((result - 0.5).abs() < 0.001);

        let result = engine.eval::<f64>("curve_quadratic(0.5)").unwrap();
        assert!((result - 0.25).abs() < 0.001);

        let result = engine.eval::<f64>("curve_inverse(0.3)").unwrap();
        assert!((result - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_considerations() {
        let mut engine = Engine::new();
        register(&mut engine);

        let result = engine.eval::<f64>("consider_health(50.0, 100.0)").unwrap();
        assert!((result - 0.5).abs() < 0.001);

        let result = engine.eval::<f64>("consider_damage(50.0, 100.0)").unwrap();
        assert!((result - 0.5).abs() < 0.001);
    }
}
