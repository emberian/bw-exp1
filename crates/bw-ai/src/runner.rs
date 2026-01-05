//! Behavior Tree Runner
//!
//! Executes behavior tree nodes and manages running state.

use std::collections::HashMap;
use parking_lot::RwLock;
use rhai::{Dynamic, Map, Engine, Scope, AST};
use uuid::Uuid;

use super::{BtNode, BtStatus, BtNodeState, DecoratorKind, UtilityOption, UtilityScore, UtilityDecision};

/// Context passed to behavior tree execution.
#[derive(Debug, Clone)]
pub struct AiContext {
    /// The entity running this behavior tree.
    pub entity_id: Uuid,
    /// Current game tick.
    pub tick: u64,
    /// Time since last update (seconds).
    pub delta_time: f64,
    /// Current game time (seconds from start).
    pub game_time: f64,
    /// Additional context data accessible to scripts.
    pub data: Map,
}

impl AiContext {
    /// Create a new AI context.
    pub fn new(entity_id: Uuid, tick: u64, delta_time: f64, game_time: f64) -> Self {
        Self {
            entity_id,
            tick,
            delta_time,
            game_time,
            data: Map::new(),
        }
    }

    /// Add data to the context.
    pub fn with_data(mut self, key: &str, value: Dynamic) -> Self {
        self.data.insert(key.into(), value);
        self
    }

    /// Convert to Rhai Dynamic for passing to scripts.
    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = Map::new();
        map.insert("entity_id".into(), self.entity_id.to_string().into());
        map.insert("tick".into(), (self.tick as i64).into());
        map.insert("delta_time".into(), self.delta_time.into());
        map.insert("game_time".into(), self.game_time.into());

        // Merge in additional data
        for (key, value) in &self.data {
            map.insert(key.clone(), value.clone());
        }

        Dynamic::from(map)
    }
}

/// Result of running a behavior tree.
#[derive(Debug)]
pub struct BtResult {
    /// Final status of the tree.
    pub status: BtStatus,
    /// Debug trace of execution (if enabled).
    pub trace: Vec<String>,
    /// Any error that occurred.
    pub error: Option<String>,
}

impl BtResult {
    pub fn success() -> Self {
        Self { status: BtStatus::Success, trace: vec![], error: None }
    }

    pub fn failure() -> Self {
        Self { status: BtStatus::Failure, trace: vec![], error: None }
    }

    pub fn running() -> Self {
        Self { status: BtStatus::Running, trace: vec![], error: None }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self { status: BtStatus::Failure, trace: vec![], error: Some(msg.into()) }
    }
}

impl Default for BtResult {
    fn default() -> Self {
        Self::failure()
    }
}

/// Executes behavior trees.
#[derive(Default)]
pub struct BehaviorTreeRunner {
    /// Compiled ASTs for behavior scripts.
    scripts: RwLock<HashMap<String, AST>>,
    /// Node states indexed by (entity_id, node_path).
    states: RwLock<HashMap<(Uuid, String), BtNodeState>>,
    /// Named subtrees.
    subtrees: RwLock<HashMap<String, BtNode>>,
    /// Enable debug tracing.
    debug: bool,
}

impl BehaviorTreeRunner {
    /// Create a new behavior tree runner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable debug mode.
    pub fn set_debug(&mut self, enabled: bool) {
        self.debug = enabled;
    }

    /// Register a compiled script for action/condition lookup.
    pub fn register_script(&self, name: &str, ast: AST) {
        self.scripts.write().insert(name.to_string(), ast);
    }

    /// Register a named subtree.
    pub fn register_subtree(&self, name: &str, tree: BtNode) {
        self.subtrees.write().insert(name.to_string(), tree);
    }

    /// Clear all state for an entity.
    pub fn clear_entity_state(&self, entity_id: Uuid) {
        self.states.write().retain(|(eid, _), _| *eid != entity_id);
    }

    /// Run a behavior tree for an entity.
    pub fn run(
        &self,
        engine: &Engine,
        tree: &BtNode,
        ctx: &AiContext,
        script_path: &str,
    ) -> BtResult {
        let mut trace = if self.debug { Some(Vec::new()) } else { None };
        let result = self.run_node(engine, tree, ctx, script_path, "root", &mut trace);

        BtResult {
            status: result,
            trace: trace.unwrap_or_default(),
            error: None,
        }
    }

    fn run_node(
        &self,
        engine: &Engine,
        node: &BtNode,
        ctx: &AiContext,
        script_path: &str,
        path: &str,
        trace: &mut Option<Vec<String>>,
    ) -> BtStatus {
        if let Some(t) = trace {
            t.push(format!("[{}] Running {}", path, node.node_type()));
        }

        match node {
            BtNode::Sequence(children) => self.run_sequence(engine, children, ctx, script_path, path, trace),
            BtNode::Selector(children) => self.run_selector(engine, children, ctx, script_path, path, trace),
            BtNode::Action(name) => self.run_action(engine, name, ctx, script_path, trace),
            BtNode::Condition(name) => self.run_condition(engine, name, ctx, script_path, trace),
            BtNode::Utility(options) => self.run_utility(engine, options, ctx, script_path, trace),
            BtNode::Decorator { kind, child } => self.run_decorator(engine, kind, child, ctx, script_path, path, trace),
            BtNode::ForceSequence(children) => self.run_force_sequence(engine, children, ctx, script_path, path, trace),
            BtNode::Wait(ticks) => self.run_wait(*ticks, ctx, path),
            BtNode::Succeed => BtStatus::Success,
            BtNode::Fail => BtStatus::Failure,
            BtNode::Subtree(name) => self.run_subtree(engine, name, ctx, script_path, path, trace),
        }
    }

    fn run_sequence(
        &self,
        engine: &Engine,
        children: &[BtNode],
        ctx: &AiContext,
        script_path: &str,
        path: &str,
        trace: &mut Option<Vec<String>>,
    ) -> BtStatus {
        let state_key = (ctx.entity_id, path.to_string());
        let start_index = self.states.read().get(&state_key).map(|s| s.child_index).unwrap_or(0);

        for (i, child) in children.iter().enumerate().skip(start_index) {
            let child_path = format!("{}/seq_{}", path, i);
            let status = self.run_node(engine, child, ctx, script_path, &child_path, trace);

            match status {
                BtStatus::Failure => {
                    self.states.write().remove(&state_key);
                    return BtStatus::Failure;
                }
                BtStatus::Running => {
                    self.states.write().entry(state_key).or_default().child_index = i;
                    return BtStatus::Running;
                }
                BtStatus::Success => {}
            }
        }

        self.states.write().remove(&state_key);
        BtStatus::Success
    }

    fn run_selector(
        &self,
        engine: &Engine,
        children: &[BtNode],
        ctx: &AiContext,
        script_path: &str,
        path: &str,
        trace: &mut Option<Vec<String>>,
    ) -> BtStatus {
        let state_key = (ctx.entity_id, path.to_string());
        let start_index = self.states.read().get(&state_key).map(|s| s.child_index).unwrap_or(0);

        for (i, child) in children.iter().enumerate().skip(start_index) {
            let child_path = format!("{}/sel_{}", path, i);
            let status = self.run_node(engine, child, ctx, script_path, &child_path, trace);

            match status {
                BtStatus::Success => {
                    self.states.write().remove(&state_key);
                    return BtStatus::Success;
                }
                BtStatus::Running => {
                    self.states.write().entry(state_key).or_default().child_index = i;
                    return BtStatus::Running;
                }
                BtStatus::Failure => {}
            }
        }

        self.states.write().remove(&state_key);
        BtStatus::Failure
    }

    fn run_force_sequence(
        &self,
        engine: &Engine,
        children: &[BtNode],
        ctx: &AiContext,
        script_path: &str,
        path: &str,
        trace: &mut Option<Vec<String>>,
    ) -> BtStatus {
        let mut any_running = false;
        let mut all_success = true;

        for (i, child) in children.iter().enumerate() {
            let child_path = format!("{}/force_{}", path, i);
            match self.run_node(engine, child, ctx, script_path, &child_path, trace) {
                BtStatus::Running => any_running = true,
                BtStatus::Failure => all_success = false,
                BtStatus::Success => {}
            }
        }

        if any_running { BtStatus::Running }
        else if all_success { BtStatus::Success }
        else { BtStatus::Failure }
    }

    fn run_action(
        &self,
        engine: &Engine,
        name: &str,
        ctx: &AiContext,
        script_path: &str,
        trace: &mut Option<Vec<String>>,
    ) -> BtStatus {
        if let Some(t) = trace {
            t.push(format!("  -> Action: {}", name));
        }

        let scripts = self.scripts.read();
        let ast = match scripts.get(script_path) {
            Some(ast) => ast,
            None => {
                tracing::warn!("Script not found for action: {}", script_path);
                return BtStatus::Failure;
            }
        };

        let mut scope = Scope::new();
        let ctx_dynamic = ctx.to_dynamic();

        match engine.call_fn::<Dynamic>(&mut scope, ast, name, (ctx_dynamic,)) {
            Ok(result) => parse_status_result(result),
            Err(e) => {
                if !e.to_string().contains("Function not found") {
                    tracing::warn!("Action '{}' failed: {}", name, e);
                }
                BtStatus::Failure
            }
        }
    }

    fn run_condition(
        &self,
        engine: &Engine,
        name: &str,
        ctx: &AiContext,
        script_path: &str,
        trace: &mut Option<Vec<String>>,
    ) -> BtStatus {
        if let Some(t) = trace {
            t.push(format!("  -> Condition: {}", name));
        }

        let scripts = self.scripts.read();
        let ast = match scripts.get(script_path) {
            Some(ast) => ast,
            None => {
                tracing::warn!("Script not found for condition: {}", script_path);
                return BtStatus::Failure;
            }
        };

        let mut scope = Scope::new();
        match engine.call_fn::<bool>(&mut scope, ast, name, (ctx.to_dynamic(),)) {
            Ok(result) => if result { BtStatus::Success } else { BtStatus::Failure },
            Err(e) => {
                tracing::warn!("Condition '{}' failed: {}", name, e);
                BtStatus::Failure
            }
        }
    }

    fn run_utility(
        &self,
        engine: &Engine,
        options: &[UtilityOption],
        ctx: &AiContext,
        script_path: &str,
        trace: &mut Option<Vec<String>>,
    ) -> BtStatus {
        if options.is_empty() {
            return BtStatus::Failure;
        }

        let scripts = self.scripts.read();
        let ast = match scripts.get(script_path) {
            Some(ast) => ast,
            None => {
                tracing::warn!("Script not found for utility: {}", script_path);
                return BtStatus::Failure;
            }
        };

        let ctx_dynamic = ctx.to_dynamic();
        let mut scores = Vec::new();

        for option in options {
            let mut consideration_scores = Vec::new();

            for consideration in &option.considerations {
                let mut scope = Scope::new();
                match engine.call_fn::<f64>(&mut scope, ast, consideration, (ctx_dynamic.clone(),)) {
                    Ok(score) => consideration_scores.push(score.clamp(0.0, 1.0)),
                    Err(e) => {
                        tracing::warn!("Consideration '{}' failed: {}", consideration, e);
                        consideration_scores.push(0.0);
                    }
                }
            }

            scores.push(UtilityScore::new(option.clone(), consideration_scores));
        }

        let decision = UtilityDecision::from_scores(scores);

        if let Some(t) = trace {
            t.push(format!("  -> Utility decision: {:?}", decision.action()));
        }

        // Execute the chosen action
        if let Some(action_name) = decision.action() {
            drop(scripts);
            self.run_action(engine, action_name, ctx, script_path, trace)
        } else {
            BtStatus::Failure
        }
    }

    fn run_decorator(
        &self,
        engine: &Engine,
        kind: &DecoratorKind,
        child: &BtNode,
        ctx: &AiContext,
        script_path: &str,
        path: &str,
        trace: &mut Option<Vec<String>>,
    ) -> BtStatus {
        let state_key = (ctx.entity_id, path.to_string());
        let child_path = format!("{}/child", path);

        match kind {
            DecoratorKind::Invert => {
                match self.run_node(engine, child, ctx, script_path, &child_path, trace) {
                    BtStatus::Success => BtStatus::Failure,
                    BtStatus::Failure => BtStatus::Success,
                    BtStatus::Running => BtStatus::Running,
                }
            }

            DecoratorKind::AlwaysSucceed => {
                let _ = self.run_node(engine, child, ctx, script_path, &child_path, trace);
                BtStatus::Success
            }

            DecoratorKind::AlwaysFail => {
                let _ = self.run_node(engine, child, ctx, script_path, &child_path, trace);
                BtStatus::Failure
            }

            DecoratorKind::Repeat(count) => {
                {
                    let states = self.states.read();
                    if let Some(state) = states.get(&state_key) {
                        if let Some(max) = count {
                            if state.repeat_count >= *max {
                                drop(states);
                                self.states.write().remove(&state_key);
                                return BtStatus::Success;
                            }
                        }
                    }
                }

                let status = self.run_node(engine, child, ctx, script_path, &child_path, trace);

                match status {
                    BtStatus::Success => {
                        self.states.write().entry(state_key).or_default().repeat_count += 1;
                        BtStatus::Running
                    }
                    BtStatus::Failure => {
                        self.states.write().remove(&state_key);
                        BtStatus::Failure
                    }
                    BtStatus::Running => BtStatus::Running,
                }
            }

            DecoratorKind::UntilFail => {
                match self.run_node(engine, child, ctx, script_path, &child_path, trace) {
                    BtStatus::Failure => BtStatus::Success,
                    _ => BtStatus::Running,
                }
            }

            DecoratorKind::Cooldown(seconds) => {
                {
                    let states = self.states.read();
                    if let Some(state) = states.get(&state_key) {
                        if let Some(last) = state.last_execution {
                            if ctx.game_time - last < *seconds {
                                return BtStatus::Failure;
                            }
                        }
                    }
                }

                let status = self.run_node(engine, child, ctx, script_path, &child_path, trace);

                if status == BtStatus::Success {
                    self.states.write().entry(state_key).or_default().last_execution = Some(ctx.game_time);
                }

                status
            }

            DecoratorKind::RunOnce => {
                {
                    let states = self.states.read();
                    if let Some(state) = states.get(&state_key) {
                        if let Some(result) = state.stored_result {
                            return result;
                        }
                    }
                }

                let status = self.run_node(engine, child, ctx, script_path, &child_path, trace);

                if status != BtStatus::Running {
                    self.states.write().entry(state_key).or_default().stored_result = Some(status);
                }

                status
            }

            DecoratorKind::Timeout(seconds) => {
                let time_key = (ctx.entity_id, format!("{}_start", path));

                {
                    let states = self.states.read();
                    if let Some(state) = states.get(&time_key) {
                        if let Some(start) = state.last_execution {
                            if ctx.game_time - start > *seconds {
                                return BtStatus::Failure;
                            }
                        }
                    }
                }

                {
                    let mut states = self.states.write();
                    let state = states.entry(time_key.clone()).or_default();
                    if state.last_execution.is_none() {
                        state.last_execution = Some(ctx.game_time);
                    }
                }

                let status = self.run_node(engine, child, ctx, script_path, &child_path, trace);

                if status != BtStatus::Running {
                    self.states.write().remove(&time_key);
                }

                status
            }
        }
    }

    fn run_wait(&self, ticks: u32, ctx: &AiContext, path: &str) -> BtStatus {
        let state_key = (ctx.entity_id, path.to_string());

        let waited = {
            let mut states = self.states.write();
            let state = states.entry(state_key.clone()).or_default();
            state.ticks_waited += 1;
            state.ticks_waited
        };

        if waited >= ticks {
            self.states.write().remove(&state_key);
            BtStatus::Success
        } else {
            BtStatus::Running
        }
    }

    fn run_subtree(
        &self,
        engine: &Engine,
        name: &str,
        ctx: &AiContext,
        script_path: &str,
        path: &str,
        trace: &mut Option<Vec<String>>,
    ) -> BtStatus {
        let subtree = self.subtrees.read().get(name).cloned();

        match subtree {
            Some(tree) => {
                let subtree_path = format!("{}/sub_{}", path, name);
                self.run_node(engine, &tree, ctx, script_path, &subtree_path, trace)
            }
            None => {
                tracing::warn!("Subtree not found: {}", name);
                BtStatus::Failure
            }
        }
    }
}

/// Parse a Dynamic result into a BtStatus.
fn parse_status_result(result: Dynamic) -> BtStatus {
    // Check for string status
    if let Some(status_str) = result.clone().try_cast::<String>() {
        return match status_str.to_lowercase().as_str() {
            "success" => BtStatus::Success,
            "failure" | "fail" => BtStatus::Failure,
            "running" => BtStatus::Running,
            _ => BtStatus::Success,
        };
    }

    // Check for bool
    if let Some(success) = result.clone().try_cast::<bool>() {
        return if success { BtStatus::Success } else { BtStatus::Failure };
    }

    // Check for map with status field
    if let Some(map) = result.try_cast::<Map>() {
        if let Some(status) = map.get("status") {
            if let Some(s) = status.clone().try_cast::<String>() {
                return match s.to_lowercase().as_str() {
                    "success" => BtStatus::Success,
                    "failure" | "fail" => BtStatus::Failure,
                    "running" => BtStatus::Running,
                    _ => BtStatus::Success,
                };
            }
        }
    }

    // Default to success if function completed without error
    BtStatus::Success
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> AiContext {
        AiContext::new(Uuid::new_v4(), 1, 0.016, 1.0)
    }

    #[test]
    fn test_simple_sequence() {
        let engine = Engine::new();
        let runner = BehaviorTreeRunner::new();
        let ctx = create_test_context();

        let tree = BtNode::sequence(vec![BtNode::Succeed, BtNode::Succeed]);
        let result = runner.run(&engine, &tree, &ctx, "test");
        assert_eq!(result.status, BtStatus::Success);
    }

    #[test]
    fn test_sequence_failure() {
        let engine = Engine::new();
        let runner = BehaviorTreeRunner::new();
        let ctx = create_test_context();

        let tree = BtNode::sequence(vec![BtNode::Succeed, BtNode::Fail, BtNode::Succeed]);
        let result = runner.run(&engine, &tree, &ctx, "test");
        assert_eq!(result.status, BtStatus::Failure);
    }

    #[test]
    fn test_selector_success() {
        let engine = Engine::new();
        let runner = BehaviorTreeRunner::new();
        let ctx = create_test_context();

        let tree = BtNode::selector(vec![BtNode::Fail, BtNode::Succeed, BtNode::Fail]);
        let result = runner.run(&engine, &tree, &ctx, "test");
        assert_eq!(result.status, BtStatus::Success);
    }

    #[test]
    fn test_selector_all_fail() {
        let engine = Engine::new();
        let runner = BehaviorTreeRunner::new();
        let ctx = create_test_context();

        let tree = BtNode::selector(vec![BtNode::Fail, BtNode::Fail]);
        let result = runner.run(&engine, &tree, &ctx, "test");
        assert_eq!(result.status, BtStatus::Failure);
    }

    #[test]
    fn test_invert_decorator() {
        let engine = Engine::new();
        let runner = BehaviorTreeRunner::new();
        let ctx = create_test_context();

        let tree = BtNode::invert(BtNode::Succeed);
        assert_eq!(runner.run(&engine, &tree, &ctx, "test").status, BtStatus::Failure);

        let tree = BtNode::invert(BtNode::Fail);
        assert_eq!(runner.run(&engine, &tree, &ctx, "test").status, BtStatus::Success);
    }

    #[test]
    fn test_wait_node() {
        let engine = Engine::new();
        let runner = BehaviorTreeRunner::new();
        let ctx = create_test_context();

        let tree = BtNode::wait(3);

        // First two ticks should return Running
        assert_eq!(runner.run(&engine, &tree, &ctx, "test").status, BtStatus::Running);
        assert_eq!(runner.run(&engine, &tree, &ctx, "test").status, BtStatus::Running);

        // Third tick should succeed
        assert_eq!(runner.run(&engine, &tree, &ctx, "test").status, BtStatus::Success);
    }
}
