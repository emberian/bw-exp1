//! Combat log component
//!
//! Displays combat events and status when the player is in combat.

use leptos::prelude::*;

use crate::api::WsService;
use crate::state::GameState;

#[component]
pub fn CombatLog() -> impl IntoView {
    let game_state = expect_context::<GameState>();
    let ws = expect_context::<WsService>();

    let in_combat = move || game_state.in_combat();
    let combat_round = move || game_state.combat_round.get();
    let combat_events = move || game_state.combat_events.get();
    let combat_resolved = move || game_state.combat_resolved.get();
    let combat_winner = move || game_state.combat_winner.get();
    let selected_target = move || game_state.selected_target.get();
    let ammunition = move || game_state.ammunition.get();

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

                // Weapon controls (only show when in active combat)
                {move || {
                    if in_combat() && !combat_resolved() {
                        let has_target_1 = move || selected_target().is_some();
                        let has_target_2 = move || selected_target().is_some();
                        let has_ammo_railgun = move || ammunition() >= 5.0;
                        let has_ammo_pd = move || ammunition() >= 2.0;
                        let no_target = move || selected_target().is_none();

                        view! {
                            <div class="p-2 border-b border-slate-700 bg-slate-800/50">
                                <div class="text-xs text-slate-400 mb-2">"Weapons"</div>
                                <div class="grid grid-cols-2 gap-1">
                                    <WeaponButton
                                        name="Railgun"
                                        weapon_index=0
                                        ammo_cost=5.0
                                        damage=20
                                        has_target=has_target_1
                                        has_ammo=has_ammo_railgun
                                        ws=ws
                                        selected_target=selected_target
                                    />
                                    <WeaponButton
                                        name="Point Def"
                                        weapon_index=1
                                        ammo_cost=2.0
                                        damage=5
                                        has_target=has_target_2
                                        has_ammo=has_ammo_pd
                                        ws=ws
                                        selected_target=selected_target
                                    />
                                </div>
                                <Show when=no_target>
                                    <div class="text-xs text-amber-400 mt-1 italic">
                                        "Select a target to fire"
                                    </div>
                                </Show>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div /> }.into_any()
                    }
                }}

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

/// Weapon button for manual firing.
#[component]
fn WeaponButton<HT, HA, ST>(
    name: &'static str,
    weapon_index: usize,
    ammo_cost: f32,
    damage: u32,
    has_target: HT,
    has_ammo: HA,
    ws: WsService,
    selected_target: ST,
) -> impl IntoView
where
    HT: Fn() -> bool + 'static + Clone + Send + Sync,
    HA: Fn() -> bool + 'static + Clone + Send + Sync,
    ST: Fn() -> Option<uuid::Uuid> + 'static + Clone + Send + Sync,
{
    // Clone closures for different uses
    let has_target_class = has_target.clone();
    let has_ammo_class = has_ammo.clone();
    let has_target_disabled = has_target.clone();
    let has_ammo_disabled = has_ammo.clone();
    let has_target_click = has_target;
    let has_ammo_click = has_ammo;
    let selected_target_click = selected_target;

    let handle_click = move |_| {
        if has_target_click() && has_ammo_click() {
            if let Some(target_id) = selected_target_click() {
                ws.fire_weapon(weapon_index, target_id);
            }
        }
    };

    view! {
        <button
            class=move || {
                let base = "px-2 py-1 rounded text-xs transition-colors";
                if has_target_class() && has_ammo_class() {
                    format!("{} bg-red-900 hover:bg-red-800 text-red-100", base)
                } else {
                    format!("{} bg-slate-700 text-slate-500 cursor-not-allowed", base)
                }
            }
            disabled=move || !(has_target_disabled() && has_ammo_disabled())
            on:click=handle_click
        >
            <div class="font-medium">{name}</div>
            <div class="text-[10px] opacity-70">
                {damage}" dmg | "{ammo_cost}" ammo"
            </div>
        </button>
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
