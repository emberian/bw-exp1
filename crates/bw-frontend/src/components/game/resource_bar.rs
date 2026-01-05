//! Resource bar component
//!
//! Displays the six core resources:
//! - Reputation, Fame (player-level)
//! - Ammunition, Fuel (ship-level)
//! - Morale, Experience (crew-level)

use leptos::prelude::*;
use crate::state::GameState;

#[component]
pub fn ResourceBar() -> impl IntoView {
    let game_state = expect_context::<GameState>();

    view! {
        <div class="flex gap-6 items-center">
            // Reputation
            <ResourceItem
                label="Rep"
                value=game_state.reputation
                icon="⭐"
            />

            // Fame
            <ResourceItemDecaying
                label="Fame"
                value=game_state.fame
                icon="📢"
            />

            <div class="w-px h-8 bg-slate-600" />

            // Ammunition
            <ResourceGaugeWarning
                label="Ammo"
                value=game_state.ammunition
                warning_threshold=15.0
            />

            // Fuel
            <ResourceGaugeWarning
                label="Fuel"
                value=game_state.fuel
                warning_threshold=10.0
            />

            <div class="w-px h-8 bg-slate-600" />

            // Morale
            <ResourceGaugeNeutral
                label="Morale"
                value=game_state.morale
                neutral_at=50.0
            />

            // Experience
            <ResourceItem
                label="XP"
                value=game_state.experience
                icon="📊"
            />
        </div>
    }
}

#[component]
fn ResourceItem(
    label: &'static str,
    value: RwSignal<i32>,
    icon: &'static str,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2">
            <span class="text-lg">{icon}</span>
            <div class="flex flex-col">
                <span class="text-xs text-slate-400">{label}</span>
                <span class="text-slate-100">
                    {move || value.get().to_string()}
                </span>
            </div>
        </div>
    }
}

#[component]
fn ResourceItemDecaying(
    label: &'static str,
    value: RwSignal<i32>,
    icon: &'static str,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2">
            <span class="text-lg">{icon}</span>
            <div class="flex flex-col">
                <span class="text-xs text-slate-400">{label}</span>
                <span class="text-amber-400">
                    {move || value.get().to_string()}
                </span>
            </div>
        </div>
    }
}

#[component]
fn ResourceGaugeWarning(
    label: &'static str,
    value: RwSignal<f32>,
    warning_threshold: f32,
) -> impl IntoView {
    let bar_color = move || {
        if value.get() <= warning_threshold {
            "bg-red-500"
        } else {
            "bg-blue-500"
        }
    };

    let text_color = move || {
        if value.get() <= warning_threshold {
            "text-red-400"
        } else {
            "text-slate-100"
        }
    };

    view! {
        <div class="flex flex-col">
            <span class="text-xs text-slate-400">{label}</span>
            <div class="w-20 h-2 bg-slate-700 rounded overflow-hidden">
                <div
                    class=move || format!("h-full transition-all {}", bar_color())
                    style=move || format!("width: {}%", value.get())
                />
            </div>
            <span class=move || format!("text-xs {}", text_color())>
                {move || format!("{:.0}%", value.get())}
            </span>
        </div>
    }
}

#[component]
fn ResourceGaugeNeutral(
    label: &'static str,
    value: RwSignal<f32>,
    neutral_at: f32,
) -> impl IntoView {
    let bar_color = move || {
        if value.get() < neutral_at {
            "bg-yellow-500"
        } else {
            "bg-green-500"
        }
    };

    view! {
        <div class="flex flex-col">
            <span class="text-xs text-slate-400">{label}</span>
            <div class="w-20 h-2 bg-slate-700 rounded overflow-hidden">
                <div
                    class=move || format!("h-full transition-all {}", bar_color())
                    style=move || format!("width: {}%", value.get())
                />
            </div>
            <span class="text-xs text-slate-100">
                {move || format!("{:.0}%", value.get())}
            </span>
        </div>
    }
}
