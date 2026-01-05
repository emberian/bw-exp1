//! Sector map component

use leptos::prelude::*;
use crate::state::GameState;

#[component]
pub fn SectorMap() -> impl IntoView {
    let game_state = expect_context::<GameState>();

    // Click handler for map
    let on_click = move |ev: web_sys::MouseEvent| {
        let x = ev.offset_x() as f64;
        let y = ev.offset_y() as f64;

        // Convert to sector coordinates (centered, scaled)
        // Map is 1000x1000 sector units, displayed in whatever size the container is
        // TODO: Proper coordinate conversion
        let sector_x = (x - 400.0) * 1.25;
        let sector_y = (y - 300.0) * 1.25;

        // TODO: Send movement command via WebSocket
        web_sys::console::log_1(&format!("Move to: ({}, {})", sector_x, sector_y).into());
    };

    view! {
        <div class="w-full h-full bg-slate-950 relative overflow-hidden" on:click=on_click>
            // Star background
            <div class="absolute inset-0 stars-pattern opacity-30" />

            // Grid lines
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

            // Locations (static for now)
            <LocationMarker
                name="Thornwick Station"
                x=50.0
                y=50.0
                location_type="station"
            />
            <LocationMarker
                name="Dustfall Mining"
                x=70.0
                y=35.0
                location_type="mining"
            />
            <LocationMarker
                name="The Wreckage"
                x=25.0
                y=60.0
                location_type="debris"
            />
            <LocationMarker
                name="Relay Alpha"
                x=85.0
                y=50.0
                location_type="jumpgate"
            />

            // Player ship
            <ShipMarker
                name="Your Ship"
                x=50.0
                y=50.0
                is_player=true
            />

            // Legend
            <div class="absolute bottom-4 left-4 bg-slate-900/80 rounded p-3 text-xs">
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
                <div class="flex items-center gap-2">
                    <div class="w-3 h-3 rounded-full bg-slate-500" />
                    <span class="text-slate-400">"Debris"</span>
                </div>
            </div>

            // Coordinates display
            <div class="absolute top-4 right-4 bg-slate-900/80 rounded px-3 py-2 text-xs text-slate-400">
                "Sector: Thornwick | Danger: Moderate"
            </div>
        </div>
    }
}

#[component]
fn LocationMarker(
    name: &'static str,
    x: f64,  // Percentage
    y: f64,  // Percentage
    location_type: &'static str,
) -> impl IntoView {
    let color = match location_type {
        "station" => "bg-blue-500",
        "mining" => "bg-amber-500",
        "jumpgate" => "bg-purple-500",
        "debris" => "bg-slate-500",
        _ => "bg-slate-500",
    };

    let size = match location_type {
        "station" => "w-4 h-4",
        "jumpgate" => "w-4 h-4",
        _ => "w-3 h-3",
    };

    view! {
        <div
            class="absolute transform -translate-x-1/2 -translate-y-1/2 cursor-pointer group"
            style=format!("left: {}%; top: {}%", x, y)
        >
            <div class=format!("{} {} rounded-full animate-pulse", size, color) />
            <div class="absolute left-1/2 -translate-x-1/2 top-full mt-1 whitespace-nowrap
                        text-xs text-slate-300 opacity-0 group-hover:opacity-100 transition-opacity
                        bg-slate-900/90 px-2 py-1 rounded">
                {name}
            </div>
        </div>
    }
}

#[component]
fn ShipMarker(
    name: &'static str,
    x: f64,
    y: f64,
    is_player: bool,
) -> impl IntoView {
    let color = if is_player { "text-green-400" } else { "text-red-400" };

    view! {
        <div
            class=format!("absolute transform -translate-x-1/2 -translate-y-1/2 cursor-pointer group {}", color)
            style=format!("left: {}%; top: {}%", x, y)
        >
            // Ship icon (simple triangle)
            <svg width="16" height="16" viewBox="0 0 16 16" class="fill-current">
                <polygon points="8,0 16,16 8,12 0,16" />
            </svg>
            <div class="absolute left-1/2 -translate-x-1/2 bottom-full mb-1 whitespace-nowrap
                        text-xs opacity-0 group-hover:opacity-100 transition-opacity
                        bg-slate-900/90 px-2 py-1 rounded">
                {name}
            </div>
        </div>
    }
}
