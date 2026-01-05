//! Sector map component
//!
//! Displays the sector with locations, ships, and allows movement.

use leptos::prelude::*;
use uuid::Uuid;

use crate::api::WsService;
use crate::state::{GameState, LocationInfo, ShipInfo};

use super::CombatIndicator;

/// Sector map dimensions in game units.
const SECTOR_SIZE: f64 = 1000.0;

#[component]
pub fn SectorMap() -> impl IntoView {
    let game_state = expect_context::<GameState>();
    let ws = expect_context::<WsService>();

    // Reactive state
    let locations = move || game_state.locations.get();
    let ships = move || game_state.ships.get();
    let player_x = move || game_state.position_x.get();
    let player_y = move || game_state.position_y.get();
    let selected_target = move || game_state.selected_target.get();
    let sector_name = move || game_state.sector_name.get();
    let sector_danger = move || game_state.sector_danger.get();

    // Click handler for map movement
    let on_click = move |ev: web_sys::MouseEvent| {
        // Use offset coordinates directly (relative to target element)
        let click_x = ev.offset_x() as f64;
        let click_y = ev.offset_y() as f64;

        // Assume standard map size for coordinate conversion
        // The actual element size is determined by CSS flex
        let map_width = 800.0; // Approximate width
        let map_height = 600.0; // Approximate height

        // Convert to sector coordinates (0-1000 range)
        let sector_x = (click_x / map_width) * SECTOR_SIZE;
        let sector_y = (click_y / map_height) * SECTOR_SIZE;

        ws.move_to_position(sector_x, sector_y);
    };

    // Target selection handler
    let select_target = move |target_id: Option<Uuid>| {
        game_state.selected_target.set(target_id);
    };

    view! {
        <div
            class="w-full h-full bg-slate-950 relative overflow-hidden"
            on:click=on_click
        >
            // Star background
            <div class="absolute inset-0 stars-pattern opacity-30" />

            // Grid overlay
            <GridOverlay />

            // Combat indicator
            <CombatIndicator />

            // Render locations
            <For
                each=locations
                key=|l| l.id
                children=move |location| {
                    let ws = ws;
                    view! {
                        <LocationMarker
                            location=location.clone()
                            on_click=move |id| {
                                ws.move_to_location(id);
                            }
                        />
                    }
                }
            />

            // Render other ships
            <For
                each=ships
                key=|s| s.id
                children=move |ship| {
                    let ship_id = ship.id;
                    let is_selected = move || selected_target() == Some(ship_id);
                    view! {
                        <ShipMarker
                            ship=ship.clone()
                            is_selected=is_selected
                            on_select=move |id| {
                                select_target(Some(id));
                            }
                        />
                    }
                }
            />

            // Player ship
            <PlayerShipMarker
                x=player_x
                y=player_y
            />

            // Legend
            <MapLegend />

            // Sector info display
            <div class="absolute top-4 right-4 bg-slate-900/80 rounded px-3 py-2 text-xs">
                <div class="text-slate-300 font-semibold">{sector_name}</div>
                <div class="text-slate-400">
                    "Danger: "
                    <span class=move || danger_color(&sector_danger())>{sector_danger}</span>
                </div>
            </div>

            // Selected target info
            <Show when=move || selected_target().is_some()>
                {move || {
                    if let Some(target_id) = selected_target() {
                        let ships_list = ships();
                        if let Some(ship) = ships_list.iter().find(|s| s.id == target_id) {
                            let ws = ws;
                            let target = target_id;
                            view! {
                                <TargetInfo
                                    ship=ship.clone()
                                    on_engage=move || ws.engage_target(target)
                                    on_deselect=move || select_target(None)
                                />
                            }.into_any()
                        } else {
                            view! { <div /> }.into_any()
                        }
                    } else {
                        view! { <div /> }.into_any()
                    }
                }}
            </Show>
        </div>
    }
}

#[component]
fn GridOverlay() -> impl IntoView {
    view! {
        <svg class="absolute inset-0 w-full h-full" style="pointer-events: none;">
            // Horizontal grid lines
            <line x1="0" y1="25%" x2="100%" y2="25%" stroke="#334155" stroke-width="1" />
            <line x1="0" y1="50%" x2="100%" y2="50%" stroke="#334155" stroke-width="1" />
            <line x1="0" y1="75%" x2="100%" y2="75%" stroke="#334155" stroke-width="1" />

            // Vertical grid lines
            <line x1="25%" y1="0" x2="25%" y2="100%" stroke="#334155" stroke-width="1" />
            <line x1="50%" y1="0" x2="50%" y2="100%" stroke="#334155" stroke-width="1" />
            <line x1="75%" y1="0" x2="75%" y2="100%" stroke="#334155" stroke-width="1" />
        </svg>
    }
}

#[component]
fn LocationMarker<F>(location: LocationInfo, on_click: F) -> impl IntoView
where
    F: Fn(Uuid) + 'static + Clone,
{
    let id = location.id;
    let name = location.name.clone();
    let location_type = location.location_type.clone();
    let x_percent = (location.x / SECTOR_SIZE) * 100.0;
    let y_percent = (location.y / SECTOR_SIZE) * 100.0;

    let on_click = on_click.clone();
    let handle_click = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        on_click(id);
    };

    let (color, size) = match location_type.as_str() {
        "station" => ("bg-blue-500", "w-4 h-4"),
        "mining" => ("bg-amber-500", "w-3 h-3"),
        "jumpgate" => ("bg-purple-500", "w-4 h-4"),
        "debris" | "wreckage" => ("bg-slate-500", "w-3 h-3"),
        "anomaly" => ("bg-cyan-500", "w-3 h-3"),
        _ => ("bg-slate-500", "w-3 h-3"),
    };

    view! {
        <div
            class="absolute transform -translate-x-1/2 -translate-y-1/2 cursor-pointer group z-10"
            style=format!("left: {}%; top: {}%", x_percent, y_percent)
            on:click=handle_click
        >
            <div class=format!("{} {} rounded-full animate-pulse", size, color) />
            <div class="absolute left-1/2 -translate-x-1/2 top-full mt-1 whitespace-nowrap
                        text-xs text-slate-300 opacity-0 group-hover:opacity-100 transition-opacity
                        bg-slate-900/90 px-2 py-1 rounded pointer-events-none z-20">
                {name}
            </div>
        </div>
    }
}

#[component]
fn ShipMarker<F, S>(ship: ShipInfo, is_selected: S, on_select: F) -> impl IntoView
where
    F: Fn(Uuid) + 'static + Clone + Send,
    S: Fn() -> bool + 'static + Clone + Send,
{
    let id = ship.id;
    let name = ship.name.clone();
    let x_percent = (ship.x / SECTOR_SIZE) * 100.0;
    let y_percent = (ship.y / SECTOR_SIZE) * 100.0;
    let is_hostile = ship.is_hostile;
    let is_player = ship.is_player;
    let hull_percent = ship.hull_percent;

    let on_select = on_select.clone();
    let handle_click = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        if !is_player {
            on_select(id);
        }
    };

    let color = if is_player {
        "text-green-400"
    } else if is_hostile {
        "text-red-400"
    } else {
        "text-slate-400"
    };

    let is_selected = is_selected.clone();

    view! {
        <div
            class=move || {
                let base = format!(
                    "absolute transform -translate-x-1/2 -translate-y-1/2 cursor-pointer group z-20 {}",
                    color
                );
                if is_selected() {
                    format!("{} ring-2 ring-amber-400 rounded-full", base)
                } else {
                    base
                }
            }
            style=format!("left: {}%; top: {}%", x_percent, y_percent)
            on:click=handle_click
        >
            // Ship icon (triangle pointing up)
            <svg width="16" height="16" viewBox="0 0 16 16" class="fill-current">
                <polygon points="8,0 16,16 8,12 0,16" />
            </svg>

            // Hull damage indicator (red overlay for low hull)
            <Show when=move || hull_percent < 50.0>
                <div class="absolute inset-0 bg-red-500/30 rounded animate-pulse" />
            </Show>

            // Name tooltip
            <div class="absolute left-1/2 -translate-x-1/2 bottom-full mb-1 whitespace-nowrap
                        text-xs opacity-0 group-hover:opacity-100 transition-opacity
                        bg-slate-900/90 px-2 py-1 rounded pointer-events-none z-30">
                <div class={format!("font-semibold {}", color)}>{name}</div>
                <div class="text-slate-400 text-[10px]">{format!("Hull: {:.0}%", hull_percent)}</div>
            </div>
        </div>
    }
}

#[component]
fn PlayerShipMarker<X, Y>(x: X, y: Y) -> impl IntoView
where
    X: Fn() -> f64 + 'static + Clone + Send,
    Y: Fn() -> f64 + 'static + Clone + Send,
{
    let x = x.clone();
    let y = y.clone();

    view! {
        <div
            class="absolute transform -translate-x-1/2 -translate-y-1/2 text-green-400 z-30"
            style=move || {
                let x_percent = (x() / SECTOR_SIZE) * 100.0;
                let y_percent = (y() / SECTOR_SIZE) * 100.0;
                format!("left: {}%; top: {}%; transition: left 0.1s, top 0.1s", x_percent, y_percent)
            }
        >
            // Player ship icon (larger, different shape)
            <svg width="20" height="20" viewBox="0 0 20 20" class="fill-current">
                <polygon points="10,0 20,20 10,15 0,20" />
            </svg>

            // Selection ring
            <div class="absolute inset-[-4px] border-2 border-green-400/50 rounded-full" />

            // Label
            <div class="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 whitespace-nowrap
                        text-xs text-green-400 font-semibold bg-slate-900/90 px-2 py-1 rounded">
                "Your Ship"
            </div>
        </div>
    }
}

#[component]
fn MapLegend() -> impl IntoView {
    view! {
        <div class="absolute bottom-4 left-4 bg-slate-900/80 rounded p-3 text-xs pointer-events-none">
            <div class="font-semibold text-slate-300 mb-2">"Legend"</div>
            <div class="flex items-center gap-2 mb-1">
                <div class="w-3 h-3 rounded-full bg-blue-500" />
                <span class="text-slate-400">"Station"</span>
            </div>
            <div class="flex items-center gap-2 mb-1">
                <div class="w-3 h-3 rounded-full bg-amber-500" />
                <span class="text-slate-400">"Mining"</span>
            </div>
            <div class="flex items-center gap-2 mb-1">
                <div class="w-3 h-3 rounded-full bg-purple-500" />
                <span class="text-slate-400">"Jumpgate"</span>
            </div>
            <div class="flex items-center gap-2 mb-1">
                <div class="w-3 h-3 rounded-full bg-slate-500" />
                <span class="text-slate-400">"Debris"</span>
            </div>
            <div class="flex items-center gap-2 mb-1">
                <svg width="12" height="12" viewBox="0 0 16 16" class="fill-green-400">
                    <polygon points="8,0 16,16 8,12 0,16" />
                </svg>
                <span class="text-slate-400">"Player"</span>
            </div>
            <div class="flex items-center gap-2">
                <svg width="12" height="12" viewBox="0 0 16 16" class="fill-red-400">
                    <polygon points="8,0 16,16 8,12 0,16" />
                </svg>
                <span class="text-slate-400">"Hostile"</span>
            </div>
        </div>
    }
}

#[component]
fn TargetInfo<E, D>(ship: ShipInfo, on_engage: E, on_deselect: D) -> impl IntoView
where
    E: Fn() + 'static + Clone + Send,
    D: Fn() + 'static + Clone + Send,
{
    let name = ship.name.clone();
    let ship_class = ship.ship_class.clone();
    let hull_percent = ship.hull_percent;
    let is_hostile = ship.is_hostile;
    let status = ship.status.clone();

    let on_engage_clone = on_engage.clone();
    let on_deselect_clone = on_deselect.clone();

    view! {
        <div class="absolute bottom-4 right-4 bg-slate-900/90 rounded-lg p-3 w-64 border border-slate-700">
            <div class="flex justify-between items-start mb-2">
                <div>
                    <div class="font-semibold text-slate-200">{name}</div>
                    <div class="text-xs text-slate-400">{ship_class}</div>
                </div>
                <button
                    class="text-slate-400 hover:text-slate-200"
                    on:click={
                        let on_deselect = on_deselect_clone.clone();
                        move |_| on_deselect()
                    }
                >
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M18 6L6 18M6 6l12 12" />
                    </svg>
                </button>
            </div>

            <div class="space-y-2 mb-3">
                <div class="flex justify-between text-xs">
                    <span class="text-slate-400">"Hull"</span>
                    <span class=if hull_percent < 30.0 { "text-red-400" } else { "text-slate-300" }>
                        {format!("{:.0}%", hull_percent)}
                    </span>
                </div>
                <div class="h-1 bg-slate-700 rounded-full overflow-hidden">
                    <div
                        class="h-full bg-green-500"
                        style=format!("width: {}%", hull_percent)
                    />
                </div>

                <div class="flex justify-between text-xs">
                    <span class="text-slate-400">"Status"</span>
                    <span class="text-slate-300">{status}</span>
                </div>
            </div>

            <div class="flex gap-2">
                {if is_hostile {
                    let on_engage = on_engage_clone.clone();
                    view! {
                        <button
                            class="flex-1 px-3 py-1 bg-red-900 hover:bg-red-800 rounded text-xs text-red-100"
                            on:click=move |_| on_engage()
                        >
                            "Engage"
                        </button>
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }}
                <button class="flex-1 px-3 py-1 bg-slate-700 hover:bg-slate-600 rounded text-xs">
                    "Hail"
                </button>
            </div>
        </div>
    }
}

/// Get danger level color class.
fn danger_color(danger: &str) -> &'static str {
    match danger.to_lowercase().as_str() {
        "safe" | "low" => "text-green-400",
        "moderate" | "medium" => "text-yellow-400",
        "high" | "dangerous" => "text-orange-400",
        "extreme" | "critical" => "text-red-400",
        _ => "text-slate-400",
    }
}
