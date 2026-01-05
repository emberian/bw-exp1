//! Main game page
//!
//! Full-featured game interface with WebSocket connection and reactive state.

use leptos::prelude::*;

use crate::api::{ConnectionState, WsService};
use crate::components::game::*;
use crate::state::GameState;

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
        // Find nearest station and dock
        let locations = game_state.locations.get();
        if let Some(station) = locations.iter().find(|l| l.location_type == "station") {
            // Set docked station info before docking
            game_state.docked_station_id.set(Some(station.id));
            game_state.docked_station_name.set(station.name.clone());
            game_state.docked_station_services.set(station.services.clone());
            ws_dock.dock(station.id);
        } else {
            game_state.set_error("No station nearby to dock at".to_string());
        }
    };

    let handle_stop = move |_| {
        ws_stop.stop_movement();
    };

    let handle_alert = move |_| {
        // Toggle alert status / disengage combat
        ws_alert.disengage_combat();
    };

    view! {
        <div class="h-screen flex flex-col bg-slate-900 text-slate-100">
            // Top bar with resources
            <header class="h-16 bg-slate-800 border-b border-slate-700 flex items-center px-4">
                <div class="flex-1 flex items-center gap-4">
                    <span class="text-xl font-bold text-amber-500">"BLACKWING"</span>
                    <span class="text-sm text-slate-400">{sector_name}</span>
                    <span class=move || {
                        let status = ship_status();
                        let color = match status.as_str() {
                            "Idle" => "text-slate-500",
                            "Moving" => "text-blue-400",
                            "Docked" => "text-green-400",
                            "InCombat" => "text-red-400",
                            _ => "text-slate-400",
                        };
                        format!("text-xs px-2 py-0.5 rounded bg-slate-700 {}", color)
                    }>
                        {ship_status}
                    </span>
                </div>
                <ResourceBar />
                <button
                    class="ml-4 px-3 py-1.5 text-xs text-slate-400 hover:text-slate-200 hover:bg-slate-700 rounded transition-colors"
                    on:click=move |_| {
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
                >
                    "Logout"
                </button>
            </header>

            // Main content
            <div class="flex-1 flex overflow-hidden">
                // Left sidebar
                <aside class="w-80 bg-slate-800 border-r border-slate-700 flex flex-col">
                    // Tab buttons
                    <div class="flex border-b border-slate-700">
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
                    // Tab content
                    <div class="flex-1 overflow-y-auto">
                        {move || match left_tab.get() {
                            LeftTab::Missions => view! { <MissionPanel /> }.into_any(),
                            LeftTab::Squadron => view! { <SquadronPanel /> }.into_any(),
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

                    // Notification overlay
                    <Show when=move || notification().is_some()>
                        <div class="absolute top-4 left-1/2 -translate-x-1/2 z-50">
                            <div class="bg-amber-900/90 border border-amber-500 rounded-lg px-4 py-2 shadow-lg flex items-center gap-3">
                                <span class="text-amber-200">{move || notification().unwrap_or_default()}</span>
                                <button
                                    class="text-amber-400 hover:text-amber-200"
                                    on:click=move |_| game_state.notification.set(None)
                                >
                                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M18 6L6 18M6 6l12 12" />
                                    </svg>
                                </button>
                            </div>
                        </div>
                    </Show>

                    // Error overlay
                    <Show when=move || last_error().is_some()>
                        <div class="absolute top-16 left-1/2 -translate-x-1/2 z-50">
                            <div class="bg-red-900/90 border border-red-500 rounded-lg px-4 py-2 shadow-lg flex items-center gap-3">
                                <span class="text-red-200">{move || last_error().unwrap_or_default()}</span>
                                <button
                                    class="text-red-400 hover:text-red-200"
                                    on:click=move |_| game_state.clear_error()
                                >
                                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M18 6L6 18M6 6l12 12" />
                                    </svg>
                                </button>
                            </div>
                        </div>
                    </Show>
                </main>

                // Right sidebar - ship status, comms & combat
                <aside class="w-80 bg-slate-800 border-l border-slate-700 flex flex-col">
                    // Tab buttons
                    <div class="flex border-b border-slate-700">
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
                    // Tab content
                    <div class="flex-1 overflow-hidden">
                        {move || match right_tab.get() {
                            RightTab::Ship => view! { <ShipStatus /> }.into_any(),
                            RightTab::Comms => view! { <CommsPanel /> }.into_any(),
                            RightTab::Combat => view! { <CombatLog /> }.into_any(),
                        }}
                    </div>
                </aside>
            </div>

            // Bottom bar - quick actions
            <footer class="h-14 bg-slate-800 border-t border-slate-700 flex items-center px-4 gap-4">
                // Action buttons
                <button
                    class="px-4 py-2 bg-blue-900 hover:bg-blue-800 disabled:bg-slate-700 disabled:cursor-not-allowed rounded text-sm font-medium transition-colors"
                    on:click=handle_dock
                    disabled=move || ship_status() == "Docked" || ship_status() == "InCombat"
                >
                    {move || if ship_status() == "Docked" { "Docked" } else { "Dock" }}
                </button>
                <button
                    class="px-4 py-2 bg-slate-700 hover:bg-slate-600 disabled:bg-slate-800 disabled:cursor-not-allowed rounded text-sm font-medium transition-colors"
                    on:click=handle_stop
                    disabled=move || ship_status() != "Moving"
                >
                    "Stop"
                </button>
                <button
                    class="px-4 py-2 bg-red-900 hover:bg-red-800 disabled:bg-slate-700 disabled:cursor-not-allowed rounded text-sm font-medium transition-colors"
                    on:click=handle_alert
                    disabled=move || ship_status() != "InCombat"
                >
                    {move || if ship_status() == "InCombat" { "Disengage" } else { "Alert" }}
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
                </div>
            </footer>

            // Mission choice dialog
            <MissionChoiceDialog />
        </div>
    }
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

/// Mission choice dialog overlay.
#[component]
fn MissionChoiceDialog() -> impl IntoView {
    let game_state = expect_context::<GameState>();
    let ws = expect_context::<WsService>();

    let show_dialog = move || game_state.show_mission_dialog.get();
    let mission_choice = move || game_state.mission_choice.get();

    view! {
        <Show when=show_dialog>
            <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
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
