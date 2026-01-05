//! Combat log component
//!
//! Displays combat events and status when the player is in combat.

use leptos::prelude::*;

use crate::state::GameState;

#[component]
pub fn CombatLog() -> impl IntoView {
    let game_state = expect_context::<GameState>();

    let in_combat = move || game_state.in_combat();
    let combat_round = move || game_state.combat_round.get();
    let combat_events = move || game_state.combat_events.get();
    let combat_resolved = move || game_state.combat_resolved.get();
    let combat_winner = move || game_state.combat_winner.get();

    view! {
        <div class="h-full flex flex-col">
            // Header
            <div class="p-2 border-b border-slate-700 flex items-center justify-between">
                <h3 class="text-sm font-semibold text-slate-300">"Combat"</h3>
                <Show when=in_combat>
                    <span class="text-xs text-red-400 animate-pulse">"ENGAGED"</span>
                </Show>
            </div>

            // Combat info
            <Show
                when=move || in_combat() || combat_resolved()
                fallback=|| view! {
                    <div class="flex-1 flex items-center justify-center text-slate-500 text-sm">
                        "No active combat"
                    </div>
                }
            >
                <div class="p-2 border-b border-slate-700 bg-slate-900/50">
                    <div class="flex justify-between text-xs">
                        <span class="text-slate-400">"Round"</span>
                        <span class="text-slate-200">{combat_round}</span>
                    </div>
                    <Show when=combat_resolved>
                        <div class="mt-1 text-xs">
                            <span class="text-green-400">"Combat Resolved - "</span>
                            <span class="text-slate-200">
                                {move || combat_winner().unwrap_or_else(|| "Draw".to_string())}
                            </span>
                        </div>
                    </Show>
                </div>

                // Event log
                <div class="flex-1 overflow-y-auto p-2 space-y-1 text-xs">
                    <For
                        each=combat_events
                        key=|e| e.message.clone()
                        children=move |event| {
                            let event_color = match event.event_type.as_str() {
                                "hit" => "text-red-400",
                                "miss" => "text-slate-500",
                                "critical" => "text-amber-400",
                                "shield" => "text-blue-400",
                                _ => "text-slate-400",
                            };

                            view! {
                                <div class="leading-relaxed">
                                    <span class=event_color>
                                        {if event.hit { "HIT" } else { "MISS" }}
                                    </span>
                                    " "
                                    <span class="text-slate-300">{event.attacker_name.clone()}</span>
                                    " → "
                                    <span class="text-slate-300">{event.target_name.clone()}</span>
                                    {event.damage.map(|d| view! {
                                        <span class="text-red-400">{format!(" ({:.0} dmg)", d)}</span>
                                    })}
                                </div>
                            }
                        }
                    />

                    <Show when=move || combat_events().is_empty()>
                        <div class="text-slate-500 italic text-center py-4">
                            "Waiting for combat events..."
                        </div>
                    </Show>
                </div>
            </Show>

            // Clear button when resolved
            <Show when=combat_resolved>
                <div class="p-2 border-t border-slate-700">
                    <button
                        class="w-full px-3 py-1 bg-slate-700 hover:bg-slate-600 rounded text-xs"
                        on:click=move |_| game_state.clear_combat()
                    >
                        "Clear Combat Log"
                    </button>
                </div>
            </Show>
        </div>
    }
}

/// Combat indicator for the sector map.
#[component]
pub fn CombatIndicator() -> impl IntoView {
    let game_state = expect_context::<GameState>();
    let in_combat = move || game_state.in_combat();

    view! {
        <Show when=in_combat>
            <div class="absolute top-4 left-4 bg-red-900/90 border border-red-500 rounded-lg px-3 py-2 animate-pulse z-40">
                <div class="flex items-center gap-2">
                    <div class="w-3 h-3 rounded-full bg-red-500 animate-ping" />
                    <span class="text-red-200 text-sm font-semibold">"COMBAT"</span>
                </div>
            </div>
        </Show>
    }
}
