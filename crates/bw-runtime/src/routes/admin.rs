//! Admin API routes
//!
//! Provides GM/admin tools for script debugging, behavior inspection, and management.

use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use rhai::Map;

use bw_game::state::AccessPermissions;
use crate::auth::AdminAuth;
use crate::GameState;
use crate::scripting::{LogLevel, ScriptLogEntry};

/// Build the admin router.
pub fn router() -> Router<Arc<GameState>> {
    Router::new()
        // Full reload (config + scripts)
        .route("/reload", post(reload_all))
        .route("/reload/config", post(reload_config))
        // Script management
        .route("/scripts", get(list_scripts))
        .route("/scripts/reload", post(reload_all_scripts))
        .route("/scripts/reload/{*path}", post(reload_script))
        .route("/scripts/eval", post(eval_script))
        // Behavior management
        .route("/behaviors", get(list_behaviors))
        .route("/behaviors/{id}", get(get_behavior))
        .route("/behaviors/{id}/pause", post(pause_behavior))
        .route("/behaviors/{id}/resume", post(resume_behavior))
        .route("/behaviors/{id}", delete(detach_behavior))
        .route("/behaviors/{id}/data", get(get_behavior_data))
        .route("/behaviors/{id}/data", post(set_behavior_data))
        // Coroutine management
        .route("/coroutines", get(list_coroutines))
        .route("/coroutines/{id}/cancel", post(cancel_coroutine))
        // Event subscriptions
        .route("/events/subscriptions", get(list_subscriptions))
        .route("/events/subscriptions/{id}", delete(unsubscribe))
        // Script logs
        .route("/logs", get(get_logs))
        .route("/logs/clear", post(clear_logs))
        // Stats
        .route("/stats", get(get_stats))
        // Config info
        .route("/config", get(get_config))
}

// =============================================================================
// Reload Management
// =============================================================================

/// Reload all watchable resources (config + scripts).
async fn reload_all(
    admin: AdminAuth,
    State(state): State<Arc<GameState>>,
) -> Json<ActionResponse> {
    tracing::info!(admin = %admin.username, "Reloading all resources via admin API");

    // Reload config
    if let Some(config) = crate::config::try_config() {
        config.reload();
    }

    // Reload scripts
    match state.scripts.load_all_scripts() {
        Ok(()) => {
            let count = state.scripts.loaded_scripts().len();
            state.log_script_info(format!("All resources reloaded by {} (HTTP)", admin.username));

            // Reinitialize all action scripts (clears old handlers and re-runs init())
            state.reinitialize_all_action_scripts();

            Json(ActionResponse {
                success: true,
                message: Some(format!("Reloaded config and {} scripts", count)),
            })
        }
        Err(e) => Json(ActionResponse {
            success: false,
            message: Some(format!("Script reload failed: {}", e)),
        }),
    }
}

/// Reload only the config file.
async fn reload_config(
    admin: AdminAuth,
) -> Json<ActionResponse> {
    tracing::info!(admin = %admin.username, "Reloading config via admin API");

    if let Some(config) = crate::config::try_config() {
        config.reload();
        Json(ActionResponse {
            success: true,
            message: Some("Config reloaded".to_string()),
        })
    } else {
        Json(ActionResponse {
            success: false,
            message: Some("Config manager not initialized".to_string()),
        })
    }
}

/// Get current config (non-sensitive parts).
#[derive(Serialize)]
pub struct ConfigResponse {
    server: ServerConfigInfo,
    admin: AdminConfigInfo,
    scripting: ScriptingConfigInfo,
}

#[derive(Serialize)]
pub struct ServerConfigInfo {
    host: String,
    port: u16,
    database_configured: bool,
}

#[derive(Serialize)]
pub struct AdminConfigInfo {
    admin_count: usize,
}

#[derive(Serialize)]
pub struct ScriptingConfigInfo {
    scripts_dir: String,
    hot_reload: bool,
    max_log_entries: usize,
}

async fn get_config(
    _admin: AdminAuth,
) -> Json<ConfigResponse> {
    let config = crate::config::config().get();

    Json(ConfigResponse {
        server: ServerConfigInfo {
            host: config.server.host.clone(),
            port: config.server.port,
            database_configured: config.server.database_url.is_some(),
        },
        admin: AdminConfigInfo {
            admin_count: config.admin.usernames.len(),
        },
        scripting: ScriptingConfigInfo {
            scripts_dir: config.scripting.scripts_dir.clone(),
            hot_reload: config.scripting.hot_reload,
            max_log_entries: config.scripting.max_log_entries,
        },
    })
}

// =============================================================================
// Script Management
// =============================================================================

#[derive(Serialize)]
pub struct ScriptsResponse {
    scripts: Vec<String>,
    count: usize,
}

async fn list_scripts(
    _admin: AdminAuth,
    State(state): State<Arc<GameState>>,
) -> Json<ScriptsResponse> {
    let scripts = state.scripts.loaded_scripts();
    let count = scripts.len();
    Json(ScriptsResponse { scripts, count })
}

#[derive(Serialize)]
pub struct ReloadResponse {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reloaded_count: Option<usize>,
}

async fn reload_all_scripts(
    admin: AdminAuth,
    State(state): State<Arc<GameState>>,
) -> Json<ReloadResponse> {
    tracing::info!(admin = %admin.username, "Reloading all scripts");

    match state.scripts.load_all_scripts() {
        Ok(()) => {
            let count = state.scripts.loaded_scripts().len();
            state.log_script_info(format!("All scripts reloaded by {}", admin.username));

            // Reinitialize all action scripts (clears old handlers and re-runs init())
            state.reinitialize_all_action_scripts();

            Json(ReloadResponse {
                success: true,
                message: format!("Reloaded {} scripts", count),
                reloaded_count: Some(count),
            })
        }
        Err(e) => Json(ReloadResponse {
            success: false,
            message: format!("Reload failed: {}", e),
            reloaded_count: None,
        }),
    }
}

async fn reload_script(
    admin: AdminAuth,
    State(state): State<Arc<GameState>>,
    Path(path): Path<String>,
) -> Json<ReloadResponse> {
    tracing::info!(admin = %admin.username, script = %path, "Reloading script");

    match state.scripts.load_script(&path) {
        Ok(()) => {
            state.log_script_info(format!("Script {} reloaded by {}", path, admin.username));

            // Reinitialize if this is an action script (calls init() to re-register handlers)
            state.reinitialize_action_script(&path);

            Json(ReloadResponse {
                success: true,
                message: format!("Reloaded: {}", path),
                reloaded_count: Some(1),
            })
        }
        Err(e) => Json(ReloadResponse {
            success: false,
            message: format!("Failed to reload {}: {}", path, e),
            reloaded_count: None,
        }),
    }
}

#[derive(Deserialize)]
pub struct EvalRequest {
    code: String,
    #[serde(default)]
    trusted: bool,
}

#[derive(Serialize)]
pub struct EvalResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn eval_script(
    admin: AdminAuth,
    State(state): State<Arc<GameState>>,
    Json(req): Json<EvalRequest>,
) -> Json<EvalResponse> {
    tracing::info!(
        admin = %admin.username,
        code_len = req.code.len(),
        trusted = req.trusted,
        "Evaluating script"
    );

    // Set up state accessor with appropriate permissions
    if let Some(accessor) = state.state_accessor.read().as_ref() {
        if req.trusted {
            accessor.set_permissions(AccessPermissions::trusted());
        } else {
            accessor.set_permissions(AccessPermissions::read_only());
        }
    }

    match state.scripts.eval::<rhai::Dynamic>(&req.code) {
        Ok(result) => Json(EvalResponse {
            success: true,
            result: Some(format!("{:?}", result)),
            error: None,
        }),
        Err(e) => Json(EvalResponse {
            success: false,
            result: None,
            error: Some(e.to_string()),
        }),
    }
}

// =============================================================================
// Behavior Management
// =============================================================================

#[derive(Serialize)]
pub struct BehaviorInfo {
    id: String,
    entity_id: String,
    entity_type: String,
    script_path: String,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sector_id: Option<String>,
}

#[derive(Serialize)]
pub struct BehaviorsResponse {
    behaviors: Vec<BehaviorInfo>,
    total: usize,
    active: usize,
    paused: usize,
}

#[derive(Deserialize)]
pub struct BehaviorQuery {
    #[serde(default)]
    entity_id: Option<Uuid>,
    #[serde(default)]
    script: Option<String>,
}

async fn list_behaviors(
    _admin: AdminAuth,
    State(state): State<Arc<GameState>>,
    Query(query): Query<BehaviorQuery>,
) -> Json<BehaviorsResponse> {
    let bm = state.behavior_manager.read();
    let all_behaviors = bm.list_all();

    let behaviors: Vec<BehaviorInfo> = all_behaviors
        .iter()
        .filter(|b| {
            // Apply filters
            if let Some(entity_id) = query.entity_id
                && b.entity_id != entity_id {
                    return false;
                }
            if let Some(ref script) = query.script
                && !b.script_path.contains(script) {
                    return false;
                }
            true
        })
        .map(|b| BehaviorInfo {
            id: b.id.to_string(),
            entity_id: b.entity_id.to_string(),
            entity_type: b.entity_type.as_str().to_string(),
            script_path: b.script_path.clone(),
            state: format!("{:?}", b.state),
            sector_id: b.sector_id.map(|id| id.to_string()),
        })
        .collect();

    let total = behaviors.len();
    let active = behaviors.iter().filter(|b| b.state == "Active").count();
    let paused = behaviors.iter().filter(|b| b.state == "Paused").count();

    Json(BehaviorsResponse {
        behaviors,
        total,
        active,
        paused,
    })
}

async fn get_behavior(
    _admin: AdminAuth,
    State(state): State<Arc<GameState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<BehaviorInfo>, StatusCode> {
    let bm = state.behavior_manager.read();
    let behavior = bm.get(id).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(BehaviorInfo {
        id: behavior.id.to_string(),
        entity_id: behavior.entity_id.to_string(),
        entity_type: behavior.entity_type.as_str().to_string(),
        script_path: behavior.script_path,
        state: format!("{:?}", behavior.state),
        sector_id: behavior.sector_id.map(|id| id.to_string()),
    }))
}

#[derive(Serialize)]
pub struct ActionResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

async fn pause_behavior(
    admin: AdminAuth,
    State(state): State<Arc<GameState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ActionResponse>, StatusCode> {
    tracing::info!(admin = %admin.username, behavior = %id, "Pausing behavior");

    let success = state.behavior_manager.write().pause(id);
    if success {
        Ok(Json(ActionResponse {
            success: true,
            message: Some("Behavior paused".to_string()),
        }))
    } else {
        Ok(Json(ActionResponse {
            success: false,
            message: Some("Behavior not found or not active".to_string()),
        }))
    }
}

async fn resume_behavior(
    admin: AdminAuth,
    State(state): State<Arc<GameState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ActionResponse>, StatusCode> {
    tracing::info!(admin = %admin.username, behavior = %id, "Resuming behavior");

    let success = state.behavior_manager.write().resume(id);
    if success {
        Ok(Json(ActionResponse {
            success: true,
            message: Some("Behavior resumed".to_string()),
        }))
    } else {
        Ok(Json(ActionResponse {
            success: false,
            message: Some("Behavior not found or not paused".to_string()),
        }))
    }
}

async fn detach_behavior(
    admin: AdminAuth,
    State(state): State<Arc<GameState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ActionResponse>, StatusCode> {
    tracing::info!(admin = %admin.username, behavior = %id, "Detaching behavior");

    let success = state.behavior_manager.write().detach(id);
    if success {
        Ok(Json(ActionResponse {
            success: true,
            message: Some("Behavior detached".to_string()),
        }))
    } else {
        Ok(Json(ActionResponse {
            success: false,
            message: Some("Behavior not found".to_string()),
        }))
    }
}

#[derive(Serialize)]
pub struct BehaviorDataResponse {
    behavior_id: String,
    local_data: serde_json::Value,
}

async fn get_behavior_data(
    _admin: AdminAuth,
    State(state): State<Arc<GameState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<BehaviorDataResponse>, StatusCode> {
    let bm = state.behavior_manager.read();
    let behavior = bm.get(id).ok_or(StatusCode::NOT_FOUND)?;

    // Convert Rhai Map to JSON
    let data = rhai_map_to_json(&behavior.local_data);

    Ok(Json(BehaviorDataResponse {
        behavior_id: id.to_string(),
        local_data: data,
    }))
}

#[derive(Deserialize)]
pub struct SetBehaviorDataRequest {
    local_data: serde_json::Value,
}

async fn set_behavior_data(
    admin: AdminAuth,
    State(state): State<Arc<GameState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<SetBehaviorDataRequest>,
) -> Result<Json<ActionResponse>, StatusCode> {
    tracing::info!(admin = %admin.username, behavior = %id, "Setting behavior data");

    // Convert JSON to Rhai Map
    let map = json_to_rhai_map(&req.local_data);

    let success = state.behavior_manager.write().set_local_data(id, map);
    if success {
        Ok(Json(ActionResponse {
            success: true,
            message: Some("Behavior data updated".to_string()),
        }))
    } else {
        Ok(Json(ActionResponse {
            success: false,
            message: Some("Behavior not found".to_string()),
        }))
    }
}

// =============================================================================
// Coroutine Management
// =============================================================================

#[derive(Serialize)]
pub struct CoroutineInfo {
    id: String,
    script_path: String,
    function_name: String,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sector_id: Option<String>,
}

#[derive(Serialize)]
pub struct CoroutinesResponse {
    coroutines: Vec<CoroutineInfo>,
    total: usize,
}

async fn list_coroutines(
    _admin: AdminAuth,
    State(state): State<Arc<GameState>>,
) -> Json<CoroutinesResponse> {
    let cs = state.coroutine_scheduler.read();
    let all_coroutines = cs.list_all();

    let coroutines: Vec<CoroutineInfo> = all_coroutines
        .iter()
        .map(|c| CoroutineInfo {
            id: c.id.to_string(),
            script_path: c.script_path.clone(),
            function_name: c.function_name.clone(),
            state: format!("{:?}", c.state),
            owner_entity_id: c.owner_entity_id.map(|id| id.to_string()),
            sector_id: c.sector_id.map(|id| id.to_string()),
        })
        .collect();

    let total = coroutines.len();

    Json(CoroutinesResponse { coroutines, total })
}

async fn cancel_coroutine(
    admin: AdminAuth,
    State(state): State<Arc<GameState>>,
    Path(id): Path<Uuid>,
) -> Json<ActionResponse> {
    tracing::info!(admin = %admin.username, coroutine = %id, "Cancelling coroutine");

    let success = state.coroutine_scheduler.write().cancel(id);
    Json(ActionResponse {
        success,
        message: if success {
            Some("Coroutine cancelled".to_string())
        } else {
            Some("Coroutine not found".to_string())
        },
    })
}

// =============================================================================
// Event Subscriptions
// =============================================================================

#[derive(Serialize)]
pub struct SubscriptionInfo {
    id: String,
    script_path: String,
    handler_function: String,
    event_types: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_entity_id: Option<String>,
    enabled: bool,
    priority: i32,
}

#[derive(Serialize)]
pub struct SubscriptionsResponse {
    subscriptions: Vec<SubscriptionInfo>,
    total: usize,
}

async fn list_subscriptions(
    _admin: AdminAuth,
    State(state): State<Arc<GameState>>,
) -> Json<SubscriptionsResponse> {
    let all_subs = state.event_registry.list_all();

    let subscriptions: Vec<SubscriptionInfo> = all_subs
        .iter()
        .map(|s| SubscriptionInfo {
            id: s.id.to_string(),
            script_path: s.script_path.clone(),
            handler_function: s.handler_function.clone(),
            event_types: s.event_types.iter().map(|e| format!("{:?}", e)).collect(),
            owner_entity_id: s.owner_entity_id.map(|id| id.to_string()),
            enabled: s.enabled,
            priority: s.priority,
        })
        .collect();

    let total = subscriptions.len();

    Json(SubscriptionsResponse {
        subscriptions,
        total,
    })
}

async fn unsubscribe(
    admin: AdminAuth,
    State(state): State<Arc<GameState>>,
    Path(id): Path<Uuid>,
) -> Json<ActionResponse> {
    tracing::info!(admin = %admin.username, subscription = %id, "Removing subscription");

    let success = state.event_registry.unsubscribe(id);
    Json(ActionResponse {
        success,
        message: if success {
            Some("Subscription removed".to_string())
        } else {
            Some("Subscription not found".to_string())
        },
    })
}

// =============================================================================
// Script Logs
// =============================================================================

#[derive(Deserialize)]
pub struct LogsQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    level: Option<String>,
}

fn default_limit() -> usize {
    100
}

#[derive(Serialize)]
pub struct LogsResponse {
    logs: Vec<ScriptLogEntry>,
    total: usize,
    errors: usize,
    warnings: usize,
}

async fn get_logs(
    _admin: AdminAuth,
    State(state): State<Arc<GameState>>,
    Query(query): Query<LogsQuery>,
) -> Json<LogsResponse> {
    let buffer = state.script_logs.read();

    let level_filter: Option<LogLevel> = query.level.as_ref().and_then(|l| match l.to_lowercase().as_str() {
        "info" => Some(LogLevel::Info),
        "warning" | "warn" => Some(LogLevel::Warning),
        "error" => Some(LogLevel::Error),
        _ => None,
    });

    let logs: Vec<ScriptLogEntry> = if let Some(level) = level_filter {
        buffer.filter_by_level(level)
            .into_iter()
            .take(query.limit)
            .cloned()
            .collect()
    } else {
        buffer.recent(query.limit)
            .into_iter()
            .cloned()
            .collect()
    };

    let total = buffer.len();
    let errors = buffer.error_count();
    let warnings = buffer.warning_count();

    Json(LogsResponse {
        logs,
        total,
        errors,
        warnings,
    })
}

async fn clear_logs(
    admin: AdminAuth,
    State(state): State<Arc<GameState>>,
) -> Json<ActionResponse> {
    tracing::info!(admin = %admin.username, "Clearing script logs");

    state.script_logs.write().clear();
    Json(ActionResponse {
        success: true,
        message: Some("Logs cleared".to_string()),
    })
}

// =============================================================================
// Stats
// =============================================================================

#[derive(Serialize)]
pub struct ScriptingStats {
    loaded_scripts: usize,
    active_behaviors: usize,
    active_coroutines: usize,
    event_subscriptions: usize,
    log_entries: usize,
    log_errors: usize,
    log_warnings: usize,
    log_capacity: usize,
    log_dropped: u64,
}

async fn get_stats(
    _admin: AdminAuth,
    State(state): State<Arc<GameState>>,
) -> Json<ScriptingStats> {
    let loaded_scripts = state.scripts.loaded_scripts().len();
    let active_behaviors = state.behavior_manager.read().active_count();
    let active_coroutines = state.coroutine_scheduler.read().active_count();
    let event_subscriptions = state.event_registry.count();
    let log_stats = state.script_logs.read().stats();

    Json(ScriptingStats {
        loaded_scripts,
        active_behaviors,
        active_coroutines,
        event_subscriptions,
        log_entries: log_stats.len,
        log_errors: log_stats.errors,
        log_warnings: log_stats.warnings,
        log_capacity: log_stats.capacity,
        log_dropped: log_stats.dropped,
    })
}

// =============================================================================
// Helpers
// =============================================================================

/// Convert a Rhai Map to serde_json::Value.
fn rhai_map_to_json(map: &Map) -> serde_json::Value {
    let mut json_map = serde_json::Map::new();

    for (key, value) in map.iter() {
        let json_value = rhai_dynamic_to_json(value);
        json_map.insert(key.to_string(), json_value);
    }

    serde_json::Value::Object(json_map)
}

/// Convert a Rhai Dynamic to serde_json::Value.
fn rhai_dynamic_to_json(value: &rhai::Dynamic) -> serde_json::Value {
    if value.is_unit() {
        serde_json::Value::Null
    } else if let Ok(b) = value.as_bool() {
        serde_json::Value::Bool(b)
    } else if let Ok(i) = value.as_int() {
        serde_json::Value::Number(i.into())
    } else if let Ok(f) = value.as_float() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            serde_json::Value::Number(n)
        } else {
            serde_json::Value::Null
        }
    } else if let Ok(s) = value.clone().into_string() {
        serde_json::Value::String(s)
    } else if let Some(map) = value.clone().try_cast::<Map>() {
        rhai_map_to_json(&map)
    } else if let Some(arr) = value.clone().try_cast::<Vec<rhai::Dynamic>>() {
        let json_arr: Vec<serde_json::Value> = arr.iter().map(rhai_dynamic_to_json).collect();
        serde_json::Value::Array(json_arr)
    } else {
        // Fallback: convert to debug string
        serde_json::Value::String(format!("{:?}", value))
    }
}

/// Convert serde_json::Value to Rhai Map.
fn json_to_rhai_map(value: &serde_json::Value) -> Map {
    let mut map = Map::new();

    if let serde_json::Value::Object(obj) = value {
        for (key, val) in obj {
            let rhai_val = json_to_rhai_dynamic(val);
            map.insert(key.clone().into(), rhai_val);
        }
    }

    map
}

/// Convert serde_json::Value to Rhai Dynamic.
fn json_to_rhai_dynamic(value: &serde_json::Value) -> rhai::Dynamic {
    match value {
        serde_json::Value::Null => rhai::Dynamic::UNIT,
        serde_json::Value::Bool(b) => rhai::Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rhai::Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                rhai::Dynamic::from(f)
            } else {
                rhai::Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => rhai::Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let rhai_arr: Vec<rhai::Dynamic> = arr.iter().map(json_to_rhai_dynamic).collect();
            rhai::Dynamic::from(rhai_arr)
        }
        serde_json::Value::Object(_) => {
            rhai::Dynamic::from(json_to_rhai_map(value))
        }
    }
}
