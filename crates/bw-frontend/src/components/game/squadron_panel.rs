//! Squadron management panel
//!
//! Displays squadron info and provides management actions.

use leptos::prelude::*;
use crate::state::GameState;

#[component]
pub fn SquadronPanel() -> impl IntoView {
    let game_state = expect_context::<GameState>();

    view! {
        <div class="p-4">
            <h2 class="text-lg font-semibold text-amber-500 mb-4">"Squadron"</h2>

            {move || {
                match game_state.squadron.get() {
                    Some(squadron) => view! {
                        <SquadronInfo squadron=squadron.clone() />
                    }.into_any(),
                    None => view! {
                        <NoSquadron />
                    }.into_any(),
                }
            }}
        </div>
    }
}

/// Display when player is in a squadron.
#[component]
fn SquadronInfo(squadron: crate::state::SquadronInfo) -> impl IntoView {
    let game_state = expect_context::<GameState>();

    let role_text = if squadron.is_leader {
        "Leader"
    } else if squadron.is_officer {
        "Officer"
    } else {
        "Member"
    };

    let role_color = if squadron.is_leader {
        "text-amber-400"
    } else if squadron.is_officer {
        "text-blue-400"
    } else {
        "text-slate-400"
    };

    view! {
        <div class="space-y-4">
            // Squadron header
            <div class="bg-slate-700/50 rounded-lg p-3 border border-slate-600">
                <div class="flex justify-between items-start">
                    <div>
                        <h3 class="font-semibold text-slate-200">
                            "["{squadron.tag.clone()}"] "{squadron.name.clone()}
                        </h3>
                        <p class="text-sm text-slate-400 italic">
                            {squadron.motto.clone().unwrap_or_else(|| "No motto set".to_string())}
                        </p>
                    </div>
                    <span class=format!("text-xs px-2 py-0.5 rounded bg-slate-800 {}", role_color)>
                        {role_text}
                    </span>
                </div>
            </div>

            // Stats
            <div class="grid grid-cols-2 gap-3 text-sm">
                <StatCard label="Members" value=format!("{}", squadron.member_count) />
                <StatCard label="Leader" value=squadron.leader_name.clone() />
                <StatCard label="Rep Bonus" value=format!("+{:.0}%", squadron.reputation_bonus) />
                <StatCard label="Fame Bonus" value=format!("+{:.0}%", squadron.fame_bonus) />
            </div>

            // War status
            {if squadron.is_at_war {
                view! {
                    <div class="bg-red-900/30 border border-red-600/50 rounded-lg p-2 text-center">
                        <span class="text-red-400 text-sm font-semibold">"AT WAR"</span>
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}

            // Actions
            <div class="space-y-2">
                <button
                    class="w-full px-3 py-2 bg-slate-700 hover:bg-slate-600 rounded text-sm text-slate-200 transition-colors"
                    on:click=move |_| {
                        // TODO: Open squadron details dialog
                        game_state.show_squadron_dialog.set(true);
                    }
                >
                    "View Details"
                </button>

                <button
                    class="w-full px-3 py-2 bg-red-900/50 hover:bg-red-800/50 rounded text-sm text-red-400 transition-colors"
                    on:click=move |_| {
                        // TODO: Send leave squadron message via WebSocket
                        leptos::logging::log!("Leave squadron clicked");
                    }
                >
                    "Leave Squadron"
                </button>
            </div>
        </div>
    }
}

/// Display when player is not in a squadron.
#[component]
fn NoSquadron() -> impl IntoView {
    let create_name = RwSignal::new(String::new());
    let create_tag = RwSignal::new(String::new());
    let show_create = RwSignal::new(false);

    view! {
        <div class="space-y-4">
            <p class="text-sm text-slate-400">
                "You are not in a squadron. Join or create one to access collective bonuses and sector control."
            </p>

            {move || {
                if show_create.get() {
                    view! {
                        <div class="bg-slate-700/50 rounded-lg p-3 border border-slate-600 space-y-3">
                            <h3 class="font-semibold text-slate-200">"Create Squadron"</h3>

                            <div>
                                <label class="text-xs text-slate-400">"Squadron Name"</label>
                                <input
                                    type="text"
                                    prop:value=move || create_name.get()
                                    on:input=move |ev| create_name.set(event_target_value(&ev))
                                    class="w-full mt-1 px-2 py-1 bg-slate-700 border border-slate-600 rounded text-sm text-slate-100 focus:outline-none focus:border-amber-500"
                                    placeholder="Enter name (3-32 chars)"
                                    maxlength="32"
                                />
                            </div>

                            <div>
                                <label class="text-xs text-slate-400">"Tag"</label>
                                <input
                                    type="text"
                                    prop:value=move || create_tag.get()
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev).to_uppercase();
                                        create_tag.set(val);
                                    }
                                    class="w-full mt-1 px-2 py-1 bg-slate-700 border border-slate-600 rounded text-sm text-slate-100 uppercase focus:outline-none focus:border-amber-500"
                                    placeholder="TAG (2-5 letters)"
                                    maxlength="5"
                                />
                            </div>

                            <p class="text-xs text-slate-500">
                                "Creating a squadron costs 50 reputation."
                            </p>

                            <div class="flex gap-2">
                                <button
                                    class="flex-1 px-3 py-2 bg-amber-600 hover:bg-amber-500 disabled:bg-slate-600 disabled:cursor-not-allowed rounded text-sm transition-colors"
                                    disabled=move || {
                                        create_name.get().len() < 3 || create_tag.get().len() < 2
                                    }
                                    on:click=move |_| {
                                        // TODO: Send create squadron message via WebSocket
                                        leptos::logging::log!("Create squadron: {} [{}]", create_name.get(), create_tag.get());
                                    }
                                >
                                    "Create"
                                </button>
                                <button
                                    class="px-3 py-2 bg-slate-700 hover:bg-slate-600 rounded text-sm transition-colors"
                                    on:click=move |_| show_create.set(false)
                                >
                                    "Cancel"
                                </button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="space-y-2">
                            <button
                                class="w-full px-3 py-2 bg-amber-600 hover:bg-amber-500 rounded text-sm transition-colors"
                                on:click=move |_| show_create.set(true)
                            >
                                "Create Squadron"
                            </button>
                        </div>
                    }.into_any()
                }
            }}

            // Benefits info
            <div class="bg-slate-800/50 rounded-lg p-3 border border-slate-700">
                <h4 class="text-sm font-semibold text-slate-300 mb-2">"Squadron Benefits"</h4>
                <ul class="text-xs text-slate-400 space-y-1">
                    <li>"+ Collective reputation bonuses"</li>
                    <li>"+ Claim and control patrol sectors"</li>
                    <li>"+ Build stations (costs reputation)"</li>
                    <li>"+ War games (PvP training)"</li>
                    <li>"+ Privateering (actual PvP)"</li>
                </ul>
            </div>
        </div>
    }
}

#[component]
fn StatCard(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="bg-slate-700/50 rounded px-2 py-1">
            <div class="text-xs text-slate-400">{label}</div>
            <div class="text-sm text-slate-200">{value}</div>
        </div>
    }
}
