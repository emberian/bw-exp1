//! Mission panel component

use leptos::prelude::*;

#[component]
pub fn MissionPanel() -> impl IntoView {
    view! {
        <div class="p-4">
            <h2 class="text-lg font-semibold text-amber-500 mb-4">"Available Missions"</h2>

            // Sample missions
            <div class="space-y-3">
                <MissionCard
                    title="Pirate Activity"
                    description="A civilian freighter reports hostile contacts nearby."
                    mission_type="PirateIntercept"
                    reputation_reward=20
                    fame_reward=5
                    expires_in="4:32"
                />
                <MissionCard
                    title="Distress Signal"
                    description="An automated beacon is broadcasting. Situation unclear."
                    mission_type="DistressSignal"
                    reputation_reward=25
                    fame_reward=10
                    expires_in="2:15"
                />
            </div>

            <h2 class="text-lg font-semibold text-amber-500 mt-8 mb-4">"Active Mission"</h2>
            <div class="text-sm text-slate-400 italic">
                "No active mission. Accept a mission to begin."
            </div>
        </div>
    }
}

#[component]
fn MissionCard(
    title: &'static str,
    description: &'static str,
    mission_type: &'static str,
    reputation_reward: i32,
    fame_reward: i32,
    expires_in: &'static str,
) -> impl IntoView {
    view! {
        <div class="bg-slate-700/50 rounded-lg p-3 border border-slate-600 hover:border-amber-500/50
                    transition-colors cursor-pointer">
            <div class="flex justify-between items-start mb-2">
                <h3 class="font-semibold text-slate-200">{title}</h3>
                <span class="text-xs text-slate-400">{expires_in}</span>
            </div>
            <p class="text-sm text-slate-400 mb-3">{description}</p>
            <div class="flex gap-4 text-xs">
                <span class="text-green-400">"+"{reputation_reward}" Rep"</span>
                <span class="text-amber-400">"+"{fame_reward}" Fame"</span>
            </div>
        </div>
    }
}
