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
        <div class="p-4">
            <h2 class="text-lg font-semibold text-amber-500 mb-4">"Ship Status"</h2>

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
            <div class="text-xs text-slate-400 bg-slate-700/50 rounded px-2 py-1">
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
        <div class="flex justify-between bg-slate-700/50 rounded px-2 py-1">
            <span class="text-slate-300">{name}</span>
            <span class=move || {
                match status_for_class() {
                    "online" => "text-green-400",
                    "damaged" => "text-amber-400",
                    "offline" => "text-red-400",
                    _ => "text-slate-400",
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
        <div class="flex justify-between bg-slate-700/50 rounded px-2 py-1">
            <span class="text-slate-300">{name}</span>
            <div class="flex gap-3">
                <span class=move || ammo_status().0>{move || ammo_status().1}</span>
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
