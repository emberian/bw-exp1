//! Mission panel component
//!
//! Displays available and active missions from GameState.

use leptos::prelude::*;
use uuid::Uuid;
use gloo_timers::callback::Interval;

use crate::api::WsService;
use crate::state::{GameState, MissionInfo};
use super::ConfirmDialogState;

#[component]
pub fn MissionPanel() -> impl IntoView {
    let game_state = expect_context::<GameState>();
    let ws = expect_context::<WsService>();

    let available_missions = move || game_state.available_missions.get();
    let active_mission = move || game_state.active_mission.get();

    view! {
        <div class="p-4">
            <h2 class="text-lg font-semibold text-amber-500 mb-4">"Available Missions"</h2>

            <div class="space-y-3">
                <For
                    each=available_missions
                    key=|m| m.id
                    children=move |mission| {
                        let ws = ws;
                        view! {
                            <MissionCard
                                mission=mission
                                on_accept=move |id| {
                                    ws.accept_mission(id);
                                }
                            />
                        }
                    }
                />
            </div>

            // Show empty state if no missions
            <Show when=move || available_missions().is_empty()>
                <div class="text-sm text-slate-400 italic py-4">
                    "No missions available. Check back later or patrol the sector."
                </div>
            </Show>

            <h2 class="text-lg font-semibold text-amber-500 mt-8 mb-4">"Active Mission"</h2>

            <Show
                when=move || active_mission().is_some()
                fallback=|| view! {
                    <div class="text-sm text-slate-400 italic">
                        "No active mission. Accept a mission to begin."
                    </div>
                }
            >
                {move || {
                    if let Some(mission) = active_mission() {
                        let ws = ws;
                        view! {
                            <ActiveMissionCard
                                mission=mission
                                on_abandon=move |id| {
                                    ws.abandon_mission(id);
                                }
                            />
                        }.into_any()
                    } else {
                        view! { <div /> }.into_any()
                    }
                }}
            </Show>
        </div>
    }
}

#[component]
fn MissionCard<F>(mission: MissionInfo, on_accept: F) -> impl IntoView
where
    F: Fn(Uuid) + 'static + Clone + Send + Sync,
{
    let id = mission.id;
    let title = mission.title.clone();
    let description = mission.description.clone();
    let reputation_reward = mission.reputation_reward;
    let fame_reward = mission.fame_reward;
    let expires_in = mission.expires_in_seconds;
    let can_accept = mission.can_accept;
    let is_high_profile = mission.is_high_profile;
    let mission_type = mission.mission_type.clone();

    // Expiry countdown (if applicable)
    let has_expiry = expires_in.is_some();
    let expiry_seconds = expires_in.unwrap_or(0);

    // Type indicator color
    let type_color = match mission_type.as_str() {
        "PirateIntercept" => "text-red-400",
        "DistressSignal" => "text-yellow-400",
        "AsteroidThreat" => "text-orange-400",
        "TerroristPlot" => "text-purple-400",
        "Investigation" => "text-blue-400",
        _ => "text-slate-400",
    };

    // Clone mission_type for icon
    let mission_type_for_icon = mission_type.clone();

    view! {
        <div
            class=move || {
                let base = "bg-slate-700/50 rounded-lg p-3 border transition-colors";
                let border = if can_accept {
                    "border-slate-600"
                } else {
                    "border-slate-600/50 opacity-60"
                };
                let high_profile = if is_high_profile {
                    " ring-1 ring-amber-500/30"
                } else {
                    ""
                };
                format!("{} {}{}", base, border, high_profile)
            }
        >
            <div class="flex justify-between items-start mb-2">
                <div class="flex items-center gap-2">
                    <MissionTypeIcon mission_type=mission_type_for_icon.clone() />
                    <h3 class="font-semibold text-slate-200">{title}</h3>
                    <Show when=move || is_high_profile>
                        <HighProfileBadge />
                    </Show>
                </div>
                <Show when=move || has_expiry>
                    <CountdownTimer initial_seconds=expiry_seconds />
                </Show>
            </div>
            <p class="text-sm text-slate-400 mb-3">{description}</p>
            <div class="flex justify-between items-center">
                <div class="flex gap-4 text-xs">
                    <span class="text-green-400">"+"{reputation_reward}" Rep"</span>
                    <span class="text-amber-400">"+"{fame_reward}" Fame"</span>
                </div>
                <div class="flex items-center gap-2">
                    <span class={format!("text-xs {}", type_color)}>{mission_type}</span>
                    {if can_accept {
                        view! {
                            <button
                                class="px-3 py-1 bg-amber-600 hover:bg-amber-500 rounded text-xs text-white font-medium transition-colors"
                                on:click=move |_| on_accept(id)
                            >
                                "Accept"
                            </button>
                        }.into_any()
                    } else {
                        view! { <span /> }.into_any()
                    }}
                </div>
            </div>
        </div>
    }
}

#[component]
fn ActiveMissionCard<F>(mission: MissionInfo, on_abandon: F) -> impl IntoView
where
    F: Fn(Uuid) + 'static + Clone + Send + Sync,
{
    let id = mission.id;
    let title = mission.title.clone();
    let description = mission.description.clone();
    let reputation_reward = mission.reputation_reward;
    let fame_reward = mission.fame_reward;
    let progress = mission.progress;
    let status = mission.status.clone();
    let mission_type = mission.mission_type.clone();

    // Confirmation dialog state
    let confirm_state = ConfirmDialogState::new();

    const CONFIRM_ABANDON: u32 = 1;

    let on_abandon_clone = on_abandon.clone();
    let handle_confirm = move |confirm_id: u32| {
        if confirm_id == CONFIRM_ABANDON {
            on_abandon_clone(id);
        }
    };

    let handle_abandon_click = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        confirm_state.show(
            "Abandon Mission",
            "Are you sure you want to abandon this mission? You will lose all progress.",
            CONFIRM_ABANDON
        );
    };

    // Progress bar width
    let progress_width = format!("{}%", (progress * 100.0) as i32);

    // Status text color
    let status_color = match status.as_str() {
        "InProgress" => "text-blue-400",
        "AwaitingChoice" => "text-yellow-400",
        "InCombat" => "text-red-400",
        _ => "text-slate-400",
    };

    view! {
        <div class="bg-slate-700/70 rounded-lg p-4 border border-amber-500/50">
            <div class="flex justify-between items-start mb-2">
                <div class="flex items-center gap-2">
                    <MissionTypeIcon mission_type=mission_type />
                    <h3 class="font-semibold text-amber-400">{title}</h3>
                </div>
                <span class={format!("text-xs {}", status_color)}>{status}</span>
            </div>
            <p class="text-sm text-slate-400 mb-3">{description}</p>

            // Progress bar
            <div class="mb-3">
                <div class="flex justify-between text-xs text-slate-500 mb-1">
                    <span>"Progress"</span>
                    <span>{format!("{:.0}%", progress * 100.0)}</span>
                </div>
                <div class="h-2 bg-slate-800 rounded-full overflow-hidden">
                    <div
                        class="h-full bg-amber-500 transition-all duration-300"
                        style=format!("width: {}", progress_width)
                    />
                </div>
            </div>

            <div class="flex justify-between items-center">
                <div class="flex gap-4 text-xs">
                    <span class="text-green-400">"+"{reputation_reward}" Rep"</span>
                    <span class="text-amber-400">"+"{fame_reward}" Fame"</span>
                </div>
                <button
                    class="text-xs text-red-400 hover:text-red-300 hover:underline"
                    on:click=handle_abandon_click
                >
                    "Abandon"
                </button>
            </div>

            // Confirmation dialog
            <super::ConfirmDialog
                state=confirm_state
                on_confirm=handle_confirm
            />
        </div>
    }
}

/// Countdown timer component for mission expiry.
#[component]
fn CountdownTimer(initial_seconds: u32) -> impl IntoView {
    use std::cell::RefCell;
    use std::rc::Rc;

    let remaining = RwSignal::new(initial_seconds);

    // Store interval in Rc<RefCell> so we can cancel it when timer reaches zero
    let interval_handle: Rc<RefCell<Option<Interval>>> = Rc::new(RefCell::new(None));

    if initial_seconds > 0 {
        let interval_handle_clone = interval_handle.clone();
        let interval = Interval::new(1_000, move || {
            let should_stop = remaining.try_update(|secs| {
                if *secs > 0 {
                    *secs -= 1;
                }
                *secs == 0
            }).unwrap_or(true);

            // Stop the interval when we reach zero
            if should_stop {
                interval_handle_clone.borrow_mut().take();
            }
        });
        *interval_handle.borrow_mut() = Some(interval);
    }

    // Format the time display
    let time_text = move || {
        let secs = remaining.get();
        if secs == 0 {
            "Expired".to_string()
        } else {
            let mins = secs / 60;
            let remaining_secs = secs % 60;
            format!("{}:{:02}", mins, remaining_secs)
        }
    };

    let time_class = move || {
        let secs = remaining.get();
        if secs == 0 {
            "text-xs text-red-400"
        } else if secs < 60 {
            "text-xs text-red-400 animate-pulse"
        } else if secs < 300 {
            "text-xs text-amber-400"
        } else {
            "text-xs text-slate-400"
        }
    };

    view! {
        <span class=time_class>{time_text}</span>
    }
}

// =============================================================================
// Asset Placeholder Components
// =============================================================================

/// Mission type icon placeholder - displays icon based on mission type.
#[component]
fn MissionTypeIcon(mission_type: String) -> impl IntoView {
    let icon_content = match mission_type.as_str() {
        "PirateIntercept" => view! {
            <svg width="18" height="18" viewBox="0 0 18 18" class="text-red-400">
                // Skull/pirate icon placeholder
                <circle cx="9" cy="7" r="5" fill="none" stroke="currentColor" stroke-width="1.5" />
                <circle cx="7" cy="6" r="1" fill="currentColor" />
                <circle cx="11" cy="6" r="1" fill="currentColor" />
                <path d="M7 9 L9 10 L11 9" fill="none" stroke="currentColor" stroke-width="1" />
                <line x1="9" y1="12" x2="9" y2="16" stroke="currentColor" stroke-width="1.5" />
                <line x1="6" y1="14" x2="12" y2="14" stroke="currentColor" stroke-width="1.5" />
            </svg>
        }.into_any(),
        "DistressSignal" => view! {
            <svg width="18" height="18" viewBox="0 0 18 18" class="text-yellow-400">
                // SOS/beacon icon placeholder
                <circle cx="9" cy="9" r="3" fill="currentColor" opacity="0.5" />
                <circle cx="9" cy="9" r="5" fill="none" stroke="currentColor" stroke-width="1" opacity="0.7" />
                <circle cx="9" cy="9" r="7" fill="none" stroke="currentColor" stroke-width="1" opacity="0.4" />
                <path d="M9 2 L9 4" stroke="currentColor" stroke-width="1.5" />
                <path d="M9 14 L9 16" stroke="currentColor" stroke-width="1.5" />
                <path d="M2 9 L4 9" stroke="currentColor" stroke-width="1.5" />
                <path d="M14 9 L16 9" stroke="currentColor" stroke-width="1.5" />
            </svg>
        }.into_any(),
        "AsteroidThreat" => view! {
            <svg width="18" height="18" viewBox="0 0 18 18" class="text-orange-400">
                // Asteroid icon placeholder
                <polygon points="9,2 14,5 16,10 13,15 6,16 3,11 4,6" fill="none" stroke="currentColor" stroke-width="1.5" />
                <circle cx="8" cy="8" r="1.5" fill="currentColor" opacity="0.3" />
                <circle cx="11" cy="11" r="1" fill="currentColor" opacity="0.3" />
                <circle cx="6" cy="12" r="0.8" fill="currentColor" opacity="0.3" />
            </svg>
        }.into_any(),
        "TerroristPlot" => view! {
            <svg width="18" height="18" viewBox="0 0 18 18" class="text-purple-400">
                // Conspiracy/bomb icon placeholder
                <circle cx="9" cy="10" r="5" fill="none" stroke="currentColor" stroke-width="1.5" />
                <path d="M9 5 L9 2 L11 1" fill="none" stroke="currentColor" stroke-width="1.5" />
                <circle cx="12" cy="1" r="1" fill="currentColor" class="animate-pulse" />
                <line x1="7" y1="9" x2="11" y2="9" stroke="currentColor" stroke-width="1" />
                <line x1="7" y1="11" x2="11" y2="11" stroke="currentColor" stroke-width="1" />
            </svg>
        }.into_any(),
        "Investigation" => view! {
            <svg width="18" height="18" viewBox="0 0 18 18" class="text-blue-400">
                // Magnifying glass icon placeholder
                <circle cx="8" cy="8" r="5" fill="none" stroke="currentColor" stroke-width="1.5" />
                <line x1="12" y1="12" x2="16" y2="16" stroke="currentColor" stroke-width="2" />
                <circle cx="8" cy="8" r="2" fill="none" stroke="currentColor" stroke-width="1" opacity="0.5" />
            </svg>
        }.into_any(),
        _ => view! {
            <svg width="18" height="18" viewBox="0 0 18 18" class="text-slate-400">
                <rect x="3" y="3" width="12" height="12" rx="2" fill="none" stroke="currentColor" stroke-width="1.5" />
                <text x="9" y="12" text-anchor="middle" font-size="8" fill="currentColor">"M"</text>
            </svg>
        }.into_any(),
    };

    view! {
        <div class="w-5 h-5 flex-shrink-0">
            {icon_content}
        </div>
    }
}

/// High profile mission badge placeholder.
#[component]
fn HighProfileBadge() -> impl IntoView {
    view! {
        <div class="flex items-center gap-1 px-1.5 py-0.5 bg-amber-500/20 border border-amber-500/50 rounded text-xs text-amber-400 font-bold">
            // Star icon
            <svg width="10" height="10" viewBox="0 0 10 10" class="text-amber-500">
                <polygon points="5,0 6,3.5 10,4 7,6.5 8,10 5,8 2,10 3,6.5 0,4 4,3.5" fill="currentColor" />
            </svg>
            "HIGH PROFILE"
        </div>
    }
}
