//! Ship status component

use leptos::prelude::*;
use crate::state::GameState;

#[component]
pub fn ShipStatus() -> impl IntoView {
    let game_state = expect_context::<GameState>();

    // Derive system statuses from ship state
    let engines_status = move || {
        if game_state.fuel.get() <= 0.0 {
            "offline"
        } else if game_state.fuel.get() < 10.0 {
            "damaged"
        } else {
            "online"
        }
    };

    let weapons_status = move || {
        if game_state.ammunition.get() <= 0.0 {
            "offline"
        } else if game_state.ammunition.get() < 15.0 {
            "damaged"
        } else {
            "online"
        }
    };

    let sensors_status = move || {
        if game_state.ship_hull.get() < 25.0 {
            "damaged"
        } else {
            "online"
        }
    };

    let comms_status = move || "online";

    view! {
        <div data-testid="ship-status" class="p-4">
            <h2 class="text-lg font-semibold text-amber-500 mb-4">"Ship Status"</h2>

            // Ship diagram - visual representation with damage overlay
            <div class="mb-4 relative">
                <ShipDiagram
                    hull_percent=game_state.ship_hull
                    shield_percent=game_state.ship_shields
                />
            </div>

            // Ship info with status
            <div class="mb-6">
                <div class="flex justify-between items-center">
                    <div class="text-slate-200 font-medium">{move || game_state.ship_name.get()}</div>
                    <span class=move || {
                        let status = game_state.ship_status.get();
                        let color = status_color(&status);
                        format!("text-xs px-2 py-0.5 rounded bg-slate-700 {}", color)
                    }>
                        {move || status_display(&game_state.ship_status.get())}
                    </span>
                </div>
                <div class="text-sm text-slate-400">"Class: "{move || game_state.ship_class.get()}</div>
            </div>

            // Hull & Shields - reactive
            <div class="space-y-3 mb-6">
                <ReactiveStatusBarDynamic
                    label="Hull"
                    value=game_state.ship_hull
                    max=game_state.ship_max_hull
                    critical_threshold=25.0
                />
                <ReactiveStatusBarDynamic
                    label="Shields"
                    value=game_state.ship_shields
                    max=game_state.ship_max_shields
                    critical_threshold=10.0
                />
            </div>

            // Systems - reactive based on resources
            <h3 class="text-sm font-semibold text-slate-300 mb-2">"Systems"</h3>
            <div class="grid grid-cols-2 gap-2 text-xs">
                <DynamicSystemStatus name="Engines" status=engines_status />
                <DynamicSystemStatus name="Weapons" status=weapons_status />
                <DynamicSystemStatus name="Sensors" status=sensors_status />
                <DynamicSystemStatus name="Comms" status=comms_status />
            </div>

            // Position display
            <h3 class="text-sm font-semibold text-slate-300 mt-6 mb-2">"Position"</h3>
            <div data-testid="ship-position" class="text-xs text-slate-400 bg-slate-700/50 rounded px-2 py-1">
                <span>"X: "</span>
                <span class="text-slate-300">{move || format!("{:.0}", game_state.position_x.get())}</span>
                <span class="mx-2">"|"</span>
                <span>"Y: "</span>
                <span class="text-slate-300">{move || format!("{:.0}", game_state.position_y.get())}</span>
            </div>

            // Weapons
            <h3 class="text-sm font-semibold text-slate-300 mt-6 mb-2">"Weapons"</h3>
            <div class="space-y-2 text-xs">
                <WeaponStatus name="Railgun" ammo_available=game_state.ammunition damage="20" />
                <WeaponStatus name="Point Defense" ammo_available=game_state.ammunition damage="5" />
            </div>
        </div>
    }
}

/// Reactive status bar with dynamic max value from signal.
#[component]
fn ReactiveStatusBarDynamic(
    label: &'static str,
    value: RwSignal<f32>,
    max: RwSignal<f32>,
    critical_threshold: f32,
) -> impl IntoView {
    let bar_color = move || {
        let v = value.get();
        if v <= critical_threshold {
            "bg-red-500"
        } else if v <= critical_threshold * 2.0 {
            "bg-amber-500"
        } else {
            if label == "Shields" { "bg-blue-500" } else { "bg-green-500" }
        }
    };

    let text_color = move || {
        if value.get() <= critical_threshold {
            "text-red-400"
        } else {
            "text-slate-300"
        }
    };

    view! {
        <div>
            <div class="flex justify-between text-xs mb-1">
                <span class="text-slate-400">{label}</span>
                <span class=text_color>
                    {move || format!("{:.0}/{:.0}", value.get(), max.get())}
                </span>
            </div>
            <div class="h-2 bg-slate-700 rounded overflow-hidden">
                <div
                    class=move || format!("h-full transition-all duration-300 {}", bar_color())
                    style=move || {
                        let max_val = max.get();
                        if max_val > 0.0 {
                            format!("width: {}%", (value.get() / max_val * 100.0).min(100.0))
                        } else {
                            "width: 0%".to_string()
                        }
                    }
                />
            </div>
        </div>
    }
}

/// Dynamic system status that updates from a closure.
#[component]
fn DynamicSystemStatus<F>(name: &'static str, status: F) -> impl IntoView
where
    F: Fn() -> &'static str + Send + Sync + Clone + 'static,
{
    let status_for_class = status.clone();
    let status_for_text = status;

    view! {
        <div class="flex items-center gap-2 bg-slate-700/50 rounded px-2 py-1.5">
            // System icon placeholder
            <SystemIcon system=name />
            <span class="text-slate-300 flex-1">{name}</span>
            <span class=move || {
                match status_for_class() {
                    "online" => "text-green-400 text-xs font-medium",
                    "damaged" => "text-amber-400 text-xs font-medium",
                    "offline" => "text-red-400 text-xs font-medium",
                    _ => "text-slate-400 text-xs font-medium",
                }
            }>
                {move || match status_for_text() {
                    "online" => "ONLINE",
                    "damaged" => "DAMAGED",
                    "offline" => "OFFLINE",
                    _ => "UNKNOWN",
                }}
            </span>
        </div>
    }
}

/// Weapon status with reactive ammo indicator.
#[component]
fn WeaponStatus(
    name: &'static str,
    ammo_available: RwSignal<f32>,
    damage: &'static str,
) -> impl IntoView {
    let ammo_status = move || {
        let ammo = ammo_available.get();
        if ammo <= 0.0 {
            ("text-red-400", "EMPTY")
        } else if ammo < 15.0 {
            ("text-amber-400", "LOW")
        } else {
            ("text-green-400", "Ready")
        }
    };

    view! {
        <div class="flex items-center gap-2 bg-slate-700/50 rounded px-2 py-1.5">
            // Weapon icon placeholder
            <WeaponIcon weapon=name />
            <span class="text-slate-300 flex-1">{name}</span>
            <div class="flex gap-3 text-xs">
                <span class=move || format!("{} font-medium", ammo_status().0)>{move || ammo_status().1}</span>
                <span class="text-amber-400">{damage}" dmg"</span>
            </div>
        </div>
    }
}

/// Get color class for ship status.
fn status_color(status: &str) -> &'static str {
    if status.starts_with("Idle") {
        "text-green-400"
    } else if status.starts_with("InTransit") {
        "text-blue-400"
    } else if status.starts_with("Docked") {
        "text-amber-400"
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

// =============================================================================
// Asset Placeholder Components
// =============================================================================
// These components render placeholder graphics that can be replaced with
// actual artist assets. Each has a clear visual indicator showing where
// the real asset should go.

/// Ship diagram placeholder - shows ship silhouette with damage/shield overlays.
/// Replace the SVG content with actual ship artwork.
#[component]
fn ShipDiagram(hull_percent: RwSignal<f32>, shield_percent: RwSignal<f32>) -> impl IntoView {
    // Damage overlay opacity based on hull damage
    let damage_opacity = move || {
        let hull = hull_percent.get();
        if hull >= 75.0 {
            0.0
        } else if hull >= 50.0 {
            0.2
        } else if hull >= 25.0 {
            0.4
        } else {
            0.6
        }
    };

    // Shield glow visibility
    let shield_visible = move || shield_percent.get() > 0.0;
    let shield_opacity = move || (shield_percent.get() / 100.0).min(1.0) * 0.5;

    view! {
        <div class="relative w-full h-32 bg-slate-800/50 rounded-lg border border-slate-700 overflow-hidden">
            // Placeholder ship silhouette
            // TODO: Replace with actual ship asset based on ship_class
            <svg class="absolute inset-0 w-full h-full" viewBox="0 0 100 60" preserveAspectRatio="xMidYMid meet">
                // Ship body placeholder (stylized top-down view)
                <polygon
                    points="50,5 65,20 65,45 55,55 45,55 35,45 35,20"
                    fill="#475569"
                    stroke="#64748b"
                    stroke-width="1"
                />
                // Engine glow
                <ellipse cx="45" cy="55" rx="4" ry="2" fill="#f59e0b" opacity="0.8" />
                <ellipse cx="55" cy="55" rx="4" ry="2" fill="#f59e0b" opacity="0.8" />
                // Cockpit
                <ellipse cx="50" cy="15" rx="6" ry="4" fill="#1e293b" stroke="#64748b" stroke-width="0.5" />
                // Wing details
                <line x1="35" y1="25" x2="20" y2="35" stroke="#64748b" stroke-width="2" />
                <line x1="65" y1="25" x2="80" y2="35" stroke="#64748b" stroke-width="2" />
            </svg>

            // Shield effect overlay
            <Show when=shield_visible>
                <div
                    class="absolute inset-2 rounded-full border-2 border-blue-400 transition-opacity duration-300"
                    style=move || format!("opacity: {}; box-shadow: 0 0 10px rgba(96, 165, 250, 0.5), inset 0 0 10px rgba(96, 165, 250, 0.3)", shield_opacity())
                />
            </Show>

            // Damage overlay (red tint increases with damage)
            <div
                class="absolute inset-0 bg-red-500 pointer-events-none transition-opacity duration-500"
                style=move || format!("opacity: {}", damage_opacity())
            />

            // Placeholder label
            <div class="absolute bottom-1 right-1 text-[8px] text-slate-600 bg-slate-900/50 px-1 rounded">
                "SHIP_DIAGRAM"
            </div>
        </div>
    }
}

/// System icon placeholder - displays icon for ship systems.
/// Replace SVG content with actual icons.
#[component]
fn SystemIcon(system: &'static str) -> impl IntoView {
    // Different placeholder icons per system type
    let icon_content = match system {
        "Engines" => view! {
            <svg width="16" height="16" viewBox="0 0 16 16" class="text-slate-400">
                // Engine/thruster icon placeholder
                <rect x="3" y="4" width="10" height="8" rx="1" fill="none" stroke="currentColor" stroke-width="1.5" />
                <line x1="6" y1="12" x2="6" y2="14" stroke="currentColor" stroke-width="1.5" />
                <line x1="10" y1="12" x2="10" y2="14" stroke="currentColor" stroke-width="1.5" />
                <circle cx="8" cy="8" r="2" fill="currentColor" opacity="0.5" />
            </svg>
        }.into_any(),
        "Weapons" => view! {
            <svg width="16" height="16" viewBox="0 0 16 16" class="text-slate-400">
                // Crosshair/targeting icon placeholder
                <circle cx="8" cy="8" r="5" fill="none" stroke="currentColor" stroke-width="1.5" />
                <line x1="8" y1="2" x2="8" y2="5" stroke="currentColor" stroke-width="1.5" />
                <line x1="8" y1="11" x2="8" y2="14" stroke="currentColor" stroke-width="1.5" />
                <line x1="2" y1="8" x2="5" y2="8" stroke="currentColor" stroke-width="1.5" />
                <line x1="11" y1="8" x2="14" y2="8" stroke="currentColor" stroke-width="1.5" />
            </svg>
        }.into_any(),
        "Sensors" => view! {
            <svg width="16" height="16" viewBox="0 0 16 16" class="text-slate-400">
                // Radar/sensor icon placeholder
                <path d="M8 2 L8 8 L12 12" fill="none" stroke="currentColor" stroke-width="1.5" />
                <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="1.5" />
                <circle cx="8" cy="8" r="3" fill="none" stroke="currentColor" stroke-width="1" opacity="0.5" />
            </svg>
        }.into_any(),
        "Comms" => view! {
            <svg width="16" height="16" viewBox="0 0 16 16" class="text-slate-400">
                // Antenna/comms icon placeholder
                <path d="M8 14 L8 6" stroke="currentColor" stroke-width="1.5" />
                <path d="M4 4 Q8 0 12 4" fill="none" stroke="currentColor" stroke-width="1.5" />
                <path d="M5 6 Q8 3 11 6" fill="none" stroke="currentColor" stroke-width="1.5" />
                <circle cx="8" cy="6" r="1.5" fill="currentColor" />
            </svg>
        }.into_any(),
        _ => view! {
            <svg width="16" height="16" viewBox="0 0 16 16" class="text-slate-400">
                <rect x="2" y="2" width="12" height="12" rx="2" fill="none" stroke="currentColor" stroke-width="1.5" />
                <text x="8" y="11" text-anchor="middle" font-size="8" fill="currentColor">"?"</text>
            </svg>
        }.into_any(),
    };

    view! {
        <div class="w-4 h-4 flex-shrink-0">
            {icon_content}
        </div>
    }
}

/// Weapon icon placeholder - displays icon for weapons.
/// Replace SVG content with actual weapon icons.
#[component]
fn WeaponIcon(weapon: &'static str) -> impl IntoView {
    let icon_content = match weapon {
        "Railgun" => view! {
            <svg width="16" height="16" viewBox="0 0 16 16" class="text-slate-400">
                // Railgun icon placeholder - long barrel
                <rect x="2" y="6" width="12" height="4" rx="1" fill="none" stroke="currentColor" stroke-width="1.5" />
                <line x1="14" y1="8" x2="16" y2="8" stroke="currentColor" stroke-width="2" />
                <rect x="3" y="7" width="2" height="2" fill="currentColor" opacity="0.5" />
            </svg>
        }.into_any(),
        "Point Defense" => view! {
            <svg width="16" height="16" viewBox="0 0 16 16" class="text-slate-400">
                // Point defense icon placeholder - multi-barrel
                <circle cx="8" cy="8" r="3" fill="none" stroke="currentColor" stroke-width="1.5" />
                <line x1="8" y1="1" x2="8" y2="4" stroke="currentColor" stroke-width="1.5" />
                <line x1="8" y1="12" x2="8" y2="15" stroke="currentColor" stroke-width="1.5" />
                <line x1="1" y1="8" x2="4" y2="8" stroke="currentColor" stroke-width="1.5" />
                <line x1="12" y1="8" x2="15" y2="8" stroke="currentColor" stroke-width="1.5" />
            </svg>
        }.into_any(),
        _ => view! {
            <svg width="16" height="16" viewBox="0 0 16 16" class="text-slate-400">
                <rect x="2" y="2" width="12" height="12" rx="2" fill="none" stroke="currentColor" stroke-width="1.5" />
                <text x="8" y="11" text-anchor="middle" font-size="8" fill="currentColor">"W"</text>
            </svg>
        }.into_any(),
    };

    view! {
        <div class="w-4 h-4 flex-shrink-0">
            {icon_content}
        </div>
    }
}
