//! Main game page
//!
//! Full-featured game interface with WebSocket connection and reactive state.

use leptos::prelude::*;
use gloo_timers::callback::Interval;

use crate::api::{ConnectionState, WsService};
use crate::components::game::*;
use crate::state::{GameState, MobilePanel};
use crate::utils::distance;

#[component]
pub fn GamePage() -> impl IntoView {
    let game_state = expect_context::<GameState>();
    let ws = expect_context::<WsService>();

    // Connect to WebSocket on mount
    Effect::new(move |_| {
        // Only connect if not already connected/connecting
        let state = ws.state.get();
        if state == ConnectionState::Disconnected {
            // Get token from localStorage
            if let Some(token) = get_auth_token() {
                ws.connect(game_state, token);
            } else {
                // No token - redirect to register
                if let Some(window) = web_sys::window() {
                    let _ = window.location().set_href("/play/register");
                }
            }
        }
    });

    // Periodic cleanup of expired hails (every second)
    let _hail_cleanup_interval = Interval::new(1_000, move || {
        game_state.cleanup_expired_hails();
    });

    // Reactive state for UI
    let _connected = move || game_state.connected.get();
    let server_tick = move || game_state.server_tick.get();
    let sector_name = move || game_state.sector_name.get();
    let ship_status = move || game_state.ship_status.get();
    let last_error = move || game_state.last_error.get();
    let notification = move || game_state.notification.get();

    // Sidebar tab state
    let left_tab = RwSignal::new(LeftTab::Missions);
    let right_tab = RwSignal::new(RightTab::Ship);

    // Footer action handlers
    let ws_dock = ws;
    let ws_stop = ws;
    let ws_alert = ws;

    let handle_dock = move |_| {
        // Maximum distance to dock at a station
        const DOCK_RANGE: f64 = 50.0;

        // Find nearest station and check proximity
        let locations = game_state.locations.get();
        let player_x = game_state.position_x.get();
        let player_y = game_state.position_y.get();

        // Find the nearest station
        if let Some(station) = locations.iter()
            .filter(|l| l.location_type == "station")
            .min_by(|a, b| {
                let dist_a = distance(player_x, player_y, a.x, a.y);
                let dist_b = distance(player_x, player_y, b.x, b.y);
                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            })
        {
            let dist = distance(player_x, player_y, station.x, station.y);
            if dist <= DOCK_RANGE {
                // Set docked station info before docking
                game_state.docked_station_id.set(Some(station.id));
                game_state.docked_station_name.set(station.name.clone());
                game_state.docked_station_services.set(station.services.clone());
                ws_dock.dock(station.id);
            } else {
                game_state.set_error(format!(
                    "Too far from {} ({:.0} units). Move closer to dock.",
                    station.name, dist
                ));
            }
        } else {
            game_state.set_error("No station in this sector".to_string());
        }
    };

    let handle_stop = move |_| {
        ws_stop.stop_movement();
    };

    let handle_alert = move |_| {
        // Toggle alert status / disengage combat
        ws_alert.disengage_combat();
    };

    // Check if initial state has been received (player_id is set after InitialState)
    let has_initial_state = move || game_state.player_id.get().is_some();
    let connection_state = move || ws.state.get();

    // Z-index hierarchy (documented here, applied throughout frontend):
    // z-[100]: Loading overlay (blocks everything during initial load)
    // z-60: Modal dialogs (confirm dialog, mission choice dialog)
    // z-50: Alert overlays (notification, error - important but not blocking)
    // z-40: Floating panels (station, debug, notifications, jump panel)
    // z-30/z-20/z-10: Map elements (tooltips, ships, locations)

    view! {
        <div class="h-screen flex flex-col bg-slate-900 text-slate-100">
            // Loading overlay - shown while connecting or waiting for initial state
            <Show when=move || !has_initial_state()>
                <div class="absolute inset-0 bg-slate-900 flex flex-col items-center justify-center z-[100]">
                    <div class="text-4xl font-bold text-amber-500 mb-8">"BLACKWING"</div>
                    <div class="flex flex-col items-center gap-4">
                        // Spinner
                        <div class="w-12 h-12 border-4 border-slate-700 border-t-amber-500 rounded-full animate-spin" />
                        // Status text
                        <div class="text-slate-400">
                            {move || match connection_state() {
                                ConnectionState::Disconnected => "Disconnected",
                                ConnectionState::Connecting => "Connecting to server...",
                                ConnectionState::Connected => "Loading game state...",
                                ConnectionState::Reconnecting => "Reconnecting...",
                            }}
                        </div>
                        // Show reconnect attempt count if reconnecting
                        <Show when=move || connection_state() == ConnectionState::Reconnecting>
                            <div class="text-xs text-slate-500">
                                "Attempt "{move || ws.reconnect_attempt()}" of 10"
                            </div>
                        </Show>
                    </div>
                </div>
            </Show>

            // Top bar with resources (condensed on mobile)
            <header class="h-12 md:h-16 bg-slate-800 border-b border-slate-700 flex items-center px-2 md:px-4">
                <div class="flex-1 flex items-center gap-2 md:gap-4 min-w-0">
                    // Logo - abbreviated on mobile
                    <span class="text-lg md:text-xl font-bold text-amber-500 shrink-0">"BW"</span>
                    <span class="hidden md:inline text-xl font-bold text-amber-500">"ING"</span>
                    // Sector name - hidden on mobile (shown in map)
                    <span class="hidden md:inline text-sm text-slate-400">{sector_name}</span>
                    // Status badge - smaller on mobile
                    <span class=move || {
                        let status = ship_status();
                        let color = status_color(&status);
                        format!("text-[10px] md:text-xs px-1.5 md:px-2 py-0.5 rounded bg-slate-700 shrink-0 {}", color)
                    }>
                        {move || status_display(&ship_status())}
                    </span>
                </div>
                // Full resource bar on desktop
                <div class="hidden md:block">
                    <ResourceBar />
                </div>
                // Condensed resources on mobile
                <MobileResourceBar />
                // Logout button - icon-only on mobile
                <button
                    class="ml-2 md:ml-4 p-2 md:px-3 md:py-1.5 text-xs text-slate-400 hover:text-slate-200 hover:bg-slate-700 rounded transition-colors min-h-[44px] md:min-h-0 flex items-center justify-center"
                    on:click=move |_| {
                        // Confirm before logging out
                        let confirmed = web_sys::window()
                            .and_then(|w| w.confirm_with_message("Are you sure you want to logout?").ok())
                            .unwrap_or(false);

                        if confirmed {
                            // Clear token from localStorage
                            if let Some(storage) = web_sys::window()
                                .and_then(|w| w.local_storage().ok())
                                .flatten()
                            {
                                let _ = storage.remove_item("auth_token");
                            }
                            // Redirect to login
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().set_href("/play/login");
                            }
                        }
                    }
                >
                    // Icon on mobile
                    <svg class="w-5 h-5 md:hidden" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M9 21H5a2 2 0 01-2-2V5a2 2 0 012-2h4M16 17l5-5-5-5M21 12H9" />
                    </svg>
                    // Text on desktop
                    <span class="hidden md:inline">"Logout"</span>
                </button>
            </header>

            // Main content
            <div class="flex-1 flex overflow-hidden relative">
                // Left sidebar - full-screen overlay on mobile when active
                <aside class=move || {
                    let panel = game_state.active_mobile_panel.get();
                    let is_visible = matches!(panel, MobilePanel::Missions | MobilePanel::Squadron);
                    if is_visible {
                        // Mobile: full-screen overlay
                        "fixed inset-0 z-50 bg-slate-800 flex flex-col md:relative md:inset-auto md:z-auto md:w-80 md:border-r md:border-slate-700"
                    } else {
                        // Mobile: hidden, Desktop: always visible
                        "hidden md:flex md:w-80 bg-slate-800 md:border-r md:border-slate-700 flex-col"
                    }
                }>
                    // Mobile header with close button
                    <div class="flex md:hidden items-center justify-between p-3 border-b border-slate-700">
                        <span class="font-semibold text-amber-400">
                            {move || match game_state.active_mobile_panel.get() {
                                MobilePanel::Missions => "Missions",
                                MobilePanel::Squadron => "Squadron",
                                _ => "",
                            }}
                        </span>
                        <button
                            class="p-2 text-slate-400 hover:text-slate-200 min-h-[44px] min-w-[44px] flex items-center justify-center"
                            on:click=move |_| game_state.active_mobile_panel.set(MobilePanel::Map)
                        >
                            <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M18 6L6 18M6 6l12 12" />
                            </svg>
                        </button>
                    </div>
                    // Tab buttons (desktop only - on mobile we show one panel at a time)
                    <div class="hidden md:flex border-b border-slate-700">
                        <TabButton
                            label="Missions"
                            active=move || left_tab.get() == LeftTab::Missions
                            on_click=move |_| left_tab.set(LeftTab::Missions)
                        />
                        <TabButton
                            label="Squadron"
                            active=move || left_tab.get() == LeftTab::Squadron
                            on_click=move |_| left_tab.set(LeftTab::Squadron)
                        />
                    </div>
                    // Tab content - on mobile show based on active_mobile_panel, on desktop use left_tab
                    <div class="flex-1 overflow-y-auto pb-16 md:pb-0">
                        {move || {
                            // On mobile, show panel based on active_mobile_panel
                            // On desktop, show based on left_tab
                            let mobile_panel = game_state.active_mobile_panel.get();
                            match mobile_panel {
                                MobilePanel::Missions => view! { <MissionPanel /> }.into_any(),
                                MobilePanel::Squadron => view! { <SquadronPanel /> }.into_any(),
                                // Desktop fallback
                                _ => match left_tab.get() {
                                    LeftTab::Missions => view! { <MissionPanel /> }.into_any(),
                                    LeftTab::Squadron => view! { <SquadronPanel /> }.into_any(),
                                }
                            }
                        }}
                    </div>
                </aside>

                // Center - sector map
                <main class="flex-1 relative">
                    <SectorMap />

                    // Station panel (when docked)
                    <StationPanel />

                    // Pending invites/proposals notifications
                    <NotificationsPanel />

                    // Notification overlay (responsive positioning)
                    // Design note: Notifications intentionally do NOT auto-dismiss.
                    // Players may be tabbed out or AFK, and need to see what happened
                    // when they return. Manual dismiss ensures nothing is missed.
                    <Show when=move || notification().is_some()>
                        <div class="absolute top-2 md:top-4 left-2 right-2 md:left-1/2 md:right-auto md:-translate-x-1/2 z-50">
                            <div class="bg-amber-900/90 border border-amber-500 rounded-lg px-3 md:px-4 py-2 shadow-lg flex items-center gap-2 md:gap-3">
                                <span class="text-amber-200 text-sm md:text-base flex-1">{move || notification().unwrap_or_default()}</span>
                                <button
                                    class="text-amber-400 hover:text-amber-200 p-1 min-h-[44px] min-w-[44px] md:min-h-0 md:min-w-0 flex items-center justify-center"
                                    on:click=move |_| game_state.notification.set(None)
                                >
                                    <svg class="w-5 h-5 md:w-4 md:h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M18 6L6 18M6 6l12 12" />
                                    </svg>
                                </button>
                            </div>
                        </div>
                    </Show>

                    // Error overlay (responsive positioning)
                    <Show when=move || last_error().is_some()>
                        <div class="absolute top-14 md:top-16 left-2 right-2 md:left-1/2 md:right-auto md:-translate-x-1/2 z-50">
                            <div class="bg-red-900/90 border border-red-500 rounded-lg px-3 md:px-4 py-2 shadow-lg flex items-center gap-2 md:gap-3">
                                <span class="text-red-200 text-sm md:text-base flex-1">{move || last_error().unwrap_or_default()}</span>
                                <button
                                    class="text-red-400 hover:text-red-200 p-1 min-h-[44px] min-w-[44px] md:min-h-0 md:min-w-0 flex items-center justify-center"
                                    on:click=move |_| game_state.clear_error()
                                >
                                    <svg class="w-5 h-5 md:w-4 md:h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M18 6L6 18M6 6l12 12" />
                                    </svg>
                                </button>
                            </div>
                        </div>
                    </Show>
                </main>

                // Right sidebar - full-screen overlay on mobile when active
                <aside class=move || {
                    let panel = game_state.active_mobile_panel.get();
                    let is_visible = matches!(panel, MobilePanel::Ship | MobilePanel::Comms | MobilePanel::Combat);
                    if is_visible {
                        // Mobile: full-screen overlay
                        "fixed inset-0 z-50 bg-slate-800 flex flex-col md:relative md:inset-auto md:z-auto md:w-80 md:border-l md:border-slate-700"
                    } else {
                        // Mobile: hidden, Desktop: always visible
                        "hidden md:flex md:w-80 bg-slate-800 md:border-l md:border-slate-700 flex-col"
                    }
                }>
                    // Mobile header with close button
                    <div class="flex md:hidden items-center justify-between p-3 border-b border-slate-700">
                        <span class="font-semibold text-amber-400">
                            {move || match game_state.active_mobile_panel.get() {
                                MobilePanel::Ship => "Ship",
                                MobilePanel::Comms => "Comms",
                                MobilePanel::Combat => "Combat",
                                _ => "",
                            }}
                        </span>
                        <button
                            class="p-2 text-slate-400 hover:text-slate-200 min-h-[44px] min-w-[44px] flex items-center justify-center"
                            on:click=move |_| game_state.active_mobile_panel.set(MobilePanel::Map)
                        >
                            <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M18 6L6 18M6 6l12 12" />
                            </svg>
                        </button>
                    </div>
                    // Tab buttons (desktop only)
                    <div class="hidden md:flex border-b border-slate-700">
                        <TabButton
                            label="Ship"
                            active=move || right_tab.get() == RightTab::Ship
                            on_click=move |_| right_tab.set(RightTab::Ship)
                        />
                        <TabButton
                            label="Comms"
                            active=move || right_tab.get() == RightTab::Comms
                            on_click=move |_| right_tab.set(RightTab::Comms)
                        />
                        <TabButton
                            label="Combat"
                            active=move || right_tab.get() == RightTab::Combat
                            on_click=move |_| right_tab.set(RightTab::Combat)
                        />
                    </div>
                    // Tab content - on mobile show based on active_mobile_panel, on desktop use right_tab
                    <div class="flex-1 overflow-hidden pb-16 md:pb-0">
                        {move || {
                            let mobile_panel = game_state.active_mobile_panel.get();
                            match mobile_panel {
                                MobilePanel::Ship => view! { <ShipStatus /> }.into_any(),
                                MobilePanel::Comms => view! { <CommsPanel /> }.into_any(),
                                MobilePanel::Combat => view! { <CombatLog /> }.into_any(),
                                // Desktop fallback
                                _ => match right_tab.get() {
                                    RightTab::Ship => view! { <ShipStatus /> }.into_any(),
                                    RightTab::Comms => view! { <CommsPanel /> }.into_any(),
                                    RightTab::Combat => view! { <CombatLog /> }.into_any(),
                                }
                            }
                        }}
                    </div>
                </aside>
            </div>

            // Bottom bar - quick actions (hidden on mobile, desktop only)
            <footer class="hidden md:flex h-14 bg-slate-800 border-t border-slate-700 items-center px-4 gap-4">
                // Action buttons
                <button
                    class="px-4 py-2 bg-blue-900 hover:bg-blue-800 disabled:bg-slate-700 disabled:cursor-not-allowed rounded text-sm font-medium transition-colors"
                    on:click=handle_dock
                    disabled=move || is_docked(&ship_status()) || is_in_combat(&ship_status())
                >
                    {move || if is_docked(&ship_status()) { "Docked" } else { "Dock" }}
                </button>
                <button
                    class="px-4 py-2 bg-slate-700 hover:bg-slate-600 disabled:bg-slate-800 disabled:cursor-not-allowed rounded text-sm font-medium transition-colors"
                    on:click=handle_stop
                    disabled=move || !is_moving(&ship_status())
                >
                    "Stop"
                </button>
                <button
                    class="px-4 py-2 bg-red-900 hover:bg-red-800 disabled:bg-slate-700 disabled:cursor-not-allowed rounded text-sm font-medium transition-colors"
                    on:click=handle_alert
                    disabled=move || !is_in_combat(&ship_status())
                >
                    {move || if is_in_combat(&ship_status()) { "Disengage" } else { "Alert" }}
                </button>

                <div class="flex-1" />

                // Status indicators
                <div class="flex items-center gap-4 text-xs">
                    // Connection status
                    <div class="flex items-center gap-2">
                        <div class=move || {
                            let state = ws.state.get();
                            match state {
                                crate::api::ConnectionState::Connected => "w-2 h-2 rounded-full bg-green-500",
                                crate::api::ConnectionState::Connecting => "w-2 h-2 rounded-full bg-yellow-500 animate-pulse",
                                crate::api::ConnectionState::Reconnecting => "w-2 h-2 rounded-full bg-amber-500 animate-pulse",
                                crate::api::ConnectionState::Disconnected => "w-2 h-2 rounded-full bg-red-500",
                            }
                        } />
                        <span class=move || {
                            let state = ws.state.get();
                            match state {
                                crate::api::ConnectionState::Connected => "text-green-400",
                                crate::api::ConnectionState::Reconnecting => "text-amber-400",
                                _ => "text-slate-400",
                            }
                        }>
                            {move || {
                                let state = ws.state.get();
                                match state {
                                    crate::api::ConnectionState::Connected => "Connected".to_string(),
                                    crate::api::ConnectionState::Connecting => "Connecting...".to_string(),
                                    crate::api::ConnectionState::Reconnecting => {
                                        format!("Reconnecting ({}/10)...", ws.reconnect_attempt())
                                    },
                                    crate::api::ConnectionState::Disconnected => "Disconnected".to_string(),
                                }
                            }}
                        </span>
                    </div>

                    // Server tick
                    <span class="text-slate-500">
                        "Tick: "{server_tick}
                    </span>

                    // Debug panel toggle
                    <button
                        class="px-2 py-1 text-slate-500 hover:text-slate-300 hover:bg-slate-700 rounded transition-colors"
                        on:click=move |_| game_state.show_debug_panel.update(|v| *v = !*v)
                        title="Toggle Performance Debug Panel"
                    >
                        {move || if game_state.show_debug_panel.get() { "Debug ▼" } else { "Debug ▲" }}
                    </button>

                    // GM Editor toggle (admin only)
                    <Show when=move || game_state.is_admin.get()>
                        <button
                            class="px-2 py-1 text-amber-500 hover:text-amber-300 hover:bg-slate-700 rounded transition-colors font-medium"
                            on:click=move |_| {
                                let show = !game_state.show_gm_editor.get();
                                game_state.show_gm_editor.set(show);
                                if show {
                                    spawn_gm_editor();
                                }
                            }
                            title="Toggle GM Editor"
                        >
                            {move || if game_state.show_gm_editor.get() { "GM ▼" } else { "GM ▲" }}
                        </button>
                    </Show>
                </div>
            </footer>

            // Mobile bottom navigation (visible only on mobile)
            <MobileBottomNav />

            // Mission choice dialog
            <MissionChoiceDialog />

            // Performance debug panel
            <DebugPanel />

            // GM Editor container (dynamically loaded)
            <div id="gm-editor-container" class="hidden" />
        </div>
    }
}

/// Spawn the GM editor by dynamically loading the WASM module.
fn spawn_gm_editor() {
    use wasm_bindgen_futures::spawn_local;

    spawn_local(async move {
        // Get WS URL and auth token
        let ws_url = get_ws_url();
        let auth_token = get_auth_token().unwrap_or_default();

        // Dynamically import and mount the GM editor
        let result = load_gm_editor_module(&ws_url, &auth_token).await;
        if let Err(e) = result {
            tracing::error!("[gm-editor] Failed to load: {:?}", e);
        }
    });
}

/// Get the WebSocket URL for the GM editor.
fn get_ws_url() -> String {
    let window = web_sys::window().expect("window");
    let location = window.location();
    let protocol = location.protocol().unwrap_or_else(|_| "https:".to_string());
    let host = location.host().unwrap_or_else(|_| "localhost:3000".to_string());

    let ws_protocol = if protocol == "https:" { "wss:" } else { "ws:" };
    format!("{}//{}/ws", ws_protocol, host)
}

/// Dynamically load the GM editor WASM module.
async fn load_gm_editor_module(ws_url: &str, auth_token: &str) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::prelude::*;
    use js_sys::{Function, Promise, Reflect};
    use wasm_bindgen_futures::JsFuture;

    // Load both the CodeMirror editor bundle and the WASM module
    // Note: Files are served at /play/gm-editor/ since the app is at /play/
    let import_code = r#"
        (async function() {
            // Load CodeMirror editor bundle first (sets up window.initCodeMirror etc)
            await import('/play/gm-editor/editor.js');

            // Then load the WASM module
            const module = await import('/play/gm-editor/bw_gm_editor.js');
            await module.default();
            return module;
        })()
    "#;

    // Execute the dynamic imports
    let eval_fn = js_sys::eval(import_code)?;
    let promise = Promise::from(eval_fn);
    let module = JsFuture::from(promise).await?;

    // Call mount_gm_editor
    let mount_fn = Reflect::get(&module, &"mount_gm_editor".into())?;
    let mount_fn: Function = mount_fn.dyn_into()?;
    mount_fn.call3(
        &JsValue::NULL,
        &"gm-editor-container".into(),
        &ws_url.into(),
        &auth_token.into(),
    )?;

    tracing::info!("[gm-editor] Loaded successfully");
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LeftTab {
    Missions,
    Squadron,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RightTab {
    Ship,
    Comms,
    Combat,
}

#[component]
fn TabButton<A, C>(label: &'static str, active: A, on_click: C) -> impl IntoView
where
    A: Fn() -> bool + 'static + Clone + Send + Sync,
    C: Fn(web_sys::MouseEvent) + 'static + Clone + Send + Sync,
{
    view! {
        <button
            class=move || {
                let base = "flex-1 px-4 py-2 text-sm font-medium transition-colors";
                if active() {
                    format!("{} text-amber-400 border-b-2 border-amber-500", base)
                } else {
                    format!("{} text-slate-400 hover:text-slate-200", base)
                }
            }
            on:click=on_click
        >
            {label}
        </button>
    }
}

/// Mobile bottom navigation bar (iOS/Android style).
/// Only visible on screens smaller than md breakpoint (768px).
#[component]
fn MobileBottomNav() -> impl IntoView {
    let game_state = expect_context::<GameState>();

    let active_panel = move || game_state.active_mobile_panel.get();
    let in_combat = move || game_state.in_combat();
    let mission_count = move || game_state.available_missions.get().len();

    // State for "More" dropdown
    let show_more = RwSignal::new(false);

    view! {
        // Only show on mobile (hidden on md and up)
        <nav class="md:hidden fixed bottom-0 left-0 right-0 h-16 bg-slate-800 border-t border-slate-700 z-40 pb-safe">
            <div class="h-full flex items-center justify-around px-1">
                // Map
                <button
                    class=move || {
                        let base = "flex flex-col items-center justify-center w-14 h-full";
                        if active_panel() == MobilePanel::Map {
                            format!("{} text-amber-400", base)
                        } else {
                            format!("{} text-slate-400", base)
                        }
                    }
                    on:click=move |_| {
                        show_more.set(false);
                        game_state.active_mobile_panel.set(MobilePanel::Map);
                    }
                >
                    <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l5.447 2.724A1 1 0 0021 18.382V7.618a1 1 0 00-.553-.894L15 4m0 13V4m0 0L9 7" />
                    </svg>
                    <span class="text-[10px] mt-0.5">"Map"</span>
                </button>

                // Missions (with badge)
                <button
                    class=move || {
                        let base = "flex flex-col items-center justify-center w-14 h-full relative";
                        if active_panel() == MobilePanel::Missions {
                            format!("{} text-amber-400", base)
                        } else {
                            format!("{} text-slate-400", base)
                        }
                    }
                    on:click=move |_| {
                        show_more.set(false);
                        game_state.active_mobile_panel.set(MobilePanel::Missions);
                    }
                >
                    <div class="relative">
                        <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" />
                        </svg>
                        // Badge for mission count
                        <Show when=move || { mission_count() > 0 }>
                            <div class="absolute -top-1 -right-1 min-w-[16px] h-4 bg-amber-500 rounded-full text-[10px] text-slate-900 font-bold flex items-center justify-center px-1">
                                {move || mission_count()}
                            </div>
                        </Show>
                    </div>
                    <span class="text-[10px] mt-0.5">"Missions"</span>
                </button>

                // Ship
                <button
                    class=move || {
                        let base = "flex flex-col items-center justify-center w-14 h-full";
                        if active_panel() == MobilePanel::Ship {
                            format!("{} text-amber-400", base)
                        } else {
                            format!("{} text-slate-400", base)
                        }
                    }
                    on:click=move |_| {
                        show_more.set(false);
                        game_state.active_mobile_panel.set(MobilePanel::Ship);
                    }
                >
                    <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
                    </svg>
                    <span class="text-[10px] mt-0.5">"Ship"</span>
                </button>

                // Combat (with alert indicator when in combat)
                <button
                    class=move || {
                        let base = "flex flex-col items-center justify-center w-14 h-full relative";
                        if active_panel() == MobilePanel::Combat {
                            format!("{} text-amber-400", base)
                        } else if in_combat() {
                            format!("{} text-red-400", base)
                        } else {
                            format!("{} text-slate-400", base)
                        }
                    }
                    on:click=move |_| {
                        show_more.set(false);
                        game_state.active_mobile_panel.set(MobilePanel::Combat);
                    }
                >
                    <div class="relative">
                        <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                        </svg>
                        // Alert pulse when in combat
                        <Show when=in_combat>
                            <div class="absolute -top-1 -right-1 w-3 h-3 bg-red-500 rounded-full animate-pulse" />
                        </Show>
                    </div>
                    <span class="text-[10px] mt-0.5">"Combat"</span>
                </button>

                // More menu (Squadron, Comms)
                <div class="relative">
                    <button
                        class=move || {
                            let base = "flex flex-col items-center justify-center w-14 h-full";
                            let is_more_active = matches!(active_panel(), MobilePanel::Squadron | MobilePanel::Comms);
                            if is_more_active || show_more.get() {
                                format!("{} text-amber-400", base)
                            } else {
                                format!("{} text-slate-400", base)
                            }
                        }
                        on:click=move |_| show_more.update(|v| *v = !*v)
                    >
                        <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M5 12h.01M12 12h.01M19 12h.01M6 12a1 1 0 11-2 0 1 1 0 012 0zm7 0a1 1 0 11-2 0 1 1 0 012 0zm7 0a1 1 0 11-2 0 1 1 0 012 0z" />
                        </svg>
                        <span class="text-[10px] mt-0.5">"More"</span>
                    </button>

                    // Dropdown menu
                    <Show when=move || show_more.get()>
                        <div class="absolute bottom-full right-0 mb-2 w-36 bg-slate-800 border border-slate-600 rounded-lg shadow-lg overflow-hidden">
                            <button
                                class="w-full px-4 py-3 text-left text-sm text-slate-200 hover:bg-slate-700 flex items-center gap-2"
                                on:click=move |_| {
                                    show_more.set(false);
                                    game_state.active_mobile_panel.set(MobilePanel::Squadron);
                                }
                            >
                                <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2M9 11a4 4 0 100-8 4 4 0 000 8zM23 21v-2a4 4 0 00-3-3.87M16 3.13a4 4 0 010 7.75" />
                                </svg>
                                "Squadron"
                            </button>
                            <button
                                class="w-full px-4 py-3 text-left text-sm text-slate-200 hover:bg-slate-700 flex items-center gap-2"
                                on:click=move |_| {
                                    show_more.set(false);
                                    game_state.active_mobile_panel.set(MobilePanel::Comms);
                                }
                            >
                                <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <path d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                                </svg>
                                "Comms"
                            </button>
                        </div>
                    </Show>
                </div>
            </div>
        </nav>
    }
}

/// Condensed resource bar for mobile header.
/// Shows only critical resources (fuel, fame) in a compact format.
#[component]
fn MobileResourceBar() -> impl IntoView {
    let game_state = expect_context::<GameState>();

    let fuel = move || game_state.fuel.get();
    let fame = move || game_state.fame.get();

    view! {
        // Only visible on mobile
        <div class="flex md:hidden items-center gap-3 text-[10px] mr-2">
            // Fame
            <div class="flex items-center gap-1">
                <span class="text-amber-400 font-medium">{fame}</span>
                <span class="text-slate-500">"F"</span>
            </div>
            // Fuel (with low-fuel warning)
            <div class="flex items-center gap-1">
                <span class=move || {
                    if fuel() < 20.0 { "text-red-400 font-medium" } else { "text-blue-400" }
                }>
                    {move || format!("{:.0}", fuel())}
                </span>
                <span class="text-slate-500">"%"</span>
            </div>
        </div>
    }
}

/// Mission choice dialog overlay.
#[component]
fn MissionChoiceDialog() -> impl IntoView {
    let game_state = expect_context::<GameState>();
    let ws = expect_context::<WsService>();

    let show_dialog = move || game_state.show_mission_dialog.get();
    let mission_choice = move || game_state.mission_choice.get();

    view! {
        <Show when=show_dialog>
            <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-60">
                {move || {
                    if let Some(choice) = mission_choice() {
                        let mission_id = choice.mission_id;
                        let description = choice.description.clone();
                        let choices = choice.choices.clone();

                        view! {
                            <div class="bg-slate-800 rounded-lg border border-slate-600 shadow-xl max-w-lg w-full mx-4">
                                // Header
                                <div class="p-4 border-b border-slate-700">
                                    <h2 class="text-lg font-semibold text-amber-400">"Mission Choice"</h2>
                                </div>

                                // Description
                                <div class="p-4 text-slate-300">
                                    {description}
                                </div>

                                // Choices
                                <div class="p-4 pt-0 space-y-2">
                                    {choices.into_iter().map(|c| {
                                        let choice_id = c.id.clone();
                                        let text = c.text.clone();
                                        let is_available = c.is_available;
                                        let requirement_text = c.requirement_text.clone();

                                        let ws_clone = ws;
                                        let game_state_clone = game_state;
                                        let handle_choice = move |_| {
                                            if is_available {
                                                ws_clone.make_mission_choice(mission_id, choice_id.clone());
                                                game_state_clone.show_mission_dialog.set(false);
                                            }
                                        };

                                        view! {
                                            <button
                                                class=move || {
                                                    let base = "w-full text-left px-4 py-3 rounded-lg border transition-colors";
                                                    if is_available {
                                                        format!("{} bg-slate-700 border-slate-600 hover:border-amber-500 cursor-pointer", base)
                                                    } else {
                                                        format!("{} bg-slate-800 border-slate-700 opacity-50 cursor-not-allowed", base)
                                                    }
                                                }
                                                disabled=!is_available
                                                on:click=handle_choice
                                            >
                                                <div class="text-slate-200">{text}</div>
                                                {requirement_text.map(|req| view! {
                                                    <div class="text-xs text-red-400 mt-1">{req}</div>
                                                })}
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div /> }.into_any()
                    }
                }}
            </div>
        </Show>
    }
}

/// Get authentication token from localStorage.
fn get_auth_token() -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .and_then(|s| s.get_item("auth_token").ok())
        .flatten()
}

/// Get color class for ship status.
fn status_color(status: &str) -> &'static str {
    if status.starts_with("Idle") {
        "text-slate-500"
    } else if status.starts_with("InTransit") {
        "text-blue-400"
    } else if status.starts_with("Docked") {
        "text-green-400"
    } else if status.starts_with("InCombat") {
        "text-red-400"
    } else if status.starts_with("Disabled") {
        "text-red-600"
    } else if status.starts_with("Destroyed") {
        "text-red-800"
    } else {
        "text-slate-400"
    }
}

/// Get display-friendly status text.
fn status_display(status: &str) -> &'static str {
    if status.starts_with("Idle") {
        "Idle"
    } else if status.starts_with("InTransit") {
        "Moving"
    } else if status.starts_with("Docked") {
        "Docked"
    } else if status.starts_with("InCombat") {
        "In Combat"
    } else if status.starts_with("Disabled") {
        "Disabled"
    } else if status.starts_with("Destroyed") {
        "Destroyed"
    } else {
        "Unknown"
    }
}

/// Check if status indicates ship is in transit/moving.
fn is_moving(status: &str) -> bool {
    status.starts_with("InTransit")
}

/// Check if status indicates ship is docked.
fn is_docked(status: &str) -> bool {
    status.starts_with("Docked")
}

/// Check if status indicates ship is in combat.
fn is_in_combat(status: &str) -> bool {
    status.starts_with("InCombat")
}
