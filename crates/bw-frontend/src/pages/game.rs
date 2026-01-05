//! Main game page

use leptos::prelude::*;
use crate::components::game::*;
use crate::state::GameState;

#[component]
pub fn GamePage() -> impl IntoView {
    let game_state = expect_context::<GameState>();

    // TODO: Connect WebSocket and load initial state

    view! {
        <div class="h-screen flex flex-col">
            // Top bar with resources
            <header class="h-16 bg-slate-800 border-b border-slate-700 flex items-center px-4">
                <div class="flex-1">
                    <span class="text-xl font-bold text-amber-500">"BLACKWING"</span>
                    <span class="ml-4 text-sm text-slate-400">"Thornwick Sector"</span>
                </div>
                <ResourceBar />
            </header>

            // Main content
            <div class="flex-1 flex overflow-hidden">
                // Left sidebar - missions
                <aside class="w-80 bg-slate-800 border-r border-slate-700 overflow-y-auto">
                    <MissionPanel />
                </aside>

                // Center - sector map
                <main class="flex-1 relative">
                    <SectorMap />
                </main>

                // Right sidebar - ship status & comms
                <aside class="w-80 bg-slate-800 border-l border-slate-700 flex flex-col">
                    <div class="flex-1 overflow-y-auto">
                        <ShipStatus />
                    </div>
                    <div class="h-64 border-t border-slate-700">
                        <CommsPanel />
                    </div>
                </aside>
            </div>

            // Bottom bar - quick actions
            <footer class="h-12 bg-slate-800 border-t border-slate-700 flex items-center px-4 gap-4">
                <button class="px-3 py-1 bg-slate-700 hover:bg-slate-600 rounded text-sm">
                    "Dock"
                </button>
                <button class="px-3 py-1 bg-slate-700 hover:bg-slate-600 rounded text-sm">
                    "Patrol"
                </button>
                <button class="px-3 py-1 bg-red-900 hover:bg-red-800 rounded text-sm">
                    "Alert"
                </button>
                <div class="flex-1" />
                <span class="text-xs text-slate-500">
                    "Server: Connected | Tick: 0"
                </span>
            </footer>
        </div>
    }
}
